//! bemu in a web page: its web build (`bemu-web.zip` of the releases of
//! github.com/rgiot/bemu), served over loopback with a debugger reaching it.
//!
//! The page has no HTTP server: bemu's debug API is a method of its
//! emulator object (`emu.api(method, path, query, body)`, answered by the
//! code that answers the desktop's server), and its events are collected
//! (`emu.debugEvents()`). So this server *relays* that API: whatever asks
//! `http://127.0.0.1:<port>/api/...` - the debugger's AMSpiriT Lite peer,
//! unchanged - is answered by the page, which long-polls for the requests
//! (`/session/next`) and posts the answers back (`/session/reply`) and the
//! events (`/session/events`, streamed again as `/api/events`).
//!
//! The page's routes carry the session's token, as `web::server`'s do. The
//! API carries none - the peer speaks bemu's own API, which has none - but
//! refuses whatever a browser sends (an `Origin` or `Sec-Fetch-*` header):
//! another page on this machine cannot drive the debugger through it.
//!
//! When the page goes (its tab closed), the server stops listening: the
//! peer's connections are refused, which is how it notices an emulator gone.

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

use super::mime_for;
use super::server::{Request, random_token, read_request, respond};

/// The web build: the development build of the releases, as bndbuild's
/// native bemu is.
pub const WEB_BUILD_URL: &str =
    "https://github.com/rgiot/bemu/releases/download/latest/bemu-web.zip";

/// What a web build with the debug API has in its `bemu.js`.
const DEBUG_API_MARK: &str = "debugEvents";

/// Where the web build is installed.
pub fn cache_folder() -> Utf8PathBuf {
    crate::delegated::base_cache_folder().join("bemu_web_latest")
}

/// The folder with `bemu.js` in it: `$CPCLIB_BEMU_WEB` (a web build of your
/// own: its `pkg` folder, or the folder holding it), else the installed one,
/// downloaded if it is missing or too old to have the debug API.
pub fn package_root() -> Result<Utf8PathBuf, String> {
    if let Some(own) = std::env::var_os("CPCLIB_BEMU_WEB") {
        let own = Utf8PathBuf::from_path_buf(own.into())
            .map_err(|p| format!("CPCLIB_BEMU_WEB is not UTF-8: {}", p.display()))?;
        return [own.clone(), own.join("pkg")]
            .into_iter()
            .find(|folder| folder.join("bemu.js").is_file())
            .ok_or_else(|| {
                format!("CPCLIB_BEMU_WEB ({own}) holds no bemu.js, nor a pkg folder with one")
            });
    }
    install()
}

fn has_debug_api(package: &Utf8Path) -> bool {
    fs_err::read_to_string(package.join("bemu.js")).is_ok_and(|text| text.contains(DEBUG_API_MARK))
}

/// Install the web build (`<cache>/pkg`), unless one with the debug API is
/// already there. Returns the `pkg` folder.
pub fn install() -> Result<Utf8PathBuf, String> {
    let root = cache_folder();
    let package = root.join("pkg");
    if has_debug_api(&package) {
        return Ok(package);
    }
    let mut reader = cpclib_common::network::download(WEB_BUILD_URL)
        .map_err(|e| format!("cannot download bemu's web build ({WEB_BUILD_URL}): {e}"))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes)
        .map_err(|e| format!("cannot read bemu's web build: {e}"))?;
    unpack(&bytes, &root)?;
    if !has_debug_api(&package) {
        return Err(format!(
            "the web build of bemu ({WEB_BUILD_URL}) has no debug API yet: set CPCLIB_BEMU_WEB to a newer \
             one, or choose bemu's own window (\"bemuWindow\": true)"
        ));
    }
    Ok(package)
}

/// Unpack the archive into `root`, replacing what was there.
fn unpack(bytes: &[u8], root: &Utf8Path) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("bemu's web build is not a zip archive: {e}"))?;
    // Unpacked aside first: a failure leaves the previous install intact.
    let fresh = root.with_extension("partial");
    let _ = fs_err::remove_dir_all(&fresh);
    fs_err::create_dir_all(&fresh).map_err(|e| format!("cannot create {fresh}: {e}"))?;
    archive
        .extract(fresh.as_std_path())
        .map_err(|e| format!("cannot unpack bemu's web build: {e}"))?;
    let _ = fs_err::remove_dir_all(root);
    fs_err::rename(&fresh, root)
        .map_err(|e| format!("cannot install bemu's web build in {root}: {e}"))
}

/// What the page starts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Load {
    /// A snapshot, loaded paused: the debugger lets it go once its
    /// breakpoints are armed (`POST /api/config {"paused": false}`).
    Snapshot(Vec<u8>),
    /// A disc in drive A, the machine booting as usual.
    Disc(Vec<u8>),
    /// A cold boot to BASIC.
    Boot
}

impl Load {
    fn kind(&self) -> &'static str {
        match self {
            Self::Snapshot(_) => "snapshot",
            Self::Disc(_) => "disc",
            Self::Boot => "boot"
        }
    }

    fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Snapshot(bytes) | Self::Disc(bytes) => Some(bytes),
            Self::Boot => None
        }
    }
}

/// A request of the API, waiting for the page.
struct Pending {
    id: u64,
    method: String,
    path: String,
    query: String,
    body: Vec<u8>
}

/// The page's answer.
struct Reply {
    status: u16,
    content_type: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>
}

/// How the page is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PageState {
    /// Not loaded yet.
    Waiting,
    /// The emulator runs, loaded.
    Ready,
    /// It could not start: why.
    Failed(String)
}

struct Relay {
    package: Utf8PathBuf,
    token: String,
    load: Load,
    /// The emulator's configuration (`Bemu.create`'s), as JSON.
    config: String,
    state: Mutex<PageState>,
    state_changed: Condvar,
    queue: Mutex<VecDeque<Pending>>,
    queued: Condvar,
    replies: Mutex<HashMap<u64, Sender<Reply>>>,
    next_id: AtomicU64,
    subscribers: Mutex<Vec<Sender<String>>>,
    /// When the page last asked for requests, and whether it is asking now.
    polling: Mutex<(Instant, usize)>,
    stopped: AtomicBool
}

/// How long a request waits for the page to answer it.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(30);
/// How long the page's long poll is held.
const POLL_HOLD: Duration = Duration::from_secs(15);
/// The page asked nothing for this long, after its first poll: it is gone.
const PAGE_GONE_AFTER: Duration = Duration::from_secs(5);
/// How long a request made before the page is up waits for it.
const PAGE_LOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// A running relay: the page to open, and the API to debug.
pub struct BemuPage {
    address: SocketAddr,
    relay: Arc<Relay>
}

impl BemuPage {
    /// The page, for an editor tab or a browser.
    pub fn page_url(&self) -> String {
        format!("http://{}/debug", self.address)
    }

    /// bemu's debug API, as a desktop bemu serves it.
    pub fn endpoint(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn port(&self) -> u16 {
        self.address.port()
    }

    /// Wait for the page to run the emulator, loaded; its error otherwise.
    pub fn wait_until_ready(&self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        let mut state = self.relay.state.lock().unwrap();
        loop {
            match &*state {
                PageState::Ready => return Ok(()),
                PageState::Failed(why) => {
                    return Err(format!("bemu could not start in its page: {why}"));
                },
                PageState::Waiting => {}
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(format!(
                    "bemu's page ({}) was not opened, or did not start, within {}s",
                    self.page_url(),
                    timeout.as_secs()
                ));
            }
            state = self
                .relay
                .state_changed
                .wait_timeout(state, deadline - now)
                .unwrap()
                .0;
        }
    }

    /// Whether the page has gone (its tab closed).
    pub fn page_gone(&self) -> bool {
        self.relay.stopped.load(Ordering::Relaxed) || self.relay.page_gone()
    }
}

impl Drop for BemuPage {
    fn drop(&mut self) {
        self.relay.stop();
        // wake the accepting thread, for it to see it
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_millis(200));
    }
}

/// Serve bemu's web build from `package` (the folder with `bemu.js`), the
/// page starting with `load`, the machine configured with `config`
/// (`Bemu.create`'s: `{"model": "6128"}`...).
pub fn serve(
    package: &Utf8Path,
    load: Load,
    config: serde_json::Value
) -> std::io::Result<BemuPage> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;
    let relay = Arc::new(Relay {
        package: package.to_path_buf(),
        token: random_token(),
        load,
        config: config.to_string(),
        state: Mutex::new(PageState::Waiting),
        state_changed: Condvar::new(),
        queue: Mutex::new(VecDeque::new()),
        queued: Condvar::new(),
        replies: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(1),
        subscribers: Mutex::new(Vec::new()),
        polling: Mutex::new((Instant::now(), 0)),
        stopped: AtomicBool::new(false)
    });
    let served = Arc::clone(&relay);
    std::thread::Builder::new()
        .name("bemu-page".into())
        .spawn(move || {
            for stream in listener.incoming() {
                if served.stopped.load(Ordering::Relaxed) || served.page_gone() {
                    served.stop();
                    break; // the listener closes: the peer's connections are refused
                }
                let Ok(stream) = stream
                else {
                    continue;
                };
                let relay = Arc::clone(&served);
                let _ = std::thread::Builder::new()
                    .name("bemu-page-conn".into())
                    .spawn(move || {
                        let _ = relay.handle(stream);
                    });
            }
        })?;
    Ok(BemuPage { address, relay })
}

impl Relay {
    fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.subscribers.lock().unwrap().clear();
        self.replies.lock().unwrap().clear();
        self.queued.notify_all();
        self.state_changed.notify_all();
    }

    fn page_gone(&self) -> bool {
        let state = self.state.lock().unwrap().clone();
        if state == PageState::Waiting {
            return false; // not come yet
        }
        let (last, active) = *self.polling.lock().unwrap();
        active == 0 && last.elapsed() > PAGE_GONE_AFTER
    }

    fn handle(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let Some(request) = read_request(&mut stream)?
        else {
            return Ok(());
        };
        if request.path == "/api" || request.path.starts_with("/api/") {
            return self.api(stream, request);
        }
        let authorised =
            request.query.get("token").map(String::as_str) == Some(self.token.as_str());
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/debug") => {
                respond(
                    &mut stream,
                    200,
                    "text/html; charset=utf-8",
                    self.page().as_bytes()
                )
            },
            ("GET", "/session/program") if authorised => {
                match self.load.bytes() {
                    Some(bytes) => respond(&mut stream, 200, "application/octet-stream", bytes),
                    None => respond(&mut stream, 404, "text/plain", b"nothing to load")
                }
            },
            ("POST", "/session/ready") if authorised => {
                self.set_state(PageState::Ready);
                respond(&mut stream, 204, "text/plain", b"")
            },
            ("POST", "/session/failed") if authorised => {
                self.set_state(PageState::Failed(
                    String::from_utf8_lossy(&request.body).into_owned()
                ));
                respond(&mut stream, 204, "text/plain", b"")
            },
            ("GET", "/session/next") if authorised => self.next(stream),
            ("POST", "/session/reply") if authorised => {
                self.reply(&request);
                respond(&mut stream, 204, "text/plain", b"")
            },
            ("POST", "/session/events") if authorised => {
                let events = String::from_utf8_lossy(&request.body).into_owned();
                self.subscribers
                    .lock()
                    .unwrap()
                    .retain(|subscriber| subscriber.send(events.clone()).is_ok());
                respond(&mut stream, 204, "text/plain", b"")
            },
            (_, path) if path.starts_with("/session/") => {
                respond(&mut stream, 403, "text/plain", b"missing or wrong token")
            },
            ("GET", path) if path.starts_with("/bemu/") => {
                self.file(&mut stream, &path["/bemu/".len()..])
            },
            _ => respond(&mut stream, 404, "text/plain", b"not found")
        }
    }

    fn set_state(&self, state: PageState) {
        let mut current = self.state.lock().unwrap();
        // the first word counts: a page reloaded is not a new emulator
        if *current == PageState::Waiting {
            *current = state;
        }
        self.polling.lock().unwrap().0 = Instant::now();
        drop(current);
        self.state_changed.notify_all();
    }

    /// A file of the web build. Any `..` is refused outright.
    fn file(&self, stream: &mut TcpStream, relative: &str) -> std::io::Result<()> {
        if relative.is_empty() || relative.contains("..") || relative.contains('\\') {
            return respond(stream, 403, "text/plain", b"forbidden");
        }
        let file = self.package.join(relative);
        match fs_err::read(&file) {
            Ok(bytes) => respond(stream, 200, mime_for(&file), &bytes),
            Err(_) => respond(stream, 404, "text/plain", b"not found")
        }
    }

    /// The page: the emulator, filling it, and the relay.
    fn page(&self) -> String {
        let session = serde_json::json!({
            "token": self.token,
            "load": self.load.kind(),
            "config": serde_json::from_str::<serde_json::Value>(&self.config).unwrap_or_default()
        });
        PAGE.replace("@SESSION@", &session.to_string().replace('<', "\\u003c"))
    }

    /// A request of the API, relayed to the page and its answer back.
    fn api(&self, mut stream: TcpStream, request: Request) -> std::io::Result<()> {
        // what a browser sends: another page on this machine, not the debugger
        if request.headers.contains_key("origin")
            || request
                .headers
                .keys()
                .any(|name| name.starts_with("sec-fetch-"))
        {
            return respond(
                &mut stream,
                403,
                "text/plain",
                b"bemu's relayed API is not for web pages"
            );
        }
        if !self.wait_for_page() {
            let why = match &*self.state.lock().unwrap() {
                PageState::Failed(why) => why.clone(),
                _ => "its page is not open".to_owned()
            };
            return json_error(&mut stream, 503, &format!("bemu is not running: {why}"));
        }
        if request.method == "GET" && request.path == "/api/events" {
            return self.events(stream);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, answer) = channel();
        self.replies.lock().unwrap().insert(id, sender);
        self.queue.lock().unwrap().push_back(Pending {
            id,
            method: request.method,
            path: request.path,
            query: request.raw_query,
            body: request.body
        });
        self.queued.notify_all();
        match answer.recv_timeout(ANSWER_TIMEOUT) {
            Ok(reply) => respond_with(&mut stream, &reply),
            Err(_) => {
                self.replies.lock().unwrap().remove(&id);
                json_error(&mut stream, 504, "bemu's page did not answer")
            }
        }
    }

    /// Wait for the page to be up; false if it failed or went.
    fn wait_for_page(&self) -> bool {
        let deadline = Instant::now() + PAGE_LOAD_TIMEOUT;
        let mut state = self.state.lock().unwrap();
        while *state == PageState::Waiting && !self.stopped.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            state = self
                .state_changed
                .wait_timeout(state, deadline - now)
                .unwrap()
                .0;
        }
        *state == PageState::Ready && !self.stopped.load(Ordering::Relaxed)
    }

    /// The page's long poll: the requests waiting, as JSON, or nothing (204)
    /// after a while.
    fn next(&self, mut stream: TcpStream) -> std::io::Result<()> {
        {
            let mut polling = self.polling.lock().unwrap();
            *polling = (Instant::now(), polling.1 + 1);
        }
        let deadline = Instant::now() + POLL_HOLD;
        let taken: Vec<Pending> = {
            let mut queue = self.queue.lock().unwrap();
            while queue.is_empty() && !self.stopped.load(Ordering::Relaxed) {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                queue = self.queued.wait_timeout(queue, deadline - now).unwrap().0;
            }
            queue.drain(..).collect()
        };
        {
            let mut polling = self.polling.lock().unwrap();
            *polling = (Instant::now(), polling.1.saturating_sub(1));
        }
        if taken.is_empty() {
            return respond(&mut stream, 204, "text/plain", b"");
        }
        let requests: Vec<serde_json::Value> = taken
            .iter()
            .map(|pending| {
                serde_json::json!({
                    "id": pending.id,
                    "method": pending.method,
                    "path": pending.path,
                    "query": pending.query,
                    "body": pending.body
                })
            })
            .collect();
        let written = respond(
            &mut stream,
            200,
            "application/json",
            serde_json::Value::from(requests.clone())
                .to_string()
                .as_bytes()
        );
        if written.is_err() {
            // the page did not get them: they wait for its next poll
            let mut queue = self.queue.lock().unwrap();
            for pending in taken.into_iter().rev() {
                queue.push_front(pending);
            }
        }
        written
    }

    /// The page's answer to request `id`: its status, type and headers in
    /// the query, its body the body.
    fn reply(&self, request: &Request) {
        let Some(id) = request
            .query
            .get("id")
            .and_then(|id| id.parse::<u64>().ok())
        else {
            return;
        };
        let Some(sender) = self.replies.lock().unwrap().remove(&id)
        else {
            return; // answered too late
        };
        let headers = request
            .query
            .get("headers")
            .and_then(|text| serde_json::from_str::<Vec<(String, String)>>(text).ok())
            .unwrap_or_default();
        let _ = sender.send(Reply {
            status: request
                .query
                .get("status")
                .and_then(|status| status.parse().ok())
                .unwrap_or(500),
            content_type: request
                .query
                .get("type")
                .cloned()
                .unwrap_or_else(|| "application/json".to_owned()),
            headers,
            body: request.body.clone()
        });
    }

    /// `/api/events`: the events the page posts, as bemu streams them, with
    /// a heartbeat that notices a reader gone.
    fn events(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let (sender, events): (Sender<String>, Receiver<String>) = channel();
        self.subscribers.lock().unwrap().push(sender);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\n"
        )?;
        stream.flush()?;
        loop {
            let chunk = match events.recv_timeout(Duration::from_secs(5)) {
                Ok(chunk) => chunk,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => ": heartbeat\n\n".to_owned(),
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break
            };
            if stream.write_all(chunk.as_bytes()).is_err() || stream.flush().is_err() {
                break;
            }
        }
        let _ = stream.shutdown(std::net::Shutdown::Both);
        Ok(())
    }
}

fn json_error(stream: &mut TcpStream, status: u16, message: &str) -> std::io::Result<()> {
    respond(
        stream,
        status,
        "application/json",
        serde_json::json!({ "error": message })
            .to_string()
            .as_bytes()
    )
}

/// An answer as the page gave it.
fn respond_with(stream: &mut TcpStream, reply: &Reply) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        reply.status,
        if reply.status < 400 { "OK" } else { "Error" },
        single_line(&reply.content_type),
        reply.body.len()
    );
    for (name, value) in &reply.headers {
        let name = single_line(name);
        if [
            "content-type",
            "content-length",
            "connection",
            "transfer-encoding"
        ]
        .contains(&name.to_ascii_lowercase().as_str())
        {
            continue;
        }
        head.push_str(&format!("{name}: {}\r\n", single_line(value)));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(&reply.body)?;
    stream.flush()
}

/// No header splitting through what the page says.
fn single_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}

/// The page. `@SESSION@` is the session's JSON: its token, what to load,
/// the machine's configuration.
const PAGE: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>bemu</title>
<style>
  html, body { margin: 0; padding: 0; height: 100%; background: #000; overflow: hidden; }
  #screen { position: absolute; inset: 0; }
  #status { position: absolute; left: 0; right: 0; bottom: 0; padding: 4px 8px;
            font: 13px system-ui, sans-serif; color: #ddd; background: rgba(0, 0, 0, 0.7); }
  #status:empty { display: none; }
</style>
<script>window.__cpclib_bemu = @SESSION@;</script>
</head>
<body>
<div id="screen"></div>
<div id="status">Starting bemu...</div>
<script type="module">
  const session = window.__cpclib_bemu;
  const token = 'token=' + encodeURIComponent(session.token);
  const status = document.getElementById('status');
  const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
  const post = (path, body) => fetch(path + (path.includes('?') ? '&' : '?') + token, { method: 'POST', body });

  async function fail(error) {
    const message = String(error && error.message ? error.message : error);
    status.textContent = 'bemu could not start: ' + message;
    await post('/session/failed', message).catch(() => {});
  }

  let emu;
  try {
    const { Bemu } = await import('./bemu/bemu.js');
    emu = await Bemu.create(document.getElementById('screen'), session.config || {}, { autoStart: false, fullscreenKey: null });
    if (typeof emu.api !== 'function' || typeof emu.debugEvents !== 'function') {
      throw new Error('this web build of bemu has no debug API; set CPCLIB_BEMU_WEB to a newer one');
    }
    if (session.load !== 'boot') {
      const answer = await fetch('/session/program?' + token);
      if (!answer.ok) { throw new Error('the program could not be read (' + answer.status + ')'); }
      const bytes = new Uint8Array(await answer.arrayBuffer());
      if (session.load === 'snapshot') {
        await emu.loadSnapshot(bytes);
        // held until the debugger has armed its breakpoints, as `--debug-paused`
        await emu.api('POST', '/api/config', '', new TextEncoder().encode('{"paused":true}'));
      }
      else {
        await emu.insertDisc(0, bytes);
      }
    }
    await emu.debugEvents(); // subscribes: the debugger sees every instruction from now on
    emu.start();
  }
  catch (error) {
    await fail(error);
    throw error;
  }

  // the sound needs a gesture
  const audio = () => { emu.enableAudio?.(); };
  window.addEventListener('pointerdown', audio, { once: true });
  window.addEventListener('keydown', audio, { once: true });

  async function answer(request) {
    let reply;
    try {
      reply = await emu.api(request.method, request.path, request.query, new Uint8Array(request.body));
    }
    catch (error) {
      reply = { status: 500, contentType: 'application/json', headers: [],
                body: new TextEncoder().encode(JSON.stringify({ error: String(error) })) };
    }
    const query = 'id=' + request.id + '&status=' + reply.status
      + '&type=' + encodeURIComponent(reply.contentType || 'application/json')
      + '&headers=' + encodeURIComponent(JSON.stringify(reply.headers || []));
    await post('/session/reply?' + query, reply.body).catch(() => {});
  }

  async function relay() {
    for (;;) {
      let requests = [];
      try {
        const polled = await fetch('/session/next?' + token);
        if (polled.status === 200) { requests = await polled.json(); }
        else if (polled.status !== 204) { await sleep(500); }
      }
      catch (_) { await sleep(500); continue; }
      for (const request of requests) { answer(request); }
    }
  }

  async function events() {
    for (;;) {
      try {
        const pending = await emu.debugEvents();
        if (pending.length) { await post('/session/events', pending.join('')); }
      }
      catch (_) {}
      await sleep(40);
    }
  }

  relay();
  events();
  await post('/session/ready', '');
  status.textContent = '';
  emu.canvas.focus();
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read};

    use super::*;

    fn call(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).unwrap();
        answer
    }

    fn package() -> camino_tempfile::Utf8TempDir {
        let folder = camino_tempfile::tempdir().unwrap();
        fs_err::write(
            folder.path().join("bemu.js"),
            "export class Bemu { debugEvents() {} }"
        )
        .unwrap();
        folder
    }

    /// A page stand-in: answers every request with its path, and posts an event.
    fn fake_page(page: &BemuPage) -> std::thread::JoinHandle<()> {
        let port = page.port();
        let token = page.relay.token.clone();
        std::thread::spawn(move || {
            call(
                port,
                &format!("POST /session/ready?token={token} HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
            );
            for _ in 0..20 {
                let answer = call(
                    port,
                    &format!("GET /session/next?token={token} HTTP/1.1\r\n\r\n")
                );
                let Some(body) = answer
                    .split("\r\n\r\n")
                    .nth(1)
                    .filter(|body| !body.is_empty())
                else {
                    continue;
                };
                let requests: Vec<serde_json::Value> = serde_json::from_str(body).unwrap();
                for request in requests {
                    let reply = format!(
                        "{{\"path\":{},\"query\":{},\"body\":{}}}",
                        request["path"], request["query"], request["body"]
                    );
                    call(
                        port,
                        &format!(
                            "POST /session/reply?token={token}&id={}&status=200&type=application%2Fjson&headers=%5B%5B%22X-Beam-Y%22%2C%2212%22%5D%5D HTTP/1.1\r\nContent-Length: {}\r\n\r\n{reply}",
                            request["id"],
                            reply.len()
                        )
                    );
                    let event = "event: frame\ndata: {}\n\n";
                    call(
                        port,
                        &format!(
                            "POST /session/events?token={token} HTTP/1.1\r\nContent-Length: {}\r\n\r\n{event}",
                            event.len()
                        )
                    );
                }
            }
        })
    }

    #[test]
    fn the_api_is_answered_by_the_page() {
        let package = package();
        let page = serve(package.path(), Load::Boot, serde_json::json!({})).unwrap();
        let _page_thread = fake_page(&page);
        page.wait_until_ready(Duration::from_secs(5)).unwrap();

        let answer = call(
            page.port(),
            "POST /api/ram?addr=%26C000 HTTP/1.1\r\nContent-Length: 2\r\n\r\nhi"
        );
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        assert!(
            answer.contains("x-beam-y: 12") || answer.contains("X-Beam-Y: 12"),
            "{answer}"
        );
        let body = answer.split("\r\n\r\n").nth(1).unwrap();
        let body: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["path"], "/api/ram");
        assert_eq!(
            body["query"], "addr=%26C000",
            "relayed as sent, the page decodes it"
        );
        assert_eq!(body["body"], serde_json::json!([104, 105]));
    }

    #[test]
    fn the_events_the_page_posts_are_streamed() {
        let package = package();
        let page = serve(package.path(), Load::Boot, serde_json::json!({})).unwrap();
        let _page_thread = fake_page(&page);
        page.wait_until_ready(Duration::from_secs(5)).unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", page.port())).unwrap();
        stream
            .write_all(b"GET /api/events HTTP/1.1\r\n\r\n")
            .unwrap();
        let mut reader = BufReader::new(stream);
        // a request for the page to answer, for it to post an event
        std::thread::spawn({
            let port = page.port();
            move || {
                std::thread::sleep(Duration::from_millis(200));
                call(port, "GET /api/ping HTTP/1.1\r\n\r\n");
            }
        });
        let mut line = String::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line.starts_with("event: frame") {
                return;
            }
        }
        panic!("no event streamed");
    }

    #[test]
    fn a_browser_cannot_drive_the_api() {
        let package = package();
        let page = serve(package.path(), Load::Boot, serde_json::json!({})).unwrap();
        let answer = call(
            page.port(),
            "POST /api/quit HTTP/1.1\r\nOrigin: http://evil.example\r\n\r\n"
        );
        assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
        let answer = call(
            page.port(),
            "GET /api/ram HTTP/1.1\r\nSec-Fetch-Site: cross-site\r\n\r\n"
        );
        assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    }

    #[test]
    fn the_session_routes_need_the_token() {
        let package = package();
        let page = serve(
            package.path(),
            Load::Snapshot(vec![1, 2, 3]),
            serde_json::json!({})
        )
        .unwrap();
        let answer = call(page.port(), "GET /session/program HTTP/1.1\r\n\r\n");
        assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
        let token = page.relay.token.clone();
        let answer = call(
            page.port(),
            &format!("GET /session/program?token={token} HTTP/1.1\r\n\r\n")
        );
        assert!(
            answer.starts_with("HTTP/1.1 200") && answer.ends_with("\u{1}\u{2}\u{3}"),
            "{answer:?}"
        );
    }

    #[test]
    fn the_page_carries_the_session_and_serves_the_build() {
        let package = package();
        let page = serve(
            package.path(),
            Load::Snapshot(vec![]),
            serde_json::json!({ "model": "6128" })
        )
        .unwrap();
        let answer = call(page.port(), "GET /debug HTTP/1.1\r\n\r\n");
        assert!(
            answer.contains(&page.relay.token),
            "the token is in the page"
        );
        assert!(answer.contains("\"load\":\"snapshot\""));
        assert!(answer.contains("\"model\":\"6128\""));
        let answer = call(page.port(), "GET /bemu/bemu.js HTTP/1.1\r\n\r\n");
        assert!(
            answer.starts_with("HTTP/1.1 200") && answer.contains("text/javascript"),
            "{answer}"
        );
        let answer = call(page.port(), "GET /bemu/../secret HTTP/1.1\r\n\r\n");
        assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    }

    #[test]
    fn a_page_that_fails_says_why() {
        let package = package();
        let page = serve(package.path(), Load::Boot, serde_json::json!({})).unwrap();
        let token = page.relay.token.clone();
        call(
            page.port(),
            &format!(
                "POST /session/failed?token={token} HTTP/1.1\r\nContent-Length: 6\r\n\r\nno gpu"
            )
        );
        let error = page.wait_until_ready(Duration::from_secs(5)).unwrap_err();
        assert!(error.contains("no gpu"), "{error}");
        let answer = call(page.port(), "GET /api/ping HTTP/1.1\r\n\r\n");
        assert!(
            answer.starts_with("HTTP/1.1 503") && answer.contains("no gpu"),
            "{answer}"
        );
    }

    #[test]
    fn a_page_gone_closes_the_api() {
        let package = package();
        let page = serve(package.path(), Load::Boot, serde_json::json!({})).unwrap();
        let token = page.relay.token.clone();
        call(
            page.port(),
            &format!("POST /session/ready?token={token} HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
        );
        // the page never polls again: it is gone after a while
        *page.relay.polling.lock().unwrap() = (Instant::now() - PAGE_GONE_AFTER * 2, 0);
        assert!(page.page_gone());
        let _ = TcpStream::connect(("127.0.0.1", page.port())); // wakes the listener up
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", page.port())).is_err() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("the relay still listens");
    }

    #[test]
    fn an_archive_is_unpacked_in_place_of_the_old_install() {
        let root = camino_tempfile::tempdir().unwrap();
        let install = root.path().join("bemu_web_latest");
        fs_err::create_dir_all(install.join("pkg")).unwrap();
        fs_err::write(install.join("pkg/old.js"), "old").unwrap();

        let mut archive = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut archive));
            writer
                .start_file("pkg/bemu.js", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"debugEvents").unwrap();
            writer.finish().unwrap();
        }
        unpack(&archive, &install).unwrap();
        assert!(has_debug_api(&install.join("pkg")));
        assert!(!install.join("pkg/old.js").exists());
    }
}

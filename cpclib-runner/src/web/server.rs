//! Serving the emulator, and carrying DAP to and from it.
//!
//! The page needs three things from us: its own files, the snapshot to run, and
//! a two-way channel for the Debug Adapter Protocol. That is a small enough
//! surface to serve directly over `std::net`, which is why there is no web
//! framework here and no cargo feature to gate one - `cpclib-runner` is
//! depended on by nearly everything in the workspace, and adding an async
//! runtime to all of it for four routes would be a poor trade.
//!
//! The DAP channel is **Server-Sent Events downstream and POST upstream**
//! rather than a WebSocket. Both directions are then plain HTTP, so the whole
//! transport is a few lines of framing instead of a handshake with a SHA-1
//! digest and a masked binary frame format. For a request/response protocol
//! with events, that is exactly the shape needed.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

use super::mime_for;

/// A running server.
pub struct ServerHandle {
    address: SocketAddr,
    token: String,
    outgoing: Sender<String>,
    incoming: Receiver<String>,
    disconnected: Arc<AtomicBool>
}

impl ServerHandle {
    pub fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    /// The URL to open for a debug session.
    ///
    /// A distinct *path* rather than a query string, and deliberately so: this
    /// URL travels through an editor before it reaches a browser - VS Code
    /// parses it into a `Uri` and re-serialises it for a webview - and a
    /// percent-encoded `?dap=1&token=...` arrives as something the page cannot
    /// read. With no `?` and no `&` there is nothing to mangle, and the token
    /// is injected into the page by the server instead of carried in the URL.
    pub fn debug_url(&self) -> String {
        format!("{}/debug", self.base_url())
    }

    /// The URL to open for ordinary use - no debugger, full emulator UI.
    pub fn plain_url(&self) -> String {
        format!("{}/", self.base_url())
    }

    pub fn port(&self) -> u16 {
        self.address.port()
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Send a DAP frame to the emulator.
    pub fn send(&self, frame: String) -> Result<(), String> {
        self.outgoing.send(frame).map_err(|e| e.to_string())
    }

    /// A frame from the emulator, if one is waiting.
    pub fn try_recv(&self) -> Option<String> {
        self.incoming.try_recv().ok()
    }

    pub fn incoming(&self) -> &Receiver<String> {
        &self.incoming
    }

    /// Whether the browser tab holding the event stream has gone away.
    ///
    /// Reported live: closing the tab left a debug session sitting in the
    /// editor forever, because nothing here ever said anything about it. Set
    /// once, from the SSE connection's own write failure - see
    /// `event_stream`'s own comment for why a write failing is exactly what
    /// "the tab closed" looks like from this side. There is no un-setting
    /// it: a session only ever serves one event-stream connection for its
    /// whole life (`Site::outgoing` is taken once - see `event_stream`), so
    /// "gone" is not a state that reconnects.
    pub fn client_gone(&self) -> bool {
        self.disconnected.load(Ordering::Relaxed)
    }
}

struct Site {
    root: Utf8PathBuf,
    snapshot: Mutex<Option<Vec<u8>>>,
    token: String,
    /// Handed to the one event-stream connection; `None` once taken.
    outgoing: Mutex<Option<Receiver<String>>>,
    from_page: Sender<String>,
    disconnected: Arc<AtomicBool>
}

/// Start serving `root` on a loopback port the OS chooses.
///
/// A random per-session token is required on every `/session/*` route.
/// Loopback is not a boundary - any page in any browser on this machine can
/// reach `127.0.0.1` - and without the token one of them could drive the
/// debugger and read the snapshot.
pub fn serve(root: &Utf8Path, snapshot: Option<Vec<u8>>) -> std::io::Result<ServerHandle> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let token = random_token();

    let (to_page, page_receiver) = channel::<String>();
    let (from_page, page_sender) = channel::<String>();
    let disconnected = Arc::new(AtomicBool::new(false));

    let site = Arc::new(Site {
        root: root.to_path_buf(),
        snapshot: Mutex::new(snapshot),
        token: token.clone(),
        outgoing: Mutex::new(Some(page_receiver)),
        from_page,
        disconnected: Arc::clone(&disconnected)
    });

    std::thread::Builder::new()
        .name("cpclib-web".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let site = Arc::clone(&site);
                // One thread per connection: there are at most a handful - the
                // page, its assets, and one long-lived event stream.
                let _ = std::thread::Builder::new()
                    .name("cpclib-web-conn".into())
                    .spawn(move || {
                        let _ = handle(stream, &site);
                    });
            }
        })?;

    Ok(ServerHandle {
        address,
        token,
        outgoing: to_page,
        incoming: page_sender,
        disconnected
    })
}

/// Enough randomness that another local page cannot guess it.
fn random_token() -> String {
    use std::hash::{BuildHasher, Hasher, RandomState};
    let mut token = String::with_capacity(64);
    for round in 0..4u64 {
        // `RandomState` is seeded per instance by the OS; four of them give a
        // 256-bit token without pulling in an RNG crate.
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(round);
        hasher.write_usize(&token as *const String as usize);
        token.push_str(&format!("{:016x}", hasher.finish()));
    }
    token
}

struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body: Vec<u8>
}

fn handle(mut stream: TcpStream, site: &Site) -> std::io::Result<()> {
    let Some(request) = read_request(&mut stream)?
    else {
        return Ok(());
    };

    let authorised = request.query.get("token").map(String::as_str) == Some(site.token.as_str());

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/debug") => serve_debug_page(&mut stream, site),
        ("GET", "/session/events") if authorised => event_stream(stream, site),
        ("POST", "/session/dap") if authorised => {
            let frame = String::from_utf8_lossy(&request.body).to_string();
            let _ = site.from_page.send(frame);
            respond(&mut stream, 204, "text/plain", b"")
        },
        ("GET", "/session/snapshot.sna") if authorised => {
            let snapshot = site.snapshot.lock().unwrap();
            match snapshot.as_ref() {
                Some(bytes) => respond(&mut stream, 200, "application/octet-stream", bytes),
                None => {
                    respond(
                        &mut stream,
                        404,
                        "text/plain",
                        b"no snapshot in this session"
                    )
                },
            }
        },
        (_, path) if path.starts_with("/session/") => {
            respond(&mut stream, 403, "text/plain", b"missing or wrong token")
        },
        ("GET", path) => serve_file(&mut stream, site, path),
        _ => respond(&mut stream, 405, "text/plain", b"method not allowed")
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or("/").to_string();

    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body)?;
    }

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), parse_query(query)),
        None => (target, HashMap::new())
    };

    Ok(Some(Request {
        method,
        path,
        query,
        body
    }))
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_string(), percent_decode(v)))
        .collect()
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    },
                    Err(_) => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            },
            b'+' => {
                out.push(b' ');
                index += 1;
            },
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The emulator page, with this session's credentials injected.
///
/// The token has to reach the page somehow, and putting it in the URL means
/// trusting every layer in between to preserve a query string. Injecting it
/// into the document the server itself produces removes that dependency, and
/// keeps the token out of the address bar.
/// Everything but the pixels, hidden.
///
/// The emulator's own page is a whole workbench - machine selector, LEDs, tape
/// deck, keyboard, monitor panel. In a browser that is the point; in an editor
/// tab beside your source it is a screenful of chrome around a small picture,
/// and every one of those controls has a better equivalent in the debugger.
///
/// Hidden rather than removed: `app.js` looks its elements up by id and would
/// fault on a missing one, so `display: none` is the only safe way to do this
/// without patching the page's own logic. Each rule un-hides exactly the
/// ancestors of the canvas and hides their siblings, which is why the chain is
/// spelled out rather than reduced to a wildcard.
///
/// Only `/debug` gets this. The plain URL - what a browser opens, and what
/// `emu ... run` serves - keeps the full machine.
const BARE_SCREEN_STYLE: &str = "<style id=\"cpclib-bare\">\n\
    html, body { margin: 0; padding: 0; height: 100%; background: #000; \
                 overflow: hidden; }\n\
    body > *:not(main.workbench):not(#cpclib-audio) { display: none !important; }\n\
    main.workbench > *:not(.receiver) { display: none !important; }\n\
    section.receiver > *:not(#screenStage) { display: none !important; }\n\
    #screenStage > *:not(#screenFrame) { display: none !important; }\n\
    #screenFrame > *:not(.screen-glass) { display: none !important; }\n\
    .screen-glass > *:not(#screen) { display: none !important; }\n\
    main.workbench, section.receiver, #screenStage, #screenFrame, .screen-glass {\n\
      display: block !important; position: static !important;\n\
      margin: 0 !important; padding: 0 !important; border: 0 !important;\n\
      border-radius: 0 !important; box-shadow: none !important;\n\
      background: #000 !important; width: 100% !important; height: 100% !important;\n\
      max-width: none !important; max-height: none !important;\n\
      min-width: 0 !important; min-height: 0 !important; gap: 0 !important;\n\
      transform: none !important; filter: none !important;\n\
    }\n\
    #screen {\n\
      display: block !important; width: 100% !important; height: 100% !important;\n\
      object-fit: contain; image-rendering: pixelated; background: #000 !important;\n\
      border: 0 !important; border-radius: 0 !important; box-shadow: none !important;\n\
    }\n\
    </style>";

fn serve_debug_page(stream: &mut TcpStream, site: &Site) -> std::io::Result<()> {
    let index = site.root.join("index.html");
    let Ok(html) = fs_err::read_to_string(&index)
    else {
        return respond(stream, 404, "text/plain", b"index.html is missing");
    };

    let injected = format!(
        "{BARE_SCREEN_STYLE}\n<script>window.__cpclib_session = {{\"token\": \"{}\"}};</script>\n</head>",
        site.token
    );
    let html = if html.contains("</head>") {
        html.replacen("</head>", &injected, 1)
    }
    else {
        format!("{injected}{html}")
    };
    respond(stream, 200, "text/html; charset=utf-8", html.as_bytes())
}

/// Serve a file from the site root.
///
/// Any `..` is refused outright rather than normalised. This server is
/// reachable by every page in every browser on the machine, and "serve exactly
/// what is in this directory" is a rule worth keeping simple enough to be
/// obviously true.
fn serve_file(stream: &mut TcpStream, site: &Site, path: &str) -> std::io::Result<()> {
    let relative = path.trim_start_matches('/');
    let relative = if relative.is_empty() {
        "index.html"
    }
    else {
        relative
    };

    if relative.contains("..") {
        return respond(stream, 403, "text/plain", b"forbidden");
    }

    let file = site.root.join(relative);
    match fs_err::read(&file) {
        Ok(bytes) => respond(stream, 200, mime_for(&file), &bytes),
        Err(_) => respond(stream, 404, "text/plain", b"not found")
    }
}

/// The downstream half of the DAP channel.
///
/// Held open for the life of the session. Each message is sent as **one SSE
/// `data:` line carrying the JSON body alone**, deliberately *not* the
/// `Content-Length`-framed form.
///
/// Server-Sent Events are line-oriented and strip carriage returns, while the
/// emulator's parser scans for the four bytes `\r\n\r\n` exactly. Sending a
/// framed message therefore delivers `\n\n` to a parser that will never accept
/// it - the conversation simply never starts, silently. A JSON body has no raw
/// newline in it, so it survives one `data:` line intact, and the page rebuilds
/// the frame with a byte-accurate length on arrival.
fn event_stream(mut stream: TcpStream, site: &Site) -> std::io::Result<()> {
    let taken = site.outgoing.lock().unwrap().take();
    let Some(receiver) = taken
    else {
        return respond(
            &mut stream,
            409,
            "text/plain",
            b"an event stream is already connected"
        );
    };

    write!(
        stream,
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-store\r\n\
         Connection: keep-alive\r\n\r\n"
    )?;
    stream.flush()?;

    for message in receiver {
        // One line in, one line out: anything that could break the record
        // apart would break the protocol, so a stray newline is refused rather
        // than sent as something the page would silently mis-parse.
        let single_line = message.replace(['\r', '\n'], " ");
        // A write failing here - the browser tab having closed, most often -
        // is the only way this side ever learns that; SSE has no explicit
        // goodbye. `ServerHandle::client_gone` is how the debug session on
        // the other end of `disconnected` finds out and ends itself, instead
        // of sitting open forever the way it used to (reported live).
        if writeln!(stream, "data: {single_line}").is_err()
            || writeln!(stream).is_err()
            || stream.flush().is_err()
        {
            site.disconnected.store(true, Ordering::Relaxed);
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8]
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        _ => "OK"
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

#[cfg(test)]
mod disconnect_tests {
    use std::io::{BufRead, BufReader};
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    use super::*;

    /// Reported live: closing the emulator's own browser tab left a debug
    /// session sitting open in the editor forever, because nothing here had
    /// ever noticed the tab was gone. `client_gone` is the fix's whole
    /// mechanism - a real client connecting to `/session/events` and then
    /// disappearing must flip it, not just compile.
    #[test]
    fn client_gone_flips_once_the_event_stream_connection_really_closes() {
        let root = Utf8PathBuf::from(std::env::temp_dir().to_string_lossy().to_string());
        let handle = serve(&root, None).expect("serve");
        assert!(!handle.client_gone(), "nothing has connected yet");

        let stream = TcpStream::connect(("127.0.0.1", handle.port())).expect("connect");
        write!(
            &stream,
            "GET /session/events?token={} HTTP/1.1\r\nHost: x\r\n\r\n",
            handle.token()
        )
        .unwrap();
        // Read the status line, so the connection is confirmed genuinely
        // open (not just TCP-accepted) before it is closed again.
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut status_line = String::new();
        reader.read_line(&mut status_line).expect("read status line");
        assert!(status_line.starts_with("HTTP/1.1 200"), "{status_line}");

        drop(stream);
        drop(reader);

        // A message has to actually be *sent* down the stream for its own
        // write to fail and notice the close - see `event_stream`'s own
        // comment for why SSE has no other way to learn this.
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let _ = handle.send("{}".to_string());
            if handle.client_gone() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("client_gone never became true within 5s of the connection closing");
    }
}

#[cfg(test)]
mod debug_page_tests {
    use super::*;

    /// The debug page hides the emulator's furniture and keeps the picture.
    ///
    /// Hidden, never removed: `app.js` looks its elements up by id and faults
    /// on a missing one, so anything that deletes nodes breaks the emulator
    /// rather than tidying it.
    #[test]
    fn the_bare_style_hides_the_chrome_without_removing_it() {
        assert!(BARE_SCREEN_STYLE.contains("display: none"));
        assert!(
            !BARE_SCREEN_STYLE.contains("remove"),
            "nodes are hidden, not deleted"
        );

        // Every ancestor of the canvas is un-hidden by name; miss one and the
        // screen disappears with the rest.
        for ancestor in [
            "main.workbench",
            "section.receiver",
            "#screenStage",
            "#screenFrame",
            ".screen-glass",
            "#screen"
        ] {
            assert!(BARE_SCREEN_STYLE.contains(ancestor), "{ancestor}");
        }

        // ...and the picture keeps its shape and its pixels.
        assert!(BARE_SCREEN_STYLE.contains("object-fit: contain"));
        assert!(BARE_SCREEN_STYLE.contains("image-rendering: pixelated"));
    }
}

#[cfg(test)]
mod bare_style_exemption_tests {
    use super::*;

    /// The one thing on the debug page that is not the screen and must still be
    /// seen: the button that turns the sound on.
    #[test]
    fn the_audio_prompt_survives_the_bare_screen() {
        assert!(
            BARE_SCREEN_STYLE.contains(":not(#cpclib-audio)"),
            "the offer to fix the sound must not be hidden by the tidying"
        );
    }
}

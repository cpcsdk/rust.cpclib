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
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

use super::mime_for;

/// A running server.
pub struct ServerHandle {
    address: SocketAddr,
    token: String,
    outgoing: Sender<String>,
    incoming: Receiver<String>
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
}

struct Site {
    root: Utf8PathBuf,
    snapshot: Mutex<Option<Vec<u8>>>,
    token: String,
    /// Handed to the one event-stream connection; `None` once taken.
    outgoing: Mutex<Option<Receiver<String>>>,
    from_page: Sender<String>
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

    let site = Arc::new(Site {
        root: root.to_path_buf(),
        snapshot: Mutex::new(snapshot),
        token: token.clone(),
        outgoing: Mutex::new(Some(page_receiver)),
        from_page
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
        incoming: page_sender
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
                None => respond(&mut stream, 404, "text/plain", b"no snapshot in this session")
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
fn serve_debug_page(stream: &mut TcpStream, site: &Site) -> std::io::Result<()> {
    let index = site.root.join("index.html");
    let Ok(html) = std::fs::read_to_string(&index)
    else {
        return respond(stream, 404, "text/plain", b"index.html is missing");
    };

    let injected = format!(
        "<script>window.__cpclib_session = {{\"token\": \"{}\"}};</script>\n</head>",
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
    match std::fs::read(&file) {
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
        writeln!(stream, "data: {single_line}")?;
        writeln!(stream)?;
        if stream.flush().is_err() {
            break; // the page went away
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

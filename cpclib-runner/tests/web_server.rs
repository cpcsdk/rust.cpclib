//! The server, against real sockets.

use std::io::{Read, Write};
use std::net::TcpStream;

use cpclib_common::camino::Utf8Path;
use cpclib_runner::web::serve;

/// A site with the two files a request can ask for.
fn site() -> camino_tempfile::Utf8TempDir {
    let tmp = camino_tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("index.html"), "<html><head></head><body>emulator</body></html>").unwrap();
    std::fs::write(tmp.path().join("6128.wasm"), [0u8, 'a' as u8, 's' as u8, 'm' as u8]).unwrap();
    tmp
}

fn request(port: u16, raw: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(raw.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut response = Vec::new();
    // The server closes after answering, except for the event stream.
    let _ = stream.read_to_end(&mut response);
    String::from_utf8_lossy(&response).to_string()
}

#[test]
fn the_index_is_served_at_the_root() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();
    let response = request(server.port(), "GET / HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("emulator"), "{response}");
}

/// The one MIME type upstream insists on.
#[test]
fn wasm_arrives_as_application_wasm() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();
    let response = request(server.port(), "GET /6128.wasm HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(response.contains("Content-Type: application/wasm"), "{response}");
}

/// Loopback is not a boundary: any local page can reach this port, so the
/// session routes must refuse without the token.
#[test]
fn session_routes_refuse_without_the_token() {
    let tmp = site();
    let server = serve(tmp.path(), Some(vec![1, 2, 3])).unwrap();

    let refused = request(
        server.port(),
        "GET /session/snapshot.sna HTTP/1.1\r\nHost: x\r\n\r\n"
    );
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");

    let wrong = request(
        server.port(),
        "GET /session/snapshot.sna?token=guessed HTTP/1.1\r\nHost: x\r\n\r\n"
    );
    assert!(wrong.starts_with("HTTP/1.1 403"), "{wrong}");
}

#[test]
fn the_snapshot_is_served_with_the_token() {
    let tmp = site();
    let server = serve(tmp.path(), Some(vec![0x4d, 0x56])).unwrap();
    let response = request(
        server.port(),
        &format!(
            "GET /session/snapshot.sna?token={} HTTP/1.1\r\nHost: x\r\n\r\n",
            server.token()
        )
    );
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.ends_with("MV"), "the bytes are the body: {response:?}");
}

/// Escaping the site root must not be possible.
#[test]
fn a_traversal_is_refused() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();
    for target in ["/../secret", "/a/../../secret", "/..%2fsecret"] {
        let response = request(
            server.port(),
            &format!("GET {target} HTTP/1.1\r\nHost: x\r\n\r\n")
        );
        assert!(
            response.starts_with("HTTP/1.1 403") || response.starts_with("HTTP/1.1 404"),
            "{target} was not refused: {response}"
        );
    }
}

/// A frame the page POSTs reaches the adapter.
#[test]
fn a_posted_frame_reaches_the_adapter() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();
    let frame = "Content-Length: 2\r\n\r\n{}";
    let response = request(
        server.port(),
        &format!(
            "POST /session/dap?token={} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{frame}",
            server.token(),
            frame.len()
        )
    );
    assert!(response.starts_with("HTTP/1.1 204"), "{response}");

    // Give the handler thread a moment to hand it over.
    for _ in 0..50 {
        if let Some(received) = server.try_recv() {
            assert_eq!(received, frame);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the frame never arrived");
}

/// The debug page carries the session token in the document, not the URL.
///
/// The URL travels through an editor, which parses and re-serialises it; a
/// query string does not reliably survive that. So there must be no `?` in it
/// at all, and the token must arrive in the page instead.
#[test]
fn the_debug_page_injects_the_session_token() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();

    let url = server.debug_url();
    assert!(url.starts_with("http://127.0.0.1:"), "{url}");
    assert!(!url.contains('?'), "nothing for a URI parser to mangle: {url}");
    assert!(!url.contains(server.token()), "the token is not in the URL: {url}");

    let page = request(server.port(), "GET /debug HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(page.contains("__cpclib_session"), "the session is injected: {page}");
    assert!(page.contains(server.token()), "with this session's token");
    assert!(page.contains("emulator"), "and the real page is still there");

    // The plain page is untouched, so ordinary browsing gets no debugger.
    let plain = request(server.port(), "GET / HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(!plain.contains("__cpclib_session"), "{plain}");
}

/// Two sessions must not share a token.
#[test]
fn each_session_gets_its_own_token() {
    let tmp = site();
    let first = serve(tmp.path(), None).unwrap();
    let second = serve(tmp.path(), None).unwrap();
    assert_ne!(first.token(), second.token());
    assert_ne!(first.port(), second.port());
}

fn _unused(_: &Utf8Path) {}

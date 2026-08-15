//! The real install: download the pinned distribution and patch it.
//!
//! Ignored by default because it hits the network and writes to the shared
//! cache. Run it when the pin moves - it is what proves the pin, the file list
//! and the patch still agree with upstream.
//!
//! ```text
//! cargo test -p cpclib-runner --test js1984_install -- --ignored --nocapture
//! ```

#[test]
#[ignore = "downloads the pinned 1984js distribution"]
fn the_pinned_distribution_installs_and_serves() {
    use cpclib_runner::web::{js1984, serve};

    let root = js1984::install().expect("the pinned distribution must install");
    for name in js1984::DIST_FILES {
        assert!(root.join(name).exists(), "{name} was not downloaded");
    }
    assert!(js1984::is_installed(), "and it reports itself installed");

    // The patch went in.
    let app = std::fs::read_to_string(root.join("app.js")).unwrap();
    assert!(app.contains("__cpclib_attach"));

    // ...and the result is servable, with the wasm typed correctly.
    let server = serve(&root, None).expect("serves");
    let response = fetch(server.port(), "/6128.wasm");
    assert!(response.contains("Content-Type: application/wasm"), "{response}");

    // A second install is a no-op rather than a re-download.
    let again = js1984::install().expect("idempotent");
    assert_eq!(again, root);
}

fn fetch(port: u16, path: &str) -> String {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(stream, "GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
    stream.flush().unwrap();
    let mut buffer = Vec::new();
    let _ = stream.read_to_end(&mut buffer);
    String::from_utf8_lossy(&buffer[..buffer.len().min(400)]).to_string()
}

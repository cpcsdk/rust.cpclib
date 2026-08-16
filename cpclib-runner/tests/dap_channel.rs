//! The DAP channel, exercised the way the page really uses it.
//!
//! This exists because of a bug no unit test could see: the downstream half is
//! Server-Sent Events, which strips carriage returns, while the emulator's
//! parser scans for `\r\n\r\n` as *bytes*. Framed messages therefore arrived as
//! `\n\n` and were never parsed - the conversation silently never started.
//! What has to hold is a round trip, not a shape.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use cpclib_runner::web::serve;

fn site() -> camino_tempfile::Utf8TempDir {
    let tmp = camino_tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("index.html"),
        "<html><head></head><body>x</body></html>"
    )
    .unwrap();
    tmp
}

/// Open the event stream and hand back a reader positioned after the headers.
fn open_event_stream(port: u16, token: &str) -> BufReader<TcpStream> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET /session/events?token={token} HTTP/1.1\r\nHost: x\r\nAccept: text/event-stream\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();

    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            return reader; // end of the response headers
        }
    }
}

/// The next `data:` payload from the stream.
fn next_event(reader: &mut BufReader<TcpStream>) -> String {
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if let Some(payload) = line.trim_end_matches(['\r', '\n']).strip_prefix("data: ") {
            return payload.to_string();
        }
    }
}

/// A message sent to the page must arrive as **one** line, and must rebuild
/// into a frame the emulator's parser accepts: `Content-Length: N\r\n\r\n` with
/// N counting bytes.
#[test]
fn a_message_survives_the_trip_to_the_page_as_a_parseable_frame() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();
    let mut reader = open_event_stream(server.port(), server.token());

    let body = r#"{"seq":1,"type":"request","command":"attach","arguments":{}}"#;
    server.send(body.to_string()).unwrap();

    let received = next_event(&mut reader);
    assert_eq!(received, body, "the body arrives intact, on one line");

    // What the page does with it, byte for byte.
    let frame = format!("Content-Length: {}\r\n\r\n{received}", received.len());
    let separator = frame
        .find("\r\n\r\n")
        .expect("the frame has a CRLF CRLF separator");
    let declared: usize = frame[..separator]
        .strip_prefix("Content-Length: ")
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        declared,
        frame[separator + 4..].len(),
        "the declared length matches the body"
    );
}

/// Non-ASCII must not desynchronise the stream: `Content-Length` counts bytes,
/// so a page counting characters would truncate every following message.
#[test]
fn a_message_with_non_ascii_keeps_its_byte_length() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();
    let mut reader = open_event_stream(server.port(), server.token());

    let body = r#"{"seq":2,"type":"event","event":"output","body":{"output":"café ✓"}}"#;
    server.send(body.to_string()).unwrap();

    let received = next_event(&mut reader);
    assert_eq!(received, body);
    assert!(
        received.len() > received.chars().count(),
        "the fixture really is multi-byte, or it proves nothing"
    );
}

/// Several messages stay separate records rather than running together.
#[test]
fn messages_arrive_one_per_record() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();
    let mut reader = open_event_stream(server.port(), server.token());

    for seq in 1..=3 {
        server
            .send(format!(
                r#"{{"seq":{seq},"type":"request","command":"threads"}}"#
            ))
            .unwrap();
    }
    for seq in 1..=3 {
        let received = next_event(&mut reader);
        assert!(received.contains(&format!(r#""seq":{seq}"#)), "{received}");
    }
}

/// The upstream half: what the page POSTs reaches the adapter unchanged,
/// framing and all.
#[test]
fn a_framed_reply_from_the_page_reaches_the_adapter() {
    let tmp = site();
    let server = serve(tmp.path(), None).unwrap();

    let body = r#"{"seq":9,"type":"event","event":"stopped","body":{"reason":"breakpoint"}}"#;
    let frame = format!("Content-Length: {}\r\n\r\n{body}", body.len());

    let mut stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
    write!(
        stream,
        "POST /session/dap?token={} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{frame}",
        server.token(),
        frame.len()
    )
    .unwrap();
    stream.flush().unwrap();
    let mut sink = Vec::new();
    let _ = stream.read_to_end(&mut sink);

    for _ in 0..100 {
        if let Some(received) = server.try_recv() {
            assert_eq!(received, frame, "the frame arrives byte for byte");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the stopped event never reached the adapter");
}

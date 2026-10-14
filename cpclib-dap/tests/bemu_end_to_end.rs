//! The adapter driving a real bemu: the whole chain an editor uses, from
//! `launch` to the emulator exiting with the session.
//!
//! Needs bemu's `amsrust` executable, named by `$CPCLIB_BEMU`; without it
//! each test says so and passes. bemu is built from its own repository
//! (`cargo +nightly build --release` in rgiot/bemu).

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// One bemu at a time: each looks for a free port, and two looking at
/// once could pick the same.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn bemu() -> Option<String> {
    match std::env::var("CPCLIB_BEMU") {
        Ok(path) if Path::new(&path).is_file() => Some(path),
        _ => {
            eprintln!("skipped: set $CPCLIB_BEMU to bemu's amsrust executable to run this test");
            None
        }
    }
}

/// The adapter, spoken to over stdio as an editor would.
struct Adapter {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    seq: i64,
    /// Everything received, for the failure messages.
    seen: Vec<Value>,
    /// Received while waiting for something else: a stop can come before
    /// the response to the request that caused it.
    backlog: Vec<Value>,
    _serial: MutexGuard<'static, ()>
}

const TIMEOUT: Duration = Duration::from_secs(60);

impl Adapter {
    fn start() -> Self {
        let serial = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut child = Command::new(env!("CARGO_BIN_EXE_cpclib-dap"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the adapter starts");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, messages) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut length = None;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("Content-Length:") {
                        length = value.trim().parse::<usize>().ok();
                    }
                }
                let Some(length) = length
                else {
                    continue;
                };
                let mut body = vec![0; length];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                if let Ok(value) = serde_json::from_slice(&body) {
                    if sender.send(value).is_err() {
                        return;
                    }
                }
            }
        });
        Adapter { child, stdin, messages, seq: 0, seen: Vec::new(), backlog: Vec::new(), _serial: serial }
    }

    fn send(&mut self, command: &str, arguments: Value) -> i64 {
        self.seq += 1;
        let body = json!({ "seq": self.seq, "type": "request", "command": command, "arguments": arguments }).to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        self.stdin.flush().unwrap();
        self.seq
    }

    /// The first message `wanted` accepts, received or to come; the others
    /// wait in the backlog.
    fn wait(&mut self, what: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        if let Some(position) = self.backlog.iter().position(&wanted) {
            return self.backlog.remove(position);
        }
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.messages.recv_timeout(left) {
                Ok(message) => {
                    self.seen.push(message.clone());
                    if wanted(&message) {
                        return message;
                    }
                    self.backlog.push(message);
                },
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    panic!("no {what}; received:\n{}", self.transcript())
                }
            }
        }
    }

    fn transcript(&self) -> String {
        self.seen.iter().map(|m| m.to_string().chars().take(300).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    /// Send a request, return its successful response.
    fn request(&mut self, command: &str, arguments: Value) -> Value {
        let seq = self.send(command, arguments);
        let response = self.wait(&format!("response to {command}"), |m| {
            m["type"] == "response" && m["request_seq"] == seq
        });
        assert_eq!(response["success"], true, "{command}: {response}\n{}", self.transcript());
        response
    }

    fn stopped(&mut self) -> Value {
        self.wait("stopped event", |m| m["event"] == "stopped")
    }

    /// The top frame's line.
    fn line(&mut self) -> i64 {
        let trace = self.request("stackTrace", json!({ "threadId": 1 }));
        trace["body"]["stackFrames"][0]["line"].as_i64().unwrap_or(-1)
    }

    /// A register from the Registers scope.
    fn register(&mut self, name: &str) -> String {
        self.variable(name, "Registers")
    }

    fn variable(&mut self, name: &str, scope_name: &str) -> String {
        let scopes = self.request("scopes", json!({ "frameId": 0 }));
        let scope = scopes["body"]["scopes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"].as_str().is_some_and(|n| n.contains(scope_name)))
            .unwrap_or_else(|| panic!("no {scope_name} scope: {scopes}"))
            .clone();
        let variables = self.request("variables", json!({ "variablesReference": scope["variablesReference"] }));
        variables["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap_or_else(|| panic!("no {name} in {variables}"))["value"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn launch(&mut self, program: &Path, bemu: &str, extra: Value) {
        self.request("initialize", json!({ "adapterID": "cpclib", "linesStartAt1": true, "columnsStartAt1": true }));
        let mut arguments = json!({
            "program": program,
            "emulator": "bemu",
            "emulatorPath": bemu,
            "headless": true
        });
        for (key, value) in extra.as_object().unwrap() {
            arguments[key] = value.clone();
        }
        self.request("launch", arguments);
    }

    /// Disconnect; the emulator must go with the session.
    fn finish(mut self) {
        self.request("disconnect", json!({}));
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.child.kill();
        panic!("the adapter did not exit after disconnect");
    }
}

impl Drop for Adapter {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// A program in a fresh directory.
fn program(name: &str, source: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("cpclib-dap-bemu-e2e-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, source).unwrap();
    path
}

const LOOP: &str = "\torg #4000\n\trun start\nstart\n\tld a,1\nloop\n\tinc a\n\tld (#c000),a\n\tjr loop\n";
/// Lines of `LOOP` (1-based).
const LINE_LD_A_1: i64 = 4;
const LINE_INC_A: i64 = 6;
const LINE_LD_C000: i64 = 7;

fn hex_value(text: &str) -> i64 {
    let digits: String = text.chars().skip_while(|c| !c.is_ascii_hexdigit()).take_while(|c| c.is_ascii_alphanumeric()).collect();
    let digits = digits.trim_start_matches("0x").trim_start_matches('#').trim_start_matches('&');
    i64::from_str_radix(digits.trim_end_matches('h'), 16).unwrap_or_else(|_| panic!("not a value: {text}"))
}

#[test]
fn stop_on_entry_then_continue_to_a_breakpoint_then_step() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({ "stopOnEntry": true }));
    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": [{ "line": LINE_INC_A }] }));
    adapter.request("configurationDone", json!({}));

    let entry = adapter.stopped();
    assert_eq!(entry["body"]["reason"], "entry", "{entry}");
    assert_eq!(adapter.line(), LINE_LD_A_1);

    adapter.request("continue", json!({ "threadId": 1 }));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    assert_eq!(adapter.line(), LINE_INC_A);
    assert_eq!(hex_value(&adapter.register("A")), 1, "ld a,1 ran, inc a not yet");

    adapter.request("next", json!({ "threadId": 1 }));
    let step = adapter.stopped();
    assert_eq!(step["body"]["reason"], "step", "{step}");
    assert_eq!(adapter.line(), LINE_LD_C000);
    assert_eq!(hex_value(&adapter.register("A")), 2);

    // The emulator's memory, as the editor reads it.
    adapter.request("next", json!({ "threadId": 1 }));
    adapter.stopped();
    let memory = adapter.request("readMemory", json!({ "memoryReference": "0xC000", "count": 1 }));
    assert_eq!(memory["body"]["data"], "Ag==", "2 written at #C000: {memory}");
    adapter.finish();
}

#[test]
fn breakpoints_are_armed_before_the_program_runs() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": [{ "line": LINE_INC_A }] }));
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    assert_eq!(adapter.line(), LINE_INC_A);
    assert_eq!(hex_value(&adapter.register("A")), 1, "the loop's first pass");
    adapter.finish();
}

#[test]
fn a_basic_program_stops_on_its_line_with_its_variables() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.bas", "10 A=1\n20 A=A+1\n30 GOTO 20\n");
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": [{ "line": 2 }] }));
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    assert_eq!(adapter.line(), 2);
    let a = adapter.variable("A", "Variables");
    assert!(a.starts_with("1 "), "A on the first pass: {a}");

    adapter.request("continue", json!({ "threadId": 1 }));
    adapter.stopped();
    let a = adapter.variable("A", "Variables");
    assert!(a.starts_with("2 "), "A on the second pass: {a}");
    adapter.finish();
}

#[test]
fn the_frame_view_shows_the_emulators_picture_and_beam() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": [{ "line": LINE_INC_A }] }));
    adapter.request("configurationDone", json!({}));
    adapter.stopped();

    let png_size = |frame: &Value| (frame["body"]["width"].as_u64().unwrap(), frame["body"]["height"].as_u64().unwrap());
    adapter.request("evaluate", json!({ "expression": "-frame", "context": "repl" }));
    let frame = adapter.wait("frame view", |m| m["event"] == "cpclib/frameView");
    assert_eq!(png_size(&frame), (768, 272), "the visible area");
    assert!(frame["body"]["png"].as_str().unwrap().starts_with("iVBOR"), "a PNG");
    assert!(frame["body"]["beam"]["y"].is_u64(), "bemu says where the beam is: {}", frame["body"]["crop"]);
    assert_eq!(frame["body"]["options"], json!({ "whole": false, "live": true, "over": true }));

    adapter.request("evaluate", json!({ "expression": "-frame whole", "context": "repl" }));
    let whole = adapter.wait("whole frame", |m| m["event"] == "cpclib/frameView");
    assert_eq!(png_size(&whole), (1024, 350), "the whole raster");
    assert_eq!(whole["body"]["crop"], json!({ "x": 0, "y": 0, "w": 1024, "h": 350 }));

    // Kept up to date: the next stop brings a new frame by itself.
    adapter.request("continue", json!({ "threadId": 1 }));
    adapter.stopped();
    let refreshed = adapter.wait("refreshed frame", |m| m["event"] == "cpclib/frameView");
    assert_eq!(refreshed["body"]["options"]["whole"], true, "with the options last asked for");

    let refused = adapter.send("evaluate", json!({ "expression": "-frame sideways", "context": "repl" }));
    let answer = adapter.wait("refusal", |m| m["type"] == "response" && m["request_seq"] == refused);
    assert_eq!(answer["success"], false, "{answer}");
    adapter.finish();
}

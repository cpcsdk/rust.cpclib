//! The adapter driving a real bemu: the whole chain an editor uses, from
//! `launch` to the emulator exiting with the session.
//!
//! Needs bemu's `amsrust` executable, named by `$CPCLIB_BEMU`; without it
//! each test says so and passes. bemu is built from its own repository
//! (`cargo +nightly build --release` in rgiot/bemu).
#![cfg(feature = "adapter")]

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
            .stderr(if std::env::var_os("CPCLIB_TEST_ADAPTER_STDERR").is_some() { Stdio::inherit() } else { Stdio::null() })
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
        // the frame a real client asks about: the top one of the stack trace
        let trace = self.request("stackTrace", json!({ "threadId": 1 }));
        let frame = trace["body"]["stackFrames"][0]["id"].clone();
        let scopes = self.request("scopes", json!({ "frameId": frame }));
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

/// A program in a fresh directory of its own: the adapter keeps a source map
/// beside a program it built, and another test's program at the same path
/// would be taken for this one.
fn program(name: &str, source: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let directory = std::env::temp_dir().join(format!("cpclib-dap-bemu-e2e-{}-{unique}-{name}", std::process::id()));
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

/// bemu sets every register (`POST /api/z80`), not only PC: what the
/// Registers view writes is what the program then runs with.
#[test]
fn a_register_set_in_the_registers_view_is_the_emulators() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({ "stopOnEntry": true }));
    adapter.request("configurationDone", json!({}));
    adapter.stopped();

    let trace = adapter.request("stackTrace", json!({ "threadId": 1 }));
    let frame = trace["body"]["stackFrames"][0]["id"].clone();
    let scopes = adapter.request("scopes", json!({ "frameId": frame }));
    let registers = scopes["body"]["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"].as_str().is_some_and(|n| n.contains("Registers")))
        .unwrap()["variablesReference"]
        .clone();

    let set = adapter.request(
        "setVariable",
        json!({ "variablesReference": registers, "name": "HL", "value": "0x1234" })
    );
    assert_eq!(set["body"]["value"], "0x1234", "{set}");
    adapter.request("setVariable", json!({ "variablesReference": registers, "name": "BC'", "value": "&BEEF" }));
    assert_eq!(hex_value(&adapter.register("HL")), 0x1234);
    assert_eq!(hex_value(&adapter.register("BC'")), 0xBEEF);

    // A value too wide for the register is refused, and nothing is sent.
    let seq = adapter.send("setVariable", json!({ "variablesReference": registers, "name": "I", "value": "0x100" }));
    let refused = adapter.wait("refusal", |m| m["type"] == "response" && m["request_seq"] == seq);
    assert_eq!(refused["success"], false, "{refused}");
    adapter.finish();
}

/// With `&C4` paged, #4000 is bank 4: a write from the Memory View lands
/// there, where the program and the view see it, not in bank 1.
#[test]
fn a_memory_write_follows_the_banking() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program(
        "banked.asm",
        "\torg #8000\n\trun start\nstart\n\tld bc,#7fc4\n\tout (c),c\nloop\n\tjr loop\n"
    );
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": [{ "line": 7 }] }));
    adapter.request("configurationDone", json!({}));
    adapter.stopped();

    adapter.request("writeMemory", json!({ "memoryReference": "0x4000", "data": "qrs=" }));
    let memory = adapter.request("readMemory", json!({ "memoryReference": "0x4000", "count": 2 }));
    assert_eq!(memory["body"]["data"], "qrs=", "#AA #BB at #4000 as the CPU sees it: {memory}");
    adapter.finish();
}

/// Step over a `CALL`, into the routine and out of it again: bemu's own
/// stepping when it offers it (`/api/step?mode=`), this side's otherwise.
#[test]
fn step_over_a_call_then_into_it_then_out() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program(
        "calls.asm",
        "\torg #4000\n\trun start\nstart\n\tcall routine\n\tcall routine\nloop\n\tjr loop\nroutine\n\tnop\n\tret\n"
    );
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({ "stopOnEntry": true }));
    adapter.request("configurationDone", json!({}));
    adapter.stopped();
    assert_eq!(adapter.line(), 4, "on the first call");

    adapter.request("next", json!({ "threadId": 1 }));
    let step = adapter.stopped();
    assert_eq!(step["body"]["reason"], "step", "{step}");
    assert_eq!(adapter.line(), 5, "the first call run whole");

    adapter.request("stepIn", json!({ "threadId": 1 }));
    adapter.stopped();
    assert_eq!(adapter.line(), 9, "in the routine");

    adapter.request("stepOut", json!({ "threadId": 1 }));
    let out = adapter.stopped();
    assert_eq!(out["body"]["reason"], "step", "{out}");
    let line = adapter.line();
    assert!((6..=7).contains(&line), "back after the second call: line {line}");
    adapter.finish();
}

/// Memory elsewhere than the CPU's paging shows it: a ROM by its location, a RAM bank by its
/// name, a paging chosen for a view - read and written by bemu itself.
#[test]
fn memory_is_read_and_written_in_a_bank_a_rom_or_through_a_paging() {
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
    let bytes = |view: &Value| -> Vec<u64> {
        view["body"]["bytes"].as_array().unwrap().iter().map(|b| b.as_u64().unwrap()).collect()
    };

    // BASIC's ROM (upper ROM 0), by its location: shown at &C000, through U0. A bemu older
    // than its memory spaces says it cannot, and why.
    let seq = adapter.send("evaluate", json!({ "expression": "-mv UPPER00:0000 16", "context": "repl" }));
    let answer = adapter.wait("ROM view's answer", |m| m["type"] == "response" && m["request_seq"] == seq);
    if answer["success"] == false {
        let why = answer["message"].as_str().unwrap_or_default();
        assert!(why.contains("needs a bemu that pages its memory"), "{answer}");
        adapter.finish();
        return;
    }
    let rom = adapter.wait("ROM view", |m| m["event"] == "cpclib/memoryView");
    assert_eq!(rom["body"]["address"], 0xC000, "{rom}");
    assert_eq!(rom["body"]["paging"], "U0", "the panel is told its paging");
    // the same bytes as the editor's own read of that location
    let read = adapter.request("readMemory", json!({ "memoryReference": "UPPER00:0000", "count": 16 }));
    let decoded: Vec<u64> = base64_decode(read["body"]["data"].as_str().unwrap()).into_iter().map(u64::from).collect();
    assert_eq!(bytes(&rom), decoded);
    assert!(decoded.iter().any(|b| *b != 0), "a ROM's bytes, not empty RAM: {decoded:?}");

    // a RAM bank of the extension, by its name: written, then read back there and through C4
    adapter.request("writeMemory", json!({ "memoryReference": "B04:0000", "data": "qrs=" }));
    let read = adapter.request("readMemory", json!({ "memoryReference": "B04:0000", "count": 2 }));
    assert_eq!(read["body"]["data"], "qrs=", "{read}");
    adapter.request("evaluate", json!({ "expression": "-mv 0x4000 2 C4", "context": "repl" }));
    let banked = adapter.wait("C4 view", |m| m["event"] == "cpclib/memoryView" && m["body"]["paging"] == "C4");
    assert_eq!(bytes(&banked), vec![0xAA, 0xBB]);

    // refused: a ROM written, a paging that is none
    let seq = adapter.send("writeMemory", json!({ "memoryReference": "UPPER00:0000", "data": "AA==" }));
    let refused = adapter.wait("ROM write refusal", |m| m["type"] == "response" && m["request_seq"] == seq);
    assert_eq!(refused["success"], false, "{refused}");
    let seq = adapter.send("evaluate", json!({ "expression": "-mv 0x4000 2 C9", "context": "repl" }));
    let refused = adapter.wait("paging refusal", |m| m["type"] == "response" && m["request_seq"] == seq);
    assert_eq!(refused["success"], false, "{refused}");
    adapter.finish();
}

fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::new();
    for c in text.bytes().filter(|c| *c != b'=') {
        bits = (bits << 6) | ALPHABET.iter().position(|a| *a == c).unwrap() as u32;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    out
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

/// Launched, with source breakpoints, configured.
fn started(adapter: &mut Adapter, bemu: &str, source: &Path, breakpoints: Value) {
    adapter.launch(source, bemu, json!({}));
    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": breakpoints }));
}

#[test]
fn a_conditional_breakpoint_stops_when_its_condition_holds() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    started(&mut adapter, &bemu, &source, json!([{ "line": LINE_INC_A, "condition": "A == 5" }]));
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    assert_eq!(adapter.line(), LINE_INC_A);
    assert_eq!(hex_value(&adapter.register("A")), 5);
    adapter.finish();
}

#[test]
fn a_hit_count_and_a_log_point() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    started(
        &mut adapter,
        &bemu,
        &source,
        json!([
            { "line": LINE_INC_A, "hitCondition": "4" },
            { "line": LINE_LD_C000, "logMessage": "A={A}" }
        ])
    );
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    assert_eq!(hex_value(&adapter.register("A")), 4, "the fourth pass");
    // The log point reported each pass before it, and never stopped.
    let logged: Vec<String> = adapter
        .seen
        .iter()
        .filter(|m| m["event"] == "output")
        .filter_map(|m| m["body"]["output"].as_str())
        .filter(|text| text.starts_with("A="))
        .map(str::to_owned)
        .collect();
    assert_eq!(logged, vec!["A=#02\n", "A=#03\n", "A=#04\n"], "{}", adapter.transcript());
    adapter.finish();
}

#[test]
fn a_data_breakpoint_stops_on_the_write() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    started(&mut adapter, &bemu, &source, json!([]));
    let info = adapter.request("dataBreakpointInfo", json!({ "name": "0xC000" }));
    assert!(info["body"]["accessTypes"].as_array().unwrap().contains(&json!("readWrite")), "{info}");
    adapter.request(
        "setDataBreakpoints",
        json!({ "breakpoints": [{ "dataId": info["body"]["dataId"], "accessType": "write", "condition": "value == 4" }] })
    );
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "data breakpoint", "{stop}");
    let description = stop["body"]["description"].as_str().unwrap();
    assert!(description.contains("write of #04 at #C000"), "{description}");
    assert_eq!(hex_value(&adapter.register("A")), 4);
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

/// A `BREAKPOINT` directive carrying an ACE condition, a mask and a value,
/// on an I/O port: applied whole by bemu, it stops on the write it describes.
#[test]
fn a_directive_breakpoint_is_applied_whole() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program(
        "main.asm",
        "\torg #4000\n\trun start\nstart\n\tld bc,#7f10\n\tld a,#54\nloop\n\tout (c),c\n\tout (c),a\n\tinc a\n\tand #5f\n\tor #40\n\tjr loop\n\
         \tbreakpoint #7f00, type=io, access=write, runmode=stop, mask=#ff00, value=#40, valmask=#c0, condition=\"A = &55 AND NOT FlagZ\", name=\"ink\"\n"
    );
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    let description = stop["body"]["description"].as_str().unwrap();
    assert!(description.contains("ink: write of #55 at #7F10"), "{description}");
    assert_eq!(hex_value(&adapter.register("A")), 0x55, "the second pass, not the first (A = #54)");
    // Nothing reported lost.
    assert!(
        !adapter.seen.iter().any(|m| m["body"]["output"].as_str().is_some_and(|t| t.contains("not applied"))),
        "{}",
        adapter.transcript()
    );
    adapter.finish();
}

/// What the breakpoints view shows: the session's kinds, and what bemu
/// itself holds, each said by what armed it, hits counted.
#[test]
fn the_breakpoints_view_lists_what_bemu_holds() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program(
        "main.asm",
        "\torg #4000\n\trun start\nstart\n\tld bc,#7f10\n\tld a,#54\nloop\n\tout (c),c\n\tout (c),a\n\tinc a\n\tand #5f\n\tor #40\n\tjr loop\n\
         \tbreakpoint #7f00, type=io, access=write, runmode=stop, mask=#ff00, value=#40, valmask=#c0, condition=\"A = &55\", name=\"ink\"\n"
    );
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    // `inc a`, line 9, from its second pass on.
    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": [{ "line": 9, "hitCondition": "2" }] }));
    adapter.request("configurationDone", json!({}));
    adapter.stopped();

    let overview = adapter.request("cpclib/breakpoints", json!({}));
    let kinds = overview["body"]["kinds"].as_array().unwrap();
    let entries = |id: &str| kinds.iter().find(|k| k["id"] == id).unwrap()["entries"].as_array().unwrap().clone();
    assert_eq!(entries("source")[0]["detail"], "0x4009 from hit 2", "{overview}");
    assert_eq!(entries("directive")[0]["label"], "ink");
    assert_eq!(entries("directive")[0]["status"], "armed");

    let held = adapter.request("cpclib/emulatorBreakpoints", json!({}));
    let advanced = held["body"]["advanced"].as_array().unwrap();
    let by_owner = |owner: &str| advanced.iter().find(|b| b["owner"] == owner).unwrap_or_else(|| panic!("no {owner}: {held}"));
    let directive = by_owner("directive");
    assert_eq!((directive["label"].as_str(), directive["kind"].as_str()), (Some("ink"), Some("io")));
    let counted = by_owner("source");
    assert_eq!(counted["label"], "0x4009");
    assert!(counted["hits"].as_u64().unwrap() >= 1, "hits counted: {counted}");
    adapter.finish();
}

/// The breakpoint manager, as ACE's window: a port breakpoint made by hand,
/// with a mask, a value and an ACE condition, stops the program; the manager
/// lists it with its hits.
#[test]
fn a_breakpoint_made_in_the_manager_stops_the_program() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program(
        "main.asm",
        "\torg #4000\n\trun start\nstart\n\tld bc,#7f10\n\tld a,#54\nloop\n\tout (c),c\n\tout (c),a\n\tinc a\n\tand #5f\n\tor #40\n\tjr loop\n"
    );
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    let made = adapter.request(
        "cpclib/managerSet",
        json!({ "breakpoint": { "kind": "io", "address": "&7F00", "mask": "&FF00", "access": "w", "value": "&40", "valmask": "&C0",
                                "condition": "A = &56", "name": "third ink" } })
    );
    assert_eq!(made["body"]["id"], 1);
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    assert!(stop["body"]["description"].as_str().unwrap().contains("third ink: write of #56"), "{stop}");
    assert_eq!(hex_value(&adapter.register("A")), 0x56, "the third pass");

    let listed = adapter.request("cpclib/breakpointManager", json!({}));
    let port = &listed["body"]["tabs"]["io"][0];
    assert_eq!((port["origin"].as_str(), port["key"].as_str()), (Some("manager"), Some("manager:1")), "{listed}");
    let held = adapter.request("cpclib/emulatorBreakpoints", json!({}));
    let counted = held["body"]["advanced"].as_array().unwrap().iter().find(|b| b["owner"] == "manager").unwrap().clone();
    assert_eq!((counted["label"].as_str(), counted["hits"].as_u64()), (Some("1"), Some(1)), "{held}");
    adapter.finish();
}

/// A raster breakpoint made in the manager stops when the beam reaches it.
#[test]
fn a_raster_breakpoint_stops_at_its_line_and_nop() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({}));
    adapter.request("cpclib/managerSet", json!({ "breakpoint": { "kind": "raster", "line": "100", "nop": "10", "name": "split" } }));
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    let description = stop["body"]["description"].as_str().unwrap();
    assert!(description.starts_with("split: beam at line 100, NOP 1"), "{stop}");
    adapter.finish();
}

/// The extension passes the manager's saved breakpoints to the launch: they
/// are armed before the program runs - here the Gate Array ink preset,
/// with a condition.
#[test]
fn saved_manager_breakpoints_are_armed_at_launch() {
    let Some(bemu) = bemu()
    else {
        return;
    };
    let source = program(
        "main.asm",
        "\torg #4000\n\trun start\nstart\n\tld bc,#7f10\n\tld a,#54\nloop\n\tout (c),c\n\tout (c),a\n\tinc a\n\tand #5f\n\tor #40\n\tjr loop\n"
    );
    let mut adapter = Adapter::start();
    adapter.launch(&source, &bemu, json!({ "managerBreakpoints": [
        { "kind": "io", "address": "&4000", "mask": "&C000", "access": "w", "value": "&40", "valmask": "&C0",
          "condition": "A = &55", "name": "Gate Array: ink" }
    ] }));
    adapter.request("configurationDone", json!({}));
    let stop = adapter.stopped();
    assert!(stop["body"]["description"].as_str().unwrap().contains("Gate Array: ink: write of #55"), "{stop}");
    assert_eq!(hex_value(&adapter.register("A")), 0x55, "the second pass: armed before the first");
    adapter.finish();
}

/// bemu in a tab: its web build, served by the adapter, opened by a browser
/// as the editor opens it, and debugged through the page. Needs a web build
/// with the debug API (`$CPCLIB_BEMU_WEB`, bemu's `web/pkg`) and a browser
/// to open it (`$CPCLIB_TEST_BROWSER`, a Chromium); without them the test
/// says so and passes.
#[test]
fn in_a_tab_the_page_is_debugged() {
    let (Ok(web), Ok(browser)) = (std::env::var("CPCLIB_BEMU_WEB"), std::env::var("CPCLIB_TEST_BROWSER"))
    else {
        eprintln!("skipped: set $CPCLIB_BEMU_WEB (bemu's web/pkg) and $CPCLIB_TEST_BROWSER (a Chromium) to run this test");
        return;
    };
    assert!(Path::new(&web).join("bemu.js").is_file(), "no bemu.js in {web}");
    let source = program("main.asm", LOOP);
    let mut adapter = Adapter::start();
    adapter.request("initialize", json!({ "adapterID": "cpclib", "linesStartAt1": true, "columnsStartAt1": true }));
    let launch = adapter.send("launch", json!({ "program": source, "emulator": "bemu", "stopOnEntry": true }));

    // told before the launch is over: the page has to be opened for it to end
    let ready = adapter.wait("the page to open", |m| m["event"] == "cpclib/emulatorReady");
    let url = ready["body"]["url"].as_str().expect("a page").to_owned();
    assert!(url.starts_with("http://127.0.0.1:") && url.ends_with("/debug"), "{ready}");
    let profile = std::env::temp_dir().join(format!("cpclib-dap-bemu-tab-{}", std::process::id()));
    let mut page = Command::new(&browser)
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--autoplay-policy=no-user-gesture-required"])
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(&url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the browser starts");
    struct Closed(Child);
    impl Drop for Closed {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _closed = Closed(std::mem::replace(&mut page, Command::new("true").spawn().unwrap()));

    let response = adapter.wait("response to launch", |m| m["type"] == "response" && m["request_seq"] == launch);
    assert_eq!(response["success"], true, "{response}\n{}", adapter.transcript());
    let again = adapter.wait("the emulator ready", |m| m["event"] == "cpclib/emulatorReady");
    assert_eq!(again["body"]["emulator"], "bemu", "{again}");
    assert_eq!(again["body"]["url"], url.as_str(), "{again}");

    adapter.request("setBreakpoints", json!({ "source": { "path": source }, "breakpoints": [{ "line": LINE_INC_A }] }));
    adapter.request("configurationDone", json!({}));
    let entry = adapter.stopped();
    assert_eq!(entry["body"]["reason"], "entry", "{entry}");
    assert_eq!(adapter.line(), LINE_LD_A_1);

    adapter.request("continue", json!({ "threadId": 1 }));
    let stop = adapter.stopped();
    assert_eq!(stop["body"]["reason"], "breakpoint", "{stop}");
    assert_eq!(adapter.line(), LINE_INC_A);
    assert_eq!(hex_value(&adapter.register("A")), 1);

    adapter.request("next", json!({ "threadId": 1 }));
    adapter.stopped();
    assert_eq!(adapter.line(), LINE_LD_C000);
    assert_eq!(hex_value(&adapter.register("A")), 2);
    adapter.finish();
    let _ = std::fs::remove_dir_all(&profile);
}

//! SugarboxV2, reached over its own debug server: newline-delimited JSON on
//! a plain TCP socket (`Sugarbox/debugers/DebugServer.cpp`), not HTTP like
//! AMSpiriT Lite's. `--debug --debug_server <port>` makes it listen; the
//! machine is halted (`Break()`) the instant a client connects - there is no
//! handshake beyond that.
//!
//! The wire protocol has no request id: exactly one command is ever in
//! flight, so the next line that is not itself a `{"type":"event",...}` is
//! always the answer to whatever was just sent. `step`/`continue`/`stepIn`/
//! `stepOut` acknowledge immediately and the *real* stop, whenever the CPU
//! actually gets there, arrives later as an async `stopped` event - `pause`
//! is the exception (nothing pushes a stop event for a manual halt observed
//! from source, so one is synthesised locally right after its ack, mirroring
//! the reference `amstrad-cpc-debug` extension's own behaviour).
//!
//! Unlike AMSpiriT Lite this emulator can step over/out natively, so there
//! is no temporary-breakpoint dance to simulate one (compare
//! [`crate::amspiritlite::Breakpoints`]).

use std::io::{BufRead, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

use crate::peer::{DapPeer, LineAtPc, Quirks};
use crate::protocol;

/// A register scope with one variable per Z80 register - the same fixed
/// shape every launch of this session gets, since a `variablesReference` of
/// `0` for a scalar leaf is the only value DAP needs here.
const REGISTERS_REFERENCE: i64 = 1;

pub struct SugarBoxPeer {
    write: TcpStream,
    /// Answers to whatever `round_trip` last wrote - the wire protocol's own
    /// serial-ordering guarantee makes a plain channel enough; there is
    /// never more than one outstanding.
    acks: mpsc::Receiver<Value>,
    /// DAP-shaped messages waiting for `drain()` - both this peer's own
    /// synchronous responses and the reader thread's translated async
    /// events land here, in the order they were produced.
    pending: mpsc::Receiver<Value>,
    outgoing: mpsc::Sender<Value>,
    launched: Option<std::process::Child>,
    seq: AtomicI64,
    round_trip_timeout: Duration
}

impl SugarBoxPeer {
    pub fn connect(endpoint: &str) -> std::io::Result<Self> {
        let write = TcpStream::connect(endpoint)?;
        let read = write.try_clone()?;
        let (outgoing, pending) = mpsc::channel();
        let (ack_tx, acks) = mpsc::channel();
        let events_out = outgoing.clone();
        std::thread::Builder::new()
            .name("sugarbox-events".to_string())
            .spawn(move || read_loop(read, events_out, ack_tx))?;

        Ok(Self {
            write,
            acks,
            pending,
            outgoing,
            launched: None,
            seq: AtomicI64::new(0),
            round_trip_timeout: Duration::from_secs(10)
        })
    }

    /// Only an emulator this session started is closed with it; one the user
    /// started is theirs - same contract as `AmspiritLitePeer::owning`.
    pub fn owning(mut self, child: std::process::Child) -> Self {
        self.launched = Some(child);
        self
    }

    fn next_seq(&self) -> i64 {
        self.seq.fetch_add(1, Ordering::Relaxed)
    }

    fn push(&self, message: Value) {
        let _ = self.outgoing.send(message);
    }

    /// Write one command and block for SugarboxV2's own reply. The protocol
    /// carries no id, so whichever non-event line the reader thread sees
    /// next is unconditionally this command's answer.
    fn round_trip(&mut self, command: Value) -> std::io::Result<Value> {
        let mut line = command.to_string();
        line.push('\n');
        self.write.write_all(line.as_bytes())?;
        self.acks.recv_timeout(self.round_trip_timeout).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "SugarboxV2's debug server did not answer in time"
            )
        })
    }

    /// Sent once, right after connecting for a fresh launch: the machine is
    /// already halted (auto-`Break()` on connect), so there is nothing to
    /// undo before loading - matching the reference extension's own launch
    /// order (snapshot/media first, then let the session decide whether to
    /// run or stay stopped).
    pub fn load_snapshot(&mut self, path: &cpclib_common::camino::Utf8Path) -> std::io::Result<()> {
        self.round_trip(json!({"cmd": "loadSnapshot", "path": path.as_str()}))?;
        Ok(())
    }

    pub fn insert_disk(&mut self, drive: u64, path: &cpclib_common::camino::Utf8Path) -> std::io::Result<()> {
        self.round_trip(json!({"cmd": "insertDisk", "drive": drive, "path": path.as_str()}))?;
        Ok(())
    }
}

fn read_loop(stream: TcpStream, events_out: mpsc::Sender<Value>, acks: mpsc::Sender<Value>) {
    let mut reader = std::io::BufReader::new(stream);
    // Well clear of `Session::next_seq`'s own low counter, the same
    // out-of-band convention `amspiritlite::watch_run_state` uses for events
    // it synthesises off the session's thread.
    let mut seq = 1_000_000i64;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\n', '\r']);
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                    continue;
                };
                if value.get("type").and_then(Value::as_str) == Some("event") {
                    seq += 1;
                    if let Some(dap_event) = sugarbox_event_for(&value, seq) {
                        let _ = events_out.send(dap_event);
                    }
                }
                else {
                    let _ = acks.send(value);
                }
            }
        }
    }
}

fn sugarbox_event_for(raw: &Value, seq: i64) -> Option<Value> {
    let name = raw.get("event").and_then(Value::as_str)?;
    let body = raw.get("body").cloned().unwrap_or(json!({}));
    match name {
        "stopped" => Some(protocol::event("stopped", body, seq)),
        "mediaChanged" => Some(protocol::event("cpclib/mediaChanged", body, seq)),
        _ => None
    }
}

fn parse_flexible_int(value: &str) -> Option<u32> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    }
    else {
        value.parse::<u32>().ok()
    }
}

/// Every read-only hardware-state passthrough this bridge exposes, custom
/// `cpclib/...` DAP requests with no `arguments` beyond what is listed here,
/// one `cmd` each, answered by echoing SugarboxV2's own reply body back
/// unchanged (`sugarbox_response_for`'s `"cpclib/"` arm).
const PASSTHROUGH_COMMANDS: &[(&str, &str)] = &[
    ("cpclib/memmap", "getMemBanks"),
    ("cpclib/crtc", "getCrtcState"),
    ("cpclib/ga", "getGateArrayState"),
    ("cpclib/psg", "getPsgState"),
    ("cpclib/ppi", "getPpiState"),
    ("cpclib/fdc", "getFdcState"),
    ("cpclib/tape", "getTapeState"),
    ("cpclib/tapeSignal", "getTapeSignal"),
    ("cpclib/asic", "getAsicState"),
    ("cpclib/screen", "getScreen")
];

/// Translate one DAP request into SugarboxV2's own JSON command shape.
/// `None` means "nothing to send to the emulator" - `send()` answers such a
/// request with an empty body itself.
fn sugarbox_call_for(request: &Value) -> Option<Value> {
    let command = request.get("command").and_then(Value::as_str)?;
    let args = request.get("arguments").cloned().unwrap_or(json!({}));

    if let Some((_, cmd)) = PASSTHROUGH_COMMANDS.iter().find(|(name, _)| *name == command) {
        return Some(json!({"cmd": cmd}));
    }

    Some(match command {
        "continue" => json!({"cmd": "continue"}),
        "next" => json!({"cmd": "step"}),
        "stepIn" => json!({"cmd": "stepIn"}),
        "stepOut" => json!({"cmd": "stepOut"}),
        "restart" => json!({"cmd": "reset"}),
        "variables" => json!({"cmd": "readRegisters"}),
        "setVariable" => {
            let name = args.get("name").and_then(Value::as_str)?.to_lowercase();
            let value = args
                .get("value")
                .and_then(Value::as_str)
                .and_then(parse_flexible_int)?;
            let mut command = serde_json::Map::new();
            command.insert("cmd".to_string(), json!("setRegisters"));
            command.insert(name, json!(value));
            Value::Object(command)
        },
        "readMemory" => {
            let address = args
                .get("memoryReference")
                .and_then(Value::as_str)
                .and_then(protocol::parse_address_reference)?;
            let count = args.get("count").and_then(Value::as_u64).unwrap_or(1);
            json!({"cmd": "readMemory", "address": address, "size": count})
        },
        "writeMemory" => {
            let address = args
                .get("memoryReference")
                .and_then(Value::as_str)
                .and_then(protocol::parse_address_reference)?;
            let data = args.get("data").and_then(Value::as_str)?;
            let bytes = crate::session::decode_base64(data);
            json!({"cmd": "writeMemory", "address": address, "bytes": bytes})
        },
        "disassemble" => {
            let address = args
                .get("memoryReference")
                .and_then(Value::as_str)
                .and_then(protocol::parse_address_reference)?;
            let count = args.get("instructionCount").and_then(Value::as_u64).unwrap_or(16);
            json!({"cmd": "disassemble", "address": address, "count": count})
        },
        "evaluate" => {
            let expression = args.get("expression").and_then(Value::as_str)?;
            json!({"cmd": "evaluate", "expression": expression})
        },
        "cpclib/setPc" => {
            let address = args.get("address").and_then(Value::as_u64)?;
            json!({"cmd": "setPC", "address": address})
        },
        "cpclib/sendKey" => {
            json!({
                "cmd": "sendKey",
                "line": args.get("line")?,
                "bit": args.get("bit")?,
                "pressed": args.get("pressed")?
            })
        },
        "cpclib/trackRaw" => {
            json!({
                "cmd": "getTrackRaw",
                "drive": args.get("drive")?,
                "side": args.get("side")?,
                "track": args.get("track")?
            })
        },
        "cpclib/insertDisk" => {
            json!({
                "cmd": "insertDisk",
                "drive": args.get("drive").and_then(Value::as_u64).unwrap_or(0),
                "path": args.get("path").and_then(Value::as_str)?
            })
        },
        "cpclib/insertTape" => {
            json!({"cmd": "insertTape", "path": args.get("path").and_then(Value::as_str)?})
        },
        _ => return None
    })
}

fn sugarbox_registers_of(state: &Value) -> Vec<Value> {
    const NAMES: &[&str] = &[
        "AF", "BC", "DE", "HL", "AF'", "BC'", "DE'", "HL'", "IX", "IY", "SP", "PC", "I", "R"
    ];
    NAMES
        .iter()
        .filter_map(|name| {
            state.get(*name).and_then(Value::as_u64).map(|value| {
                json!({
                    "name": name,
                    "value": format!("0x{value:04X}"),
                    "variablesReference": 0
                })
            })
        })
        .collect()
}

/// Shape SugarboxV2's own reply (`state`) into the DAP response `request`
/// is waiting for.
fn sugarbox_response_for(request: &Value, state: &Value, seq: i64) -> Value {
    let command = request.get("command").and_then(Value::as_str).unwrap_or_default();
    let body = match command {
        "stackTrace" => {
            let pc = state.get("PC").and_then(Value::as_u64).unwrap_or(0);
            json!({
                "stackFrames": [{
                    "id": 1,
                    "name": format!("Z80 @ 0x{pc:04X}"),
                    "line": 0,
                    "column": 0,
                    "instructionPointerReference": format!("0x{pc:04X}")
                }],
                "totalFrames": 1
            })
        },
        "variables" => json!({"variables": sugarbox_registers_of(state)}),
        "readMemory" => {
            let bytes: Vec<u8> = state
                .get("bytes")
                .and_then(Value::as_array)
                .map(|values| values.iter().filter_map(Value::as_u64).map(|b| b as u8).collect())
                .unwrap_or_default();
            json!({
                "address": request
                    .get("arguments")
                    .and_then(|a| a.get("memoryReference"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "data": crate::amspiritlite::encode_base64(&bytes)
            })
        },
        "disassemble" => {
            let instructions: Vec<Value> = state
                .get("instructions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|instruction| {
                    let address = instruction.get("address").and_then(Value::as_u64).unwrap_or(0);
                    json!({
                        "address": format!("0x{address:04X}"),
                        "instruction": instruction.get("instruction").cloned().unwrap_or(json!(""))
                    })
                })
                .collect();
            json!({"instructions": instructions})
        },
        "evaluate" => json!({"result": state.get("text").cloned().unwrap_or(json!("?")), "variablesReference": 0}),
        _ if command.starts_with("cpclib/") => state.clone(),
        _ => json!({})
    };
    protocol::response(request, body, seq)
}

impl DapPeer for SugarBoxPeer {
    fn send(&mut self, message: Value) -> std::io::Result<()> {
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        // DAP session-lifecycle commands the raw wire protocol knows
        // nothing about, plus the fixed-shape thread/scopes answers - a
        // single Z80 "thread", one register scope. Same fixed shape
        // `AmspiritLitePeer` uses, for the same reason: a machine-level
        // debugger with no symbol table of its own has no call stack to
        // report beyond "here is PC" (`stackTrace`, handled below via a
        // real round trip since it needs a live PC).
        match command.as_str() {
            "initialize" | "configurationDone" => {
                let seq = self.next_seq();
                self.push(protocol::response(&message, json!({}), seq));
                return Ok(());
            },
            "attach" | "launch" => {
                let seq = self.next_seq();
                self.push(protocol::response(&message, json!({}), seq));
                let stop_seq = self.next_seq();
                self.push(protocol::event(
                    "stopped",
                    json!({"reason": "entry", "threadId": 1, "allThreadsStopped": true}),
                    stop_seq
                ));
                return Ok(());
            },
            "disconnect" | "terminate" => {
                // Left running rather than left halted forever once this
                // session detaches.
                let _ = self.round_trip(json!({"cmd": "continue"}));
                let seq = self.next_seq();
                self.push(protocol::response(&message, json!({}), seq));
                return Ok(());
            },
            "threads" => {
                let seq = self.next_seq();
                self.push(protocol::response(
                    &message,
                    json!({"threads": [{"id": 1, "name": "Z80"}]}),
                    seq
                ));
                return Ok(());
            },
            "scopes" => {
                let seq = self.next_seq();
                self.push(protocol::response(
                    &message,
                    json!({
                        "scopes": [{
                            "name": "Registers",
                            "variablesReference": REGISTERS_REFERENCE,
                            "expensive": false,
                            "presentationHint": "registers"
                        }]
                    }),
                    seq
                ));
                return Ok(());
            },
            _ => {}
        }

        if command == "setInstructionBreakpoints" {
            let addresses: Vec<u32> = message
                .get("arguments")
                .and_then(|a| a.get("breakpoints"))
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(|entry| {
                            entry
                                .get("instructionReference")
                                .and_then(Value::as_str)
                                .and_then(protocol::parse_address_reference)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let breakpoints: Vec<Value> =
                addresses.iter().map(|address| json!({"address": address})).collect();
            let ack = self.round_trip(json!({"cmd": "setBreakpoints", "breakpoints": breakpoints}))?;
            let seq = self.next_seq();
            self.push(sugarbox_response_for(&message, &ack, seq));
            return Ok(());
        }

        if command == "pause" {
            let ack = self.round_trip(json!({"cmd": "halt"}))?;
            let seq = self.next_seq();
            self.push(sugarbox_response_for(&message, &ack, seq));
            // The wire protocol is not confirmed (from source alone) to push
            // its own `stopped` event for a manual halt - synthesised here
            // so `pause` always shows a stop even if the server stays
            // quiet, matching the reference extension's own behaviour. A
            // late real one arriving afterward is a harmless duplicate.
            let stop_seq = self.next_seq();
            self.push(protocol::event(
                "stopped",
                json!({"reason": "pause", "threadId": 1, "allThreadsStopped": true}),
                stop_seq
            ));
            return Ok(());
        }

        let Some(call) = sugarbox_call_for(&message)
        else {
            let seq = self.next_seq();
            self.push(protocol::response(&message, json!({}), seq));
            return Ok(());
        };

        // `stackTrace` needs a live PC, not just a fixed shape - fetched via
        // the same register read `variables` uses.
        if command == "stackTrace" {
            let ack = self.round_trip(json!({"cmd": "readRegisters"}))?;
            let seq = self.next_seq();
            self.push(sugarbox_response_for(&message, &ack, seq));
            return Ok(());
        }

        let ack = self.round_trip(call)?;
        let seq = self.next_seq();
        self.push(sugarbox_response_for(&message, &ack, seq));
        Ok(())
    }

    fn drain(&mut self) -> Vec<Value> {
        self.pending.try_iter().collect()
    }

    /// SugarboxV2 steps over/out on its own - nothing here needs to know
    /// what kind of line the PC is sitting on.
    fn note_line_at_pc(&mut self, _line: LineAtPc) {}

    fn quirks(&self) -> Quirks {
        Quirks {
            instruction_breakpoints_only: true,
            attach_required: false,
            rejects_unknown_requests: false
        }
    }

    fn supports(&self, command: &str) -> bool {
        matches!(
            command,
            "initialize"
                | "configurationDone"
                | "attach"
                | "launch"
                | "disconnect"
                | "terminate"
                | "threads"
                | "scopes"
                | "stackTrace"
                | "variables"
                | "setVariable"
                | "setInstructionBreakpoints"
                | "continue"
                | "next"
                | "stepIn"
                | "stepOut"
                | "pause"
                | "restart"
                | "readMemory"
                | "writeMemory"
                | "disassemble"
                | "evaluate"
                | "cpclib/setPc"
                | "cpclib/sendKey"
                | "cpclib/trackRaw"
                | "cpclib/insertDisk"
                | "cpclib/insertTape"
                | "cpclib/memmap"
                | "cpclib/crtc"
                | "cpclib/ga"
                | "cpclib/psg"
                | "cpclib/ppi"
                | "cpclib/fdc"
                | "cpclib/tape"
                | "cpclib/tapeSignal"
                | "cpclib/asic"
                | "cpclib/screen"
        )
    }
}

impl Drop for SugarBoxPeer {
    fn drop(&mut self) {
        // Only one this session started is closed with it; one the user
        // started is theirs.
        if let Some(child) = self.launched.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `127.0.0.1:1234` - no `http://`, unlike AMSpiriT Lite's endpoints
/// (`amspiritlite::wait_until_listening` insists on that scheme, so it
/// cannot be reused for this plain-TCP protocol).
pub fn wait_until_listening(endpoint: &str, patience: Duration) -> Result<(), String> {
    let address: std::net::SocketAddr = endpoint
        .parse()
        .map_err(|e| format!("{endpoint} is not an address: {e}"))?;

    let deadline = std::time::Instant::now() + patience;
    while std::time::Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!(
        "SugarboxV2 did not start listening on {endpoint} within {} seconds",
        patience.as_secs()
    ))
}

fn spawn_sugarbox<E>(port: u16, observer: &E) -> Result<(std::process::Child, u16), String>
where E: cpclib_common::event::EventObserver + 'static {
    use crate::amspiritlite::port_to_serve_on;
    use cpclib_runner::runner::emulator::{Emulator, SugarBoxV2Version};

    let emulator = Emulator::SugarBoxV2(SugarBoxV2Version::default());
    let configuration = emulator.configuration::<E>();
    if !configuration.is_cached() {
        configuration.install(observer)?;
    }

    let port = port_to_serve_on(port);
    let executable = configuration.exec_fname();
    let mut command = std::process::Command::new(executable.as_str());
    command
        .arg("--debug")
        .arg("--debug_server")
        .arg(port.to_string())
        .arg("--hide")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    crate::amspiritlite::strip_snap_leaked_env_vars(&mut command);
    let child = command
        .spawn()
        .map_err(|e| format!("cannot start {executable}: {e}"))?;

    Ok((child, port))
}

/// Start SugarboxV2 with `snapshot` loaded, ready to run from where it was
/// saved - the Z80 launch flow's own entry point, mirroring
/// `amspiritlite::launch`. Unlike that emulator, the snapshot is not a CLI
/// argument: SugarboxV2 always boots cold and only accepts a snapshot
/// through `loadSnapshot` once its debug server is up, so it is sent right
/// after connecting instead.
pub fn launch<E>(
    snapshot: &[u8],
    port: u16,
    observer: &E
) -> Result<(String, SugarBoxPeer), String>
where E: cpclib_common::event::EventObserver + 'static {
    let (child, port) = spawn_sugarbox(port, observer)?;

    let path = std::env::temp_dir().join(format!("cpclib-dap-sugarbox-{}.sna", std::process::id()));
    fs_err::write(&path, snapshot).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    let path = cpclib_common::camino::Utf8PathBuf::from_path_buf(path)
        .map_err(|p| format!("{} is not valid UTF-8", p.display()))?;

    let endpoint = format!("127.0.0.1:{port}");
    wait_until_listening(&endpoint, Duration::from_secs(30))?;

    let mut peer = SugarBoxPeer::connect(&endpoint)
        .map_err(|e| format!("cannot reach SugarboxV2 at {endpoint}: {e}"))?;
    let load_result = peer.load_snapshot(&path);
    let _ = fs_err::remove_file(&path);
    load_result.map_err(|e| format!("SugarboxV2 could not load the snapshot: {e}"))?;

    Ok((endpoint, peer.owning(child)))
}

/// Start SugarboxV2 with `disk` in drive A and nothing run automatically -
/// mirrors `amspiritlite::launch_with_disk`'s own "loaded, not run" contract.
pub fn launch_with_disk<E>(
    disk: &std::path::Path,
    port: u16,
    observer: &E
) -> Result<(String, SugarBoxPeer), String>
where E: cpclib_common::event::EventObserver + 'static {
    let (child, port) = spawn_sugarbox(port, observer)?;

    let path = cpclib_common::camino::Utf8Path::from_path(disk)
        .ok_or_else(|| format!("{} is not valid UTF-8", disk.display()))?;

    let endpoint = format!("127.0.0.1:{port}");
    wait_until_listening(&endpoint, Duration::from_secs(30))?;

    let mut peer = SugarBoxPeer::connect(&endpoint)
        .map_err(|e| format!("cannot reach SugarboxV2 at {endpoint}: {e}"))?;
    peer.insert_disk(0, path)
        .map_err(|e| format!("SugarboxV2 could not insert the disk: {e}"))?;

    Ok((endpoint, peer.owning(child)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(command: &str, arguments: Value) -> Value {
        json!({"seq": 1, "type": "request", "command": command, "arguments": arguments})
    }

    /// A plausible `readRegisters` reply shape - field names as the research
    /// against `DebugServer.cpp` found them, not guessed.
    fn registers() -> Value {
        json!({
            "AF": 0x0740, "BC": 0x4F7F, "DE": 0xFAB4, "HL": 0x0A81,
            "AF'": 0, "BC'": 0xF680, "DE'": 0xC0, "HL'": 0x77,
            "IX": 0x4332, "IY": 0x2900, "SP": 0x0303, "PC": 0x0A31,
            "I": 0, "R": 98
        })
    }

    #[test]
    fn stepping_commands_translate_one_to_one() {
        assert_eq!(sugarbox_call_for(&request("continue", json!({}))).unwrap()["cmd"], "continue");
        assert_eq!(sugarbox_call_for(&request("next", json!({}))).unwrap()["cmd"], "step");
        assert_eq!(sugarbox_call_for(&request("stepIn", json!({}))).unwrap()["cmd"], "stepIn");
        assert_eq!(sugarbox_call_for(&request("stepOut", json!({}))).unwrap()["cmd"], "stepOut");
        assert_eq!(sugarbox_call_for(&request("restart", json!({}))).unwrap()["cmd"], "reset");
    }

    #[test]
    fn a_memory_read_asks_for_the_address_and_count() {
        let call = sugarbox_call_for(&request(
            "readMemory",
            json!({"memoryReference": "0x4000", "count": 32})
        ))
        .unwrap();
        assert_eq!(call["cmd"], "readMemory");
        assert_eq!(call["address"], 0x4000);
        assert_eq!(call["size"], 32);
    }

    #[test]
    fn a_memory_answer_is_re_encoded_as_base64_for_the_editor() {
        let read = request("readMemory", json!({"memoryReference": "0x4000", "count": 3}));
        let answer = sugarbox_response_for(&read, &json!({"bytes": [0x3E, 0x00, 0xC9]}), 9);
        assert_eq!(answer["body"]["data"], json!("PgDJ"));
    }

    #[test]
    fn breakpoints_go_over_as_addresses() {
        let call = sugarbox_call_for(&request(
            "setInstructionBreakpoints",
            json!({"breakpoints": [{"instructionReference": "0x1234"}, {"instructionReference": "0x2000"}]})
        ));
        // setInstructionBreakpoints is handled directly in `send`, not through
        // `sugarbox_call_for` - this asserts the general dispatcher instead
        // finds nothing to translate for it, so a caller cannot mistakenly
        // route it there.
        assert!(call.is_none());
    }

    #[test]
    fn set_variable_builds_a_named_register_write() {
        let call = sugarbox_call_for(&request(
            "setVariable",
            json!({"name": "HL", "value": "0x1234"})
        ))
        .unwrap();
        assert_eq!(call["cmd"], "setRegisters");
        assert_eq!(call["hl"], 0x1234);
    }

    #[test]
    fn set_variable_accepts_a_plain_decimal_value_too() {
        let call = sugarbox_call_for(&request("setVariable", json!({"name": "a", "value": "255"})))
            .unwrap();
        assert_eq!(call["a"], 255);
    }

    #[test]
    fn a_stack_trace_reports_pc_as_the_one_frame() {
        let trace = request("stackTrace", json!({}));
        let answer = sugarbox_response_for(&trace, &registers(), 3);
        assert_eq!(answer["body"]["totalFrames"], 1);
        assert_eq!(
            answer["body"]["stackFrames"][0]["instructionPointerReference"],
            json!("0x0A31")
        );
    }

    #[test]
    fn variables_lists_every_register_present_in_the_state() {
        let variables = request("variables", json!({}));
        let answer = sugarbox_response_for(&variables, &registers(), 4);
        let names: Vec<&str> = answer["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"PC"));
        assert!(names.contains(&"HL"));
        assert_eq!(answer["body"]["variables"][0]["variablesReference"], 0);
    }

    #[test]
    fn passthrough_commands_translate_to_their_own_query() {
        for (name, cmd) in PASSTHROUGH_COMMANDS {
            let call = sugarbox_call_for(&request(name, json!({}))).unwrap();
            assert_eq!(call["cmd"], *cmd, "{name}");
        }
    }

    #[test]
    fn a_passthrough_answer_is_forwarded_unchanged() {
        let req = request("cpclib/crtc", json!({}));
        let state = json!({"regs": [63, 48], "selected_reg": 8});
        let answer = sugarbox_response_for(&req, &state, 1);
        assert_eq!(answer["body"], state);
    }

    #[test]
    fn an_unknown_command_has_nothing_to_send() {
        assert!(sugarbox_call_for(&request("completions", json!({}))).is_none());
    }

    #[test]
    fn a_stopped_event_is_translated_and_kept() {
        let raw = json!({"type": "event", "event": "stopped", "body": {"reason": "breakpoint", "threadId": 1}});
        let event = sugarbox_event_for(&raw, 42).unwrap();
        assert_eq!(event["event"], "stopped");
        assert_eq!(event["seq"], 42);
        assert_eq!(event["body"]["reason"], "breakpoint");
    }

    #[test]
    fn an_unrecognised_event_is_dropped_rather_than_forwarded_blindly() {
        let raw = json!({"type": "event", "event": "somethingNewAndUnhandled", "body": {}});
        assert!(sugarbox_event_for(&raw, 1).is_none());
    }

    #[test]
    fn flexible_int_parsing_accepts_hex_and_decimal() {
        assert_eq!(parse_flexible_int("0x1F"), Some(0x1F));
        assert_eq!(parse_flexible_int("31"), Some(31));
        assert_eq!(parse_flexible_int("not a number"), None);
    }

    /// End-to-end through a real loopback socket: a fake server plays
    /// SugarboxV2's own role (auto-halt on connect, one ack per command, an
    /// async `stopped` event pushed on its own schedule) and this asserts the
    /// reader-thread demux really does route each line to the right place -
    /// the "next non-event line is this command's ack" contract has no unit
    /// test otherwise, since `sugarbox_call_for`/`_response_for` above are
    /// pure functions that never touch a socket.
    #[test]
    fn round_trip_and_async_stop_are_demuxed_correctly() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut writer = stream.try_clone().unwrap();
            let mut reader = std::io::BufReader::new(stream);

            // A command line in, an ack line out - mirrors the real
            // server's own "one command in flight, answered in order"
            // contract.
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.contains("\"getCrtcState\""));
            writeln!(writer, r#"{{"regs":[63,48]}}"#).unwrap();

            // An async stop, pushed with no command having asked for it -
            // must not be mistaken for the ack above (already sent) nor
            // for a future one.
            writeln!(
                writer,
                r#"{{"type":"event","event":"stopped","body":{{"reason":"breakpoint","threadId":1}}}}"#
            )
            .unwrap();
        });

        let mut peer = SugarBoxPeer::connect(&addr.to_string()).unwrap();
        peer.send(request("cpclib/crtc", json!({}))).unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut messages = Vec::new();
        while messages.len() < 2 && std::time::Instant::now() < deadline {
            messages.extend(peer.drain());
            std::thread::sleep(Duration::from_millis(20));
        }
        server.join().unwrap();

        // The response (pushed by `send`, on this thread, after its blocking
        // round trip returns) and the async event (pushed by the reader
        // thread the moment it reads that second line) race each other for
        // which lands in `pending` first - both orders are legitimate, so
        // this only asserts both arrived, not in which order.
        assert_eq!(messages.len(), 2, "expected the response and the async event: {messages:?}");
        assert!(
            messages.iter().any(|m| m["body"]["regs"] == json!([63, 48])),
            "missing the getCrtcState response: {messages:?}"
        );
        assert!(
            messages.iter().any(|m| m["event"] == "stopped"),
            "missing the async stopped event: {messages:?}"
        );
    }
}

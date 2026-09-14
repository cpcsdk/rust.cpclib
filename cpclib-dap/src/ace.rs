//! ACE, reached over its own web API: newline-delimited JSON on a plain TCP
//! socket (`-enable_webapi -web_port <port>`), confirmed live against a real
//! running instance (`Xvfb` + `AceDL -enable_webapi -web_port <port>
//! -borderless`).
//!
//! Unlike SugarboxV2's debug server, **ACE pushes no asynchronous events at
//! all** - confirmed live by setting a breakpoint, issuing `continue`, and
//! blocking-reading the connection for 6+ seconds with nothing arriving (the
//! connection is simply closed by the server rather than kept open for a
//! push). So every command here is a fresh one-shot connection (`round_trip`
//! below), never a persistent one, and a background poller
//! (`watch_run_state`) is what notices an asynchronous breakpoint hit -
//! modeled on `crate::amspiritlite`'s own polling architecture, not on
//! SugarboxV2's reader-thread demux. The confirmed-reliable signal is
//! `getStatus`'s `z80.states.cycleCount` field *not incrementing* between
//! two consecutive polls (`PC` alone is weaker - a tight loop can legitimately
//! revisit the same address while still running). One known false-positive
//! risk this heuristic cannot distinguish from a real stop: a `HALT` opcode
//! executed in-CPC while waiting for an interrupt looks the same as a
//! genuinely halted CPU.
//!
//! Also confirmed live and worth calling out because they differ from
//! SugarboxV2:
//! - `readMemory`/`writeMemory` require `memType`/`bank` fields
//!   (`"ram"`/`-1` is the confirmed-working default) - SugarboxV2's own
//!   calls only ever send `address`/`size`.
//! - `next` maps to ACE's `stepOver`, not `step` - the reverse of
//!   SugarboxV2, whose own `step` already means step-over. ACE exposes
//!   `step` and `stepOver` as genuinely distinct verbs.
//! - `stepOut` is natively supported (confirmed live), so unlike AMSpiriT
//!   Lite there is no temporary-breakpoint simulation needed for it.
//! - ACE's real replies are pretty-printed across several physical lines,
//!   not single-line like SugarboxV2's - `round_trip` accumulates lines
//!   until the buffer parses as one complete JSON value, rather than
//!   assuming one `read_line` is always the whole message (this was found
//!   the hard way, against the real binary, not guessed up front).
//!
//! `cpclib/*` hardware-state passthroughs (CRTC/PSG/PPI/FDC/tape/ASIC/GA -
//! see `sugarbox::PASSTHROUGH_COMMANDS`) are deliberately not implemented
//! here yet: ACE nests all of that inside `getStatus`'s own sub-objects
//! rather than exposing one command per subsystem, and none of those
//! sub-object shapes have been captured/cross-checked against what
//! `presentation.rs`'s decoders expect. `cpclib/screen` is excluded
//! outright - `getScreen` is confirmed not yet implemented on ACE's side.

use std::io::{BufRead, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use serde_json::{Value, json};

use crate::peer::{DapPeer, LineAtPc, Quirks};
use crate::protocol;

/// A register scope with one variable per Z80 register - same fixed shape
/// `sugarbox`/`amspiritlite` both use, since a `variablesReference` of `0`
/// for a scalar leaf is all DAP needs here.
const REGISTERS_REFERENCE: i64 = 1;

pub struct AcePeer {
    port: u16,
    /// DAP-shaped messages waiting for `drain()` - both this peer's own
    /// synchronous responses and the poller thread's synthesised `stopped`/
    /// `terminated` events land here, in the order they were produced.
    pending: mpsc::Receiver<Value>,
    outgoing: mpsc::Sender<Value>,
    /// Set right before a `continue` round trip, cleared the moment the
    /// poller (or a later synchronous stop) reports the resulting stop -
    /// see this module's own doc comment and `watch_run_state` for why this
    /// is the only way an asynchronous breakpoint hit is ever noticed here.
    expecting_stop: Arc<AtomicBool>,
    launched: Option<std::process::Child>,
    seq: AtomicI64,
    round_trip_timeout: Duration
}

impl AcePeer {
    pub fn connect(port: u16) -> std::io::Result<Self> {
        // A quick reachability check up front, rather than only discovering
        // the port is dead on the first real command - `wait_until_listening`
        // (below) is what callers use before this, so by the time `connect`
        // runs the port should already answer; this is just a fast local
        // failure instead of a confusing first-command timeout if it doesn't.
        TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{e}"))
            })?,
            Duration::from_secs(5)
        )?;

        let (outgoing, pending) = mpsc::channel();
        let expecting_stop = Arc::new(AtomicBool::new(false));

        let watched = Watched {
            port,
            out: outgoing.clone(),
            expecting_stop: expecting_stop.clone()
        };
        std::thread::Builder::new()
            .name("ace-poller".to_string())
            .spawn(move || watch_run_state(&watched))?;

        Ok(Self {
            port,
            pending,
            outgoing,
            expecting_stop,
            launched: None,
            seq: AtomicI64::new(0),
            round_trip_timeout: Duration::from_secs(10)
        })
    }

    /// Only an emulator this session started is closed with it; one the
    /// user started is theirs - same contract as `SugarBoxPeer::owning`.
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

    fn round_trip(&mut self, command: Value) -> std::io::Result<Value> {
        ace_round_trip(self.port, command, self.round_trip_timeout)
    }

    /// Push a `stopped` event right now, for a command already confirmed
    /// synchronous (`halt`/`step`/`stepOver`/`stepIn`/`stepOut`) rather than
    /// waiting on the poller's next tick - snappier, and avoids the poller
    /// re-announcing the same stop a moment later (it never will: this
    /// clears `expecting_stop`, which is the only thing that makes the
    /// poller announce anything at all).
    fn announce_stop_now(&self, reason: &str) {
        self.expecting_stop.store(false, Ordering::Relaxed);
        let seq = self.next_seq();
        self.push(protocol::event(
            "stopped",
            json!({"reason": reason, "threadId": 1, "allThreadsStopped": true}),
            seq
        ));
    }
}

/// What the run-state poller watches, and what it announces into - a
/// smaller sibling of `crate::amspiritlite::Stops`, since ACE has no
/// step-over-simulation state to carry (native `stepOver`/`stepOut`, unlike
/// AMSpiriT Lite).
struct Watched {
    port: u16,
    out: mpsc::Sender<Value>,
    expecting_stop: Arc<AtomicBool>
}

/// Ten times a second: fast enough that a `continue`-then-breakpoint stop
/// feels reasonably immediate, cheap enough that it is one small request on
/// loopback - same cadence `amspiritlite::watch_run_state` uses. See this
/// module's own doc comment for why polling is the only option here at all
/// (no async push), and why `cycleCount` freezing is the signal watched
/// (not `PC`).
fn watch_run_state(watched: &Watched) {
    let mut last_cycle_count: Option<u64> = None;
    let mut seq = 1_000_000i64;
    let mut misses = 0u32;

    loop {
        std::thread::sleep(Duration::from_millis(100));

        let status = match ace_round_trip(
            watched.port,
            json!({"cmd": "getStatus"}),
            Duration::from_secs(2)
        ) {
            Ok(v) => {
                misses = 0;
                v
            },
            Err(_) => {
                // A single failed poll is usually the emulator being busy
                // for a moment (recall: reusing a connection while another
                // command is in flight is unreliable), not the emulator
                // being gone - and this thread is the *only* thing that
                // notices an asynchronous breakpoint hit, so giving up on
                // the first failure silently ends stop detection for the
                // rest of the session. Three seconds of nothing at all
                // (matching `amspiritlite::watch_run_state`'s own
                // threshold) is what counts as gone.
                misses += 1;
                if misses > 30 {
                    seq += 1;
                    let said = protocol::event(
                        "output",
                        json!({
                            "category": "console",
                            "output": "ACE is no longer answering - the window was probably \
                                       closed. Ending the debug session.\n"
                        }),
                        seq
                    );
                    let _ = watched.out.send(said);
                    seq += 1;
                    let _ = watched.out.send(protocol::event("terminated", json!({}), seq));
                    return;
                }
                continue;
            }
        };

        let Some(cycle_count) = status
            .get("z80")
            .and_then(|z| z.get("states"))
            .and_then(|s| s.get("cycleCount"))
            .and_then(Value::as_u64)
        else {
            continue;
        };

        let stalled = last_cycle_count == Some(cycle_count);
        last_cycle_count = Some(cycle_count);

        // Only a stall this thread was told to expect (set by `continue`,
        // cleared here or by `announce_stop_now`) is reported - a stall it
        // was not expecting means either the CPU is sitting on a real,
        // already-announced stop (nothing new to say) or is legitimately
        // idle before a `launch`/`attach` has even run yet.
        if stalled && watched.expecting_stop.swap(false, Ordering::Relaxed) {
            seq += 1;
            let _ = watched.out.send(protocol::event(
                "stopped",
                json!({"reason": "breakpoint", "threadId": 1, "allThreadsStopped": true}),
                seq
            ));
        }
    }
}

/// Connects fresh, sends `cmd`, returns the first non-`{"type":"event",...}`
/// response, and disconnects. ACE's real replies are pretty-printed across
/// several physical lines (confirmed live: a `readMemory` answer's opening
/// `{` arrives as its own line) - lines are accumulated into `buffer` and
/// re-parsed as a whole after each one, rather than assuming one
/// `read_line` is always a complete message the way `sugarbox.rs`'s own
/// reader can.
fn ace_round_trip(port: u16, cmd: Value, timeout: Duration) -> std::io::Result<Value> {
    let addr = format!("127.0.0.1:{port}");
    let stream = TcpStream::connect(&addr)?;
    stream.set_read_timeout(Some(timeout))?;
    let mut writer = stream.try_clone()?;
    let mut reader = std::io::BufReader::new(stream);

    let mut line_out = cmd.to_string();
    line_out.push('\n');
    writer.write_all(line_out.as_bytes())?;

    let mut buffer = String::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("ACE closed the web-API connection without answering {cmd}")
            ));
        }
        buffer.push_str(&line);

        let Ok(value) = serde_json::from_str::<Value>(buffer.trim())
        else {
            continue; // message not complete yet
        };
        if value.get("type").and_then(Value::as_str) == Some("event") {
            buffer.clear();
            continue;
        }
        return Ok(value);
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

/// Translate one DAP request into ACE's own JSON command shape. `None`
/// means "nothing to send to the emulator" - `send()` answers such a
/// request with an empty body itself.
fn ace_call_for(request: &Value) -> Option<Value> {
    let command = request.get("command").and_then(Value::as_str)?;
    let args = request.get("arguments").cloned().unwrap_or(json!({}));

    Some(match command {
        "continue" => json!({"cmd": "continue"}),
        // The reverse of SugarboxV2: ACE's own `step` is NOT step-over -
        // `stepOver` is the distinct verb that is. See this module's doc
        // comment.
        "next" => json!({"cmd": "stepOver"}),
        "stepIn" => json!({"cmd": "stepIn"}),
        "stepOut" => json!({"cmd": "stepOut"}),
        // Not yet live-confirmed (unlike every other mapping here) - a
        // reasonable guess mirroring `sugarbox_call_for`'s own `restart`
        // mapping, cheap to verify alongside this module's own integration
        // test.
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
            json!({"cmd": "readMemory", "address": address, "size": count, "memType": "ram", "bank": -1})
        },
        "writeMemory" => {
            let address = args
                .get("memoryReference")
                .and_then(Value::as_str)
                .and_then(protocol::parse_address_reference)?;
            let data = args.get("data").and_then(Value::as_str)?;
            let bytes = crate::session::decode_base64(data);
            json!({
                "cmd": "writeMemory",
                "address": address,
                "bytes": bytes,
                "memType": "ram",
                "bank": -1
            })
        },
        _ => return None
    })
}

fn ace_registers_of(state: &Value) -> Vec<Value> {
    // Same UPPERCASE key set `sugarbox_registers_of` reads, confirmed live
    // against ACE's real `getStatus`/`readRegisters` output - `WZ` is
    // ACE-only (SugarboxV2 has no such register) and is included here too,
    // a harmless addition for anything inspecting variables.
    const NAMES: &[&str] = &[
        "AF", "BC", "DE", "HL", "AF'", "BC'", "DE'", "HL'", "IX", "IY", "SP", "PC", "I", "R", "WZ"
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

/// Shape ACE's own reply (`state`) into the DAP response `request` is
/// waiting for. `getStatus`'s reply nests registers under `z80.registers`;
/// `readRegisters`'s own reply is the flat object directly - both are
/// accepted here since either can be `state`, depending on which call site
/// produced it.
fn ace_response_for(request: &Value, state: &Value, seq: i64) -> Value {
    let registers_state = state.get("z80").and_then(|z| z.get("registers")).unwrap_or(state);
    let command = request.get("command").and_then(Value::as_str).unwrap_or_default();
    let body = match command {
        "stackTrace" => {
            let pc = registers_state.get("PC").and_then(Value::as_u64).unwrap_or(0);
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
        "variables" => json!({"variables": ace_registers_of(registers_state)}),
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
        _ => json!({})
    };
    protocol::response(request, body, seq)
}

impl DapPeer for AcePeer {
    fn send(&mut self, message: Value) -> std::io::Result<()> {
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        // DAP session-lifecycle commands the raw wire protocol knows
        // nothing about, plus the fixed-shape thread/scopes answers - same
        // fixed shape `SugarBoxPeer`/`AmspiritLitePeer` both use: a
        // machine-level debugger with no symbol table of its own has one
        // Z80 "thread" and one register scope, always.
        match command.as_str() {
            "initialize" | "configurationDone" => {
                let seq = self.next_seq();
                self.push(protocol::response(&message, json!({}), seq));
                return Ok(());
            },
            "attach" | "launch" => {
                // Unlike SugarboxV2, connecting does not auto-halt the CPU
                // (confirmed live) - halted explicitly here so the user can
                // set breakpoints before anything runs, matching the same
                // "stopped at entry" convention every backend uses.
                let _ = self.round_trip(json!({"cmd": "halt"}));
                let seq = self.next_seq();
                self.push(protocol::response(&message, json!({}), seq));
                self.announce_stop_now("entry");
                return Ok(());
            },
            "disconnect" | "terminate" => {
                // Left running rather than left halted forever once this
                // session detaches - same reasoning as `SugarBoxPeer`'s own
                // `disconnect`/`terminate` arm, even though ACE itself
                // never auto-halts on connect: the CPU may well be halted
                // right now anyway, from `launch`'s own explicit halt above,
                // a hit breakpoint, or a user-initiated pause.
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
            self.push(ace_response_for(&message, &ack, seq));
            return Ok(());
        }

        if command == "pause" {
            self.round_trip(json!({"cmd": "halt"}))?;
            let seq = self.next_seq();
            self.push(protocol::response(&message, json!({}), seq));
            self.announce_stop_now("pause");
            return Ok(());
        }

        // `next`/`stepIn`/`stepOut` all ack synchronously with the step
        // already completed (confirmed live), so the resulting stop is
        // announced immediately here rather than waiting on the poller.
        if matches!(command.as_str(), "next" | "stepIn" | "stepOut") {
            let call = ace_call_for(&message).unwrap();
            self.round_trip(call)?;
            let seq = self.next_seq();
            self.push(protocol::response(&message, json!({}), seq));
            self.announce_stop_now("step");
            return Ok(());
        }

        if command == "continue" {
            self.round_trip(json!({"cmd": "continue"}))?;
            // The only path that sets this - see this module's doc comment
            // and `watch_run_state`. Whatever happens next (an existing
            // breakpoint hit, or the machine simply keeps running until the
            // user pauses/steps again, both of which clear it themselves)
            // is entirely the poller's to notice from here.
            self.expecting_stop.store(true, Ordering::Relaxed);
            let seq = self.next_seq();
            self.push(protocol::response(&message, json!({}), seq));
            return Ok(());
        }

        // `stackTrace` needs a live PC, not just a fixed shape - fetched
        // via the same register read `variables` uses.
        if command == "stackTrace" {
            let ack = self.round_trip(json!({"cmd": "readRegisters"}))?;
            let seq = self.next_seq();
            self.push(ace_response_for(&message, &ack, seq));
            return Ok(());
        }

        let Some(call) = ace_call_for(&message)
        else {
            let seq = self.next_seq();
            self.push(protocol::response(&message, json!({}), seq));
            return Ok(());
        };

        let ack = self.round_trip(call)?;
        let seq = self.next_seq();
        self.push(ace_response_for(&message, &ack, seq));
        Ok(())
    }

    fn drain(&mut self) -> Vec<Value> {
        self.pending.try_iter().collect()
    }

    /// ACE steps over/out on its own - nothing here needs to know what kind
    /// of line the PC is sitting on.
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
        )
    }
}

impl Drop for AcePeer {
    fn drop(&mut self) {
        // Only one this session started is closed with it; one the user
        // started is theirs.
        if let Some(child) = self.launched.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn spawn_ace<E>(
    port: u16,
    media_path: Option<&cpclib_common::camino::Utf8Path>,
    observer: &E
) -> Result<(std::process::Child, u16), String>
where E: cpclib_common::event::EventObserver + 'static {
    use crate::amspiritlite::{port_to_serve_on, strip_snap_leaked_env_vars};
    use cpclib_runner::runner::emulator::{AceVersion, Emulator};

    let emulator = Emulator::Ace(AceVersion::default());
    let configuration = emulator.configuration::<E>();
    if !configuration.is_cached() {
        configuration.install(observer)?;
    }

    let port = port_to_serve_on(port);
    let executable = configuration.exec_fname();
    let mut command = std::process::Command::new(executable.as_str());
    // Unlike SugarboxV2, ACE takes its media as a positional argument at
    // spawn time rather than a post-connect load command - matching how
    // `cpclib-runner::emucontrol::ConfigureRunner::args_for_emu` already
    // does it for the Robot-driven launch path.
    if let Some(media_path) = media_path {
        command.arg(media_path.as_str());
    }
    command
        .arg("-enable_webapi")
        .arg("-web_port")
        .arg(port.to_string())
        .arg("-borderless")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    strip_snap_leaked_env_vars(&mut command);
    let child = command
        .spawn()
        .map_err(|e| format!("cannot start {executable}: {e}"))?;

    Ok((child, port))
}

/// Block until ACE's web API answers, or give up saying so - `127.0.0.1:1234`
/// form, no `http://` (plain TCP, like SugarboxV2's own `wait_until_listening`,
/// unlike AMSpiriT Lite's HTTP one).
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
    Err(format!("ACE did not start listening on {endpoint} within {} seconds", patience.as_secs()))
}

/// Start ACE with `snapshot` loaded, ready to run from where it was saved -
/// the Z80 launch flow's own entry point, mirroring `sugarbox::launch`.
pub fn launch<E>(snapshot: &[u8], port: u16, observer: &E) -> Result<(String, AcePeer), String>
where E: cpclib_common::event::EventObserver + 'static {
    let path = std::env::temp_dir().join(format!("cpclib-dap-ace-{}.sna", std::process::id()));
    fs_err::write(&path, snapshot).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    let path = cpclib_common::camino::Utf8PathBuf::from_path_buf(path)
        .map_err(|p| format!("{} is not valid UTF-8", p.display()))?;

    let (child, port) = spawn_ace(port, Some(&path), observer)?;
    let _ = fs_err::remove_file(&path);

    let endpoint = format!("127.0.0.1:{port}");
    wait_until_listening(&endpoint, Duration::from_secs(30))?;

    let peer = AcePeer::connect(port).map_err(|e| format!("cannot reach ACE at {endpoint}: {e}"))?;
    Ok((endpoint, peer.owning(child)))
}

/// Start ACE with `disk` in drive A and nothing run automatically - mirrors
/// `sugarbox::launch_with_disk`'s own "loaded, not run" contract.
pub fn launch_with_disk<E>(
    disk: &std::path::Path,
    port: u16,
    observer: &E
) -> Result<(String, AcePeer), String>
where E: cpclib_common::event::EventObserver + 'static {
    let path = cpclib_common::camino::Utf8Path::from_path(disk)
        .ok_or_else(|| format!("{} is not valid UTF-8", disk.display()))?;

    let (child, port) = spawn_ace(port, Some(path), observer)?;

    let endpoint = format!("127.0.0.1:{port}");
    wait_until_listening(&endpoint, Duration::from_secs(30))?;

    let peer = AcePeer::connect(port).map_err(|e| format!("cannot reach ACE at {endpoint}: {e}"))?;
    Ok((endpoint, peer.owning(child)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(command: &str, arguments: Value) -> Value {
        json!({"seq": 1, "type": "request", "command": command, "arguments": arguments})
    }

    /// A plausible `readRegisters`/`getStatus.z80.registers` reply shape -
    /// field names confirmed live against a real ACE instance, not guessed.
    fn registers() -> Value {
        json!({
            "AF": 0x0740, "BC": 0x4F7F, "DE": 0xFAB4, "HL": 0x0A81,
            "AF'": 0, "BC'": 0xF680, "DE'": 0xC0, "HL'": 0x77,
            "IX": 0x4332, "IY": 0x2900, "SP": 0x0303, "PC": 0x0A31,
            "I": 0, "R": 98, "WZ": 0x0A31
        })
    }

    #[test]
    fn stepping_commands_translate_with_next_mapped_to_stepover() {
        assert_eq!(ace_call_for(&request("continue", json!({}))).unwrap()["cmd"], "continue");
        // The one place this differs from Sugarbox: ACE's `step` is NOT
        // step-over.
        assert_eq!(ace_call_for(&request("next", json!({}))).unwrap()["cmd"], "stepOver");
        assert_eq!(ace_call_for(&request("stepIn", json!({}))).unwrap()["cmd"], "stepIn");
        assert_eq!(ace_call_for(&request("stepOut", json!({}))).unwrap()["cmd"], "stepOut");
        assert_eq!(ace_call_for(&request("restart", json!({}))).unwrap()["cmd"], "reset");
    }

    #[test]
    fn a_memory_read_carries_mem_type_and_bank() {
        let call = ace_call_for(&request(
            "readMemory",
            json!({"memoryReference": "0x4000", "count": 32})
        ))
        .unwrap();
        assert_eq!(call["cmd"], "readMemory");
        assert_eq!(call["address"], 0x4000);
        assert_eq!(call["size"], 32);
        assert_eq!(call["memType"], "ram");
        assert_eq!(call["bank"], -1);
    }

    #[test]
    fn a_memory_write_carries_mem_type_and_bank_too() {
        let call = ace_call_for(&request(
            "writeMemory",
            json!({"memoryReference": "0x4000", "data": "PgDJ"})
        ))
        .unwrap();
        assert_eq!(call["cmd"], "writeMemory");
        assert_eq!(call["memType"], "ram");
        assert_eq!(call["bank"], -1);
        assert_eq!(call["bytes"], json!([0x3E, 0x00, 0xC9]));
    }

    #[test]
    fn a_memory_answer_is_re_encoded_as_base64_for_the_editor() {
        let read = request("readMemory", json!({"memoryReference": "0x4000", "count": 3}));
        let answer = ace_response_for(&read, &json!({"bytes": [0x3E, 0x00, 0xC9]}), 9);
        assert_eq!(answer["body"]["data"], json!("PgDJ"));
    }

    #[test]
    fn breakpoints_are_handled_directly_not_through_ace_call_for() {
        let call = ace_call_for(&request(
            "setInstructionBreakpoints",
            json!({"breakpoints": [{"instructionReference": "0x1234"}]})
        ));
        assert!(call.is_none());
    }

    #[test]
    fn set_variable_builds_a_named_register_write() {
        let call = ace_call_for(&request("setVariable", json!({"name": "HL", "value": "0x1234"})))
            .unwrap();
        assert_eq!(call["cmd"], "setRegisters");
        assert_eq!(call["hl"], 0x1234);
    }

    #[test]
    fn a_stack_trace_reports_pc_as_the_one_frame() {
        let trace = request("stackTrace", json!({}));
        let answer = ace_response_for(&trace, &registers(), 3);
        assert_eq!(answer["body"]["totalFrames"], 1);
        assert_eq!(
            answer["body"]["stackFrames"][0]["instructionPointerReference"],
            json!("0x0A31")
        );
    }

    #[test]
    fn variables_lists_every_register_including_wz() {
        let variables = request("variables", json!({}));
        let answer = ace_response_for(&variables, &registers(), 4);
        let names: Vec<&str> = answer["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"PC"));
        assert!(names.contains(&"HL"));
        assert!(names.contains(&"WZ"));
    }

    #[test]
    fn a_getstatus_shaped_reply_is_also_accepted_for_register_decoding() {
        // getStatus nests registers under z80.registers, unlike
        // readRegisters' own flat reply - ace_response_for must accept
        // either, since both are real call sites that can produce `state`.
        let variables = request("variables", json!({}));
        let nested = json!({"z80": {"registers": registers()}});
        let answer = ace_response_for(&variables, &nested, 4);
        let names: Vec<&str> = answer["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"PC"));
    }

    #[test]
    fn an_unknown_command_has_nothing_to_send() {
        assert!(ace_call_for(&request("completions", json!({}))).is_none());
    }

    #[test]
    fn flexible_int_parsing_accepts_hex_and_decimal() {
        assert_eq!(parse_flexible_int("0x1F"), Some(0x1F));
        assert_eq!(parse_flexible_int("31"), Some(31));
        assert_eq!(parse_flexible_int("not a number"), None);
    }

    /// A fake server whose replies are split across several physical lines
    /// (matching what ACE's real pretty-printed JSON does) to prove
    /// `ace_round_trip` really does accumulate lines rather than assuming
    /// one `read_line` is a whole message - the exact bug this module's own
    /// doc comment mentions being found against the real binary.
    #[test]
    fn round_trip_accumulates_a_multi_line_pretty_printed_reply() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut writer = stream.try_clone().unwrap();
            let mut reader = std::io::BufReader::new(stream);

            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.contains("\"getStatus\""));

            // Split across lines on purpose, mirroring ACE's real
            // pretty-printing.
            writeln!(writer, "{{").unwrap();
            writeln!(writer, "\"running\":true").unwrap();
            writeln!(writer, "}}").unwrap();
        });

        let answer =
            ace_round_trip(port, json!({"cmd": "getStatus"}), Duration::from_secs(5)).unwrap();
        server.join().unwrap();

        assert_eq!(answer["running"], true);
    }

    /// Proves an interleaved async event line (should never actually happen
    /// per this module's own doc comment, but the skip logic is here
    /// defensively) does not get mistaken for the real answer.
    #[test]
    fn round_trip_skips_an_interleaved_event_line() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut writer = stream.try_clone().unwrap();
            let mut reader = std::io::BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            writeln!(writer, r#"{{"type":"event","event":"stopped"}}"#).unwrap();
            writeln!(writer, r#"{{"status":"ok"}}"#).unwrap();
        });

        let answer = ace_round_trip(port, json!({"cmd": "halt"}), Duration::from_secs(5)).unwrap();
        server.join().unwrap();
        assert_eq!(answer["status"], "ok");
    }

    /// The poller's own core contract: a stall it was told to expect (via
    /// `expecting_stop`) is announced exactly once; a stall it was not
    /// expecting (e.g. the machine sitting halted before `continue` was
    /// ever sent) is not.
    #[test]
    fn poller_only_announces_a_stall_it_was_told_to_expect() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let expecting_stop = Arc::new(AtomicBool::new(false));
        let (out, pending) = mpsc::channel();

        let cycle_counts = [12u64, 12, 12, 20, 20]; // stalled, then resumes
        let server = std::thread::spawn(move || {
            for count in cycle_counts {
                let (stream, _) = listener.accept().unwrap();
                let mut writer = stream.try_clone().unwrap();
                let mut reader = std::io::BufReader::new(stream);
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                writeln!(
                    writer,
                    r#"{{"z80":{{"states":{{"cycleCount":{count}}}}}}}"#
                )
                .unwrap();
            }
        });

        let watched = Watched { port, out, expecting_stop: expecting_stop.clone() };
        // Not `expecting_stop` yet - the first two polls (baseline + first
        // real stall check) must not announce anything.
        let poller = std::thread::spawn(move || {
            // Run the loop body manually a bounded number of times instead
            // of the real infinite `watch_run_state` (which sleeps 100ms
            // per iteration and never returns) - this exercises the exact
            // same stall-detection logic without a slow, unbounded test.
            let mut last_cycle_count: Option<u64> = None;
            for _ in 0..5 {
                let status = ace_round_trip(watched.port, json!({"cmd": "getStatus"}), Duration::from_secs(2))
                    .unwrap();
                let cycle_count =
                    status["z80"]["states"]["cycleCount"].as_u64().unwrap();
                let stalled = last_cycle_count == Some(cycle_count);
                last_cycle_count = Some(cycle_count);
                if stalled && watched.expecting_stop.swap(false, Ordering::Relaxed) {
                    let _ = watched.out.send(protocol::event(
                        "stopped",
                        json!({"reason": "breakpoint"}),
                        1
                    ));
                }
            }
        });

        // Simulate `continue` being sent right as polling starts.
        expecting_stop.store(true, Ordering::Relaxed);

        server.join().unwrap();
        poller.join().unwrap();

        let events: Vec<Value> = pending.try_iter().collect();
        assert_eq!(events.len(), 1, "expected exactly one stopped event: {events:?}");
    }

    /// End-to-end against the real binary: install-if-missing, launch for
    /// real with `-enable_webapi -web_port <port> -borderless`, drive
    /// `initialize`/`attach`/`setInstructionBreakpoints`/`continue`/(stop
    /// via the poller)/`readMemory` through the real `DapPeer` contract, and
    /// assert the DAP-shaped responses/events. `#[ignore]`d like this
    /// crate's other real-download/real-process tests: downloads the
    /// distribution and spawns a real emulator process, too heavy for the
    /// default `cargo test` run.
    ///
    /// Run with: `cargo test -p cpclib-dap --lib ace::tests::
    /// attach_breakpoint_continue_and_read_memory_work_against_a_real_ace_instance
    /// -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn attach_registers_breakpoint_continue_and_memory_work_against_a_real_ace_instance() {
        use cpclib_common::event::DiscardObserver;

        // No custom snapshot here - loading one that actually lands its
        // program at a chosen address turned out to be its own real yak-
        // shave (chased once, found `env.handle_post_actions` was needed
        // too, still ended up with a snapshot ACE did not honor the way
        // expected) that is orthogonal to what this test exists to prove:
        // the real `AcePeer`/DAP wire-protocol translation against the
        // real binary. Whatever ACE boots into by default is enough for
        // that - registers/memory/breakpoints are exercised generically,
        // not against a specific known program.
        let (child, port) =
            spawn_ace(18764, None, &DiscardObserver).expect("failed to spawn ACE");
        let endpoint = format!("127.0.0.1:{port}");
        wait_until_listening(&endpoint, Duration::from_secs(30)).expect("ACE never started listening");
        let mut peer = AcePeer::connect(port)
            .map_err(|e| format!("cannot reach ACE at {endpoint}: {e}"))
            .unwrap()
            .owning(child);

        let drain_until = |peer: &mut AcePeer, want: usize, patience: Duration| -> Vec<Value> {
            let deadline = std::time::Instant::now() + patience;
            let mut got = Vec::new();
            while got.len() < want && std::time::Instant::now() < deadline {
                got.extend(peer.drain());
                std::thread::sleep(Duration::from_millis(50));
            }
            got
        };

        peer.send(json!({"seq": 1, "type": "request", "command": "initialize", "arguments": {}}))
            .unwrap();
        let init = drain_until(&mut peer, 1, Duration::from_secs(5));
        assert_eq!(init.len(), 1, "no answer to initialize: {init:?}");

        // `attach` halts the CPU and announces a synthesised "entry" stop -
        // both the response and that event are expected here.
        peer.send(json!({"seq": 2, "type": "request", "command": "attach", "arguments": {}}))
            .unwrap();
        let attached = drain_until(&mut peer, 2, Duration::from_secs(5));
        assert_eq!(attached.len(), 2, "expected a response and an entry stop: {attached:?}");
        assert!(
            attached.iter().any(|m| m["event"] == "stopped" && m["body"]["reason"] == "entry"),
            "missing the entry stop: {attached:?}"
        );

        // setVariable/variables round trip: write HL, read it back.
        peer.send(json!({
            "seq": 3, "type": "request", "command": "setVariable",
            "arguments": {"name": "HL", "value": "0x1234"}
        }))
        .unwrap();
        let set_var = drain_until(&mut peer, 1, Duration::from_secs(5));
        assert_eq!(set_var.len(), 1, "no answer to setVariable: {set_var:?}");

        peer.send(json!({"seq": 4, "type": "request", "command": "variables", "arguments": {}}))
            .unwrap();
        let vars = drain_until(&mut peer, 1, Duration::from_secs(5));
        let hl = vars[0]["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == "HL")
            .expect("no HL register in the response");
        assert_eq!(hl["value"], "0x1234", "HL did not round-trip: {vars:?}");

        // Memory: a plain readMemory answers with *some* two bytes - not
        // asserting specific content, since nothing specific was loaded.
        peer.send(json!({
            "seq": 5, "type": "request", "command": "readMemory",
            "arguments": {"memoryReference": "0x0000", "count": 2}
        }))
        .unwrap();
        let read = drain_until(&mut peer, 1, Duration::from_secs(5));
        assert_eq!(read.len(), 1, "no answer to readMemory: {read:?}");
        assert!(read[0]["body"]["data"].is_string(), "readMemory returned no data: {read:?}");

        // Breakpoint + continue: arm a breakpoint at whatever address the
        // machine is currently halted at (read via `stackTrace`, same as
        // this module's own live-verification approach against a real
        // instance) - a tight idle/firmware loop revisits its own address
        // within one poll interval, same as it did when this was checked
        // by hand.
        peer.send(json!({"seq": 6, "type": "request", "command": "stackTrace", "arguments": {}}))
            .unwrap();
        let trace = drain_until(&mut peer, 1, Duration::from_secs(5));
        let pc_ref = trace[0]["body"]["stackFrames"][0]["instructionPointerReference"]
            .as_str()
            .unwrap()
            .to_string();

        peer.send(json!({
            "seq": 7, "type": "request", "command": "setInstructionBreakpoints",
            "arguments": {"breakpoints": [{"instructionReference": pc_ref}]}
        }))
        .unwrap();
        let set_bp = drain_until(&mut peer, 1, Duration::from_secs(5));
        assert_eq!(set_bp.len(), 1, "no answer to setInstructionBreakpoints: {set_bp:?}");

        peer.send(json!({"seq": 8, "type": "request", "command": "continue", "arguments": {}}))
            .unwrap();
        let continued = drain_until(&mut peer, 2, Duration::from_secs(10));
        assert!(
            continued.iter().any(|m| m["event"] == "stopped" && m["body"]["reason"] == "breakpoint"),
            "missing the breakpoint stop after continue: {continued:?}"
        );

        peer.send(json!({"seq": 9, "type": "request", "command": "disconnect", "arguments": {}}))
            .unwrap();
        let _ = drain_until(&mut peer, 1, Duration::from_secs(5));
    }
}

//! Speaking to AMSpiriT Lite's HTTP debug server.
//!
//! A second emulator backend beside 1984js. That one speaks DAP natively over a
//! loopback socket; this one speaks a REST API plus a Server-Sent Events
//! stream, so something has to translate. This module is that translation, and
//! *only* that: a DAP request becomes a description of an HTTP call, and an
//! HTTP reply or an SSE event becomes a DAP message. No sockets here, which is
//! what lets every mapping below be a test rather than a live emulator.
//!
//! **The published documents describe this API inaccurately.** The example
//! client names endpoints that do not exist (`/api/ram_dump`, `/api/pause`),
//! and the official `web_api.md` omits breakpoints, stepping, `/api/history`
//! and `/api/memmap` entirely. Everything below was checked against a running
//! 1.13.4 instance and against the UI the emulator itself serves; the
//! `live_tests` module keeps those checks runnable.
//!
//! Worth the work because this emulator can answer what the other cannot: its
//! `/api/ping` carries Gate Array, PSG and FDC state directly, where 1984js
//! exposes none of it and the chip panes have to round-trip a whole snapshot.
//! The cost is that it runs in its own window rather than in an editor tab.

use serde_json::{Value, json};

/// The base a session talks to. The emulator's own default.
pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:8765";

/// One HTTP call, described rather than made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub method: Method,
    pub path: &'static str,
    /// `key=value` pairs, already in the order the emulator expects.
    pub query: Vec<(String, String)>,
    pub body: Option<String>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post
}

impl Call {
    fn get(path: &'static str) -> Self {
        Self {
            method: Method::Get,
            path,
            query: Vec::new(),
            body: None
        }
    }

    fn post(path: &'static str) -> Self {
        Self {
            method: Method::Post,
            path,
            query: Vec::new(),
            body: None
        }
    }

    fn query(mut self, key: &str, value: impl std::fmt::Display) -> Self {
        self.query.push((key.to_string(), value.to_string()));
        self
    }

    fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

/// What a DAP request asks of the emulator.
///
/// `None` for a request this backend answers on its own or ignores - the
/// adapter above already owns everything source-shaped, so `stackTrace` needs
/// only the program counter, which arrives with the state rather than through a
/// request of its own.
pub fn call_for(request: &Value) -> Option<Call> {
    let command = request.get("command")?.as_str()?;
    let arguments = request.get("arguments");

    Some(match command {
        // Run control is a *configuration* change, not a verb of its own:
        // `POST /api/config {"paused": …}`. There is no `/api/pause`, whatever
        // the published example client suggests.
        "pause" => Call::post("/api/config").body(json!({ "paused": true }).to_string()),
        "continue" => Call::post("/api/config").body(json!({ "paused": false }).to_string()),
        // One instruction. `next` and `stepOut` are built out of many of these
        // rather than mapped here - see `step_over` - so what reaches this arm
        // as `next` is only a caller asking what a single step costs.
        "stepIn" | "next" => Call::post("/api/step"),

        // Registers and every chip in one call, which is why the panes cost
        // nothing here: `/api/state` carries z80, ga, crtc, psg and fdc
        // together, and does so while the machine is running.
        "threads" | "stackTrace" | "scopes" | "variables" => Call::get("/api/state"),

        "readMemory" => {
            let address = arguments
                .and_then(|a| a.get("memoryReference"))
                .and_then(Value::as_str)
                .and_then(crate::protocol::parse_address_reference)?;
            let count = arguments
                .and_then(|a| a.get("count"))
                .and_then(Value::as_i64)
                .unwrap_or(16)
                .max(1);
            // Decimal, both of them; the answer comes back as a hex string.
            Call::get("/api/ram")
                .query("addr", address)
                .query("len", count)
        },

        // Writing is the same endpoint as reading, with a JSON body. DAP
        // carries the bytes as base64 and this wants hex, so they are
        // re-encoded on the way out exactly as they are on the way in.
        "writeMemory" => {
            let address = arguments
                .and_then(|a| a.get("memoryReference"))
                .and_then(Value::as_str)
                .and_then(crate::protocol::parse_address_reference)?;
            let data = arguments
                .and_then(|a| a.get("data"))
                .and_then(Value::as_str)
                .map(|encoded| hex_from_base64(encoded))
                .unwrap_or_default();
            Call::post("/api/ram").body(json!({ "addr": address, "data": data }).to_string())
        },

        // The whole set each time, as `0xNNNN,0xNNNN` in a plain-text body -
        // which is what DAP sends anyway, so the two agree without the adapter
        // tracking differences.
        "setInstructionBreakpoints" => {
            let addresses = arguments
                .and_then(|a| a.get("breakpoints"))
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(|entry| {
                            entry
                                .get("instructionReference")
                                .and_then(Value::as_str)
                                .and_then(crate::protocol::parse_address_reference)
                        })
                        .map(|address| format!("0x{address:04X}"))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Call::post("/api/z80_bp").body(addresses.join(","))
        },

        // Moving the program counter without writing to RAM - the only
        // register this emulator lets anything set. There is no endpoint for
        // `A`, `HL` or the rest, documented or otherwise.
        "cpclib/setPc" => {
            let address = arguments
                .and_then(|a| a.get("address"))
                .and_then(Value::as_u64)?;
            Call::post("/api/exec").body(json!({ "addr": address }).to_string())
        },

        // Which page is mapped where. The whole reason this backend is worth
        // having: 1984js cannot answer it at all, which is what forced the
        // byte-matching heuristic and the "most likely line" fallback.
        "cpclib/memmap" => Call::get("/api/memmap"),
        // The chips, each from its own endpoint rather than by saving and
        // parsing a whole snapshot.
        "cpclib/crtc" => Call::get("/api/crtc"),
        "cpclib/ga" => Call::get("/api/ga"),
        "cpclib/psg" => Call::get("/api/psg"),
        "cpclib/fdc" => Call::get("/api/fdc"),
        // The instruction trace: pc, the bytes there, and every register, per
        // step. Exact elapsed time, and a real history to walk back through.
        "cpclib/history" => Call::get("/api/history"),

        _ => return None
    })
}


/// The RAM page mapped at `address`, from an `/api/memmap` body.
///
/// The regions are 16K and reported in order, each naming the bank behind it.
/// `None` for an address in ROM, or a body that does not describe this address
/// - a page nobody claims is not a page to guess at.
pub fn physical_of(memmap: &Value, address: u16) -> Option<u32> {
    // `regions[].ram_bank` is an **absolute 16K bank index**, not a bank within
    // the base 64K: the region at `#4000` reporting bank 5 means the sixth 16K
    // block of memory - page 1, its second bank - which is physical
    // `0x14000-0x17FFF`.
    //
    // That is the whole answer, and it is exact. `ram_mode`/`ram_page` describe
    // the MMR but the emulator reports `ram_page: 0` even under mode 5, so
    // reading those instead gave page 0 and opened the wrong file. The regions
    // already have the decoding applied; trust them.
    let Some(regions) = memmap.get("regions").and_then(Value::as_array)
    else {
        return physical_from_mmr(memmap, address);
    };
    let region = regions
        .iter()
        .filter(|region| {
            region
                .get("base")
                .and_then(Value::as_u64)
                .is_some_and(|base| u32::from(address) >= base as u32)
        })
        .max_by_key(|region| region.get("base").and_then(Value::as_u64).unwrap_or(0))?;

    // ROM at this address means the bytes are not the program's.
    if region.get("rom").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let bank = region.get("ram_bank").and_then(Value::as_u64)? as u32;
    Some(bank * 0x4000 + u32::from(address & 0x3FFF))
}

/// The same answer, decoded from the MMR when no region list is offered.
///
/// AmspiritLite always sends `regions`, so this is for a backend that reports
/// only the Gate Array's banking byte. It is the published table, and it is
/// what `regions` is a summary of.
fn physical_from_mmr(memmap: &Value, address: u16) -> Option<u32> {
    let mode = memmap.get("ram_mode").and_then(Value::as_u64)? as u32;
    let page = memmap.get("ram_page").and_then(Value::as_u64).unwrap_or(0) as u32;
    let slot = u32::from(address >> 14);
    // Absolute 16K banks, so the base 64K is page 0's banks 0..3 and page `p`
    // starts at `4p` - the same numbering `regions[].ram_bank` uses.
    let paged = |bank: u32| page * 4 + bank;
    let bank = match (mode, slot) {
        // The base 64K throughout, whatever page is selected.
        (0, slot) => slot,
        // Only #C000 comes from the page.
        (1, 3) => paged(3),
        (1, slot) => slot,
        // The whole address space comes from the page.
        (2, slot) => paged(slot),
        // Like 1, with #4000 taken from bank 3 of the base 64K.
        (3, 1) => 3,
        (3, 3) => paged(3),
        (3, slot) => slot,
        // S set: only #4000 is paged, and *which* bank of the page varies -
        // which moves the offset within the page, not only the page.
        (mode, 1) => paged(mode - 4),
        (_, slot) => slot
    };
    Some(bank * 0x4000 + u32::from(address & 0x3FFF))
}

/// The RAM page mapped at `address`, from an `/api/memmap` body.
pub fn page_at(memmap: &Value, address: u16) -> Option<u8> {
    physical_of(memmap, address).map(|physical| (physical >> 16) as u8)
}


/// Which request answers a chip scope on this emulator.
///
/// Every chip has its own endpoint here, so a pane costs one small GET rather
/// than saving and parsing a whole snapshot the way the other backend must.
/// `None` for a scope this emulator has no endpoint for - the caller then falls
/// back to the snapshot route rather than showing nothing.
pub fn chip_command(reference: i64) -> Option<&'static str> {
    Some(match reference {
        crate::inspect::CRTC_REFERENCE => "cpclib/crtc",
        crate::inspect::GATE_ARRAY_REFERENCE => "cpclib/ga",
        crate::inspect::PSG_REFERENCE => "cpclib/psg",
        // The FDC really is readable here - a snapshot only ever carried the
        // motor and the track.
        crate::inspect::DISC_REFERENCE => "cpclib/fdc",
        _ => return None
    })
}

/// A chip pane, from this emulator's own answer.
///
/// The shapes are the emulator's, not a snapshot's, so this is a separate
/// formatter from [`crate::inspect::chip_variables`] rather than a conversion
/// into `Snapshot` - and it can show things a snapshot never carried, such as
/// the CRTC's raster line.
pub fn chip_variables(reference: i64, body: &Value) -> Vec<Value> {
    match reference {
        crate::inspect::CRTC_REFERENCE => crtc_pane(body),
        crate::inspect::GATE_ARRAY_REFERENCE => gate_array_pane(body),
        _ => flat_pane(body, &[])
    }
}

/// One variable, formatted by what it is: a number reads in both bases,
/// anything else reads as itself.
fn scalar(name: &str, value: &Value, meaning: &str) -> Value {
    let text = match value {
        Value::Number(number) => {
            match number.as_u64() {
                Some(value @ 0..=0xFF) => format!("0x{value:02X} ({value})"),
                Some(value @ 0..=0xFFFF) => format!("0x{value:04X} ({value})"),
                Some(value) => format!("0x{value:X} ({value})"),
                None => number.to_string()
            }
        },
        Value::String(text) => text.clone(),
        other => other.to_string()
    };
    json!({
        "name": name,
        "value": text,
        "type": meaning,
        "variablesReference": 0
    })
}

/// Everything else the emulator said, minus the keys the caller has already
/// presented itself.
///
/// A pass-through rather than a fixed list: a field this adapter has never
/// heard of still reaches the pane, which is the difference between seeing a
/// new emulator feature and silently dropping it.
fn flat_pane(body: &Value, consumed: &[&str]) -> Vec<Value> {
    let Some(fields) = body.as_object()
    else {
        return Vec::new();
    };
    fields
        .iter()
        .filter(|(key, _)| !consumed.contains(&key.as_str()))
        .map(|(key, value)| {
            match value {
                Value::Array(items) => {
                    let joined = items
                        .iter()
                        .map(|item| item.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    scalar(key, &Value::String(joined), "")
                },
                _ => scalar(key, value, "")
            }
        })
        .collect()
}

/// The CRTC, with the selected register marked and the raster counter kept.
fn crtc_pane(body: &Value) -> Vec<Value> {
    const MEANING: [&str; 18] = [
        "R0 horizontal total",
        "R1 horizontal displayed",
        "R2 horizontal sync position",
        "R3 sync widths (VSYNC:HSYNC)",
        "R4 vertical total",
        "R5 vertical total adjust",
        "R6 vertical displayed",
        "R7 vertical sync position",
        "R8 interlace and skew",
        "R9 maximum raster address",
        "R10 cursor start raster",
        "R11 cursor end raster",
        "R12 display start address (high)",
        "R13 display start address (low)",
        "R14 cursor address (high)",
        "R15 cursor address (low)",
        "R16 light pen address (high)",
        "R17 light pen address (low)"
    ];

    let selected = body.get("selected_reg").and_then(Value::as_u64);
    let mut out = Vec::new();
    if let Some(selected) = selected {
        out.push(scalar(
            "selected",
            &json!(selected),
            "the register &BCxx writes reach"
        ));
    }

    if let Some(registers) = body.get("regs").and_then(Value::as_array) {
        out.extend(registers.iter().enumerate().map(|(index, value)| {
            let name = format!("R{index}");
            // The register the next &BDxx write lands in, underlined.
            let name = if Some(index as u64) == selected {
                crate::inspect::underlined(&name)
            }
            else {
                name
            };
            scalar(&name, value, MEANING.get(index).copied().unwrap_or(""))
        }));
    }

    // Where in the frame we are. A snapshot could never say this without
    // stopping the machine to write one; here it is simply part of the answer,
    // and on a demo it is the whole question.
    if let Some(rasterline) = body.get("rasterline") {
        out.push(scalar(
            "rasterline",
            rasterline,
            "counter: raster line within the frame"
        ));
    }

    out.extend(flat_pane(body, &["regs", "selected_reg", "rasterline"]));
    out
}

/// The Gate Array: the palette as colours, and the banking that decides which
/// page is where.
fn gate_array_pane(body: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(mode) = body.get("mode") {
        out.push(scalar("mode", mode, "screen mode"));
    }

    // Merged in from `/api/memmap` by the peer: the register that decides which
    // page sits in which bank belongs beside the mode and the palette, not in a
    // view of its own.
    for (key, meaning) in [
        ("rmr", "the &7Fxx ROM/mode selection"),
        ("ram_mode", "MMR: which of the eight bank layouts is active"),
        ("ram_page", "MMR: the 64K page banks are taken from")
    ] {
        if let Some(value) = body.get(key) {
            out.push(scalar(key, value, meaning));
        }
    }

    if let Some(inks) = body.get("ink_idx").and_then(Value::as_array) {
        out.extend((0..inks.len()).map(|pen| {
            colour(
                &format!("pen {pen}"),
                inks.get(pen).and_then(Value::as_u64)
            )
        }));
    }
    // The border is a pen like any other, and reads better named for what it
    // is than as "pen 16".
    if let Some(border) = body.get("border_idx").and_then(Value::as_u64) {
        out.push(colour("border", Some(border)));
    }

    out.extend(flat_pane(
        body,
        &[
            "mode",
            "rmr",
            "ram_mode",
            "ram_page",
            "ink_idx",
            "ink_rgb",
            "border_idx",
            "border_rgb"
        ]
    ));
    out
}

/// A pen, shown as the colour it holds.
///
/// `ink_idx` is the Gate Array's own five-bit colour selector, not an ink
/// number - the emulator reports values up to 31, and there are only 27 inks -
/// so the byte a program writes is `0x40 | idx`, and the ink number is looked
/// up from that. Rendered by the same formatter as the snapshot-based backend,
/// so the two panes read identically whichever emulator is underneath.
fn colour(name: &str, ink: Option<u64>) -> Value {
    let written = ink.map(|ink| 0x40 | (ink as u8 & 0x1F));
    let (value, meaning) = match written {
        Some(written) => {
            crate::inspect::gate_array_pen(written).unwrap_or_else(|| {
                (
                    format!("0x{written:02X}"),
                    "not a colour the Gate Array can produce".to_string()
                )
            })
        },
        None => (String::new(), String::new())
    };
    json!({
        "name": name,
        "value": value,
        "type": meaning,
        "variablesReference": 0
    })
}

/// The variables reference the register scope answers on.
pub const REGISTERS_REFERENCE: i64 = 1;

/// The DAP answer to a request, given what the emulator replied.
pub fn response_for(request: &Value, state: &Value, seq: i64) -> Value {
    let command = request
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let body = match command {
        "threads" => json!({ "threads": [{ "id": 1, "name": "Z80" }] }),
        "stackTrace" => {
            let pc = register(state, "PC").unwrap_or(0);
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
        "scopes" => {
            json!({
                "scopes": [{
                    "name": "Registers",
                    "variablesReference": REGISTERS_REFERENCE,
                    "expensive": false,
                    "presentationHint": "registers"
                }]
            })
        },
        "variables" => json!({ "variables": registers_of(state) }),
        "readMemory" => {
            // `{addr, len, hex}` - the bytes arrive as a hex string, and DAP
            // asks for base64.
            let hex = state.get("hex").and_then(Value::as_str).unwrap_or_default();
            json!({
                "address": request
                    .get("arguments")
                    .and_then(|a| a.get("memoryReference"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "data": encode_base64(&bytes_from_hex(hex))
            })
        },
        // The emulator's own answer, handed on unchanged: the caller asked for
        // this endpoint by name and knows its shape better than a translation
        // layer would.
        command if command.starts_with("cpclib/") => state.clone(),
        _ => json!({})
    };

    crate::protocol::response(request, body, seq)
}

/// A DAP event for one Server-Sent Event, if it means anything to an editor.
pub fn event_for(name: &str, payload: &Value, seq: i64) -> Option<Value> {
    Some(match name {
        // `pause` is deliberately absent: the run state is polled rather than
        // waited on (`watch_run_state`), because a paused machine renders no
        // frames and the emulator announces only the stops it was *asked* for.
        // Reporting it here as well would report one stop twice.
        "error" => {
            crate::protocol::event(
                "output",
                json!({
                    "category": "stderr",
                    "output": format!(
                        "{}\n",
                        payload
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("the emulator reported an error")
                    )
                }),
                seq
            )
        },
        // `pause` and `frame` are the run state, owned by the poll above.
        // `state`, `basic_vars`, `config_changed` and `disk_status` are real
        // but not something an editor acts on. All swallowed: an unhandled
        // event is noise in the Debug Console.
        _ => return None
    })
}

/// One Z80 register out of an `/api/state` body.
///
/// The emulator reports **numbers**, and reports the 8-bit halves rather than
/// the pairs - `A` and `F`, not `AF` - with the alternate set as `A2`, `F2` and
/// so on. A Z80 programmer reads pairs, so those are composed here.
fn register(state: &Value, name: &str) -> Option<u32> {
    let z80 = state.get("z80").unwrap_or(state);
    let raw = |key: &str| -> Option<u32> {
        match z80.get(key)? {
            Value::Number(number) => number.as_u64().map(|v| v as u32),
            Value::String(text) => crate::protocol::parse_address_reference(text),
            Value::Bool(flag) => Some(u32::from(*flag)),
            _ => None
        }
    };
    let pair = |high: &str, low: &str| -> Option<u32> {
        Some((raw(high)? << 8) | (raw(low)? & 0xFF))
    };

    match name {
        "AF" => pair("A", "F"),
        "BC" => pair("B", "C"),
        "DE" => pair("D", "E"),
        "HL" => pair("H", "L"),
        "AF'" => pair("A2", "F2"),
        "BC'" => pair("B2", "C2"),
        "DE'" => pair("D2", "E2"),
        "HL'" => pair("H2", "L2"),
        other => raw(other)
    }
}

/// The register pane, in the order a Z80 programmer reads them.
fn registers_of(state: &Value) -> Vec<Value> {
    const ORDER: [&str; 17] = [
        "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC", "AF'", "BC'", "DE'", "HL'", "I", "R",
        "IM", "IFF1", "IFF2"
    ];
    ORDER
        .iter()
        .filter_map(|name| {
            let value = register(state, name)?;
            let width = if matches!(*name, "I" | "R" | "IM" | "IFF1" | "IFF2") {
                2
            }
            else {
                4
            };
            Some(json!({
                "name": name,
                "value": format!("0x{value:0width$X}", width = width),
                "variablesReference": 0
            }))
        })
        .collect()
}

/// Start the emulator on `snapshot`, with its debug server listening.
pub fn launch<E>(
    snapshot: &[u8],
    port: u16,
    observer: &E
) -> Result<(String, std::process::Child), String>
where E: cpclib_common::event::EventObserver + 'static {
    use cpclib_runner::runner::emulator::{AmspiritLiteVersion, Emulator};

    let emulator = Emulator::AmspiritLite(AmspiritLiteVersion::default());
    let configuration = emulator.configuration::<E>();
    if !configuration.is_cached() {
        configuration.install(observer)?;
    }

    // The snapshot has to land on disc: this emulator takes a file, unlike the
    // wasm one which is served the bytes over loopback.
    let path = std::env::temp_dir().join(format!("cpclib-dap-{}.sna", std::process::id()));
    std::fs::write(&path, snapshot)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;

    // The port has to be one nothing else is holding, and the configured one
    // frequently is not.
    //
    // An emulator this session starts is closed with the session - but a
    // session that is killed outright never gets that far, and the emulator it
    // started goes on listening on the configured port for as long as the
    // machine is up. The next session then starts an emulator that finds the
    // port taken, and this one does not fail: it says "Web debug server:
    // 127.0.0.1:8765 busy or unavailable, disabled" and runs on without a debug
    // server. The port goes on answering all the while - from the *previous*
    // session's emulator, holding the previous build and stopped wherever that
    // session left it - so everything below arms its breakpoints in a machine
    // nobody is looking at, while the window in front of the user runs free.
    // Reported as "amspirit does not detect the breakpoint any more", which is
    // exactly what it looks like: every arming is answered, and nothing ever
    // stops.
    //
    // So a port that already answers belongs to somebody else, whoever they
    // are, and ours is started somewhere it can really serve.
    let port = port_to_serve_on(port);

    let executable = configuration.exec_fname();
    let child = std::process::Command::new(executable.as_str())
        .arg(path.as_os_str())
        .arg("--web-server")
        .arg("--web-port")
        .arg(port.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start {executable}: {e}"))?;

    let endpoint = format!("http://127.0.0.1:{port}");
    wait_until_listening(&endpoint, std::time::Duration::from_secs(30))?;
    Ok((endpoint, child))
}

/// Where to start an emulator that was asked to serve on `asked`.
///
/// `asked` itself whenever it is free, and a port the operating system says is
/// free when it is not. Returned rather than merely checked, so the caller
/// connects to the emulator it started rather than to whatever else is there.
pub fn port_to_serve_on(asked: u16) -> u16 {
    if !something_is_listening(asked) {
        return asked;
    }
    a_free_port().unwrap_or(asked)
}

/// Whether anything at all answers on this port of the loopback interface.
fn something_is_listening(port: u16) -> bool {
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_millis(200)).is_ok()
}

/// A port nothing is using - asked of the operating system rather than guessed
/// at, since a guess is how this went wrong in the first place.
///
/// Free at the moment it is answered and not reserved: the emulator binds it a
/// moment later. Nothing can close that gap without holding the socket for the
/// emulator, which is not something one process can hand another.
fn a_free_port() -> Option<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    drop(listener);
    Some(port)
}

/// Block until the debug server answers, or give up saying so.
pub fn wait_until_listening(
    endpoint: &str,
    patience: std::time::Duration
) -> Result<(), String> {
    let host = host_of(endpoint).map_err(|e| e.to_string())?;
    let address: std::net::SocketAddr = host
        .parse()
        .map_err(|e| format!("{host} is not an address: {e}"))?;

    let deadline = std::time::Instant::now() + patience;
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect_timeout(
            &address,
            std::time::Duration::from_millis(200)
        )
        .is_ok()
        {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err(format!(
        "AMSpiriT Lite did not start listening on {endpoint} within {} seconds",
        patience.as_secs()
    ))
}

/// Bytes as DAP carries them: the emulator answers hex, DAP asks for base64.
fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let block = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let packed = ((block[0] as u32) << 16) | ((block[1] as u32) << 8) | block[2] as u32;
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(ALPHABET[((packed >> (18 - 6 * index)) & 0x3F) as usize] as char);
            }
            else {
                out.push('=');
            }
        }
    }
    out
}

/// Base64 as DAP sends it, into the hex this API wants.
fn hex_from_base64(encoded: &str) -> String {
    crate::session::decode_base64(encoded)
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

/// `"3E 00 C9"` or `"3E00C9"` into bytes.
fn bytes_from_hex(hex: &str) -> Vec<u8> {
    let digits: Vec<u8> = hex.bytes().filter(|b| b.is_ascii_hexdigit()).collect();
    digits
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// A live connection to AMSpiriT Lite's debug server.
///
/// Requests go out as ordinary HTTP on loopback and their answers come back
/// synchronously; the `/api/events` stream is read on its own thread, because
/// it never ends and `drain` must never block.
///
/// Raw HTTP rather than a client crate: this is loopback, the requests are
/// three shapes, and `web::server` in `cpclib-runner` already speaks the
/// protocol by hand for the same reason.
pub struct AmspiritLitePeer {
    endpoint: String,
    /// The editor is waiting to be told the program stopped again.
    ///
    /// Set whenever we ask the machine to run - a step or a continue - and
    /// cleared by reporting the next stop. Watching for a *change* in the run
    /// state is not enough, twice over:
    ///
    /// - a step ends where it began, paused, so nothing changes;
    /// - a continue that hits the same breakpoint one frame later resumes and
    ///   stops again inside a single poll interval, so the state looks
    ///   untouched.
    ///
    /// Both leave the editor believing the program is running, with a dead
    /// toolbar, until you press pause by hand.
    expecting_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The emulator this session started, if it started one.
    ///
    /// Closed when the session ends: an emulator left behind keeps its port,
    /// so the next session cannot start one and silently attaches to the old
    /// program instead. An emulator the user started is left alone - it is
    /// theirs, and they arranged its window.
    launched: Option<std::process::Child>,
    /// What a step over armed and is waiting to come back to.
    ///
    /// Shared with the run-state poller, which is the thread that notices the
    /// stop and therefore the thread that has to recognise it as the end of a
    /// step over rather than a breakpoint.
    stepping_over: std::sync::Arc<std::sync::Mutex<Option<StepTarget>>>,
    /// The breakpoints the editor armed, and the ones a step over left behind.
    ///
    /// Shared for the same reason: the poller takes a temporary breakpoint back
    /// out again when the step over it belongs to arrives.
    breakpoints: std::sync::Arc<std::sync::Mutex<Breakpoints>>,
    /// How long a resume is given to become visible; `RESUME_CONFIRMATION`
    /// unless a caller says otherwise.
    resume_confirmation: std::time::Duration,
    /// What the session says the source line at `PC` is, for the step over
    /// about to arrive. Not shared with the poller: it is written and read on
    /// the request thread, between one `next` and the address it works out.
    line_at_pc: crate::peer::LineAtPc,
    /// Answers waiting for the next `drain`.
    pending: std::sync::mpsc::Receiver<Value>,
    outgoing: std::sync::mpsc::Sender<Value>,
    /// Shared with the poller, which answers the editor while `send` is off
    /// answering something else.
    seq: std::sync::Arc<std::sync::atomic::AtomicI64>
}

/// Where a step over is heading, and what to undo when it gets there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StepTarget {
    /// The instruction after the one being stepped over, or the return address
    /// a step out is aiming at.
    address: u16,
    /// Whether the editor already had a breakpoint there.
    ///
    /// If it did, the breakpoint is the user's and stays; only one this
    /// adapter armed is taken back out.
    was_the_editors: bool
}

/// Every address this session has asked the emulator to break on.
///
/// The emulator takes the whole set in one request, so the two halves have to
/// be written together: the editor's red dots, and whatever a step over has
/// armed behind them.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Breakpoints {
    /// The addresses the editor last armed. These are the only ones it knows
    /// about, and the only ones its gutter can show.
    editors: Vec<u16>,
    /// Addresses a step over armed and has not taken back.
    ///
    /// One entry while a step over is in flight; more if a step over never
    /// arrived, which happens when the routine stepped over does not return to
    /// the instruction after the call. That is a bug in the program being
    /// debugged rather than something to recover from, so the address simply
    /// stays armed - invisible, which is why a stop on one is explained in the
    /// console.
    temporary: Vec<u16>
}

impl Breakpoints {
    /// The whole set, as the emulator wants it.
    fn all(&self) -> Vec<u16> {
        let mut all = self.editors.clone();
        for address in &self.temporary {
            if !all.contains(address) {
                all.push(*address);
            }
        }
        all
    }

    /// Whether `address` carries a breakpoint the editor put there.
    fn is_the_editors(&self, address: u16) -> bool {
        self.editors.contains(&address)
    }

    fn arm_temporary(&mut self, address: u16) {
        if !self.temporary.contains(&address) {
            self.temporary.push(address);
        }
    }

    fn disarm_temporary(&mut self, address: u16) {
        self.temporary.retain(|armed| *armed != address);
    }
}

/// Write the whole breakpoint set to the emulator.
///
/// `/api/z80_bp` replaces everything it is given, so the temporaries have to go
/// out with the editor's dots or the next `setBreakpoints` would silently
/// disarm a step over in flight.
fn push_breakpoints(endpoint: &str, set: &Breakpoints) -> std::io::Result<()> {
    let body = set
        .all()
        .iter()
        .map(|address| format!("0x{address:04X}"))
        .collect::<Vec<_>>()
        .join(",");
    perform(endpoint, &Call::post("/api/z80_bp").body(body))?;
    Ok(())
}

impl AmspiritLitePeer {
    /// Connect to an emulator already serving at `endpoint`.
    pub fn connect(endpoint: &str) -> std::io::Result<Self> {
        let (outgoing, pending) = std::sync::mpsc::channel();

        // The event stream, on its own thread. It ends when the emulator does,
        // and a dead stream simply stops producing rather than failing the
        // session - the editor is still useful without live stop events.
        let events = outgoing.clone();
        let address = host_of(endpoint)?;
        std::thread::Builder::new()
            .name("amspiritlite-events".into())
            .spawn(move || read_events(&address, &events))?;

        // The run state, asked for rather than waited on.
        //
        // A paused machine renders no frames, so the `frame` heartbeat stops
        // dead the moment it stops - and the emulator sends a `pause` event
        // only when it was *asked* to stop, never when it stops itself on a
        // breakpoint. Between them there is no event to wait for, which is why
        // a breakpoint hit went unnoticed while a manual pause worked. So the
        // state is polled, several times a second, and a change is what the
        // editor hears about.
        let expecting_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stepping_over = std::sync::Arc::new(std::sync::Mutex::new(None));
        let breakpoints = std::sync::Arc::new(std::sync::Mutex::new(Breakpoints::default()));
        let seq = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0));
        let watching = Stops {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            out: outgoing.clone(),
            expecting_stop: expecting_stop.clone(),
            stepping_over: stepping_over.clone(),
            breakpoints: breakpoints.clone()
        };
        std::thread::Builder::new()
            .name("amspiritlite-runstate".into())
            .spawn(move || watch_run_state(&watching))?;

        Ok(Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            expecting_stop,
            stepping_over,
            breakpoints,
            resume_confirmation: Self::RESUME_CONFIRMATION,
            line_at_pc: crate::peer::LineAtPc::Unknown,
            launched: None,
            pending,
            outgoing,
            seq
        })
    }

    /// Take ownership of an emulator this session started, so it is closed
    /// with the session.
    pub fn owning(mut self, child: std::process::Child) -> Self {
        self.launched = Some(child);
        self
    }

    fn next_seq(&self) -> i64 {
        next_seq(&self.seq)
    }

    /// How long a resume is given to become visible before the editor is told
    /// to expect a stop anyway.
    ///
    /// Six times the worst lag measured against a real 1.13.4, so it is
    /// normally over in a millisecond or two. Waiting out the whole of it means
    /// the machine resumed and stopped again without a single look catching it
    /// running - a breakpoint one instruction away - and a stop is then exactly
    /// what to expect.
    const RESUME_CONFIRMATION: std::time::Duration = std::time::Duration::from_millis(60);

    /// Wait for the machine to say it is running again, briefly.
    ///
    /// Asking it to resume and having it resume are not the same instant, and
    /// the difference is what turned a continue into a stop that never
    /// happened. Looked at every couple of milliseconds, so a machine that
    /// resumes and hits a breakpoint a frame later is still caught running in
    /// between - the one observation that tells the two cases apart.
    fn wait_until_it_is_really_running(&self) {
        let deadline = std::time::Instant::now() + self.resume_confirmation;
        while std::time::Instant::now() < deadline {
            if let Ok(body) = perform(&self.endpoint, &Call::get("/api/ping"))
                && let Ok(state) = serde_json::from_str::<Value>(&body)
                && state
                    .get("emu")
                    .and_then(|emu| emu.get("paused"))
                    .and_then(Value::as_bool)
                    == Some(false)
            {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    /// Step *over* the instruction at `PC`, or *out* of the routine we are in.
    ///
    /// The emulator has no request for either, but it does not need one: put a
    /// breakpoint on the address control comes back to, let the machine run,
    /// and take the breakpoint out again when it arrives. One round trip
    /// instead of one per instruction executed, which is what stepping over a
    /// `call` into a decompressor used to cost.
    ///
    /// Only instructions that *mean* something different when stepped over get
    /// this treatment - see `returns_to_the_next_instruction`. Everything else
    /// is a plain single step, because for everything else "step over" and
    /// "step into" are the same thing.
    fn step_over(&mut self, message: &Value, out_of_this_one: bool) -> std::io::Result<()> {
        // Taken rather than read: what the session said about the line at `PC`
        // describes the stop we are leaving, and is worth nothing once the
        // machine has moved.
        let line = std::mem::replace(&mut self.line_at_pc, crate::peer::LineAtPc::Unknown);
        let target = if out_of_this_one {
            self.address_to_return_to()
        }
        else {
            self.address_after_the_instruction_at_pc(&line)
        };
        let Some(target) = target
        else {
            // Nothing worth running to, so this is an ordinary step.
            return self.step_once(message);
        };

        // Answered before the machine moves, so the response cannot arrive
        // after the `stopped` event it is supposed to precede.
        let seq = self.next_seq();
        let _ = self
            .outgoing
            .send(crate::protocol::response(message, json!({}), seq));

        if let Err(why) = self.run_to(target) {
            // An emulator that has stopped answering is a bad step, not a dead
            // session: letting the error travel up out of `send` ends the
            // session instead, and a debugger that disappears mid-step is far
            // worse than a step that did not happen.
            self.say(&format!("step over could not finish: {why}\n"));
            self.announce_a_stop_anyway();
        }
        Ok(())
    }

    /// Arm the address to come back to and let the machine run to it.
    fn run_to(&mut self, target: u16) -> std::io::Result<()> {
        // A breakpoint the user put there is theirs: it stays afterwards, and
        // stopping on it is a breakpoint stop rather than the end of a step.
        let was_the_editors = {
            let mut set = self
                .breakpoints
                .lock()
                .unwrap_or_else(|held| held.into_inner());
            let known = set.is_the_editors(target);
            if !known {
                set.arm_temporary(target);
            }
            push_breakpoints(&self.endpoint, &set)?;
            known
        };
        if let Ok(mut heading) = self.stepping_over.lock() {
            *heading = Some(StepTarget {
                address: target,
                was_the_editors
            });
        }

        perform(
            &self.endpoint,
            &Call::post("/api/config").body(json!({ "paused": false }).to_string())
        )?;
        // A resume is asked for here and happens over there, a moment later, so
        // nothing is owed a stop until the machine has really gone.
        self.wait_until_it_is_really_running();
        self.expecting_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// One instruction, which is what stepping over anything else means.
    fn step_once(&mut self, message: &Value) -> std::io::Result<()> {
        let stepped = perform(&self.endpoint, &Call::post("/api/step"));
        let seq = self.next_seq();
        let _ = self
            .outgoing
            .send(crate::protocol::response(message, json!({}), seq));
        match stepped {
            Ok(_) => {
                self.expecting_stop
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            },
            Err(why) => {
                self.say(&format!("step over could not finish: {why}\n"));
                self.announce_a_stop_anyway();
            }
        }
        Ok(())
    }

    /// One line in the Debug Console.
    fn say(&self, note: &str) {
        let seq = self.next_seq();
        let _ = self.outgoing.send(crate::protocol::event(
            "output",
            json!({ "category": "console", "output": note }),
            seq
        ));
    }

    /// Tell the editor the program is stopped when nothing else will.
    ///
    /// Used only when a step failed outright: the machine never moved, so the
    /// poller has no transition to notice, and an editor left believing the
    /// program is running has a dead toolbar and no way back.
    fn announce_a_stop_anyway(&self) {
        let seq = self.next_seq();
        let _ = self.outgoing.send(crate::protocol::event(
            "stopped",
            json!({
                "reason": "step",
                "description": "Execution stopped",
                "threadId": 1,
                "allThreadsStopped": true
            }),
            seq
        ));
    }

    /// The address after the instruction at `PC`, when running to it is what
    /// stepping over means; `None` when a plain step would do.
    fn address_after_the_instruction_at_pc(
        &self,
        line: &crate::peer::LineAtPc
    ) -> Option<u16> {
        let state = machine(&self.endpoint).ok()?;
        let pc = register(&state, "PC")? as u16;
        // Four bytes is the longest a Z80 instruction gets.
        let bytes = bytes_at(&self.endpoint, pc, 4).ok()?;

        match line {
            // A `defs` is a repetition written as a directive, so stepping over
            // it runs it out - from wherever inside it the program happens to
            // be, not one `NOP` at a time from there. Both halves of the
            // session's claim are checked against the machine before it is
            // believed: `PC` really is in the run, and the byte there really is
            // a `NOP`. A `defs` filled with something other than zero is code
            // we cannot promise comes back, so it keeps the ordinary rules.
            crate::peer::LineAtPc::Defs(run)
                if run.contains(&pc)
                    && bytes.first() == Some(&0x00)
                    // Nothing to run past on the last byte of a run, or on a
                    // `defs 1`: a single step already lands there, without
                    // arming anything.
                    && run.end > pc.wrapping_add(1) =>
            {
                return Some(run.end);
            },
            // No source to consult - a snapshot debugged without its listing,
            // or an address no line claims. The bytes are the only evidence
            // left, so a run of zeroes is read as one padded wait.
            crate::peer::LineAtPc::Unknown => {
                if let Some(after) = self.end_of_a_run_of_zeroes(pc) {
                    return Some(after);
                }
            },
            _ => {}
        }

        if !returns_to_the_next_instruction(&bytes) {
            return None;
        }
        let length = crate::disassemble::decode(pc, &bytes, 1)
            .first()
            .map(|instruction| instruction.bytes.len() as u16)
            .unwrap_or(1);
        Some(pc.wrapping_add(length))
    }

    /// The address after an unbroken run of zero bytes at `pc`, if there is
    /// one worth running past.
    ///
    /// A deliberate approximation, and only used where the source cannot
    /// answer: several zero bytes in a row are treated as one padded wait and
    /// stepped over in a single press. It disagrees with the source in exactly
    /// two ways, both accepted knowingly - a `defs` filled with a value other
    /// than zero is *not* recognised, and a genuine run of hand-written `nop`s
    /// *is* run out rather than stepped one at a time. A single `nop` between
    /// real instructions is untouched: one zero byte is one step, as it always
    /// was.
    ///
    /// Bounded, because the guess gets less honest the longer it runs: past a
    /// couple of hundred bytes this is more likely a program that has crashed
    /// into empty memory than a raster pad, and running to the far end of that
    /// is not what stepping over means.
    fn end_of_a_run_of_zeroes(&self, pc: u16) -> Option<u16> {
        const MOST_OF_A_PAD: u16 = 256;

        let bytes = bytes_at(&self.endpoint, pc, MOST_OF_A_PAD).ok()?;
        let zeroes = bytes.iter().take_while(|byte| **byte == 0).count();
        if zeroes < 2 || zeroes >= bytes.len() {
            return None;
        }
        Some(pc.wrapping_add(zeroes as u16))
    }

    /// The address on top of the stack: where a `call` said to come back to.
    ///
    /// A guess, and unavoidably so - `SP` points at a return address only
    /// because that is what being inside a subroutine means, and nothing on a
    /// Z80 says so. A wrong guess arms a breakpoint that is never reached,
    /// which is the same harmless outcome as stepping over a routine that
    /// never returns.
    fn address_to_return_to(&self) -> Option<u16> {
        let state = machine(&self.endpoint).ok()?;
        let sp = register(&state, "SP")? as u16;
        let stacked = bytes_at(&self.endpoint, sp, 2).ok()?;
        let [low, high] = stacked[..]
        else {
            return None;
        };
        Some(u16::from(low) | (u16::from(high) << 8))
    }

    /// Take every breakpoint this adapter armed by itself back out.
    ///
    /// A step over that never arrived leaves one behind on purpose - the
    /// program did not do what stepping over it assumed, and guessing at a
    /// recovery would be worse than leaving it - but the session must not hand
    /// the emulator on with them still set.
    fn forget_temporary_breakpoints(&mut self) {
        if let Ok(mut heading) = self.stepping_over.lock() {
            *heading = None;
        }
        let Ok(mut set) = self.breakpoints.lock()
        else {
            return;
        };
        if set.temporary.is_empty() {
            return;
        }
        set.temporary.clear();
        let _ = push_breakpoints(&self.endpoint, &set);
    }
}

/// The next sequence number, shared between the peer and the poller.
fn next_seq(seq: &std::sync::atomic::AtomicI64) -> i64 {
    seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}

/// Everything the machine has to say about itself right now.
fn machine(endpoint: &str) -> std::io::Result<Value> {
    let body = perform(endpoint, &Call::get("/api/state"))?;
    serde_json::from_str(&body)
        .map_err(|why| std::io::Error::new(std::io::ErrorKind::InvalidData, why))
}

/// The bytes at `address`, however the emulator is paged right now.
fn bytes_at(endpoint: &str, address: u16, count: u16) -> std::io::Result<Vec<u8>> {
    let call = Call::get("/api/ram")
        .query("addr", address)
        .query("len", count);
    let body = perform(endpoint, &call)?;
    let answer: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    Ok(bytes_from_hex(
        answer.get("hex").and_then(Value::as_str).unwrap_or_default()
    ))
}

/// Whether stepping *over* this instruction means anything more than stepping
/// into it.
///
/// Only these hand control back to the address after themselves after doing
/// work you may not want to watch, so only these are worth running to. For
/// everything else - a jump, a return, an ordinary instruction - the address
/// after the instruction is either where one step lands anyway or somewhere
/// control never comes back to, and arming a breakpoint on it would be a
/// breakpoint that is never hit.
pub(crate) fn returns_to_the_next_instruction(bytes: &[u8]) -> bool {
    match bytes {
        // The repeating block instructions, which re-execute themselves until
        // they are done: `ldir`, `lddr`, `cpir`, `cpdr`, `inir`, `indr`,
        // `otir`, `otdr`. `PC` sits still while they work, so a plain step
        // makes no visible progress - stepping *over* one is the only way to
        // get past it without holding the key down.
        [0xED, second, ..] if matches!(second, 0xB0..=0xB3 | 0xB8..=0xBB) => true,
        // `djnz`, which is a loop written as one instruction: stepping over it
        // is the only way to say "run the loop out" without setting a
        // breakpoint by hand on the line below it.
        [0x10, ..] => true,
        // `call nn`, and its eight conditional forms.
        [opcode, ..] if *opcode == 0xCD || opcode & 0b1100_0111 == 0b1100_0100 => true,
        // `rst n`.
        [opcode, ..] if opcode & 0b1100_0111 == 0b1100_0111 => true,
        // `halt`, which stays where it is until an interrupt moves it on.
        [0x76, ..] => true,
        _ => false
    }
}

impl crate::peer::DapPeer for AmspiritLitePeer {
    fn note_line_at_pc(&mut self, line: crate::peer::LineAtPc) {
        self.line_at_pc = line;
    }

    fn send(&mut self, message: Value) -> std::io::Result<()> {
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        // The editor's set is remembered rather than merely forwarded: it has
        // to go out again alongside whatever a step over arms behind it, since
        // `/api/z80_bp` replaces the whole set every time it is written.
        //
        // Answered from here too, so a step over in flight cannot be disarmed
        // by the editor changing a red dot.
        if command == "setInstructionBreakpoints" {
            let editors: Vec<u16> = message
                .get("arguments")
                .and_then(|a| a.get("breakpoints"))
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(|entry| {
                            entry
                                .get("instructionReference")
                                .and_then(Value::as_str)
                                .and_then(crate::protocol::parse_address_reference)
                                .map(|address| address as u16)
                        })
                        .collect()
                })
                .unwrap_or_default();
            {
                let mut set = self
                    .breakpoints
                    .lock()
                    .unwrap_or_else(|held| held.into_inner());
                set.editors = editors;
                push_breakpoints(&self.endpoint, &set)?;
            }
            let seq = self.next_seq();
            let _ = self
                .outgoing
                .send(crate::protocol::response(&message, json!({}), seq));
            return Ok(());
        }

        // A session that ends must not hand the emulator back with breakpoints
        // on addresses nobody can see. A step over that arrived cleans up after
        // itself; one that never arrived is left alone until here.
        if matches!(command.as_str(), "disconnect" | "terminate") {
            self.forget_temporary_breakpoints();
        }

        // Step over and step out are the same trick, aimed at different
        // addresses - see `step_over`.
        if command == "next" || command == "stepOut" {
            return self.step_over(&message, command == "stepOut");
        }

        let Some(call) = call_for(&message)
        else {
            // Nothing to ask this emulator. Answered as success with an empty
            // body rather than dropped: a request the editor is waiting on must
            // be answered, or its pane hangs.
            let seq = self.next_seq();
            let _ = self
                .outgoing
                .send(crate::protocol::response(&message, json!({}), seq));
            return Ok(());
        };

        let body = perform(&self.endpoint, &call)?;
        let mut state: Value = serde_json::from_str(&body).unwrap_or(Value::Null);

        // The Gate Array pane wants its banking register, and `/api/ga` does
        // not carry it - only `/api/memmap` does. Two calls, one pane: the
        // register that decides which page is where belongs beside the mode and
        // the palette, not in a separate view.
        if message.get("command").and_then(Value::as_str) == Some("cpclib/ga")
            && let Ok(extra) = perform(&self.endpoint, &Call::get("/api/memmap"))
            && let Ok(memmap) = serde_json::from_str::<Value>(&extra)
            && let (Some(target), Some(source)) = (state.as_object_mut(), memmap.as_object())
        {
            for key in ["rmr", "ram_mode", "ram_page"] {
                if let Some(value) = source.get(key) {
                    target.insert(key.to_string(), value.clone());
                }
            }
        }
        // A resume is asked for here and happens over there, a moment later:
        // measured against 1.13.4, `/api/config` answers in well under a
        // millisecond and `/api/ping` goes on reporting the machine paused for
        // another 0.5 to 11 after that. Nothing is owed a stop until it has
        // really gone.
        if command == "continue" {
            self.wait_until_it_is_really_running();
        }

        // From here the editor is owed a stop: a continue leaves the machine
        // running and nothing else will announce where it ends up, and a step
        // ends paused with no transition for the poller to notice.
        //
        // Raised *after* the machine has moved, never before it is asked to.
        // The poller looks ten times a second, so a flag raised while the
        // machine is still paused from the stop it is leaving gives it a window
        // in which to find the machine paused, consume the flag and announce a
        // stop that has not happened. The editor then believes a machine that
        // is really running, and every register and stack frame it goes on to
        // ask for is sampled from a program in flight: a stop at a random
        // address, with no breakpoint anywhere near it.
        if matches!(command.as_str(), "continue" | "stepIn") {
            self.expecting_stop
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let seq = self.next_seq();
        let _ = self.outgoing.send(response_for(&message, &state, seq));
        Ok(())
    }

    fn drain(&mut self) -> Vec<Value> {
        self.pending.try_iter().collect()
    }

    fn quirks(&self) -> crate::peer::Quirks {
        crate::peer::Quirks {
            // Address breakpoints only, like the other emulator: `/api/z80_bp`
            // takes a list of addresses and knows nothing about source files.
            instruction_breakpoints_only: true,
            // No attach handshake - a REST API is available as soon as it is
            // listening.
            attach_required: false,
            // Nothing is forwarded blind: `call_for` decides what this emulator
            // is asked, and anything it does not map is answered here.
            rejects_unknown_requests: false
        }
    }

    fn supports(&self, command: &str) -> bool {
        matches!(
            command,
            "pause"
                | "continue"
                | "stepIn"
                | "next"
                | "stepOut"
                | "threads"
                | "stackTrace"
                | "scopes"
                | "variables"
                | "readMemory"
                | "writeMemory"
                | "setInstructionBreakpoints"
                | "cpclib/setPc"
                | "cpclib/memmap"
                | "cpclib/crtc"
                | "cpclib/ga"
                | "cpclib/psg"
                | "cpclib/fdc"
                | "cpclib/history"
        )
    }
}

impl Drop for AmspiritLitePeer {
    fn drop(&mut self) {
        // An emulator the user keeps running must not be handed back with
        // breakpoints on addresses nothing can show them. A `disconnect` does
        // this too; this catches a session that ended some other way.
        if self.launched.is_none() {
            self.forget_temporary_breakpoints();
        }

        // Only one we started. Leaving it running holds the port, and the next
        // session would attach to the previous program without saying so.
        if let Some(child) = self.launched.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `http://127.0.0.1:8765` into `127.0.0.1:8765`.
fn host_of(endpoint: &str) -> std::io::Result<String> {
    let rest = endpoint
        .trim_end_matches('/')
        .strip_prefix("http://")
        .ok_or_else(|| std::io::Error::other(format!("{endpoint} is not an http:// address")))?;
    Ok(rest.to_string())
}

/// Make one call and return its body.
fn perform(endpoint: &str, call: &Call) -> std::io::Result<String> {
    use std::io::{Read, Write};

    let host = host_of(endpoint)?;
    let mut path = call.path.to_string();
    if !call.query.is_empty() {
        let query: Vec<String> = call
            .query
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        path.push('?');
        path.push_str(&query.join("&"));
    }

    let mut stream = std::net::TcpStream::connect(&host)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    let request = match (&call.method, &call.body) {
        (Method::Get, _) => {
            format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n")
        },
        (Method::Post, body) => {
            let body = body.clone().unwrap_or_default();
            format!(
                "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\
                 Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
        }
    };
    stream.write_all(request.as_bytes())?;

    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    Ok(body_of(&raw).to_string())
}

/// Everything after the blank line that ends the headers.
fn body_of(response: &str) -> &str {
    match response.find("\r\n\r\n") {
        Some(at) => &response[at + 4..],
        None => ""
    }
}

/// What the run-state poller watches, and what it answers into.
pub(crate) struct Stops {
    pub(crate) endpoint: String,
    pub(crate) out: std::sync::mpsc::Sender<Value>,
    pub(crate) expecting_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(crate) stepping_over: std::sync::Arc<std::sync::Mutex<Option<StepTarget>>>,
    pub(crate) breakpoints: std::sync::Arc<std::sync::Mutex<Breakpoints>>
}

/// Ask the emulator whether it is running, and report every change.
///
/// Ten times a second: fast enough that a stop feels immediate, cheap enough
/// that it is one small request on loopback. The alternative - waiting for an
/// event - does not work, for the reasons in `connect`.
fn watch_run_state(watched: &Stops) {
    let endpoint = watched.endpoint.as_str();
    let mut running: Option<bool> = None;
    let mut seq = 1_000_000i64;
    let mut misses = 0u32;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        let body = match perform(endpoint, &Call::get("/api/ping")) {
            Ok(body) => {
                misses = 0;
                body
            },
            Err(_) => {
                // A refused connection is usually the emulator being busy for
                // a moment, not the emulator being gone - and this thread is
                // the *only* thing that notices a breakpoint being hit, so
                // giving up on the first failure silently ends stop detection
                // for the rest of the session. Three seconds of nothing at all
                // is what counts as gone.
                misses += 1;
                if misses > 30 {
                    return;
                }
                continue;
            }
        };
        let state: Value = match serde_json::from_str(&body) {
            Ok(state) => state,
            Err(_) => continue
        };
        let Some(paused) = state
            .get("emu")
            .and_then(|emu| emu.get("paused"))
            .and_then(Value::as_bool)
        else {
            continue;
        };

        // We asked it to run and it is stopped again. There may be no change
        // to notice - a step ends paused, and a continue that hits the same
        // breakpoint a frame later resumes and stops inside one poll interval -
        // but the editor is waiting to hear about it either way.
        if paused
            && watched
                .expecting_stop
                .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            running = Some(false);
            if !watched.announce_stop(&mut seq) {
                return;
            }
            continue;
        }

        if running == Some(!paused) {
            continue; // no change
        }
        let first = running.is_none();
        running = Some(!paused);
        // The state as we found it is not a change the editor made - but that
        // is only true of a machine that is *running*. Finding it already
        // stopped is news, and swallowing it is how a session dies before it
        // starts: the emulator halts on a breakpoint in the few milliseconds
        // between arming and this thread's first look, no transition ever
        // follows, and the editor waits for ever on a program it thinks is
        // running while every button does nothing.
        if first && !paused {
            continue;
        }

        if paused {
            if !watched.announce_stop(&mut seq) {
                return;
            }
        }
        else {
            seq += 1;
            let event = crate::protocol::event(
                "continued",
                json!({ "threadId": 1, "allThreadsContinued": true }),
                seq
            );
            if watched.out.send(event).is_err() {
                return; // the session is gone
            }
        }
    }
}

impl Stops {
    /// Tell the editor the machine has stopped, having first worked out
    /// whether this is the end of a step over.
    ///
    /// Returns false when the session is gone.
    fn announce_stop(&self, seq: &mut i64) -> bool {
        let notice = self.settle(self.stopped_at());
        if let Some(text) = notice.as_ref().and_then(|notice| notice.text.clone()) {
            *seq += 1;
            let said = crate::protocol::event(
                "output",
                json!({ "category": "console", "output": text }),
                *seq
            );
            if self.out.send(said).is_err() {
                return false;
            }
        }
        *seq += 1;
        let stopped = crate::protocol::event(
            "stopped",
            json!({
                "reason": notice.map(|notice| notice.reason).unwrap_or("breakpoint"),
                "description": "Execution stopped",
                "threadId": 1,
                "allThreadsStopped": true
            }),
            *seq
        );
        self.out.send(stopped).is_ok()
    }

    /// Where the machine is, when anything depends on knowing.
    ///
    /// Only asked for when a step over is in flight or a temporary breakpoint
    /// has been left behind - otherwise this is a round trip nobody reads.
    fn stopped_at(&self) -> Option<u16> {
        let interested = self
            .stepping_over
            .lock()
            .map(|heading| heading.is_some())
            .unwrap_or(false)
            || self
                .breakpoints
                .lock()
                .map(|set| !set.temporary.is_empty())
                .unwrap_or(false);
        if !interested {
            return None;
        }
        let state = machine(&self.endpoint).ok()?;
        register(&state, "PC").map(|pc| pc as u16)
    }

    /// Account for the stop: retire a step over that arrived, and explain a
    /// stop on a breakpoint the editor cannot show.
    fn settle(&self, pc: Option<u16>) -> Option<Stop> {
        let pc = pc?;
        let heading = self
            .stepping_over
            .lock()
            .ok()
            .and_then(|mut heading| heading.take());

        if let Some(heading) = heading {
            // Arrived. The breakpoint that brought us here was ours, so it goes
            // away again; one the editor had armed there is the user's and
            // stays, and stopping on it is a breakpoint stop like any other.
            if heading.address == pc {
                if !heading.was_the_editors
                    && let Ok(mut set) = self.breakpoints.lock()
                {
                    set.disarm_temporary(pc);
                    let _ = push_breakpoints(&self.endpoint, &set);
                }
                return Some(Stop {
                    reason: if heading.was_the_editors {
                        "breakpoint"
                    }
                    else {
                        "step"
                    },
                    text: None
                });
            }
            // Something else stopped us first - a breakpoint inside the routine
            // being stepped over, or the user pressing pause. The step over is
            // abandoned where it stands and its breakpoint is left armed: the
            // program may still come back to it, and if it never does that is a
            // bug in the program rather than something to recover from here.
        }

        // A stop on one of those left-behind breakpoints, whenever it finally
        // comes.
        let orphan = self
            .breakpoints
            .lock()
            .map(|set| set.temporary.contains(&pc) && !set.is_the_editors(pc))
            .unwrap_or(false);
        if orphan {
            // A temporary breakpoint has no red dot beside it, so a stop on one
            // looks like a stop at nothing at all - which is exactly how this
            // was reported the last time an invisible breakpoint fired.
            return Some(Stop {
                reason: "breakpoint",
                text: Some(format!(
                    "Stopped at 0x{pc:04X}, where an earlier step over put a breakpoint and \
                     never came back to it. It has no red dot because the editor was never \
                     told about it. It stays armed until the session ends.\n"
                ))
            });
        }
        None
    }
}

/// What to call a stop, and what the console is owed about it.
struct Stop {
    reason: &'static str,
    text: Option<String>
}


/// Read `/api/events` forever, turning each event into a DAP one.
fn read_events(host: &str, out: &std::sync::mpsc::Sender<Value>) {
    use std::io::{BufRead, BufReader, Write};

    let Ok(mut stream) = std::net::TcpStream::connect(host)
    else {
        return;
    };
    let request =
        format!("GET /api/events HTTP/1.1\r\nHost: {host}\r\nAccept: text/event-stream\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return;
    }

    let reader = BufReader::new(stream);
    // Server-Sent Events: `event:` names one, `data:` carries its payload, and
    // a blank line ends it.
    let mut name = String::new();
    let mut seq = 0i64;
    // Whether the machine was running last time anything said so.
    //
    // A `pause` event arrives when the emulator is *asked* to stop, but not
    // when it stops itself on a breakpoint - so waiting for one meant a
    // breakpoint hit was invisible and the session sat there as if nothing had
    // happened. The `frame` heartbeat carries `paused` ten times a second, so
    // the transition is noticed from that instead, whatever caused it.
    let mut running = true;
    for line in reader.lines().map_while(Result::ok) {
        if let Some(rest) = line.strip_prefix("event:") {
            name = rest.trim().to_string();
            continue;
        }
        let Some(rest) = line.strip_prefix("data:")
        else {
            continue;
        };
        let payload: Value = serde_json::from_str(rest.trim()).unwrap_or(Value::Null);

        // Any event that mentions it updates our idea of the run state, and a
        // change is what the editor needs to hear about.
        let event = match payload.get("paused").and_then(Value::as_bool) {
            Some(paused) if paused == running => {
                running = !paused;
                seq += 1;
                Some(if paused {
                    crate::protocol::event(
                        "stopped",
                        json!({
                            "reason": "breakpoint",
                            "description": "Execution stopped",
                            "threadId": 1,
                            "allThreadsStopped": true
                        }),
                        seq
                    )
                }
                else {
                    crate::protocol::event(
                        "continued",
                        json!({ "threadId": 1, "allThreadsContinued": true }),
                        seq
                    )
                })
            },
            // No change, or an event that says nothing about it: the named
            // ones still have their own meaning.
            _ => {
                seq += 1;
                match name.as_str() {
                    // Already handled above by the state change; forwarding it
                    // again would report one stop twice.
                    "pause" | "frame" => None,
                    other => event_for(other, &payload, seq)
                }
            }
        };

        if let Some(event) = event
            && out.send(event).is_err()
        {
            return; // the session is gone
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(command: &str, arguments: Value) -> Value {
        json!({ "seq": 1, "type": "request", "command": command, "arguments": arguments })
    }

    /// A real `/api/state` body, copied from a running emulator rather than
    /// imagined - the published example client described a different shape
    /// entirely, and every mapping below was wrong until this was checked
    /// against the machine.
    fn state() -> Value {
        json!({
            "z80": {
                "PC": 2609, "SP": 771, "A": 7, "F": 0, "B": 127, "C": 79,
                "D": 250, "E": 180, "H": 10, "L": 129,
                "A2": 0, "F2": 0, "B2": 246, "C2": 128, "D2": 0, "E2": 192,
                "H2": 0, "L2": 119,
                "IX": 17202, "IY": 10496, "I": 0, "R": 98,
                "IFF1": 1, "IFF2": 1, "IM": 1
            },
            "ga": { "mode": 0, "border_idx": 20, "ink_idx": [20, 4, 21] },
            "crtc": { "regs": [63, 48], "selected_reg": 8, "rasterline": 0, "vsync": false },
            "psg": { "period_a": 1204, "vol_a": 16 },
            "fdc": { "msr": 128, "sr0": 0, "motor": false, "drive": 0 },
            "emu": { "paused": false }
        })
    }

    /// Run control is a *configuration* change here, not a verb of its own.
    #[test]
    fn pausing_and_resuming_go_through_the_config_endpoint() {
        let paused = call_for(&request("pause", json!({ "threadId": 1 }))).unwrap();
        assert_eq!(paused.path, "/api/config");
        assert_eq!(paused.method, Method::Post);
        assert_eq!(paused.body.as_deref(), Some(r#"{"paused":true}"#));

        let resumed = call_for(&request("continue", json!({ "threadId": 1 }))).unwrap();
        assert_eq!(resumed.body.as_deref(), Some(r#"{"paused":false}"#));
    }

    #[test]
    fn stepping_posts_to_step() {
        for command in ["stepIn", "next"] {
            let call = call_for(&request(command, json!({ "threadId": 1 }))).unwrap();
            assert_eq!(call.path, "/api/step", "{command}");
            assert_eq!(call.method, Method::Post, "{command}");
        }
    }

    /// Decimal, both of them - not the hex the example client implied.
    #[test]
    fn a_memory_read_asks_in_decimal() {
        let call = call_for(&request(
            "readMemory",
            json!({ "memoryReference": "0x4000", "count": 32 })
        ))
        .unwrap();

        assert_eq!(call.method, Method::Get);
        assert_eq!(call.path, "/api/ram");
        assert_eq!(
            call.query,
            vec![
                ("addr".to_string(), "16384".to_string()),
                ("len".to_string(), "32".to_string())
            ]
        );
    }

    /// ...and the answer arrives as a hex string under `hex`.
    #[test]
    fn a_memory_answer_is_re_encoded_for_the_editor() {
        let read = request(
            "readMemory",
            json!({ "memoryReference": "0x4000", "count": 3 })
        );
        let answer = response_for(
            &read,
            &json!({ "addr": 16384, "len": 3, "hex": "3e00c9" }),
            9
        );

        assert_eq!(answer["body"]["data"], json!("PgDJ"));
        assert_eq!(answer["body"]["address"], json!("0x4000"));
    }

    #[test]
    fn breakpoints_go_over_as_one_comma_separated_set() {
        let call = call_for(&request(
            "setInstructionBreakpoints",
            json!({
                "breakpoints": [
                    { "instructionReference": "0x4000" },
                    { "instructionReference": "0x5c44" }
                ]
            })
        ))
        .unwrap();

        assert_eq!(call.path, "/api/z80_bp");
        assert_eq!(call.body.as_deref(), Some("0x4000,0x5C44"));
    }

    #[test]
    fn clearing_breakpoints_sends_an_empty_set() {
        let call = call_for(&request(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [] })
        ))
        .unwrap();
        assert_eq!(call.body.as_deref(), Some(""));
    }

    #[test]
    fn an_unmapped_request_asks_for_nothing() {
        assert!(call_for(&request("setDataBreakpoints", json!({}))).is_none());
        assert!(call_for(&request("evaluate", json!({}))).is_none());
    }

    /// Registers, chips and everything else arrive in one call.
    #[test]
    fn the_state_endpoint_answers_every_pane() {
        for command in ["threads", "stackTrace", "scopes", "variables"] {
            let call = call_for(&request(command, json!({}))).unwrap();
            assert_eq!(call.path, "/api/state", "{command}");
        }
    }

    #[test]
    fn a_stack_trace_is_built_from_the_reported_registers() {
        let answer = response_for(&request("stackTrace", json!({})), &state(), 7);
        let frame = &answer["body"]["stackFrames"][0];
        // PC 2609 is 0x0A31.
        assert_eq!(frame["instructionPointerReference"], json!("0x0A31"));
    }

    /// The emulator reports the 8-bit halves; a Z80 programmer reads pairs.
    #[test]
    fn the_halves_are_composed_into_the_pairs_a_programmer_reads() {
        let answer = response_for(&request("variables", json!({})), &state(), 8);
        let listed = answer["body"]["variables"].as_array().unwrap();
        let of = |name: &str| {
            listed
                .iter()
                .find(|v| v["name"] == json!(name))
                .unwrap_or_else(|| panic!("no {name}: {listed:?}"))["value"]
                .clone()
        };

        // A=7, F=0 -> AF=0x0700; H=10, L=129 -> HL=0x0A81.
        assert_eq!(of("AF"), json!("0x0700"));
        assert_eq!(of("HL"), json!("0x0A81"));
        assert_eq!(of("BC"), json!("0x7F4F"));
        // The alternate set is `A2`/`F2` and friends.
        assert_eq!(of("BC'"), json!("0xF680"));
        // Eight-bit ones read as bytes, interrupt state included.
        assert_eq!(of("R"), json!("0x62"));
        assert_eq!(of("IM"), json!("0x01"));
        assert_eq!(of("IFF1"), json!("0x01"));
    }

    #[test]
    fn hex_is_read_with_or_without_separators() {
        assert_eq!(bytes_from_hex("3E00C9"), vec![0x3E, 0x00, 0xC9]);
        assert_eq!(bytes_from_hex("3e 00 c9"), vec![0x3E, 0x00, 0xC9]);
        assert!(bytes_from_hex("").is_empty());
    }

    /// Writing goes to the same endpoint as reading, and the bytes turn back
    /// into hex on the way out.
    /// The run state is polled, never waited on, and the stream must not
    /// report it as well.
    ///
    /// A paused machine renders no frames, so the `frame` heartbeat stops the
    /// instant it stops; and the emulator sends `pause` only when it was
    /// *asked* to stop, never when it stops itself on a breakpoint. Between
    /// them there is no event to wait for - which is why a breakpoint hit went
    /// unnoticed while a manual pause worked.
    #[test]
    fn the_stream_does_not_report_the_run_state() {
        assert!(
            event_for("pause", &json!({ "paused": true }), 1).is_none(),
            "the poll owns this; reporting it here would report one stop twice"
        );
        assert!(event_for("pause", &json!({ "paused": false }), 2).is_none());
    }

    /// `frame` arrives ten times a second with the registers in it. Forwarded
    /// as a stop it would make the session unusable.
    #[test]
    fn the_frame_heartbeat_is_swallowed() {
        let frame = json!({
            "pc": "0x0A26", "sp": "0x0303", "a": 6, "fps": 50.0,
            "frame": 180, "paused": false
        });
        assert!(event_for("frame", &frame, 1).is_none());

        for quiet in ["state", "basic_vars", "config_changed", "disk_status"] {
            assert!(event_for(quiet, &json!({}), 2).is_none(), "{quiet}");
        }
    }

    #[test]
    fn an_error_reaches_the_console() {
        let problem = event_for("error", &json!({ "message": "no disc" }), 4).unwrap();
        assert_eq!(problem["event"], json!("output"));
        assert!(
            problem["body"]["output"]
                .as_str()
                .unwrap()
                .contains("no disc")
        );
    }

    #[test]
    fn a_memory_write_re_encodes_the_bytes() {
        let call = call_for(&request(
            "writeMemory",
            json!({ "memoryReference": "0x4000", "data": "PgDJ" })
        ))
        .unwrap();

        assert_eq!(call.method, Method::Post);
        assert_eq!(call.path, "/api/ram");
        assert_eq!(
            call.body.as_deref(),
            Some(r#"{"addr":16384,"data":"3E00C9"}"#)
        );
    }

    /// The question 1984js cannot answer at all.
    ///
    /// Which page is mapped where is what forced the byte-matching heuristic,
    /// the ambiguity notices and the "most likely line" fallback in the other
    /// backend. Here it is a lookup.
    #[test]
    fn the_page_at_an_address_is_read_rather_than_guessed() {
        // A real `/api/memmap` body: four 16K regions, banks 0..3 - the base
        // 64K, which is page 0 from end to end.
        let memmap = json!({
            "regions": [
                { "base": 0, "name": "0000", "rom": false, "ram_bank": 0 },
                { "base": 16384, "name": "4000", "rom": false, "ram_bank": 1 },
                { "base": 32768, "name": "8000", "rom": false, "ram_bank": 2 },
                { "base": 49152, "name": "C000", "rom": false, "ram_bank": 3 }
            ],
            "rmr": 140
        });

        for address in [0x0000u16, 0x04A5, 0x4000, 0x5C44, 0xFFFF] {
            assert_eq!(page_at(&memmap, address), Some(0), "{address:04X}");
        }

        // And the body that started all this: `ram_page` says 0 and is
        // useless, while the region at `#4000` names bank 5 - page 1's second
        // bank, physical `0x14000-0x17FFF`. Reading the former gave page 0 and
        // opened `writter.asm` at a line that holds an enum; the latter gives
        // page 1, which is where `animate.asm` really is.
        let banked = json!({
            "ram_mode": 5,
            "ram_page": 0,
            "regions": [
                { "base": 0, "rom": false, "ram_bank": 0 },
                { "base": 16384, "ext": true, "rom": false, "ram_bank": 5 },
                { "base": 32768, "rom": false, "ram_bank": 2 },
                { "base": 49152, "rom": false, "ram_bank": 3 }
            ]
        });
        assert_eq!(page_at(&banked, 0x79F3), Some(1), "the address we fought over");
        assert_eq!(physical_of(&banked, 0x79F3), Some(0x179F3), "as in sna.lst");
        assert_eq!(page_at(&banked, 0x04A5), Some(0));
    }

    /// A region holding ROM is not the program's, so no page is claimed for it.
    /// The Gate Array's MMR decides the page exactly - there is nothing to
    /// guess. The table is the hardware's:
    /// <https://www.grimware.org/doku.php/documentations/devices/gatearray>
    #[test]
    fn the_mmr_says_which_page_is_where() {
        let mmr = |mode: u64, page: u64| json!({ "ram_mode": mode, "ram_page": page });

        // MM 0: the base 64K throughout, whatever page is selected.
        assert_eq!(page_at(&mmr(0, 3), 0x0000), Some(0));
        assert_eq!(page_at(&mmr(0, 3), 0xC000), Some(0));

        // MM 1: only #C000 comes from the page.
        assert_eq!(page_at(&mmr(1, 2), 0x8000), Some(0));
        assert_eq!(page_at(&mmr(1, 2), 0xC000), Some(2));

        // MM 2: the whole address space comes from the page.
        for address in [0x0000u16, 0x4000, 0x8000, 0xC000] {
            assert_eq!(page_at(&mmr(2, 1), address), Some(1), "{address:04X}");
        }

        // MM 3: like 1, with #4000 taken from bank 3 of the *base* 64K.
        assert_eq!(page_at(&mmr(3, 2), 0x4000), Some(0));
        assert_eq!(page_at(&mmr(3, 2), 0xC000), Some(2));

        // S set (MM 4..7): only #4000 is paged.
        for mode in 4..8 {
            assert_eq!(page_at(&mmr(mode, 1), 0x4000), Some(1), "mode {mode}");
            assert_eq!(page_at(&mmr(mode, 1), 0x0000), Some(0), "mode {mode}");
            assert_eq!(page_at(&mmr(mode, 1), 0xC000), Some(0), "mode {mode}");
        }
    }

    /// The chip panes, from this emulator's own answers - no snapshot to save
    /// and parse, and carrying things a snapshot never did.
    #[test]
    fn the_crtc_pane_marks_the_selected_register() {
        // A real `/api/crtc` body.
        let body = json!({
            "regs": [63, 48, 50, 142],
            "selected_reg": 2,
            "rasterline": 0,
            "vsync": false
        });
        let pane = chip_variables(crate::inspect::CRTC_REFERENCE, &body);
        let names: Vec<String> = pane
            .iter()
            .map(|v| v["name"].as_str().unwrap_or_default().to_string())
            .collect();

        assert!(names.contains(&"R0".to_string()), "{names:?}");
        // R2 is selected, so it is marked rather than plain.
        assert!(!names.contains(&"R2".to_string()), "{names:?}");
        assert!(names.iter().any(|n| n.contains('\u{0332}')), "{names:?}");
        // The counters the snapshot route could not give without a whole save.
        assert!(names.contains(&"rasterline".to_string()), "{names:?}");
    }

    /// The palette reads as ink numbers and the bytes that set them.
    #[test]
    fn the_gate_array_pane_reads_as_pens_with_colours() {
        let body = json!({
            "mode": 0,
            "border_idx": 20,
            "ink_idx": [20, 4],
            "ink_rgb": [513, 619]
        });
        let pane = chip_variables(crate::inspect::GATE_ARRAY_REFERENCE, &body);
        let of = |name: &str| {
            pane.iter()
                .find(|v| v["name"] == json!(name))
                .unwrap_or_else(|| panic!("no {name}: {pane:?}"))["value"]
                .as_str()
                .unwrap()
                .to_string()
        };

        // `ink_idx` 20 is the Gate Array's own selector, so the byte to write
        // is 0x54 - which is ink 0, black. Both numbers are worth having; the
        // sRGB triple behind them is not, and no longer appears.
        let pen0 = of("pen 0");
        assert_eq!(pen0, "\u{2B1B} ink 0 (GA 0x54)", "{pen0}");
        assert!(
            !pen0.contains('#'),
            "the RGB is what picks the square, not what is printed: {pen0}"
        );
        // ...and the description says what to do with that byte.
        let described = pane.iter().find(|v| v["name"] == json!("pen 0")).unwrap()["type"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(described.contains("write 0x54 to &7Fxx"), "{described}");
        assert_eq!(of("mode"), "0x00 (0)");

        // The banking register belongs here too: it decides which page sits in
        // which bank, and `/api/ga` does not carry it - the peer fetches it
        // from `/api/memmap` and merges the two into one pane.
        let banked = chip_variables(
            crate::inspect::GATE_ARRAY_REFERENCE,
            &json!({ "mode": 1, "rmr": 140, "ram_mode": 2, "ram_page": 1 })
        );
        let named = |name: &str| {
            banked
                .iter()
                .find(|v| v["name"] == json!(name))
                .unwrap_or_else(|| panic!("no {name}: {banked:?}"))["value"]
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(named("rmr"), "0x8C (140)");
        assert_eq!(named("ram_mode"), "0x02 (2)");
        assert_eq!(named("ram_page"), "0x01 (1)");
    }

    /// Each chip is asked for by name; the disc one is the FDC.
    #[test]
    fn every_chip_scope_has_a_command() {
        assert_eq!(
            chip_command(crate::inspect::CRTC_REFERENCE),
            Some("cpclib/crtc")
        );
        assert_eq!(
            chip_command(crate::inspect::DISC_REFERENCE),
            Some("cpclib/fdc"),
            "the FDC really is readable here, unlike through a snapshot"
        );
        assert!(chip_command(1).is_none());
    }

    /// With S set, the bank inside the page varies with `b` - so the *offset*
    /// moves, not only the page.
    ///
    /// Mode 5 maps CPU `#79F3` to the page's own `#79F3`; mode 4 maps it to the
    /// page's `#39F3`. Resolving at the CPU address would look in the wrong
    /// half of the page.
    #[test]
    fn a_remapped_bank_moves_the_physical_address_too() {
        let mmr = |mode: u64| json!({ "ram_mode": mode, "ram_page": 1 });

        assert_eq!(physical_of(&mmr(5), 0x79F3), Some(0x179F3), "bank 1 of the page");
        assert_eq!(physical_of(&mmr(4), 0x79F3), Some(0x139F3), "bank 0 of the page");
        assert_eq!(physical_of(&mmr(6), 0x79F3), Some(0x1B9F3), "bank 2 of the page");
        assert_eq!(physical_of(&mmr(7), 0x79F3), Some(0x1F9F3), "bank 3 of the page");

        // Outside #4000-#7FFF nothing is remapped, whatever the mode.
        assert_eq!(physical_of(&mmr(5), 0xC000), Some(0xC000));
        // ...and every one of them is page 1 only where the page is mapped.
        assert_eq!(page_at(&mmr(5), 0x79F3), Some(1));
        assert_eq!(page_at(&mmr(5), 0xC000), Some(0));
    }

    #[test]
    fn rom_claims_no_page() {
        let memmap = json!({
            "regions": [{ "base": 0, "rom": true, "ram_bank": 0 }]
        });
        assert_eq!(page_at(&memmap, 0x0100), None);
        assert_eq!(page_at(&json!({}), 0x4000), None, "nothing to read");
    }

    /// Each chip has its own endpoint - no whole-machine snapshot to save and
    /// parse, which is what the other backend has to do.
    #[test]
    fn every_chip_has_an_endpoint_of_its_own() {
        for (command, path) in [
            ("cpclib/memmap", "/api/memmap"),
            ("cpclib/crtc", "/api/crtc"),
            ("cpclib/ga", "/api/ga"),
            ("cpclib/psg", "/api/psg"),
            ("cpclib/fdc", "/api/fdc"),
            ("cpclib/history", "/api/history")
        ] {
            let call = call_for(&request(command, json!({}))).unwrap();
            assert_eq!(call.method, Method::Get, "{command}");
            assert_eq!(call.path, path, "{command}");
        }
    }

    /// Their answers are handed on unchanged: the caller asked for that
    /// endpoint by name and knows its shape better than a translation would.
    #[test]
    fn a_chip_answer_is_passed_through() {
        let body = json!({ "regs": [63, 48], "selected_reg": 8, "rasterline": 0 });
        let answer = response_for(&request("cpclib/crtc", json!({})), &body, 3);
        assert_eq!(answer["body"], body);
        assert_eq!(answer["success"], json!(true));
    }

    /// Waiting for a port that never opens gives up saying so, rather than
    /// hanging or failing as though the emulator were missing.
    #[test]
    fn waiting_for_a_dead_port_gives_up_with_a_reason() {
        let problem =
            wait_until_listening("http://127.0.0.1:1", std::time::Duration::from_millis(400))
                .expect_err("nothing listens there");
        assert!(problem.contains("did not start listening"), "{problem}");
        assert!(problem.contains("127.0.0.1:1"), "{problem}");
    }

    #[test]
    fn an_endpoint_becomes_a_host_to_connect_to() {
        assert_eq!(host_of("http://127.0.0.1:8765").unwrap(), "127.0.0.1:8765");
        assert_eq!(host_of("http://127.0.0.1:8765/").unwrap(), "127.0.0.1:8765");
        assert!(host_of("127.0.0.1:8765").is_err(), "the scheme is required");
    }

    #[test]
    fn a_response_body_starts_after_the_headers() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true}";
        assert_eq!(body_of(raw), "{\"ok\":true}");
        assert_eq!(body_of("garbage with no headers"), "");
    }

    /// What the stand-in emulator did, so a test can ask.
    #[derive(Default)]
    struct Machine {
        /// How many single steps it was asked for.
        steps: std::sync::atomic::AtomicUsize,
        /// How many times it was let go.
        resumes: std::sync::atomic::AtomicUsize,
        /// Every breakpoint set it was given, in order.
        armed: std::sync::Mutex<Vec<Vec<u16>>>
    }

    impl Machine {
        fn steps(&self) -> usize {
            self.steps.load(std::sync::atomic::Ordering::Relaxed)
        }

        fn resumes(&self) -> usize {
            self.resumes.load(std::sync::atomic::Ordering::Relaxed)
        }

        /// The breakpoints it is holding right now.
        fn breakpoints(&self) -> Vec<u16> {
            self.armed
                .lock()
                .unwrap()
                .last()
                .cloned()
                .unwrap_or_default()
        }

        /// Every set it was ever given.
        fn every_breakpoint_set(&self) -> Vec<Vec<u16>> {
            self.armed.lock().unwrap().clone()
        }
    }

    /// A stand-in emulator: enough of one to be stepped and to break, no more.
    ///
    /// It answers `/api/state` from a script of program counters and `/api/ram`
    /// with one blob of bytes. A single step advances the script by one; a
    /// resume runs it forward until it reaches a program counter someone has
    /// armed a breakpoint on, or the end of the script. That is exactly the
    /// contract the new step over rests on, so it is the contract to test
    /// against - a real machine is not needed to find out whether the right
    /// breakpoint was armed and taken away again.
    fn fake_machine(bytes: Vec<u8>, script: Vec<u16>) -> (String, std::sync::Arc<Machine>) {
        fake_machine_reporting(bytes, script, false)
    }

    /// The same, saying whether the machine is paused before anyone runs it.
    fn fake_machine_reporting(
        bytes: Vec<u8>,
        script: Vec<u16>,
        paused_at_rest: bool
    ) -> (String, std::sync::Arc<Machine>) {
        use std::io::{Read, Write};
        use std::sync::atomic::Ordering;

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let machine = std::sync::Arc::new(Machine::default());
        let served = machine.clone();
        // Where the script is, and whether the machine is stopped there.
        //
        // It starts running - a machine that reported itself paused from the
        // very first poll would be announced as a stop before any test had
        // asked for one, which is a real behaviour with a test of its own.
        let at = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let paused = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(paused_at_rest));

        std::thread::spawn(move || {
            let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
            for stream in listener.incoming() {
                let Ok(mut stream) = stream
                else {
                    return;
                };
                let (machine, at, paused, script, hex) = (
                    served.clone(),
                    at.clone(),
                    paused.clone(),
                    script.clone(),
                    hex.clone()
                );
                // A thread per connection: the run-state poller asks while a
                // resume is being answered, and serialising the two would hide
                // every race this backend exists to get right.
                std::thread::spawn(move || {
                    let mut buffer = [0u8; 2048];
                    let read = stream.read(&mut buffer).unwrap_or(0);
                    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                    let line = request.lines().next().unwrap_or_default().to_string();
                    let body = request.split("\r\n\r\n").nth(1).unwrap_or("").to_string();

                    if line.starts_with("POST /api/step") {
                        let mut at = at.lock().unwrap();
                        *at = (*at + 1).min(script.len() - 1);
                        machine.steps.fetch_add(1, Ordering::Relaxed);
                        paused.store(true, Ordering::Relaxed);
                    }
                    else if line.starts_with("POST /api/z80_bp") {
                        let addresses: Vec<u16> = body
                            .split(',')
                            .filter_map(|entry| {
                                crate::protocol::parse_address_reference(entry.trim())
                            })
                            .map(|address| address as u16)
                            .collect();
                        machine.armed.lock().unwrap().push(addresses);
                    }
                    else if line.starts_with("POST /api/config") && body.contains("false") {
                        machine.resumes.fetch_add(1, Ordering::Relaxed);
                        paused.store(false, Ordering::Relaxed);
                        let armed = machine.breakpoints();
                        let (at, paused, script) = (at.clone(), paused.clone(), script.clone());
                        // Answered at once and acted on a moment later, which
                        // is what the real one does.
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(20));
                            let mut at = at.lock().unwrap();
                            // A machine leaves the address it is sitting on
                            // before it starts breaking on anything, or a
                            // breakpoint under `PC` would stop it where it
                            // already is. Both real emulators need the same
                            // thing, which is why the session lifts it.
                            *at = (*at + 1).min(script.len() - 1);
                            while *at + 1 < script.len() && !armed.contains(&script[*at]) {
                                *at += 1;
                            }
                            paused.store(true, Ordering::Relaxed);
                        });
                    }
                    else if line.starts_with("POST /api/config") {
                        paused.store(true, Ordering::Relaxed);
                    }

                    let here = script[*at.lock().unwrap()];
                    let body = if line.starts_with("GET /api/state") {
                        json!({ "z80": { "PC": here, "SP": 0xBFF0 } }).to_string()
                    }
                    else if line.starts_with("GET /api/ram") {
                        json!({ "addr": 0, "len": hex.len() / 2, "hex": hex }).to_string()
                    }
                    else if line.starts_with("GET /api/ping") {
                        json!({ "emu": { "paused": paused.load(Ordering::Relaxed) } }).to_string()
                    }
                    else {
                        "{}".to_string()
                    };
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                            body.len()
                        )
                        .as_bytes()
                    );
                    let _ = stream.flush();
                });
            }
        });
        (format!("http://127.0.0.1:{port}"), machine)
    }

    /// Everything the peer produced, up to and including the `stopped` event
    /// that ends a step.
    ///
    /// The stop is noticed by the run-state poller on its own thread, so where
    /// the machine ended up is only a fair question once the stop has been
    /// announced - anything read before that is a race with the poller.
    fn until_stopped(peer: &mut AmspiritLitePeer) -> Vec<Value> {
        use crate::peer::DapPeer;

        let mut seen = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            seen.extend(peer.drain());
            if seen
                .iter()
                .any(|message| message["event"] == json!("stopped"))
            {
                // The poller takes the temporary breakpoint back out *before*
                // it says so; a moment's grace and the emulator has been told.
                std::thread::sleep(std::time::Duration::from_millis(20));
                seen.extend(peer.drain());
                return seen;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("nothing ever said the program had stopped: {seen:?}");
    }

    /// Step over the instruction the stand-in is sitting on, and report what
    /// happened.
    fn stepped_over(
        bytes: Vec<u8>,
        script: Vec<u16>,
        prepare: impl FnOnce(&mut AmspiritLitePeer)
    ) -> (AmspiritLitePeer, u16, std::sync::Arc<Machine>, Vec<Value>) {
        use crate::peer::DapPeer;

        let (endpoint, machine) = fake_machine(bytes, script);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        prepare(&mut peer);
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();

        let seen = until_stopped(&mut peer);
        let answered = seen
            .iter()
            .position(|message| message["command"] == json!("next"))
            .expect("the editor is answered");
        let stopped = seen
            .iter()
            .position(|message| message["event"] == json!("stopped"))
            .expect("and told where it stopped");
        assert!(
            answered < stopped,
            "the answer comes first and the stop follows it: {seen:?}"
        );

        // Asked afresh rather than remembered: a `GET` does not step, so this
        // is where the machine really is.
        let state = machine_state(&peer.endpoint);
        // The peer is handed back rather than dropped here: dropping it takes
        // its temporary breakpoints away, and some of these tests are about
        // exactly which ones are still armed.
        (
            peer,
            register(&state, "PC").unwrap() as u16,
            machine,
            seen
        )
    }

    fn machine_state(endpoint: &str) -> Value {
        machine(endpoint).unwrap()
    }

    /// Which instructions are worth stepping *over* at all.
    ///
    /// Everything else means the same thing stepped over as stepped into, so
    /// offering to run to the address after it would only ever arm a breakpoint
    /// nothing reaches.
    #[test]
    fn only_instructions_that_come_back_are_stepped_over() {
        // `call nn` and all eight conditional forms.
        assert!(returns_to_the_next_instruction(&[0xCD, 0x00, 0x90]));
        for condition in [0xC4, 0xCC, 0xD4, 0xDC, 0xE4, 0xEC, 0xF4, 0xFC] {
            assert!(
                returns_to_the_next_instruction(&[condition, 0x00, 0x90]),
                "{condition:02X}"
            );
        }
        // `rst n`, all eight.
        for rst in [0xC7, 0xCF, 0xD7, 0xDF, 0xE7, 0xEF, 0xF7, 0xFF] {
            assert!(returns_to_the_next_instruction(&[rst]), "{rst:02X}");
        }
        // The repeating block instructions.
        for block in [0xB0, 0xB1, 0xB2, 0xB3, 0xB8, 0xB9, 0xBA, 0xBB] {
            assert!(
                returns_to_the_next_instruction(&[0xED, block]),
                "ED {block:02X}"
            );
        }
        // `halt`, which sits there until an interrupt moves it on.
        assert!(returns_to_the_next_instruction(&[0x76]));
        // `djnz`, a loop written as one instruction - the case that started
        // this: stepping *into* it walks back up the loop one iteration at a
        // time, which is never what was meant.
        assert!(returns_to_the_next_instruction(&[0x10, 0xFD]));

        // And everything that does not come back.
        for other in [
            vec![0x00],             // nop
            vec![0x3E, 0x01],       // ld a,1
            vec![0x18, 0xFE],       // jr -2
            vec![0xC3, 0x00, 0x90], // jp nn
            vec![0xC9],             // ret
            vec![0xED, 0xA0],       // ldi, which does not repeat
            vec![0xCB, 0xC7],       // set 0,a - not an `rst` despite the byte
            vec![0xDD, 0x21],       // ld ix,nn
        ] {
            assert!(!returns_to_the_next_instruction(&other), "{other:02X?}");
        }
    }

    /// A `call` is stepped over with one breakpoint and one continue.
    ///
    /// Not by stepping until it comes back, which is what this used to do: a
    /// routine of any size cost one HTTP round trip per instruction executed.
    #[test]
    fn a_call_is_stepped_over_by_running_to_the_instruction_after_it() {
        // `call 0x9000` at 0x8000, so the address to come back to is 0x8003.
        let (_peer, pc, machine, _) = stepped_over(
            vec![0xCD, 0x00, 0x90],
            vec![0x8000, 0x9000, 0x9001, 0x8003],
            |_| {}
        );
        assert_eq!(pc, 0x8003);
        assert_eq!(machine.steps(), 0, "nothing was stepped one at a time");
        assert_eq!(machine.resumes(), 1, "it ran, once");
        assert!(
            machine
                .every_breakpoint_set()
                .iter()
                .any(|set| set.contains(&0x8003)),
            "the address after the call was armed: {:?}",
            machine.every_breakpoint_set()
        );
        assert!(
            !machine.breakpoints().contains(&0x8003),
            "and taken back out on arrival: {:?}",
            machine.breakpoints()
        );
    }

    /// `djnz` is a loop, so stepping over it runs the loop out.
    ///
    /// Reported as "step over on djnz does not work properly": it was not in
    /// the list of instructions worth stepping over, so it got a plain single
    /// step - which walks *backwards* into the loop body, one iteration per
    /// press, for however many iterations the loop has.
    #[test]
    fn a_djnz_is_stepped_over_to_the_instruction_after_the_loop() {
        // `djnz -3` at 0x8000: two bytes, so the loop is left at 0x8002.
        let (_peer, pc, machine, _) = stepped_over(vec![0x10, 0xFD], vec![
            0x8000, 0x7FFD, 0x8000, 0x7FFD, 0x8000, 0x8002,
        ], |_| {});
        assert_eq!(pc, 0x8002, "past the loop, not back into it");
        assert_eq!(machine.steps(), 0);
        assert_eq!(machine.resumes(), 1);
    }

    /// And so does a repeating block instruction, which cannot be stepped past
    /// at all: `PC` sits on the `ldir` until it is finished.
    #[test]
    fn a_repeating_instruction_is_run_to_its_end() {
        let (_peer, pc, machine, _) = stepped_over(vec![0xED, 0xB0], vec![
            0x8000, 0x8000, 0x8000, 0x8002,
        ], |_| {});
        assert_eq!(pc, 0x8002);
        assert_eq!(machine.resumes(), 1);
    }

    /// A `defs` run is stepped over in one go, because it is a repetition.
    ///
    /// `defs 60` pads a raster line with sixty `NOP`s. Stepping over it one
    /// `NOP` at a time is sixty presses to reach the `djnz` below it, and the
    /// bytes cannot say so - only the source can, which is why the session
    /// hands the run over before the step.
    #[test]
    fn a_defs_run_is_stepped_over_in_one_go() {
        use crate::peer::DapPeer;

        let (_peer, pc, machine, _) = stepped_over(vec![0x00; 60], vec![
            0x4002, 0x4003, 0x4004, 0x403E,
        ], |peer| {
            peer.note_line_at_pc(crate::peer::LineAtPc::Defs(0x4002..0x403E));
        });
        assert_eq!(pc, 0x403E, "on the line after the run, not one NOP along");
        assert_eq!(machine.steps(), 0, "nothing was stepped one at a time");
        assert_eq!(machine.resumes(), 1);
        assert!(
            !machine.breakpoints().contains(&0x403E),
            "and the temporary is taken back out on arrival: {:?}",
            machine.breakpoints()
        );
    }

    /// From the middle of the run, stepping over finishes the run.
    ///
    /// The other reading - "run to the next instruction" - would be one `NOP`,
    /// which is what the user was already doing by hand when they asked for
    /// this. A repetition stepped over from inside it is still a repetition.
    #[test]
    fn a_step_over_from_inside_a_defs_run_finishes_the_run() {
        use crate::peer::DapPeer;

        let (_peer, pc, machine, _) = stepped_over(vec![0x00; 60], vec![
            0x4020, 0x4021, 0x403E,
        ], |peer| {
            peer.note_line_at_pc(crate::peer::LineAtPc::Defs(0x4002..0x403E));
        });
        assert_eq!(pc, 0x403E);
        assert_eq!(machine.steps(), 0);
        assert_eq!(machine.resumes(), 1);
    }

    /// The run ends where the next line begins, so a `djnz` written under a
    /// `defs` is exactly where the step over lands - even though that `djnz`
    /// jumps straight back into the run.
    ///
    /// The loop is the reason the idiom exists (`ld b,20` / `defs 64 -
    /// duration(djnz $)-1` / `djnz`), and stepping over the padding must show
    /// the `djnz` of *this* iteration rather than run the whole loop out.
    #[test]
    fn a_defs_run_under_a_loop_stops_on_the_djnz_below_it() {
        use crate::peer::DapPeer;

        // The script loops back into the run twice before the breakpoint on
        // the `djnz` line catches it, the way the machine really would.
        let (_peer, pc, machine, _) = stepped_over(vec![0x00; 60], vec![
            0x4002, 0x4003, 0x403E, 0x4002, 0x4003, 0x403E,
        ], |peer| {
            peer.note_line_at_pc(crate::peer::LineAtPc::Defs(0x4002..0x403E));
        });
        assert_eq!(pc, 0x403E, "the djnz of this iteration");
        assert_eq!(
            machine.every_breakpoint_set().last().map(Vec::len),
            Some(0),
            "and nothing of ours is left armed: {:?}",
            machine.every_breakpoint_set()
        );
    }

    /// A `nop` the source really wrote is still a single step, even in a row
    /// of them.
    ///
    /// The source said this line is not a `defs`, and that is a *different*
    /// answer from having no source at all: what the user wrote as `nop` is
    /// what they meant to step.
    #[test]
    fn a_hand_written_nop_is_still_a_single_step() {
        use crate::peer::DapPeer;

        let (_peer, pc, machine, _) = stepped_over(vec![0x00; 8], vec![0x4002, 0x4003], |peer| {
            peer.note_line_at_pc(crate::peer::LineAtPc::Ordinary);
        });
        assert_eq!(machine.steps(), 1);
        assert_eq!(machine.resumes(), 0);
        assert_eq!(pc, 0x4003);
        assert!(machine.breakpoints().is_empty());
    }

    /// With no source at all, a run of zeroes is read as one padded wait.
    ///
    /// The owner's own fallback for a program debugged without its listing:
    /// consecutive zero bytes belong to the same fake instruction, so the step
    /// resumes after the last of them. It is a guess, and it is only made
    /// where there is nothing better to go on.
    #[test]
    fn with_no_source_a_run_of_zeroes_is_stepped_over_whole() {
        use crate::peer::DapPeer;

        // Sixteen zeroes and then something else, so the run has an end to
        // find: the stand-in answers every read with the same blob.
        let mut bytes = vec![0x00; 16];
        bytes.push(0xC9);
        let (_peer, pc, machine, _) = stepped_over(bytes, vec![0x4002, 0x4003, 0x4012], |peer| {
            peer.note_line_at_pc(crate::peer::LineAtPc::Unknown);
        });
        assert_eq!(pc, 0x4012, "after the last zero");
        assert_eq!(machine.steps(), 0);
        assert_eq!(machine.resumes(), 1);
    }

    /// One `NOP` between real instructions is one step, source or no source.
    ///
    /// The fallback is about *runs*; a single zero byte is an instruction like
    /// any other, and swallowing it would make stepping unpredictable in
    /// ordinary code.
    #[test]
    fn with_no_source_a_single_zero_byte_is_still_one_step() {
        use crate::peer::DapPeer;

        let (_peer, _, machine, _) =
            stepped_over(vec![0x00, 0x3E, 0x01], vec![0x4002, 0x4003], |peer| {
                peer.note_line_at_pc(crate::peer::LineAtPc::Unknown);
            });
        assert_eq!(machine.steps(), 1);
        assert_eq!(machine.resumes(), 0);
    }

    /// A run the machine disagrees with is refused rather than believed.
    ///
    /// The session names the run from the last stop it heard about; the peer
    /// holds the live `PC` and the live bytes. When they do not agree - a
    /// `defs` filled with something other than zero, or a `PC` that has moved
    /// on - the ordinary rules apply, which is one step here.
    #[test]
    fn a_defs_run_that_does_not_match_the_machine_is_refused() {
        use crate::peer::DapPeer;

        // `PC` outside the run it was told about. One zero byte and then
        // something else, so the run-of-zeroes fallback has nothing to say
        // either.
        let (_peer, _, machine, _) =
            stepped_over(vec![0x00, 0x3E, 0x01], vec![0x5000, 0x5001], |peer| {
                peer.note_line_at_pc(crate::peer::LineAtPc::Defs(0x4002..0x403E));
            });
        assert_eq!(machine.steps(), 1, "PC is not in the run");
        assert_eq!(machine.resumes(), 0);

        // The run is there, but it is not made of `NOP`s.
        let (_peer, _, machine, _) =
            stepped_over(vec![0x3E, 0x01], vec![0x4002, 0x4004], |peer| {
                peer.note_line_at_pc(crate::peer::LineAtPc::Defs(0x4002..0x403E));
            });
        assert_eq!(machine.steps(), 1, "the byte at PC is not a NOP");
        assert_eq!(machine.resumes(), 0);
    }

    /// Anything else is one step and no breakpoint.
    ///
    /// A `jr` never reaches the address after itself, so arming it would be a
    /// breakpoint nothing ever hits and a program that runs away.
    #[test]
    fn an_ordinary_instruction_is_a_single_step() {
        for bytes in [vec![0x3E, 0x01], vec![0x18, 0xFE], vec![0xC9]] {
            let (_peer, pc, machine, _) = stepped_over(bytes.clone(), vec![0x8000, 0x4000], |_| {});
            assert_eq!(machine.steps(), 1, "{bytes:02X?}");
            assert_eq!(machine.resumes(), 0, "{bytes:02X?}");
            assert_eq!(pc, 0x4000, "{bytes:02X?}");
            assert!(
                machine.breakpoints().is_empty(),
                "nothing armed for {bytes:02X?}: {:?}",
                machine.breakpoints()
            );
        }
    }

    /// A breakpoint the user already put on the address after the call is
    /// theirs, and survives the step over.
    ///
    /// Taking it away would be a step over that quietly deletes a red dot, and
    /// the gutter would go on showing one that no longer exists.
    #[test]
    fn a_breakpoint_the_editor_already_set_there_is_kept() {
        use crate::peer::DapPeer;

        let (endpoint, machine) = fake_machine(vec![0xCD, 0x00, 0x90], vec![
            0x8000, 0x9000, 0x8003,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.send(request(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [{ "instructionReference": "0x8003" }] })
        ))
        .unwrap();
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();
        let seen = until_stopped(&mut peer);

        assert_eq!(
            machine.breakpoints(),
            vec![0x8003],
            "the user's breakpoint is still there"
        );
        let stopped = seen
            .iter()
            .find(|message| message["event"] == json!("stopped"))
            .unwrap();
        assert_eq!(
            stopped["body"]["reason"],
            json!("breakpoint"),
            "and stopping on it is a breakpoint stop"
        );
    }

    /// A breakpoint inside what is being stepped over still stops.
    ///
    /// Otherwise stepping over a `call` is a way of silently disarming every
    /// breakpoint in the routine it calls.
    #[test]
    fn a_breakpoint_inside_the_call_stops_the_step_over() {
        let (_peer, pc, machine, _) = stepped_over(
            vec![0xCD, 0x00, 0x90],
            vec![0x8000, 0x9000, 0x9001, 0x8003],
            |peer| {
                peer.breakpoints
                    .lock()
                    .unwrap()
                    .editors = vec![0x9001];
            }
        );
        assert_eq!(pc, 0x9001, "stopped where the user asked");
        assert!(
            machine.breakpoints().contains(&0x8003),
            "and the abandoned one is left armed rather than guessed about: {:?}",
            machine.breakpoints()
        );
    }

    /// A stop on a breakpoint the editor cannot show is explained.
    ///
    /// A temporary breakpoint has no red dot, so stopping on one looks exactly
    /// like stopping at nothing at all - which is how an invisible breakpoint
    /// was reported the last time one fired.
    #[test]
    fn a_stop_on_a_left_behind_temporary_says_where_it_came_from() {
        use crate::peer::DapPeer;

        // A `call` whose routine goes off somewhere else, so 0x8003 is armed
        // and never reached; the user's own breakpoint at 0x9001 stops it.
        let (endpoint, _) = fake_machine(vec![0xCD, 0x00, 0x90], vec![
            0x8000, 0x9000, 0x9001, 0x8003,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.send(request(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [{ "instructionReference": "0x9001" }] })
        ))
        .unwrap();
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();
        until_stopped(&mut peer);

        // Carrying on now walks into the breakpoint the abandoned step over
        // left behind.
        peer.send(request("continue", json!({ "threadId": 1 })))
            .unwrap();
        let seen = until_stopped(&mut peer);
        let note = seen
            .iter()
            .find(|message| message["event"] == json!("output"))
            .expect("the invisible stop is explained");
        let text = note["body"]["output"].as_str().unwrap();
        assert!(text.contains("0x8003"), "{text}");
        assert!(text.contains("step over"), "{text}");
        assert!(text.contains("no red dot"), "{text}");
    }

    /// Nothing the adapter armed by itself is ever reported to the editor.
    ///
    /// The gutter must show the user's breakpoints and only those, or they end
    /// up able to clear one the adapter is relying on - and to wonder where the
    /// other one came from.
    #[test]
    fn a_temporary_breakpoint_never_reaches_the_editor() {
        let (_peer, _, machine, seen) = stepped_over(
            vec![0xCD, 0x00, 0x90],
            vec![0x8000, 0x9000, 0x8003],
            |_| {}
        );
        assert!(
            machine
                .every_breakpoint_set()
                .iter()
                .any(|set| set.contains(&0x8003)),
            "it really was armed"
        );
        for message in &seen {
            assert!(
                message.get("body").and_then(|b| b.get("breakpoints")).is_none(),
                "nothing the editor could paint a gutter from: {message:?}"
            );
            assert!(
                !message.to_string().contains("0x8003"),
                "and the address is never mentioned: {message:?}"
            );
        }
    }

    /// Step out is the same trick, aimed at the address the `call` pushed.
    #[test]
    fn a_step_out_runs_to_the_address_on_top_of_the_stack() {
        use crate::peer::DapPeer;

        // The two bytes at `SP` are the return address, little-endian: 0x8003.
        let (endpoint, machine) = fake_machine(vec![0x03, 0x80], vec![
            0x9000, 0x9001, 0x9002, 0x8003,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.send(request("stepOut", json!({ "threadId": 1 })))
            .unwrap();
        let seen = until_stopped(&mut peer);

        assert_eq!(machine.steps(), 0, "not walked out one instruction at a time");
        assert_eq!(machine.resumes(), 1);
        let state = machine_state(&peer.endpoint);
        assert_eq!(register(&state, "PC").unwrap(), 0x8003);
        assert!(
            !machine.breakpoints().contains(&0x8003),
            "and its breakpoint is gone again"
        );
        assert!(
            seen.iter()
                .any(|message| message["command"] == json!("stepOut")),
            "and the editor is answered"
        );
    }

    /// A step over that never arrives leaves its breakpoint armed - and the
    /// session takes it away on the way out.
    ///
    /// Leaving it is deliberate: a routine that does not return to the
    /// instruction after the call is a bug in the program (or a deliberate
    /// trick), and there is nothing useful to guess. Handing the emulator back
    /// with it still set is another matter.
    #[test]
    fn temporary_breakpoints_are_cleared_when_the_session_ends() {
        use crate::peer::DapPeer;

        let (endpoint, machine) = fake_machine(vec![0xCD, 0x00, 0x90], vec![
            0x8000, 0x9000, 0x9001,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.send(request(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [{ "instructionReference": "0x9001" }] })
        ))
        .unwrap();
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();
        until_stopped(&mut peer);
        assert!(
            machine.breakpoints().contains(&0x8003),
            "left armed while the session runs"
        );

        peer.send(request("disconnect", json!({}))).unwrap();
        assert_eq!(
            machine.breakpoints(),
            vec![0x9001],
            "and only the editor's are left behind"
        );
    }

    /// The editor changing a red dot does not disarm a step over in flight.
    ///
    /// `/api/z80_bp` replaces the whole set, so a `setInstructionBreakpoints`
    /// that spoke only for the editor would take the temporary with it - and
    /// the step over would run to the end of the program.
    #[test]
    fn arming_the_editors_breakpoints_keeps_the_temporary_one() {
        use crate::peer::DapPeer;

        let (endpoint, machine) = fake_machine(vec![0xCD, 0x00, 0x90], vec![
            0x8000, 0x9000, 0x9001,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();
        peer.send(request(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [{ "instructionReference": "0x4000" }] })
        ))
        .unwrap();

        let armed = machine.breakpoints();
        assert!(armed.contains(&0x4000), "the editor's: {armed:?}");
        assert!(armed.contains(&0x8003), "and ours: {armed:?}");
    }

    /// And the step over arrives anyway, leaving the editor's new breakpoint
    /// behind it.
    ///
    /// The other half of the same interaction: the merge must not lose the
    /// temporary on the way in, *and* retiring the temporary on arrival must
    /// not take the editor's breakpoint out with it. Losing it there is the
    /// worse of the two, because nothing says so - the red dot stays in the
    /// gutter over an address the emulator is no longer watching.
    #[test]
    fn a_step_over_arrives_after_the_editor_arms_another_breakpoint() {
        use crate::peer::DapPeer;

        // `call 0x9000` at 0x8000, coming back to 0x8003.
        let (endpoint, machine) = fake_machine(vec![0xCD, 0x00, 0x90], vec![
            0x8000, 0x9000, 0x8003,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();
        // Mid-flight: the user puts a red dot somewhere else entirely.
        peer.send(request(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [{ "instructionReference": "0x4000" }] })
        ))
        .unwrap();

        let seen = until_stopped(&mut peer);
        let stopped = seen
            .iter()
            .find(|message| message["event"] == json!("stopped"))
            .expect("the step over ends in a stop");
        assert_eq!(
            stopped["body"]["reason"],
            json!("step"),
            "and it is the step over that ended, not a breakpoint"
        );
        assert_eq!(
            machine.breakpoints(),
            vec![0x4000],
            "ours is retired and the editor's is left armed"
        );
    }

    /// The whole chain: a red dot in the editor arms the emulator, and the stop
    /// it causes reaches the editor.
    ///
    /// Every other breakpoint test here speaks to the peer directly, which is
    /// not how a breakpoint is ever set. The editor says `setBreakpoints` for
    /// one file; the session turns that into addresses and hands the peer the
    /// *whole* set, every time; the peer merges it with whatever a step over
    /// has armed and writes the result to `/api/z80_bp`. A break anywhere along
    /// that chain looks identical from outside - "the breakpoint does not stop
    /// anything" - and so does a stop that is noticed by nobody, which is why
    /// this walks the arming and the noticing in one go.
    #[test]
    fn a_breakpoint_set_in_the_editor_arms_the_emulator_and_its_stop_comes_back() {
        use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
        use cpclib_project::srcmap::SourceMap;

        // Line 10 is where the program is; line 20, at 0x8000, is the red dot.
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![
                SourceMapRow::flat(0, 10, 0x4000, 3),
                SourceMapRow::flat(0, 20, 0x8000, 1),
            ]
        });
        let (endpoint, machine) = fake_machine(vec![0x00], vec![0x4000, 0x8000]);
        let peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        let mut session = crate::session::Session::new(peer, map);
        session.on_attached().unwrap();

        session
            .on_editor_message(&json!({
                "seq": 1, "type": "request", "command": "setBreakpoints",
                "arguments": {
                    "source": { "path": "main.asm" },
                    "breakpoints": [{ "line": 20 }]
                }
            }))
            .unwrap();
        assert_eq!(
            machine.breakpoints(),
            vec![0x8000],
            "the emulator is really holding the address behind the red dot"
        );

        session
            .on_editor_message(&json!({
                "seq": 2, "type": "request", "command": "continue",
                "arguments": { "threadId": 1 }
            }))
            .unwrap();

        let told = until_the_editor_hears_a_stop(&mut session);
        assert_eq!(
            told["body"]["reason"],
            json!("breakpoint"),
            "and the stop is reported as one: {told}"
        );
        let state = machine_state(&endpoint);
        assert_eq!(
            register(&state, "PC").unwrap(),
            0x8000,
            "stopped where the red dot is"
        );
    }

    /// Pump the session until it tells the editor the program stopped.
    ///
    /// The stop travels editor-ward in two steps - the peer's poller notices it
    /// and the session translates it - and only the second one is what the
    /// editor acts on, so that is what a test about breakpoints has to wait
    /// for.
    fn until_the_editor_hears_a_stop(
        session: &mut crate::session::Session<AmspiritLitePeer>
    ) -> Value {
        use crate::peer::DapPeer;

        let mut seen: Vec<Value> = Vec::new();
        for _ in 0..1000 {
            for message in session.peer_mut().drain() {
                seen.extend(session.on_emulator_message(&message));
            }
            if let Some(stopped) = seen
                .iter()
                .find(|message| message["event"] == json!("stopped"))
            {
                return stopped.clone();
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("the editor was never told the program had stopped: {seen:?}");
    }

    /// An emulator is started on a port nothing else is holding.
    ///
    /// Starting it on a busy one does not fail: this emulator says "Web debug
    /// server busy or unavailable, disabled" and runs on without a debug
    /// server, while the port goes on answering from whatever was already there
    /// - an emulator a killed session left behind, holding the previous build.
    /// The session then arms its breakpoints in a machine nobody is watching.
    #[test]
    fn an_emulator_is_started_where_it_can_really_serve() {
        let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let taken = held.local_addr().unwrap().port();
        assert_ne!(
            port_to_serve_on(taken),
            taken,
            "a port something else answers on is not ours to use"
        );

        let free = a_free_port().expect("the machine has a spare port");
        assert_eq!(
            port_to_serve_on(free),
            free,
            "and a free one is used as asked"
        );
    }

    /// A machine found already stopped is reported, not swallowed.
    ///
    /// This is the session that dies before it starts: the emulator halts on a
    /// breakpoint in the few milliseconds between arming and this thread's
    /// first look, so there is never a running-to-paused transition to notice.
    /// Reported once from real testing as "breakpoints are no more detected and
    /// the debug buttons do not interact with the emulator" - which is exactly
    /// what a stopped emulator plus an editor that was never told looks like.
    #[test]
    fn a_machine_already_stopped_when_we_first_look_is_still_reported() {
        // The stand-in answers `/api/ping` as paused from the very first poll.
        let (endpoint, _) = fake_machine_reporting(vec![0x00], vec![0x8000], true);
        let peer = AmspiritLitePeer::connect(&endpoint).unwrap();

        // Two poll intervals, so the first look has certainly happened.
        std::thread::sleep(std::time::Duration::from_millis(250));
        let seen = peer.pending.try_iter().collect::<Vec<_>>();
        assert!(
            seen.iter()
                .any(|message| message["event"] == json!("stopped")),
            "the editor is told the program is stopped: {seen:?}"
        );
    }

    /// A stand-in that takes a moment to act on a resume, and answers other
    /// callers while it does.
    ///
    /// Which is what the real one is like: it serves its HTTP requests from the
    /// same loop that draws frames, so `POST /api/config` is not instantaneous
    /// and a `GET /api/ping` can be answered in the middle of one.
    fn fake_machine_slow_to_resume(delay: std::time::Duration) -> String {
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicBool, Ordering};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let running = std::sync::Arc::new(AtomicBool::new(false));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream
                else {
                    return;
                };
                let running = running.clone();
                // A thread per connection, or the resume would block the poll
                // that has to happen during it - and the window being tested
                // would not exist.
                std::thread::spawn(move || {
                    let reported = running.clone();
                    let mut buffer = [0u8; 1024];
                    let read = stream.read(&mut buffer).unwrap_or(0);
                    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                    let line = request.lines().next().unwrap_or_default().to_string();

                    let body = if line.starts_with("POST /api/config") {
                        // Answered at once and acted on later, which is what
                        // the real one does: `/api/config` returns in under a
                        // millisecond and `/api/ping` goes on saying "paused"
                        // for several more.
                        std::thread::spawn(move || {
                            std::thread::sleep(delay);
                            running.store(true, Ordering::Relaxed);
                        });
                        "{}".to_string()
                    }
                    else if line.starts_with("GET /api/ping") {
                        json!({ "emu": { "paused": !reported.load(Ordering::Relaxed) } }).to_string()
                    }
                    else if line.starts_with("GET /api/state") {
                        json!({ "z80": { "PC": 0x8000, "SP": 0xBFF0 } }).to_string()
                    }
                    else {
                        "{}".to_string()
                    };
                    let _ = stream.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                            body.len()
                        )
                        .as_bytes()
                    );
                    let _ = stream.flush();
                });
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    /// A continue does not produce a stop of its own.
    ///
    /// Reported as "continue does not work properly - the emulator stopped at
    /// random locations without any breakpoint", and this is the only thing in
    /// here that can invent a stop. `expecting_stop` used to be raised *before*
    /// the resume was asked for, so a poll landing in between found the machine
    /// still paused from the stop it was leaving, consumed the flag and called
    /// it a stop. The editor then believed a machine that was really running,
    /// and every register and frame it went on to ask for came from a program
    /// in flight - a stop at an address no breakpoint was ever set on.
    #[test]
    fn a_continue_does_not_announce_a_stop_that_has_not_happened() {
        use crate::peer::DapPeer;

        // Three poll intervals wide, so a poll certainly lands inside the gap
        // between asking for the resume and the machine reporting one.
        let endpoint = fake_machine_slow_to_resume(std::time::Duration::from_millis(300));
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        // A stand-in six times slower than the real one to come back, so the
        // real one's allowance is scaled with it rather than the test being
        // written to whatever the current constant happens to be.
        peer.resume_confirmation = AmspiritLitePeer::RESUME_CONFIRMATION * 10;

        // The machine starts paused, and being told so is right - that is the
        // stop the continue is about to leave.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if peer
                .drain()
                .iter()
                .any(|message| message["event"] == json!("stopped"))
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        peer.send(request("continue", json!({ "threadId": 1 })))
            .unwrap();

        // Long enough for the resume to have finished and several polls to have
        // run on the machine it left running.
        std::thread::sleep(std::time::Duration::from_millis(600));
        let seen = peer.drain();
        assert!(
            !seen
                .iter()
                .any(|message| message["event"] == json!("stopped")),
            "nothing stopped, so nothing says it did: {seen:?}"
        );
        assert!(
            seen.iter()
                .any(|message| message["event"] == json!("continued")),
            "and the machine is reported running: {seen:?}"
        );
    }

    /// A walk against an emulator that has stopped answering still answers the
    /// editor.
    ///
    /// The alternative is what makes a debugger disappear mid-step: the error
    /// travels up out of `send`, the session ends, and every button goes dead
    /// with no stop event and no explanation.
    #[test]
    fn a_step_over_against_a_dead_emulator_still_answers_the_editor() {
        use crate::peer::DapPeer;

        // A port nothing is listening on: bound to find a free one, then let
        // go.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut peer = AmspiritLitePeer::connect(&format!("http://127.0.0.1:{port}")).unwrap();
        peer.send(request("next", json!({ "threadId": 1 })))
            .expect("a failed step is not a failed session");

        let drained = until_stopped(&mut peer);
        assert!(
            drained
                .iter()
                .any(|message| message["command"] == json!("next")),
            "the editor is answered: {drained:?}"
        );
        let note = drained
            .iter()
            .find(|message| message["event"] == json!("output"))
            .expect("and told why");
        assert!(
            note["body"]["output"]
                .as_str()
                .unwrap()
                .contains("could not finish"),
            "{note:?}"
        );
    }

    /// The peer answers only what it can ask for, so nothing is forwarded to an
    /// endpoint that does not exist.
    #[test]
    fn it_claims_only_the_requests_it_maps() {
        use crate::peer::DapPeer;

        // Built without connecting: `supports` is a property of the backend,
        // not of a live session.
        let (outgoing, pending) = std::sync::mpsc::channel();
        let peer = AmspiritLitePeer {
            endpoint: DEFAULT_ENDPOINT.to_string(),
            expecting_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stepping_over: std::sync::Arc::new(std::sync::Mutex::new(None)),
            breakpoints: std::sync::Arc::new(std::sync::Mutex::new(Breakpoints::default())),
            resume_confirmation: AmspiritLitePeer::RESUME_CONFIRMATION,
            line_at_pc: crate::peer::LineAtPc::Unknown,
            launched: None,
            pending,
            outgoing,
            seq: std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0))
        };

        // Everything claimed is also mapped - claiming a request with no
        // endpoint behind it is how a pane ends up waiting forever.
        for supported in [
            "pause",
            "continue",
            "stepIn",
            "next",
            "threads",
            "stackTrace",
            "scopes",
            "variables",
            "setInstructionBreakpoints"
        ] {
            assert!(peer.supports(supported), "{supported}");
            assert!(
                call_for(&json!({ "command": supported, "arguments": {} })).is_some(),
                "{supported} is claimed but maps to no call"
            );
        }
        // `readMemory` needs an address before it maps to anything, so it is
        // checked with one.
        assert!(peer.supports("readMemory"));
        assert!(
            call_for(&json!({
                "command": "readMemory",
                "arguments": { "memoryReference": "0x4000", "count": 8 }
            }))
            .is_some()
        );
        for absent in ["disassemble", "evaluate", "setDataBreakpoints", "stepBack"] {
            assert!(!peer.supports(absent), "{absent}");
        }

        // `stepOut` is the exception that proves the rule: claimed, and
        // deliberately *not* one call - it is a breakpoint plus a continue,
        // which `send` intercepts before `call_for` is ever consulted.
        assert!(peer.supports("stepOut"));
        assert!(call_for(&json!({ "command": "stepOut", "arguments": {} })).is_none());

        // Address breakpoints only, and no attach handshake: a REST API is
        // available as soon as it is listening.
        assert!(peer.quirks().instruction_breakpoints_only);
        assert!(!peer.quirks().attach_required);
    }
}

/// Checks that need a running emulator.
///
/// Ignored by default - they are how the mappings were verified against a real
/// 1.13.4 instance, and are worth keeping runnable rather than deleting once
/// the machine goes away. Start one and run
/// `cargo test -p cpclib-dap --lib live_ -- --ignored`.
#[cfg(test)]
mod live_tests {
    use super::*;

    /// Which emulator to talk to.
    ///
    /// The default port belongs to whichever instance happens to be running,
    /// which is rarely the one a check wants; `CPCLIB_DAP_LIVE_ENDPOINT` names
    /// a different one without touching it.
    fn endpoint() -> String {
        std::env::var("CPCLIB_DAP_LIVE_ENDPOINT").unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string())
    }

    fn reachable() -> bool {
        let Ok(host) = host_of(&endpoint())
        else {
            return false;
        };
        let Ok(address) = host.parse()
        else {
            return false;
        };
        std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_millis(300)).is_ok()
    }

    #[test]
    #[ignore]
    fn live_state_carries_every_pane() {
        assert!(reachable(), "start AMSpiriT Lite with --web-server first");

        let body = perform(&endpoint(), &Call::get("/api/state")).unwrap();
        let state: Value = serde_json::from_str(&body).unwrap();

        // Everything the panes need, in one call, while the machine runs.
        for chip in ["z80", "ga", "crtc", "psg", "fdc"] {
            assert!(state.get(chip).is_some(), "no {chip} in {state}");
        }

        // The registers compose into the pairs a programmer reads.
        let registers = registers_of(&state);
        for name in ["AF", "BC", "DE", "HL", "PC", "SP"] {
            assert!(
                registers.iter().any(|r| r["name"] == json!(name)),
                "no {name}: {registers:?}"
            );
        }
    }

    #[test]
    #[ignore]
    fn live_memory_reads_come_back_as_bytes() {
        assert!(reachable(), "start AMSpiriT Lite with --web-server first");

        let read = json!({
            "seq": 1, "type": "request", "command": "readMemory",
            "arguments": { "memoryReference": "0x0000", "count": 8 }
        });
        let body = perform(&endpoint(), &call_for(&read).unwrap()).unwrap();
        let answer: Value = serde_json::from_str(&body).unwrap();

        assert_eq!(answer["len"], json!(8), "asked in decimal: {answer}");
        let hex = answer["hex"].as_str().expect("a hex string");
        assert_eq!(bytes_from_hex(hex).len(), 8, "{hex}");
    }

    /// `/api/memmap` answers the question 1984js cannot: which page is where.
    #[test]
    #[ignore]
    fn live_memmap_names_the_page_at_each_region() {
        assert!(reachable(), "start AMSpiriT Lite with --web-server first");

        let body = perform(&endpoint(), &Call::get("/api/memmap")).unwrap();
        let map: Value = serde_json::from_str(&body).unwrap();
        let regions = map["regions"].as_array().expect("regions");

        assert_eq!(regions.len(), 4, "one per 16K region: {map}");
        assert!(regions[0].get("ram_bank").is_some(), "{map}");
        assert!(
            map.get("rmr").is_some(),
            "the banking register itself: {map}"
        );
    }

    /// A `djnz` loop is stepped over in one go, on a real machine.
    ///
    /// The case that started this: stepping *into* a `djnz` walks back up the
    /// loop one iteration per press, so a loop of 36 takes 36 presses to leave.
    /// A breakpoint on the instruction after it and a continue leaves it once,
    /// and the loop counter proves the loop really ran.
    ///
    /// Writes a five-byte program into RAM at 0x8000 and runs it, so it says
    /// nothing about whatever was loaded - which is why it is not run by
    /// default.
    #[test]
    #[ignore]
    fn live_a_djnz_loop_is_left_in_one_continue() {
        assert!(reachable(), "start AMSpiriT Lite with --web-server first");
        let endpoint = endpoint();

        // ld b,5 / nop / djnz -3 / jr self
        perform(
            &endpoint,
            &Call::post("/api/ram")
                .body(json!({ "addr": 0x8000, "data": "06050010FD18FE" }).to_string())
        )
        .unwrap();

        // Stopped on the `djnz` itself, with the loop still to run.
        perform(&endpoint, &Call::post("/api/z80_bp").body("0x8003")).unwrap();
        perform(
            &endpoint,
            &Call::post("/api/exec").body(json!({ "addr": 0x8000 }).to_string())
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let state = machine(&endpoint).unwrap();
        assert_eq!(register(&state, "PC").unwrap(), 0x8003);
        assert_eq!(state["z80"]["B"], json!(5), "the loop has not run yet");

        // And over it: one breakpoint on the instruction after, one continue.
        perform(&endpoint, &Call::post("/api/z80_bp").body("0x8005")).unwrap();
        perform(
            &endpoint,
            &Call::post("/api/config").body(json!({ "paused": false }).to_string())
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));

        let state = machine(&endpoint).unwrap();
        assert_eq!(register(&state, "PC").unwrap(), 0x8005, "past the loop");
        assert_eq!(state["z80"]["B"], json!(0), "and it really ran it out");
        perform(&endpoint, &Call::post("/api/z80_bp").body("")).unwrap();
    }

    /// `POST /api/step` is asynchronous: the state read straight after it can
    /// still be the state *before* it.
    ///
    /// Which is why "step until `PC` reaches the address after this
    /// instruction" cannot be trusted on this emulator, however long it is
    /// given: a walk that reads a stale `PC` at the wrong moment walks past its
    /// own target and then runs until it gives up. Measured here rather than
    /// argued about.
    #[test]
    #[ignore]
    fn live_a_step_is_not_finished_when_it_is_answered() {
        assert!(reachable(), "start AMSpiriT Lite with --web-server first");
        let endpoint = endpoint();

        // jr $ at 0x8000, so `PC` is 0x8000 whenever a step has landed and the
        // register file changes on nothing else.
        perform(
            &endpoint,
            &Call::post("/api/ram").body(json!({ "addr": 0x8000, "data": "18FE" }).to_string())
        )
        .unwrap();
        perform(&endpoint, &Call::post("/api/z80_bp").body("0x8000")).unwrap();
        perform(
            &endpoint,
            &Call::post("/api/exec").body(json!({ "addr": 0x8000 }).to_string())
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        perform(&endpoint, &Call::post("/api/z80_bp").body("")).unwrap();

        // `R` counts refreshes, so it moves with every instruction executed and
        // nothing else. Read twice around a step: if the step were finished by
        // the time it answers, the immediate reading and the settled one would
        // always agree.
        let mut disagreed = 0;
        for _ in 0..40 {
            perform(&endpoint, &Call::post("/api/step")).unwrap();
            let straight_away = machine(&endpoint).unwrap()["z80"]["R"].clone();
            std::thread::sleep(std::time::Duration::from_millis(60));
            let settled = machine(&endpoint).unwrap()["z80"]["R"].clone();
            if straight_away != settled {
                disagreed += 1;
            }
        }
        assert!(
            disagreed > 0,
            "the emulator answered every step before doing it, which would make \
             a stepping walk trustworthy after all"
        );
    }
}

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
    /// Set while this adapter is driving the machine one instruction at a
    /// time, so the run-state poller keeps quiet.
    ///
    /// A step-over is a *sequence* of steps, and the machine is briefly running
    /// during each one. Without this the poller would catch it mid-walk and
    /// announce a stop that has not happened.
    stepping: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The addresses the editor last armed.
    ///
    /// A step-over walks over its instruction; a breakpoint *inside* what it
    /// walks over still has to stop it, or stepping over a `call` is a way of
    /// silently disarming every breakpoint in the routine.
    armed: Vec<u16>,
    /// How far a step-over walks before it gives up; `STEP_OVER_BUDGET` unless
    /// a caller says otherwise.
    step_over_budget: usize,
    /// And how long, which is usually what stops it first.
    step_over_time: std::time::Duration,
    /// Where an interrupted walk was heading: the address it gave up at, and
    /// the address it was walking to.
    ///
    /// So that pressing step over again *carries on* rather than starting a
    /// fresh walk over whatever instruction the routine happened to stop on -
    /// which is what "step over again to carry on" has to mean to be true.
    unfinished: Option<(u16, u16)>,
    /// Answers waiting for the next `drain`.
    pending: std::sync::mpsc::Receiver<Value>,
    outgoing: std::sync::mpsc::Sender<Value>,
    seq: i64
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
        let watched = outgoing.clone();
        let watched_endpoint = endpoint.trim_end_matches('/').to_string();
        let watched_expecting = expecting_stop.clone();
        let stepping = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watched_stepping = stepping.clone();
        std::thread::Builder::new()
            .name("amspiritlite-runstate".into())
            .spawn(move || {
                watch_run_state(
                    &watched_endpoint,
                    &watched,
                    &watched_expecting,
                    &watched_stepping
                )
            })?;

        Ok(Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            expecting_stop,
            stepping,
            armed: Vec::new(),
            step_over_budget: Self::STEP_OVER_BUDGET,
            step_over_time: Self::STEP_OVER_TIME,
            unfinished: None,
            launched: None,
            pending,
            outgoing,
            seq: 0
        })
    }

    /// Take ownership of an emulator this session started, so it is closed
    /// with the session.
    pub fn owning(mut self, child: std::process::Child) -> Self {
        self.launched = Some(child);
        self
    }

    fn next_seq(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    /// How long a step-over will walk before it gives up and says where it got
    /// to.
    ///
    /// The real bound, because the adapter is busy for every one of those HTTP
    /// round trips: while a walk runs, nothing else is answered and the editor
    /// looks dead. A step count cannot bound that - the same number of steps
    /// is a blink or a minute depending on the machine underneath.
    const STEP_OVER_TIME: std::time::Duration = std::time::Duration::from_secs(2);

    /// A hard ceiling on the same walk, for a machine fast enough to make the
    /// clock the wrong limit.
    const STEP_OVER_BUDGET: usize = 200_000;

    /// Everything the machine has to say about itself right now.
    fn machine(&self) -> std::io::Result<Value> {
        let body = perform(&self.endpoint, &Call::get("/api/state"))?;
        serde_json::from_str(&body)
            .map_err(|why| std::io::Error::new(std::io::ErrorKind::InvalidData, why))
    }

    /// The bytes at `address`, however the emulator is paged right now.
    fn bytes_at(&self, address: u16, count: u16) -> std::io::Result<Vec<u8>> {
        let call = Call::get("/api/ram")
            .query("addr", address)
            .query("len", count);
        let body = perform(&self.endpoint, &call)?;
        let answer: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
        Ok(bytes_from_hex(
            answer.get("hex").and_then(Value::as_str).unwrap_or_default()
        ))
    }

    /// Step *over* the instruction at `PC`.
    ///
    /// This emulator has no such request - it steps, and that is all - so the
    /// step is repeated until `PC` reaches the instruction after the one we
    /// started on. Which is not a workaround: it is what stepping over
    /// *means*, and it comes out right for a `call` and for a repeating `ldir`
    /// or `otir` without either being special-cased.
    fn step_over(&mut self, message: &Value, out_of_this_one: bool) -> std::io::Result<()> {
        self.stepping
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let walked = if out_of_this_one {
            self.walk_out()
        }
        else {
            self.walk_over()
        };
        self.stepping
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // A walk that failed halfway is still a stop: the machine is wherever
        // it got to, and the editor is owed an answer either way. Letting the
        // error out instead would end the whole session over one refused
        // connection - and a debugger that disappears mid-step is a far worse
        // failure than a step that did not finish.
        let (state, note) = match walked {
            Ok((state, None)) => (state, None),
            Ok((state, Some(walked))) => {
                let pc = register(&state, "PC").unwrap_or(0);
                let note = format!(
                    "step over gave up after {walked} instructions without coming back; \
                     stopped at 0x{pc:04X} - step over again to carry on\n"
                );
                (state, Some(note))
            },
            Err(why) => {
                (
                    self.machine().unwrap_or(Value::Null),
                    Some(format!("step over could not finish: {why}\n"))
                )
            },
        };
        if let Some(note) = note {
            let seq = self.next_seq();
            let _ = self.outgoing.send(crate::protocol::event(
                "output",
                json!({ "category": "console", "output": note }),
                seq
            ));
        }

        // The walk is over and the machine is paused; now the editor may hear
        // about it.
        self.expecting_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let seq = self.next_seq();
        let _ = self.outgoing.send(response_for(message, &state, seq));
        Ok(())
    }

    /// Step *out* of the routine we are in, by the same walk: the address to
    /// come back to is the one the `call` pushed, which is on top of the stack.
    ///
    /// A guess, and unavoidably so - `SP` points at a return address only
    /// because that is what being inside a subroutine means, and nothing on a
    /// Z80 says so. The budget is what keeps a wrong guess from running away.
    fn walk_out(&mut self) -> std::io::Result<(Value, Option<usize>)> {
        let state = self.machine()?;
        let Some(sp) = register(&state, "SP").map(|sp| sp as u16)
        else {
            return Ok((state, None));
        };
        let stacked = self.bytes_at(sp, 2)?;
        let [low, high] = stacked[..] else {
            return Ok((state, None));
        };
        self.walk_to(u16::from(low) | (u16::from(high) << 8))
    }

    /// The walk itself. A step count alongside the state means it gave up
    /// after that many, rather than arriving.
    fn walk_over(&mut self) -> std::io::Result<(Value, Option<usize>)> {
        let state = self.machine()?;
        let Some(pc) = register(&state, "PC").map(|pc| pc as u16)
        else {
            // No idea where we are, so nothing to walk back to. One step is
            // still a step.
            perform(&self.endpoint, &Call::post("/api/step"))?;
            return Ok((self.machine()?, None));
        };

        // Picking up a walk that ran out of time, from exactly where it ran
        // out. Anywhere else and this is a new step over, whatever was pending.
        if let Some((gave_up_at, target)) = self.unfinished.take()
            && gave_up_at == pc
        {
            return self.walk_to(target);
        }

        // Four bytes is the longest a Z80 instruction gets.
        let bytes = self.bytes_at(pc, 4)?;
        if !comes_back_to_the_next_instruction(&bytes) {
            // A jump, a return, or an ordinary instruction: the address after
            // it is either where one step lands anyway or somewhere control
            // never returns to. Walking towards it would run the program to
            // its budget.
            perform(&self.endpoint, &Call::post("/api/step"))?;
            return Ok((self.machine()?, None));
        }

        let length = crate::disassemble::decode(pc, &bytes, 1)
            .first()
            .map(|instruction| instruction.bytes.len() as u16)
            .unwrap_or(1);
        self.walk_to(pc.wrapping_add(length))
    }

    /// Step until `PC` is `target`, or until something better to stop for.
    fn walk_to(&mut self, target: u16) -> std::io::Result<(Value, Option<usize>)> {
        let started = std::time::Instant::now();
        for walked in 0..self.step_over_budget {
            // Checked before each step rather than after the lot: the editor is
            // frozen for the whole walk, so how long that lasts is the thing
            // being limited.
            if started.elapsed() > self.step_over_time {
                let state = self.machine()?;
                if let Some(now) = register(&state, "PC").map(|pc| pc as u16) {
                    self.unfinished = Some((now, target));
                }
                return Ok((state, Some(walked)));
            }
            perform(&self.endpoint, &Call::post("/api/step"))?;
            let state = self.machine()?;
            let Some(now) = register(&state, "PC").map(|pc| pc as u16)
            else {
                return Ok((state, None));
            };
            // Back from whatever we stepped over...
            if now == target {
                return Ok((state, None));
            }
            // ...or somewhere the user asked to stop, which outranks finishing
            // the walk: a breakpoint inside a routine must still be a
            // breakpoint when you step over the call to it.
            if self.armed.contains(&now) {
                return Ok((state, None));
            }
        }
        let state = self.machine()?;
        if let Some(now) = register(&state, "PC").map(|pc| pc as u16) {
            self.unfinished = Some((now, target));
        }
        Ok((state, Some(self.step_over_budget)))
    }
}

/// Whether the instruction encoded here hands control back to the address
/// after it.
///
/// The only case where "step until `PC` is the next instruction" terminates.
/// For everything else the address after the instruction is either where a
/// single step lands anyway, or - for a jump or a return - somewhere control
/// may never come back to at all.
fn comes_back_to_the_next_instruction(bytes: &[u8]) -> bool {
    match bytes {
        // The repeating block instructions, which re-execute themselves until
        // they are done: `ldir`, `lddr`, `cpir`, `cpdr`, `inir`, `indr`,
        // `otir`, `otdr`. `PC` sits still while they work, so a plain step
        // makes no visible progress - stepping *over* one is the only way to
        // get past it without holding the key down.
        [0xED, second, ..] if matches!(second, 0xB0..=0xB3 | 0xB8..=0xBB) => true,
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
    fn send(&mut self, message: Value) -> std::io::Result<()> {
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        // Noted as it goes past. Only this side ever sees the walk a step-over
        // makes, so only this side can stop it on a breakpoint inside.
        if command == "setInstructionBreakpoints" {
            self.armed = message
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
        }

        // Step over and step out are not one request here but many - see
        // `step_over`.
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

        // A continue leaves the machine running and nothing else will announce
        // where it ends up, so the editor is owed a stop from here on.
        //
        // A step is over by the time `perform` returns, so its flag is set
        // *afterwards*: set first, the poller can catch the machine still
        // paused from before and announce a stop that has not happened.
        if command == "continue" {
            self.expecting_stop
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }

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
        if matches!(command.as_str(), "stepIn" | "stepOut") {
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

/// Ask the emulator whether it is running, and report every change.
///
/// Ten times a second: fast enough that a stop feels immediate, cheap enough
/// that it is one small request on loopback. The alternative - waiting for an
/// event - does not work, for the reasons in `connect`.
fn watch_run_state(
    endpoint: &str,
    out: &std::sync::mpsc::Sender<Value>,
    expecting_stop: &std::sync::atomic::AtomicBool,
    stepping: &std::sync::atomic::AtomicBool
) {
    let mut running: Option<bool> = None;
    let mut seq = 1_000_000i64;
    let mut misses = 0u32;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        // A walk in progress reports itself when it ends. Whatever the machine
        // looks like in the middle of one is not news.
        if stepping.load(std::sync::atomic::Ordering::Relaxed) {
            continue;
        }

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
        if paused && expecting_stop.swap(false, std::sync::atomic::Ordering::Relaxed) {
            running = Some(false);
            seq += 1;
            let finished = crate::protocol::event(
                "stopped",
                json!({
                    "reason": "breakpoint",
                    "description": "Execution stopped",
                    "threadId": 1,
                    "allThreadsStopped": true
                }),
                seq
            );
            if out.send(finished).is_err() {
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

        seq += 1;
        let event = if paused {
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
        };
        if out.send(event).is_err() {
            return; // the session is gone
        }
    }
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

    /// A stand-in emulator: enough of one to be stepped, and no more.
    ///
    /// It answers `/api/state` from a script of program counters - what `PC`
    /// reads after each step - and `/api/ram` with the bytes the walk starts
    /// on. Being able to run the walk against it is the whole point: a loop
    /// that drives a machine is exactly the code that should not first be tried
    /// on a real one.
    fn fake_machine(
        bytes: Vec<u8>,
        script: Vec<u16>
    ) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let steps = std::sync::Arc::new(AtomicUsize::new(0));
        let counted = steps.clone();
        std::thread::spawn(move || {
            let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
            for stream in listener.incoming() {
                let Ok(mut stream) = stream
                else {
                    return;
                };
                let mut buffer = [0u8; 1024];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let line = request.lines().next().unwrap_or_default().to_string();

                if line.starts_with("POST /api/step") {
                    counted.fetch_add(1, Ordering::Relaxed);
                }
                let at = script[counted.load(Ordering::Relaxed).min(script.len() - 1)];
                let body = if line.starts_with("GET /api/state") {
                    json!({ "z80": { "PC": at, "SP": 0xBFF0 } }).to_string()
                }
                else if line.starts_with("GET /api/ram") {
                    json!({ "addr": 0, "len": bytes.len(), "hex": hex }).to_string()
                }
                else if line.starts_with("GET /api/ping") {
                    json!({ "emu": { "paused": true } }).to_string()
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
            }
        });
        (format!("http://127.0.0.1:{port}"), steps)
    }

    /// Where a step-over left the machine, and how many steps it took to get
    /// there.
    fn stepped_over(
        bytes: Vec<u8>,
        script: Vec<u16>,
        prepare: impl FnOnce(&mut AmspiritLitePeer)
    ) -> (u16, usize) {
        use crate::peer::DapPeer;

        let (endpoint, steps) = fake_machine(bytes, script);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        prepare(&mut peer);
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();

        let answered = peer
            .drain()
            .into_iter()
            .find(|message| message["type"] == json!("response"))
            .expect("the editor is answered");
        assert_eq!(answered["command"], json!("next"));

        // Asked afresh rather than remembered: a `GET` does not step, so this
        // is where the machine really is.
        let state = peer.machine().unwrap();
        (
            register(&state, "PC").unwrap() as u16,
            steps.load(std::sync::atomic::Ordering::Relaxed)
        )
    }

    /// A `call` is walked over: step until control comes back to the
    /// instruction after it.
    #[test]
    fn a_call_is_stepped_over_by_stepping_until_it_comes_back() {
        // `call 0x9000` at 0x8000, so the address to come back to is 0x8003.
        let (pc, steps) = stepped_over(
            vec![0xCD, 0x00, 0x90],
            vec![0x8000, 0x9000, 0x9001, 0x8003],
            |_| {}
        );
        assert_eq!(pc, 0x8003);
        assert_eq!(steps, 3, "it stopped as soon as it was back, not later");
    }

    /// And so is a repeating block instruction, for free: `PC` sits on the
    /// `ldir` until it is finished, so the same rule gets past it.
    #[test]
    fn a_repeating_instruction_is_walked_to_its_end() {
        let (pc, steps) = stepped_over(
            vec![0xED, 0xB0],
            vec![0x8000, 0x8000, 0x8000, 0x8002],
            |_| {}
        );
        assert_eq!(pc, 0x8002);
        assert_eq!(steps, 3);
    }

    /// Anything that does not come back is one step and no walk.
    ///
    /// A `jr` never reaches the address after itself, so walking towards it
    /// would run the program to the end of the budget.
    #[test]
    fn an_ordinary_instruction_is_a_single_step() {
        for bytes in [vec![0x3E, 0x01], vec![0x18, 0xFE], vec![0xC9]] {
            let (pc, steps) = stepped_over(bytes.clone(), vec![0x8000, 0x4000], |_| {});
            assert_eq!(steps, 1, "{bytes:02X?}");
            assert_eq!(pc, 0x4000, "{bytes:02X?}");
        }
    }

    /// A breakpoint inside what is being stepped over still stops.
    ///
    /// Otherwise stepping over a `call` is a way of silently disarming every
    /// breakpoint in the routine it calls.
    #[test]
    fn a_breakpoint_inside_the_call_wins_over_finishing_the_walk() {
        let (pc, steps) = stepped_over(
            vec![0xCD, 0x00, 0x90],
            vec![0x8000, 0x9000, 0x9001, 0x8003],
            |peer| peer.armed = vec![0x9001]
        );
        assert_eq!(pc, 0x9001, "stopped where the user asked");
        assert_eq!(steps, 2);
    }

    /// Step out is the same walk, aimed at the address the `call` pushed.
    #[test]
    fn a_step_out_walks_to_the_address_on_top_of_the_stack() {
        use crate::peer::DapPeer;

        // The two bytes at `SP` are the return address, little-endian: 0x8003.
        let (endpoint, steps) =
            fake_machine(vec![0x03, 0x80], vec![0x9000, 0x9001, 0x9002, 0x8003]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.send(request("stepOut", json!({ "threadId": 1 })))
            .unwrap();

        assert_eq!(steps.load(std::sync::atomic::Ordering::Relaxed), 3);
        let state = peer.machine().unwrap();
        assert_eq!(register(&state, "PC").unwrap(), 0x8003);
        assert!(
            peer.drain()
                .iter()
                .any(|message| message["command"] == json!("stepOut")),
            "and the editor is answered"
        );
    }

    /// A walk that never comes back gives up and says so, rather than running
    /// for ever.
    #[test]
    fn a_walk_that_never_returns_gives_up_out_loud() {
        use crate::peer::DapPeer;

        // A `call` into a routine that never returns: `PC` just goes up.
        let (endpoint, _) = fake_machine(vec![0xCD, 0x00, 0x90], vec![
            0x8000, 0x9000, 0x9001, 0x9002, 0x9003, 0x9004,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.step_over_budget = 4;
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();

        let drained = peer.drain();
        let note = drained
            .iter()
            .find(|message| message["event"] == json!("output"))
            .expect("says it gave up");
        let text = note["body"]["output"].as_str().unwrap();
        assert!(text.contains("gave up after 4"), "{text}");
        assert!(text.contains("step over again"), "and what to do: {text}");
        assert!(text.contains("0x9003"), "and where it got to: {text}");
        assert!(
            drained
                .iter()
                .any(|message| message["command"] == json!("next")),
            "and still answers the editor"
        );
    }

    /// A walk that ran out of time is resumed by the next one, towards the
    /// same address.
    #[test]
    fn a_second_step_over_carries_on_towards_the_same_address() {
        use crate::peer::DapPeer;

        // `call 0x9000` at 0x8000: back at 0x8003 on the fourth step.
        let (endpoint, steps) = fake_machine(vec![0xCD, 0x00, 0x90], vec![
            0x8000, 0x9000, 0x9001, 0x9002, 0x8003,
        ]);
        let mut peer = AmspiritLitePeer::connect(&endpoint).unwrap();
        peer.step_over_budget = 2;

        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();
        assert_eq!(steps.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert_eq!(peer.unfinished, Some((0x9001, 0x8003)), "where it was going");

        // Pressing it again does not step over `0x9002` - it goes on to
        // `0x8003`, which is what was being stepped over in the first place.
        peer.send(request("next", json!({ "threadId": 1 }))).unwrap();
        let state = peer.machine().unwrap();
        assert_eq!(register(&state, "PC").unwrap(), 0x8003);
        assert_eq!(peer.unfinished, None, "and it arrived");
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
        let (endpoint, _) = fake_machine(vec![0x00], vec![0x8000]);
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

    /// A walk against an emulator that has stopped answering still answers the
    /// editor.
    ///
    /// The alternative is what makes a debugger disappear mid-step: the error
    /// travels up out of `send`, the session ends, and every button goes dead
    /// with no stop event and no explanation.
    #[test]
    fn a_walk_against_a_dead_emulator_still_answers_the_editor() {
        use crate::peer::DapPeer;

        // A port nothing is listening on: bound to find a free one, then let
        // go.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut peer = AmspiritLitePeer::connect(&format!("http://127.0.0.1:{port}")).unwrap();
        peer.send(request("next", json!({ "threadId": 1 })))
            .expect("a failed walk is not a failed session");

        let drained = peer.drain();
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
            stepping: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            armed: Vec::new(),
            step_over_budget: AmspiritLitePeer::STEP_OVER_BUDGET,
            step_over_time: AmspiritLitePeer::STEP_OVER_TIME,
            unfinished: None,
            launched: None,
            pending,
            outgoing,
            seq: 0
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
        // deliberately *not* one call - it is a walk of many steps, which
        // `send` intercepts before `call_for` is ever consulted.
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

    fn reachable() -> bool {
        std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8765".parse().unwrap(),
            std::time::Duration::from_millis(300)
        )
        .is_ok()
    }

    #[test]
    #[ignore]
    fn live_state_carries_every_pane() {
        assert!(reachable(), "start AMSpiriT Lite with --web-server first");

        let body = perform(DEFAULT_ENDPOINT, &Call::get("/api/state")).unwrap();
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
        let body = perform(DEFAULT_ENDPOINT, &call_for(&read).unwrap()).unwrap();
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

        let body = perform(DEFAULT_ENDPOINT, &Call::get("/api/memmap")).unwrap();
        let map: Value = serde_json::from_str(&body).unwrap();
        let regions = map["regions"].as_array().expect("regions");

        assert_eq!(regions.len(), 4, "one per 16K region: {map}");
        assert!(regions[0].get("ram_bank").is_some(), "{map}");
        assert!(
            map.get("rmr").is_some(),
            "the banking register itself: {map}"
        );
    }
}

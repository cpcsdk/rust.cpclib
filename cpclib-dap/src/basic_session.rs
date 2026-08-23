//! The BASIC-flavoured translating session.
//!
//! Deliberately not a mode of [`crate::session::Session`]: that type is
//! shaped around Z80 addresses, registers and a call stack reconstructed
//! from memory - none of which describe "where a BASIC program is" the way
//! a line number does. This session is built on the same [`DapPeer`]
//! primitives (`readMemory`, `setInstructionBreakpoints`, `continue`) that
//! `Session` uses, interpreted through [`crate::basic`] instead - so it
//! works identically against 1984js and AMSpiriT Lite, with nothing
//! backend-specific here.
//!
//! The whole feature rests on one fact: the ROM calls
//! [`crate::basic::EXECUTE_LINE_ENTRY`] once per BASIC line, with `HL`
//! pointing at it. One instruction breakpoint at
//! [`crate::basic::LINE_BREAKPOINT_TARGET`] (a few bytes into that same
//! routine - see its own doc comment for why not the entry point itself),
//! left armed for the whole session, plus reading
//! [`crate::basic::PTR_CURRENT_LINE_NUMBER_FIELD`] on every hit to compare
//! against the user's actual breakpoints, is the entire stepping/breakpoint
//! mechanism - no per-breakpoint address computation, unlike a Z80 session
//! or the reference `amspirit-basic` extension's own address-mapped
//! approach.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::basic::{
    self, BasicVariableValue, LINE_BREAKPOINT_TARGET, PTR_CURRENT_LINE_NUMBER_FIELD,
    VARIABLE_CHAIN_HEADS, VARIABLE_CHAIN_HEADS_COUNT
};
use crate::peer::DapPeer;
use crate::protocol::{self, address_reference};
use crate::session::decode_base64;

/// How many bytes of the variable storage area to read in one go - matches
/// the reference `amspirit-basic` extension's own cap for the same bulk
/// read, a reasonable safety limit rather than a measured one.
const MAX_VARIABLE_BYTES: u16 = 8192;

/// The one synthetic scope this session ever offers.
const VARIABLES_REFERENCE: i64 = 1000;

const THREAD_ID: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Plain,
    /// The peer's own `attach` handshake, for a peer that needs one.
    Attach,
    /// The very first `continue`, sent right after `configurationDone`. Its
    /// answer is the earliest point the machine is confirmed genuinely
    /// running (AMSpiritLite's own `continue` handling blocks on exactly
    /// that) - the right moment to auto-type `RUN`, on a peer that can.
    LaunchResumed,
    /// Reading [`basic::PTR_CURRENT_LINE_NUMBER_FIELD`] itself - its value
    /// is a pointer to dereference next, or the direct-mode sentinel.
    CurrentLinePointer,
    /// Dereferencing that pointer to get the actual line number.
    CurrentLineValue,
    /// The 27 chain heads, on the way to decoding variables.
    VariableChainHeads,
    /// The bulk variable-storage read, once the chain heads are known.
    VariableStorage
}

#[derive(Debug, Clone)]
struct OwnRequest {
    purpose: Purpose
}

/// Why the program is being resumed - decides whether the next line
/// boundary is reported unconditionally (a step) or only when it matches a
/// user breakpoint (a plain continue, including the very first run after
/// `configurationDone`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResumeKind {
    Step,
    Continue
}

/// Variable-storage reads gathered on the way to answering one `variables`
/// request - both must be in before anything can be decoded.
#[derive(Debug, Clone, Default)]
struct PendingVariables {
    request: Option<Value>,
    chain_heads: Option<Vec<u8>>,
    storage: Option<Vec<u8>>,
    variables_base: u16
}

pub struct BasicSession<P: DapPeer> {
    peer: P,
    source_path: PathBuf,
    /// BASIC line number -> 0-based index into the source file's own
    /// lines, computed once at launch by parsing the source text directly
    /// - exact, unlike the reference extension's own regex-based
    /// line-number-prefix heuristic.
    line_index: Vec<(u16, usize)>,
    /// Where the tokenised program starts in RAM. Known outright, not read
    /// back from the emulator: the launch flow chose this address itself
    /// when it built the boot snapshot (see `lib.rs`), so there is nothing
    /// to ask.
    program_start: u16,
    /// The tokenised program's own byte length, known locally since this
    /// session built the bytes itself - `variables_base` is just
    /// `program_start + program_len`, needing no round trip to recompute.
    program_len: u16,
    /// BASIC line numbers with a user breakpoint.
    breakpoints: Vec<u16>,
    seq: i64,
    own_requests: HashMap<i64, OwnRequest>,
    own_seq: i64,
    attached: bool,
    configured: bool,
    started: bool,
    /// Set while resuming, until the next line boundary is reported or
    /// filtered past.
    resuming_as: Option<ResumeKind>,
    /// The line the program is stopped at, once known.
    current_line: Option<u16>,
    pending_variables: Option<PendingVariables>
}

impl<P: DapPeer> BasicSession<P> {
    pub fn new(
        peer: P,
        source_path: PathBuf,
        source_text: &str,
        program_start: u16,
        program_bytes_len: usize
    ) -> Self {
        let line_index = line_index_from_source(source_text);
        Self {
            peer,
            source_path,
            line_index,
            program_start,
            program_len: program_bytes_len as u16,
            breakpoints: Vec::new(),
            seq: 1,
            own_requests: HashMap::new(),
            own_seq: 100_000,
            attached: false,
            configured: false,
            started: false,
            resuming_as: None,
            current_line: None,
            pending_variables: None
        }
    }

    pub fn peer_mut(&mut self) -> &mut P {
        &mut self.peer
    }

    /// Sends the peer's own `attach` handshake. Called once, right after
    /// construction, regardless of whether this particular peer actually
    /// needs it - one that does not just answers immediately.
    pub fn attach(&mut self) -> std::io::Result<()> {
        self.send_own("attach", json!({}), Purpose::Attach)
    }

    fn variables_base(&self) -> u16 {
        self.program_start.wrapping_add(self.program_len)
    }

    fn next_seq(&mut self) -> i64 {
        let seq = self.seq;
        self.seq += 1;
        seq
    }

    fn send_own(&mut self, command: &str, arguments: Value, purpose: Purpose) -> std::io::Result<()> {
        let seq = self.own_seq;
        self.own_seq += 1;
        self.own_requests.insert(seq, OwnRequest { purpose });
        self.peer.send(protocol::request(command, arguments, seq))
    }

    fn is_our_answer(&mut self, response: &Value) -> Option<OwnRequest> {
        let request_seq = response.get("request_seq").and_then(Value::as_i64)?;
        self.own_requests.remove(&request_seq)
    }

    fn read_memory_bytes(response: &Value) -> Vec<u8> {
        response
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default()
    }

    /// Arms the one breakpoint this whole session ever needs, and starts
    /// the program if the editor has already finished configuring.
    fn on_attached(&mut self) -> std::io::Result<()> {
        self.attached = true;
        self.send_own(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [{ "instructionReference": address_reference(LINE_BREAKPOINT_TARGET as u32) }] }),
            Purpose::Plain
        )?;
        self.start_if_ready()
    }

    fn start_if_ready(&mut self) -> std::io::Result<()> {
        if self.attached && self.configured && !self.started {
            self.started = true;
            self.resuming_as = Some(ResumeKind::Continue);
            self.send_own("continue", json!({ "threadId": THREAD_ID }), Purpose::LaunchResumed)?;
        }
        Ok(())
    }

    /// Types `RUN` into the emulator, on a peer that can - the emulator's
    /// own keyboard, not a firmware trick: AMSpiritLite exposes exactly this
    /// as `POST /api/keytype` (confirmed against its bundled documentation
    /// and its own web UI, which uses the identical call to auto-run an
    /// injected BASIC program), and 1984js's bridge script drives the same
    /// keyboard-matrix simulation its own UI's keydown handler uses. Neither
    /// is a ROM-internals trick like jumping `PC` into `RUN_from_HL` or
    /// forging its caller's stack - both were investigated and rejected
    /// earlier for exactly that reason.
    ///
    /// Silently does nothing on a peer that answers neither: the launch
    /// flow's own notice (`lib.rs`) covers that case by telling the user to
    /// type it themselves, checked against this same `supports()` call so
    /// the two do not disagree.
    fn autotype_run(&mut self) -> std::io::Result<()> {
        if !self.peer.supports("cpclib/autotype") {
            return Ok(());
        }
        self.send_own("cpclib/autotype", json!({ "text": "RUN\n" }), Purpose::Plain)
    }

    pub fn on_editor_message(&mut self, message: &Value) -> std::io::Result<Vec<Value>> {
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();

        match command {
            "setBreakpoints" => Ok(self.set_breakpoints(message)),
            "configurationDone" => {
                self.configured = true;
                self.start_if_ready()?;
                let seq = self.next_seq();
                Ok(vec![protocol::response(message, json!({}), seq)])
            },
            "continue" | "next" | "stepIn" | "stepOut" => {
                // BASIC has no instruction-level step: "next"/"stepIn"/
                // "stepOut" all mean "run to the next line boundary and
                // stop there unconditionally", which at the Z80 level this
                // session already gets by resuming past the one armed
                // breakpoint and reporting the first hit regardless of the
                // user's own breakpoints.
                self.resuming_as = Some(if command == "continue" {
                    ResumeKind::Continue
                }
                else {
                    ResumeKind::Step
                });
                self.send_own("continue", json!({ "threadId": THREAD_ID }), Purpose::Plain)?;
                let seq = self.next_seq();
                Ok(vec![protocol::response(message, json!({}), seq)])
            },
            "pause" => {
                self.peer.send(message.clone())?;
                Ok(Vec::new())
            },
            "stackTrace" => Ok(vec![self.stack_trace(message)]),
            "scopes" => Ok(vec![self.scopes(message)]),
            "variables"
                if message
                    .get("arguments")
                    .and_then(|a| a.get("variablesReference"))
                    .and_then(Value::as_i64)
                    == Some(VARIABLES_REFERENCE) =>
            {
                self.begin_variables(message)?;
                Ok(Vec::new())
            },
            "threads" => {
                let seq = self.next_seq();
                Ok(vec![protocol::response(
                    message,
                    json!({ "threads": [{ "id": THREAD_ID, "name": "BASIC" }] }),
                    seq
                )])
            },
            "disconnect" | "terminate" => {
                let seq = self.next_seq();
                Ok(vec![protocol::response(message, json!({}), seq)])
            },
            _ => {
                if self.peer.quirks().rejects_unknown_requests && !self.peer.supports(command) {
                    let seq = self.next_seq();
                    return Ok(vec![protocol::failure(
                        message,
                        &format!("the emulator being debugged does not implement '{command}'."),
                        seq
                    )]);
                }
                self.peer.send(message.clone())?;
                Ok(Vec::new())
            }
        }
    }

    fn set_breakpoints(&mut self, message: &Value) -> Vec<Value> {
        let requested = message
            .get("arguments")
            .and_then(|a| a.get("breakpoints"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut verified = Vec::new();
        let mut lines = Vec::new();
        for bp in &requested {
            let Some(source_line) = bp.get("line").and_then(Value::as_i64) else {
                verified.push(json!({ "verified": false }));
                continue;
            };
            // The editor's lines are 1-based; `line_index` keys by 0-based
            // source-line index.
            let idx = (source_line - 1).max(0) as usize;
            match self.line_index.iter().find(|(_, i)| *i == idx) {
                Some((basic_line, _)) => {
                    lines.push(*basic_line);
                    verified.push(json!({ "verified": true, "line": source_line }));
                },
                None => {
                    verified.push(json!({
                        "verified": false,
                        "message": "no BASIC line starts here"
                    }));
                }
            }
        }
        self.breakpoints = lines;

        let seq = self.next_seq();
        vec![protocol::response(message, json!({ "breakpoints": verified }), seq)]
    }

    fn stack_trace(&mut self, message: &Value) -> Value {
        let seq = self.next_seq();
        let line = self.current_line.unwrap_or(0);
        let source_line = self
            .line_index
            .iter()
            .find(|(n, _)| *n == line)
            .map(|(_, idx)| *idx as i64 + 1)
            .unwrap_or(1);

        protocol::response(
            message,
            json!({
                "stackFrames": [{
                    "id": 0,
                    "name": format!("BASIC {line}"),
                    "line": source_line,
                    "column": 1,
                    "source": {
                        "path": self.source_path.display().to_string()
                    }
                }],
                "totalFrames": 1
            }),
            seq
        )
    }

    fn scopes(&mut self, message: &Value) -> Value {
        let seq = self.next_seq();
        protocol::response(
            message,
            json!({
                "scopes": [{
                    "name": "Variables",
                    "variablesReference": VARIABLES_REFERENCE,
                    "expensive": false
                }]
            }),
            seq
        )
    }

    fn begin_variables(&mut self, message: &Value) -> std::io::Result<()> {
        self.pending_variables = Some(PendingVariables {
            request: Some(message.clone()),
            chain_heads: None,
            storage: None,
            variables_base: self.variables_base()
        });
        self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(VARIABLE_CHAIN_HEADS as u32),
                "count": VARIABLE_CHAIN_HEADS_COUNT * 2 + 2 // + the DEF FN head
            }),
            Purpose::VariableChainHeads
        )?;
        let base = self.variables_base();
        self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(base as u32),
                "count": MAX_VARIABLE_BYTES
            }),
            Purpose::VariableStorage
        )
    }

    fn complete_variables(&mut self) -> Option<Vec<Value>> {
        let pending = self.pending_variables.as_ref()?;
        let (chain_heads, storage) = (pending.chain_heads.as_ref()?, pending.storage.as_ref()?);

        let mut heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        for (i, h) in heads.iter_mut().enumerate() {
            *h = u16::from_le_bytes([chain_heads[i * 2], chain_heads[i * 2 + 1]]);
        }
        let def_fn_offset = VARIABLE_CHAIN_HEADS_COUNT * 2;
        let def_fn_head = u16::from_le_bytes([
            chain_heads[def_fn_offset],
            chain_heads[def_fn_offset + 1]
        ]);

        let vars = basic::decode_variable_chains(&heads, def_fn_head, pending.variables_base, storage);

        let request = self.pending_variables.take()?.request?;
        let seq = self.next_seq();
        let entries: Vec<Value> = vars
            .iter()
            .map(|v| {
                json!({
                    "name": v.name,
                    "value": format_variable_value(&v.value),
                    "variablesReference": 0
                })
            })
            .collect();

        Some(vec![protocol::response(
            &request,
            json!({ "variables": entries }),
            seq
        )])
    }

    pub fn on_emulator_message(&mut self, message: &Value) -> Vec<Value> {
        if message.get("type").and_then(Value::as_str) == Some("response")
            && let Some(own) = self.is_our_answer(message)
        {
            match own.purpose {
                Purpose::Plain => {},
                Purpose::Attach => {
                    if let Err(problem) = self.on_attached() {
                        return vec![protocol::event(
                            "output",
                            json!({ "category": "stderr", "output": format!("{problem}\n") }),
                            1
                        )];
                    }
                },
                Purpose::LaunchResumed => {
                    if let Err(problem) = self.autotype_run() {
                        return vec![protocol::event(
                            "output",
                            json!({ "category": "stderr", "output": format!("{problem}\n") }),
                            1
                        )];
                    }
                },
                Purpose::CurrentLinePointer => return self.on_line_pointer_read(message),
                Purpose::CurrentLineValue => return self.on_line_value_read(message),
                Purpose::VariableChainHeads => {
                    if let Some(pending) = self.pending_variables.as_mut() {
                        pending.chain_heads = Some(Self::read_memory_bytes(message));
                    }
                    return self.complete_variables().unwrap_or_default();
                },
                Purpose::VariableStorage => {
                    if let Some(pending) = self.pending_variables.as_mut() {
                        pending.storage = Some(Self::read_memory_bytes(message));
                    }
                    return self.complete_variables().unwrap_or_default();
                }
            }
            return Vec::new();
        }

        let kind = message.get("type").and_then(Value::as_str).unwrap_or_default();
        if kind == "event" {
            let event = message.get("event").and_then(Value::as_str).unwrap_or_default();
            if event == "stopped" {
                return self.on_z80_stopped();
            }
            if event == "initialized" {
                return Vec::new();
            }
        }
        Vec::new()
    }

    /// The one Z80 breakpoint fired. Not necessarily a line the user
    /// actually wants to stop at - that is only known once the current
    /// line number has been read and compared.
    fn on_z80_stopped(&mut self) -> Vec<Value> {
        if self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(PTR_CURRENT_LINE_NUMBER_FIELD as u32),
                "count": 2
            }),
            Purpose::CurrentLinePointer
        )
        .is_err()
        {
            return Vec::new();
        }
        Vec::new()
    }

    fn on_line_pointer_read(&mut self, message: &Value) -> Vec<Value> {
        let bytes = Self::read_memory_bytes(message);
        let Some(ptr_bytes) = bytes.get(0..2) else {
            return Vec::new();
        };
        match basic::current_line_number_field_address([ptr_bytes[0], ptr_bytes[1]]) {
            Some(address) => {
                let _ = self.send_own(
                    "readMemory",
                    json!({ "memoryReference": address_reference(address as u32), "count": 2 }),
                    Purpose::CurrentLineValue
                );
                Vec::new()
            },
            // Direct/immediate mode: the program ended (or was never
            // started). Report it stopped where it last was rather than
            // silently doing nothing.
            None => self.report_stopped("entry")
        }
    }

    fn on_line_value_read(&mut self, message: &Value) -> Vec<Value> {
        let bytes = Self::read_memory_bytes(message);
        let Some(line_bytes) = bytes.get(0..2) else {
            return Vec::new();
        };
        let line = basic::decode_line_number([line_bytes[0], line_bytes[1]]);

        let should_stop = match self.resuming_as {
            Some(ResumeKind::Step) => true,
            Some(ResumeKind::Continue) | None => self.breakpoints.contains(&line)
        };

        self.current_line = Some(line);

        if should_stop {
            self.resuming_as = None;
            self.report_stopped(if self.breakpoints.contains(&line) {
                "breakpoint"
            }
            else {
                "step"
            })
        }
        else {
            // Not a line the user cares about: keep going.
            let _ = self.send_own("continue", json!({ "threadId": THREAD_ID }), Purpose::Plain);
            Vec::new()
        }
    }

    fn report_stopped(&mut self, reason: &str) -> Vec<Value> {
        let seq = self.next_seq();
        vec![protocol::event(
            "stopped",
            json!({
                "reason": reason,
                "threadId": THREAD_ID,
                "allThreadsStopped": true
            }),
            seq
        )]
    }
}

fn format_variable_value(value: &BasicVariableValue) -> String {
    match value {
        BasicVariableValue::Integer(i) => i.to_string(),
        BasicVariableValue::Real(f) => f.to_string(),
        BasicVariableValue::StringRef { len, address } => {
            format!("<string, {len} bytes at {}>", address_reference(*address as u32))
        },
        BasicVariableValue::DefFn => "<DEF FN>".to_string(),
        BasicVariableValue::Unknown(code) => format!("<unknown type {code:#04x}>")
    }
}

/// Parses `source` with `cpclib_basic` and pairs each BASIC line number
/// with the 0-based index of the source line it was written on - exact,
/// since it comes from the same tokenizer the launch flow already used to
/// build the program, not a text heuristic re-deriving line numbers from
/// scratch.
fn line_index_from_source(source: &str) -> Vec<(u16, usize)> {
    let mut index = Vec::new();
    for (line_idx, text) in source.lines().enumerate() {
        let trimmed = text.trim_start();
        let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(number) = digits.parse::<u16>() {
            if !digits.is_empty() {
                index.push((number, line_idx));
            }
        }
    }
    index
}

#[cfg(test)]
mod tests {
    use crate::peer::RecordingPeer;

    use super::*;

    #[test]
    fn line_index_pairs_basic_line_numbers_with_source_line_indices() {
        let source = "10 PRINT \"HI\"\n20 GOTO 10\n";
        let index = line_index_from_source(source);
        assert_eq!(index, vec![(10, 0), (20, 1)]);
    }

    #[test]
    fn line_index_ignores_lines_with_no_leading_number() {
        let source = "10 PRINT \"HI\"\n' a comment maybe\n20 GOTO 10\n";
        let index = line_index_from_source(source);
        assert_eq!(index, vec![(10, 0), (20, 2)]);
    }

    const SOURCE: &str = "10 PRINT \"HI\"\n20 GOTO 10\n";

    fn encode_base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = *chunk.get(1).unwrap_or(&0) as u32;
            let b2 = *chunk.get(2).unwrap_or(&0) as u32;
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[((n >> 6) & 0x3F) as usize] as char
            }
            else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[(n & 0x3F) as usize] as char
            }
            else {
                '='
            });
        }
        out
    }

    fn new_session(source: &str) -> BasicSession<RecordingPeer> {
        let bytes = cpclib_basic::BasicProgram::parse(source)
            .unwrap()
            .as_bytes();
        BasicSession::new(
            RecordingPeer::new(),
            PathBuf::from("test.bas"),
            source,
            basic::PROGRAM_START,
            bytes.len()
        )
    }

    /// Sends `attach` and simulates the peer answering it successfully,
    /// processing whatever that answer triggers (arming the breakpoint).
    fn complete_attach(session: &mut BasicSession<RecordingPeer>) {
        session.attach().unwrap();
        let seq = last_sent_seq(session);
        answer(session, seq, json!({ "success": true }));
    }

    fn last_sent_seq(session: &mut BasicSession<RecordingPeer>) -> i64 {
        session
            .peer_mut()
            .sent
            .last()
            .unwrap()
            .get("seq")
            .and_then(Value::as_i64)
            .unwrap()
    }

    /// Queues a response to request `seq` and processes it.
    fn answer(session: &mut BasicSession<RecordingPeer>, seq: i64, extra: Value) -> Vec<Value> {
        let mut response = json!({ "type": "response", "request_seq": seq, "success": true });
        for (k, v) in extra.as_object().unwrap() {
            response[k] = v.clone();
        }
        session.peer_mut().push_incoming(response);
        let incoming = session.peer_mut().drain();
        let mut out = Vec::new();
        for message in incoming {
            out.extend(session.on_emulator_message(&message));
        }
        out
    }

    fn read_memory_response(bytes: &[u8]) -> Value {
        json!({ "body": { "data": encode_base64(bytes) } })
    }

    #[test]
    fn attach_arms_the_line_breakpoint_target_not_the_entry_point() {
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);

        // Regression test: this used to arm EXECUTE_LINE_ENTRY itself, which
        // reads PTR_CURRENT_LINE_NUMBER_FIELD one line too early (the ROM
        // updates it a few bytes later in the same routine - see
        // LINE_BREAKPOINT_TARGET's doc comment) - so every breakpoint
        // comparison was off by one line and a real breakpoint could go the
        // whole session without ever matching.
        let armed = session.peer_mut().last("setInstructionBreakpoints").unwrap();
        assert_eq!(
            armed["arguments"]["breakpoints"][0]["instructionReference"],
            address_reference(LINE_BREAKPOINT_TARGET as u32)
        );
    }

    #[test]
    fn configuration_done_starts_the_program_once_attached() {
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);

        let before = session.peer_mut().commands().len();
        session
            .on_editor_message(&json!({ "seq": 1, "command": "configurationDone", "arguments": {} }))
            .unwrap();

        let commands = session.peer_mut().commands();
        assert!(commands.len() > before);
        assert_eq!(commands.last().unwrap(), "continue");
    }

    #[test]
    fn the_first_resume_types_run_on_a_peer_that_supports_autotype() {
        let bytes = cpclib_basic::BasicProgram::parse(SOURCE).unwrap().as_bytes();
        let mut session = BasicSession::new(
            RecordingPeer::new().also_supporting(&["cpclib/autotype"]),
            PathBuf::from("test.bas"),
            SOURCE,
            basic::PROGRAM_START,
            bytes.len()
        );
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({ "seq": 1, "command": "configurationDone", "arguments": {} }))
            .unwrap();

        // The peer answering the very first `continue` is what triggers the
        // autotype - matching AMSpiritLite's own `continue` handling, which
        // does not answer until the machine is genuinely running.
        let continue_seq = last_sent_seq(&mut session);
        answer(&mut session, continue_seq, json!({ "success": true }));

        let last = session.peer_mut().sent.last().unwrap();
        assert_eq!(last["command"], "cpclib/autotype");
        assert_eq!(last["arguments"]["text"], "RUN\n");
    }

    #[test]
    fn the_first_resume_types_nothing_on_a_peer_without_autotype() {
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({ "seq": 1, "command": "configurationDone", "arguments": {} }))
            .unwrap();

        let continue_seq = last_sent_seq(&mut session);
        let before = session.peer_mut().commands().len();
        answer(&mut session, continue_seq, json!({ "success": true }));

        // Plain RecordingPeer claims no cpclib/autotype support - nothing
        // extra should have been sent for it to reject or misinterpret.
        assert_eq!(session.peer_mut().commands().len(), before);
    }

    #[test]
    fn set_breakpoints_verifies_a_line_that_starts_a_basic_line() {
        let mut session = new_session(SOURCE);
        let response = session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // "20 GOTO 10"
            }))
            .unwrap();

        assert_eq!(response.len(), 1);
        assert_eq!(response[0]["body"]["breakpoints"][0]["verified"], true);
    }

    #[test]
    fn set_breakpoints_rejects_a_line_with_no_basic_line_number() {
        // A trailing blank line (common in real files) starts no BASIC
        // line of its own - built directly rather than through
        // `new_session`, since the parser itself does not tolerate one
        // (irrelevant here: `line_index_from_source` is a separate, plain
        // text scan, and only it is under test).
        let program_bytes = cpclib_basic::BasicProgram::parse(SOURCE).unwrap().as_bytes();
        let mut session = BasicSession::new(
            RecordingPeer::new(),
            PathBuf::from("test.bas"),
            "10 PRINT \"HI\"\n20 GOTO 10\n\n",
            basic::PROGRAM_START,
            program_bytes.len()
        );
        let response = session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 3 }] } // the blank line
            }))
            .unwrap();

        assert_eq!(response[0]["body"]["breakpoints"][0]["verified"], false);
    }

    /// Drives one full breakpoint stop-or-continue cycle: `attach`, arm the
    /// single Z80 breakpoint, then a `stopped` event from the peer, feeding
    /// in `current_line`'s two bytes as the current BASIC line.
    fn stop_at_line(session: &mut BasicSession<RecordingPeer>, current_line: u16) -> Vec<Value> {
        complete_attach(session);
        session.peer_mut().push_incoming(json!({
            "type": "event",
            "event": "stopped",
            "body": {}
        }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }

        // First round trip: PTR_CURRENT_LINE_NUMBER_FIELD -> a pointer.
        let seq = last_sent_seq(session);
        let field_target = 0x9000u16;
        answer(
            session,
            seq,
            read_memory_response(&field_target.to_le_bytes())
        );

        // Second round trip: dereferencing that pointer -> the line number.
        let seq = last_sent_seq(session);
        answer(session, seq, read_memory_response(&current_line.to_le_bytes()))
    }

    #[test]
    fn a_stop_on_a_breakpoint_line_is_reported() {
        let mut session = new_session(SOURCE);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();

        let events = stop_at_line(&mut session, 20);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "breakpoint");
    }

    #[test]
    fn a_stop_on_a_non_breakpoint_line_resumes_silently() {
        let mut session = new_session(SOURCE);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20 only
            }))
            .unwrap();

        let commands_before = session.peer_mut().commands().len();
        let events = stop_at_line(&mut session, 10); // not a breakpoint

        assert!(events.is_empty(), "no stopped event should reach the editor");
        let commands = session.peer_mut().commands();
        assert!(commands.len() > commands_before);
        assert_eq!(commands.last().unwrap(), "continue");
    }

    #[test]
    fn next_stops_at_the_next_line_even_with_no_breakpoint_there() {
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({ "seq": 1, "command": "next", "arguments": {} }))
            .unwrap();

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        let field_target = 0x9000u16;
        answer(
            &mut session,
            seq,
            read_memory_response(&field_target.to_le_bytes())
        );
        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&10u16.to_le_bytes()));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "step");
    }

    #[test]
    fn direct_mode_after_a_stop_is_reported_as_entry() {
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);
        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }

        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&0u16.to_le_bytes()));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "entry");
    }

    #[test]
    fn stack_trace_reports_the_current_line_and_its_source_line() {
        let mut session = new_session(SOURCE);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] }
            }))
            .unwrap();
        stop_at_line(&mut session, 20);

        let response = session
            .on_editor_message(&json!({ "seq": 2, "command": "stackTrace", "arguments": {} }))
            .unwrap();

        assert_eq!(response.len(), 1);
        let frame = &response[0]["body"]["stackFrames"][0];
        assert_eq!(frame["name"], "BASIC 20");
        assert_eq!(frame["line"], 2);
    }

    #[test]
    fn variables_request_decodes_a_chain_walk_from_two_reads() {
        let mut session = new_session(SOURCE);

        let response = session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "variables",
                "arguments": { "variablesReference": VARIABLES_REFERENCE }
            }))
            .unwrap();
        assert!(response.is_empty(), "waits on two reads before answering");

        // 27 chain heads: 'I' (9th letter) points at offset 1.
        let mut heads = vec![0u8; VARIABLE_CHAIN_HEADS_COUNT * 2 + 2];
        heads[8 * 2] = 1;
        heads[8 * 2 + 1] = 0;
        let seq = session.own_requests.keys().min().copied().unwrap();
        answer(&mut session, seq, read_memory_response(&heads));

        // One node: next=0, name "I" (bit 7 set on its one char), type
        // 0x01 (integer), value 42.
        let mut storage = vec![0u8, 0u8, b'I' | 0x80, 0x01, 42, 0];
        storage.resize(64, 0);
        let seq = session.own_requests.keys().min().copied().unwrap();
        let events = answer(&mut session, seq, read_memory_response(&storage));

        assert_eq!(events.len(), 1);
        let vars = &events[0]["body"]["variables"];
        assert_eq!(vars[0]["name"], "I");
        assert_eq!(vars[0]["value"], "42");
    }
}

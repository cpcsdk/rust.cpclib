//! The emulator, seen as something that speaks DAP.
//!
//! 1984js already does, so its implementation is a transport. A native
//! emulator that only offers monitor primitives would be wrapped into the same
//! shape rather than special-cased upstream, which is why the translation layer
//! is written against this trait and never against a particular emulator.

use std::collections::HashMap;

use serde_json::Value;

use crate::protocol;

/// Where a session's own requests are numbered from.
///
/// The editor and a session each number their requests from 1, and both
/// conversations run over the same connection. A response carries only the
/// seq it answers, so an emulator reply to *our* `attach` (request_seq 2) is
/// indistinguishable from a reply to the *editor's* request 2 - which is its
/// `launch`. Forwarding one to the other tells the editor its launch was
/// answered by an attach, and the session falls apart.
///
/// Numbering ours from far away keeps the two readable in a transcript; not
/// forwarding their responses at all is what actually makes it correct.
///
/// Shared by `Session` and `BasicSession`: each owns its own peer connection
/// and its own [`OwnRequestTracker`], so the two never see each other's
/// numbers - there is nothing for the two ranges to need staying distinct
/// from.
pub const OWN_REQUEST_BASE: i64 = 1_000_000;

/// Requests a session originated, tracked so an answer can be routed back to
/// whichever internal consumer is waiting for it instead of forwarded to the
/// editor.
///
/// `Session` and `BasicSession` each speak to their own peer over their own
/// connection and used to carry near-identical `peer`/`seq`/`own_seq`/
/// `own_requests` bookkeeping side by side with it; this wraps the four
/// together so there is one implementation of "send a request of our own,
/// remember what it is for, and recognise its answer" rather than two.
///
/// `R` is whatever a caller needs to remember about a request it sent - a
/// plain purpose tag, or (`Session`'s case) a purpose plus the command name.
pub struct OwnRequestTracker<P, R> {
    peer: P,
    /// The next seq for a request forwarded from the editor.
    seq: i64,
    /// The next seq for a request of our own.
    own_seq: i64,
    /// Requests this session originated, and what each answer is for.
    own_requests: HashMap<i64, R>
}

impl<P: DapPeer, R> OwnRequestTracker<P, R> {
    pub fn new(peer: P, own_seq_base: i64) -> Self {
        Self {
            peer,
            seq: 1,
            own_seq: own_seq_base,
            own_requests: HashMap::new()
        }
    }

    pub fn peer(&self) -> &P {
        &self.peer
    }

    pub fn peer_mut(&mut self) -> &mut P {
        &mut self.peer
    }

    /// The next seq for a request forwarded from the editor.
    pub fn next_seq(&mut self) -> i64 {
        let seq = self.seq;
        self.seq += 1;
        seq
    }

    /// The seq [`Self::send_own`] is about to hand out, without consuming it -
    /// for a caller that needs to record it before the request actually goes
    /// out.
    pub fn peek_own_seq(&self) -> i64 {
        self.own_seq
    }

    /// Send a request of our own to the peer, and remember `data` as what its
    /// answer is for.
    pub fn send_own(&mut self, command: &str, arguments: Value, data: R) -> std::io::Result<()> {
        let seq = self.own_seq;
        self.own_seq += 1;
        self.own_requests.insert(seq, data);
        self.peer.send(protocol::request(command, arguments, seq))
    }

    /// Whether `response` answers something this session asked for - and if
    /// so, what it was for.
    pub fn is_our_answer(&mut self, response: &Value) -> Option<R> {
        let request_seq = response.get("request_seq").and_then(Value::as_i64)?;
        self.own_requests.remove(&request_seq)
    }

    /// The seq of the oldest outstanding request of our own - for a test
    /// that wants to answer it without predicting its number ahead of time.
    #[cfg(test)]
    pub(crate) fn smallest_pending_own_seq(&self) -> Option<i64> {
        self.own_requests.keys().min().copied()
    }
}

/// Either forward `message` (reporting itself as `command`) to the peer, or
/// refuse it with a DAP failure response.
///
/// Not every command reaching here is ours - most are the editor's own,
/// meant for the peer - but a peer that rejects requests it does not
/// implement turns "forward and see" into an error message shown to the
/// user in place of the thing they actually asked for. Ask first via
/// [`DapPeer::supports`], and only when [`Quirks::rejects_unknown_requests`]
/// says that matters for this peer.
pub fn forward_or_reject<P: DapPeer, R>(
    tracker: &mut OwnRequestTracker<P, R>,
    message: &Value,
    command: &str
) -> std::io::Result<Vec<Value>> {
    if tracker.peer().quirks().rejects_unknown_requests && !tracker.peer_mut().supports(command) {
        let seq = tracker.next_seq();
        return Ok(vec![crate::protocol::failure(
            message,
            &format!("the emulator being debugged does not implement '{command}'."),
            seq
        )]);
    }
    tracker.peer_mut().send(message.clone())?;
    Ok(Vec::new())
}

/// What a peer cannot do, so the translation layer can adapt instead of
/// guessing.
///
/// Explicitly **not** a capacity count: how many breakpoints an emulator
/// supports is its business and changes between versions, so it is discovered
/// from the per-breakpoint answers it gives rather than encoded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quirks {
    /// The peer understands only address breakpoints, so source breakpoints
    /// must be translated (1984js: true).
    pub instruction_breakpoints_only: bool,
    /// The peer must be `attach`ed before it will answer anything about the
    /// running program (1984js: true).
    pub attach_required: bool,
    /// The peer refuses requests it does not implement rather than ignoring
    /// them, so unsupported requests must not be forwarded (1984js: true).
    pub rejects_unknown_requests: bool
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            instruction_breakpoints_only: true,
            attach_required: true,
            rejects_unknown_requests: true
        }
    }
}

/// What the source says about the line the program is stopped on, which is
/// the one thing about a step over an emulator cannot work out for itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineAtPc {
    /// A `defs` directive, occupying this run of bytes. Stepping over it means
    /// running to the end of the run.
    Defs(std::ops::Range<u16>),
    /// A line that is not a `defs`: an ordinary instruction, priced and
    /// stepped by the ordinary rules.
    Ordinary,
    /// Nothing to consult - no source map row for `PC`, or no listing at all.
    /// The bytes are then the only evidence there is.
    Unknown
}

/// A peer that speaks DAP.
pub trait DapPeer: Send {
    /// Send one message to the emulator.
    fn send(&mut self, message: Value) -> std::io::Result<()>;
    /// Every message received since the last call. Never blocks.
    fn drain(&mut self) -> Vec<Value>;
    fn quirks(&self) -> Quirks {
        Quirks::default()
    }

    /// What the source says about the line at `PC`, told to the peer just
    /// before a `next`.
    ///
    /// `defs 60` assembles to sixty zero bytes, which the Z80 runs as sixty
    /// `NOP`s - the way a demo pads a raster line to an exact width. Stepping
    /// over one is meant to be like stepping over a repetition: one press and
    /// the run is done. Nothing in the bytes at `PC` can say so, because they
    /// are `NOP`s and so is a hand-written `nop`; only the source says which of
    /// the two this is, and the source map lives on this side of the seam while
    /// running to an address lives on the other.
    ///
    /// Deliberately **not** a defaulted method. It was one, and the enum that
    /// picks between the two backends forwards `DapPeer` arm by arm - so the
    /// arm nobody wrote fell back to the default, the hint was dropped on the
    /// way to the emulator, and stepping over a real `defs` went on advancing
    /// one `NOP` at a time while every test passed. Required here, the compiler
    /// asks each peer what it does with this.
    fn note_line_at_pc(&mut self, line: LineAtPc);

    /// Whether this peer implements `command`.
    ///
    /// Consulted before anything is forwarded, because a peer with
    /// [`Quirks::rejects_unknown_requests`] answers an unknown request with a
    /// protocol error - and the editor shows that error *instead of* whatever
    /// it asked for. That is how `source` once produced "DAP request 'source'
    /// is not supported" in place of a file's contents.
    ///
    /// The default is what 1984js's `dap.js` dispatches on. An emulator
    /// wrapper with a different set overrides this rather than the translation
    /// layer growing a special case for it.
    fn supports(&self, command: &str) -> bool {
        matches!(
            command,
            "initialize"
                | "attach"
                | "launch"
                | "configurationDone"
                | "disconnect"
                | "threads"
                | "stackTrace"
                | "scopes"
                | "variables"
                | "pause"
                | "continue"
                | "stepIn"
                | "next"
                | "stepOut"
                | "stepBack"
                | "reverseContinue"
                | "setInstructionBreakpoints"
                | "readMemory"
                | "writeMemory"
                | "disassemble"
                // Answered by the page-side bridge rather than by `dap.js`,
                // but it does reach a handler, which is what matters here.
                | "cpclib/setWatches"
                // Same: the bridge drives `_poc_key` itself, typing the
                // requested text as key-matrix events.
                | "cpclib/autotype"
        )
    }
}

/// A peer that records what it was sent and replays scripted answers.
///
/// The test double the translation layer is developed against - the real
/// emulator needs a browser, and every interesting case (a breakpoint the
/// emulator refuses, a stop with no source, an unknown request) is easier to
/// arrange here than in one.
#[derive(Debug, Default)]
pub struct RecordingPeer {
    /// Extra commands this peer claims, for testing a backend that knows more
    /// than 1984js does.
    pub also_supports: Vec<String>,
    pub sent: Vec<Value>,
    pub incoming: Vec<Value>,
    /// Everything the session said about the line at `PC`, in order, so a
    /// test can ask what a step over was told about the source.
    pub lines_at_pc: Vec<LineAtPc>
}

impl RecordingPeer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim a command 1984js does not implement, for testing a richer
    /// backend.
    pub fn also_supporting(mut self, commands: &[&str]) -> Self {
        self.also_supports = commands.iter().map(|c| c.to_string()).collect();
        self
    }

    /// Queue a message as if the emulator had produced it.
    pub fn push_incoming(&mut self, message: Value) {
        self.incoming.push(message);
    }

    /// The commands sent to the emulator, in order.
    pub fn commands(&self) -> Vec<String> {
        self.sent
            .iter()
            .filter_map(|m| m.get("command").and_then(Value::as_str))
            .map(str::to_owned)
            .collect()
    }

    /// The last message sent with `command`.
    pub fn last(&self, command: &str) -> Option<&Value> {
        self.sent
            .iter()
            .rev()
            .find(|m| m.get("command").and_then(Value::as_str) == Some(command))
    }
}

impl DapPeer for RecordingPeer {
    fn send(&mut self, message: Value) -> std::io::Result<()> {
        self.sent.push(message);
        Ok(())
    }

    fn drain(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.incoming)
    }

    fn note_line_at_pc(&mut self, line: LineAtPc) {
        self.lines_at_pc.push(line);
    }

    fn supports(&self, command: &str) -> bool {
        self.also_supports.iter().any(|extra| extra == command)
            || matches!(
                command,
                "initialize"
                    | "attach"
                    | "launch"
                    | "configurationDone"
                    | "disconnect"
                    | "threads"
                    | "stackTrace"
                    | "scopes"
                    | "variables"
                    | "pause"
                    | "continue"
                    | "next"
                    | "stepIn"
                    | "stepOut"
                    | "stepBack"
                    | "reverseContinue"
                    | "setInstructionBreakpoints"
                    | "readMemory"
                    | "writeMemory"
                    | "disassemble"
                    | "cpclib/setWatches"
                    | "cpclib/machineState"
            )
    }
}

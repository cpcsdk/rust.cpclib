//! The emulator, seen as something that speaks DAP.
//!
//! 1984js already does, so its implementation is a transport. A native
//! emulator that only offers monitor primitives would be wrapped into the same
//! shape rather than special-cased upstream, which is why the translation layer
//! is written against this trait and never against a particular emulator.

use serde_json::Value;

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

/// A peer that speaks DAP.
pub trait DapPeer: Send {
    /// Send one message to the emulator.
    fn send(&mut self, message: Value) -> std::io::Result<()>;
    /// Every message received since the last call. Never blocks.
    fn drain(&mut self) -> Vec<Value>;
    fn quirks(&self) -> Quirks {
        Quirks::default()
    }

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
    pub incoming: Vec<Value>
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

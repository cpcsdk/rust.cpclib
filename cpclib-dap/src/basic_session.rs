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
//! [`crate::basic::EXECUTE_STATEMENT_ENTRY`] once per **statement** - every
//! `:`-separated one on a line, not just its first. One instruction
//! breakpoint at [`crate::basic::STATEMENT_BREAKPOINT_TARGET`] (a few bytes
//! into that same routine - see its own doc comment for why not the entry
//! point itself), left armed for the whole session, plus reading
//! [`crate::basic::PTR_CURRENT_LINE_NUMBER_FIELD`] on every hit to compare
//! against the user's actual breakpoints, is the entire stepping/breakpoint
//! mechanism - no per-breakpoint address computation, unlike a Z80 session
//! or the reference `amspirit-basic` extension's own address-mapped
//! approach.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::basic::{
    self, BasicVariableValue, STATEMENT_BREAKPOINT_TARGET, PTR_CURRENT_STATEMENT,
    VARIABLE_CHAIN_HEADS, VARIABLE_CHAIN_HEADS_COUNT
};
use crate::peer::DapPeer;
use crate::protocol::{self, address_reference};
use crate::session::decode_base64;

/// How many bytes of the variable storage area to read in one go - matches
/// the reference `amspirit-basic` extension's own cap for the same bulk
/// read, a reasonable safety limit rather than a measured one.
const MAX_VARIABLE_BYTES: u16 = 8192;

/// The user's own BASIC variables.
const VARIABLES_REFERENCE: i64 = 1000;
/// Program size, variable/array zone boundaries, free RAM, BASIC version -
/// what AMSpiriT Lite's own UI shows in a dedicated info panel
/// ("TXTTOP.../Taille.../Zone variables..."), asked for inside the
/// Variables pane instead: no dedicated webview to build, and the editor
/// already refreshes this scope on every stop for free.
const WORKSPACE_REFERENCE: i64 = 1001;

const THREAD_ID: i64 = 1;

#[derive(Debug, Clone)]
enum Purpose {
    Plain,
    /// The peer's own `attach` handshake, for a peer that needs one.
    Attach,
    /// The very first `continue`, sent right after `configurationDone`. Its
    /// answer is the earliest point the machine is confirmed genuinely
    /// running (AMSpiritLite's own `continue` handling blocks on exactly
    /// that) - the right moment to auto-type `RUN`, on a peer that can.
    LaunchResumed,
    /// The one `setInstructionBreakpoints` this session ever sends. Its
    /// answer used to be discarded (`Purpose::Plain`) - a peer that refused
    /// or failed to verify it (an exhausted breakpoint-channel table, an
    /// address it rejects) left every breakpoint and step silently inert for
    /// the rest of the session, with nothing in the Debug Console to say so.
    BreakpointArmed,
    /// Reading [`basic::PTR_CURRENT_LINE_NUMBER_FIELD`] itself - its value
    /// is a pointer to dereference next, or the direct-mode sentinel.
    CurrentLinePointer,
    /// Dereferencing that pointer to get the actual line number.
    CurrentLineValue,
    /// The 27 chain heads, on the way to decoding variables.
    VariableChainHeads,
    /// The bulk variable-storage read, once the chain heads are known.
    VariableStorage,
    /// `cpclib/basicState`, answered after a genuine stop on a peer with
    /// native BASIC debugging - carries `cur_linenum`/`stmt_addr` directly,
    /// replacing the two-round-trip readMemory dance the generic path
    /// needs.
    NativeBasicState,
    /// `cpclib/basicStep` (`stepIn`/`next`/`stepOut` on a peer with native
    /// BASIC debugging) - the emulator has already paused and stepped by
    /// the time this answers, so what is left is reading where it landed.
    NativeStepDone,
    /// The `cpclib/basicState` read [`Purpose::NativeStepDone`] itself
    /// triggers, to learn where the completed step actually landed.
    NativeStateAfterStep,
    /// A second `cpclib/basicState` read, sent only when
    /// [`Purpose::NativeStateAfterStep`]'s own answer looked stale - see
    /// [`Purpose::NativeContinueStateRetry`]'s doc comment for the same
    /// mechanism on the `Continue` loop. Reported live as the *last*
    /// statement of a multi-statement line never getting highlighted while
    /// single-stepping through one with "Step Into": stepping onto it
    /// echoed the position from *before* that step (still the
    /// second-to-last statement), and stepping again from there moved
    /// straight past it to the next line, so the last statement's own
    /// position was never the one actually shown.
    NativeStateAfterStepRetry,
    /// `Continue` on a native peer, and the launch's own first run
    /// (`autotype_run`) alike: `cpclib/basicStep` answering one statement
    /// of the loop that drives either - see `cpclib/basicSetBreakpoints`'s
    /// own doc comment for why neither trusts `/api/basic_bp` to decide
    /// when to stop.
    NativeContinueStep,
    /// The `cpclib/basicState` read [`Purpose::NativeContinueStep`] itself
    /// triggers, to decide whether this statement is a breakpoint, a
    /// pending pause, or another one to step past.
    NativeContinueState,
    /// A second `cpclib/basicState` read, sent only when
    /// [`Purpose::NativeContinueState`]'s own answer looked stale (see its
    /// doc comment) - decided the same way, but never retried a second time:
    /// a real self-loop (`10 GOTO 10`) legitimately revisits the exact same
    /// address on every step, and this is what stops that case from polling
    /// forever mistaking it for staleness.
    NativeContinueStateRetry,
    /// Reading [`crate::basic::PTR_ARRAYS_START`] for the Workspace scope,
    /// on a peer without native BASIC debugging - `PTR_VARIABLES_START`
    /// itself is not read at all: this session already knows it locally,
    /// having chosen it when it built the boot snapshot.
    WorkspaceArraysStart,
    /// `cpclib/basicState`, answered for the Workspace scope on a peer
    /// with native BASIC debugging - the same call
    /// [`Purpose::NativeBasicState`] uses for a stop, but read here purely
    /// for its workspace fields (`vartop`/`arrend`/`var_size`/`basic_ver`),
    /// not to decide whether to report a stop at all.
    NativeWorkspaceInfo,
    /// `cpclib/basicInject` answering at attach time, on a native peer.
    /// `cpclib/basicListing` is fetched next if the peer offers it (see
    /// [`Purpose::NativeListingFetched`]); either way, `start_if_ready` is
    /// only called once this is known, not fired blind alongside it.
    NativeInjected,
    /// `cpclib/basicListing`, fetched once right after injection on a peer
    /// that supports it - decoded into `native_listing`, then
    /// `start_if_ready` runs.
    NativeListingFetched,
    /// `cpclib/autotype`'s own answer, on a native peer, right after typing
    /// `RUN\n` - kicks off the poll loop ([`Purpose::NativeAwaitRunState`]).
    /// No breakpoint/step machinery is armed yet on purpose: the user's own
    /// framing is exactly right - there is nothing to debug before `RUN` has
    /// even been typed, direct mode has no statements of the program to stop
    /// on, and stepping through it is what broke autotype in the first
    /// place (see `autotype_run`'s own doc comment).
    NativeAwaitRun,
    /// `cpclib/basicState`, polled back to back (no `basicStep` in between)
    /// while still waiting for `RUN` to leave direct mode. Loops on itself
    /// (same purpose) while `cur_linenum` is still the direct-mode sentinel;
    /// once a real line shows up, pauses the machine
    /// ([`Purpose::NativeAwaitRunPaused`]) and only *then* does this session
    /// start caring about breakpoints/statements - see `autotype_run`.
    NativeAwaitRunState,
    /// The `pause` sent once [`Purpose::NativeAwaitRunState`] finally saw a
    /// real line. Re-reads `cpclib/basicState` next
    /// ([`Purpose::NativeAwaitRunSettled`]) to see where the (asynchronous,
    /// not instant - confirmed live: it can still land several lines further
    /// than the one that triggered it) pause actually landed.
    NativeAwaitRunPaused,
    /// The `cpclib/basicState` [`Purpose::NativeAwaitRunPaused`] re-reads
    /// once actually paused. Reports a stop right here on whatever real line
    /// this is - breakpoint if it happens to be one, entry otherwise -
    /// rather than feeding it to [`Purpose::NativeContinueState`]'s own
    /// "not a line I care about, keep stepping" logic: reusing that here
    /// was tried and reproduced the exact same hang this whole chain exists
    /// to avoid, live - a non-breakpoint line right after launch restarted
    /// the very `cpclib/basicStep` loop `autotype_run`'s doc comment
    /// documents as interfering with the machine, which then ran the
    /// program right back into direct mode before ever reporting anything
    /// sensible to the editor.
    NativeAwaitRunSettled,
    /// One chip's own endpoint (`cpclib/crtc`/`ga`/`psg`/`fdc`) answering a
    /// `variables` request against that scope - `reference` says which one,
    /// `request` is what to reply to. Carries its own data rather than going
    /// through a `pending_*` field like `Workspace` does: unlike the Z80
    /// session's `chip_scope` (which batches every expanded chip pane behind
    /// one shared `machineState` snapshot fetch), each chip here has its own
    /// endpoint and its own round trip, so there is nothing to batch and no
    /// reason to force them to serialize through one shared slot.
    NativeChipScope { reference: i64, request: Value }
}

#[derive(Debug, Clone)]
struct OwnRequest {
    purpose: Purpose
}

/// Why the program is being resumed - decides which of the (now
/// statement-granular) breakpoint hits actually gets reported.
///
/// `next`/`stepOut` stay line-granular on purpose - "it is ok for me that
/// step over execute the whole line, but not step into" - so a
/// multi-statement line is still one step for them, exactly as before this
/// session went statement-granular for everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResumeKind {
    /// `stepIn`: stop at the very next statement, whatever line it is on -
    /// several stops on one multi-statement line, matching the Z80
    /// session's own per-instruction stepping.
    StepStatement,
    /// `next`/`stepOut`: stop only once execution reaches a *different*
    /// line than `from_line` - a multi-statement line is one step.
    StepLine { from_line: Option<u16> },
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
    /// The source text itself, kept only for `cpclib/basicInject` (AMSpiriT
    /// Lite's own tokeniser - see `native_amspirit`); nothing else here
    /// needs it, since `line_index`/`statement_index` already extracted
    /// what they need from it once at launch.
    source_text: String,
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
    /// Every statement's RAM address and source-text column span, built
    /// once at launch (see [`basic::build_statement_index`]) - what makes a
    /// stop on a multi-statement line point at the one statement that
    /// actually ran rather than the line's first token every time.
    statement_index: Vec<basic::StatementPosition>,
    /// Each BASIC line's own statements, in address order, as AMSpiriT
    /// Lite's own tokeniser laid them out - `(addr, end)` per statement,
    /// straight from `GET /api/basic_listing` (`cpclib/basicListing`),
    /// fetched once after injection on a peer that supports it. `None` on a
    /// peer without it (nothing to fall back to but `statement_index`'s own
    /// address-based guess), or before the fetch answers.
    ///
    /// Exists because `statement_index` - this session's *own*, independent
    /// tokeniser - does not agree with AMSpiriT Lite's on byte addresses: a
    /// live session showed the drift compounding within a single
    /// multi-statement line, misattributing a later statement's `stmt_addr`
    /// to an earlier one every time (see `apply_native_basic_state`'s doc
    /// comment). This is addressed by position, not address: the *n*th
    /// entry here for a line and the *n*th `statement_index` entry for the
    /// same line are the same statement, whichever addresses either
    /// tokeniser happened to give it - matching by index sidesteps needing
    /// the two to agree on bytes at all, only on how many statements a line
    /// splits into, which just depends on counting colons.
    native_listing: Option<HashMap<u16, Vec<(u16, u16)>>>,
    /// The current statement's source column span, once known - `None`
    /// before the first stop, or when the current statement address does
    /// not appear in `statement_index` (should not happen for a line this
    /// session's own launch flow tokenised, but stack traces still need a
    /// column either way).
    current_statement_column: Option<(u32, u32)>,
    /// The current statement's own RAM address, for the "current
    /// instruction" line of the Workspace scope - the address itself,
    /// unlike `current_statement_column`, which is already resolved to a
    /// source position.
    current_statement_address: Option<u16>,
    /// BASIC line numbers with a user breakpoint.
    breakpoints: Vec<u16>,
    /// Whether the peer answers `cpclib/basicState` - AMSpiriT Lite's own
    /// native BASIC debugging (`/api/basic_bp`/`/api/basic_step`/
    /// `/api/basic_state`), discovered once at `on_attached`. On a peer
    /// that does, the generic setInstructionBreakpoints/&AE1B mechanism
    /// (built for 1984js, which has nothing BASIC-aware to ask) is not
    /// used at all: AMSpiriT Lite resolves BASIC line numbers to statement
    /// addresses itself and only ever pauses on a real match, so there is
    /// nothing here to arm, filter, or read memory for.
    native_amspirit: bool,
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
    pending_variables: Option<PendingVariables>,
    /// The editor's own `variables` request for the Workspace scope, held
    /// until whichever single read answering it (generic or native) comes
    /// back.
    pending_workspace: Option<Value>,
    /// Set when the editor asks to pause, cleared once a stop is actually
    /// reported for it.
    ///
    /// Neither path trusts the peer to decide `Continue`'s own stopping
    /// point: the generic (1984js) path has no per-line breakpoint of its
    /// own (one shared instruction breakpoint fires on *every* statement,
    /// and Rust decides whether it matters), and AMSpiriT Lite's own
    /// `/api/basic_bp` turned out not to be trustworthy either (a live
    /// session saw it report a breakpoint stop with none armed, and resume
    /// on its own unasked) - both loop their own "keep going" step/continue
    /// themselves. A `pause` sent by the editor while that loop's own next
    /// step was already in flight was getting raced by it: both land at the
    /// emulator, ours last, and the pause is undone before the editor ever
    /// sees a stop. This flag is checked at the same point the loop would
    /// otherwise send its next step/continue, so pausing takes effect at the
    /// very next statement boundary instead of never.
    pause_requested: bool,
    /// Set while a native peer's own step/continue mechanism has a
    /// `cpclib/basicStep` in flight, cleared centrally in `report_stopped`
    /// alongside `pause_requested`.
    ///
    /// `cpclib/basicStep`'s own internal mechanism resumes the machine and
    /// re-pauses it - and fires the *same* `basic_bp` SSE event as a side
    /// effect of that, on *every* call, whether or not the line it lands on
    /// is one the user actually armed (confirmed directly: injecting a
    /// program and stepping through it live showed `basic_bp` firing once
    /// per step, addresses matching exactly, entirely independent of what
    /// was armed). A live session showed the cost of not knowing that: every
    /// step this session's own Rust-driven loop takes - one `Continue`
    /// click, dozens of statements - reached the generic unsolicited
    /// `stopped`/`continued` handling too, which cannot tell "our own step
    /// just resumed and re-paused the machine, as designed" apart from "the
    /// peer decided to stop or resume on its own" - and reported *both*, one
    /// spurious stop/continue pair per internal step, on top of whatever the
    /// loop's own tracked chain (`NativeStepDone`/`NativeStateAfterStep`/
    /// `NativeContinueStep`/`NativeContinueState`) already reports correctly
    /// on its own. The editor saw the debug session flicker between paused
    /// and running continuously - exactly what it looked like happening.
    /// This flag tells that generic handling "a Purpose-tracked chain is
    /// already answering for this, stay quiet" - it does not suppress a
    /// genuine unsolicited stop (a manual pause with nothing of ours in
    /// flight, say), only the noise this session's own in-flight request
    /// causes as a side effect of itself.
    native_operation_pending: bool,
    /// Set alongside `native_operation_pending` at every one of the same
    /// sites, but on a longer clock: `native_operation_pending` clears the
    /// moment *this session's own* tracked chain reports a stop
    /// (`report_stopped`), while this stays set straight through that and
    /// only clears at the next resume. Needed because the emulator does not
    /// stop signalling once: reported live, a single real pause produced
    /// three separate `stopped` events reaching the editor as three
    /// separate "unwanted breakpoints" for one actual stop - our own tracked
    /// chain answers the first one and clears `native_operation_pending`
    /// right there, but the straggler `basic_bp`/`stopped` events the
    /// emulator keeps sending for that same pause (its own `pause` SSE
    /// event, `basic_bp`'s own side effect, sometimes both) arrive in
    /// *later* poll cycles, by which point nothing was suppressing them any
    /// more and each got read and reported as if it were a brand new stop.
    /// The editor already knows the machine is stopped once this is set -
    /// nothing unsolicited it hears before the next resume is new
    /// information.
    native_already_stopped: bool
}

impl<P: DapPeer> BasicSession<P> {
    pub fn new(
        peer: P,
        source_path: PathBuf,
        source_text: &str,
        program_start: u16,
        program_bytes: &[u8]
    ) -> Self {
        let line_index = line_index_from_source(source_text);
        let statement_index = basic::build_statement_index(program_bytes, program_start, source_text);
        Self {
            peer,
            source_path,
            source_text: source_text.to_string(),
            line_index,
            program_start,
            program_len: program_bytes.len() as u16,
            statement_index,
            native_listing: None,
            current_statement_column: None,
            current_statement_address: None,
            breakpoints: Vec::new(),
            native_amspirit: false,
            seq: 1,
            own_requests: HashMap::new(),
            own_seq: 100_000,
            attached: false,
            configured: false,
            started: false,
            resuming_as: None,
            current_line: None,
            pending_variables: None,
            pending_workspace: None,
            pause_requested: false,
            native_operation_pending: false,
            native_already_stopped: false
        }
    }

    pub fn peer_mut(&mut self) -> &mut P {
        &mut self.peer
    }

    /// Sends the peer's own `initialize`/`attach` handshake. Called once,
    /// right after construction, regardless of whether this particular peer
    /// actually needs either - one that does not just answers immediately.
    ///
    /// `initialize` first is not optional for 1984js: its embedded DAP
    /// server (`dap.js`) is a real, independent DAP implementation with its
    /// own protocol state machine, and refuses *every* request - including
    /// `attach` itself - with "initialize must be the first request" until
    /// it has seen one. `Session` (the Z80 launch flow, `lib.rs`) already
    /// does this; missing it here left every peer-directed request this
    /// session ever sends failing the same way, confirmed against a real
    /// transcript - `attach`, `setInstructionBreakpoints` and `continue` all
    /// rejected identically, with only `cpclib/autotype` appearing to work
    /// because the bridge script answers that one itself, before it ever
    /// reaches `dap.js`.
    pub fn attach(&mut self) -> std::io::Result<()> {
        self.send_own(
            "initialize",
            json!({ "supportsMemoryEvent": true }),
            Purpose::Plain
        )?;
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
    ///
    /// On a native peer, `start_if_ready` is not called here directly - it
    /// waits for injection to answer (and, on a peer that also offers it,
    /// for `cpclib/basicListing` too) so the very first run already has
    /// `native_listing` in place, the same as any later one. Injection's
    /// own request is otherwise a plain, synchronous "tokenise and place
    /// this" - nothing here needs the fire-and-forget timing the generic
    /// path's own arm-then-start still uses.
    fn on_attached(&mut self) -> std::io::Result<()> {
        self.attached = true;
        self.native_amspirit = self.peer.supports("cpclib/basicState");
        if self.native_amspirit {
            // Re-injects the program through the emulator's own tokeniser
            // and workspace bookkeeping, on top of whatever the launch
            // snapshot already put there - this emulator keeps producing
            // corrupted BASIC state from the hand-built snapshot alone,
            // even with every known pointer fixed and breakpoints/stepping
            // already switched to its own native API, so this sidesteps
            // needing to know why by not depending on the hand-built
            // version at all once this answers.
            self.send_own(
                "cpclib/basicInject",
                json!({ "source": self.source_text }),
                Purpose::NativeInjected
            )?;
            return Ok(());
        }
        self.send_own(
            "setInstructionBreakpoints",
            json!({ "breakpoints": [{ "instructionReference": address_reference(STATEMENT_BREAKPOINT_TARGET as u32) }] }),
            Purpose::BreakpointArmed
        )?;
        self.start_if_ready()
    }

    /// `None` if the peer verified the one breakpoint this session lives or
    /// dies by; otherwise a message worth putting in front of the user,
    /// since a silently-unarmed breakpoint looks identical to "stepping and
    /// breakpoints just don't work" - which is exactly the bug report this
    /// exists to rule in or out on the next attempt.
    fn breakpoint_arm_warning(response: &Value) -> Option<String> {
        if response.get("success").and_then(Value::as_bool) == Some(false) {
            let message = response
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("the emulator refused it");
            return Some(format!(
                "could not arm the line breakpoint at {}: {message} - \
                 breakpoints and stepping will not work this session",
                address_reference(STATEMENT_BREAKPOINT_TARGET as u32)
            ));
        }
        let verified = response
            .get("body")
            .and_then(|b| b.get("breakpoints"))
            .and_then(Value::as_array)
            .and_then(|list| list.first())
            .and_then(|bp| bp.get("verified"))
            .and_then(Value::as_bool);
        match verified {
            Some(false) => {
                let message = response
                    .get("body")
                    .and_then(|b| b.get("breakpoints"))
                    .and_then(Value::as_array)
                    .and_then(|list| list.first())
                    .and_then(|bp| bp.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("not verified");
                Some(format!(
                    "the line breakpoint at {} did not verify: {message} - \
                     breakpoints and stepping will not work this session",
                    address_reference(STATEMENT_BREAKPOINT_TARGET as u32)
                ))
            },
            // `Some(true)`: verified. `None`: a peer that answers this
            // request with no `body.breakpoints` at all (not one this crate
            // has seen) - nothing to warn about from the shape alone.
            Some(true) | None => None
        }
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
    ///
    /// On a native peer, typing `RUN` alone would leave the machine running
    /// freely from here on, trusting `/api/basic_bp` to catch a breakpoint -
    /// the exact mechanism `cpclib/basicSetBreakpoints`'s own doc comment
    /// documents as unreliable, reproduced live: a breakpoint set *before*
    /// this very first run went uncaught, the program simply finished, and
    /// only a later manual pause (on an unrelated line) ever stopped
    /// anything.
    ///
    /// This does *not* start the Rust-driven `cpclib/basicStep` loop an
    /// editor-requested `continue` uses (the native `"continue"` arm in
    /// `on_editor_message`) - that was tried first and made things *worse*,
    /// reproduced live and confirmed directly against a real instance:
    /// `basic_step`'s own pause/resume cycling, called back to back while
    /// the machine is still processing the typed keystrokes, seems to
    /// interfere with the firmware's own keyboard-scan timing - it took
    /// hundreds of steps and never actually left direct mode, live-testing
    /// showed, exactly matching a user having to finish typing RUN
    /// themselves because the automatic version never got there. Left
    /// running completely untouched instead (no step, no poll in between),
    /// the same program reached its very first real line in well under a
    /// second. So this starts `Purpose::NativeAwaitRun` instead: poll
    /// `cpclib/basicState` alone, back to back, no `basicStep` in between,
    /// until `cur_linenum` leaves direct mode, then pause
    /// (`Purpose::NativeAwaitRunPaused`) and report a stop right on whatever
    /// real line the pause actually lands on
    /// (`Purpose::NativeAwaitRunSettled`) - breakpoint if it is one, entry
    /// otherwise. It does *not* hand off into `NativeContinueState`'s own
    /// "keep stepping past lines I don't care about" loop: that was tried
    /// too and reproduced the same hang one level down, live - `pause` is
    /// itself asynchronous (confirmed live: the state read right after a
    /// successful `pause` response can already be several lines past the
    /// one the poll saw), so a non-breakpoint landing line would restart
    /// exactly the `basicStep` cycling this function exists to avoid, and it
    /// did, running the program straight back into direct mode before
    /// anything sensible ever reached the editor. There is nothing to debug
    /// yet at this point anyway (the user's own framing) - Continue/step
    /// only need to start caring about breakpoints once the editor actually
    /// asks for one, not during this handoff. The real cost is precision:
    /// the program is not observed statement by statement during the poll
    /// window the way a later `Continue` is, so a breakpoint on a line
    /// reached very early (before the first poll catches a real line at
    /// all, or between the poll and the pause actually landing) can still be
    /// run straight past - a narrower version of the same problem this
    /// function's own launch fix exists for, traded deliberately for not
    /// hanging the launch itself.
    fn autotype_run(&mut self) -> std::io::Result<()> {
        if !self.peer.supports("cpclib/autotype") {
            return Ok(());
        }
        let purpose = if self.native_amspirit {
            // See `native_operation_pending`'s own doc comment - this is a
            // separate trigger site from the editor's own "continue" (the
            // launch's first run, not requested through it), so it needs
            // the same flag set here too. `native_already_stopped` has
            // nothing to clear yet at this specific site (nothing has been
            // reported stopped before the very first run) but is reset here
            // anyway for the same reason `resuming_as`/`pause_requested` are
            // reset at every resume site: this being the one that is ever
            // skipped is how a stale `true` survives to cause the next
            // straggler to be dropped.
            self.native_operation_pending = true;
            self.native_already_stopped = false;
            // The freshly-loaded snapshot is captured at the Ready prompt,
            // firmware already set up - `continue` here is a plain unpause,
            // not a cold boot, so there is no boot sequence to wait out. But
            // "the CPU is executing" (confirmed synchronously inside
            // `continue`'s own `send`, see `wait_until_it_is_really_running`)
            // is not the same guarantee as "the keyboard-scan interrupt has
            // actually run at least once since" - reported live, and
            // directly visible in a captured screenshot: characters typed
            // this early landed as if the line editor had *already* been mid
            // multi-key-scan (a bare digit or two, then `RUN"` with a stray
            // quote, echoed exactly the way BASIC's own line editor echoes
            // real keystrokes it received - not garbled screen memory, real
            // input landing before the machine had settled long enough to
            // scan it correctly). A short settle here, before the very first
            // keystroke of the whole session, costs one launch's worth of
            // latency to avoid it.
            std::thread::sleep(std::time::Duration::from_millis(500));
            Purpose::NativeAwaitRun
        }
        else {
            Purpose::Plain
        };
        self.send_own("cpclib/autotype", json!({ "text": "RUN\n" }), purpose)
    }

    pub fn on_editor_message(&mut self, message: &Value) -> std::io::Result<Vec<Value>> {
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();

        match command {
            "setBreakpoints" => self.set_breakpoints(message),
            "configurationDone" => {
                self.configured = true;
                self.start_if_ready()?;
                let seq = self.next_seq();
                Ok(vec![protocol::response(message, json!({}), seq)])
            },
            "continue" | "next" | "stepIn" | "stepOut" if self.native_amspirit => {
                // See `native_operation_pending`'s own doc comment: every
                // one of these makes `cpclib/basicStep` fire the same
                // `basic_bp` SSE event as a side effect, and the generic
                // unsolicited handling needs to know not to report that
                // itself while this session's own tracked chain already
                // will. `native_already_stopped` clears here too - see its
                // own doc comment: the machine is about to actually resume,
                // so anything unsolicited from here on is new again.
                self.native_operation_pending = true;
                self.native_already_stopped = false;
                if command == "continue" {
                    // AMSpiriT Lite's own `/api/basic_bp` cannot be trusted
                    // to decide this: a live session showed it reporting a
                    // "breakpoint" stop with zero breakpoints armed, and
                    // resuming on its own with nothing having asked it to.
                    // Its statement stepper is already proven correct
                    // (stepIn/next already run on it) - looping that
                    // ourselves and deciding the stop here, exactly like the
                    // generic path already does, is simpler and more
                    // trustworthy than a second, independent breakpoint
                    // mechanism living entirely on the peer's side.
                    self.resuming_as = Some(ResumeKind::Continue);
                    self.send_own(
                        "cpclib/basicStep",
                        json!({ "mode": "stmt" }),
                        Purpose::NativeContinueStep
                    )?;
                }
                else {
                    // The emulator's own stepper: it pauses (if not
                    // already) and steps in the one call, so there is no
                    // loop to run here - `mode=stmt`/`mode=line` already
                    // encode the stepIn/next distinction the generic path
                    // otherwise needs `ResumeKind` for.
                    let mode = if command == "stepIn" { "stmt" } else { "line" };
                    self.send_own(
                        "cpclib/basicStep",
                        json!({ "mode": mode }),
                        Purpose::NativeStepDone
                    )?;
                }
                let seq = self.next_seq();
                Ok(vec![protocol::response(message, json!({}), seq)])
            },
            "continue" | "next" | "stepIn" | "stepOut" => {
                // `stepIn` stops at the very next statement - several stops
                // on one multi-statement line. `next`/`stepOut` stay
                // line-granular: "step over the whole line" is the point of
                // them, so they keep filtering by line the way this session
                // always has, just now against a statement-granular stream
                // of hits instead of a line-granular one.
                self.resuming_as = Some(match command {
                    "continue" => ResumeKind::Continue,
                    "stepIn" => ResumeKind::StepStatement,
                    _ => {
                        ResumeKind::StepLine {
                            from_line: self.current_line
                        }
                    }
                });
                self.send_own("continue", json!({ "threadId": THREAD_ID }), Purpose::Plain)?;
                let seq = self.next_seq();
                Ok(vec![protocol::response(message, json!({}), seq)])
            },
            "pause" => {
                // Both paths now drive their own "keep going" loop for
                // Continue (see `pause_requested`'s own doc comment) and can
                // race a pause the same way. Forwarded to the peer either
                // way - harmless on the native path, where the CPU is
                // already effectively paused between our own `basicStep`
                // calls, but there is no reason to withhold it.
                self.pause_requested = true;
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
            "variables"
                if message
                    .get("arguments")
                    .and_then(|a| a.get("variablesReference"))
                    .and_then(Value::as_i64)
                    == Some(WORKSPACE_REFERENCE) =>
            {
                self.begin_workspace(message)?;
                Ok(Vec::new())
            },
            "variables"
                if self.native_amspirit
                    && message
                        .get("arguments")
                        .and_then(|a| a.get("variablesReference"))
                        .and_then(Value::as_i64)
                        .is_some_and(crate::inspect::is_chip_scope) =>
            {
                self.begin_chip_scope(message)
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

    fn set_breakpoints(&mut self, message: &Value) -> std::io::Result<Vec<Value>> {
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

        // AMSpiriT Lite resolves these line numbers to statement addresses
        // itself and only pauses on a real match - nothing to arm on the
        // generic path (no single shared breakpoint there to filter after
        // the fact).
        if self.native_amspirit {
            self.send_own(
                "cpclib/basicSetBreakpoints",
                json!({ "lines": self.breakpoints }),
                Purpose::Plain
            )?;
        }

        let seq = self.next_seq();
        Ok(vec![protocol::response(message, json!({ "breakpoints": verified }), seq)])
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
        // Which statement on a multi-statement line actually ran, not just
        // which line - `50 sp=0 : px=320 : py=300` is one line and five
        // stops. `1` (no highlight) when the current address is not one
        // this session's own launch flow tokenised - direct mode, most
        // often, which has no statement to point at.
        let (column, end_column) = self.current_statement_column.unwrap_or((1, 1));

        protocol::response(
            message,
            json!({
                "stackFrames": [{
                    "id": 0,
                    "name": format!("BASIC {line}"),
                    "line": source_line,
                    "column": column,
                    "endColumn": end_column,
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
        let mut scopes = vec![
            json!({
                "name": "Variables",
                "variablesReference": VARIABLES_REFERENCE,
                "expensive": false,
                // VS Code only auto-highlights a value that changed
                // since the last stop for a scope hinted this way -
                // there is no separate "changed" flag to set per
                // variable, the way the memory view's own custom
                // event has one. Not literally CPU registers, but
                // the same "small set of frequently-changing named
                // values" this hint is for, and the same one the
                // Z80 session's own Registers scope already uses.
                "presentationHint": "registers"
            }),
            json!({
                "name": "Workspace",
                "variablesReference": WORKSPACE_REFERENCE,
                "expensive": false
            })
        ];
        // The chips behind the BASIC program: added on request, to help
        // diagnose a screen/timing problem the BASIC variables alone cannot
        // explain (a broken snapshot, a CRTC left in a bad state, ...) -
        // native only, since unlike the Z80 session's own chip scopes there
        // is no `machineState`-snapshot fallback wired up on this session
        // for a peer without a dedicated endpoint per chip. `expensive: true`
        // keeps every one of these opt-in, fetched only once actually
        // expanded rather than on every stop - the same reasoning that
        // throttled the RUN-await poll loop applies here too: this session
        // has already seen what hammering AMSpiriT Lite's HTTP server with
        // requests it did not ask to be asked does to it.
        if self.native_amspirit {
            for (name, reference, command) in [
                ("CRTC", crate::inspect::CRTC_REFERENCE, "cpclib/crtc"),
                ("Gate Array", crate::inspect::GATE_ARRAY_REFERENCE, "cpclib/ga"),
                ("PSG", crate::inspect::PSG_REFERENCE, "cpclib/psg"),
                ("Disc", crate::inspect::DISC_REFERENCE, "cpclib/fdc")
            ] {
                if self.peer.supports(command) {
                    scopes.push(json!({
                        "name": name,
                        "variablesReference": reference,
                        "expensive": true,
                        "presentationHint": "registers"
                    }));
                }
            }
        }
        protocol::response(message, json!({ "scopes": scopes }), seq)
    }

    fn begin_workspace(&mut self, message: &Value) -> std::io::Result<()> {
        self.pending_workspace = Some(message.clone());
        if self.native_amspirit {
            self.send_own("cpclib/basicState", json!({}), Purpose::NativeWorkspaceInfo)
        }
        else {
            self.send_own(
                "readMemory",
                json!({
                    "memoryReference": address_reference(basic::PTR_ARRAYS_START as u32),
                    "count": 2
                }),
                Purpose::WorkspaceArraysStart
            )
        }
    }

    /// Fetches one chip's own endpoint directly - no snapshot, no batching,
    /// see [`Purpose::NativeChipScope`]. `reference` not being one of the
    /// four this session actually advertises in `scopes` (stale state from
    /// before a peer swap, or a client that asks anyway) answers empty
    /// rather than sending a request nobody can route.
    fn begin_chip_scope(&mut self, message: &Value) -> std::io::Result<Vec<Value>> {
        let reference = message
            .get("arguments")
            .and_then(|a| a.get("variablesReference"))
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let Some(command) = crate::amspiritlite::chip_command(reference) else {
            let seq = self.next_seq();
            return Ok(vec![protocol::response(message, json!({ "variables": [] }), seq)]);
        };
        self.send_own(
            command,
            json!({}),
            Purpose::NativeChipScope {
                reference,
                request: message.clone()
            }
        )?;
        Ok(Vec::new())
    }

    /// One `{name, value}` entry for the Workspace scope - always a leaf
    /// (`variablesReference: 0`): everything shown here is a single
    /// address, size or version string, nothing worth expanding.
    fn workspace_entry(name: &str, value: impl Into<String>) -> Value {
        json!({ "name": name, "value": value.into(), "variablesReference": 0 })
    }

    /// The Workspace scope on a peer without native BASIC debugging:
    /// `PTR_VARIABLES_START` (TXTTOP) is not read at all, since this
    /// session already knows it locally - only `vartop`
    /// ([`basic::PTR_ARRAYS_START`]'s live value, which moves as variables
    /// are created) needs a round trip.
    fn complete_workspace_generic(&mut self, vartop: Option<u16>) -> Option<Vec<Value>> {
        let request = self.pending_workspace.take()?;
        let seq = self.next_seq();
        let txttop = self.variables_base();

        let mut entries = vec![
            Self::workspace_entry("Program start", address_reference(self.program_start as u32)),
            Self::workspace_entry("Program size", format!("{} B", self.program_len)),
            Self::workspace_entry("BASIC version", "1.1")
        ];
        if let Some(vartop) = vartop {
            entries.push(Self::workspace_entry(
                "Variables zone",
                format!(
                    "{}\u{2013}{} ({} B)",
                    address_reference(txttop as u32),
                    address_reference(vartop as u32),
                    vartop.saturating_sub(txttop)
                )
            ));
        }
        else {
            entries.push(Self::workspace_entry(
                "Variables start (TXTTOP)",
                address_reference(txttop as u32)
            ));
        }
        if let Some(address) = self.current_statement_address {
            entries.push(Self::workspace_entry(
                "Current instruction",
                address_reference(address as u32)
            ));
        }

        Some(vec![protocol::response(&request, json!({ "variables": entries }), seq)])
    }

    /// The Workspace scope on a peer with native BASIC debugging - every
    /// field `cpclib/basicState` carries beyond what a stop itself needs
    /// (`cur_linenum`/`stmt_addr`), matching this emulator's own info panel
    /// field for field ("TXTTOP.../Taille.../Zone variables...") but with
    /// English titles and inside the standard Variables pane rather than a
    /// dedicated view.
    fn complete_workspace_native(&mut self, message: &Value) -> Option<Vec<Value>> {
        let request = self.pending_workspace.take()?;
        let seq = self.next_seq();
        let body = message.get("body");
        let field = |key: &str| body.and_then(|b| b.get(key)).and_then(Value::as_u64);

        let txttop = field("txttop");
        let vartop = field("vartop");
        let arrend = field("arrend");

        let mut entries = Vec::new();
        if let Some(size) = field("prog_size") {
            entries.push(Self::workspace_entry("Program size", format!("{size} B")));
        }
        if let (Some(t), Some(v)) = (txttop, vartop) {
            let size = field("var_size").unwrap_or_else(|| v.saturating_sub(t));
            entries.push(Self::workspace_entry(
                "Variables zone",
                format!(
                    "{}\u{2013}{} ({size} B)",
                    address_reference(t as u32),
                    address_reference(v as u32)
                )
            ));
        }
        if let (Some(v), Some(a)) = (vartop, arrend) {
            entries.push(Self::workspace_entry(
                "Arrays zone",
                format!("{}\u{2013}{}", address_reference(v as u32), address_reference(a as u32))
            ));
        }
        if let Some(end) = arrend {
            // Matches this emulator's own web UI (`basicRefresh`'s "Free
            // RAM" field): the gap from the array zone's end to the fixed
            // start of the BASIC system workspace.
            let free = if end < 0xae14 { 0xae14 - end } else { 0 };
            entries.push(Self::workspace_entry("Free RAM", format!("{free} B")));
        }
        if let Some(version) = field("basic_ver") {
            entries.push(Self::workspace_entry(
                "BASIC version",
                format!("1.{}", if version == 10 { "0" } else { "1" })
            ));
        }
        if let Some(address) = field("stmt_addr") {
            entries.push(Self::workspace_entry(
                "Current instruction",
                address_reference(address as u32)
            ));
        }

        Some(vec![protocol::response(&request, json!({ "variables": entries }), seq)])
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

        // Both reads have arrived, but "arrived" is not "succeeded": a
        // `readMemory` the peer refused (real transcript - AMSpiriT Lite's
        // "The CPC must be stopped for this request", when the editor asks
        // for variables while a step is still in flight) comes back with no
        // `body.data` at all, and `read_memory_bytes` turns that into an
        // empty `Vec` rather than failing outright. Indexing into that
        // blindly - `chain_heads[i * 2]` on a 0-length buffer - is exactly
        // what crashed the whole adapter process, taking the session with
        // it. An empty variables list is a far better answer than a panic.
        let mut heads = [0u16; VARIABLE_CHAIN_HEADS_COUNT];
        for (i, h) in heads.iter_mut().enumerate() {
            let Some(pair) = chain_heads.get(i * 2..i * 2 + 2) else {
                return self.fail_variables();
            };
            *h = u16::from_le_bytes([pair[0], pair[1]]);
        }
        let def_fn_offset = VARIABLE_CHAIN_HEADS_COUNT * 2;
        let Some(def_fn_bytes) = chain_heads.get(def_fn_offset..def_fn_offset + 2)
        else {
            return self.fail_variables();
        };
        let def_fn_head = u16::from_le_bytes([def_fn_bytes[0], def_fn_bytes[1]]);

        let vars = basic::decode_variable_chains(&heads, def_fn_head, pending.variables_base, storage);

        let request = self.pending_variables.take()?.request?;
        let seq = self.next_seq();
        let entries: Vec<Value> = vars
            .iter()
            .map(|v| {
                let type_name = variable_type_name(&v.value);
                json!({
                    "name": v.name,
                    // The DAP `type` field alone is not enough to actually
                    // see it: VS Code's own Variables tree only ever shows
                    // it in a hover tooltip, never inline, whatever
                    // `supportsVariableType` the editor declared. Baked
                    // into the value string instead, where it is always
                    // visible - the field itself stays too, for whatever
                    // else might read it.
                    "value": format!("{} ({type_name})", format_variable_value(&v.value)),
                    "type": type_name,
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

    /// Answers a pending `variables` request with an empty list rather than
    /// leaving it hanging - the editor asked something, and something is
    /// always owed back, even when the read behind it did not succeed.
    fn fail_variables(&mut self) -> Option<Vec<Value>> {
        let request = self.pending_variables.take()?.request?;
        let seq = self.next_seq();
        Some(vec![protocol::response(
            &request,
            json!({ "variables": [] }),
            seq
        )])
    }

    pub fn on_emulator_message(&mut self, message: &Value) -> Vec<Value> {
        if message.get("type").and_then(Value::as_str) == Some("response") {
            let Some(own) = self.is_our_answer(message)
            else {
                // Not one of this session's own tracked requests: the
                // answer to something forwarded to the peer verbatim under
                // the editor's *own* seq - `pause`, or anything the
                // catch-all in `on_editor_message` passed straight through.
                // Those never got acknowledged at all before this: the
                // editor's request just sat there, which is what "pause
                // does not seem to work" looks like from the outside.
                let seq = self.next_seq();
                let mut forwarded = message.clone();
                forwarded["seq"] = json!(seq);
                return vec![forwarded];
            };
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
                Purpose::BreakpointArmed => {
                    if let Some(warning) = Self::breakpoint_arm_warning(message) {
                        return vec![protocol::event(
                            "output",
                            json!({ "category": "stderr", "output": format!("{warning}\n") }),
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
                },
                Purpose::NativeBasicState => {
                    return match self.apply_native_basic_state(message) {
                        Some(line) => {
                            let reason = if self.breakpoints.contains(&line) {
                                "breakpoint"
                            }
                            else {
                                "pause"
                            };
                            self.report_stopped(reason)
                        },
                        None => self.report_stopped("entry")
                    };
                },
                Purpose::NativeStepDone => {
                    // The emulator already paused and stepped by the time
                    // this answers - what is left is reading where it
                    // landed.
                    let _ = self.send_own(
                        "cpclib/basicState",
                        json!({}),
                        Purpose::NativeStateAfterStep
                    );
                },
                Purpose::NativeStateAfterStep => {
                    // Stale-read guard - see `Purpose::NativeStateAfterStepRetry`'s
                    // own doc comment. Capped at exactly one retry for the
                    // same reason the `Continue` loop's own version is: a
                    // real self-loop (`10 GOTO 10`) legitimately revisits
                    // the same address every step.
                    let address_before_step = self.current_statement_address;
                    let line = self.apply_native_basic_state(message);
                    if line.is_some()
                        && address_before_step.is_some()
                        && self.current_statement_address == address_before_step
                    {
                        let _ = self.send_own(
                            "cpclib/basicState",
                            json!({}),
                            Purpose::NativeStateAfterStepRetry
                        );
                        return Vec::new();
                    }
                    return self.decide_step_stop(line);
                },
                Purpose::NativeStateAfterStepRetry => {
                    let line = self.apply_native_basic_state(message);
                    return self.decide_step_stop(line);
                },
                Purpose::NativeContinueStep => {
                    // The peer has already paused and stepped by the time
                    // this answers - what is left is reading where it
                    // landed, to decide whether to report a stop or step
                    // past it.
                    let _ = self.send_own("cpclib/basicState", json!({}), Purpose::NativeContinueState);
                },
                Purpose::NativeContinueState => {
                    // Stale-read guard, reported live: a `basicState` read
                    // taken immediately after `basicStep` can still echo the
                    // address from *before* the step - the emulator's own
                    // `basic_bp` SSE event already named the real, later
                    // line by the time this same read came back showing the
                    // old one. Trusted anyway, a breakpoint got reported
                    // twice for what was really only one stop: this step
                    // landed on it, was reported, `continue` was clicked,
                    // one step ran, and the stale read said the machine was
                    // still sitting on the exact same statement it had just
                    // left. One extra read, only when nothing moved, is
                    // cheap; capped at exactly one retry
                    // (`NativeContinueStateRetry` does not check again) so a
                    // real self-loop (`10 GOTO 10`, which legitimately
                    // revisits the same address every step) cannot turn this
                    // into an infinite poll.
                    let address_before_step = self.current_statement_address;
                    let line = self.apply_native_basic_state(message);
                    if line.is_some()
                        && address_before_step.is_some()
                        && self.current_statement_address == address_before_step
                    {
                        let _ = self.send_own(
                            "cpclib/basicState",
                            json!({}),
                            Purpose::NativeContinueStateRetry
                        );
                        return Vec::new();
                    }
                    return self.decide_continue_stop(line);
                },
                Purpose::NativeContinueStateRetry => {
                    let line = self.apply_native_basic_state(message);
                    return self.decide_continue_stop(line);
                },
                Purpose::NativeAwaitRun => {
                    let _ = self.send_own("cpclib/basicState", json!({}), Purpose::NativeAwaitRunState);
                },
                Purpose::NativeAwaitRunState => {
                    let line = message
                        .get("body")
                        .and_then(|b| b.get("cur_linenum"))
                        .and_then(Value::as_u64)
                        .map(|n| n as u16);
                    match line {
                        // Still direct mode: RUN hasn't reached the
                        // interpreter yet. No `basicStep` here, see
                        // `autotype_run`'s doc comment for why stepping this
                        // window is what breaks autotype in the first place
                        // - but a bare throttled sleep before polling again
                        // instead of firing back to back: reported live,
                        // polling at the request rate `perform`'s own
                        // one-connection-per-call design allowed (every
                        // request opens and tears down a fresh TCP
                        // connection - hundreds of these in a couple of
                        // seconds) reproduced the exact keyboard/rendering
                        // corruption this whole poll-not-step design exists
                        // to avoid, just from a different cause: connection
                        // churn competing with the emulator's own
                        // single-threaded frame/keyboard-scan loop for CPU
                        // time, not pause/resume cycling. Paced at roughly
                        // the same ballpark as the emulator's own `frame`
                        // SSE heartbeat (ten times a second, see
                        // `amspiritlite.rs`), which it already sustains
                        // without trouble.
                        None | Some(0xffff) => {
                            std::thread::sleep(std::time::Duration::from_millis(30));
                            let _ = self.send_own(
                                "cpclib/basicState",
                                json!({}),
                                Purpose::NativeAwaitRunState
                            );
                        },
                        // A real line at last: stop free-running and only
                        // now start caring about breakpoints/statements.
                        Some(_) => {
                            let _ = self.send_own(
                                "pause",
                                json!({ "threadId": THREAD_ID }),
                                Purpose::NativeAwaitRunPaused
                            );
                        }
                    }
                },
                Purpose::NativeAwaitRunPaused => {
                    let _ = self.send_own("cpclib/basicState", json!({}), Purpose::NativeAwaitRunSettled);
                },
                Purpose::NativeAwaitRunSettled => {
                    return match self.apply_native_basic_state(message) {
                        Some(line) if self.breakpoints.contains(&line) => {
                            self.report_stopped("breakpoint")
                        },
                        _ => self.report_stopped("entry")
                    };
                },
                Purpose::NativeChipScope { reference, request } => {
                    let body = message.get("body").cloned().unwrap_or_default();
                    let variables = crate::amspiritlite::chip_variables(reference, &body);
                    let seq = self.next_seq();
                    return vec![protocol::response(&request, json!({ "variables": variables }), seq)];
                },
                Purpose::WorkspaceArraysStart => {
                    let bytes = Self::read_memory_bytes(message);
                    let vartop = bytes.get(0..2).map(|b| u16::from_le_bytes([b[0], b[1]]));
                    return self.complete_workspace_generic(vartop).unwrap_or_default();
                },
                Purpose::NativeWorkspaceInfo => {
                    return self.complete_workspace_native(message).unwrap_or_default();
                },
                Purpose::NativeInjected => {
                    if self.peer.supports("cpclib/basicListing") {
                        let _ =
                            self.send_own("cpclib/basicListing", json!({}), Purpose::NativeListingFetched);
                    }
                    else if let Err(problem) = self.start_if_ready() {
                        return vec![protocol::event(
                            "output",
                            json!({ "category": "stderr", "output": format!("{problem}\n") }),
                            1
                        )];
                    }
                },
                Purpose::NativeListingFetched => {
                    self.apply_native_listing(message);
                    if let Err(problem) = self.start_if_ready() {
                        return vec![protocol::event(
                            "output",
                            json!({ "category": "stderr", "output": format!("{problem}\n") }),
                            1
                        )];
                    }
                }
            }
            return Vec::new();
        }

        let kind = message.get("type").and_then(Value::as_str).unwrap_or_default();
        if kind == "event" {
            let event = message.get("event").and_then(Value::as_str).unwrap_or_default();
            // See `native_operation_pending`'s own doc comment: this
            // session's own step/continue loop makes the peer pause and
            // resume as a normal, internal part of itself, and both are
            // *also* visible here, unsolicited, on a native peer - reported
            // live as the whole debug session appearing to flicker between
            // paused and running continuously while a single `Continue`
            // click's own loop was quietly working through dozens of
            // statements underneath it. Neither is dropped outright (a
            // manual pause with nothing of ours in flight still needs this
            // path), only while this session's own tracked chain is already
            // going to answer for it.
            if self.native_amspirit && self.native_operation_pending {
                return Vec::new();
            }
            if event == "stopped" {
                // The editor already knows the machine is stopped - see
                // `native_already_stopped`'s own doc comment for why the
                // emulator alone does not stop saying so just because this
                // session's own tracked chain already reported it once.
                if self.native_amspirit && self.native_already_stopped {
                    return Vec::new();
                }
                return self.on_z80_stopped();
            }
            if event == "continued" {
                // The peer resuming on its own reaches here unsolicited (a
                // native peer can decide this by itself - reported live: the
                // emulator resumed, its own `continued` event arrived, and
                // with nothing forwarding it the editor never found out and
                // sat showing "paused" while the program was actually
                // running again). Forward it as-is rather than dropping it.
                // The machine is not stopped any more either way, so a
                // `stopped` seen after this is new again, not a straggler.
                self.native_already_stopped = false;
                let seq = self.next_seq();
                return vec![protocol::event(
                    "continued",
                    message
                        .get("body")
                        .cloned()
                        .unwrap_or_else(|| json!({ "threadId": THREAD_ID, "allThreadsContinued": true })),
                    seq
                )];
            }
            if event == "initialized" {
                return Vec::new();
            }
        }
        Vec::new()
    }

    /// The peer just paused, with nothing of this session's own already
    /// in flight to account for it (`on_emulator_message`'s own caller
    /// already filtered out the case where there is - see
    /// `native_operation_pending`'s doc comment). On the generic path this
    /// is the one shared Z80 breakpoint firing - not necessarily a line the
    /// user actually wants to stop at, which is only known once the current
    /// line has been read and compared. On a native peer, reaching here
    /// unfiltered means either a real armed breakpoint or a manual pause -
    /// nothing left to distinguish beyond reading where it landed.
    fn on_z80_stopped(&mut self) -> Vec<Value> {
        let sent = if self.native_amspirit {
            self.send_own("cpclib/basicState", json!({}), Purpose::NativeBasicState)
        }
        else {
            // One 4-byte read rather than two: PTR_CURRENT_STATEMENT and
            // PTR_CURRENT_LINE_NUMBER_FIELD are adjacent (`&AE1B`, `&AE1D`),
            // so both come back in the same round trip.
            self.send_own(
                "readMemory",
                json!({
                    "memoryReference": address_reference(PTR_CURRENT_STATEMENT as u32),
                    "count": 4
                }),
                Purpose::CurrentLinePointer
            )
        };
        if sent.is_err() {
            return Vec::new();
        }
        Vec::new()
    }

    /// Decodes `cpclib/basicState`'s body and updates `current_line`/
    /// `current_statement_column` from it. Returns the decoded line
    /// number, or `None` for direct mode (`cur_linenum` `0xFFFF`) - the
    /// caller decides what stop reason that means, since a breakpoint-
    /// triggered stop and a completed step report it differently.
    ///
    /// `stmt_addr` is resolved to a *position* within `cur_linenum` - the
    /// statement's index on the line, first/second/third - not directly to
    /// an address, and that position is what actually picks the column: see
    /// `statement_position_in_line`'s own doc comment for why an address
    /// comparison is not trustworthy even scoped to one line, and
    /// `native_listing`'s for how position sidesteps that entirely when it
    /// is available.
    ///
    /// `stmt_addr` itself is not always trustworthy either: reported live
    /// (and directly reproduced against a real instance), a `basicState`
    /// read taken right after a fresh `pause` can answer with a `stmt_addr`
    /// nowhere near this program at all (`63`, once, against a program
    /// starting at `368`) - stale bookkeeping the pause has not caught up
    /// on yet, not a real statement address. Trusting it anyway silently
    /// misattributed the highlight to the *first* statement on the line
    /// (the floor search in `statement_position_in_line` finds nothing at
    /// or below a too-small address and falls back to position `0`) -
    /// reported live as "the right line is selected but not the right
    /// token." Anything outside this program's own address range is
    /// rejected instead: `current_statement_column` is left as it was
    /// (whole-line highlight, via `report_stopped`'s own fallback) rather
    /// than actively pointing at the wrong token.
    fn apply_native_basic_state(&mut self, message: &Value) -> Option<u16> {
        let body = message.get("body")?;
        let line = body.get("cur_linenum").and_then(Value::as_u64)? as u16;
        if line == 0xffff {
            return None;
        }
        self.current_line = Some(line);
        let program_range = self.program_start..self.program_start.wrapping_add(self.program_len);
        if let Some(address) = body
            .get("stmt_addr")
            .and_then(Value::as_u64)
            .map(|a| a as u16)
            .filter(|address| program_range.contains(address))
        {
            self.current_statement_address = Some(address);
            if let Some(source_line) = self.line_index.iter().find(|(l, _)| *l == line).map(|(_, i)| *i) {
                let position = self.statement_position_in_line(line, source_line, address);
                let statement = self
                    .statement_index
                    .iter()
                    .filter(|s| s.source_line == source_line)
                    .nth(position);
                if let Some(statement) = statement {
                    self.current_statement_column = Some((statement.column, statement.end_column));
                }
            }
        }
        Some(line)
    }

    /// The "breakpoint, step, or entry" decision for a single explicit
    /// `stepIn`/`next`/`stepOut`, shared between
    /// [`Purpose::NativeStateAfterStep`] and
    /// [`Purpose::NativeStateAfterStepRetry`] - `line` is whatever
    /// `apply_native_basic_state` already decoded from the read either of
    /// them is answering.
    fn decide_step_stop(&mut self, line: Option<u16>) -> Vec<Value> {
        match line {
            Some(line) => {
                let reason = if self.breakpoints.contains(&line) {
                    "breakpoint"
                }
                else {
                    "step"
                };
                self.report_stopped(reason)
            },
            None => self.report_stopped("entry")
        }
    }

    /// The actual "breakpoint, pause, keep stepping, or entry" decision for
    /// the native step loop, shared between [`Purpose::NativeContinueState`]
    /// and [`Purpose::NativeContinueStateRetry`] - `line` is whatever
    /// `apply_native_basic_state` already decoded from the read either of
    /// them is answering.
    fn decide_continue_stop(&mut self, line: Option<u16>) -> Vec<Value> {
        match line {
            Some(line) if self.breakpoints.contains(&line) => {
                self.resuming_as = None;
                self.report_stopped("breakpoint")
            },
            Some(_) if self.pause_requested => {
                self.resuming_as = None;
                self.report_stopped("pause")
            },
            // Not a line the user cares about: keep stepping.
            Some(_) => {
                let _ =
                    self.send_own("cpclib/basicStep", json!({ "mode": "stmt" }), Purpose::NativeContinueStep);
                Vec::new()
            },
            // Direct mode. Reached from the very first resume, this loop is
            // *typing* RUN, not running the program yet - `cur_linenum`
            // stays direct-mode for the few steps it takes the typed
            // keystrokes to actually reach the interpreter, exactly the way
            // the generic path's own noise-stop fix already distinguishes
            // (`current_line.is_some()`: has a real line ever been seen this
            // session). Reported only once one has - landing back in direct
            // mode with a real line already behind it is the program
            // genuinely ending.
            None if self.current_line.is_none() => {
                let _ =
                    self.send_own("cpclib/basicStep", json!({ "mode": "stmt" }), Purpose::NativeContinueStep);
                Vec::new()
            },
            None => {
                self.resuming_as = None;
                self.report_stopped("entry")
            }
        }
    }

    /// Which statement of `line` (already resolved to its 0-based
    /// `source_line` index into `statement_index`) contains `address` - a
    /// *position* on the line (first, second, third...), not the address
    /// itself.
    ///
    /// Prefers `native_listing`'s own `{addr,end}` ranges for `line` when
    /// available: those come from AMSpiriT Lite's own tokeniser, the same
    /// one that produced `address`, so there is nothing to disagree with -
    /// matching by position rather than address only matters for lining
    /// this session's own `statement_index` (a *different*, independent
    /// tokeniser) up against it afterward.
    ///
    /// Without `native_listing`, falls back to a floor match against
    /// `statement_index`'s own addresses directly - right most of the time,
    /// but a live session showed it is not trustworthy even scoped to one
    /// line: a later statement's `stmt_addr` can land *below* this
    /// tokeniser's computed address for it (the two tokenisers' addresses
    /// drift against each other, growing statement by statement, not just
    /// line by line), silently misattributing it to an earlier statement on
    /// the same line instead - the wrong token, still on the right line.
    fn statement_position_in_line(&self, line: u16, source_line: usize, address: u16) -> usize {
        if let Some(spans) = self.native_listing.as_ref().and_then(|listing| listing.get(&line)) {
            return spans
                .iter()
                .enumerate()
                .rev()
                .find(|(_, (addr, _))| *addr <= address)
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
        self.statement_index
            .iter()
            .filter(|s| s.source_line == source_line)
            .enumerate()
            .filter(|(_, s)| s.address <= address)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Decodes `cpclib/basicListing`'s body into `native_listing`. Absent or
    /// malformed leaves `native_listing` at `None` - `statement_position_in_line`
    /// already falls back cleanly, so a peer that answers this oddly costs
    /// nothing beyond losing the improvement it would have offered.
    fn apply_native_listing(&mut self, message: &Value) {
        let Some(lines) = message
            .get("body")
            .and_then(|b| b.get("lines"))
            .and_then(Value::as_array)
        else {
            return;
        };
        let mut listing = HashMap::new();
        for line in lines {
            let Some(num) = line.get("num").and_then(Value::as_u64) else {
                continue;
            };
            let Some(stmts) = line.get("stmts").and_then(Value::as_array) else {
                continue;
            };
            let spans: Vec<(u16, u16)> = stmts
                .iter()
                .filter_map(|s| {
                    let addr = s.get("addr").and_then(Value::as_u64)? as u16;
                    let end = s.get("end").and_then(Value::as_u64)? as u16;
                    Some((addr, end))
                })
                .collect();
            if !spans.is_empty() {
                listing.insert(num as u16, spans);
            }
        }
        self.native_listing = Some(listing);
    }

    fn on_line_pointer_read(&mut self, message: &Value) -> Vec<Value> {
        let bytes = Self::read_memory_bytes(message);
        let Some(chunk) = bytes.get(0..4) else {
            return Vec::new();
        };
        // PTR_CURRENT_STATEMENT's own value needs no *pointer* dereference,
        // unlike the line-number field below, but it is not the statement's
        // own address either: `Execution.asm` names the ROM variable
        // `address_of_byte_before_current_statement` and says so directly
        // in its own comment ("HL points to byte before first token") -
        // confirmed both for a line's first statement (HL last incremented
        // to the line-number field's high byte, one below the first token)
        // and every later one (HL left on the `StatementSeparator` token
        // itself, again one below the next statement's first token). `+1`
        // is that offset undone, universally, in both cases. `None` (no
        // match) leaves the previous stop's column standing rather than
        // guessing; the very next line-number lookup either confirms this
        // is a real stop (and the caller gets a column - or does not, on a
        // statement layout the launch flow's own tokeniser did not expect)
        // or a filtered-past one, in which case nobody reads it anyway.
        let statement_address = u16::from_le_bytes([chunk[0], chunk[1]]).wrapping_add(1);
        self.current_statement_address = Some(statement_address);
        if let Some(statement) = self
            .statement_index
            .iter()
            .find(|s| s.address == statement_address)
        {
            self.current_statement_column = Some((statement.column, statement.end_column));
        }

        match basic::current_line_number_field_address([chunk[2], chunk[3]]) {
            Some(address) => {
                let _ = self.send_own(
                    "readMemory",
                    json!({ "memoryReference": address_reference(address as u32), "count": 2 }),
                    Purpose::CurrentLineValue
                );
                Vec::new()
            },
            // Direct/immediate mode. Two very different things produce
            // this, and only one is worth reporting: the program ending
            // (or being stepped past its last line) - and the "RUN"
            // command autotype itself just typed, executed as a direct-
            // mode statement before the program has run a single line of
            // its own. Reported live: a spurious "stopped" the instant the
            // program was told to run at all, well before any real
            // breakpoint had a chance to matter. `current_line` being
            // still unset (this session has never seen a real program
            // line yet) is what tells the two apart when nothing else can
            // - direct mode carries no line number either way - except
            // during an active step, where landing back in direct mode is
            // itself the answer regardless of what came before it.
            None => {
                let seen_a_real_line = self.current_line.is_some();
                let actively_stepping = matches!(
                    self.resuming_as,
                    Some(ResumeKind::StepStatement) | Some(ResumeKind::StepLine { .. })
                );
                if seen_a_real_line || actively_stepping {
                    self.resuming_as = None;
                    self.report_stopped("entry")
                }
                else if self.pause_requested {
                    self.resuming_as = None;
                    self.report_stopped("pause")
                }
                else {
                    let _ = self.send_own("continue", json!({ "threadId": THREAD_ID }), Purpose::Plain);
                    Vec::new()
                }
            }
        }
    }

    fn on_line_value_read(&mut self, message: &Value) -> Vec<Value> {
        let bytes = Self::read_memory_bytes(message);
        let Some(line_bytes) = bytes.get(0..2) else {
            return Vec::new();
        };
        let line = basic::decode_line_number([line_bytes[0], line_bytes[1]]);

        let should_stop = match self.resuming_as {
            Some(ResumeKind::StepStatement) => true,
            Some(ResumeKind::StepLine { from_line }) => Some(line) != from_line,
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
        else if self.pause_requested {
            self.resuming_as = None;
            self.report_stopped("pause")
        }
        else {
            // Not a line the user cares about: keep going.
            let _ = self.send_own("continue", json!({ "threadId": THREAD_ID }), Purpose::Plain);
            Vec::new()
        }
    }

    fn report_stopped(&mut self, reason: &str) -> Vec<Value> {
        // A stop for any reason answers whatever pause was pending - an
        // unrelated breakpoint or step landing first is still a stop, and a
        // pause left set past it would fire on the next unrelated `continue`
        // that was never asked to pause at all.
        self.pause_requested = false;
        // Whatever native operation was in flight is answered too - this is
        // the one place every Purpose-tracked native chain converges on a
        // real, editor-visible stop, so it is the one place safe to say the
        // generic unsolicited handling can stop staying quiet about *this*
        // in-flight request. It is not safe yet to stop staying quiet about
        // stragglers this same stop is still causing - see
        // `native_already_stopped`'s own doc comment - so that one is set,
        // not cleared, here.
        self.native_operation_pending = false;
        self.native_already_stopped = true;
        let seq = self.next_seq();
        let mut out = vec![protocol::event(
            "stopped",
            json!({
                "reason": reason,
                "threadId": THREAD_ID,
                "allThreadsStopped": true
            }),
            seq
        )];

        // The standard `stopped` event alone gets a whole-line highlight
        // from VS Code, not the precise statement `stackTrace`'s own
        // column/endColumn describe - the Z80 session's editor-side
        // `revealStop` (`cpclib-vscode/src/debug.ts`) is what actually
        // narrows it to a span, and it is driven by this custom event, not
        // by DAP's native stack-frame columns. `instruction` is left out
        // entirely rather than sent as `null`: it is a Z80-only concept
        // (the resolved byte pattern behind a line), and its absence is
        // exactly what tells `revealStop` there is no hint to show.
        if let Some(line) = self.current_line
            && let Some(source_line) = self
                .line_index
                .iter()
                .find(|(n, _)| *n == line)
                .map(|(_, idx)| *idx as i64 + 1)
        {
            let seq = self.next_seq();
            let (column, end_column) = self.current_statement_column.unwrap_or((1, 1));
            out.push(protocol::event(
                "cpclib/stoppedAt",
                json!({
                    "path": self.source_path.display().to_string(),
                    "line": source_line,
                    "column": column,
                    "endColumn": end_column
                }),
                seq
            ));
        }

        out
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

/// The `Variable.type` DAP field - the editor asked for this outright
/// (`"supportsVariableType": true` in its own `initialize`), and Locomotive
/// BASIC already tells the difference apart by the same type byte
/// [`BasicVariableValue`] itself came from, so there is nothing to infer.
fn variable_type_name(value: &BasicVariableValue) -> &'static str {
    match value {
        BasicVariableValue::Integer(_) => "Integer",
        BasicVariableValue::Real(_) => "Real",
        BasicVariableValue::StringRef { .. } => "String",
        BasicVariableValue::DefFn => "DEF FN",
        BasicVariableValue::Unknown(_) => "Unknown"
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
            &bytes
        )
    }

    /// A session over a peer with native BASIC debugging (AMSpiriT Lite),
    /// rather than 1984js's generic Z80-breakpoint shape.
    fn native_session(source: &str) -> BasicSession<RecordingPeer> {
        let bytes = cpclib_basic::BasicProgram::parse(source)
            .unwrap()
            .as_bytes();
        BasicSession::new(
            RecordingPeer::new().also_supporting(&[
                "cpclib/basicState",
                "cpclib/basicSetBreakpoints",
                "cpclib/basicStep"
            ]),
            PathBuf::from("test.bas"),
            source,
            basic::PROGRAM_START,
            &bytes
        )
    }

    /// Same as [`native_session`], but the peer also offers
    /// `cpclib/basicListing` - for tests exercising `native_listing` itself,
    /// kept separate so every other native test still covers the (still
    /// supported) peer that does not have it.
    fn native_session_with_listing(source: &str) -> BasicSession<RecordingPeer> {
        let bytes = cpclib_basic::BasicProgram::parse(source)
            .unwrap()
            .as_bytes();
        BasicSession::new(
            RecordingPeer::new().also_supporting(&[
                "cpclib/basicState",
                "cpclib/basicSetBreakpoints",
                "cpclib/basicStep",
                "cpclib/basicListing"
            ]),
            PathBuf::from("test.bas"),
            source,
            basic::PROGRAM_START,
            &bytes
        )
    }

    /// Same as [`native_session`], but the peer also offers the four chip
    /// endpoints - for tests exercising the chip scopes, kept separate so
    /// every other native test still covers the (still supported) peer
    /// that does not have them.
    fn native_session_with_chips(source: &str) -> BasicSession<RecordingPeer> {
        let bytes = cpclib_basic::BasicProgram::parse(source)
            .unwrap()
            .as_bytes();
        BasicSession::new(
            RecordingPeer::new().also_supporting(&[
                "cpclib/basicState",
                "cpclib/basicSetBreakpoints",
                "cpclib/basicStep",
                "cpclib/crtc",
                "cpclib/ga",
                "cpclib/psg",
                "cpclib/fdc"
            ]),
            PathBuf::from("test.bas"),
            source,
            basic::PROGRAM_START,
            &bytes
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
    fn attach_sends_initialize_before_attach() {
        // Regression test: 1984js's embedded DAP server refuses every
        // request - `attach` included - with "initialize must be the first
        // request" until it has seen one. Confirmed against a real
        // transcript where this was missing: attach, setInstructionBreakpoints
        // and continue all failed identically from message one.
        let mut session = new_session(SOURCE);
        session.attach().unwrap();

        let commands = session.peer_mut().commands();
        assert_eq!(commands, vec!["initialize", "attach"]);
    }

    #[test]
    fn attach_arms_the_statement_breakpoint_target_not_an_entry_point() {
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);

        // Regression test: this used to arm EXECUTE_LINE_ENTRY itself, which
        // reads PTR_CURRENT_LINE_NUMBER_FIELD one line too early (the ROM
        // updates it a few bytes later in the same routine - see
        // STATEMENT_BREAKPOINT_TARGET's doc comment) - so every breakpoint
        // comparison was off by one line and a real breakpoint could go the
        // whole session without ever matching.
        let armed = session.peer_mut().last("setInstructionBreakpoints").unwrap();
        assert_eq!(
            armed["arguments"]["breakpoints"][0]["instructionReference"],
            address_reference(STATEMENT_BREAKPOINT_TARGET as u32)
        );
    }

    #[test]
    fn a_native_peer_gets_no_generic_breakpoint_armed() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);

        assert!(
            session.peer_mut().last("setInstructionBreakpoints").is_none(),
            "AMSpiriT Lite resolves its own breakpoints; nothing generic to arm"
        );
    }

    #[test]
    fn a_native_peer_gets_the_program_re_injected_through_its_own_tokeniser() {
        // The hand-built launch snapshot alone kept producing corrupted
        // BASIC state on this emulator specifically, even with every known
        // pointer fixed - re-injecting through its own /api/basic sidesteps
        // needing to know why.
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);

        let injected = session.peer_mut().last("cpclib/basicInject").unwrap();
        assert_eq!(injected["arguments"]["source"], SOURCE);
    }

    #[test]
    fn set_breakpoints_arms_them_natively_on_a_peer_that_supports_it() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();

        let armed = session.peer_mut().last("cpclib/basicSetBreakpoints").unwrap();
        assert_eq!(armed["arguments"]["lines"], json!([20]));
    }

    #[test]
    fn a_native_stop_is_reported_from_basic_state_directly() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }

        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicState");
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": 0 } })
        );

        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "breakpoint");
    }

    #[test]
    fn a_native_pause_not_on_a_breakpoint_is_reported_as_pause() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": 0 } })
        );

        assert_eq!(events[0]["body"]["reason"], "pause");
    }

    /// Regression test: a live AMSpiriT Lite session had the peer resume on
    /// its own - a real, unsolicited `continued` event arrived - and nothing
    /// forwarded it, so the editor never found out and sat showing "paused"
    /// while the program was actually running again.
    #[test]
    fn an_unsolicited_continued_event_reaches_the_editor() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);

        let events = session.on_emulator_message(&json!({
            "type": "event",
            "event": "continued",
            "body": { "threadId": 1, "allThreadsContinued": true }
        }));

        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0]["event"], "continued");
        assert_eq!(events[0]["body"]["allThreadsContinued"], true);
    }

    /// Regression test, reported live: the whole debug session appeared to
    /// flicker continuously between paused and running during a single
    /// `Continue` click. `cpclib/basicStep`'s own internal mechanism (which
    /// this session's own Rust-driven loop calls once per statement) fires
    /// the same unsolicited `stopped`/`continued` events as a side effect of
    /// every call, whether or not the statement it lands on is one the user
    /// armed - and both of those unsolicited events used to reach the editor
    /// on top of whatever the loop's own tracked chain already reported,
    /// once per statement stepped.
    #[test]
    fn unsolicited_stopped_and_continued_are_suppressed_while_a_native_operation_is_in_flight() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();

        session
            .on_editor_message(&json!({ "seq": 2, "command": "continue", "arguments": {} }))
            .unwrap();
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicStep");

        // The step loop's own cpclib/basicStep is now in flight - an
        // unsolicited pair arriving before it answers must not reach the
        // editor at all.
        let stopped = session.on_emulator_message(&json!({
            "type": "event",
            "event": "stopped",
            "body": {}
        }));
        assert!(stopped.is_empty(), "{stopped:?}");
        let continued = session.on_emulator_message(&json!({
            "type": "event",
            "event": "continued",
            "body": { "threadId": 1, "allThreadsContinued": true }
        }));
        assert!(continued.is_empty(), "{continued:?}");

        // The loop's own tracked chain still works once it actually answers.
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": 0 } })
        );
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "breakpoint");

        // The stop cleared the flag: an unsolicited event now (nothing of
        // this session's own in flight) reaches the editor again.
        let continued = session.on_emulator_message(&json!({
            "type": "event",
            "event": "continued",
            "body": { "threadId": 1, "allThreadsContinued": true }
        }));
        assert_eq!(continued.len(), 1, "{continued:?}");
    }

    /// Regression test, reported live: one real pause produced three
    /// separate `stopped` events reaching the editor - "2 or 3 unwanted
    /// breakpoints at the beginning". `native_operation_pending` answers for
    /// the *in-flight request*, cleared the moment this session's own
    /// tracked chain reports the stop - but the emulator keeps sending
    /// straggler `stopped` events for that same pause afterwards (its own
    /// `pause` SSE event, `basic_bp`'s own side effect, sometimes both),
    /// arriving once nothing was suppressing them any more. Every one of
    /// them before the next resume must be dropped, not just the first.
    #[test]
    fn stragglers_after_an_already_reported_stop_are_dropped_until_the_next_resume() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();
        session
            .on_editor_message(&json!({ "seq": 2, "command": "continue", "arguments": {} }))
            .unwrap();

        // The loop's own tracked chain reports the one real stop.
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": 0 } })
        );
        assert_eq!(events[0]["event"], "stopped");

        // Two straggler `stopped` events for that exact same pause, arriving
        // after the flag that suppressed the in-flight request has already
        // cleared - neither is a new stop, both must be dropped.
        for _ in 0..2 {
            let stopped = session.on_emulator_message(&json!({
                "type": "event",
                "event": "stopped",
                "body": {}
            }));
            assert!(stopped.is_empty(), "{stopped:?}");
        }

        // A genuinely new stop, after an actual resume, is not suppressed.
        session
            .on_editor_message(&json!({ "seq": 3, "command": "continue", "arguments": {} }))
            .unwrap();
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": 0 } })
        );
        assert_eq!(events[0]["event"], "stopped", "{events:?}");
    }

    /// Regression test: AMSpiriT Lite's own web UI - confirmed live to get
    /// this right - resolves `stmt_addr` by range
    /// (`sa >= a && sa < e`, in its own `basicRenderListing`), not by
    /// comparing it to a statement's own start address, offset or not. A
    /// fixed `+1` (guessing it shared the generic path's own "byte before
    /// the first token" ROM semantics) happened to look plausible in
    /// isolation but was never actually tested against a `stmt_addr` that
    /// lands *inside* a statement rather than exactly on one boundary - the
    /// case that actually matters, since nothing guarantees the emulator
    /// only ever reports the very first byte.
    #[test]
    fn a_native_stop_resolves_the_statement_column_by_range_not_exact_match() {
        let source = "10 a=1:b=2\n";
        let bytes = cpclib_basic::BasicProgram::parse(source).unwrap().as_bytes();
        let index = basic::build_statement_index(&bytes, basic::PROGRAM_START, source);
        assert_eq!(index.len(), 2, "{index:?}");
        let second_statement = index[1].clone();
        assert_ne!(
            second_statement.column, 1,
            "the test fixture must actually exercise a non-first statement"
        );

        // A handful of bytes into the statement, not its very first one -
        // an exact-match lookup (offset or not) would miss this.
        for offset in [0u16, 1, 2] {
            let mut session = native_session(source);
            complete_attach(&mut session);

            session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
            let incoming = session.peer_mut().drain();
            for message in incoming {
                session.on_emulator_message(&message);
            }
            let seq = last_sent_seq(&mut session);
            answer(
                &mut session,
                seq,
                json!({
                    "body": {
                        "cur_linenum": 10,
                        "stmt_addr": second_statement.address + offset
                    }
                })
            );

            let frame = session
                .on_editor_message(&json!({ "seq": 2, "command": "stackTrace", "arguments": {} }))
                .unwrap();
            let frame = &frame[0]["body"]["stackFrames"][0];
            assert_eq!(frame["column"], second_statement.column, "offset {offset}");
            assert_eq!(frame["endColumn"], second_statement.end_column, "offset {offset}");
        }
    }

    /// Regression test, reported live: a `basicState` read taken right after
    /// a fresh `pause` answered with `stmt_addr: 63` against a program
    /// starting at `368` - stale bookkeeping nowhere near this program,
    /// still on `cur_linenum`'s own real, correct line. Trusted anyway, the
    /// floor search in `statement_position_in_line` found nothing at or
    /// below `63` and silently fell back to this line's *first* statement -
    /// "the right line is selected but not the right token." An address
    /// outside the program's own range must be rejected instead: no
    /// specific-token highlight at all (the (1,1) "whole line" fallback)
    /// beats a confidently wrong one.
    #[test]
    fn a_stmt_addr_outside_the_program_is_rejected_not_floored_to_the_first_statement() {
        let source = "10 a=1:b=2\n";
        let mut session = native_session(source);
        complete_attach(&mut session);

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": 63 } })
        );

        let frame = session
            .on_editor_message(&json!({ "seq": 2, "command": "stackTrace", "arguments": {} }))
            .unwrap();
        let frame = &frame[0]["body"]["stackFrames"][0];
        assert_eq!(frame["column"], 1, "{frame:?}");
        assert_eq!(frame["endColumn"], 1, "{frame:?}");
    }

    /// Regression test, reported live as a wrong token being highlighted: a
    /// captured session showed a new line's own first `stmt_addr` regularly
    /// arriving a handful of bytes *below* this session's own tokeniser's
    /// computed start for that line - the two tokenisers do not agree on
    /// addresses byte-for-byte, and an unscoped floor search took that as
    /// license to walk backward into the *previous* line's last statement.
    /// `cur_linenum` is not in question (it is a stored line number, not a
    /// computed address), so the lookup must never cross into a different
    /// line than the one already known to be running - it should land on
    /// this line's own first statement instead, however far off `stmt_addr`
    /// actually is.
    #[test]
    fn a_native_stop_never_attributes_a_statement_to_the_wrong_line() {
        let source = "10 a=1:b=2\n20 c=3\n";
        let bytes = cpclib_basic::BasicProgram::parse(source).unwrap().as_bytes();
        let index = basic::build_statement_index(&bytes, basic::PROGRAM_START, source);
        assert_eq!(index.len(), 3, "{index:?}");
        let line_20_statement = index[2].clone();
        assert_eq!(line_20_statement.source_line, 1);

        let mut session = native_session(source);
        complete_attach(&mut session);

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        // Below this session's own computed start for line 20 - simulating
        // the drift a live session actually showed, not a fixed offset.
        answer(
            &mut session,
            seq,
            json!({
                "body": {
                    "cur_linenum": 20,
                    "stmt_addr": line_20_statement.address.wrapping_sub(5)
                }
            })
        );

        let frame = session
            .on_editor_message(&json!({ "seq": 2, "command": "stackTrace", "arguments": {} }))
            .unwrap();
        let frame = &frame[0]["body"]["stackFrames"][0];
        assert_eq!(frame["line"], 2, "must stay on line 20, not spill back into line 10");
        assert_eq!(frame["column"], line_20_statement.column);
        assert_eq!(frame["endColumn"], line_20_statement.end_column);
    }

    /// The case `native_listing` exists for: even scoped to the right line,
    /// an address-based floor search can still misattribute a later
    /// statement to an earlier one once the two tokenisers' addresses have
    /// drifted enough *within* that line - reported live as "the right line
    /// is selected but not the right token". `native_listing`'s own
    /// `{addr,end}` ranges come from the same tokeniser that produces
    /// `stmt_addr`, so matching against them (by position, then translated
    /// into this session's own `statement_index` at that same position)
    /// sidesteps the drift entirely rather than working around it.
    #[test]
    fn a_native_stop_with_basic_listing_resolves_the_right_statement_despite_drift() {
        let source = "10 a=1:b=2:c=3\n";
        let bytes = cpclib_basic::BasicProgram::parse(source).unwrap().as_bytes();
        let index = basic::build_statement_index(&bytes, basic::PROGRAM_START, source);
        assert_eq!(index.len(), 3, "{index:?}");
        let second_statement = index[1].clone();
        assert_ne!(
            second_statement.column, index[0].column,
            "the test fixture must actually exercise a distinct second statement"
        );

        let mut session = native_session_with_listing(source);
        session.attach().unwrap();
        let attach_seq = last_sent_seq(&mut session);
        answer(&mut session, attach_seq, json!({ "success": true }));

        // Injection's own answer triggers the basic_listing fetch, on a peer
        // that offers it.
        let inject_seq = last_sent_seq(&mut session);
        answer(&mut session, inject_seq, json!({ "success": true }));
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicListing");

        // The second statement's own address, from AMSpiriT Lite's own
        // tokeniser, deliberately *below* this session's own tokeniser's
        // computed address for it - the drift a live session actually
        // showed, but now with the authoritative range to resolve against.
        let drifted_addr = second_statement.address.wrapping_sub(5);
        let listing_seq = last_sent_seq(&mut session);
        answer(
            &mut session,
            listing_seq,
            json!({
                "body": {
                    "lines": [{
                        "addr": 0,
                        "num": 10,
                        "stmts": [
                            { "addr": 100, "end": drifted_addr, "colon": false, "text": "a=1", "vars": [] },
                            { "addr": drifted_addr, "end": drifted_addr + 10, "colon": true, "text": "b=2", "vars": [] },
                            { "addr": drifted_addr + 10, "end": drifted_addr + 20, "colon": true, "text": "c=3", "vars": [] }
                        ]
                    }]
                }
            })
        );

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": drifted_addr } })
        );

        let frame = session
            .on_editor_message(&json!({ "seq": 2, "command": "stackTrace", "arguments": {} }))
            .unwrap();
        let frame = &frame[0]["body"]["stackFrames"][0];
        assert_eq!(
            frame["column"], second_statement.column,
            "must resolve to the second statement, not fall back to the first"
        );
        assert_eq!(frame["endColumn"], second_statement.end_column);
    }

    #[test]
    fn step_in_sends_a_native_statement_step_and_reports_where_it_landed() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);

        session
            .on_editor_message(&json!({ "seq": 1, "command": "stepIn", "arguments": {} }))
            .unwrap();
        let step = session.peer_mut().last("cpclib/basicStep").unwrap();
        assert_eq!(step["arguments"]["mode"], "stmt");

        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        // basic_step's own answer triggers a follow-up basic_state read.
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicState");

        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": 0 } })
        );
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "step");
    }

    /// Regression test, reported live: stepping through a multi-statement
    /// line with "Step Into" never showed the *last* statement highlighted -
    /// the display jumped straight from the second-to-last statement to the
    /// first one of the next line. Root cause matches the `Continue` loop's
    /// own stale-read bug (`a_stale_step_read_that_echoes_the_old_position_is_retried_not_reported_twice`):
    /// `basicState` read right after `basicStep` can still echo the
    /// pre-step position, so stepping *onto* the last statement reported the
    /// second-to-last one again, and the next step (now genuinely landing on
    /// the first statement of the next line) was the first read to ever show
    /// real movement - the last statement's own position was never the one
    /// actually reported.
    #[test]
    fn a_stale_step_in_read_is_retried_not_reported_as_no_movement() {
        let source = "10 a=1:b=2:c=3\n20 d=4\n";
        let bytes = cpclib_basic::BasicProgram::parse(source).unwrap().as_bytes();
        let index = basic::build_statement_index(&bytes, basic::PROGRAM_START, source);
        let second_statement_address = index[1].address; // line 10's "b=2"
        let third_statement_address = index[2].address; // line 10's "c=3"

        let mut session = native_session(source);
        complete_attach(&mut session);

        // A real stop on the second statement first, establishing a known
        // "before" position.
        session
            .on_editor_message(&json!({ "seq": 1, "command": "stepIn", "arguments": {} }))
            .unwrap();
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": second_statement_address } })
        );

        // Step Into again, onto the third (last) statement - but the read
        // right after the step echoes back the exact same position as
        // before it: stale, not a second real visit to the same statement.
        session
            .on_editor_message(&json!({ "seq": 2, "command": "stepIn", "arguments": {} }))
            .unwrap();
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": second_statement_address } })
        );
        assert!(events.is_empty(), "a stale echo must not be reported as a stop: {events:?}");
        assert_eq!(
            session.peer_mut().commands().last().unwrap(),
            "cpclib/basicState",
            "retried with another read, not another step"
        );

        // The retry shows where the step actually landed - the real, last
        // statement, reported normally instead of silently skipped.
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": third_statement_address } })
        );
        assert_eq!(events[0]["event"], "stopped", "{events:?}");
        assert_eq!(events[0]["body"]["reason"], "step");
        assert_eq!(events[1]["body"]["column"], index[2].column);
    }

    #[test]
    fn next_sends_a_native_line_step() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);

        session
            .on_editor_message(&json!({ "seq": 1, "command": "next", "arguments": {} }))
            .unwrap();
        let step = session.peer_mut().last("cpclib/basicStep").unwrap();
        assert_eq!(step["arguments"]["mode"], "line");
    }

    /// Regression test, reported live: with breakpoints armed through
    /// `cpclib/basicSetBreakpoints`, `Continue` still never stopped, and a
    /// live session showed AMSpiriT Lite reporting a spontaneous "breakpoint"
    /// stop with *zero* breakpoints set, and resuming on its own with
    /// nothing having asked it to - proof its own `/api/basic_bp` cannot be
    /// trusted to decide this. `Continue` now drives the same statement
    /// stepper `stepIn` already uses, in a loop, deciding the stop here.
    #[test]
    fn continue_on_a_native_peer_steps_past_lines_it_does_not_care_about() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();

        session
            .on_editor_message(&json!({ "seq": 2, "command": "continue", "arguments": {} }))
            .unwrap();
        let step = session.peer_mut().last("cpclib/basicStep").unwrap();
        assert_eq!(step["arguments"]["mode"], "stmt");

        // Lands on line 10, not the armed line: step again rather than
        // reporting anything.
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": 0 } })
        );
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(
            session.peer_mut().commands().last().unwrap(),
            "cpclib/basicStep",
            "not a match - the loop must keep going, not stop"
        );

        // Lands on line 20, the armed line: report it.
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": 0 } })
        );
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "breakpoint");
    }

    /// Regression test, reported live: `basicState` read right after
    /// `basicStep` can still echo the address from *before* the step - the
    /// emulator's own `basic_bp` SSE event already named the real, later
    /// line, but this read had not caught up yet. Trusted anyway, a
    /// breakpoint the user had already been stopped at once got reported a
    /// *second* time as soon as they clicked Continue, for a step that had
    /// actually already moved on. One extra `basicState` read, triggered
    /// only when nothing appears to have moved, must resolve it instead.
    #[test]
    fn a_stale_step_read_that_echoes_the_old_position_is_retried_not_reported_twice() {
        let bytes = cpclib_basic::BasicProgram::parse(SOURCE).unwrap().as_bytes();
        let index = basic::build_statement_index(&bytes, basic::PROGRAM_START, SOURCE);
        let line_10_address = index[0].address; // "10 PRINT ..."
        let line_20_address = index[1].address; // "20 GOTO 10"

        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();

        // A real stop at the armed line first, establishing a known
        // "before" position.
        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": line_20_address } })
        );

        // Continue: one step runs, but the read right after it echoes back
        // the exact same line and address as before the step - stale, not a
        // second real visit.
        session
            .on_editor_message(&json!({ "seq": 2, "command": "continue", "arguments": {} }))
            .unwrap();
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": line_20_address } })
        );
        assert!(events.is_empty(), "a stale echo must not be reported as a stop: {events:?}");
        assert_eq!(
            session.peer_mut().commands().last().unwrap(),
            "cpclib/basicState",
            "retried with another read, not another step"
        );

        // The retry shows where the step actually landed - real movement
        // (line 20's own `GOTO 10` looping back), reported normally.
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": line_10_address } })
        );
        assert!(events.is_empty(), "line 10 is not armed: {events:?}");
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicStep");
    }

    /// Same race as the generic path's own (`pause_requested`'s doc
    /// comment), for the native loop: a pause requested while a
    /// `cpclib/basicStep` was already in flight used to be undone by the
    /// loop's own next step, sent right behind it.
    #[test]
    fn a_pause_requested_mid_native_continue_stops_instead_of_stepping_again() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);

        session
            .on_editor_message(&json!({ "seq": 1, "command": "continue", "arguments": {} }))
            .unwrap();
        // Captured before `pause` is sent - `pause` is forwarded under the
        // *editor's* own seq (2), which would otherwise shadow this one as
        // "the last thing sent".
        let step_seq = last_sent_seq(&mut session);

        session
            .on_editor_message(&json!({ "seq": 2, "command": "pause", "arguments": { "threadId": 1 } }))
            .unwrap();
        session.peer_mut().push_incoming(json!({
            "type": "response",
            "request_seq": 2,
            "success": true,
            "command": "pause",
            "body": {}
        }));
        for message in session.peer_mut().drain() {
            session.on_emulator_message(&message);
        }

        // The step already in flight lands on line 10 - not a breakpoint,
        // but the pause must win over sending another step.
        answer(&mut session, step_seq, json!({ "success": true }));
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 10, "stmt_addr": 0 } })
        );

        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "pause");
        assert_ne!(
            session.peer_mut().commands().last().unwrap(),
            "cpclib/basicStep",
            "the pause must win, not another step"
        );
    }

    #[test]
    fn an_unverified_breakpoint_is_reported_to_the_user() {
        // The response used to be discarded outright (Purpose::Plain): a
        // peer that answered "verified: false" left every breakpoint and
        // step silently inert for the rest of the session, indistinguishable
        // from the mechanism just not working at all.
        let mut session = new_session(SOURCE);
        session.attach().unwrap();
        let attach_seq = last_sent_seq(&mut session);
        answer(&mut session, attach_seq, json!({ "success": true }));

        let arm_seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            arm_seq,
            json!({
                "success": true,
                "body": {
                    "breakpoints": [{
                        "verified": false,
                        "message": "all 16 breakpoint channels are in use"
                    }]
                }
            })
        );

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["event"], "output");
        let output = events[0]["body"]["output"].as_str().unwrap();
        assert!(output.contains("did not verify"), "{output}");
        assert!(
            output.contains("all 16 breakpoint channels are in use"),
            "{output}"
        );
    }

    #[test]
    fn a_verified_breakpoint_is_silent() {
        let mut session = new_session(SOURCE);
        session.attach().unwrap();
        let attach_seq = last_sent_seq(&mut session);
        answer(&mut session, attach_seq, json!({ "success": true }));

        let arm_seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            arm_seq,
            json!({
                "success": true,
                "body": { "breakpoints": [{ "verified": true }] }
            })
        );

        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn pause_forwards_the_peers_answer_back_to_the_editor() {
        // Regression test, reported live as "pause does not seem to work":
        // `pause` is forwarded to the peer verbatim, under the *editor's*
        // own seq rather than one of this session's own tracked requests -
        // so when the peer's answer came back, `is_our_answer` never
        // recognised it and `on_emulator_message` silently dropped it. The
        // editor's own `pause` request never got a response at all.
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);

        let response = session
            .on_editor_message(&json!({ "seq": 7, "command": "pause", "arguments": { "threadId": 1 } }))
            .unwrap();
        assert!(
            response.is_empty(),
            "no immediate ack - the peer's own answer is what completes it"
        );
        assert_eq!(session.peer_mut().last("pause").unwrap()["seq"], 7);

        // The peer answers using that same editor seq, since the request
        // was forwarded unchanged.
        session.peer_mut().push_incoming(json!({
            "type": "response",
            "request_seq": 7,
            "success": true,
            "command": "pause",
            "body": {}
        }));
        let incoming = session.peer_mut().drain();
        let mut events = Vec::new();
        for message in incoming {
            events.extend(session.on_emulator_message(&message));
        }

        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0]["request_seq"], 7);
        assert_eq!(events[0]["success"], true);
    }

    /// Regression test, reported live: pausing while the generic path's own
    /// "not a breakpoint line, keep going" logic was mid-flight raced that
    /// very `continue` - both land at the emulator, ours last, undoing the
    /// pause before the editor ever saw a stop. No breakpoint is armed here
    /// on purpose, so without the fix every statement boundary auto-continues
    /// forever and `pause` never gets a chance to matter.
    #[test]
    fn a_pause_requested_mid_auto_continue_stops_at_the_next_statement_instead_of_being_raced() {
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);

        session
            .on_editor_message(&json!({ "seq": 1, "command": "pause", "arguments": { "threadId": 1 } }))
            .unwrap();
        session.peer_mut().push_incoming(json!({
            "type": "response",
            "request_seq": 1,
            "success": true,
            "command": "pause",
            "body": {}
        }));
        for message in session.peer_mut().drain() {
            session.on_emulator_message(&message);
        }

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        for message in session.peer_mut().drain() {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        let field_target = 0x9000u16;
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&field_target.to_le_bytes());
        answer(&mut session, seq, read_memory_response(&response_bytes));

        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&20u16.to_le_bytes()));

        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "pause");
        assert_ne!(
            session.peer_mut().commands().last().unwrap(),
            "continue",
            "the pause must win, not another auto-continue"
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
            &bytes
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

    /// Regression test, reported live: a breakpoint armed *before* launch
    /// went uncaught on a native peer's very first run - the program simply
    /// finished, and only an unrelated later manual pause ever stopped
    /// anything. Drives `autotype_run`'s own poll-then-pause chain
    /// (`NativeAwaitRun` -> `NativeAwaitRunState` -> `NativeAwaitRunPaused`
    /// -> `NativeAwaitRunSettled`) end to end, through the "still typing
    /// RUN" direct-mode polls a launch actually has to get through before a
    /// real line ever shows up, confirming the breakpoint set before launch
    /// is honored right where the poll-and-pause lands.
    #[test]
    fn a_breakpoint_armed_before_launch_is_caught_on_the_first_run() {
        let bytes = cpclib_basic::BasicProgram::parse(SOURCE).unwrap().as_bytes();
        let mut session = BasicSession::new(
            RecordingPeer::new().also_supporting(&[
                "cpclib/basicState",
                "cpclib/basicSetBreakpoints",
                "cpclib/basicStep",
                "cpclib/autotype"
            ]),
            PathBuf::from("test.bas"),
            SOURCE,
            basic::PROGRAM_START,
            &bytes
        );
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 2 }] } // line 20
            }))
            .unwrap();
        session
            .on_editor_message(&json!({ "seq": 2, "command": "configurationDone", "arguments": {} }))
            .unwrap();

        // The bare `continue` that unfreezes the machine so RUN can be typed.
        let continue_seq = last_sent_seq(&mut session);
        answer(&mut session, continue_seq, json!({ "success": true }));
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/autotype");

        // Autotype's own answer must now kick off the poll loop, not stop
        // here trusting /api/basic_bp.
        let autotype_seq = last_sent_seq(&mut session);
        answer(&mut session, autotype_seq, json!({ "success": true }));
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicState");

        // A couple of polls still typing RUN: direct mode, nothing reported,
        // and critically no `basicStep` in between - stepping this window is
        // exactly what used to stop RUN from ever registering.
        for _ in 0..2 {
            let seq = last_sent_seq(&mut session);
            let events = answer(&mut session, seq, json!({ "body": { "cur_linenum": 65535 } }));
            assert!(events.is_empty(), "{events:?}");
            assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicState");
        }

        // A real line at last: stop free-running before doing anything else.
        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, json!({ "body": { "cur_linenum": 20 } }));
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(session.peer_mut().commands().last().unwrap(), "pause");

        let pause_seq = last_sent_seq(&mut session);
        let events = answer(&mut session, pause_seq, json!({ "success": true }));
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicState");

        // Only now does this session start caring about breakpoints: line
        // 20, the one armed before launch, is honored on this very first
        // real line instead of being skipped past.
        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({ "body": { "cur_linenum": 20, "stmt_addr": 0 } })
        );
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "breakpoint");
    }

    /// Regression test, reported live: `pause` is itself asynchronous - the
    /// state the poll loop saw right before sending it (a line with no
    /// breakpoint on it) was already stale by the time the machine actually
    /// stopped (landed on a different, later line instead). Feeding that into
    /// `NativeContinueState`'s own "not a line I care about, keep stepping"
    /// logic reproduced the exact hang this whole chain exists to avoid: it
    /// restarted `cpclib/basicStep`, which ran the program straight back
    /// into direct mode without ever reporting a sensible stop. This
    /// confirms the fix - report a stop right where the pause landed,
    /// breakpoint or not, with no `basicStep` sent at any point in the
    /// chain.
    #[test]
    fn a_pause_after_run_lands_past_the_line_the_poll_saw_and_still_stops_cleanly() {
        let bytes = cpclib_basic::BasicProgram::parse(SOURCE).unwrap().as_bytes();
        let mut session = BasicSession::new(
            RecordingPeer::new().also_supporting(&[
                "cpclib/basicState",
                "cpclib/basicSetBreakpoints",
                "cpclib/basicStep",
                "cpclib/autotype"
            ]),
            PathBuf::from("test.bas"),
            SOURCE,
            basic::PROGRAM_START,
            &bytes
        );
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({ "seq": 1, "command": "configurationDone", "arguments": {} }))
            .unwrap();

        let continue_seq = last_sent_seq(&mut session);
        answer(&mut session, continue_seq, json!({ "success": true }));
        let autotype_seq = last_sent_seq(&mut session);
        answer(&mut session, autotype_seq, json!({ "success": true }));

        // The poll sees a real line (10, no breakpoint on it) and asks to
        // pause.
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, json!({ "body": { "cur_linenum": 10 } }));
        assert_eq!(session.peer_mut().commands().last().unwrap(), "pause");

        // By the time the pause actually lands, the machine has moved on to
        // a different, also non-breakpoint line - not the one that
        // triggered the pause.
        let pause_seq = last_sent_seq(&mut session);
        answer(&mut session, pause_seq, json!({ "success": true }));
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicState");

        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, json!({ "body": { "cur_linenum": 20 } }));

        // A stop is reported right here - no `cpclib/basicStep` sent at any
        // point in this chain.
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "entry");
        assert!(
            !session.peer_mut().commands().contains(&"cpclib/basicStep".to_string()),
            "{:?}",
            session.peer_mut().commands()
        );
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
            &program_bytes
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

        // First round trip: PTR_CURRENT_STATEMENT (a statement address, not
        // under test here - out of range on purpose so it never happens to
        // collide with a real entry) followed immediately by
        // PTR_CURRENT_LINE_NUMBER_FIELD -> a pointer, both in one 4-byte
        // read.
        let seq = last_sent_seq(session);
        let field_target = 0x9000u16;
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&field_target.to_le_bytes());
        answer(session, seq, read_memory_response(&response_bytes));

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
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "breakpoint");
        // The custom event editor-side highlighting is driven by - see
        // `report_stopped`'s doc comment for why the standard `stopped`
        // event's own column/endColumn are not enough on their own.
        assert_eq!(events[1]["event"], "cpclib/stoppedAt");
        assert_eq!(events[1]["body"]["line"], 2);
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
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&field_target.to_le_bytes());
        answer(&mut session, seq, read_memory_response(&response_bytes));
        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&10u16.to_le_bytes()));

        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "step");
        assert_eq!(events[1]["event"], "cpclib/stoppedAt");
    }

    #[test]
    fn step_in_stops_at_every_statement_on_a_multi_statement_line() {
        // "stepIn stops at the very next statement, whatever line it is on" -
        // reported directly: `next` executes a multi-statement line whole,
        // which is expected, but `stepIn` did too, which is not.
        let source = "10 a=1:b=2:c=3\n20 d=4\n";
        let bytes = cpclib_basic::BasicProgram::parse(source).unwrap().as_bytes();
        let mut session = BasicSession::new(
            RecordingPeer::new(),
            PathBuf::from("test.bas"),
            source,
            basic::PROGRAM_START,
            &bytes
        );
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({ "seq": 1, "command": "stepIn", "arguments": {} }))
            .unwrap();

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }

        // The hit reports line 10 again (its second statement) - stepIn
        // must stop here even though the *line* has not changed.
        let seq = last_sent_seq(&mut session);
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&0x9000u16.to_le_bytes());
        answer(&mut session, seq, read_memory_response(&response_bytes));
        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&10u16.to_le_bytes()));

        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "step");
        assert_eq!(events[1]["event"], "cpclib/stoppedAt");
    }

    #[test]
    fn next_skips_every_statement_on_the_current_line_and_stops_only_on_a_new_one() {
        let source = "10 a=1:b=2:c=3\n20 d=4\n";
        let bytes = cpclib_basic::BasicProgram::parse(source).unwrap().as_bytes();
        let mut session = BasicSession::new(
            RecordingPeer::new(),
            PathBuf::from("test.bas"),
            source,
            basic::PROGRAM_START,
            &bytes
        );
        // A real "next" always follows a previous stop - establish current_line
        // = 10 first, the same way stepping through the debugger would.
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 1 }] } // "10 a=1:b=2:c=3"
            }))
            .unwrap();
        stop_at_line(&mut session, 10);

        session
            .on_editor_message(&json!({ "seq": 2, "command": "next", "arguments": {} }))
            .unwrap();
        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }

        // First hit after "next": still line 10 (its second statement) -
        // must NOT stop, matching "step over the whole line".
        let seq = last_sent_seq(&mut session);
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&0x9000u16.to_le_bytes());
        answer(&mut session, seq, read_memory_response(&response_bytes));
        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&10u16.to_le_bytes()));
        assert!(events.is_empty(), "{events:?}");

        // Silently continuing sent its own "continue"; the peer, having run
        // on, hits the breakpoint again for line 20 - which "next" does
        // stop at.
        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&0x9100u16.to_le_bytes());
        answer(&mut session, seq, read_memory_response(&response_bytes));
        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&20u16.to_le_bytes()));
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "step");
        assert_eq!(events[1]["event"], "cpclib/stoppedAt");
    }

    #[test]
    fn direct_mode_after_the_program_has_run_is_reported_as_entry() {
        // Realistic scenario: the program has already run at least one
        // real line, then returns to direct mode - a genuine "the program
        // ended" transition, worth reporting.
        let mut session = new_session(SOURCE);
        session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "setBreakpoints",
                "arguments": { "breakpoints": [{ "line": 1 }] } // line 10
            }))
            .unwrap();
        stop_at_line(&mut session, 10);
        session
            .on_editor_message(&json!({ "seq": 2, "command": "next", "arguments": {} }))
            .unwrap();

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let seq = last_sent_seq(&mut session);
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&0u16.to_le_bytes());
        let events = answer(&mut session, seq, read_memory_response(&response_bytes));

        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0]["event"], "stopped");
        assert_eq!(events[0]["body"]["reason"], "entry");
    }

    #[test]
    fn direct_mode_before_the_program_has_run_a_line_is_not_reported() {
        // Regression test, reported live: the very first statement
        // breakpoint hit of a session is the autotyped "RUN" command
        // itself, executed as a direct-mode statement before the program
        // has run a single line of its own - PTR_CURRENT_LINE_NUMBER_FIELD
        // reads 0 (direct mode) exactly the same way "the program just
        // ended" does. Reporting it unconditionally produced a spurious
        // "stopped" the instant the program was told to run at all, before
        // any real breakpoint had a chance to matter - "a breakpoint
        // raised just before the first instruction".
        let mut session = new_session(SOURCE);
        complete_attach(&mut session);
        session
            .on_editor_message(&json!({ "seq": 1, "command": "configurationDone", "arguments": {} }))
            .unwrap();

        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }
        let commands_before = session.peer_mut().commands().len();
        let seq = last_sent_seq(&mut session);
        let mut response_bytes = 0xffffu16.to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&0u16.to_le_bytes());
        let events = answer(&mut session, seq, read_memory_response(&response_bytes));

        assert!(events.is_empty(), "{events:?}");
        // Silently resumed rather than left hanging.
        let commands = session.peer_mut().commands();
        assert!(commands.len() > commands_before);
        assert_eq!(commands.last().unwrap(), "continue");
    }

    #[test]
    fn scopes_offers_both_variables_and_workspace() {
        let mut session = new_session(SOURCE);
        let response = session
            .on_editor_message(&json!({ "seq": 1, "command": "scopes", "arguments": {} }))
            .unwrap();
        let scopes = &response[0]["body"]["scopes"];
        assert_eq!(scopes[0]["name"], "Variables");
        assert_eq!(scopes[0]["variablesReference"], VARIABLES_REFERENCE);
        // VS Code only auto-highlights a changed value between stops for a
        // scope hinted this way - reported live as missing entirely.
        assert_eq!(scopes[0]["presentationHint"], "registers");
        assert_eq!(scopes[1]["name"], "Workspace");
        assert_eq!(scopes[1]["variablesReference"], WORKSPACE_REFERENCE);
    }

    /// Requested live: chip state visible from the BASIC debugger, to
    /// diagnose a screen/timing problem the BASIC variables alone cannot
    /// explain - a broken snapshot, a CRTC left in a bad state.
    #[test]
    fn scopes_offers_chip_panes_on_a_native_peer_that_supports_them() {
        let mut session = native_session_with_chips(SOURCE);
        complete_attach(&mut session);
        let response = session
            .on_editor_message(&json!({ "seq": 1, "command": "scopes", "arguments": {} }))
            .unwrap();
        let scopes = response[0]["body"]["scopes"].as_array().unwrap();
        let names: Vec<&str> = scopes.iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["Variables", "Workspace", "CRTC", "Gate Array", "PSG", "Disc"]);
        let crtc = scopes.iter().find(|s| s["name"] == "CRTC").unwrap();
        assert_eq!(crtc["variablesReference"], crate::inspect::CRTC_REFERENCE);
        // Opt-in only: this session has already seen what hammering AMSpiriT
        // Lite's HTTP server with requests nobody asked for does to it.
        assert_eq!(crtc["expensive"], true);
    }

    /// A native peer without the dedicated per-chip endpoints (an older
    /// build, or a future native peer that never grows them) still works -
    /// just without the extra panes, rather than advertising a scope
    /// nothing can answer.
    #[test]
    fn scopes_omits_chip_panes_on_a_native_peer_without_them() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        let response = session
            .on_editor_message(&json!({ "seq": 1, "command": "scopes", "arguments": {} }))
            .unwrap();
        let scopes = response[0]["body"]["scopes"].as_array().unwrap();
        assert_eq!(scopes.len(), 2, "{scopes:?}");
    }

    /// Each chip has its own endpoint and its own round trip - no shared
    /// `machineState` snapshot to batch behind, unlike the Z80 session's own
    /// chip scopes.
    #[test]
    fn a_chip_scope_variables_request_fetches_its_own_endpoint_and_answers_the_right_request() {
        let mut session = native_session_with_chips(SOURCE);
        complete_attach(&mut session);
        let response = session
            .on_editor_message(&json!({
                "seq": 9,
                "command": "variables",
                "arguments": { "variablesReference": crate::inspect::PSG_REFERENCE }
            }))
            .unwrap();
        assert!(response.is_empty(), "{response:?}");
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/psg");

        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, json!({ "body": { "regs": [1, 2, 3] } }));
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0]["command"], "variables");
        assert_eq!(events[0]["request_seq"], 9);
        assert!(events[0]["body"]["variables"].is_array());
    }

    #[test]
    fn workspace_variables_on_a_generic_peer_reads_only_arrays_start() {
        let mut session = new_session(SOURCE);
        let response = session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "variables",
                "arguments": { "variablesReference": WORKSPACE_REFERENCE }
            }))
            .unwrap();
        assert!(response.is_empty());

        let armed = session.peer_mut().last("readMemory").unwrap();
        assert_eq!(
            armed["arguments"]["memoryReference"],
            address_reference(basic::PTR_ARRAYS_START as u32)
        );

        let seq = last_sent_seq(&mut session);
        let events = answer(&mut session, seq, read_memory_response(&0x200u16.to_le_bytes()));

        assert_eq!(events.len(), 1, "{events:?}");
        let vars = &events[0]["body"]["variables"];
        let names: Vec<&str> = vars.as_array().unwrap().iter().map(|v| v["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"Program size"), "{names:?}");
        assert!(names.contains(&"Variables zone"), "{names:?}");
        assert!(names.contains(&"BASIC version"), "{names:?}");
    }

    #[test]
    fn workspace_variables_on_a_native_peer_uses_basic_state_directly() {
        let mut session = native_session(SOURCE);
        complete_attach(&mut session);
        let response = session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "variables",
                "arguments": { "variablesReference": WORKSPACE_REFERENCE }
            }))
            .unwrap();
        assert!(response.is_empty());
        assert_eq!(session.peer_mut().commands().last().unwrap(), "cpclib/basicState");

        let seq = last_sent_seq(&mut session);
        let events = answer(
            &mut session,
            seq,
            json!({
                "body": {
                    "txttop": 0x0200,
                    "vartop": 0x0300,
                    "arrend": 0x0300,
                    "var_size": 256,
                    "prog_size": 144,
                    "basic_ver": 11,
                    "stmt_addr": 0x0170
                }
            })
        );

        assert_eq!(events.len(), 1, "{events:?}");
        let vars = &events[0]["body"]["variables"];
        let names: Vec<&str> = vars.as_array().unwrap().iter().map(|v| v["name"].as_str().unwrap()).collect();
        for expected in [
            "Program size",
            "Variables zone",
            "Arrays zone",
            "Free RAM",
            "BASIC version",
            "Current instruction"
        ] {
            assert!(names.contains(&expected), "missing {expected:?} in {names:?}");
        }
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
    fn stack_trace_points_at_the_statement_that_actually_ran_not_the_lines_first_one() {
        // "50 sp=0 : px=320 : py=300" is one BASIC line and several stops -
        // this is what tells them apart, matching the Z80 session's own
        // per-instruction highlight instead of only ever pointing at the
        // start of the line.
        let source = "10 a=1:b=2\n";
        let bytes = cpclib_basic::BasicProgram::parse(source).unwrap().as_bytes();
        let index = basic::build_statement_index(&bytes, basic::PROGRAM_START, source);
        assert_eq!(index.len(), 2, "{index:?}");
        let second_statement = index[1].clone();

        let mut session = BasicSession::new(
            RecordingPeer::new(),
            PathBuf::from("test.bas"),
            source,
            basic::PROGRAM_START,
            &bytes
        );
        complete_attach(&mut session);
        session.peer_mut().push_incoming(json!({ "type": "event", "event": "stopped", "body": {} }));
        let incoming = session.peer_mut().drain();
        for message in incoming {
            session.on_emulator_message(&message);
        }

        let seq = last_sent_seq(&mut session);
        // PTR_CURRENT_STATEMENT holds the byte *before* the statement's own
        // first token (see `on_line_pointer_read`'s doc comment) - `- 1` is
        // what a real ROM would actually report here.
        let mut response_bytes = second_statement.address.wrapping_sub(1).to_le_bytes().to_vec();
        response_bytes.extend_from_slice(&0x9000u16.to_le_bytes());
        answer(&mut session, seq, read_memory_response(&response_bytes));
        let seq = last_sent_seq(&mut session);
        answer(&mut session, seq, read_memory_response(&10u16.to_le_bytes()));

        let response = session
            .on_editor_message(&json!({ "seq": 2, "command": "stackTrace", "arguments": {} }))
            .unwrap();
        let frame = &response[0]["body"]["stackFrames"][0];
        assert_eq!(frame["column"], second_statement.column);
        assert_eq!(frame["endColumn"], second_statement.end_column);
        assert_ne!(
            second_statement.column, 1,
            "the test fixture must actually exercise a non-first statement"
        );
    }

    #[test]
    fn a_refused_read_answers_with_an_empty_list_instead_of_panicking() {
        // Regression test for a real crash: a `readMemory` the peer refused
        // ("The CPC must be stopped for this request" - the editor asked
        // for variables while a step was still in flight, a real DAP
        // client's own doing, not a bug in this crate) came back with no
        // `body.data`, which `read_memory_bytes` turns into an empty `Vec`
        // rather than `None`. Indexing into that directly used to panic and
        // take the whole adapter process down with it.
        let mut session = new_session(SOURCE);
        let response = session
            .on_editor_message(&json!({
                "seq": 1,
                "command": "variables",
                "arguments": { "variablesReference": VARIABLES_REFERENCE }
            }))
            .unwrap();
        assert!(response.is_empty());

        let seq = session.own_requests.keys().min().copied().unwrap();
        let refused = json!({
            "type": "response",
            "request_seq": seq,
            "success": false,
            "message": "notStopped",
            "body": { "error": { "format": "The CPC must be stopped for this request" } }
        });
        session.peer_mut().push_incoming(refused.clone());
        let mut events = Vec::new();
        for message in session.peer_mut().drain() {
            events.extend(session.on_emulator_message(&message));
        }
        // The second (storage) read refused the same way.
        let seq = session.own_requests.keys().min().copied().unwrap();
        let mut second = refused;
        second["request_seq"] = json!(seq);
        session.peer_mut().push_incoming(second);
        for message in session.peer_mut().drain() {
            events.extend(session.on_emulator_message(&message));
        }

        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0]["body"]["variables"], json!([]));
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
        assert_eq!(vars[0]["value"], "42 (Integer)");
        assert_eq!(vars[0]["type"], "Integer");
    }
}

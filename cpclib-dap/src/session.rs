//! The translation itself.
//!
//! Everything here exists because the editor and the emulator disagree about
//! what a location is. The editor says "line 42 of main.asm"; the emulator says
//! `0x4021`. The [`SourceMap`] knows both, and this turns each side's messages
//! into the other's vocabulary.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cpclib_project::srcmap::{AddressResolution, SourceMap};
use serde_json::{Value, json};

use crate::peer::{DapPeer, LineAtPc};

/// The Z80 has one thread of execution, and the emulator numbers it 1.
pub const THREAD_ID: i64 = 1;

/// The reference under which the decoded flags expand into one row per bit.
const FLAGS_REFERENCE: i64 = 0x7C00_0003;

/// Where our own requests to the emulator start numbering.
///
/// The editor and this adapter each number their requests from 1, and both
/// conversations run over the same session. A response carries only the seq it
/// answers, so an emulator reply to *our* `attach` (request_seq 2) is
/// indistinguishable from a reply to the *editor's* request 2 - which is its
/// `launch`. Forwarding one to the other tells the editor its launch was
/// answered by an attach, and the session falls apart.
///
/// Numbering ours from far away keeps the two readable in a transcript; not
/// forwarding their responses at all is what actually makes it correct.
const OWN_REQUEST_BASE: i64 = 1_000_000;

/// Where the frame ids we invent start.
///
/// The emulator numbers its own frames `stopEpoch * 16 + 1` and refuses any
/// other; keeping ours far away means a `scopes` request for a reconstructed
/// frame is rejected rather than silently answered with frame 0's registers,
/// which are not that frame's registers at all.
const SYNTHETIC_FRAME_BASE: i64 = 0x4000_0000;

/// Where the variable references for those frames start.
const SYNTHETIC_SCOPE_BASE: i64 = 0x4800_0000;

/// How many ids each synthetic base owns.
///
/// Bounded rather than open-ended: the flag and chip scopes this adapter also
/// answers for live at `0x7C00_000x`, which is *above* the synthetic base, so a
/// `>=` test would swallow them and answer a register pane with a stack frame.
/// One walk is capped at [`crate::callstack::MAX_STACK_ITEMS`] frames, so that
/// is exactly how many ids are needed.
const SYNTHETIC_RANGE: i64 = crate::callstack::MAX_STACK_ITEMS as i64;

/// How many bytes to compare when working out which page is selected.
///
/// Enough that two pages holding different code differ somewhere, short enough
/// that it is one small read per stop and only for programs that need it.
const PAGE_PROBE_BYTES: usize = 16;

/// Why an outer frame has no registers, said once where the user is looking.
///
/// This is a hard limit, not an omission. A Z80 `CALL` pushes the return
/// address and nothing else: the caller's `HL` is whatever the callee has since
/// done to `HL`. Recovering it would mean the emulator recording the whole
/// register file at every `CALL` and releasing it at the matching `RET` - which
/// is the right design, and which no emulator here offers: 1984js's debug API
/// (`_poc_debug_*`) exposes registers only for the *current* instruction, and
/// its DAP session refuses `scopes` for any frame but the innermost, which is
/// exactly the "Stack frame reference has expired" error this replaces.
///
/// So what an outer frame can show is what the *stack* still holds - the return
/// address, where the call was made, and the words that frame pushed - and it
/// says plainly that the registers are gone rather than showing stale ones from
/// the innermost frame, which would be worse than showing nothing.
const OUTER_FRAME_NOTE: &str = "the CPU's registers belong to the innermost frame only; \
                                a Z80 CALL pushes the return address and nothing else, so \
                                an outer frame's register values no longer exist anywhere \
                                to be read";

/// What `-help` prints.
const CONSOLE_HELP: &str = "\
CPC debug console commands:
  -mv <address|label> [count]   open a memory view (count defaults to 64)
  -dv [address|label] [count]   disassemble memory (defaults to PC, and then
                                follows it); rows link to your source
  -chips                        CRTC, Gate Array, PSG and PPI, with counters
  -timer [add|reset|rm] [name]  stopwatches in NOPs; bare -timer lists them
  -help                         this list

Anything not starting with `-` is read as a label: `animation_state` shows the
byte there, `animation_state,w` the word.";
use crate::protocol::{self, address_reference, parse_address_reference};

/// A breakpoint the assembled program asked for.
#[derive(Debug, Clone)]
struct ProgramBreakpoint {
    address: u32,
    /// The watch this is, when it asks to break on memory access rather than
    /// execution.
    /// What the source asked for that a plain address cannot express is
    /// reported when the breakpoint is adopted and not kept: the notice is
    /// said once, at the start, and repeating it on every stop would be noise.
    watch: Option<WatchRequest>,
    /// Dropped after the first stop.
    ///
    /// `stopOnEntry` means *entry*, once. Left armed it would stop again every
    /// time a main loop came back past the entry address, and it would hold one
    /// of the emulator's scarce breakpoint channels for the whole session.
    one_shot: bool,
    /// Where the `BREAKPOINT` directive is written, when the assembler knew.
    ///
    /// Not the same place as `address`: a directive arms the instruction that
    /// follows it, so one written inside a macro body stops the program
    /// wherever the macro was *used*.
    written_at: Option<cpclib_asm::assembler::delayed_command::BreakpointSource>
}

/// A memory watch, in the form the emulator's watch slots take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchRequest {
    pub address: u32,
    pub read: bool,
    pub write: bool,
    pub label: String
}

/// A request this adapter sent, and what its answer is for.
///
/// The command alone is no longer enough to route an answer: `readMemory` is
/// asked both to satisfy a watch expression and to fetch the stack, and giving
/// one answer to the other's handler produces a plausible-looking wrong result
/// rather than an error.
#[derive(Debug, Clone)]
struct OwnRequest {
    command: String,
    purpose: Purpose
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Purpose {
    /// Housekeeping - `initialize`, `attach`, breakpoints. The answer is
    /// consumed and that is all.
    Plain,
    /// The bytes a watch expression is waiting for.
    WatchRead,
    /// Arming the emulator's write-watch channels.
    WatchArm,
    /// Bytes for the memory view.
    MemoryView,
    /// Bytes at `PC`, to work out which page is selected.
    PageProbe,
    /// Bytes at `PC`, to decode the instruction the machine really holds
    /// there.
    StopHint,
    /// Instructions for the disassembly view.
    DisassemblyView,
    /// A snapshot of the whole machine, for the chip scopes.
    MachineState,
    /// Bytes for the editor's own disassembly view.
    EditorDisassembly,
    /// Finding the register scope, on the way to `SP`.
    StackScopes,
    /// Reading `SP` out of the register scope.
    StackRegisters,
    /// The stack itself.
    StackRead
}

/// An editor `stackTrace` held while the stack is fetched.
///
/// The emulator answers with one frame, the program counter. Everything above
/// it takes three more round trips - scopes, registers, memory - so its answer
/// waits here rather than going out incomplete and being corrected afterwards.
#[derive(Debug, Clone)]
struct PendingStack {
    /// The emulator's own answer, which is frame 0 and stays frame 0.
    response: Value,
    /// Where `SP` pointed, once known.
    sp: Option<u16>
}

/// A stop whose instruction hint is waiting for the bytes at `PC`.
///
/// The reveal is not held up for it: the editor is told where the program
/// stopped as soon as the stack trace is annotated, and the hint follows as a
/// message of its own a round trip later.
#[derive(Debug, Clone)]
struct PendingStopHint {
    path: String,
    line: i64,
    /// The instruction's own columns, carried so the hint can be placed just
    /// after it.
    ///
    /// Without these the extension can only fall back to the end of the line,
    /// which looks right exactly when the executing instruction happens to be
    /// the last one on it - and wrong on every other statement of a
    /// `ld hl,(x) : ld de,(y)` line. They are already known: they are the same
    /// columns the stop selects.
    column: i64,
    end_column: i64,
    /// The source line as written, to compare the decoded instruction against.
    written: Option<String>,
    address: u16,
    /// The read this is waiting for. An editor that asks for a stack trace
    /// twice puts two reads in flight, and the first answer must not be
    /// dressed up as the second one's.
    request_seq: i64
}

/// A watch waiting for the emulator to report the bytes at its label.
#[derive(Debug, Clone)]
struct PendingWatch {
    request: Value,
    name: String,
    address: u32,
    width: usize
}

/// A stopwatch counting the NOPs the program spends.
///
/// A CPC demo is a budget in NOPs per raster line, and the question "how long
/// did that take" is asked constantly. The emulator cannot answer it: its whole
/// debug API is registers, memory, breakpoints and stepping - there is no
/// cycle, instruction or frame counter anywhere in it (`_poc_tape_counter` is a
/// tape position). So the cost is accumulated here, from the instructions the
/// program is seen to execute.
///
/// That is exact while stepping, and only while stepping. Between a `continue`
/// and the next stop the program runs unobserved, and nothing on this side can
/// say for how long - so a timer that spans a free run says so rather than
/// reporting a total it cannot stand behind. Making it exact needs one call
/// from the emulator: the NOPs elapsed since the last stop.
#[derive(Debug, Clone)]
struct Timer {
    name: String,
    nops: u64,
    /// Cleared the first time the program runs on unobserved. Once false the
    /// total is a floor, not a measurement.
    exact: bool
}

/// A memory view waiting for the bytes it asked for.
#[derive(Debug, Clone)]
struct PendingMemoryView {
    /// The console line that asked, when a person asked. `None` for the
    /// refresh a stop triggers: the panel updates itself, and printing a
    /// receipt for something nobody typed would fill the console with noise
    /// while stepping.
    request: Option<Value>,
    label: Option<String>,
    address: u32
}

/// A `-dv` waiting for its instructions.
#[derive(Debug, Clone)]
struct PendingDisassembly {
    /// The console line that asked, when a person asked. `None` for the
    /// refresh a stop triggers.
    request: Option<Value>,
    label: Option<String>,
    address: u32
}

/// Where a disassembly view is looking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisassemblyAnchor {
    /// Wherever the program is now. Re-read on every stop, which is the whole
    /// point: it lets you watch what is really executing beside the source.
    ProgramCounter,
    /// A place you asked for by name or address. It stays put - a view of
    /// `draw_sprite` that wanders off is not a view of `draw_sprite`.
    Fixed(u32)
}

/// A disassembly view that is open, so it can be kept current.
#[derive(Debug, Clone)]
struct OpenDisassemblyView {
    anchor: DisassemblyAnchor,
    count: i64,
    label: Option<String>,
    /// Where it was last fetched from, so a `PC`-following view is only
    /// re-asked when `PC` has actually moved.
    fetched_at: Option<u32>
}

/// A memory view that is open, so it can be kept current.
#[derive(Debug, Clone)]
struct OpenMemoryView {
    address: u32,
    count: usize,
    label: Option<String>,
    /// What it showed last time, so the bytes that moved can be marked. A
    /// memory view during stepping is watched for *changes*; finding them by
    /// eye across a screen of hex is the part worth automating.
    previous: Vec<u8>
}

/// A source breakpoint the editor asked for, and where it ended up.
#[derive(Debug, Clone)]
struct PlacedBreakpoint {
    /// The id handed to the editor, stable for the life of the session.
    id: i64,
    /// The line the editor asked for.
    requested_line: u32,
    /// The line it actually landed on, after sliding past lines with no code.
    line: u32,
    address: Option<u32>,
    /// Why it could not be placed, when it could not.
    message: Option<String>
}

/// The translating session.
pub struct Session<P: DapPeer> {
    peer: P,
    map: SourceMap,
    /// Source breakpoints per file. `setBreakpoints` is *per file* while the
    /// emulator's `setInstructionBreakpoints` replaces one global set, so every
    /// per-file update has to re-send the union of all files.
    breakpoints: HashMap<PathBuf, Vec<PlacedBreakpoint>>,
    next_breakpoint_id: i64,
    seq: i64,
    /// Breakpoints the *program* asked for, via `BREAKPOINT` directives.
    ///
    /// Kept apart from the editor's, because the editor does not know about
    /// them and a red-dot change in one file must not wipe a directive in
    /// another.
    ///
    /// It does **not** follow that the editor cannot clear one. In this
    /// extension the gutter red dot *is* a `BREAKPOINT` directive - clicking
    /// the gutter writes one into the source - so a directive that was present
    /// at build time and a red dot the user has since removed are the same
    /// breakpoint. Treating them as untouchable made every gutter breakpoint
    /// unremovable for the life of the session. See `suppressed`.
    program_breakpoints: Vec<ProgramBreakpoint>,
    /// Program breakpoints the editor has explicitly taken back.
    ///
    /// Recorded by address rather than removed, so that re-adding the red dot
    /// brings the breakpoint back without needing the program reassembled.
    suppressed: std::collections::HashSet<u32>,
    /// Requests this adapter originated, and what each answer is for.
    ///
    /// Their answers belong to us, never to the editor.
    own_requests: HashMap<i64, OwnRequest>,
    /// The next seq for a request of our own.
    own_seq: i64,
    /// Watch expressions waiting on a memory read.
    pending_watches: Vec<PendingWatch>,
    /// Memory views waiting on the same.
    pending_memory_views: Vec<PendingMemoryView>,
    /// The memory view that is open, if one is.
    open_memory_view: Option<OpenMemoryView>,
    /// `-dv` requests waiting on the emulator.
    pending_disassembly: Vec<PendingDisassembly>,
    /// The editor's own `disassemble` requests, waiting for their bytes:
    /// the request, where the read starts, the address it is anchored on, how
    /// many instructions of context precede it, and how many are wanted.
    pending_editor_disassembly: Vec<(Value, u32, u32, usize, i64)>,
    /// The disassembly view that is open, if one is.
    open_disassembly_view: Option<OpenDisassemblyView>,
    /// The `F` byte from the last register read, so expanding the flags row
    /// does not need a second round trip to an emulator that may since have
    /// resumed.
    last_flags: u8,
    /// Buffered until the emulator is attached: it refuses breakpoints before
    /// that, and the editor sends them as soon as it is told `initialized`.
    attached: bool,
    /// The assembled program's memory, for questions that would otherwise cost
    /// a round trip each - "are the three bytes at this address a `CALL`?" is
    /// asked up to a hundred times per stop.
    image: Option<Vec<u8>>,
    /// Where the program's stack starts, so the walk knows where to stop.
    top_of_stack: Option<u16>,
    /// An editor `stackTrace` waiting for the stack to be read.
    pending_stack: Option<PendingStack>,
    /// A stack trace held while the page at `PC` is worked out.
    pending_page_probe: Option<(Value, Vec<crate::callstack::CallFrame>, u16)>,
    /// The stop whose instruction hint is still being fetched.
    pending_stop_hint: Option<PendingStopHint>,
    /// Why the emulator last stopped, so a stop caused by a breakpoint can be
    /// told from one caused by a step.
    last_stop_reason: Option<String>,
    /// The page the emulator turned out to have paged in at `PC`, for as long
    /// as the program is stopped there.
    pc_page: Option<u8>,
    /// Which label each call site was found to name, so a stack walked on
    /// every stop reads its few source lines once.
    call_target_names: std::collections::HashMap<u16, String>,
    /// Where the program last stopped. What `-dv` with no argument means, and
    /// what a `PC`-following disassembly view re-anchors to on every step.
    last_pc: Option<u16>,
    /// Whether the editor has finished configuring the session.
    configured: bool,
    /// A `pause` we sent ourselves, whose `stopped` event is not the editor's
    /// business.
    internal_pause: bool,
    /// A breakpoint lifted so the program can step off the address it is
    /// sitting on. Put back at the next stop.
    ///
    /// The emulator cannot step off an address it has a breakpoint on: it
    /// resumes, re-detects the breakpoint at the address it has not left yet,
    /// and stops again - answering "Instruction step completed" with the
    /// program counter unmoved. Every debugger has to lift the breakpoint under
    /// its own feet to step off it.
    ///
    /// Removed once on a mistaken reading of a log, which immediately broke
    /// stepping off a breakpoint again. It stays.
    stepped_off: Option<u32>,
    /// Whether the program has been let go. Once only: a second `continue`
    /// after the user has paused would restart it behind their back.
    started: bool,
    /// Chip-scope requests waiting for the machine to describe itself.
    pending_chip_scopes: Vec<Value>,
    /// `-chips` requests waiting for the same.
    pending_chip_prints: Vec<Value>,
    /// The last snapshot the emulator wrote, valid until it runs again.
    ///
    /// Expanding CRTC and then Gate Array must not cost two whole-machine
    /// saves: they are two views of the same instant.
    machine_state: Option<Box<cpclib_sna::Snapshot>>,
    /// Stopwatches, in NOPs.
    timers: Vec<Timer>,
    /// The register values from the last stop, by upper-case name.
    ///
    /// The editor fetches the whole set on every stop, so typing `hl` in the
    /// console is answered from what is already here rather than costing a
    /// round trip - and the answer is by construction the one the register
    /// pane is showing.
    last_registers: HashMap<String, u32>,
    /// The frames reconstructed at the last stop, so `scopes` and `variables`
    /// can be answered for them here - the emulator refuses any frame but its
    /// own, and refusing is not a useful answer to "what is in this frame".
    synthetic_frames: Vec<crate::callstack::CallFrame>,
    /// Watches the launch configuration asked for, beside the program's own.
    extra_watches: Vec<WatchRequest>,
    /// Source files, read once and kept: the register pane asks for the line
    /// at `PC` on every stop, and stepping is a lot of stops.
    source_cache: HashMap<PathBuf, Vec<String>>,
    /// What each source file looked like when the program was built.
    ///
    /// Editing a file mid-session does not move the code that is running, so
    /// a breakpoint set on line 40 of an edited file lands wherever line 40
    /// *was*. That is confusing in a way that looks like the debugger being
    /// wrong, so the file is checked and the breakpoint is marked instead.
    built_from: HashMap<PathBuf, u64>,
    /// Whether the paging limitation has already been explained.
    ///
    /// It is worth saying the first time a stop actually lands on an address
    /// two pages claim, and not worth saying again at every step afterwards.
    banking_explained: bool
}

impl<P: DapPeer> Session<P> {
    pub fn new(peer: P, map: SourceMap) -> Self {
        Self {
            peer,
            map,
            breakpoints: HashMap::new(),
            next_breakpoint_id: 1,
            seq: 1,
            program_breakpoints: Vec::new(),
            suppressed: std::collections::HashSet::new(),
            own_requests: HashMap::new(),
            own_seq: OWN_REQUEST_BASE,
            pending_watches: Vec::new(),
            pending_memory_views: Vec::new(),
            open_memory_view: None,
            pending_disassembly: Vec::new(),
            pending_editor_disassembly: Vec::new(),
            open_disassembly_view: None,
            last_flags: 0,
            attached: false,
            image: None,
            top_of_stack: None,
            pending_stack: None,
            pending_page_probe: None,
            pending_stop_hint: None,
            last_stop_reason: None,
            pc_page: None,
            call_target_names: std::collections::HashMap::new(),
            last_pc: None,
            timers: Vec::new(),
            last_registers: HashMap::new(),
            configured: false,
            internal_pause: false,
            stepped_off: None,
            started: false,
            pending_chip_scopes: Vec::new(),
            pending_chip_prints: Vec::new(),
            machine_state: None,
            synthetic_frames: Vec::new(),
            extra_watches: Vec::new(),
            source_cache: HashMap::new(),
            built_from: HashMap::new(),
            banking_explained: false
        }
    }

    fn next_seq(&mut self) -> i64 {
        let seq = self.seq;
        self.seq += 1;
        seq
    }

    /// Give the session the assembled program's memory.
    ///
    /// Without it there is no call stack: every candidate return address would
    /// need its own round trip to the emulator to be checked, and at fifty
    /// milliseconds a poll a hundred of those is not a debugger.
    pub fn with_image(mut self, image: Vec<u8>) -> Self {
        self.image = Some(image);
        self
    }

    /// Where the program's stack starts.
    ///
    /// DeZog's `topOfStack`, and for the same reason: reading past it is
    /// reading memory the program never pushed, where every word is a chance
    /// at a frame that does not exist.
    pub fn with_top_of_stack(mut self, top: u16) -> Self {
        self.top_of_stack = Some(top);
        self
    }

    /// The top of stack this session will use, taken from the program's own
    /// symbols when it was not configured.
    ///
    /// The names are the ones people actually write; none of them existing is
    /// not a problem, it only means the walk is capped by count instead.
    pub fn top_of_stack_from_symbols(mut self) -> Self {
        if self.top_of_stack.is_some() {
            return self;
        }
        for name in ["topOfStack", "stack_top", "stack_end", "stack"] {
            if let Some(address) = self.map.address_of_symbol(name)
                && let Ok(address) = u16::try_from(address)
            {
                self.top_of_stack = Some(address);
                break;
            }
        }
        self
    }

    /// Record what the program's sources looked like at build time.
    ///
    /// Cheap: the files are the ones just assembled, so they are in the page
    /// cache, and it is done once per session.
    pub fn record_source_state(&mut self) {
        let files: Vec<PathBuf> = self.map.files().to_vec();
        for file in files {
            if let Some(fingerprint) = fingerprint_of(&file) {
                self.built_from.insert(file, fingerprint);
            }
        }
    }

    /// Whether `file` has changed since the build this session is debugging.
    fn is_stale(&self, file: &Path) -> bool {
        match self.built_from.get(file) {
            // Never fingerprinted - a file we were not built from, or a build
            // we did not drive. Not knowing is not the same as knowing it is
            // stale, and claiming otherwise would put a warning on every
            // breakpoint.
            None => false,
            Some(built) => fingerprint_of(file).is_some_and(|now| now != *built)
        }
    }

    /// The program's map, for callers that need to report on it.
    pub fn map(&self) -> &SourceMap {
        &self.map
    }

    pub fn peer(&self) -> &P {
        &self.peer
    }

    pub fn peer_mut(&mut self) -> &mut P {
        &mut self.peer
    }

    /// Send a request of our own to the emulator, and remember that its answer
    /// is ours to consume.
    pub fn send_own_request(&mut self, command: &str, arguments: Value) -> std::io::Result<()> {
        self.send_own(command, arguments, Purpose::Plain)
    }

    fn send_own(
        &mut self,
        command: &str,
        arguments: Value,
        purpose: Purpose
    ) -> std::io::Result<()> {
        let seq = self.own_seq;
        self.own_seq += 1;
        self.own_requests.insert(
            seq,
            OwnRequest {
                command: command.to_string(),
                purpose
            }
        );
        self.peer.send(protocol::request(command, arguments, seq))
    }

    /// Whether `response` answers something we asked for.
    fn is_our_answer(&mut self, response: &Value) -> Option<OwnRequest> {
        let request_seq = response.get("request_seq").and_then(Value::as_i64)?;
        self.own_requests.remove(&request_seq)
    }

    /// Adopt the breakpoints the assembled program asked for.
    ///
    /// A `BREAKPOINT` directive is the author saying where they want to stop,
    /// which is worth at least as much as a red dot - and unlike a red dot it
    /// survives in the source. They are sent alongside the editor's.
    pub fn adopt_program_breakpoints(
        &mut self,
        breakpoints: &[cpclib_asm::assembler::delayed_command::AssembledBreakpoint]
    ) -> Vec<String> {
        use cpclib_asm::assembler::delayed_command::AssembledBreakpointKind as Kind;

        let mut notices = Vec::new();
        for breakpoint in breakpoints {
            let where_ = breakpoint
                .name
                .clone()
                .unwrap_or_else(|| self.address_in_source(breakpoint.address as u32));

            let (watch, unsupported) = match breakpoint.kind {
                Kind::Execution => (None, breakpoint.extra.clone()),
                Kind::Memory { read, write } => {
                    (
                        Some(WatchRequest {
                            address: breakpoint.address as u32,
                            read,
                            write,
                            label: where_.clone()
                        }),
                        breakpoint.extra.clone()
                    )
                },
                // The emulator has no I/O breakpoints at all; say so rather
                // than quietly turning it into something else.
                Kind::Io => {
                    (
                        None,
                        Some(match &breakpoint.extra {
                            Some(extra) => format!("an I/O breakpoint ({extra})"),
                            None => "an I/O breakpoint".to_string()
                        })
                    )
                },
            };

            if let Some(unsupported) = &unsupported {
                notices.push(format!(
                    "{where_} asks for {unsupported}; this emulator can only break on the \
                     address, so that part is not applied."
                ));
            }

            self.program_breakpoints.push(ProgramBreakpoint {
                address: breakpoint.address as u32,
                watch,
                one_shot: false,
                written_at: breakpoint.written_at.clone()
            });
        }
        notices
    }

    /// Where an address is in the source, as a human would say it.
    fn address_in_source(&self, address: u32) -> String {
        self.map
            .location_at(address)
            .map(|l| {
                format!(
                    "{}:{}",
                    l.file
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    l.line
                )
            })
            .unwrap_or_else(|| address_reference(address))
    }

    /// What the program's own `BREAKPOINT` directives will do, said before
    /// they do it.
    ///
    /// The editor's gutter cannot show these - it does not know they exist -
    /// so a program that stops at one looks like a program that stopped at
    /// nothing, which is exactly what it was reported as: "the emulator
    /// stopped at random locations without any breakpoint". The addresses were
    /// eight expansions of one `BREAKPOINT` written inside a macro body, in a
    /// file the user had never put a red dot in. One directive, one address is
    /// the assumption the red-dot-writes-the-directive design rests on, and a
    /// macro breaks it, so the set has to be spelled out instead.
    pub fn program_breakpoint_notice(&self) -> Option<String> {
        // As many as fit in a console line anyone will actually read; the
        // count carries the rest.
        const LISTED: usize = 8;

        let addresses: Vec<u32> = self
            .program_breakpoints
            .iter()
            .filter(|bp| {
                bp.watch.is_none() && !bp.one_shot && !self.suppressed.contains(&bp.address)
            })
            .map(|bp| bp.address)
            .collect();
        if addresses.is_empty() {
            return None;
        }

        let mut where_: Vec<String> = addresses
            .iter()
            .take(LISTED)
            .map(|address| self.address_in_source(*address))
            .collect();
        if addresses.len() > LISTED {
            where_.push(format!("and {} more", addresses.len() - LISTED));
        }
        let count = addresses.len();
        let plural = if count == 1 { "" } else { "s" };
        Some(format!(
            "{count} BREAKPOINT directive{plural} in the program {} armed alongside the editor's \
             red dots, at {}. The gutter cannot show a directive, so stopping at one looks like \
             stopping at nothing - and a directive written inside a macro body is armed once for \
             every place the macro is used. Setting a red dot anywhere in a file and clearing it \
             again takes that file's directives back for this session.",
            if count == 1 { "is" } else { "are" },
            where_.join(", ")
        ))
    }

    /// The memory watches the program asked for, for a peer that has watch
    /// slots.
    pub fn watch_requests(&self) -> Vec<WatchRequest> {
        let mut watches: Vec<WatchRequest> = self
            .program_breakpoints
            .iter()
            .filter_map(|bp| bp.watch.clone())
            .collect();
        // The launch configuration's come second: the program's own directives
        // were written by the person who knows what matters in it, and the
        // channels are scarce.
        for watch in &self.extra_watches {
            if !watches.iter().any(|known| known.address == watch.address) {
                watches.push(watch.clone());
            }
        }
        watches
    }

    /// Mark the emulator attached and flush any breakpoints held back.
    pub fn on_attached(&mut self) -> std::io::Result<()> {
        self.attached = true;
        self.push_breakpoints()?;
        self.push_watches()?;
        self.start_program_if_ready()
    }

    /// The editor has finished configuring the session.
    pub fn on_configuration_done(&mut self) -> std::io::Result<()> {
        self.configured = true;
        self.start_program_if_ready()
    }

    /// Note that the session is ready. **Does not resume the program.**
    ///
    /// It used to: the emulator starts executing the moment its snapshot is
    /// loaded, milliseconds before the breakpoints are armed, so a program that
    /// passes its breakpoint address once during start-up has already gone by.
    /// Holding it with a `pause` at launch and releasing it here was the
    /// protocol's own answer to that.
    ///
    /// It also stopped the CPU dead. After that pause/resume the emulator
    /// accepted every `stepIn`, answered "Instruction step completed", and
    /// executed nothing at all - `PC`, `AF` and `HL` identical across fifty
    /// steps. Whatever that pause does to the run state, the machine does not
    /// come back from it, so the race is the lesser problem and this no longer
    /// touches execution.
    ///
    /// The race is still real, and the fix belongs in the emulator: a snapshot
    /// should load *paused*, so breakpoints can be armed before anything runs.
    fn start_program_if_ready(&mut self) -> std::io::Result<()> {
        if self.started || !self.attached || !self.configured {
            return Ok(());
        }
        self.started = true;
        Ok(())
    }

    /// What this adapter tells the editor it can do.
    ///
    /// Source breakpoints need no capability flag - they are the baseline - and
    /// everything else is what the emulator advertised, since we forward those
    /// requests to it unchanged.
    pub fn capabilities() -> Value {
        json!({
            "supportsConfigurationDoneRequest": true,
            "supportsStepBack": true,
            "supportsReadMemoryRequest": true,
            "supportsWriteMemoryRequest": true,
            // Deliberately *not* advertised.
            //
            // A stop should land on your source, and it does - the frames carry
            // the file, line and instruction columns. But an editor told it can
            // disassemble opens that view the moment any frame lacks a source
            // line, and a reconstructed frame in firmware legitimately does; it
            // then also switches stepping to instruction granularity and stays
            // there. Not claiming the capability keeps the source in front of
            // you, which is the point of a source-level debugger.
            //
            // Disassembly is still a request away: `-dv` opens it, in a panel
            // that can show the source column and name the addresses in
            // operands, neither of which the built-in view can do.
            // `supportsSteppingGranularity` is deliberately absent, for the
            // same reason as `supportsDisassembleRequest` below: it exists so
            // an editor can ask for *instruction* stepping, which is what the
            // Disassembly view does - and a step that answers in instructions
            // rather than in source lines is the thing this adapter is built
            // to avoid. `-dv` is where disassembly lives.

            "supportsEvaluateForHovers": true,
            // Watchpoints, with the caveat spelled out in the description
            // `dataBreakpointInfo` hands back: the emulator's watch channels
            // *report* writes, they do not stop on them. Advertising the
            // capability is still right - it is the only way to reach the
            // channels from the UI rather than from launch.json - but the
            // limitation is named where the user reads it, not hidden.
            "supportsDataBreakpoints": true,
            // Only `PC` is really settable, and only on an emulator that
            // offers it - but the capability is what puts an edit box on the
            // register pane at all, and a refusal that says *why* is worth
            // more than a pane that looks read-only for no stated reason.
            "supportsSetVariable": true
        })
    }

    /// Handle one message from the editor. Returns what to send back to it.
    pub fn on_editor_message(&mut self, message: &Value) -> std::io::Result<Vec<Value>> {
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();

        match command {
            "setBreakpoints" => self.set_breakpoints(message),
            // Stepping off an address that has a breakpoint on it needs the
            // breakpoint lifted first, on *every* emulator tried so far: the
            // machine resumes, immediately re-detects the breakpoint at the
            // address it has not left yet, and stops again - reporting a
            // completed step with the program counter unmoved.
            //
            // Once made conditional, on the reasoning that 1984js builds its
            // step from a temporary breakpoint channel that re-arming would
            // tear down. That reasoning was wrong twice over: its
            // `_setInstructionBreakpoints` clears only the *user* slots, and
            // the lift happens *before* the step, when no temporary channel
            // exists. A transcript settled it - six consecutive `stepIn`s at
            // `0x0403`, each answered "Instruction step completed", with `PC`
            // never leaving `0x0403`.
            "next" | "stepIn" | "stepOut" => {
                self.lift_breakpoint_under_pc()?;
                // Only a step *over* asks about the source. Stepping *into*
                // is an instruction by definition, and stepping *out* is aimed
                // at a return address no source line describes.
                if command == "next" {
                    let line = self.line_at_pc();
                    self.peer.note_line_at_pc(line);
                }
                self.peer.send(message.clone())?;
                Ok(Vec::new())
            },
            // Noted *and* forwarded: the emulator wants it too, and this is
            // where the program is finally allowed to run.
            "configurationDone" => {
                self.on_configuration_done()?;
                self.peer.send(message.clone())?;
                Ok(Vec::new())
            },
            // Watchpoints, from the editor's own right-click rather than from
            // a `BREAKPOINT ... TYPE=MEM` directive or `watchLabels`.
            // Frames we reconstructed are ours to describe: the emulator
            // validates frame ids against its own and answers "Stack frame
            // reference has expired" for anything else, which tells the user
            // nothing about the frame they clicked.
            "scopes" if self.is_synthetic_frame(message) => self.synthetic_scopes(message),
            "variables" if self.is_synthetic_reference(message) => {
                self.synthetic_variables(message)
            },
            // The editor's own disassembly view, decoded here rather than by
            // the emulator - so it reads exactly like `-dv` and like the source
            // beside it, whatever emulator is underneath.
            //
            // Only the forward case: a negative `instructionOffset` asks what
            // precedes an address, and decoding Z80 backwards is genuinely
            // ambiguous. That one is still the emulator's to answer, which is
            // no worse than before.
            "disassemble" => self.editor_disassembly(message),
            "setVariable" => self.set_variable(message),
            "dataBreakpointInfo" => self.data_breakpoint_info(message),
            "setDataBreakpoints" => self.set_data_breakpoints(message),
            // Answered here rather than forwarded: the editor asks for threads
            // the moment it is told the program stopped, and an emulator that
            // has not finished attaching would refuse - leaving a stop with no
            // thread to hang the toolbar on. A CPC has one CPU; that answer is
            // never going to be more complicated.
            // Variables we invented - the decoded flags, and the chips the
            // emulator cannot report yet - are answered here rather than
            // forwarded, since the emulator has never heard of them and
            // rejects references it does not know.
            "variables"
                if message
                    .get("arguments")
                    .and_then(|a| a.get("variablesReference"))
                    .and_then(Value::as_i64)
                    == Some(FLAGS_REFERENCE) =>
            {
                let seq = self.next_seq();
                Ok(vec![protocol::response(
                    message,
                    json!({ "variables": crate::inspect::flag_variables(self.last_flags) }),
                    seq
                )])
            },
            // The chips are not on the emulator's debug API at all, so their
            // state is recovered by asking the machine to write a snapshot of
            // itself and reading its header. That is a round trip, which is why
            // these scopes are declared expensive.
            "variables"
                if message
                    .get("arguments")
                    .and_then(|a| a.get("variablesReference"))
                    .and_then(Value::as_i64)
                    .is_some_and(crate::inspect::is_chip_scope) =>
            {
                self.chip_scope(message)
            },
            // A watch expression. `animation_state` is a label, and what the
            // user wants to see is the *byte there*, not the address - reading
            // an address they already know is no use while stepping.
            "evaluate" => self.evaluate(message),
            // The editor asks for a file's contents when it cannot open the
            // path itself. Answered here - the emulator has never heard of a
            // source file and rejects requests it does not implement, which is
            // what produced "DAP request 'source' is not supported" in place of
            // the code.
            "source" => self.source(message),
            "threads" => {
                let seq = self.next_seq();
                Ok(vec![protocol::response(
                    message,
                    json!({ "threads": [{ "id": THREAD_ID, "name": "Z80" }] }),
                    seq
                )])
            },
            other => {
                // Not ours - but a peer that rejects what it does not
                // implement turns "forward and see" into an error message
                // shown to the user in place of the thing they asked for. Ask
                // first.
                if self.peer.quirks().rejects_unknown_requests && !self.peer.supports(other) {
                    let seq = self.next_seq();
                    return Ok(vec![protocol::failure(
                        message,
                        &format!("the emulator being debugged does not implement '{other}'."),
                        seq
                    )]);
                }
                self.peer.send(message.clone())?;
                Ok(Vec::new())
            }
        }
    }

    /// Handle one message from the emulator. Returns what to send to the editor.
    pub fn on_emulator_message(&mut self, message: &Value) -> Vec<Value> {
        // Answers to our own requests never reach the editor: it numbers its
        // requests from 1 as well, so ours would arrive looking like replies to
        // messages it sent - an `attach` response answering its `launch`.
        if message.get("type").and_then(Value::as_str) == Some("response")
            && let Some(own) = self.is_our_answer(message)
        {
            match own.purpose {
                Purpose::WatchRead => {
                    if let Some(answer) = self.complete_watch(message) {
                        return answer;
                    }
                },
                Purpose::WatchArm => return self.report_armed_watches(message),
                Purpose::MemoryView => return self.complete_memory_view(message),
                Purpose::PageProbe => return self.stack_step_page_probe(message),
                Purpose::StopHint => return self.complete_stop_hint(message),
                Purpose::DisassemblyView => return self.complete_disassembly_view(message),
                Purpose::MachineState => return self.complete_machine_state(message),
                Purpose::EditorDisassembly => {
                    return self.complete_editor_disassembly(message);
                },
                Purpose::StackScopes => return self.stack_step_registers(message),
                Purpose::StackRegisters => return self.stack_step_read(message),
                Purpose::StackRead => return self.stack_step_finish(message),
                Purpose::Plain => {}
            }
            let command = own.command;
            if command == "attach" && message.get("success").and_then(Value::as_bool) == Some(true)
            {
                self.attached = true;
                // Halt the program before arming anything.
                //
                // The emulator begins executing the moment its snapshot loads,
                // and the breakpoints go out over the wire some milliseconds
                // later - so a program that passes its breakpoint address once
                // during start-up has already gone by, and nothing ever stops.
                //
                // Through the session's own `pause` request, never by halting
                // the core directly: the emulator tracks a `running` flag
                // beside the core's real state, and pausing behind its back
                // leaves the two disagreeing - it then refuses `continue` as
                // "notStopped" and answers every `stepIn` with success while
                // the program counter stays put.
                // The breakpoints held while the emulator was starting go out
                // now, and the program is let go once they and the editor's
                // configuration are both in - see `start_program_if_ready`.
                // Failing to send them is worth reporting, but not worth
                // losing the stop event that follows.
                if let Err(problem) = self
                    .push_breakpoints()
                    .and_then(|()| self.push_watches())
                    .and_then(|()| self.start_program_if_ready())
                {
                    return vec![protocol::event(
                        "output",
                        json!({
                            "category": "stderr",
                            "output": format!("could not set breakpoints: {problem}\n")
                        }),
                        self.next_seq()
                    )];
                }
            }
            return Vec::new();
        }

        let kind = message
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let command = message
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if kind == "response" {
            match command {
                "stackTrace" => {
                    // The emulator's one frame is the bottom of a stack it
                    // cannot see the rest of; recovering the callers takes a
                    // few more questions, so its answer waits.
                    if let Some(answer) = self.begin_stack_walk(message) {
                        return answer;
                    }
                    if self.pending_stack.is_some() {
                        return Vec::new();
                    }
                    return self.annotate_stack_trace(message);
                },
                // Each instruction gets the line it came from, so the
                // disassembly view can show your source beside the opcodes and
                // jump to it.
                "disassemble" => {
                    let mut annotated = message.clone();
                    if let Some(instructions) = annotated
                        .get_mut("body")
                        .and_then(|b| b.get_mut("instructions"))
                        .and_then(Value::as_array_mut)
                    {
                        crate::inspect::annotate_disassembly(instructions, &self.map, self.pc_page);
                    }
                    return vec![annotated];
                },
                // `AF` is one hex word; the flags inside it are what gets read
                // while stepping.
                "variables" => {
                    let mut annotated = message.clone();
                    if let Some(variables) = annotated
                        .get_mut("body")
                        .and_then(|b| b.get_mut("variables"))
                        .and_then(Value::as_array_mut)
                    {
                        // Remember `F` so expanding the flags row later does
                        // not need a second round trip to an emulator that may
                        // have resumed in the meantime.
                        if let Some(flags) = crate::inspect::flags_of(variables) {
                            self.last_flags = flags;
                        }
                        self.remember_registers(variables);
                        let cost = self.cost_at_pc(variables);
                        crate::inspect::annotate_registers_with(
                            variables,
                            FLAGS_REFERENCE,
                            Some(&self.map)
                        );
                        if let Some(nops) = cost {
                            crate::inspect::annotate_cost(variables, nops);
                        }
                    }
                    return vec![annotated];
                },
                // The CPC is not only its CPU: leave room for the chips that
                // decide what the Z80's work actually looks like.
                "scopes" => {
                    let mut annotated = message.clone();
                    if let Some(scopes) = annotated
                        .get_mut("body")
                        .and_then(|b| b.get_mut("scopes"))
                        .and_then(Value::as_array_mut)
                    {
                        scopes.extend(crate::inspect::extra_scopes());
                    }
                    return vec![annotated];
                },
                _ => {}
            }
        }

        if kind == "event" {
            let event = message
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if event == "stopped" {
                // The halt we asked for ourselves is not a stop the user made.
                // Forwarded, the editor believes the program is stopped, asks
                // for a stack trace, and gets "The CPC must be stopped" from an
                // emulator we have already released - which it shows as an
                // error the user did nothing to cause.
                if self.swallow_internal_pause(message) {
                    return Vec::new();
                }
                return self.on_stopped(message);
            }
            // The emulator's own `initialized` is about *its* readiness, not
            // the editor's configuration handshake - the editor was told
            // `initialized` when the source map existed. Forwarding a second
            // one makes VS Code re-send its whole breakpoint set for no reason.
            if event == "initialized" {
                return Vec::new();
            }
        }
        vec![message.clone()]
    }

    /// Turn a memory read into the watch that asked for it.
    fn complete_watch(&mut self, response: &Value) -> Option<Vec<Value>> {
        if self.pending_watches.is_empty() {
            return None;
        }
        // Reads are answered in order, so the oldest waiting watch is this one.
        let watch = self.pending_watches.remove(0);
        let seq = self.next_seq();

        let bytes = response
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default();

        if bytes.is_empty() {
            return Some(vec![protocol::failure(
                &watch.request,
                &format!(
                    "{} is at {} but could not be read",
                    watch.name,
                    address_reference(watch.address)
                ),
                seq
            )]);
        }

        // Little-endian, like everything else the Z80 does with a word.
        let value = match watch.width {
            2 if bytes.len() >= 2 => u32::from(bytes[0]) | (u32::from(bytes[1]) << 8),
            _ => u32::from(bytes[0])
        };
        // Both halves matter, and which one depends on the label: for a code
        // label the address is the answer, for a variable the contents are.
        // Showing only the byte made `event_handler` read as `0x01`.
        let rendered = if watch.width == 2 {
            format!(
                "{} -> 0x{value:04X} ({value})",
                address_reference(watch.address)
            )
        }
        else {
            format!(
                "{} -> 0x{value:02X} ({value})",
                address_reference(watch.address)
            )
        };

        Some(vec![protocol::response(
            &watch.request,
            json!({
                "result": rendered,
                "type": format!("{} byte(s) at {}", watch.width,
                    address_reference(watch.address)),
                "variablesReference": 0,
                "memoryReference": address_reference(watch.address)
            }),
            seq
        )])
    }

    /// Make a stop actionable for the editor.
    ///
    /// A `stopped` event is what turns the toolbar on - continue, step, pause -
    /// so anything missing from it costs the user a button. `threadId` is
    /// required for that, and `allThreadsStopped` tells the editor not to go
    /// looking for others on a machine that has exactly one.
    /// Lift the breakpoint the program is sitting on, so it can leave.
    fn lift_breakpoint_under_pc(&mut self) -> std::io::Result<()> {
        let Some(pc) = self.last_pc.map(u32::from)
        else {
            return Ok(());
        };
        let armed = self
            .breakpoints
            .values()
            .flatten()
            .filter_map(|bp| bp.address)
            .chain(
                self.program_breakpoints
                    .iter()
                    .filter(|bp| bp.watch.is_none())
                    .map(|bp| bp.address)
            )
            .any(|address| address == pc);
        if !armed || self.stepped_off == Some(pc) {
            return Ok(());
        }
        self.stepped_off = Some(pc);
        self.push_breakpoints()
    }

    /// Put back whatever was lifted to let the program move.
    fn restore_lifted_breakpoint(&mut self) -> std::io::Result<()> {
        if self.stepped_off.take().is_none() {
            return Ok(());
        }
        self.push_breakpoints()
    }

    /// Whether this `stopped` is the one our own launch-time `pause` caused.
    fn swallow_internal_pause(&mut self, message: &Value) -> bool {
        if !self.internal_pause {
            return false;
        }
        // Exactly one event, whichever it turns out to be. The `continue` that
        // releases the program is sent in the same breath as the `pause`, long
        // before either is answered, so this cannot be cleared on release - and
        // leaving it armed would swallow the user's own first pause instead.
        self.internal_pause = false;

        let reason = message
            .get("body")
            .and_then(|b| b.get("reason"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        // Only the pause itself. A breakpoint reached in the same instant is a
        // real stop and must go out.
        reason == "pause"
    }

    /// Everything that has to happen when the program stops.
    ///
    /// The `stopped` event goes out first: it is what turns the toolbar on, and
    /// the housekeeping behind it must never be able to delay or lose it.
    fn on_stopped(&mut self, message: &Value) -> Vec<Value> {
        self.last_stop_reason = message
            .get("body")
            .and_then(|body| body.get("reason"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let mut out = vec![self.enrich_stopped(message)];

        // `stopOnEntry` has now happened. Leaving it armed would stop again
        // every time the program came back past its entry address.
        if self.retire_one_shot_breakpoints()
            && let Err(problem) = self.push_breakpoints()
        {
            let seq = self.next_seq();
            out.push(protocol::event(
                "output",
                json!({
                    "category": "stderr",
                    "output": format!("could not clear the entry breakpoint: {problem}\n")
                }),
                seq
            ));
        }

        // The program has left the address it was sitting on, so the
        // breakpoint lifted to let it go can be armed again.
        if let Err(problem) = self.restore_lifted_breakpoint() {
            let seq = self.next_seq();
            out.push(protocol::event(
                "output",
                json!({
                    "category": "stderr",
                    "output": format!("could not re-arm a breakpoint: {problem}\n")
                }),
                seq
            ));
        }

        // The chips have moved on; the snapshot describing where they were is
        // no longer where they are.
        self.machine_state = None;

        self.refresh_memory_view();
        self.refresh_disassembly_view();
        out
    }

    fn enrich_stopped(&self, message: &Value) -> Value {
        let mut stopped = message.clone();
        let Some(body) = stopped.get_mut("body")
        else {
            return stopped;
        };
        if body.get("threadId").is_none() {
            body["threadId"] = json!(THREAD_ID);
        }
        if body.get("allThreadsStopped").is_none() {
            body["allThreadsStopped"] = json!(true);
        }
        stopped
    }

    /// `setBreakpoints` is source-shaped, so it is answered here and turned
    /// into addresses for the emulator.
    fn set_breakpoints(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let arguments = request.get("arguments").cloned().unwrap_or(json!({}));
        let path = arguments
            .get("source")
            .and_then(|s| s.get("path"))
            .and_then(Value::as_str)
            .map(PathBuf::from);

        let Some(path) = path
        else {
            let seq = self.next_seq();
            return Ok(vec![protocol::failure(
                request,
                "the breakpoint request named no source file",
                seq
            )]);
        };

        let requested = arguments
            .get("breakpoints")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        // Checked once per request rather than once per breakpoint: the answer
        // cannot change between two lines of the same file.
        let stale = self.is_stale(&path);

        let mut placed = Vec::new();
        for entry in &requested {
            let line = entry.get("line").and_then(Value::as_u64).unwrap_or(0) as u32;
            let id = self.next_breakpoint_id;
            self.next_breakpoint_id += 1;

            match self.map.breakpoint_at(&path, line) {
                Some(placement) => {
                    placed.push(PlacedBreakpoint {
                        id,
                        requested_line: line,
                        line: placement.line,
                        address: Some(placement.address),
                        message: stale.then(|| {
                            format!(
                                "{} has changed since this program was built; this \
                                 breakpoint is at the address line {} had at build time. \
                                 Restart the session to place it where you mean.",
                                path.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.display().to_string()),
                                placement.line
                            )
                        })
                    })
                },
                None => {
                    // Distinguish "this line has no code" from "we have no map
                    // at all": they look identical in the breakpoints pane, and
                    // only one of them is the user's fault. An empty map means
                    // the project's entry point could not be resolved, which is
                    // a configuration problem with a known fix.
                    let message = if self.map.is_empty() {
                        "no source map: the entry point of this project could not be \
                         resolved, so no breakpoint can be placed. Set `[asm] entry` in \
                         cpclib-lsp.toml to the file that carries the RUN directive."
                            .to_string()
                    }
                    else if self.map.files().iter().all(|known| known != &path) {
                        format!(
                            "{} is not part of the program being debugged",
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.display().to_string())
                        )
                    }
                    else {
                        "no code was assembled at or after this line".to_string()
                    };
                    placed.push(PlacedBreakpoint {
                        id,
                        requested_line: line,
                        line,
                        address: None,
                        message: Some(message)
                    })
                }
            }
        }

        // A `BREAKPOINT` directive on a line of *this* file that the editor did
        // not just ask for is one the user has taken back: in this extension
        // the gutter writes the directive, so the red dot and the directive are
        // the same breakpoint, and removing the dot has to remove it.
        //
        // Scoped to this file's addresses on purpose: `setBreakpoints` speaks
        // for one file, and a directive in another file is not being spoken
        // about at all.
        self.resync_suppressions(&path, &placed);

        self.breakpoints.insert(path, placed.clone());
        self.push_breakpoints()?;

        let body = json!({
            "breakpoints": placed
                .iter()
                .map(|bp| {
                    let mut entry = json!({
                        "id": bp.id,
                        "verified": bp.address.is_some(),
                        "line": bp.line
                    });
                    if let Some(address) = bp.address {
                        entry["instructionReference"] = json!(address_reference(address));
                    }
                    if let Some(message) = &bp.message {
                        entry["message"] = json!(message);
                    }
                    entry
                })
                .collect::<Vec<_>>()
        });
        let seq = self.next_seq();
        Ok(vec![protocol::response(request, body, seq)])
    }

    /// Answer a watch or hover expression.
    ///
    /// Deliberately narrow: a label, optionally with a size suffix, and nothing
    /// resembling an expression language. A wrong answer here is worse than no
    /// answer - it would be believed - so anything not understood is refused
    /// and handed to the emulator, which can at least answer for registers.
    fn evaluate(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let expression = request
            .get("arguments")
            .and_then(|a| a.get("expression"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();

        // Console commands, DeZog's spelling. Typed where you already are, so
        // that looking at memory does not mean leaving the keyboard - but the
        // *result* goes to a panel, because a memory dump is something you keep
        // open and watch, not something to scroll back for.
        if expression.starts_with('-') {
            return self.console_command(request, expression);
        }

        // A register name answers from the values the editor just fetched, and
        // does so *before* the symbol table is consulted. Typing `hl` means the
        // register; a program that also has a label called `hl` is not what
        // anyone means by it, and reading that label's memory instead would be
        // a wrong answer wearing the right shape.
        if let Some(answer) = self.evaluate_register(request, expression) {
            return Ok(answer);
        }

        // `label,w` asks for a word; plain `label` for a byte.
        let (name, width) = match expression.rsplit_once(',') {
            Some((name, "w" | "W")) => (name.trim(), 2usize),
            Some((name, "b" | "B")) => (name.trim(), 1usize),
            _ => (expression, 1usize)
        };

        let Some(address) = self.resolve_watch_address(name)
        else {
            // Never forwarded: the emulator implements no `evaluate` at all and
            // answers "not supported", which tells the user nothing about the
            // name they typed. Naming near misses is more use than a protocol
            // error - a mistyped or not-yet-exported label is the common case.
            let suggestions = self.similar_symbols(name);
            let seq = self.next_seq();
            let detail = if suggestions.is_empty() {
                String::new()
            }
            else {
                format!(" Did you mean {}?", suggestions.join(", "))
            };
            return Ok(vec![protocol::failure(
                request,
                &format!("'{name}' is not a label of the program being debugged.{detail}"),
                seq
            )]);
        };

        // The bytes have to come from the emulator, and its answer arrives
        // later; report the address now and the value when the read returns.
        self.pending_watches.push(PendingWatch {
            request: request.clone(),
            name: name.to_string(),
            address,
            width
        });
        self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(address),
                "count": width
            }),
            Purpose::WatchRead
        )?;
        Ok(Vec::new())
    }

    fn is_synthetic_frame(&self, request: &Value) -> bool {
        request
            .get("arguments")
            .and_then(|a| a.get("frameId"))
            .and_then(Value::as_i64)
            .is_some_and(|id| {
                (SYNTHETIC_FRAME_BASE..SYNTHETIC_FRAME_BASE + SYNTHETIC_RANGE).contains(&id)
            })
    }

    fn is_synthetic_reference(&self, request: &Value) -> bool {
        request
            .get("arguments")
            .and_then(|a| a.get("variablesReference"))
            .and_then(Value::as_i64)
            .is_some_and(|reference| {
                (SYNTHETIC_SCOPE_BASE..SYNTHETIC_SCOPE_BASE + SYNTHETIC_RANGE).contains(&reference)
            })
    }

    /// What a reconstructed frame can offer.
    fn synthetic_scopes(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let index = request
            .get("arguments")
            .and_then(|a| a.get("frameId"))
            .and_then(Value::as_i64)
            .map(|id| id - SYNTHETIC_FRAME_BASE)
            .unwrap_or(0);

        let seq = self.next_seq();
        Ok(vec![protocol::response(
            request,
            json!({
                "scopes": [{
                    "name": "Frame",
                    "presentationHint": "locals",
                    "variablesReference": SYNTHETIC_SCOPE_BASE + index,
                    "expensive": false
                }]
            }),
            seq
        )])
    }

    /// The contents of a reconstructed frame: what the stack still holds.
    fn synthetic_variables(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let index = request
            .get("arguments")
            .and_then(|a| a.get("variablesReference"))
            .and_then(Value::as_i64)
            .map(|reference| (reference - SYNTHETIC_SCOPE_BASE) as usize)
            .unwrap_or(0);

        let Some(frame) = self.synthetic_frames.get(index).cloned()
        else {
            let seq = self.next_seq();
            return Ok(vec![protocol::response(
                request,
                json!({ "variables": [] }),
                seq
            )]);
        };

        let describe = |address: u16, map: &SourceMap| -> String {
            match map.symbol_at(address as u32) {
                Some(name) => format!("0x{address:04X} ({name})"),
                None => {
                    match map.location_at(address as u32) {
                        Some(location) => {
                            format!(
                                "0x{address:04X} ({}:{})",
                                location
                                    .file
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                location.line
                            )
                        },
                        None => format!("0x{address:04X}")
                    }
                },
            }
        };

        let mut variables = vec![
            json!({
                "name": "called",
                "value": describe(frame.called, &self.map),
                "type": "the routine this frame entered",
                "variablesReference": 0,
                "memoryReference": address_reference(frame.called as u32)
            }),
            json!({
                "name": "call site",
                "value": describe(frame.call_site, &self.map),
                "type": "where the CALL is",
                "variablesReference": 0,
                "memoryReference": address_reference(frame.call_site as u32)
            }),
            json!({
                "name": "returns to",
                "value": describe(frame.return_address, &self.map),
                "type": "where the RET goes",
                "variablesReference": 0,
                "memoryReference": address_reference(frame.return_address as u32)
            }),
            json!({
                "name": "registers",
                "value": "not available for an outer frame",
                "type": OUTER_FRAME_NOTE,
                "variablesReference": 0
            }),
        ];

        // The words this frame pushed. They fell out of the stack walk, so
        // showing them costs nothing - and saved registers are usually exactly
        // what someone opening an outer frame is hoping to find.
        for (position, value) in frame.locals.iter().enumerate() {
            variables.push(json!({
                "name": format!("pushed[{position}]"),
                "value": describe(*value, &self.map),
                "type": "a word this frame pushed",
                "variablesReference": 0,
                "memoryReference": address_reference(*value as u32)
            }));
        }

        let seq = self.next_seq();
        Ok(vec![protocol::response(
            request,
            json!({ "variables": variables }),
            seq
        )])
    }

    /// Answer the editor's disassembly view from memory we read ourselves.
    ///
    /// Both directions. A negative `instructionOffset` asks for context
    /// *before* the anchor, which Z80 cannot be read backwards to find - so the
    /// bytes before it are read too and every possible alignment is tried, the
    /// winner being the one whose instruction boundaries land exactly on the
    /// anchor. If none does, the emulator answers instead.
    fn editor_disassembly(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let arguments = request.get("arguments");
        let reference = arguments
            .and_then(|a| a.get("memoryReference"))
            .and_then(Value::as_str)
            .and_then(parse_address_reference);
        let Some(reference) = reference
        else {
            let seq = self.next_seq();
            return Ok(vec![protocol::failure(
                request,
                "disassemble needs a memoryReference",
                seq
            )]);
        };

        let number = |name: &str| {
            arguments
                .and_then(|a| a.get(name))
                .and_then(Value::as_i64)
                .unwrap_or(0)
        };
        let anchor = reference.wrapping_add(number("offset").max(0) as u32) & 0xFFFF;
        let count = number("instructionCount").clamp(1, 512);
        // Four bytes per instruction is the Z80's worst case, so this always
        // covers what was asked for.
        let before = (-number("instructionOffset")).max(0) as usize;
        let back = (before * 4).min(0x800) as u32;
        let start = anchor.wrapping_sub(back) & 0xFFFF;
        let bytes = (back as usize + (count as usize) * 4).clamp(1, 0x1000);

        self.pending_editor_disassembly
            .push((request.clone(), start, anchor, before, count));
        if self
            .send_own(
                "readMemory",
                json!({
                    "memoryReference": address_reference(start),
                    "count": bytes
                }),
                Purpose::EditorDisassembly
            )
            .is_err()
        {
            // Could not ask; let the emulator answer rather than failing the
            // view outright.
            self.pending_editor_disassembly.pop();
            self.peer.send(request.clone())?;
            return Ok(Vec::new());
        }
        Ok(Vec::new())
    }

    /// The bytes came back; decode them for the editor.
    fn complete_editor_disassembly(&mut self, response: &Value) -> Vec<Value> {
        if self.pending_editor_disassembly.is_empty() {
            return Vec::new();
        }
        let (request, start, anchor, before, count) = self.pending_editor_disassembly.remove(0);

        let bytes = response
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default();

        let decoded = match (u16::try_from(start), u16::try_from(anchor)) {
            (Ok(start), Ok(anchor)) if before > 0 => {
                crate::disassemble::decode_aligned(start, &bytes, anchor, before, count as usize)
            },
            (Ok(start), _) => Some(crate::disassemble::decode(start, &bytes, count as usize)),
            _ => None
        };

        let Some(decoded) = decoded.filter(|d| !d.is_empty())
        else {
            // No alignment agrees with the address asked about - a region that
            // is mostly data. Guessing would put invented instructions in front
            // of the one being looked at, so the emulator answers this one.
            if self.peer.send(request.clone()).is_err() {
                let seq = self.next_seq();
                return vec![protocol::failure(
                    &request,
                    &format!("{} could not be read", address_reference(anchor)),
                    seq
                )];
            }
            return Vec::new();
        };

        let mut instructions = crate::disassemble::as_dap_instructions(&decoded);
        crate::inspect::annotate_disassembly(&mut instructions, &self.map, self.pc_page);

        let seq = self.next_seq();
        vec![protocol::response(
            &request,
            json!({ "instructions": instructions }),
            seq
        )]
    }

    /// Can this name be watched, and under what id?
    ///
    /// The editor asks before offering "Break When Value Changes" on a
    /// variable, and the `description` it gets back is shown in the UI - which
    /// is the right place to say that this emulator reports writes rather than
    /// stopping on them.
    fn data_breakpoint_info(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let name = request
            .get("arguments")
            .and_then(|a| a.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();

        let seq = self.next_seq();
        let address = parse_number(&name).or_else(|| self.map.address_of_symbol(&name));
        let Some(address) = address
        else {
            // A `null` dataId is the protocol's "not here", and the editor
            // shows the description as the reason.
            return Ok(vec![protocol::response(
                request,
                json!({
                    "dataId": Value::Null,
                    "description": format!("'{name}' is not a label of the program being debugged")
                }),
                seq
            )]);
        };

        Ok(vec![protocol::response(
            request,
            json!({
                "dataId": format!("{name}@{address}"),
                "description": format!(
                    "{name} ({}) - writes are reported in the Debug Console; this \
                     emulator cannot stop on them",
                    address_reference(address)
                ),
                "accessTypes": ["write"],
                "canPersist": false
            }),
            seq
        )])
    }

    /// Replace the editor's watchpoints.
    ///
    /// Like `setInstructionBreakpoints`, this request carries the whole set
    /// each time. The program's own `BREAKPOINT ... TYPE=MEM` watches are kept
    /// separately and are not the editor's to clear.
    fn set_data_breakpoints(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let requested = request
            .get("arguments")
            .and_then(|a| a.get("breakpoints"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        self.extra_watches.clear();
        let mut answers = Vec::new();
        for entry in &requested {
            let id = entry
                .get("dataId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            // `name@address`, as handed out above. The address is carried in
            // the id so a watch survives the label table being consulted twice.
            match id.rsplit_once('@').and_then(|(name, address)| {
                address.parse::<u32>().ok().map(|address| (name, address))
            }) {
                Some((name, address)) => {
                    self.extra_watches.push(WatchRequest {
                        address,
                        read: false,
                        write: true,
                        label: name.to_string()
                    });
                    answers.push(json!({
                        "verified": true,
                        "message": "writes are reported in the Debug Console"
                    }));
                },
                None => {
                    answers.push(json!({
                        "verified": false,
                        "message": format!("'{id}' cannot be watched")
                    }))
                },
            }
        }

        self.push_watches()?;
        let seq = self.next_seq();
        Ok(vec![protocol::response(
            request,
            json!({ "breakpoints": answers }),
            seq
        )])
    }

    /// `-chips` - the whole machine's chip state, printed once.
    ///
    /// The same data the CRTC/Gate Array scopes show, for when you want it in
    /// the transcript beside what you were doing rather than in a pane that
    /// updates under you.
    fn chips_command(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        if self.machine_state.is_none() {
            self.pending_chip_prints.push(request.clone());
            if self.pending_chip_scopes.is_empty() && self.pending_chip_prints.len() == 1 {
                self.send_own("cpclib/machineState", json!({}), Purpose::MachineState)?;
            }
            return Ok(Vec::new());
        }
        let seq = self.next_seq();
        let text = self.describe_chips();
        Ok(vec![protocol::response(
            request,
            json!({ "result": text, "variablesReference": 0 }),
            seq
        )])
    }

    /// Every chip scope, flattened into console text.
    fn describe_chips(&self) -> String {
        let Some(sna) = self.machine_state.as_deref()
        else {
            return "the emulator could not describe its machine state".to_string();
        };
        let mut out = String::new();
        for (name, reference) in [
            ("CRTC", crate::inspect::CRTC_REFERENCE),
            ("Gate Array", crate::inspect::GATE_ARRAY_REFERENCE),
            ("PSG", crate::inspect::PSG_REFERENCE),
            ("PPI", crate::inspect::PPI_REFERENCE),
            ("Disc", crate::inspect::DISC_REFERENCE)
        ] {
            out.push_str(&format!("\n{name}\n"));
            for variable in crate::inspect::chip_variables(reference, sna).unwrap_or_default() {
                out.push_str(&format!(
                    "  {:<18} {}\n",
                    variable["name"].as_str().unwrap_or_default(),
                    variable["value"].as_str().unwrap_or_default()
                ));
            }
        }
        out
    }

    /// Answer a chip scope, fetching the machine's state if we do not have it.
    fn chip_scope(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let reference = request
            .get("arguments")
            .and_then(|a| a.get("variablesReference"))
            .and_then(Value::as_i64)
            .unwrap_or_default();

        // Already have this instant's snapshot: answer without asking again.
        if let Some(sna) = self.machine_state.as_deref() {
            let variables = crate::inspect::chip_variables(reference, sna).unwrap_or_default();
            let seq = self.next_seq();
            return Ok(vec![protocol::response(
                request,
                json!({ "variables": variables }),
                seq
            )]);
        }

        // An emulator with an endpoint per chip is asked directly. Only the one
        // that has none has to save and parse a whole machine to answer.
        if let Some(command) = crate::amspiritlite::chip_command(reference)
            && self.peer.supports(command)
        {
            self.pending_chip_scopes.push(request.clone());
            self.send_own(command, json!({}), Purpose::MachineState)?;
            return Ok(Vec::new());
        }

        self.pending_chip_scopes.push(request.clone());
        // One request in flight, however many scopes are expanded: the others
        // are answered from the same snapshot when it arrives.
        if self.pending_chip_scopes.len() == 1 {
            self.send_own("cpclib/machineState", json!({}), Purpose::MachineState)?;
        }
        Ok(Vec::new())
    }

    /// The machine described itself; answer everyone who was waiting.
    fn complete_machine_state(&mut self, response: &Value) -> Vec<Value> {
        let waiting = std::mem::take(&mut self.pending_chip_scopes);
        let printing = std::mem::take(&mut self.pending_chip_prints);
        if waiting.is_empty() && printing.is_empty() {
            return Vec::new();
        }

        // An answer straight from a chip endpoint carries the values
        // themselves, not a snapshot to parse.
        if response
            .get("body")
            .is_some_and(|body| body.get("snapshot").is_none() && body.get("error").is_none())
        {
            let body = response.get("body").cloned().unwrap_or(json!({}));
            let mut out = Vec::new();
            for request in waiting {
                let reference = request
                    .get("arguments")
                    .and_then(|a| a.get("variablesReference"))
                    .and_then(Value::as_i64)
                    .unwrap_or_default();
                let seq = self.next_seq();
                out.push(protocol::response(
                    &request,
                    json!({
                        "variables": crate::amspiritlite::chip_variables(reference, &body)
                    }),
                    seq
                ));
            }
            return out;
        }

        let parsed = response
            .get("body")
            .and_then(|b| b.get("snapshot"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .filter(|bytes| !bytes.is_empty())
            .and_then(|bytes| cpclib_sna::Snapshot::from_buffer(bytes).ok());

        let why = response
            .get("body")
            .and_then(|b| b.get("error"))
            .and_then(Value::as_str)
            .unwrap_or("the emulator could not describe its machine state")
            .to_string();

        if let Some(sna) = parsed {
            self.machine_state = Some(Box::new(sna));
        }

        let mut out = Vec::new();
        for request in waiting {
            let reference = request
                .get("arguments")
                .and_then(|a| a.get("variablesReference"))
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let variables = match self.machine_state.as_deref() {
                Some(sna) => crate::inspect::chip_variables(reference, sna).unwrap_or_default(),
                None => crate::inspect::chip_placeholder(reference, &why).unwrap_or_default()
            };
            let seq = self.next_seq();
            out.push(protocol::response(
                &request,
                json!({ "variables": variables }),
                seq
            ));
        }
        for request in printing {
            let seq = self.next_seq();
            let text = self.describe_chips();
            out.push(protocol::response(
                &request,
                json!({ "result": text, "variablesReference": 0 }),
                seq
            ));
        }
        out
    }

    /// Where a watch expression points.
    ///
    /// A label, or a label with an offset: `demo_frame_run.wait_lines-1` is
    /// what you write when the byte you care about is the one *before* a
    /// label - the self-modifying-code idiom puts operands there constantly.
    ///
    /// Evaluated here rather than sent anywhere. The emulator implements no
    /// expression evaluation at all, but it never needed to: the symbol table
    /// and the arithmetic are both on this side, and only the final address is
    /// ever asked of it. So there is no limitation to report - the sum is done
    /// before the read.
    ///
    /// Terms are `+`/`-` separated and may be labels, registers or numbers in
    /// any of the notations used around here (`0x`, `&`, `#`, decimal).
    /// Deliberately no multiplication or parentheses: a watch is a place, and
    /// anything needing more than an offset is better written as a label.
    fn resolve_watch_address(&self, expression: &str) -> Option<u32> {
        let expression = expression.trim();
        if expression.is_empty() {
            return None;
        }

        // A label that is itself named with a `-` is one label, not a
        // subtraction. An exact match wins, so the arithmetic below only ever
        // sees expressions that are not already a place.
        if let Some(address) = self.map.address_of_symbol(expression) {
            return Some(address);
        }

        let mut total: i64 = 0;
        let mut negative = false;
        let mut term = String::new();
        let push = |term: &str, negative: bool, total: &mut i64| -> bool {
            let term = term.trim();
            if term.is_empty() {
                return false;
            }
            let Some(value) = self.term_value(term)
            else {
                return false;
            };
            if negative {
                *total -= value as i64;
            }
            else {
                *total += value as i64;
            }
            true
        };

        for character in expression.chars() {
            // A `-` inside a term is part of it: labels here legitimately
            // contain them, and so does nothing else at this point.
            if (character == '+' || character == '-') && !term.trim().is_empty() {
                if !push(&term, negative, &mut total) {
                    return None;
                }
                negative = character == '-';
                term.clear();
                continue;
            }
            term.push(character);
        }
        if !push(&term, negative, &mut total) {
            return None;
        }

        // The Z80's address space wraps, and a watch just before a label at 0
        // is a real thing to want.
        Some((total.rem_euclid(0x1_0000)) as u32)
    }

    /// One term of a watch expression: a number, a register, or a label.
    fn term_value(&self, term: &str) -> Option<u32> {
        if let Some(number) = parse_number(term) {
            return Some(number);
        }
        if let Some(value) = self.register_value(term) {
            return Some(value);
        }
        self.map.address_of_symbol(term)
    }

    /// Answer `hl`, `a`, `(hl)` and friends from the last stop.
    ///
    /// `None` when the expression is not about a register, so the caller falls
    /// through to the symbol table.
    fn evaluate_register(&mut self, request: &Value, expression: &str) -> Option<Vec<Value>> {
        let trimmed = expression.trim();

        // `(hl)` is the byte at that address - the thing you actually want to
        // know when a pointer register is involved.
        let indirect = trimmed.starts_with('(') && trimmed.ends_with(')');
        let name = if indirect {
            trimmed[1..trimmed.len() - 1].trim()
        }
        else {
            trimmed
        };

        let value = self.register_value(name)?;

        if indirect {
            // The byte has to come from the emulator; the watch machinery
            // already knows how to wait for one.
            self.pending_watches.push(PendingWatch {
                request: request.clone(),
                name: format!("({name})"),
                address: value,
                width: 1
            });
            let sent = self.send_own(
                "readMemory",
                json!({
                    "memoryReference": address_reference(value),
                    "count": 1
                }),
                Purpose::WatchRead
            );
            return match sent {
                Ok(()) => Some(Vec::new()),
                Err(problem) => {
                    self.pending_watches.pop();
                    let seq = self.next_seq();
                    Some(vec![protocol::failure(
                        request,
                        &format!("could not read ({name}): {problem}"),
                        seq
                    )])
                }
            };
        }

        // Eight-bit registers read as bytes, sixteen-bit ones as words, and a
        // register holding a known address says whose it is.
        let byte_wide = matches!(
            name.trim().to_ascii_uppercase().as_str(),
            "A" | "F" | "B" | "C" | "D" | "E" | "H" | "L" | "I" | "R" | "IM"
        );
        let mut rendered = if byte_wide {
            format!("0x{value:02X} ({value})")
        }
        else {
            format!("0x{value:04X} ({value})")
        };
        if !byte_wide && let Some(symbol) = self.map.symbol_at(value) {
            rendered.push_str(&format!(" {symbol}"));
        }
        if name.trim().eq_ignore_ascii_case("f") {
            rendered.push_str(&format!(" {}", crate::inspect::describe_flags(value as u8)));
        }

        let seq = self.next_seq();
        Some(vec![protocol::response(
            request,
            json!({
                "result": rendered,
                "type": "Z80 register",
                "variablesReference": 0,
                "memoryReference": address_reference(value)
            }),
            seq
        )])
    }

    /// Keep the register values the editor just fetched.
    fn remember_registers(&mut self, variables: &[Value]) {
        for variable in variables {
            let (Some(name), Some(value)) = (
                variable.get("name").and_then(Value::as_str),
                variable
                    .get("value")
                    .and_then(Value::as_str)
                    .and_then(parse_address_reference)
            )
            else {
                continue;
            };
            self.last_registers.insert(name.to_ascii_uppercase(), value);
        }
        // `A` and `F` are halves of `AF`, and worth having by name: `a` is what
        // anyone types.
        if let Some(af) = self.last_registers.get("AF").copied() {
            self.last_registers.insert("A".into(), (af >> 8) & 0xFF);
            self.last_registers.insert("F".into(), af & 0xFF);
        }
    }

    /// The value of a Z80 register by name, if the program has stopped.
    fn register_value(&self, name: &str) -> Option<u32> {
        const REGISTERS: &[&str] = &[
            "A", "F", "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC", "AF\'", "BC\'", "DE\'",
            "HL\'", "I", "R", "IM", "B", "C", "D", "E", "H", "L"
        ];
        let wanted = name.trim().to_ascii_uppercase();
        if !REGISTERS.contains(&wanted.as_str()) {
            return None;
        }
        if let Some(value) = self.last_registers.get(&wanted) {
            return Some(*value);
        }
        // The emulator reports pairs; the halves are derived on request rather
        // than stored, so `b` works without the pane having to list it.
        let (pair, high) = match wanted.as_str() {
            "B" => ("BC", true),
            "C" => ("BC", false),
            "D" => ("DE", true),
            "E" => ("DE", false),
            "H" => ("HL", true),
            "L" => ("HL", false),
            _ => return None
        };
        let value = *self.last_registers.get(pair)?;
        Some(if high {
            (value >> 8) & 0xFF
        }
        else {
            value & 0xFF
        })
    }

    /// A `-command` typed in the debug console.
    fn console_command(&mut self, request: &Value, line: &str) -> std::io::Result<Vec<Value>> {
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or_default();
        let arguments: Vec<&str> = words.collect();

        match command {
            "-mv" | "-memoryview" => self.memory_view(request, &arguments),
            "-dv" | "-disassemble" => self.disassembly_view(request, &arguments),
            "-chips" | "-crtc" | "-ga" => self.chips_command(request),
            "-timer" | "-t" => self.timer_command(request, &arguments),
            "-help" | "-h" => {
                let seq = self.next_seq();
                Ok(vec![protocol::response(
                    request,
                    json!({
                        "result": CONSOLE_HELP,
                        "variablesReference": 0
                    }),
                    seq
                )])
            },
            other => {
                let seq = self.next_seq();
                Ok(vec![protocol::failure(
                    request,
                    &format!("unknown command '{other}'. Type -help for the list."),
                    seq
                )])
            }
        }
    }

    /// `-mv <address|label> [count]` - open a memory view.
    fn memory_view(&mut self, request: &Value, arguments: &[&str]) -> std::io::Result<Vec<Value>> {
        let seq = self.next_seq();
        let Some(where_) = arguments.first()
        else {
            return Ok(vec![protocol::failure(
                request,
                "-mv needs an address or a label: -mv 0xC000 0x20",
                seq
            )]);
        };

        // A label is as good an answer as a number, and rather more likely to
        // be what someone wants to look at.
        let address = parse_number(where_).or_else(|| self.map.address_of_symbol(where_));
        let Some(address) = address
        else {
            let mut detail = format!("'{where_}' is neither an address nor a label");
            let similar = self.similar_symbols(where_);
            if !similar.is_empty() {
                detail.push_str(&format!(". Did you mean {}?", similar.join(", ")));
            }
            return Ok(vec![protocol::failure(request, &detail, seq)]);
        };

        let count = arguments
            .get(1)
            .and_then(|count| parse_number(count))
            .unwrap_or(0x40)
            .clamp(1, 0x1000) as usize;

        let label = self.map.symbol_at(address).map(str::to_owned);
        // Replaces whatever was open: there is one panel, and `-mv` on a new
        // address is a request to look at that instead.
        self.open_memory_view = Some(OpenMemoryView {
            address,
            count,
            label: label.clone(),
            previous: Vec::new()
        });
        self.pending_memory_views.push(PendingMemoryView {
            request: Some(request.clone()),
            label,
            address
        });
        self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(address),
                "count": count
            }),
            Purpose::MemoryView
        )?;
        Ok(Vec::new())
    }

    /// `-dv <address|label> [count]` - disassemble a region of memory.
    ///
    /// Deliberately a view over *memory*, not over the source: a demo's memory
    /// and its source differ in two ways that matter while debugging. A macro
    /// or a `REPEAT` turns one source line into a screenful of opcodes, and
    /// self-modifying code means the bytes running are not the bytes that were
    /// assembled. The source view answers "what did I write"; this answers
    /// "what is actually there", and each row links back to the former.
    fn disassembly_view(
        &mut self,
        request: &Value,
        arguments: &[&str]
    ) -> std::io::Result<Vec<Value>> {
        let seq = self.next_seq();

        // No argument means "where I am", which is what you want nine times out
        // of ten while stepping - and a view opened that way *follows* `PC`
        // afterwards rather than being left behind by the next step.
        let (anchor, address) = match arguments.first() {
            None => {
                let Some(pc) = self.last_pc
                else {
                    return Ok(vec![protocol::failure(
                        request,
                        "-dv with no address disassembles from PC, but the program has not \
                         stopped yet. Give it a place to look: -dv 0x4000 32",
                        seq
                    )]);
                };
                (DisassemblyAnchor::ProgramCounter, pc as u32)
            },
            Some(where_) => {
                let address = parse_number(where_).or_else(|| self.map.address_of_symbol(where_));
                let Some(address) = address
                else {
                    let mut detail = format!("'{where_}' is neither an address nor a label");
                    let similar = self.similar_symbols(where_);
                    if !similar.is_empty() {
                        detail.push_str(&format!(". Did you mean {}?", similar.join(", ")));
                    }
                    return Ok(vec![protocol::failure(request, &detail, seq)]);
                };
                (DisassemblyAnchor::Fixed(address), address)
            }
        };

        let count = arguments
            .get(1)
            .and_then(|count| parse_number(count))
            .unwrap_or(32)
            .clamp(1, 512) as i64;

        let label = self.map.symbol_at(address).map(str::to_owned);
        // Replaces whatever was open: there is one panel, and `-dv` elsewhere
        // is a request to look there instead.
        self.open_disassembly_view = Some(OpenDisassemblyView {
            anchor,
            count,
            label: label.clone(),
            fetched_at: Some(address)
        });
        self.ask_for_disassembly(address, count, label, Some(request.clone()))?;
        Ok(Vec::new())
    }

    /// Ask the emulator for instructions, for a view or for a refresh.
    fn ask_for_disassembly(
        &mut self,
        address: u32,
        count: i64,
        label: Option<String>,
        request: Option<Value>
    ) -> std::io::Result<()> {
        self.pending_disassembly.push(PendingDisassembly {
            request,
            label,
            address
        });
        // Bytes, not a disassembly. The emulator can decode them, but its
        // mnemonics are *its* mnemonics: swap the emulator and the view changes
        // under you for a program that has not. Reading the bytes and decoding
        // them with the assembler's own tables makes this view read like the
        // source it sits beside, whatever is running underneath.
        //
        // Four bytes per instruction is the Z80's worst case, so this always
        // covers the instructions asked for and usually overshoots - the
        // decoder stops at `count` regardless.
        let bytes = ((count as usize) * 4).clamp(1, 0x1000);
        self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(address),
                "count": bytes
            }),
            Purpose::DisassemblyView
        )
    }

    /// Re-read whatever the disassembly view is showing.
    ///
    /// A `PC`-anchored view moves with the program, so the panel and the source
    /// can be read side by side at every step - which matters here because the
    /// two genuinely differ: a macro turns one line into a screenful of
    /// opcodes, and self-modifying code means the bytes running are not the
    /// bytes that were assembled.
    fn refresh_disassembly_view(&mut self) {
        let Some(open) = self.open_disassembly_view.as_ref()
        else {
            return;
        };
        let (anchor, count, fetched_at) = (open.anchor, open.count, open.fetched_at);

        let address = match anchor {
            // A fixed view is re-read on every stop because the *bytes* may
            // have changed - self-modifying code is the reason to be looking at
            // memory rather than at the source in the first place.
            DisassemblyAnchor::Fixed(address) => address,
            // A following view is re-read when `PC` has moved. `stopped`
            // arrives before anyone has asked where we now are, so this is
            // driven by `PC` becoming known rather than by the stop itself -
            // refreshing on the stop would show the *previous* instruction.
            DisassemblyAnchor::ProgramCounter => {
                let Some(pc) = self.last_pc.map(u32::from)
                else {
                    return;
                };
                if fetched_at == Some(pc) {
                    return;
                }
                pc
            }
        };

        let label = self.map.symbol_at(address).map(str::to_owned);
        if let Some(open) = self.open_disassembly_view.as_mut() {
            open.label = label.clone();
            open.fetched_at = Some(address);
        }
        // A failure here is not worth reporting: the panel keeps what it had,
        // and the stop event must not be lost behind it.
        let _ = self.ask_for_disassembly(address, count, label, None);
    }

    /// Note where the program is, charge the timers, and move a following
    /// view with it.
    fn note_program_counter(&mut self, pc: u16) {
        if self.last_pc == Some(pc) {
            return;
        }
        if let Some(previous) = self.last_pc {
            self.charge_timers(previous, pc);
        }
        self.last_pc = Some(pc);
        self.refresh_disassembly_view();
    }

    /// Add to every running timer what the step from `previous` to `now` cost.
    ///
    /// Recognised only when the program moved by exactly the length of the
    /// instruction at `previous` - that is a step, and its cost is known. A
    /// jump, a call, or a free run between two breakpoints is *not*, and no
    /// amount of arithmetic here can recover it; the timers are marked
    /// inexact instead of being fed a guess.
    fn charge_timers(&mut self, previous: u16, now: u16) {
        if self.timers.is_empty() {
            return;
        }
        match self.step_cost(previous, now) {
            Some(nops) => {
                for timer in &mut self.timers {
                    timer.nops += nops as u64;
                }
            },
            None => {
                for timer in &mut self.timers {
                    timer.exact = false;
                }
            },
        }
    }

    /// The NOP cost of the instruction at `previous`, if the program simply
    /// stepped over it.
    fn step_cost(&self, previous: u16, now: u16) -> Option<usize> {
        let instruction = self.instruction_in_image(previous)?;
        if now != previous.wrapping_add(instruction.bytes.len() as u16) {
            return None;
        }
        instruction.cost
    }

    /// The instruction the machine really holds at `address`, decoded from the
    /// assembled program.
    ///
    /// Read from the image rather than from the emulator: a stop already costs
    /// several round trips, and one more on every stop is exactly what made
    /// earlier panes feel slow. Code that rewrites itself is the price, and it
    /// is the price the call stack and the timers already pay.
    fn instruction_in_image(&self, address: u16) -> Option<crate::disassemble::Instruction> {
        self.image.as_ref()?;
        // Four bytes is the Z80's longest instruction.
        let window: Vec<u8> = (0..4)
            .filter_map(|offset| {
                self.image_byte(self.pc_page.unwrap_or(0), address.wrapping_add(offset))
            })
            .collect();
        crate::disassemble::decode(address, &window, 1)
            .into_iter()
            .next()
    }

    /// `-timer ...` - the stopwatches.
    fn timer_command(
        &mut self,
        request: &Value,
        arguments: &[&str]
    ) -> std::io::Result<Vec<Value>> {
        let seq = self.next_seq();
        let (action, name) = match arguments.split_first() {
            None => ("list", None),
            Some((action, rest)) => {
                let name = rest.first().map(|n| n.to_string());
                (*action, name)
            }
        };

        let result = match action {
            "add" | "new" | "start" => {
                let name = name.unwrap_or_else(|| format!("timer{}", self.timers.len() + 1));
                if self.timers.iter().any(|t| t.name == name) {
                    format!("there is already a timer called '{name}'")
                }
                else {
                    self.timers.push(Timer {
                        name: name.clone(),
                        nops: 0,
                        exact: true
                    });
                    format!("timer '{name}' started at 0 NOPs")
                }
            },
            "reset" | "zero" => {
                let mut touched = Vec::new();
                for timer in &mut self.timers {
                    if name.as_deref().is_none_or(|wanted| wanted == timer.name) {
                        timer.nops = 0;
                        timer.exact = true;
                        touched.push(timer.name.clone());
                    }
                }
                if touched.is_empty() {
                    "no such timer".to_string()
                }
                else {
                    format!("reset {}", touched.join(", "))
                }
            },
            "rm" | "del" | "delete" | "stop" => {
                let before = self.timers.len();
                match name.as_deref() {
                    Some(wanted) => self.timers.retain(|t| t.name != wanted),
                    None => self.timers.clear()
                }
                format!("removed {} timer(s)", before - self.timers.len())
            },
            "list" => self.describe_timers(),
            other => {
                return Ok(vec![protocol::failure(
                    request,
                    &format!(
                        "unknown timer action '{other}'. Try: -timer add [name], \
                         -timer reset [name], -timer rm [name], -timer"
                    ),
                    seq
                )]);
            }
        };

        Ok(vec![protocol::response(
            request,
            json!({ "result": result, "variablesReference": 0 }),
            seq
        )])
    }

    fn describe_timers(&self) -> String {
        if self.timers.is_empty() {
            return "no timers. `-timer add [name]` starts one.".to_string();
        }
        self.timers
            .iter()
            .map(|timer| {
                let qualifier = if timer.exact {
                    ""
                }
                else {
                    " (at least; the program ran on unobserved)"
                };
                format!("{}: {} NOPs{}", timer.name, timer.nops, qualifier)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The timers, for the variables pane.
    pub fn timer_variables(&self) -> Vec<Value> {
        self.timers
            .iter()
            .map(|timer| {
                json!({
                    "name": timer.name,
                    "value": if timer.exact {
                        format!("{} NOPs", timer.nops)
                    }
                    else {
                        format!(">= {} NOPs", timer.nops)
                    },
                    "type": if timer.exact {
                        "exact: every instruction since the reset was observed"
                    }
                    else {
                        "a floor, not a measurement: the program ran on between stops \
                         and this emulator reports no elapsed time"
                    },
                    "variablesReference": 0
                })
            })
            .collect()
    }

    /// The bytes came back; decode them and hand the instructions to the panel.
    fn complete_disassembly_view(&mut self, response: &Value) -> Vec<Value> {
        if self.pending_disassembly.is_empty() {
            return Vec::new();
        }
        let view = self.pending_disassembly.remove(0);

        let bytes = response
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default();

        let count = self
            .open_disassembly_view
            .as_ref()
            .map(|open| open.count as usize)
            .unwrap_or(32);
        let decoded = u16::try_from(view.address)
            .ok()
            .map(|address| crate::disassemble::decode(address, &bytes, count))
            .unwrap_or_default();

        if decoded.is_empty() {
            let Some(request) = view.request
            else {
                // A refresh that read nothing: the panel keeps what it had.
                return Vec::new();
            };
            let seq = self.next_seq();
            return vec![protocol::failure(
                &request,
                &format!(
                    "{} could not be disassembled",
                    address_reference(view.address)
                ),
                seq
            )];
        }

        let mut instructions = crate::disassemble::as_dap_instructions(&decoded);
        crate::inspect::annotate_disassembly(&mut instructions, &self.map, self.pc_page);

        let seq = self.next_seq();
        let event = protocol::event(
            "cpclib/disassemblyView",
            json!({
                "address": view.address,
                "label": view.label,
                "instructions": instructions,
                // So the panel can mark where the program actually is.
                "pc": self.last_pc,
                "followsPc": matches!(
                    self.open_disassembly_view.as_ref().map(|open| open.anchor),
                    Some(DisassemblyAnchor::ProgramCounter)
                )
            }),
            seq
        );

        let Some(request) = view.request
        else {
            return vec![event];
        };
        let seq = self.next_seq();
        let receipt = protocol::response(
            &request,
            json!({
                "result": format!(
                    "{} instructions from {}{}",
                    decoded.len(),
                    address_reference(view.address),
                    view.label.map(|l| format!(" ({l})")).unwrap_or_default()
                ),
                "variablesReference": 0,
                "memoryReference": address_reference(view.address)
            }),
            seq
        );
        vec![event, receipt]
    }

    /// The bytes came back; hand them to the panel and say so in the console.
    ///
    /// Two messages rather than one: the event carries the dump to a view that
    /// stays open and can be refreshed, and the console line is the receipt for
    /// what was just typed there.
    fn complete_memory_view(&mut self, response: &Value) -> Vec<Value> {
        if self.pending_memory_views.is_empty() {
            return Vec::new();
        }
        let view = self.pending_memory_views.remove(0);
        let bytes = response
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default();

        if bytes.is_empty() {
            let Some(request) = view.request
            else {
                // A refresh that read nothing: the panel keeps what it had
                // rather than blanking, and nothing is said.
                return Vec::new();
            };
            let seq = self.next_seq();
            return vec![protocol::failure(
                &request,
                &format!("{} could not be read", address_reference(view.address)),
                seq
            )];
        }

        // Which bytes moved since the last look. Empty on the first, where
        // everything would otherwise be "changed".
        let changed: Vec<usize> = match self.open_memory_view.as_ref() {
            Some(open) if open.previous.len() == bytes.len() => {
                (0..bytes.len())
                    .filter(|i| open.previous[*i] != bytes[*i])
                    .collect()
            },
            _ => Vec::new()
        };
        if let Some(open) = self.open_memory_view.as_mut() {
            open.previous = bytes.clone();
        }

        // Labels inside the range, so the panel can mark where each one starts
        // rather than leaving the reader to count bytes.
        let marks: Vec<Value> = (0..bytes.len())
            .filter_map(|offset| {
                let address = view.address + offset as u32;
                self.map
                    .symbol_at(address)
                    .map(|name| json!({ "offset": offset, "name": name }))
            })
            .collect();

        let seq = self.next_seq();
        let event = protocol::event(
            "cpclib/memoryView",
            json!({
                "address": view.address,
                "label": view.label,
                "bytes": bytes,
                "marks": marks,
                "changed": changed
            }),
            seq
        );

        let Some(request) = view.request
        else {
            return vec![event];
        };
        let seq = self.next_seq();
        let receipt = protocol::response(
            &request,
            json!({
                "result": format!(
                    "{} bytes from {}{}",
                    bytes.len(),
                    address_reference(view.address),
                    view.label.map(|l| format!(" ({l})")).unwrap_or_default()
                ),
                "variablesReference": 0,
                "memoryReference": address_reference(view.address)
            }),
            seq
        );
        vec![event, receipt]
    }

    /// Re-read whatever the memory view is showing.
    ///
    /// A memory view is something you keep open while stepping, and one that
    /// shows what memory looked like three steps ago is worse than no view at
    /// all - it looks current.
    fn refresh_memory_view(&mut self) {
        let Some(open) = self.open_memory_view.as_ref()
        else {
            return;
        };
        let (address, count, label) = (open.address, open.count, open.label.clone());
        self.pending_memory_views.push(PendingMemoryView {
            request: None,
            label,
            address
        });
        // A failure here is not worth reporting: the panel simply keeps what it
        // had, and the stop event must not be lost behind it.
        let _ = self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(address),
                "count": count
            }),
            Purpose::MemoryView
        );
    }

    /// Bring the suppressed set in line with what the editor just said about
    /// one file.
    ///
    /// A program breakpoint whose address belongs to `file` is suppressed
    /// unless the editor listed it; listing it again brings it back. Addresses
    /// in other files are untouched.
    fn resync_suppressions(&mut self, file: &Path, placed: &[PlacedBreakpoint]) {
        let wanted: std::collections::HashSet<u32> =
            placed.iter().filter_map(|bp| bp.address).collect();

        let addresses: Vec<u32> = self
            .program_breakpoints
            .iter()
            .filter(|bp| bp.watch.is_none() && !bp.one_shot)
            .map(|bp| bp.address)
            .collect();

        for address in addresses {
            // Whether this breakpoint is in the file being spoken about.
            //
            // Every page that claims the address is asked, not just the
            // unambiguous answer: `location_at` returns nothing at all for an
            // address two pages both hold, so on a paged program this said
            // "belongs to no file" about every such breakpoint and clearing the
            // red dot did nothing at all. If any page's row for this address is
            // in the file the user just clicked in, they are speaking about it.
            let in_this_file = self
                .map
                .candidates_at(address)
                .iter()
                .any(|(_, location)| same_file(&location.file, file));
            if !in_this_file {
                continue;
            }
            if wanted.contains(&address) {
                self.suppressed.remove(&address);
            }
            else {
                self.suppressed.insert(address);
            }
        }
    }

    /// Labels that look like what was typed, for a watch that found nothing.
    fn similar_symbols(&self, wanted: &str) -> Vec<String> {
        let wanted_lower = wanted.to_ascii_lowercase();
        // Containment catches "the label I typed is part of a longer one";
        // edit distance catches the far more common case of a typo, which
        // containment misses entirely - `scroll_ofset` contains none of
        // `scroll_offset` and is contained by none of it.
        let mut matches: Vec<(usize, String)> = self
            .map
            .symbols()
            .map(|(name, _)| name.to_string())
            .filter_map(|name| {
                let lower = name.to_ascii_lowercase();
                if lower.contains(&wanted_lower) || wanted_lower.contains(&lower) {
                    return Some((0, name));
                }
                let distance = edit_distance(&lower, &wanted_lower, 2)?;
                Some((distance, name))
            })
            .collect();
        matches.sort();
        matches.truncate(5);
        matches.into_iter().map(|(_, name)| name).collect()
    }

    /// Hand the editor the contents of a source file.
    fn source(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let arguments = request.get("arguments").cloned().unwrap_or(json!({}));
        let path = arguments
            .get("source")
            .and_then(|s| s.get("path"))
            .and_then(Value::as_str)
            .map(PathBuf::from);

        // A name with no usable path can still be found: it is almost always a
        // file of the program being debugged, and the map knows where those are.
        let path = path.filter(|p| p.is_absolute()).or_else(|| {
            let name = arguments
                .get("source")
                .and_then(|s| s.get("name"))
                .and_then(Value::as_str)?;
            self.map
                .files()
                .iter()
                .find(|known| known.file_name().is_some_and(|f| f == name))
                .cloned()
        });

        let seq = self.next_seq();
        match path.as_deref().map(std::fs::read_to_string) {
            Some(Ok(content)) => {
                Ok(vec![protocol::response(
                    request,
                    json!({ "content": content }),
                    seq
                )])
            },
            Some(Err(problem)) => {
                Ok(vec![protocol::failure(
                    request,
                    &format!("{} could not be read: {problem}", path.unwrap().display()),
                    seq
                )])
            },
            None => {
                Ok(vec![protocol::failure(
                    request,
                    "that source is not part of the program being debugged",
                    seq
                )])
            },
        }
    }

    /// Send the union of every file's breakpoints, because the emulator
    /// replaces its whole set on each call.
    fn push_breakpoints(&mut self) -> std::io::Result<()> {
        if !self.attached {
            return Ok(());
        }
        // Sorted by address: the emulator sees a stable set whichever order
        // the editor happened to send the files in, and the order is one a
        // human reading the traffic can follow.
        let mut addresses: Vec<u32> = self
            .breakpoints
            .values()
            .flatten()
            .filter_map(|bp| bp.address)
            // Plus what the source itself asked for. A `BREAKPOINT` directive
            // is not something the editor knows about, so it would otherwise be
            // wiped by the next red-dot change.
            .chain(
                self.program_breakpoints
                    .iter()
                    .filter(|bp| bp.watch.is_none() && !self.suppressed.contains(&bp.address))
                    .map(|bp| bp.address)
            )
            // The one under our feet, while stepping off it.
            .filter(|address| Some(*address) != self.stepped_off)
            .collect();
        addresses.sort_unstable();
        addresses.dedup();
        let references: Vec<Value> = addresses
            .into_iter()
            .map(|address| json!({ "instructionReference": address_reference(address) }))
            .collect();
        self.send_own_request(
            "setInstructionBreakpoints",
            json!({ "breakpoints": references })
        )
    }

    /// Watch labels the launch configuration asked for, on top of the ones the
    /// program's own `BREAKPOINT` directives declared.
    ///
    /// Returns what could not be resolved: a label that is not in the program
    /// is almost always a typo, and silently watching nothing is the least
    /// helpful possible response to one.
    pub fn add_watch_labels(&mut self, labels: &[String]) -> Vec<String> {
        let mut problems = Vec::new();
        for label in labels {
            match self.map.address_of_symbol(label) {
                Some(address) => {
                    self.extra_watches.push(WatchRequest {
                        address,
                        read: false,
                        write: true,
                        label: label.clone()
                    })
                },
                None => {
                    let mut problem =
                        format!("\"{label}\" is in watchLabels but not in the program");
                    let similar = self.similar_symbols(label);
                    if !similar.is_empty() {
                        problem.push_str(&format!(" - did you mean {}?", similar.join(", ")));
                    }
                    problems.push(problem);
                }
            }
        }
        problems
    }

    /// Break at the address the program starts from.
    ///
    /// Kept with the program's own breakpoints rather than the editor's, for
    /// the same reason: the editor did not ask for it and must not be able to
    /// clear it by clearing a file's red dots.
    pub fn stop_on_entry(&mut self, address: u16) {
        self.program_breakpoints.push(ProgramBreakpoint {
            address: address as u32,
            watch: None,
            one_shot: true,
            // No directive to point at: this one is the launch configuration's.
            written_at: None
        });
    }

    /// Retire the breakpoints that were only meant to fire once.
    ///
    /// Returns whether anything went, so the caller only pays for a re-push
    /// when there is something to re-push - which is once per session at most.
    fn retire_one_shot_breakpoints(&mut self) -> bool {
        let before = self.program_breakpoints.len();
        self.program_breakpoints.retain(|bp| !bp.one_shot);
        before != self.program_breakpoints.len()
    }

    /// Arm the emulator's write-watch channels with everything being watched.
    ///
    /// Not a DAP request: the channels are on the emulator itself rather than
    /// on its DAP session, so the page-side bridge answers this one. Sending
    /// nothing is still worth doing - it clears whatever a previous session
    /// left armed.
    fn push_watches(&mut self) -> std::io::Result<()> {
        let watches: Vec<Value> = self
            .watch_requests()
            .into_iter()
            .map(|watch| {
                json!({
                    "label": watch.label,
                    "address": watch.address,
                    "read": watch.read,
                    "write": watch.write
                })
            })
            .collect();
        // Sent even when empty: that is what disarms a channel the editor just
        // cleared, and what clears whatever a previous session left behind.
        self.send_own(
            "cpclib/setWatches",
            json!({ "watches": watches }),
            Purpose::WatchArm
        )
    }

    /// Say which watches took and which did not.
    ///
    /// The emulator has a fixed number of channels, and the number is its
    /// business rather than something written down here. What matters is that a
    /// label which did not fit is named, because a watch that silently does
    /// nothing looks exactly like a variable that is never written.
    fn report_armed_watches(&mut self, response: &Value) -> Vec<Value> {
        let rejected: Vec<String> = response
            .get("body")
            .and_then(|b| b.get("rejected"))
            .and_then(Value::as_array)
            .map(|names| {
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        if rejected.is_empty() {
            return Vec::new();
        }
        let seq = self.next_seq();
        vec![protocol::event(
            "output",
            json!({
                "category": "console",
                "output": format!(
                    "the emulator has no free watch channel for {}; \
                     writes to those will not be reported.\n",
                    rejected.join(", ")
                )
            }),
            seq
        )]
    }

    /// The NOP cost of the source line the program counter is on.
    fn cost_at_pc(&mut self, variables: &[Value]) -> Option<usize> {
        let pc = variables
            .iter()
            .find(|variable| variable.get("name").and_then(Value::as_str) == Some("PC"))
            .and_then(|variable| variable.get("value"))
            .and_then(Value::as_str)
            .and_then(parse_address_reference)?;

        // The register pane is asked for on every stop, whether or not anyone
        // asked for a stack trace - so this is the reliable place to learn
        // where the program is, and a session with no program image (and thus
        // no stack walk) still gets a working `-dv`.
        let pc = u16::try_from(pc).ok()?;
        self.note_program_counter(pc);
        self.line_cost(pc)
    }

    /// What the source says about the line the program is standing on.
    ///
    /// The one thing about a step over the emulator cannot work out for
    /// itself. `defs 60` - the raster-timing idiom - assembles to sixty
    /// `NOP`s, and sixty `NOP`s are exactly what a hand-written `nop` looks
    /// like; only the source text tells the two apart, and the source lives on
    /// this side of the seam. The run itself comes from the source map, which
    /// already grows a line's extent by adjacency so a line a macro or a
    /// `repeat` emitted several times yields only the copy being executed.
    ///
    /// Three answers rather than two, because "this line is not a `defs`" and
    /// "there is no line" are different things to the emulator: the first says
    /// step one instruction, the second leaves it to fall back on the bytes.
    fn line_at_pc(&mut self) -> LineAtPc {
        let Some(pc) = self.last_pc
        else {
            return LineAtPc::Unknown;
        };
        let page = self.pc_page.unwrap_or(0);
        let Some(at) = self.map.location_at_long(page, pc)
        else {
            return LineAtPc::Unknown;
        };
        let Some(text) = self.source_line(&at.file, at.line)
        else {
            // The map named a line and the file it is in cannot be read: the
            // listing is describing a program whose source has moved or gone,
            // so there is nothing to consult after all.
            return LineAtPc::Unknown;
        };
        if !is_a_defs_directive(&text) {
            return LineAtPc::Ordinary;
        }
        let Some(run) = self
            .map
            .line_extent_at(page, pc)
            .and_then(|run| Some(u16::try_from(run.start).ok()?..u16::try_from(run.end).ok()?))
        else {
            // A run reaching the very top of memory has no address after it to
            // stop on. The line is still a `defs`, so it is not for the bytes
            // to guess at either.
            return LineAtPc::Ordinary;
        };
        LineAtPc::Defs(run)
    }

    /// What the source line covering `address` costs to execute, in NOPs.
    ///
    /// Priced from the program's own bytes, not from its text. The bytes are
    /// what the Z80 fetches, so this answers for a line the parser could not
    /// have answered for at all: a macro call, a `defs` run - which executes
    /// as `NOP`s and is how a demo pads a raster line - or a region built at
    /// runtime that has no source text anywhere.
    ///
    /// The line rather than the single instruction, because one basm line is
    /// routinely several instructions (`ld a,0 : ld b,0`) and it is the line
    /// the user is looking at. The source map gives the run of addresses that
    /// line occupies; the assembler prices each instruction in it.
    ///
    /// Falls back to the instruction at `address` alone when no line claims it
    /// - generated code, or a jump into the firmware - where one instruction
    /// is the only honest unit left.
    fn line_cost(&self, address: u16) -> Option<usize> {
        let page = self.pc_page.unwrap_or(0);
        let Some(extent) = self.map.line_extent_at(page, address)
        else {
            return self.instruction_in_image(address)?.cost;
        };

        let start = u16::try_from(extent.start).ok()?;
        let bytes: Vec<u8> = (extent.start..extent.end)
            .map(|at| {
                u16::try_from(at)
                    .ok()
                    .and_then(|at| self.image_byte(page, at))
            })
            .collect::<Option<Vec<u8>>>()?;

        crate::disassemble::nops(&crate::disassemble::decode(start, &bytes, bytes.len()))
    }

    /// One line of a source file, from a cache.
    ///
    /// Stepping asks for the same handful of lines over and over, and the
    /// files are the ones this session was built from - they are not going to
    /// change under us without the session being restarted.
    fn source_line(&mut self, file: &Path, line: u32) -> Option<String> {
        if !self.source_cache.contains_key(file) {
            let lines = std::fs::read_to_string(file)
                .ok()?
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            self.source_cache.insert(file.to_path_buf(), lines);
        }
        self.source_cache
            .get(file)?
            .get(line.checked_sub(1)? as usize)
            .cloned()
    }

    /// The byte at an address in the assembled program, if we have its image.
    ///
    /// The image is the whole of the CPC's memory as assembled, laid out by
    /// `offset_in_cpc()`: page 0 first, then one 64K block per extra page. So
    /// the same logical address in two pages is two different offsets, and the
    /// page selects between them.
    fn image_byte(&self, page: u8, address: u16) -> Option<u8> {
        let offset = address as usize + page as usize * 0x1_0000;
        self.image.as_ref()?.get(offset).copied()
    }

    /// Which page's assembled bytes look most like what is really at `address`.
    ///
    /// The heuristic for a paged program: the emulator cannot say which page
    /// is paged in, but it can say what the bytes *are*, and only one page was
    /// assembled to hold those bytes. Self-modifying code makes this a
    /// best-match rather than an exact one - a routine that has patched itself
    /// still matches its own page far better than it matches another.
    ///
    /// `None` when nothing matches better than everything else: a tie is a
    /// genuine "cannot tell", and guessing between two equal candidates is how
    /// the wrong line gets highlighted.
    fn page_matching(&self, address: u16, actual: &[u8], among: &[u8]) -> Option<u8> {
        if actual.is_empty() || among.is_empty() {
            return None;
        }
        if let [only] = among {
            // Nothing to choose between. Worth stating rather than falling into
            // the scoring below, which would refuse a single candidate whose
            // image we cannot read.
            return Some(*only);
        }

        let mut scored: Vec<(usize, u8)> = among
            .iter()
            .map(|page| {
                let matches = actual
                    .iter()
                    .enumerate()
                    .filter(|(offset, byte)| {
                        let at = address.wrapping_add(*offset as u16);
                        self.image_byte(*page, at) == Some(**byte)
                    })
                    .count();
                (matches, *page)
            })
            .collect();
        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        match scored.as_slice() {
            [] => None,
            [(score, page)] => (*score > 0).then_some(*page),
            [(best, page), (second, _), ..] => {
                // A clear winner only. Equal scores mean the pages hold the
                // same bytes here, which is exactly the case where picking one
                // would be a coin toss presented as an answer.
                (*best > *second).then_some(*page)
            }
        }
    }

    /// The address of the innermost frame in a `stackTrace` response.
    fn frame_address(response: &Value) -> Option<u16> {
        response
            .get("body")?
            .get("stackFrames")?
            .as_array()?
            .first()?
            .get("instructionPointerReference")?
            .as_str()
            .and_then(parse_address_reference)
            .and_then(|address| u16::try_from(address).ok())
    }

    /// Ask what is really in memory at `address`, to choose between the pages
    /// that claim it.
    fn ask_which_page(&mut self, address: u16) -> std::io::Result<()> {
        // An emulator that knows is simply asked.
        //
        // The byte comparison below exists because 1984js cannot report its
        // banking state at all - it is a heuristic, and it fails honestly but
        // uselessly when two pages hold the same bytes. A backend that can
        // answer makes the whole question a lookup.
        if self.peer.supports("cpclib/memmap") {
            return self.send_own("cpclib/memmap", json!({}), Purpose::PageProbe);
        }

        self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(address as u32),
                // Enough to tell two pages apart without being a second stack
                // read: a dozen bytes of real code differ in more places than
                // one.
                "count": PAGE_PROBE_BYTES
            }),
            Purpose::PageProbe
        )
    }

    /// The pages that would have to be told apart for `address` to resolve.
    ///
    /// Empty when the address already resolves to one line, or to none at all -
    /// in both cases there is nothing a byte comparison could add.
    fn pages_to_tell_apart(&self, address: u32) -> Vec<u8> {
        let candidates = self.map.candidates_at(address);
        if candidates.len() < 2 {
            return Vec::new();
        }
        candidates.into_iter().map(|(page, _)| page).collect()
    }

    /// Begin recovering the frames above the one the emulator reported.
    ///
    /// The emulator answers `stackTrace` with the program counter and nothing
    /// else, because that is all it knows: the rest is on the stack, and only
    /// something holding the assembled program can tell a return address from
    /// a number that looks like one. Its answer is held here while `SP` and the
    /// stack are fetched - three round trips, once per stop - rather than being
    /// sent out short and corrected afterwards, which makes the editor jump.
    fn begin_stack_walk(&mut self, response: &Value) -> Option<Vec<Value>> {
        if response.get("success").and_then(Value::as_bool) != Some(true) {
            return None;
        }
        let frame_id = response
            .get("body")?
            .get("stackFrames")?
            .as_array()?
            .first()?
            .get("id")?
            .as_i64()?;

        // The page this stop is at has not been worked out yet, and last
        // stop's answer is about last stop.
        self.pc_page = None;

        // An emulator that knows its own paging is asked before anything else,
        // and asked for *every* stop rather than only when a stack walk
        // happens to run.
        //
        // This is the whole of the `0x79F3` bug: two files hold code at that
        // logical address, page 0's and page 1's, and the answer was falling
        // back to the lower page - so a breakpoint in `animate.asm` opened
        // `writter.asm`. The assembler was right all along; its listing records
        // the physical address `0x179F3`, which names page 1 unambiguously.
        //
        // No program image is needed for this: the page comes from the Gate
        // Array's MMR, not from comparing bytes.
        if self.peer.supports("cpclib/memmap")
            && let Some(pc) = Self::frame_address(response)
            && !self.pages_to_tell_apart(pc as u32).is_empty()
        {
            let superseded = self.pending_page_probe.take().map(|(held, ..)| held);
            self.pending_page_probe = Some((response.clone(), Vec::new(), pc));
            if self.ask_which_page(pc).is_err() {
                let (held, frames, _) = self.pending_page_probe.take().unwrap();
                return Some(self.finish_stack_walk(held, frames));
            }
            return Some(
                superseded
                    .map(|response| self.annotate_stack_trace(&response))
                    .unwrap_or_default()
            );
        }

        // Otherwise the page has to be guessed from the bytes, and that needs
        // the program's own image to compare against. Falling through leaves
        // the honest report rather than spending a read to learn nothing.
        self.image.as_ref()?;

        // A walk already in flight means the editor asked twice; the newer
        // question is the one worth answering, and the older response is
        // released so the editor is not left waiting on it.
        let superseded = self.pending_stack.take().map(|held| held.response);
        self.pending_stack = Some(PendingStack {
            response: response.clone(),
            sp: None
        });
        if self
            .send_own(
                "scopes",
                json!({ "frameId": frame_id }),
                Purpose::StackScopes
            )
            .is_err()
        {
            // The emulator is gone; the single frame is better than nothing.
            let held = self.pending_stack.take();
            return held.map(|held| self.finish_stack_walk(held.response, Vec::new()));
        }
        superseded.map(|response| self.annotate_stack_trace(&response))
    }

    /// The register scope came back; ask it for its variables.
    fn stack_step_registers(&mut self, response: &Value) -> Vec<Value> {
        let reference = response
            .get("body")
            .and_then(|b| b.get("scopes"))
            .and_then(Value::as_array)
            .and_then(|scopes| {
                scopes.iter().find(|scope| {
                    scope.get("presentationHint").and_then(Value::as_str) == Some("registers")
                        || scope.get("name").and_then(Value::as_str) == Some("Registers")
                })
            })
            .and_then(|scope| scope.get("variablesReference"))
            .and_then(Value::as_i64);

        let Some(reference) = reference
        else {
            return self.abandon_stack_walk();
        };
        if self
            .send_own(
                "variables",
                json!({ "variablesReference": reference }),
                Purpose::StackRegisters
            )
            .is_err()
        {
            return self.abandon_stack_walk();
        }
        Vec::new()
    }

    /// The registers came back; read the stack from `SP`.
    fn stack_step_read(&mut self, response: &Value) -> Vec<Value> {
        let sp = response
            .get("body")
            .and_then(|b| b.get("variables"))
            .and_then(Value::as_array)
            .and_then(|variables| {
                variables
                    .iter()
                    .find(|variable| variable.get("name").and_then(Value::as_str) == Some("SP"))
            })
            .and_then(|variable| variable.get("value"))
            .and_then(Value::as_str)
            .and_then(parse_address_reference)
            .and_then(|address| u16::try_from(address).ok());

        let Some(sp) = sp
        else {
            return self.abandon_stack_walk();
        };
        let count = crate::callstack::bytes_to_read(sp, self.top_of_stack);
        if count == 0 {
            return self.abandon_stack_walk();
        }
        if let Some(held) = self.pending_stack.as_mut() {
            held.sp = Some(sp);
        }
        if self
            .send_own(
                "readMemory",
                json!({
                    "memoryReference": address_reference(sp as u32),
                    "count": count
                }),
                Purpose::StackRead
            )
            .is_err()
        {
            return self.abandon_stack_walk();
        }
        Vec::new()
    }

    /// The stack came back; walk it and answer the editor.
    fn stack_step_finish(&mut self, response: &Value) -> Vec<Value> {
        let Some(held) = self.pending_stack.take()
        else {
            return Vec::new();
        };
        let bytes = response
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default();

        let pages = if self.map.pages().is_empty() {
            vec![0]
        }
        else {
            self.map.pages()
        };
        let frames = crate::callstack::walk_paged(&bytes, &pages, |page, address| {
            self.image_byte(page, address)
        });

        // Frame 0 is the one the editor highlights and jumps to, and it is the
        // one the walk cannot answer for: no `CALL` points at it. In a banked
        // program, ask what the bytes there actually are and match them against
        // each page's image - the emulator will not say which page is selected,
        // but it will say what is in it.
        let pc = held
            .response
            .get("body")
            .and_then(|b| b.get("stackFrames"))
            .and_then(Value::as_array)
            .and_then(|list| list.first())
            .and_then(|frame| frame.get("instructionPointerReference"))
            .and_then(Value::as_str)
            .and_then(parse_address_reference)
            .and_then(|address| u16::try_from(address).ok());

        // Kept for `-dv` with no argument, and for a view that follows `PC`.
        if let Some(pc) = pc {
            self.note_program_counter(pc);
        }

        // Only this address matters, not whether the program pages *anywhere*:
        // a stop at an address one page claims resolves on its own, and one
        // several pages claim is worth the read whatever the rest of the
        // program does.
        if let Some(pc) = pc
            && !self.pages_to_tell_apart(pc as u32).is_empty()
        {
            self.pending_page_probe = Some((held.response, frames, pc));
            if self.ask_which_page(pc).is_ok() {
                return Vec::new();
            }
            // Could not ask; fall through and report the ambiguity as before.
            let (response, frames, _) = self.pending_page_probe.take().unwrap();
            return self.finish_stack_walk(response, frames);
        }

        self.pc_page = None;
        self.finish_stack_walk(held.response, frames)
    }

    /// The bytes at `PC` came back; work out which page they came from.
    fn stack_step_page_probe(&mut self, response: &Value) -> Vec<Value> {
        let Some((held, frames, pc)) = self.pending_page_probe.take()
        else {
            return Vec::new();
        };
        let bytes = response
            .get("body")
            .and_then(|b| b.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default();

        // Either the emulator named the page, or its bytes have to be
        // compared against each candidate's image.
        // A memmap answer is recognised by carrying the Gate Array's own
        // banking fields - the MMR decides this, and `regions` is only the
        // emulator's summary of it.
        self.pc_page = match response.get("body").filter(|body| {
            body.get("ram_mode").is_some()
                || body.get("rmr").is_some()
                || body.get("regions").is_some()
        }) {
            // The page, and only the page: a source map row is keyed on the
            // *logical* address the code was assembled to run at, which is the
            // address the CPU is at. Translating `PC` into an offset within
            // the page before looking it up would miss by exactly the bank
            // remapping under MMR modes 4-7.
            Some(memmap) => crate::amspiritlite::page_at(memmap, pc),
            None => self.page_matching(pc, &bytes, &self.pages_to_tell_apart(pc as u32))
        };
        self.finish_stack_walk(held, frames)
    }

    /// Give up on the extra frames and send what the emulator said.
    ///
    /// A one-frame stack is what this adapter did before any of this; failing
    /// back to it costs the caller nothing they had.
    fn abandon_stack_walk(&mut self) -> Vec<Value> {
        match self.pending_stack.take() {
            Some(held) => self.finish_stack_walk(held.response, Vec::new()),
            None => Vec::new()
        }
    }

    /// Splice the reconstructed callers behind frame 0 and annotate the lot.
    fn finish_stack_walk(
        &mut self,
        response: Value,
        frames: Vec<crate::callstack::CallFrame>
    ) -> Vec<Value> {
        let mut response = response;
        if let Some(list) = response
            .get_mut("body")
            .and_then(|b| b.get_mut("stackFrames"))
            .and_then(Value::as_array_mut)
        {
            // Frame ids of our own, well away from the emulator's (which are
            // `stopEpoch * 16 + 1`): a `scopes` request naming one of ours must
            // fail rather than quietly answer for the wrong frame.
            for (index, frame) in frames.iter().enumerate() {
                // Naming the routine the `CALL` entered, not the address it
                // returns to: "play_music" is what the user is looking at in
                // the stack, "0x4003" is where they already are.
                // The page came out of the walk, so a paged frame resolves to
                // exactly one line rather than being reported as ambiguous.
                let located = frame
                    .page
                    .and_then(|page| self.map.location_at_long(page, frame.call_site));
                // The address is always shown beside the name: labels share
                // addresses, so the name is sometimes a choice between several
                // and the number is what makes that visible rather than
                // silently authoritative.
                let name = match self.name_of_call_target(frame.called, frame.call_site, &located)
                {
                    Some(symbol) => format!("{symbol} @ 0x{:04X}", frame.called),
                    None => format!("0x{:04X}", frame.called)
                };
                let locals = if frame.locals.is_empty() {
                    String::new()
                }
                else {
                    format!(" [{} pushed]", frame.locals.len())
                };
                let mut entry = json!({
                    "id": SYNTHETIC_FRAME_BASE + index as i64,
                    "name": format!("{name}{locals}"),
                    "line": 0,
                    "column": 0,
                    "instructionPointerReference": address_reference(frame.call_site as u32),
                    "presentationHint": "normal"
                });
                if let Some(location) = located {
                    entry["line"] = json!(location.line);
                    entry["column"] = json!(location.column.max(1));
                    if location.column_end > location.column {
                        entry["endLine"] = json!(location.line);
                        entry["endColumn"] = json!(location.column_end);
                    }
                    entry["source"] = json!({
                        "name": location
                            .file
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        "path": location.file.to_string_lossy()
                    });
                }
                list.push(entry);
            }
            let total = list.len() as i64;
            if let Some(body) = response.get_mut("body") {
                body["totalFrames"] = json!(total);
            }
        }
        self.synthetic_frames = frames;
        self.annotate_stack_trace(&response)
    }

    /// Change a register, when the emulator can.
    ///
    /// Only the program counter, and only where the emulator offers a way to
    /// move it: AMSpiriT Lite has `POST /api/exec`, and 1984js exposes no
    /// register write at all. Everything else is refused *by name*, because an
    /// edit that is silently ignored is worse than one that is declined - the
    /// pane would show the value you typed while the machine kept the old one.
    fn set_variable(&mut self, request: &Value) -> std::io::Result<Vec<Value>> {
        let arguments = request.get("arguments");
        let name = arguments
            .and_then(|a| a.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let wanted = arguments
            .and_then(|a| a.get("value"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();

        let seq = self.next_seq();
        if !name.eq_ignore_ascii_case("pc") {
            return Ok(vec![protocol::failure(
                request,
                &format!(
                    "this emulator cannot set {name}; only PC can be moved, and only                      where the emulator offers it"
                ),
                seq
            )]);
        }
        if !self.peer.supports("cpclib/setPc") {
            return Ok(vec![protocol::failure(
                request,
                "this emulator offers no way to move PC",
                seq
            )]);
        }

        let Some(address) = parse_number(&wanted)
            .or_else(|| self.map.address_of_symbol(&wanted))
            .and_then(|address| u16::try_from(address).ok())
        else {
            return Ok(vec![protocol::failure(
                request,
                &format!("{wanted} is neither an address nor a known label"),
                seq
            )]);
        };

        self.send_own("cpclib/setPc", json!({ "address": address }), Purpose::Plain)?;
        self.note_program_counter(address);
        let seq = self.next_seq();
        Ok(vec![protocol::response(
            request,
            json!({ "value": format!("0x{address:04X}") }),
            seq
        )])
    }

    /// Which label the `call` at this site actually named.
    ///
    /// Several labels routinely share an address, and no rule about their
    /// spelling reliably separates them - a real case named a frame
    /// `PLY_AKG_DisarkWordRegionEnd_50` where the source plainly reads
    /// `call spectral_sprite_move_along_curve`, because that name is *shorter*
    /// and does not end in `_end`. The call site itself is the evidence: read
    /// the line the call was made from, and prefer the candidate it names.
    ///
    /// Cached on the call site, because reading a source line is not free and
    /// a stack is walked on every single stop, over the same few call sites.
    fn name_of_call_target(
        &mut self,
        called: u16,
        call_site: u16,
        located: &Option<cpclib_project::srcmap::SourceLocation>
    ) -> Option<String> {
        if let Some(known) = self.call_target_names.get(&call_site) {
            return known.clone().into();
        }

        // Owned, so the source line can be read while they are held.
        let candidates: Vec<String> = self
            .map
            .symbols_at(called as u32)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let best = match candidates.len() {
            0 => None,
            // Nothing to disambiguate; do not pay to read a line.
            1 => Some(candidates[0].clone()),
            _ => {
                let named = located
                    .as_ref()
                    .and_then(|at| self.source_line(&at.file, at.line))
                    .and_then(|text| {
                        // The one this line actually mentions. Whole words
                        // only: `draw` must not match `draw_sprite`.
                        candidates
                            .iter()
                            .find(|name| mentions_word(&text, name))
                            .cloned()
                    });
                named.or_else(|| candidates.first().cloned())
            },
        };

        if let Some(best) = best.as_ref() {
            self.call_target_names.insert(call_site, best.clone());
        }
        best
    }

    /// Say where the program stopped, in a message of our own.
    ///
    /// The stack trace already carries the file and line, and the editor is
    /// meant to open them - but whether it *reveals* that file depends on what
    /// happens to hold the editor area at the time (a webview with the
    /// emulator in it, a panel, a view restored from last session), and the
    /// answer had become "sometimes". The extension listens for this and opens
    /// the line itself, which does not depend on any of that.
    fn announce_where_we_stopped(&mut self, answer: &Value) -> Vec<Value> {
        self.pending_stop_hint = None;
        let Some(top) = answer
            .get("body")
            .and_then(|body| body.get("stackFrames"))
            .and_then(Value::as_array)
            .and_then(|frames| frames.first())
        else {
            return Vec::new();
        };
        let Some(source) = top
            .get("source")
            .and_then(|source| source.get("path"))
            .and_then(Value::as_str)
        else {
            return Vec::new();
        };
        let Some(line) = top.get("line").and_then(Value::as_i64).filter(|line| *line > 0)
        else {
            return Vec::new();
        };

        // The source says `ld a,ANIMATION_STATE_FINISHED`; the machine holds
        // `ld a,0x01`. Carrying the resolved form lets the editor show it
        // beside the line the program is stopped on, so what is running and
        // what is written can be told apart.
        let address = top
            .get("instructionPointerReference")
            .and_then(Value::as_str)
            .and_then(parse_address_reference)
            .and_then(|address| u16::try_from(address).ok())
            .or(self.last_pc);
        let written = u32::try_from(line)
            .ok()
            .and_then(|line| self.source_line(Path::new(source), line));

        // The emulator's memory is the only honest source for this. An
        // instruction can have been modified in place; a written one can be
        // several real ones (`ld ix,de` is three); and a routine generated at
        // run time occupies a line that reads `defs`, whose assembled bytes
        // say nothing whatever about what is executing there. Asking costs one
        // read - ~0.2ms on AMSpiriT Lite - and it is *not* waited for: the
        // reveal goes out now and the hint follows as its own message.
        // The columns the stop selects are the columns the hint belongs after,
        // so they travel with it rather than being worked out twice.
        let column = top.get("column").and_then(Value::as_i64).unwrap_or(1);
        let end_column = top.get("endColumn").and_then(Value::as_i64).unwrap_or(column);
        let asked = address
            .is_some_and(|address| self.ask_what_is_at(address, source, line, column, end_column));
        let instruction = match asked {
            true => None,
            // Nothing to ask, or nobody to ask: the assembled image is what is
            // left, and a hint from it beats no hint at all.
            false => {
                self.image_hint(address, written.as_deref())
            }
        };

        let seq = self.next_seq();
        let mut out = vec![protocol::event(
            "cpclib/stoppedAt",
            json!({
                "path": source,
                "line": line,
                "column": top.get("column").cloned().unwrap_or(json!(1)),
                "endColumn": top.get("endColumn").cloned().unwrap_or(Value::Null),
                "instruction": instruction
            }),
            seq
        )];
        out.extend(self.directive_behind_the_stop(address, source, line));
        out
    }

    /// Ask the emulator what it holds at `address`, for this stop's hint.
    ///
    /// Whether the question went out: `false` means the caller has to make do
    /// with the assembled image.
    fn ask_what_is_at(
        &mut self,
        address: u16,
        source: &str,
        line: i64,
        column: i64,
        end_column: i64
    ) -> bool {
        if !self.peer.supports("readMemory") {
            return false;
        }
        let written = u32::try_from(line)
            .ok()
            .and_then(|line| self.source_line(Path::new(source), line));
        self.pending_stop_hint = Some(PendingStopHint {
            path: source.to_string(),
            line,
            column,
            end_column,
            written,
            address,
            request_seq: self.own_seq
        });
        let sent = self.send_own(
            "readMemory",
            json!({
                "memoryReference": address_reference(address as u32),
                // The Z80's longest instruction; anything past it belongs to
                // the next one.
                "count": 4
            }),
            Purpose::StopHint
        );
        if sent.is_err() {
            self.pending_stop_hint = None;
            return false;
        }
        true
    }

    /// The hint the assembled program's own bytes give, when the emulator
    /// cannot be asked.
    fn image_hint(&self, address: Option<u16>, written: Option<&str>) -> Option<String> {
        let decoded = address.and_then(|address| self.instruction_in_image(address))?;
        Self::hint_worth_showing(decoded.text, written)
    }

    /// A hint that repeats the line it sits on is noise, so it is only sent
    /// when it says something the source does not.
    fn hint_worth_showing(decoded: String, written: Option<&str>) -> Option<String> {
        written
            .is_none_or(|written| !line_already_says(written, &decoded))
            .then_some(decoded)
    }

    /// The bytes at `PC` came back; say what they decode to.
    ///
    /// A message of its own rather than a second `cpclib/stoppedAt`: the reveal
    /// has already happened and the user may have moved the cursor since, so
    /// re-announcing the stop would drag them back for a decoration.
    fn complete_stop_hint(&mut self, response: &Value) -> Vec<Value> {
        let Some(pending) = self
            .pending_stop_hint
            .take_if(|pending| {
                response.get("request_seq").and_then(Value::as_i64) == Some(pending.request_seq)
            })
        else {
            return Vec::new();
        };
        let bytes = response
            .get("body")
            .and_then(|body| body.get("data"))
            .and_then(Value::as_str)
            .map(decode_base64)
            .unwrap_or_default();

        let decoded = crate::disassemble::decode(pending.address, &bytes, 1)
            .into_iter()
            .next()
            .map(|instruction| instruction.text);
        let instruction = match decoded {
            Some(decoded) => Self::hint_worth_showing(decoded, pending.written.as_deref()),
            // The read failed or came back empty; the image is the fallback,
            // and it is still better than nothing.
            None => self.image_hint(Some(pending.address), pending.written.as_deref())
        };

        let seq = self.next_seq();
        vec![protocol::event(
            "cpclib/stoppedInstruction",
            json!({
                "path": pending.path,
                "line": pending.line,
                "column": pending.column,
                "endColumn": pending.end_column,
                "instruction": instruction
            }),
            seq
        )]
    }

    /// A link to the `BREAKPOINT` directive that stopped the program, when it
    /// is not written where the program stopped.
    ///
    /// That is the macro case and only the macro case: a directive arms the
    /// address of the instruction *after* it, so one written inside a macro
    /// body stops the program on a line of whichever file used the macro -
    /// with no red dot in the gutter and nothing on the line to explain it.
    /// A directive on the line the program stopped on needs no link; it is
    /// already on screen.
    fn directive_behind_the_stop(
        &mut self,
        address: Option<u16>,
        source: &str,
        line: i64
    ) -> Vec<Value> {
        // Only a stop the emulator attributes to a breakpoint. Stepping onto
        // an armed address is the user walking there themselves, and saying so
        // on every step through a macro-heavy demo would be noise.
        let stopped_at_one = self
            .last_stop_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("breakpoint"));
        let Some(address) = address.filter(|_| stopped_at_one)
        else {
            return Vec::new();
        };

        let written_at = self
            .program_breakpoints
            .iter()
            .find(|breakpoint| {
                breakpoint.address == address as u32
                    && breakpoint.watch.is_none()
                    && !self.suppressed.contains(&breakpoint.address)
            })
            .and_then(|breakpoint| breakpoint.written_at.clone());
        let Some(written_at) = written_at
        else {
            return Vec::new();
        };

        let file = self.known_path(&written_at.file);
        if same_file(&file, Path::new(source)) && written_at.line as i64 == line {
            return Vec::new();
        }

        // `file:line:column` is what VS Code turns into a link in the debug
        // console, so the jump costs no protocol of its own.
        let seq = self.next_seq();
        vec![protocol::event(
            "output",
            json!({
                "category": "console",
                "output": format!(
                    "stopped on a BREAKPOINT directive written at {}:{}:{} - the directive \
                     breaks on the instruction that follows it, which is why the stop is on \
                     another line.\n",
                    file.display(),
                    written_at.line,
                    written_at.column
                )
            }),
            seq
        )]
    }

    /// The path the source map uses for `file`, when it knows it.
    ///
    /// A macro body's parser context names its file however the `include` that
    /// pulled it in did - `macros.asm`, on the project this was built for -
    /// and a relative path is not a link: the editor has no directory to
    /// resolve it against. The source map holds the same file as the absolute
    /// path the assembler canonicalised, so the tail is matched against it.
    fn known_path(&self, file: &str) -> PathBuf {
        let candidate = Path::new(file);
        if candidate.is_absolute() {
            return candidate.to_path_buf();
        }
        self.map
            .files()
            .iter()
            // Whole components, so `writter.asm` does not match
            // `sprite_writter.asm`. More of the relative path than just the
            // name is used when the directive gave more.
            .find(|known| known.ends_with(candidate))
            .cloned()
            .unwrap_or_else(|| candidate.to_path_buf())
    }

    /// Put the file and line back into a stack trace the emulator answered with
    /// addresses only.
    fn annotate_stack_trace(&mut self, response: &Value) -> Vec<Value> {
        let mut annotated = response.clone();
        let mut notes = Vec::new();
        let Some(frames) = annotated
            .get_mut("body")
            .and_then(|b| b.get_mut("stackFrames"))
            .and_then(Value::as_array_mut)
        else {
            return vec![annotated];
        };

        let mut ambiguities = Vec::new();
        for frame in frames.iter_mut() {
            let address = frame
                .get("instructionPointerReference")
                .and_then(Value::as_str)
                .and_then(parse_address_reference);
            let Some(address) = address
            else {
                continue;
            };
            let location = match self.map.resolution_at(address) {
                AddressResolution::Line(location) => location,
                // An address in no source line is a real answer: the editor
                // then offers disassembly instead of highlighting an unrelated
                // line.
                AddressResolution::Unknown => continue,
                // So is an address two pages both claim - and that one is
                // worth explaining, because "why is there no source here"
                // otherwise has no visible answer.
                AddressResolution::Ambiguous { candidates, .. } => {
                    // The page was worked out from the bytes really at `PC`;
                    // if it was, the address is not ambiguous after all.
                    match self.pc_page.and_then(|page| {
                        u16::try_from(address)
                            .ok()
                            .and_then(|address| self.map.location_at_long(page, address))
                    }) {
                        Some(resolved) => resolved,
                        None => {
                            // The most probable line, rather than none at all.
                            //
                            // `-dv` shows this address's source happily - it
                            // asks `location_at`, which answers with the last
                            // span covering the address. Only the stack frame
                            // was stricter, and refusing here is what leaves
                            // the editor with no source to open and a bare
                            // disassembly view instead. The knowledge is the
                            // same; the caution was costing more than it saved.
                            //
                            // Still reported once, because a guess presented as
                            // certainty is the thing actually worth avoiding.
                            // The lowest page that claims it - a guess, and
                            // demonstrably the wrong one: `0x79F3` is claimed
                            // by page 0's `writter.asm` and page 1's
                            // `animate.asm`, and the code really running there
                            // was page 1's. Only kept for an emulator that
                            // cannot report its paging at all; one that can is
                            // asked instead, above.
                            let probable = candidates.first().map(|(_, l)| l.clone());
                            ambiguities.push((address, candidates));
                            match probable {
                                Some(probable) => probable,
                                None => continue
                            }
                        }
                    }
                }
            };
            frame["line"] = json!(location.line);
            // The instruction, not the line. On `ld a,l : inc a : ld (.p),a`
            // the editor then highlights the one being executed instead of
            // putting the cursor at the start of all three.
            frame["column"] = json!(location.column.max(1));
            if location.column_end > location.column {
                frame["endLine"] = json!(location.line);
                frame["endColumn"] = json!(location.column_end);
            }
            frame["source"] = json!({
                "name": location
                    .file
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "path": location.file.to_string_lossy()
            });
        }

        if !ambiguities.is_empty() && !self.banking_explained {
            self.banking_explained = true;
            let compared = self.image.is_some();
            for (address, candidates) in ambiguities {
                let choices = candidates
                    .iter()
                    .map(|(page, location)| {
                        format!(
                            "page {page}: {}:{}",
                            location
                                .file
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            location.line
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let why = if compared {
                    "the bytes really in memory there were compared against each page's \
                     assembled image and matched them equally well, so picking one would \
                     be a guess"
                }
                else {
                    "the program's assembled image is not available to this session, so \
                     there is nothing to compare the bytes in memory against - check that \
                     the launch configuration names a program or a rule whose entry point \
                     resolves"
                };
                notes.push(format!(
                    "0x{address:04X} holds code from more than one page ({choices}); {why}. \
                     The most likely line is shown; `-dv 0x{address:04X}` shows what is \
                     actually running there."
                ));
            }
        }

        let mut out: Vec<Value> = notes
            .into_iter()
            .map(|note| {
                let seq = self.next_seq();
                protocol::event(
                    "output",
                    json!({"category": "console", "output": format!("{note}\n")}),
                    seq
                )
            })
            .collect();
        // Before the response, not after: every caller reads the stack trace as
        // the last message, and an event of ours must not displace it.
        let announcement = self.announce_where_we_stopped(&annotated);
        out.extend(announcement);
        out.push(annotated);
        out
    }

    /// Which lines currently hold a breakpoint in `file` - used to report a
    /// breakpoint the editor should move.
    pub fn breakpoint_lines(&self, file: &Path) -> Vec<u32> {
        self.breakpoints
            .get(file)
            .map(|bps| bps.iter().map(|bp| bp.line).collect())
            .unwrap_or_default()
    }

    /// Whether any breakpoint had to move from the line the editor asked for.
    pub fn moved_breakpoints(&self) -> Vec<(u32, u32)> {
        self.breakpoints
            .values()
            .flatten()
            .filter(|bp| bp.address.is_some() && bp.line != bp.requested_line)
            .map(|bp| (bp.requested_line, bp.line))
            .collect()
    }
}

/// Whether the line as written already spells out the instruction decoded from
/// memory.
///
/// The hint exists to resolve a symbol into the value the machine holds, so a
/// line that already reads the same has nothing to disambiguate. Compared
/// loosely, because the two spellings come from different places: case and
/// spacing differ, and a label or a comment routinely shares the line with the
/// instruction - hence "contains" rather than "equals".
fn line_already_says(written: &str, decoded: &str) -> bool {
    let decoded = squash_instruction(decoded);
    !decoded.is_empty() && squash_instruction(written).contains(&decoded)
}

/// An instruction reduced to what it means: no comment, no spacing, no case,
/// and every literal written in one base.
///
/// The base matters as much as the spacing. Source says `ld a,1`, the
/// disassembler says `LD A, 0x1`, and the machine holds the same byte - a hint
/// repeating that would be noise on most lines of a demo.
fn squash_instruction(text: &str) -> String {
    let text: Vec<char> = text
        .split(';')
        .next()
        .unwrap_or_default()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let mut out = String::new();
    let mut at = 0;
    while at < text.len() {
        // Only where a name cannot be starting: `label1` is one word, and the
        // `1` in it is not a literal.
        let opens_a_word = at == 0 || !matches!(text[at - 1], 'a'..='z' | '0'..='9' | '_' | '.');
        match literal_at(&text[at..]).filter(|_| opens_a_word) {
            Some((value, length)) => {
                out.push_str(&value.to_string());
                at += length;
            },
            None => {
                out.push(text[at]);
                at += 1;
            }
        }
    }
    out
}

/// A number written the way anyone here writes one - `0x1F`, `&1f`, `#1f`,
/// `%0001`, `31` - as its value, with the length it occupied.
fn literal_at(text: &[char]) -> Option<(u32, usize)> {
    let (radix, prefix) = match text {
        ['0', 'x', ..] => (16, 2),
        ['&' | '#', ..] => (16, 1),
        ['%', ..] => (2, 1),
        [digit, ..] if digit.is_ascii_digit() => (10, 0),
        _ => return None
    };
    let digits: String = text[prefix..]
        .iter()
        .take_while(|c| c.is_digit(radix))
        .collect();
    if digits.is_empty() {
        return None;
    }
    let length = prefix + digits.len();
    // `12ab` is a name, not a decimal followed by letters.
    if text
        .get(length)
        .is_some_and(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
    {
        return None;
    }
    u32::from_str_radix(&digits, radix)
        .ok()
        .map(|value| (value, length))
}

/// Base64, as DAP encodes memory contents.
/// Levenshtein distance, giving up past `limit`.
///
/// Bounded on purpose: it is asked against every symbol in the program, and
/// what it is asked is "is this a typo of that", which nothing past a couple of
/// edits answers yes to.
fn edit_distance(left: &str, right: &str, limit: usize) -> Option<usize> {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.len().abs_diff(right.len()) > limit {
        return None;
    }

    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];
    for (i, l) in left.iter().enumerate() {
        current[0] = i + 1;
        let mut best = current[0];
        for (j, r) in right.iter().enumerate() {
            current[j + 1] = (previous[j] + usize::from(l != r))
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
            best = best.min(current[j + 1]);
        }
        // Every path through the rest of the table only grows, so a row whose
        // cheapest cell already exceeds the limit cannot come back.
        if best > limit {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }
    let distance = previous[right.len()];
    (distance <= limit).then_some(distance)
}

/// A number written the way anyone here writes one: `0xC000`, `&C000`, `#C000`
/// or plain decimal.
/// A cheap content fingerprint: length and an FNV-1a hash of the bytes.
///
/// Not a modification time - a rebuild that rewrites a file byte for byte would
/// look like an edit - and not a cryptographic hash, because the question is
/// "did this change since a minute ago", not "did someone forge it".
fn fingerprint_of(file: &Path) -> Option<u64> {
    let bytes = std::fs::read(file).ok()?;
    let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
    for byte in &bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01B3);
    }
    Some(hash ^ bytes.len() as u64)
}

/// Whether two paths name the same file.
///
/// The assembler records the path it was given and the editor sends whatever
/// the user opened, so a plain `==` is not enough - but canonicalising is a
/// syscall, so the cheap comparison is tried first.
fn same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false
    }
}

fn parse_number(text: &str) -> Option<u32> {
    let text = text.trim();
    for prefix in ["0x", "0X", "&", "#", "$"] {
        if let Some(digits) = text.strip_prefix(prefix) {
            return u32::from_str_radix(digits, 16).ok();
        }
    }
    text.parse::<u32>().ok()
}

pub(crate) fn decode_base64(encoded: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for byte in encoded.bytes() {
        if byte == b'=' {
            break;
        }
        let Some(index) = ALPHABET.iter().position(|c| *c == byte)
        else {
            continue; // whitespace and anything else are not data
        };
        accumulator = (accumulator << 6) | index as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }
    out
}

/// Whether a source line is a `defs` directive - the only line a step over
/// treats as a repetition.
///
/// basm accepts four spellings of the same thing (`DEFS`, `DS`, `RMEM`,
/// `FILL`), any of which may be preceded by a label and followed by an
/// expression. The comment is dropped first, so a line that merely mentions
/// one in prose is not mistaken for one, and whole words only, so
/// `defs_table` and `.ds` are labels rather than directives.
fn is_a_defs_directive(text: &str) -> bool {
    let code = text.split(';').next().unwrap_or_default();
    let code = code.split("//").next().unwrap_or_default().to_lowercase();
    ["defs", "ds", "rmem", "fill"]
        .iter()
        .any(|spelling| mentions_word(&code, spelling))
}

/// Whether `text` mentions `word` as a whole identifier.
///
/// `call draw` must not be taken as naming `draw_sprite`, and
/// `call spectral_sprite_move_along_curve` must not be taken as naming
/// `sprite` - so the characters either side have to be non-identifier ones.
fn mentions_word(text: &str, word: &str) -> bool {
    let is_part = |c: char| c.is_alphanumeric() || c == '_' || c == '.';
    let mut from = 0;
    while let Some(at) = text[from..].find(word) {
        let start = from + at;
        let end = start + word.len();
        let before_ok = start == 0 || !text[..start].chars().next_back().is_some_and(is_part);
        let after_ok = end == text.len() || !text[end..].chars().next().is_some_and(is_part);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_a_defs_directive;

    /// The one thing that decides whether a run of `NOP`s is stepped over
    /// whole, so its edges are worth pinning.
    #[test]
    fn a_defs_line_is_recognised_and_nothing_else_is() {
        for line in [
            "\tdefs 64 - duration(djnz $)-1",
            "\tDEFS 60",
            "wait\tds 40",
            "\trmem 10",
            "\tfill 8, 0",
        ] {
            assert!(is_a_defs_directive(line), "{line}");
        }

        for line in [
            "\tnop",
            // A label is not a directive, whichever way it is spelled.
            "defs_table\tdb 0",
            ".ds\tnop",
            // Nor is prose about one.
            "\tnop ; padded like a defs run",
            "\tnop // ds here would be neater"
        ] {
            assert!(!is_a_defs_directive(line), "{line}");
        }
    }
}

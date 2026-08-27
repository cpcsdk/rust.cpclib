use std::collections::BTreeMap;

use codespan_reporting::diagnostic::Severity;
use cpclib_common::itertools::Itertools;
use cpclib_sna::{
    AceBreakPoint, AceBrkRuntimeMode, AdvancedRemuBreakPoint, RemuBreakPoint,
    RemuBreakPointAccessMode, RemuBreakPointType, WabpAnyBreakpoint, WinapeBreakPoint
};

use std::sync::Arc;

use super::report::SavedFile;
use super::save_command::SaveCommand;
use super::string::PreprocessedFormattedString;
use super::{Env, EnvEventObserver};
use crate::error::{AssemblerError, build_simple_error_message};
use crate::preamble::{LocatedListing, Z80Span};

#[allow(unused)]
trait DelayedCommand {}

#[derive(Debug, Clone)]
pub struct PrintCommand {
    pub(crate) prefix: Option<String>,
    pub(crate) span: Option<Z80Span>,
    pub(crate) print_or_error: either::Either<PreprocessedFormattedString, Box<AssemblerError>>
}

impl PrintCommand {
    pub fn relocate(&mut self, span: Z80Span) {
        self.span.replace(span);
    }
}
#[derive(Debug, Clone)]
pub struct FailedAssertCommand {
    pub(crate) failure: Box<AssemblerError>,
    /// Keeps alive whichever macro/struct-expansion buffer(s) `failure`'s
    /// span (if any) points into - see `Env::active_expansion_listings`.
    /// Without this, the buffer can be dropped (a later pass re-expanding
    /// the same macro call, or the whole token tree going out of scope once
    /// assembling finishes) before this command is finally formatted, e.g.
    /// in `PageInformation::collect_assert_failure`, and formatting the
    /// resulting error then dereferences a dangling `Z80Span`.
    pub(crate) _keep_alive: Vec<Arc<LocatedListing>>
}

/// Expect an assert error or a exval error. Carries no keep-alive: only
/// safe for a `failure` whose span (if any) doesn't point into a transient
/// macro-expansion buffer - e.g. one already flattened via `.render()`, or
/// one located against the top-level source file.
impl From<AssemblerError> for FailedAssertCommand {
    fn from(failure: AssemblerError) -> Self {
        Self {
            failure: Box::new(failure),
            _keep_alive: Vec::new()
        }
    }
}

impl From<Box<AssemblerError>> for FailedAssertCommand {
    fn from(failure: Box<AssemblerError>) -> Self {
        Self {
            failure,
            _keep_alive: Vec::new()
        }
    }
}

impl DelayedCommand for PrintCommand {}

impl DelayedCommand for FailedAssertCommand {}

impl PrintCommand {
    #[inline]
    pub fn string_or_error(&self) -> Result<String, Box<AssemblerError>> {
        match &self.print_or_error {
            either::Either::Left(msg) => {
                // TODO improve printting + integrate z80span information
                let file_location = if let Some(span) = &self.span {
                    let fname = span.filename();
                    let (line, col) = span.relative_line_and_column();

                    Some((fname, line, col))
                }
                else {
                    None
                };

                // duplicate code to speed it up
                let repr = match (&self.prefix, file_location) {
                    (Some(prefix), Some(loc)) => {
                        format!("{}{}:{}:{} PRINT: {}", prefix, loc.0, loc.1, loc.2, msg)
                    },

                    (Some(prefix), None) => {
                        format!("{prefix} PRINT: {msg}")
                    },

                    (None, Some(loc)) => {
                        format!("{}:{}:{} PRINT: {}", loc.0, loc.1, loc.2, msg)
                    },

                    (None, None) => {
                        format!("PRINT: {msg}")
                    }
                };

                Ok(repr)
            },
            either::Either::Right(e) => Err(e.clone())
        }
    }

    // XXX The code is the same than string_or_error
    #[inline]
    pub fn execute(&self, writer: &dyn EnvEventObserver) -> Result<(), Box<AssemblerError>> {
        match &self.print_or_error {
            either::Either::Left(msg) => {
                // TODO improve printting + integrate z80span information
                let file_location = if let Some(span) = &self.span {
                    let fname = span.filename();
                    let (line, col) = span.relative_line_and_column();

                    Some((fname, line, col))
                }
                else {
                    None
                };

                // duplicate code to speed it up
                match (&self.prefix, file_location) {
                    (Some(prefix), Some(loc)) => {
                        writer.emit_stdout(&format!(
                            "{}{}:{}:{} PRINT: {}\n",
                            prefix, loc.0, loc.1, loc.2, msg
                        ))
                    },

                    (Some(prefix), None) => writer.emit_stdout(&format!("{prefix} PRINT: {msg}\n")),

                    (None, Some(loc)) => {
                        writer.emit_stdout(&format!("{}:{}:{} PRINT: {}", loc.0, loc.1, loc.2, msg))
                    },

                    (None, None) => writer.emit_stdout(&format!("PRINT: {msg}"))
                };

                Ok(())
            },
            either::Either::Right(e) => Err(e.clone())
        }
    }

    #[inline]
    pub fn is_print(&self) -> bool {
        self.print_or_error.is_left()
    }
}
#[derive(Debug, Clone)]

pub struct PauseCommand(Option<Z80Span>);

impl From<Option<Z80Span>> for PauseCommand {
    fn from(s: Option<Z80Span>) -> Self {
        Self(s)
    }
}

impl PauseCommand {
    /// `dry_run` skips the message *and* the blocking read of the real
    /// process stdin entirely — a `PAUSE` in a dry-run assembling pass must
    /// never block waiting for input nobody can provide (e.g. an LSP
    /// server, whose real stdin carries protocol traffic, not a human at a
    /// terminal).
    #[inline]
    pub fn execute(
        &self,
        writer: &dyn EnvEventObserver,
        dry_run: bool
    ) -> Result<(), Box<AssemblerError>> {
        if dry_run {
            return Ok(());
        }

        let msg = "PAUSE - press enter to continue.";
        writer.emit_stdout(
            &(if let Some(span) = &self.0 {
                build_simple_error_message(msg, span, Severity::Note)
            }
            else {
                msg.to_owned()
            })
            .to_string()
        );

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();
        Ok(())
    }

    pub fn relocate(&mut self, span: Z80Span) {
        self.0.replace(span);
    }
}

#[derive(Debug, Clone)]
pub enum PrintOrPauseCommand {
    Print(PrintCommand),
    Pause(PauseCommand)
}

impl From<PrintCommand> for PrintOrPauseCommand {
    fn from(p: PrintCommand) -> Self {
        PrintOrPauseCommand::Print(p)
    }
}

impl From<PauseCommand> for PrintOrPauseCommand {
    fn from(p: PauseCommand) -> Self {
        PrintOrPauseCommand::Pause(p)
    }
}

impl PrintOrPauseCommand {
    pub fn execute(
        &self,
        writer: &dyn EnvEventObserver,
        dry_run: bool
    ) -> Result<(), Box<AssemblerError>> {
        match self {
            PrintOrPauseCommand::Print(p) => p.execute(writer),
            PrintOrPauseCommand::Pause(p) => p.execute(writer, dry_run)
        }
    }

    pub fn relocate(&mut self, span: Z80Span) {
        match self {
            PrintOrPauseCommand::Print(p) => p.relocate(span),
            PrintOrPauseCommand::Pause(p) => p.relocate(span)
        }
    }
}

/// Information for a breakpoint:
/// TODO: add condition
#[derive(Debug, Clone)]
pub struct BreakpointCommand {
    pub(crate) brk: InnerBreakpointCommand,
    pub(crate) info: AssemblerError,
    /// Where the directive itself is written.
    ///
    /// Kept apart from `info`: that one is rendered to a string as soon as it
    /// is built, and rendering is what loses the span. A debugger stopping at
    /// this breakpoint needs the location back - a `BREAKPOINT` inside a macro
    /// body stops the program on the line *after* every expansion, in a file
    /// the user never marked, and only this says where it was actually asked
    /// for.
    pub(crate) written_at: Option<BreakpointSource>
}

/// Where a `BREAKPOINT` directive is written, in the file's own numbering.
///
/// A macro body is re-parsed as a source of its own, so the span's line counts
/// from the body rather than from the file; the conversion happens here, once,
/// rather than being left for every reader to get wrong.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BreakpointSource {
    pub file: String,
    /// 1-based.
    pub line: u32,
    /// 1-based, and `1` whenever the expansion cannot be mapped back onto a
    /// column of the file - pointing at the start of the line is honest, a
    /// column inside the substituted text is not.
    pub column: u32
}

impl BreakpointSource {
    /// The location of the directive, from the span the parser gave it.
    fn of_span(span: &Z80Span) -> Self {
        let context = span.context();
        let (line, column) = span.relative_line_and_column();
        let (line, column) = (line.max(1) as u32, column.max(1) as u32);

        let Some(name) = context
            .context_name()
            .filter(|_| context.filename().is_none() && context.is_expansion())
        else {
            let file = context
                .filename()
                .map(|p| p.to_string())
                .unwrap_or_else(|| span.filename().to_owned());
            return Self { file, line, column };
        };

        // Inside an expansion the columns belong to the substituted text - the
        // map built while substituting is the only thing that can put them
        // back, and a struct expansion has none at all.
        let column = context
            .expansion_columns()
            .and_then(|columns| {
                // Width zero: only the start is wanted, and a `BREAKPOINT`
                // span reaches further than the directive itself.
                columns.source_columns(span.offset_from_start(), column as usize, 0)
            })
            .map(|(start, _)| start as u32)
            .unwrap_or(1);
        Self {
            file: crate::assembler::listing_output::source_map::real_file_name(name).to_owned(),
            line: line + crate::assembler::listing_output::source_map::expansion_line_offset(name),
            column
        }
    }
}

#[derive(Debug, Clone)]
pub enum InnerBreakpointCommand {
    Simple(BreakPointCommandSimple),
    Advanced(AdvancedRemuBreakPoint)
}

impl InnerBreakpointCommand {
    fn info_repr(&self) -> String {
        match self {
            InnerBreakpointCommand::Simple(brk) => {
                format! {"PC=&{:X}@{}", brk.address, brk.page}
            },
            InnerBreakpointCommand::Advanced(brk) => {
                format! {"{brk}"}
            }
        }
    }
}
impl From<AdvancedRemuBreakPoint> for InnerBreakpointCommand {
    fn from(value: AdvancedRemuBreakPoint) -> Self {
        Self::Advanced(value)
    }
}

impl From<BreakPointCommandSimple> for InnerBreakpointCommand {
    fn from(value: BreakPointCommandSimple) -> Self {
        Self::Simple(value)
    }
}

#[derive(Debug, Clone)]
pub struct BreakPointCommandSimple {
    pub(crate) address: u16,
    pub(crate) page: u8
}

impl<T: Into<InnerBreakpointCommand>> From<(T, Option<Z80Span>)> for BreakpointCommand {
    fn from(value: (T, Option<Z80Span>)) -> Self {
        let brk = value.0.into();
        let repr = brk.info_repr();

        let span = value.1.unwrap();
        let written_at = Some(BreakpointSource::of_span(&span));
        let info = AssemblerError::RelocatedInfo {
            info: Box::new(AssemblerError::AssemblingError {
                msg: format!("Add a breakpoint: {} ", repr)
            }),
            span
        }
        .render();

        Self {
            brk,
            info,
            written_at
        }
    }
}

/// One breakpoint the assembled program asked for, in a form a debugger can
/// act on.
///
/// The assembler's own representation is shaped for the snapshot chunks it
/// writes; this is the same information without that commitment, so a debug
/// adapter can decide for itself what its emulator is able to honour.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AssembledBreakpoint {
    pub address: u16,
    pub page: u8,
    /// What the program asked to break on. Anything other than execution needs
    /// an emulator that implements watchpoints.
    pub kind: AssembledBreakpointKind,
    /// Set when the directive carried attributes beyond an address - a
    /// condition, a size, a mask. Held as text because its only use is telling
    /// the user what an emulator could not honour.
    pub extra: Option<String>,
    pub name: Option<String>,
    /// Where the directive is written, when the assembler knew.
    ///
    /// Only interesting when it differs from the line the program stops on,
    /// which is exactly the macro case: `BREAKPOINT` in a macro body arms the
    /// address of the next real instruction, so the stop lands wherever the
    /// macro was *used*.
    #[serde(default)]
    pub written_at: Option<BreakpointSource>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssembledBreakpointKind {
    Execution,
    /// A memory watchpoint, with the accesses it watches for.
    Memory {
        read: bool,
        write: bool
    },
    Io
}

impl BreakpointCommand {
    /// This breakpoint, described for a debugger rather than for a chunk.
    pub fn described(&self) -> AssembledBreakpoint {
        match &self.brk {
            InnerBreakpointCommand::Simple(brk) => {
                AssembledBreakpoint {
                    address: brk.address,
                    page: brk.page,
                    kind: AssembledBreakpointKind::Execution,
                    extra: None,
                    name: None,
                    written_at: self.written_at.clone()
                }
            },
            InnerBreakpointCommand::Advanced(brk) => {
                let kind = match brk.brk_type {
                    RemuBreakPointType::Exec => AssembledBreakpointKind::Execution,
                    RemuBreakPointType::IO => AssembledBreakpointKind::Io,
                    RemuBreakPointType::Mem => {
                        AssembledBreakpointKind::Memory {
                            read: matches!(
                                brk.access_mode,
                                RemuBreakPointAccessMode::Read
                                    | RemuBreakPointAccessMode::ReadWrite
                            ),
                            write: matches!(
                                brk.access_mode,
                                RemuBreakPointAccessMode::Write
                                    | RemuBreakPointAccessMode::ReadWrite
                            )
                        }
                    },
                };

                // Only the attributes a plain address breakpoint cannot express
                // - the ones worth telling the user were lost.
                let mut extra = Vec::new();
                if let Some(condition) = &brk.condition {
                    extra.push(format!("condition {}", AsRef::<str>::as_ref(condition)));
                }
                if brk.size > 1 {
                    extra.push(format!("size {}", brk.size));
                }
                if brk.mask != 0xFFFF {
                    extra.push(format!("mask 0x{:04X}", brk.mask));
                }
                if let Some(step) = brk.step {
                    extra.push(format!("step {step}"));
                }

                AssembledBreakpoint {
                    address: brk.addr,
                    // The advanced form carries no page of its own.
                    page: 0,
                    kind,
                    extra: (!extra.is_empty()).then(|| extra.join(", ")),
                    name: brk
                        .name
                        .as_ref()
                        .map(|n| AsRef::<str>::as_ref(n).to_string()),
                    written_at: self.written_at.clone()
                }
            }
        }
    }

    pub fn new_simple(address: u16, page: u8, span: Option<Z80Span>) -> Self {
        (BreakPointCommandSimple { address, page }, span).into()
    }

    // Convert when possible
    pub fn winape(&self) -> Option<WinapeBreakPoint> {
        match &self.brk {
            InnerBreakpointCommand::Simple(brk) => {
                Some(WinapeBreakPoint::new(brk.address, brk.page))
            },
            _ => None
        }
    }

    // Convert when possible. ATTENTION, I have not implemented all the case
    pub fn ace(&self) -> Option<AceBreakPoint<'_>> {
        match &self.brk {
            InnerBreakpointCommand::Simple(brk) => {
                Some(AceBreakPoint::new_execution(
                    brk.address,
                    AceBrkRuntimeMode::Break,
                    cpclib_sna::AceMemMapType::Undefined
                ))
            },
            _ => None
        }
    }

    pub fn remu(&self) -> RemuBreakPoint {
        match &self.brk {
            InnerBreakpointCommand::Simple(brk) => RemuBreakPoint::Memory(brk.address, brk.page),
            InnerBreakpointCommand::Advanced(brk) => RemuBreakPoint::Advanced(brk.clone())
        }
    }

    pub fn wabp(&self) -> WabpAnyBreakpoint {
        match &self.brk {
            InnerBreakpointCommand::Simple(brk) => WabpAnyBreakpoint::new(brk.address),
            InnerBreakpointCommand::Advanced(advanced_remu_break_point) => {
                unimplemented!("{advanced_remu_break_point} not converted in wabp")
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DelayedCommands {
    failed_assert_commands: Vec<FailedAssertCommand>,
    save_commands: BTreeMap<u8, Vec<SaveCommand>>, // commands are ordered per ga_mmr
    print_commands: Vec<PrintOrPauseCommand>,
    breakpoint_commands: Vec<BreakpointCommand>
}

impl DelayedCommands {
    pub fn clear(&mut self) {
        self.failed_assert_commands.clear();
        self.save_commands.clear();
        self.print_commands.clear();
        self.breakpoint_commands.clear();
    }
}

/// Commands addition
impl DelayedCommands {
    pub fn add_breakpoint_command(&mut self, command: BreakpointCommand) {
        self.breakpoint_commands.push(command);
    }

    pub fn add_save_command(&mut self, command: SaveCommand) {
        self.save_commands
            .entry(command.ga_mmr())
            .or_default()
            .push(command);
    }

    pub fn get_save_mmrs(&self) -> Vec<u8> {
        self.save_commands.keys().cloned().collect_vec()
    }

    /// can save in parallel if all commands can be saved in parallel (we are strict because we miss lots of parallelism)
    pub fn can_save_in_parallel(&self) -> bool {
        self.save_commands
            .values()
            .all(|s| s.iter().all(|s| s.can_be_saved_in_parallel()))
    }

    pub fn add_failed_assert_command(&mut self, command: FailedAssertCommand) {
        self.failed_assert_commands.push(command);
    }

    pub fn add_print_command(&mut self, command: PrintCommand) {
        self.add_print_or_pause_command(command.into());
    }

    pub fn add_pause_command(&mut self, command: PauseCommand) {
        self.add_print_or_pause_command(command.into());
    }

    pub fn add_print_or_pause_command(&mut self, command: PrintOrPauseCommand) {
        self.print_commands.push(command)
    }
}

/// Commands execution
impl DelayedCommands {
    /// Execute the commands that correspond to the appropriate mmr configuration
    pub fn execute_save(
        &self,
        env: &Env,
        ga_mmr: u8
    ) -> Result<Vec<SavedFile>, Box<AssemblerError>> {
        let cmds = self
            .save_commands
            .iter()
            .filter_map(|(save_mmr, save_cmds)| (*save_mmr == ga_mmr).then_some(save_cmds))
            .flat_map(|save_cmds| save_cmds.iter())
            .collect_vec();

        // TODO reactivate parallalism for save. BUT ATM I am unable to make it compile
        //#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
        // let cmds = CondIterator::new(&cmds, self.can_save_in_parallel());
        //#[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
        let cmds = cmds.iter();

        use either::Either;
        let (oks, errs): (Vec<_>, Vec<_>) =
            cmds.map(|cmd| cmd.execute_on(env)).partition_map(|res| {
                match res {
                    Ok(val) => Either::Left(val),
                    Err(e) => Either::Right(e)
                }
            });
        if !errs.is_empty() {
            Err(Box::new(AssemblerError::MultipleErrors { errors: errs }))
        }
        else {
            Ok(oks)
        }
    }

    pub fn nb_files_to_save(&self) -> usize {
        self.save_commands.len()
    }

    /// Return Ok if no assertion error, Err otherwise
    pub fn collect_assert_failure(&self) -> Result<(), Box<AssemblerError>> {
        if self.failed_assert_commands.is_empty() {
            Ok(())
        }
        else {
            let errors = self
                .failed_assert_commands
                .iter()
                .map(|a| a.failure.clone())
                .collect_vec();
            Err(Box::new(AssemblerError::MultipleErrors { errors }))
        }
    }

    pub fn execute_print_or_pause(
        &self,
        writer: &dyn EnvEventObserver,
        dry_run: bool
    ) -> Result<(), Box<AssemblerError>> {
        let iter = self.print_commands.iter();

        let errors: Vec<Box<AssemblerError>> = iter
            .filter_map(|c| {
                match c {
                    PrintOrPauseCommand::Print(p) => {
                        if p.is_print() {
                            let _ = p.execute(writer);
                            None
                        }
                        else {
                            Some(p.print_or_error.as_ref().right().unwrap().clone())
                        }
                    },
                    PrintOrPauseCommand::Pause(p) => {
                        let _ = p.execute(writer, dry_run);
                        None
                    }
                }
            })
            .collect();

        if errors.is_empty() {
            Ok(())
        }
        else {
            Err(Box::new(AssemblerError::MultipleErrors { errors }))
        }
    }
}

impl DelayedCommands {
    pub fn print_commands(&self) -> &[PrintOrPauseCommand] {
        &self.print_commands
    }

    pub fn print_commands_mut(&mut self) -> &mut [PrintOrPauseCommand] {
        &mut self.print_commands
    }

    pub fn failed_assert_commands(&self) -> &[FailedAssertCommand] {
        &self.failed_assert_commands
    }

    pub fn failed_assert_commands_mut(&mut self) -> &mut [FailedAssertCommand] {
        &mut self.failed_assert_commands
    }
}

impl DelayedCommands {
    pub fn collect_breakpoints(&self) -> &[BreakpointCommand] {
        &self.breakpoint_commands
    }
}

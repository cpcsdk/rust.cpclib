/// Canonical form of a source path, so the same file reached by different
/// routes (`include "x.asm"` from one directory, an absolute path from
/// another) hashes to the same key in [`Env::address_trace_by_file`].
///
/// Falls back to the path as written when it cannot be canonicalised - a
/// synthetic in-memory source has no real path - which is harmless, since such
/// a source is only ever compared against itself.
fn canonical_source_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(name);
    Some(std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

use crate::disass::disassemble;
use crate::error::AssemblerError;
pub mod control;
pub mod delayed_command;
pub mod embedded;
pub mod file;
pub mod function;
pub mod list;
pub mod listing_output;
pub mod r#macro;
pub mod maths;
pub mod matrix;
pub mod page_info;
pub mod processed_token;
pub mod report;
pub mod save_command;
pub mod section;
pub mod stable_ticker;
pub mod string;
pub mod support;
pub mod symbols_output;

use std::borrow::BorrowMut;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::ops::{Deref, Neg};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use cpclib_basic::*;
use cpclib_common::bitvec::prelude::BitVec;
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::chars::{Charset, char_to_amscii};
use cpclib_common::event::EventObserver;
use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::SmallVec;
use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::stream::UpdateSlice;
use cpclib_disc::built_info;
use cpclib_files::{FileType, StorageSupport};
use cpclib_sna::*;
use cpclib_tokens::ToSimpleToken;
use either::Either;
use file::AnyFileNameOwned;
use processed_token::build_processed_token;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use rayon_cond::CondIterator;
use support::banks::DecoratedPages;
use support::cpr::CprAssembler;
use support::sna::SnaAssembler;

use self::control::ControlOutputStore;
use self::function::{Function, FunctionBuilder, HardCodedFunction};
use self::listing_output::*;
use self::processed_token::ProcessedToken;
use self::report::SavedFile;
use self::string::PreprocessedFormattedString;
use self::symbols_output::{SymbolOutputFormat, SymbolOutputGenerator};
use crate::assembler::processed_token::visit_processed_tokens;
use crate::delayed_command::*;
use crate::page_info::PageInformation;
use crate::preamble::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::progress::Progress;
use crate::report::Report;
use crate::save_command::*;
use crate::section::Section;
use crate::stable_ticker::*;
use crate::{AssemblingOptions, MemoryPhysicalAddress};

#[derive(Clone, Copy, PartialEq, Debug)]
enum OutputKind {
    Snapshot,
    Cpr,
    FreeBank
}

/// Use smallvec to put stuff on the stack not the heap and (hope so) speed up assembling
const MAX_SIZE: usize = 4;
const MMR_PAGES_SELECTION: [u8; 9] = [
    0xC0,
    0b1100_0001,
    0b1100_1001,
    0b1101_0001,
    0b1101_1001,
    0b1110_0001,
    0b1110_1001,
    0b1111_0001,
    0b1111_1001
];

#[allow(missing_docs)]
pub type Bytes = SmallVec<[u8; MAX_SIZE]>;

#[derive(Clone, Debug)]
pub struct EnvOptions {
    parse: ParserOptions,
    assemble: AssemblingOptions,
    observer: Arc<dyn EnvEventObserver>
}

impl Default for EnvOptions {
    fn default() -> Self {
        Self {
            parse: Default::default(),
            assemble: Default::default(),
            observer: Arc::new(())
        }
    }
}

impl From<AssemblingOptions> for EnvOptions {
    fn from(ass: AssemblingOptions) -> EnvOptions {
        let mut opt = Self::default();
        opt.assemble = ass;
        opt
    }
}

impl EnvOptions {
    delegate::delegate! {
        to self.parse {
            pub fn context_builder(self) -> ParserContextBuilder;
        }

        to self.assemble {
            pub fn case_sensitive(&self) -> bool;
            pub fn symbols(&self) -> &cpclib_tokens::symbols::SymbolsTable;
            pub fn symbols_mut(&mut self) -> &mut cpclib_tokens::symbols::SymbolsTable;
            pub fn save_behavior(&self) -> cpclib_disc::amsdos::AmsdosAddBehavior;
            pub fn dry_run(&self) -> bool;
            pub fn set_dry_run(&mut self, dry_run: bool) -> &mut AssemblingOptions;
            pub fn record_token_addresses(&self) -> bool;
            pub fn set_record_token_addresses(&mut self, record: bool) -> &mut AssemblingOptions;

            pub fn write_listing_output<W: 'static + Write + Send + Sync>(
                &mut self,
                writer: W
            ) -> &mut AssemblingOptions;

            pub fn write_listing_output_with_format<W: 'static + Write + Send + Sync>(
                &mut self,
                writer: W,
                format: ListingOutputFormat
            ) -> &mut AssemblingOptions;

        }
    }

    pub fn new(
        parse: ParserOptions,
        assemble: AssemblingOptions,
        observer: Arc<dyn EnvEventObserver>
    ) -> Self {
        Self {
            parse,
            assemble,
            observer
        }
    }

    pub fn parse_options(&self) -> &ParserOptions {
        &self.parse
    }

    pub fn assemble_options(&self) -> &AssemblingOptions {
        &self.assemble
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn show_progress(&self) -> bool {
        self.parse.show_progress
    }

    #[cfg(target_arch = "wasm32")]
    pub fn show_progress(&self) -> bool {
        false
    }
}

fn add_index(m: &mut Bytes, idx: i32, env: &mut Env) -> Result<(), Box<AssemblerError>> {
    if !(-128..=127).contains(&idx) {
        env.add_warning(Box::new(AssemblerWarning::AssemblingError {
            msg: format!("index {idx} does not fit in 8 bits")
        }));
    }
    let val = (idx & 0xFF) as u8;
    add_byte(m, val);
    Ok(())
}

fn add_byte(m: &mut Bytes, b: u8) {
    m.push(b);
}

fn add_word(m: &mut Bytes, w: u16) {
    m.push((w % 256) as u8);
    m.push((w / 256) as u8);
}

fn add_index_register_code(m: &mut Bytes, r: IndexRegister16) {
    add_byte(m, indexed_register16_to_code(r));
}

const DD: u8 = 0xDD;
const FD: u8 = 0xFD;

pub trait MyDefault {
    fn default() -> Self;
}

/// Several passes are needed to properly assemble a source file.
///
/// This structure allows to code which pass is going to be analysed.
/// First pass consists in collecting the various labels to manipulate and so on. Some labels stay unknown at this moment.
/// Second pass serves to get the final values
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssemblingPass {
    Uninitialized,
    FirstPass,
    SecondPass(usize), // and subsequent
    Finished(usize),
    ListingPass // pass dedicated to the listing production
}

impl AssemblingPass {
    // maximum number of passes to avoid to use all memory of the computer and make it freezes
    const MAX_PASSES: usize = 10;
}

impl fmt::Display for AssemblingPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = match self {
            AssemblingPass::Uninitialized => "Uninitialized",
            AssemblingPass::FirstPass => "1",
            AssemblingPass::SecondPass(_) => "2",
            AssemblingPass::Finished(_) => "Finished",
            AssemblingPass::ListingPass => "Listing"
        };
        write!(f, "{content}")
    }
}

#[allow(missing_docs)]
#[allow(unused)]
impl AssemblingPass {
    fn is_uninitialized(self) -> bool {
        matches!(self, AssemblingPass::Uninitialized)
    }

    pub fn is_finished(self) -> bool {
        matches!(self, AssemblingPass::Finished(_))
    }

    pub fn is_first_pass(self) -> bool {
        matches!(self, AssemblingPass::FirstPass)
    }

    pub fn is_second_pass(self) -> bool {
        matches!(self, AssemblingPass::SecondPass(_))
    }

    pub fn is_listing_pass(self) -> bool {
        matches!(self, AssemblingPass::ListingPass)
    }

    pub fn nb_passes(&self) -> Option<usize> {
        match self {
            AssemblingPass::FirstPass => Some(1),
            AssemblingPass::Finished(count) | AssemblingPass::SecondPass(count) => Some(*count),
            _ => None
        }
    }

    fn next_pass(self) -> Self {
        match self {
            AssemblingPass::Uninitialized => AssemblingPass::FirstPass,
            AssemblingPass::FirstPass => AssemblingPass::SecondPass(2),
            AssemblingPass::SecondPass(count) => AssemblingPass::Finished(count),
            AssemblingPass::Finished(_) | AssemblingPass::ListingPass => panic!()
        }
    }

    pub fn nb_passes_exceeded(&self) -> bool {
        self.nb_passes().unwrap_or(0) > Self::MAX_PASSES
    }
}

/// Trait to implement for each type of token.
/// it allows to drive the appropriate data vonversion
pub trait Visited {
    /// Make all the necessary for the given token
    fn visited(&self, env: &mut Env) -> Result<(), Box<AssemblerError>>;
}

impl Visited for Token {
    fn visited(&self, env: &mut Env) -> Result<(), Box<AssemblerError>> {
        env.visit_token(self)
    }
}

impl Visited for LocatedToken {
    fn visited(&self, env: &mut Env) -> Result<(), Box<AssemblerError>> {
        // dbg!(env.output_address, self.as_token());
        Ok(env
            .visit_located_token(self)
            .map_err(|e| e.locate(self.span().clone()))?)
    }
}

type AssemblerWarning = AssemblerError;

/// Store all the necessary information when handling a crunched section
#[derive(Clone)]
struct CrunchedSectionState {
    /// Start of the crunched section for code assembled from the sources.
    /// None for code assembled from tokens
    // mainly usefull for error messages; nothing more
    #[allow(unused)]
    crunched_section_start: Option<Z80Span>
}

impl CrunchedSectionState {
    pub fn new(span: Option<Z80Span>) -> Self {
        CrunchedSectionState {
            crunched_section_start: span
        }
    }
}

#[derive(Clone)]
pub struct CharsetEncoding {
    lut: std::collections::HashMap<char, i32>
}

impl Default for CharsetEncoding {
    fn default() -> Self {
        Self::new()
    }
}

impl CharsetEncoding {
    pub fn new() -> Self {
        let mut enc = Self {
            lut: Default::default()
        };
        enc.reset();
        enc
    }

    pub fn reset(&mut self) {
        self.lut.clear()
    }

    pub fn update(
        &mut self,
        spec: &CharsetFormat,
        env: &mut Env
    ) -> Result<(), Box<AssemblerError>> {
        match spec {
            CharsetFormat::Reset => self.reset(),
            CharsetFormat::CharsList(l, s) => {
                let result = env.resolve_expr_must_never_fail(s)?;
                let mut s = env.int_forward(&result)?;
                for c in l.iter() {
                    self.lut.insert(*c, s);
                    s += 1;
                }
            },
            CharsetFormat::Char(c, i) => {
                let c = env.resolve_expr_must_never_fail(c)?.char()?;
                let result = env.resolve_expr_must_never_fail(i)?;
                let i = env.int_forward(&result)?;
                self.lut.insert(c, i);
            },
            CharsetFormat::Interval(a, b, s) => {
                let a = env.resolve_expr_must_never_fail(a)?.char()?;
                let b = env.resolve_expr_must_never_fail(b)?.char()?;
                let result = env.resolve_expr_must_never_fail(s)?;
                let mut s = env.int_forward(&result)?;
                for c in a..=b {
                    self.lut.insert(c, s);
                    s += 1;
                }
            }
        }

        Ok(())
    }

    pub fn transform_char(&self, c: char) -> u8 {
        self.lut
            .get(&c)
            .cloned()
            .unwrap_or_else(|| char_to_amscii(c, Charset::English).unwrap_or(c as _) as i32)
            as _
    }

    pub fn transform_string(&self, s: &str) -> Vec<u8> {
        s.chars().map(|c| self.transform_char(c)).collect()
    }
}

pub trait EnvEventObserver: EventObserver {}

impl<T> EnvEventObserver for T where T: EventObserver {}

/// One entry of `Env::active_frames` - see its doc comment.
#[derive(Debug, Clone)]
enum ActiveFrame {
    Expansion(ActiveExpansion),
    Include(IncludeFrame)
}

/// A macro/struct call currently being expanded - see `Env::active_frames`.
#[derive(Debug, Clone)]
struct ActiveExpansion {
    /// Keeps this expansion's buffer alive independently of the
    /// `ExpandState` that owns it - see `Env::active_frames`.
    listing: Arc<LocatedListing>,
    name: SmolStr,
    /// Where the macro/struct itself is defined.
    location: Option<SourceLocation>,
    /// Where it is called from - the span an error should point at to lead
    /// the user back to the actual call site, not just the macro body.
    call_site: Z80Span
}

/// A file currently being visited because of an `INCLUDE`/`READ` directive -
/// see `Env::active_frames`. Unlike `ActiveExpansion`, no keep-alive buffer
/// is needed: an included file's `LocatedListing` is parsed once and cached
/// for the whole assembling run (see `IncludeState::retreive_listing`), never
/// rebuilt mid-run the way a macro's `ExpandState` is, so a `Z80Span` into it
/// never goes stale.
#[derive(Debug, Clone)]
struct IncludeFrame {
    /// Where the `INCLUDE`/`READ` directive itself is written - all
    /// `include_chain_note`/`Display` actually need: the included file's own
    /// name is whatever `error`'s own span already names, unaffected by this
    /// wrapping.
    call_site: Z80Span
}

/// A one-line "how we got here" note for a single `INCLUDE`/`READ`, used
/// both for `Env::active_frames_as_notes` (the whole stack, batched) and for
/// accumulating one note at a time as a normally-propagated error travels
/// out through nested `IncludeState::handle` calls (see
/// `AssemblerError::with_chain_note`).
fn include_chain_note(call_site: &Z80Span) -> String {
    let (line, column) = call_site.relative_line_and_column();
    let call_file =
        processed_token::relative_to_project_root(&Utf8PathBuf::from(call_site.filename()));
    format!("included from {call_file}:{line}:{column}")
}

/// Same as `include_chain_note`, for a macro/struct call.
fn macro_chain_note(name: &SmolStr, location: &Option<SourceLocation>, call_site: &Z80Span) -> String {
    let (line, column) = call_site.relative_line_and_column();
    let call_file =
        processed_token::relative_to_project_root(&Utf8PathBuf::from(call_site.filename()));
    match location {
        Some(location) => {
            format!("inside MACRO {name} ({call_file}:{line}:{column}), defined in {location}")
        },
        None => format!("inside MACRO {name} ({call_file}:{line}:{column})")
    }
}

/// Environment of the assembly
#[allow(missing_docs)]
pub struct Env {
    /// Lookup directory when searching for a file. Must be pushed at each import directive and pop after
    lookup_directory_stack: Vec<Utf8PathBuf>,

    /// Current pass
    pass: AssemblingPass,
    options: EnvOptions,
    real_nb_passes: usize,
    /// If true at the end of the pass, can prematurely stop the assembling
    /// Hidden in a rwlock to allow a modification even in non mutable state
    can_skip_next_passes: RwLock<bool>,
    /// An issue in a crunched section requires an additional pass
    request_additional_pass: RwLock<bool>,
    /// true when it is an additional pass
    requested_additional_pass: bool,

    /// Check if we are assembling a crunched section as there are some limitations
    crunched_section_state: Option<CrunchedSectionState>,

    /// Stable counter of nops
    stable_counters: StableTickerCounters,

    /// gate array configuration
    ga_mmr: u8,
    /// duplicate of the output address to be sure to select the appropriate page info
    output_address: u16,
    /// Memoized `(output_address, ga_mmr, page index)` for `active_page_index`
    /// - see that method's doc comment. Needs interior mutability because
    /// `active_page_info` (many other accessors' foundation) takes `&self`;
    /// self-invalidating on the key, so this can never go stale, only
    /// briefly not-yet-warm. `Mutex`, not `Cell`: `Env` crosses threads via
    /// `Arc<RwLock<&mut Env>>` in a few places (parallel token-tree
    /// construction, gated behind the `rayon` feature) - `Cell` isn't
    /// `Sync`, which `Env` has to stay for that. The lock is never actually
    /// contended (that outer `RwLock` already serializes real access to
    /// `Env`), so this costs one uncontended lock/unlock, far cheaper than
    /// the redundant page-index recomputation it replaces.
    active_page_index_cache: std::sync::Mutex<Option<(u16, u8, usize)>>,

    /// Ensemble of pages (2 for a stock CPC) for the snapshot

    /// Memory configuration is controlled by the underlying snapshot.
    /// It will ease the generation of snapshots but may complexity the generation of files
    sna: SnaAssembler,
    // TODO remove it as it is store within the sna
    sna_version: cpclib_sna::SnapshotVersion,

    /// If buildcpr is used, we work within a Cpr
    cpr: Option<CprAssembler>,

    /// List of banks (temporary memory)
    free_banks: DecoratedPages,

    /// Counter for the unique labels within macros
    macro_seed: usize,

    /// Stack of the macro/struct expansions and `INCLUDE`d files currently
    /// being visited, innermost last, interleaved in true nesting order (an
    /// `INCLUDE` inside a macro body, or a macro called from an included
    /// file, both push/pop into this same stack). Serves two purposes for
    /// anything captured while one of these is active (e.g. a failed
    /// `assert`'s span, see `FailedAssertCommand`):
    ///
    /// - for an `Expansion`, cloning its `listing` out keeps that macro
    ///   expansion's buffer alive independently of the `ExpandState` that
    ///   originally owned it - which is dropped well before such a delayed
    ///   error is finally formatted (see `Env::handle_assert`). An `Include`
    ///   frame needs no such keep-alive - see `IncludeFrame`'s doc comment;
    /// - both let the error be wrapped in the same "error in macro call NAME
    ///   (defined in LOCATION)" / "error in included file PATH" shape the
    ///   propagated-error path already builds automatically for a genuinely
    ///   returned `Err` (`MacroCallOrBuildStruct`/`Include` arms of
    ///   `ProcessedToken::visited`) - a delayed `assert` never propagates as
    ///   an `Err` (so every assert in a file can be collected in one run),
    ///   so it never goes through that wrapping on its own and would
    ///   otherwise point only at the assert's own line, with no way back to
    ///   the call site/include that actually led there.
    active_frames: Vec<ActiveFrame>,

    /// Parsed macro/struct expansions, keyed on identity + each call
    /// argument's *resolved* string form (computed the same lazy way
    /// `expand_param` already does - see `processed_token::MacroExpansionKey`).
    /// A hit skips both the (potentially large) body-splicing step and the
    /// `winnow` parse; a miss costs exactly what it costs today. Lives for
    /// the whole run, same lifetime as `IncludeState`'s per-file cache - no
    /// eviction needed. `Arc<RwLock<_>>` so `Env::clone()` (crunched/confined
    /// sections, below) stays cheap - matches how `functions`/`sections`
    /// already keep only their *values* behind `Arc` - and so a clone
    /// usefully *shares* the cache with the `Env` it was cloned from.
    macro_expansion_cache:
        Arc<RwLock<HashMap<processed_token::MacroExpansionKey, Arc<LocatedListing>>>>,

    /// For a symbol defined while `active_frames` was non-empty, the same
    /// "how we got here" chain as `active_frames` itself, but flattened to
    /// plain, owned text (`Env::active_frames_as_notes`) at the moment of
    /// definition - unlike `active_frames`, this must survive for as long as
    /// the symbol table entry it describes might (an "already defined"
    /// error can be raised passes later, once the `Z80Span`s that were
    /// active back then are long gone), so it can never hold a live span.
    /// Keyed by the same normalized symbol name `SymbolsTable` itself uses,
    /// so a lookup here always agrees with `contains_symbol`/`any_value`
    /// regardless of case-sensitivity settings.
    symbol_definition_chains: HashMap<String, Vec<String>>,

    charset_encoding: CharsetEncoding,

    /// Track where bytes has been written to forbid overriding them when generating data
    /// BUG: should be stored individually in each bank ?
    byte_written: bool,

    symbols: SymbolsTable,

    /// Return value of the currently executed function. Is almost always None
    return_value: Option<ExprResult>,
    functions: BTreeMap<String, Arc<Function>>,

    /// Set only if the run instruction has been used
    run_options: Option<(u16, Option<u16>)>,

    /// optional object that manages the listing output
    output_trigger: Option<ListingOutputTrigger>,
    /// How deep we are inside expression evaluation.
    ///
    /// An instruction's recorded source location is the *instruction's*, never
    /// its operands' - whatever the operands contain. Resolving an expression
    /// can walk arbitrary tokens (a user `function` body most obviously, but
    /// also an `assert` condition or a `print` argument), and each of those
    /// announces itself to the listing as "we are here now". Left unguarded,
    /// `ld a, SPECTRAL_START + integral(...)` records its bytes against the
    /// `return` inside `integral`, and the debugger jumps to a line the
    /// program never executes - a function only runs at assembly time.
    ///
    /// A counter rather than a flag: expressions nest, and functions call
    /// functions.
    expression_depth: usize,
    /// Listing of symbols generator
    symbols_output: SymbolOutputGenerator,

    warnings: Vec<Box<AssemblerWarning>>,
    /// Monotonic count of every `Box<AssemblerWarning>` ever pushed via
    /// `add_warning` - unlike `warnings.len()`, this never shrinks (
    /// `merge_overriding_warnings` truncates `warnings` as it merges
    /// adjacent entries), so it is what `cleanup_warnings` compares against
    /// `warnings_cleaned_up_to` to detect "nothing new happened" cheaply.
    warning_push_count: u64,
    /// `warning_push_count`'s value as of the last `cleanup_warnings` call -
    /// see that method's doc comment for why comparing these two avoids
    /// redundant work.
    warnings_cleaned_up_to: u64,

    /// Counter to disable some instruction in rorg stuff
    nested_rorg: usize,

    /// List of all sections
    sections: HashMap<String, Arc<RwLock<Section>>>,
    /// Current section if any
    current_section: Option<Arc<RwLock<Section>>>,

    saved_files: Option<Vec<SavedFile>>,

    // Store the error that has been temporarily discarded at previous pass, by expecting they will be not be raised at current pass
    previous_pass_discarded_errors: HashSet<String>,
    // Store the error that has been temporarily discarded, by expecting they will be fixed at next pass
    current_pass_discarded_errors: HashSet<String>,

    if_token_adr_to_used_decision: HashMap<usize, bool>,
    if_token_adr_to_unused_decision: HashMap<usize, bool>,

    /// Real assembled address of every visited token that carries a real
    /// span, keyed by `Z80Span::identity()` (context identity + offset, NOT
    /// just `offset_from_start()` alone - a real project assembles several
    /// source files via `include`, each with its own span whose offset
    /// restarts at 0, so two tokens in different files can otherwise share an
    /// offset; see `Z80Span::identity`'s own doc comment). A token with no
    /// span at all (some `ListingElement` implementors this assembler can
    /// visit, e.g. plain `Token`, are not guaranteed one) is simply never
    /// recorded - there is no meaningful "position" to key it by.
    ///
    /// Only populated when `AssemblingOptions::record_token_addresses` is set
    /// - see that field's doc comment. Overwritten (never reset) pass over
    /// pass, since one `Env` is reused across the whole multi-pass assemble
    /// (`visit_tokens_all_passes_with_options`), so it naturally converges to
    /// the final pass's real addresses.
    ///
    /// Looking this up therefore only ever makes sense for a span that came
    /// from *the same parse* the assemble itself visited - a listing that was
    /// merely re-parsed (even from identical text) gets fresh contexts and
    /// simply misses, which is the safe failure mode (`Option::None`, never a
    /// wrong address).
    address_trace: HashMap<SpanIdentity, u16>,
    /// The same addresses, keyed by `(canonical file, byte offset in that
    /// file)` instead of by [`SpanIdentity`].
    ///
    /// `SpanIdentity` is deliberately *parse-local* - it exists so that two
    /// files whose spans both start at offset 0 cannot collide - which also
    /// means it cannot connect one assemble to a *different* parse of the same
    /// file. That is exactly what an editor needs: it assembles the project's
    /// entry file, then wants the addresses of tokens in its own, separately
    /// parsed copy of an `include`d file. A `(file, offset)` key survives that
    /// crossing; a `SpanIdentity` cannot.
    address_trace_by_file: HashMap<(std::path::PathBuf, usize), u16>,

    included_paths: HashSet<Utf8PathBuf>,

    map_counter: i32,

    // repeat conf
    repeat_start: ExprResult,
    repeat_step: ExprResult,

    // Output filename if set by OUTPUT directive
    output_filename: Option<String>,

    // temporary stuff
    extra_print_from_function: RwLock<Vec<PrintOrPauseCommand>>,
    extra_failed_assert_from_function: RwLock<Vec<FailedAssertCommand>>,

    // list of output commands that are generated in a restricted assembling env
    pub(crate) assembling_control_current_output_commands: Vec<ControlOutputStore>
}

impl Default for Env {
    fn default() -> Self {
        Env::new(Default::default())
    }
}


impl AsRef<Env> for Env {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Clone for Env {
    fn clone(&self) -> Self {
        Self {
            lookup_directory_stack: self.lookup_directory_stack.clone(),
            options: self.options.clone(),
            can_skip_next_passes: (*self.can_skip_next_passes.read().unwrap().deref()).into(),
            request_additional_pass: (*self.request_additional_pass.read().unwrap().deref()).into(),
            pass: self.pass,
            real_nb_passes: self.real_nb_passes,
            crunched_section_state: self.crunched_section_state.clone(),
            stable_counters: self.stable_counters.clone(),
            ga_mmr: self.ga_mmr,
            output_address: self.output_address,
            active_page_index_cache: std::sync::Mutex::new(
                *self.active_page_index_cache.lock().unwrap()
            ),
            sna: self.sna.clone(),
            sna_version: self.sna_version,
            free_banks: self.free_banks.clone(),
            macro_seed: self.macro_seed,
            active_frames: self.active_frames.clone(),
            macro_expansion_cache: self.macro_expansion_cache.clone(),
            symbol_definition_chains: self.symbol_definition_chains.clone(),
            charset_encoding: self.charset_encoding.clone(),
            byte_written: self.byte_written,
            symbols: self.symbols.clone(),
            run_options: self.run_options,
            output_trigger: self.output_trigger.clone(),
            expression_depth: self.expression_depth,
            symbols_output: self.symbols_output.clone(),
            warnings: self.warnings.clone(),
            warning_push_count: self.warning_push_count,
            warnings_cleaned_up_to: self.warnings_cleaned_up_to,
            nested_rorg: self.nested_rorg,
            sections: self.sections.clone(),
            current_section: self.current_section.clone(),
            saved_files: self.saved_files.clone(),

            if_token_adr_to_used_decision: self.if_token_adr_to_used_decision.clone(),
            if_token_adr_to_unused_decision: self.if_token_adr_to_unused_decision.clone(),
            address_trace: self.address_trace.clone(),
            address_trace_by_file: self.address_trace_by_file.clone(),
            requested_additional_pass: self.requested_additional_pass,

            functions: self.functions.clone(),
            return_value: self.return_value.clone(),

            current_pass_discarded_errors: self.current_pass_discarded_errors.clone(),
            previous_pass_discarded_errors: self.previous_pass_discarded_errors.clone(),

            included_paths: self.included_paths.clone(),
            extra_print_from_function: self
                .extra_print_from_function
                .read()
                .unwrap()
                .clone()
                .into(),
            extra_failed_assert_from_function: self
                .extra_failed_assert_from_function
                .read()
                .unwrap()
                .clone()
                .into(),

            map_counter: self.map_counter,

            cpr: self.cpr.clone(),

            repeat_start: self.repeat_start.clone(),
            repeat_step: self.repeat_step.clone(),

            output_filename: self.output_filename.clone(),

            assembling_control_current_output_commands: self
                .assembling_control_current_output_commands
                .clone()
        }
    }
}
impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Env{{ pass: {:?}, symbols {:?} }}",
            self.pass,
            self.symbols()
        )
    }
}

/// Symbols handling
impl Env {
    pub fn assemble_rst_fake<D: DataAccessElem>(
        &mut self,
        arg1: &D,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let result = self.resolve_expr_may_fail_in_first_pass(arg2.get_expression().unwrap())?;
        let val = self.int_forward(&result)?;

        let _p = match val {
            0x38 | 7 | 38 => 0b111,
            _ => {
                return Err(Box::new(AssemblerError::InvalidArgument {
                    msg: format!(
                        "Conditionnal RST cannot take {val} as argument. Expected values are 0x38|7|38."
                    )
                }));
            }
        };

        let flag = arg1.get_flag_test().unwrap();
        if flag != FlagTest::NZ
            && flag != FlagTest::Z
            && flag != FlagTest::NC
            && flag != FlagTest::C
        {
            return Err(Box::new(AssemblerError::InvalidArgument {
                msg: format!(
                    "Conditionnal RST cannot take {flag} as flag. Expected values are C|NC|Z|NZ."
                )
            }));
        }

        self.assemble_opcode_impl(
            Mnemonic::Jr,
            &Some(DataAccess::from(flag)),
            &Some(DataAccess::from(
                Expr::Label("$".into()).add(Expr::Value(1))
            )),
            &None
        )
    }

    pub fn assemble_rst<D: DataAccessElem>(
        &mut self,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();
        let result = self.resolve_expr_may_fail_in_first_pass(arg1.get_expression().unwrap())?;
        let val = self.int_forward(&result)?;

        let p = match val {
            0x00 => 0b000,
            0x08 | 1 => 0b001,
            0x10 | 2 | 10 => 0b010,
            0x18 | 3 | 18 => 0b011,
            0x20 | 4 | 20 => 0b100,
            0x28 | 5 | 28 => 0b101,
            0x30 | 6 | 30 => 0b110,
            0x38 | 7 | 38 => 0b111,
            _ => {
                return Err(Box::new(AssemblerError::InvalidArgument {
                    msg: format!("{val} is an invalid value for RST")
                }));
            }
        };
        bytes.push(0b1100_0111 | (p << 3));
        Ok(bytes)
    }

    pub fn assemble_im<D: DataAccessElem>(
        &mut self,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();
        let result = self.resolve_expr_may_fail_in_first_pass(arg1.get_expression().unwrap())?;
        let val = self.int_forward(&result)?;

        let code = match val {
            0x00 => 0x46,
            0x01 => 0x56,
            0x02 => 0x5E,
            _ => {
                return Err(Box::new(AssemblerError::InvalidArgument {
                    msg: format!("IM cannot take {val} as argument.")
                }));
            }
        };

        bytes.push(0xED);
        bytes.push(code);
        Ok(bytes)
    }

    pub fn assemble_ret<D: DataAccessElem>(
        &self,
        arg1: Option<&D>
    ) -> Result<Bytes, Box<AssemblerError>> {
        let mut bytes = Bytes::new();

        if let Some(arg1) = arg1 {
            if let Some(test) = arg1.get_flag_test() {
                let flag = flag_test_to_code(test);
                bytes.push(0b1100_0000 | (flag << 3));
            }
            else {
                return Err(Box::new(AssemblerError::InvalidArgument {
                    msg: "RET: wrong argument for ret".to_string()
                }));
            }
        }
        else {
            bytes.push(0xC9);
        }

        Ok(bytes)
    }

    pub fn options(&self) -> &EnvOptions {
        &self.options
    }

    pub fn symbols(&self) -> &SymbolsTable {
        &self.symbols
    }

    pub fn symbols_mut(&mut self) -> &mut SymbolsTable {
        &mut self.symbols
    }

    pub fn build_fname<E: ExprEvaluationExt + Debug>(
        &mut self,
        exp: &E
    ) -> Result<String, Box<AssemblerError>> {
        let fname = match self.resolve_expr_must_never_fail(exp) {
            Ok(fname) => Ok(fname),
            Err(e) => {
                match &*e {
                    // the parser consider file.ext to be a label ... because it could ! So if it is not the case we need to fallback
                    AssemblerError::UnknownSymbol { symbol, .. } => {
                        let exp_str = exp.to_string();
                        // Case-insensitive comparison since symbol names may be normalized (uppercased)
                        if exp_str.eq_ignore_ascii_case(symbol.as_str()) {
                            Ok(exp_str.into())
                        }
                        else {
                            Err(e)
                        }
                    },
                    AssemblerError::RelocatedError { error, .. } => {
                        if let AssemblerError::UnknownSymbol { symbol, .. } = error.as_ref() {
                            let exp_str = exp.to_string();
                            // Case-insensitive comparison since symbol names may be normalized (uppercased)
                            if exp_str.eq_ignore_ascii_case(symbol.as_str()) {
                                Ok(exp_str.into())
                            }
                            else {
                                Err(e)
                            }
                        }
                        else {
                            Err(e)
                        }
                    },
                    _ => Err(e)
                }
            }
        }?;
        let fname = if fname.is_string() {
            fname.string()?.to_owned()
        }
        else {
            fname.to_string()
        };
        Ok(fname)
    }

    /// Compute the expression thanks to the symbol table of the environment.
    /// If the expression is not solvable in first pass, 0 is returned.
    /// If the expression is not solvable in second pass, an error is returned
    ///
    /// However, when assembling in a crunched section, the expression MUST NOT fail. edit: why ? I do not get it now and I have removed this limitation
    /// Resolve an expression with the listing's idea of "where we are" frozen.
    ///
    /// Every expression in the assembler goes through `resolve`, and resolving
    /// one can walk arbitrary tokens - a user `function`'s body, an `assert`
    /// condition, a `print` argument. Each of those announces itself to the
    /// listing, which would leave the current position inside the expression
    /// rather than on the instruction that owns it. See `expression_depth`.
    ///
    /// The only place `resolve` should ever be called from `Env`.
    fn resolve_isolated<E: ExprEvaluationExt>(
        &mut self,
        exp: &E
    ) -> Result<ExprResult, Box<AssemblerError>> {
        self.expression_depth += 1;
        let result = exp.resolve(self);
        self.expression_depth -= 1;
        result
    }

    pub fn resolve_expr_may_fail_in_first_pass<E: ExprEvaluationExt>(
        &mut self,
        exp: &E
    ) -> Result<ExprResult, Box<AssemblerError>> {
        self.resolve_expr_may_fail_in_first_pass_with_default(exp, 0)
    }

    pub fn resolve_index_may_fail_in_first_pass<E: ExprEvaluationExt>(
        &mut self,
        (op, exp): (BinaryOperation, &E)
    ) -> Result<ExprResult, Box<AssemblerError>> {
        let res = self.resolve_expr_may_fail_in_first_pass(exp)?;
        let res = if op == BinaryOperation::Sub {
            res.neg()?
        }
        else {
            res
        };
        Ok(res)
    }

    pub fn resolve_expr_may_fail_in_first_pass_with_default<
        E: ExprEvaluationExt,
        R: Into<ExprResult>
    >(
        &mut self,
        exp: &E,
        r: R
    ) -> Result<ExprResult, Box<AssemblerError>> {
        self.track_used_symbols(exp)?;

        match self.resolve_isolated(exp) {
            Ok(value) => Ok(value),
            Err(e) => {
                // if we have no more remaining passes, we fail !
                if let Some(commands) = self.assembling_control_current_output_commands.last()
                    && !commands.has_remaining_passes()
                {
                    return Err(e);
                }

                if self.pass.is_first_pass() {
                    *self.can_skip_next_passes.write().unwrap() = false;
                    Ok(r.into())
                }
                else {
                    Err(Box::new(*e))
                }
            }
        }
    }

    /// Compute the expression thanks to the symbol table of the environment.
    /// An error is systematically raised if the expression is not solvable (i.e., labels are unknown)
    fn resolve_expr_must_never_fail<E: ExprEvaluationExt>(
        &mut self,
        exp: &E
    ) -> Result<ExprResult, Box<AssemblerError>> {
        match self.resolve_isolated(exp) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.pass.is_first_pass() {
                    *self.can_skip_next_passes.write().unwrap() = false;
                    Err(e)
                }
                else {
                    Err(e)
                }
            },
        }
    }

    pub(crate) fn add_function_parameter_to_symbols_table<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<(), Box<AssemblerError>> {
        let symbol = symbol.into();
        // // we do not test that, otherwise it is impossible to do recursive functions
        // if self.symbols().contains_symbol(symbol.clone())? {
        // return Err(AssemblerError::IncoherentCode{msg: format!("Function parameter {} already present", symbol)})
        // }
        self.symbols
            .set_symbol_to_value(symbol, ValueAndSource::new_unlocated(value))?;
        Ok(())
    }

    /// Add a symbol to the symbol table.
    /// In pass 1: the label MUST be absent
    /// In pass 2: the label MUST be present and of the same value
    fn add_symbol_to_symbol_table<E: Into<Value>>(
        &mut self,
        label: &str,
        value: E,
        location: Option<SourceLocation>
    ) -> Result<(), Box<AssemblerError>> {
        let already_present = self.symbols().contains_symbol(label)?;
        let value = value.into();
        let value = ValueAndSource::new(value, location);

        match (already_present, self.pass) {
            (true, AssemblingPass::FirstPass) => {
                Err(Box::new(AssemblerError::SymbolAlreadyExists {
                    symbol: label.to_string()
                }))
            },
            (false, AssemblingPass::SecondPass(_)) => {
                // here we weaken the test to allow multipass stuff
                if !self.requested_additional_pass && !*self.request_additional_pass.read().unwrap()
                {
                    Err(Box::new(AssemblerError::IncoherentCode {
                        msg: format!(
                            "Label {} is not present in the symbol table in pass {}. There is an issue with some  conditional code.",
                            label, self.pass
                        )
                    }))
                }
                else {
                    self.symbols_mut().set_symbol_to_value(label, value)?;
                    Ok(())
                }
            },
            (false, AssemblingPass::ListingPass) => {
                Err(Box::new(AssemblerError::IncoherentCode {
                    msg: format!(
                        "Label {} is not present in the symbol table in pass {}. There is an issue with some conditional code.",
                        label, self.pass
                    )
                }))
            },
            (false, AssemblingPass::FirstPass) | (false, AssemblingPass::Uninitialized) => {
                self.symbols_mut().set_symbol_to_value(label, value)?;
                Ok(())
            },
            (true, AssemblingPass::SecondPass(_) | AssemblingPass::ListingPass) => {
                self.symbols_mut().update_symbol_to_value(label, value)?;
                Ok(())
            },
            (..) => {
                panic!(
                    "add_symbol_to_symbol_table / unmanaged case {}, {}, {} {:#?}",
                    self.pass, label, already_present, value
                )
            }
        }
    }

    /// Track the symbols for an expression that has been properly executed
    fn track_used_symbols<E: ExprEvaluationExt>(&mut self, e: &E) -> Result<(), AssemblerError> {
        e.symbols_used()
            .into_iter()
            .map(|symbol| self.symbols.use_symbol(symbol.as_ref()))
            .filter_map(Result::err)
            .try_for_each(|e| Result::Err(Box::new(AssemblerError::from(e))))?;
        Ok(())
    }
}
/// Report handling
impl Env {
    pub fn report(&self, start: &Instant) -> Report<'_> {
        Report::from((self, start))
    }
}

/// Include once handling {
impl Env {
    #[inline]
    fn included_marks_reset(&mut self) {
        self.included_paths.clear();
    }

    #[inline]
    fn included_marks_includes(&self, path: &Utf8PathBuf) -> bool {
        self.included_paths.contains(path)
    }

    #[inline]
    fn included_marks_add(&mut self, path: Utf8PathBuf) {
        self.included_paths.insert(path);
    }
}

/// Handle the file search relatively to the current file
impl Env {
    fn set_current_working_directory<P: Into<Utf8PathBuf>>(&mut self, p: P) {
        self.lookup_directory_stack.push(p.into())
    }

    pub fn enter_current_working_file<P: AsRef<Utf8Path>>(&mut self, f: P) {
        let f = f.as_ref();
        debug_assert!(f.is_file() || f.starts_with("inner://"));
        self.set_current_working_directory(f.parent().unwrap());
    }

    pub fn leave_current_working_file(&mut self) -> Option<Utf8PathBuf> {
        self.lookup_directory_stack.pop()
    }

    pub fn get_current_working_directory(&self) -> Option<&Utf8Path> {
        self.lookup_directory_stack.last().map(|p| p.as_path())
    }

    pub fn has_current_working_directory(&self) -> bool {
        !self.lookup_directory_stack.is_empty()
    }
}

/// Error handling
impl Env {
    /// If the error has not been raised at the previous pass, store it and do not propagate it. Otherwise, propagate it
    pub fn add_error_discardable_one_pass(
        &mut self,
        e: Box<AssemblerError>
    ) -> Result<(), Box<AssemblerError>> {
        let repr = SimplerAssemblerError(&e).to_string();
        if self.previous_pass_discarded_errors.contains(&repr) {
            Err(e)
        }
        else {
            self.current_pass_discarded_errors.insert(repr);
            Ok(())
        }
    }
}
/// Namespace handling
impl Env {
    fn enter_namespace(&mut self, namespace: &str) -> Result<(), Box<AssemblerError>> {
        if namespace.as_bytes().contains(&b'.') {
            return Err(Box::new(AssemblerError::AssemblingError {
                msg: format!("Invalid namespace \"{namespace}\"")
            }));
        }
        self.symbols_mut().enter_namespace(namespace);
        Ok(())
    }

    fn leave_namespace(&mut self) -> Result<Symbol, Box<AssemblerError>> {
        self.symbols_mut()
            .leave_namespace()
            .map_err(|e| Box::new(e.into()))
    }
}

impl Env {
    /// Return the current state of writting of the assembler
    fn output_kind(&self) -> OutputKind {
        if self.cpr.is_some() {
            OutputKind::Cpr
        }
        else if self.free_banks.selected_index.is_some() {
            OutputKind::FreeBank
        }
        else {
            OutputKind::Snapshot
        }
    }
}

#[allow(missing_docs)]
impl Env {
    /// Create an environment that embeds a copy of the given table and is configured to be in the latest pass.
    /// Mainly used for tests.
    /// TODO use bon here
    pub fn with_table(symbols: &SymbolsTable) -> Self {
        let mut env = Self::new(Default::default());
        env.symbols = symbols.clone();
        env.pass = AssemblingPass::SecondPass(1);
        env
    }

    pub fn warnings(&self) -> &[Box<AssemblerWarning>] {
        &self.warnings
    }

    /// Manage the play with data for the output listing
    /// The listing recorder, unless we are inside an expression.
    ///
    /// Every route into the listing goes through here, because "an
    /// instruction's location is the instruction's, not its operands'" is not
    /// only about which token is current: assigning a symbol also overrides the
    /// address column with the assigned value, and `acc = 0` inside a
    /// `function` body would leave the *next* instruction's row claiming to be
    /// at address 0. See `expression_depth`.
    pub(crate) fn listing_trigger(&mut self) -> Option<&mut ListingOutputTrigger> {
        if self.expression_depth > 0 {
            return None;
        }
        self.output_trigger.as_mut()
    }

    /// Whether the listing is recording right now.
    fn listing_is_recording(&self) -> bool {
        self.expression_depth == 0 && self.pass.is_listing_pass() && self.output_trigger.is_some()
    }

    fn handle_output_trigger(&mut self, new: &LocatedToken) {
        // Tokens reached while resolving an expression are not where the code
        // is: they are how a value was computed. Announcing them would move the
        // listing's position into a `function` body that emits nothing and
        // leave it there for the instruction that follows.
        if self.expression_depth > 0 {
            return;
        }
        if self.listing_is_recording() {
            let code_addr = self.logical_code_address();
            let phy_addr = self.logical_to_physical_address(self.logical_output_address());

            let kind = if self.crunched_section_state.is_some() {
                AddressKind::CrunchedArea
            }
            else {
                AddressKind::Address
            };
            let symbols = Some(self.symbols() as *const _);

            let trig = self.listing_trigger().unwrap();

            trig.new_token(new, code_addr as _, kind, phy_addr, symbols);
        }
    }

    fn retrieve_options_symbols(&mut self) {
        let opts = self.options();
        let available: Vec<_> = opts
            .symbols()
            .available_symbols()
            .filter_map(|symbol| {
                opts.symbols()
                    .any_value(symbol.clone())
                    .ok()
                    .and_then(|v| v)
                    .map(|val| (symbol.clone(), val.clone()))
            })
            .collect();

        for (symbol, val) in available {
            let _ = self.symbols_mut().set_symbol_to_value(symbol, val);
        }
    }

    /// Start a new pass by cleaning up datastructures.
    /// The only thing to keep is the symbol table
    pub(crate) fn start_new_pass(&mut self) -> Result<(), Box<AssemblerError>> {
        if self.options().assemble_options().debug() {
            eprintln!("Start a new pass {}", self.pass());
            let _ = self.handle_print();
            let _ = self.generate_symbols_output(
                std::io::stderr().borrow_mut(),
                SymbolOutputFormat::Winape
            );
        }

        self.included_marks_reset();
        self.requested_additional_pass |= !self.current_pass_discarded_errors.is_empty();

        let mut can_change_request = true;
        if !self.pass.is_listing_pass() {
            self.pass = if self.real_nb_passes == 0
                || !*self.can_skip_next_passes.read().unwrap().deref()
            {
                if *self.request_additional_pass.read().unwrap() {
                    if self.pass.is_first_pass() {
                        can_change_request = false;
                    }

                    AssemblingPass::SecondPass(self.pass().nb_passes().unwrap() + 1)
                }
                else {
                    self.pass.next_pass()
                }
            }
            else if !*self.request_additional_pass.read().unwrap() {
                AssemblingPass::Finished(self.pass().nb_passes().unwrap())
            }
            else {
                AssemblingPass::SecondPass(self.pass().nb_passes().unwrap() + 1)
            };
        }

        if self.pass().nb_passes_exceeded() {
            return Err(Box::new(AssemblerError::MaximumNumberOfPassesReached(
                self.pass().nb_passes().unwrap()
            )));
        }

        if !self.pass.is_finished() || self.pass.is_listing_pass() {
            if !self.pass.is_listing_pass() {
                self.real_nb_passes += 1;
            }

            std::mem::swap(
                &mut self.current_pass_discarded_errors,
                &mut self.previous_pass_discarded_errors
            );
            self.current_pass_discarded_errors.clear();

            self.stable_counters.new_pass();
            self.run_options = None;

            // A pass starts where the file starts: outside every global label.
            //
            // Local labels are stored as `<current global>.<local>`, and the
            // current global was left wherever the *previous* pass ended. So a
            // `.loop` written before the first global label became `.loop` in
            // pass 1 and `message.loop` in pass 2 - "Label .loop is not present
            // in the symbol table in pass 2", for a program with nothing
            // conditional in it at all.
            let _ = self.symbols_mut().set_current_global_label("");

            self.sna.reset_written_bytes();
            if let Some(cpr) = self.cpr.as_mut() {
                cpr.reset_written_bytes()
            }
            self.free_banks.reset_written_bytes();

            self.warnings.retain(|elem| !elem.is_override_memory());
            self.sna.pages_info.iter_mut().for_each(|p| p.new_pass());

            self.sections
                .iter_mut()
                .for_each(|s| s.1.write().unwrap().new_pass());
            self.current_section = None;

            self.free_banks.pages.iter_mut().for_each(|bank| {
                bank.1.new_pass();
                bank.2.fill(false);
            });
            self.free_banks.selected_index = None;

            // environnement is not reset when assembling is finished
            self.output_address = 0;
            let page_info = self.active_page_info_mut();
            page_info.logical_outputadr = 0;
            page_info.logical_codeadr = 0;
            self.update_dollar();

            self.ga_mmr = 0xC0;
            self.macro_seed = 0;
            self.charset_encoding.reset();
            // self.sna = Default::default(); // We finally keep the snapshot for the memory function
            // self.sna_version = cpclib_sna::SnapshotVersion::V3; // why changing it ?

            self.can_skip_next_passes = true.into();
            if can_change_request {
                self.request_additional_pass = false.into();
            }

            // reset the symbol table
            self.symbols.new_pass();
            self.retrieve_options_symbols();

            if self.options.show_progress() {
                #[cfg(not(target_arch = "wasm32"))]
                Progress::progress().new_pass();
            }
        }

        let basm_version = built_info::PKG_VERSION.to_owned();
        let basm = true;
        let basm_feature_hfe = cfg!(feature = "hfe");

        if AssemblingPass::FirstPass == self.pass {
            let _ = self.add_symbol_to_symbol_table("BASM_VERSION", basm_version, None);
            let _ = self.add_symbol_to_symbol_table("BASM", basm, None);
            let _ = self.add_symbol_to_symbol_table("BASM_FEATURE_HFE", basm_feature_hfe, None);
        }
        else {
            let _ = self.symbols_mut().update_symbol_to_value(
                "BASM_VERSION",
                ValueAndSource::new(basm_version, Option::<SourceLocation>::None)
            );
            let _ = self.symbols_mut().update_symbol_to_value(
                "BASM",
                ValueAndSource::new(basm, Option::<SourceLocation>::None)
            );
            let _ = self.symbols_mut().update_symbol_to_value(
                "BASM_FEATURE_HFE",
                ValueAndSource::new(basm_feature_hfe, Option::<SourceLocation>::None)
            );
        }

        Ok(())
    }

    /// Handle the actions to do after assembling.
    /// ATM it is only the save of data for each page
    pub fn handle_post_actions<'token, T>(
        &mut self,
        tokens: &'token [T]
    ) -> Result<(Option<RemuChunk>, Option<WabpChunk>), Box<AssemblerError>>
    where
        T: Visited + ToSimpleToken + Debug + Sync + ListingElement + MayHaveSpan,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement + Sync,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt + ExprElement,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        self.handle_print()?;
        self.handle_assert()?;

        let remu_in_sna = self
            .options()
            .assemble_options()
            .get_flag(crate::AssemblingOptionFlags::SnaRemu);
        let remu_in_file = self
            .options()
            .assemble_options()
            .get_flag(crate::AssemblingOptionFlags::RemuInFile);
        let wabp_in_file = self
            .options()
            .assemble_options()
            .get_flag(crate::AssemblingOptionFlags::WabpInFile);

        let mut remu = if remu_in_file || remu_in_sna {
            Some(RemuChunk::empty())
        }
        else {
            None
        };

        let mut wabp = if wabp_in_file {
            Some(WabpChunk::empty())
        }
        else {
            None
        };

        self.handle_breakpoints(&mut remu.as_mut(), &mut wabp.as_mut())?;
        self.handle_sna_symbols(&mut remu.as_mut())?;

        if let Some(remu) = &remu
            && remu_in_sna
        {
            self.sna.add_chunk(remu.clone());
        }

        self.run_listing_pass(tokens)?;

        // BUG this is definitevely a bug
        // - I have moved file saving here because output was wrong when done before listing
        // - Ther eis no reason to do that. it should even be the opposite
        self.saved_files = Some(self.handle_file_save()?);

        Ok((remu, wabp))
    }

    /// Re-visit the whole program one last time, with the listing machinery
    /// switched on.
    ///
    /// A separate pass because the listing needs *final* addresses: during the
    /// convergence passes an address can still move, and a listing built from
    /// them would be quietly wrong. Running it once, after everything has
    /// settled, is also why it costs one extra pass rather than one per pass.
    ///
    /// Drives both the textual/HTML listing and - when
    /// [`AssemblingOptions::record_source_map`] asked for it - the source map,
    /// which are two consumers of the same records rather than two mechanisms.
    /// Does nothing at all when no output was requested.
    pub fn run_listing_pass<'token, T>(
        &mut self,
        tokens: &'token [T]
    ) -> Result<(), Box<AssemblerError>>
    where
        T: Visited + ToSimpleToken + Debug + Sync + ListingElement + MayHaveSpan,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement + Sync,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt + ExprElement,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        if self.options().assemble_options().output_builder.is_none() {
            return Ok(());
        }

        let mut tokens = processed_token::build_processed_tokens_list(
            tokens,
            std::sync::Arc::new(std::sync::RwLock::new(self))
        )
        .expect("No errors must occur here");
        self.pass = AssemblingPass::ListingPass;
        self.start_new_pass()?;
        processed_token::visit_processed_tokens(&mut tokens, self)
            .map_err(|e| eprintln!("{e}"))
            .expect("No error can arise in listing output mode; there is a bug somewhere");

        if let Some(trigger) = self.listing_trigger() {
            trigger.finish();
        }
        Ok(())
    }

    // Add the symbols in the snapshot
    fn handle_sna_symbols(
        &mut self,
        remu: &mut Option<&mut RemuChunk>
    ) -> Result<(), Box<AssemblerError>> {
        let options = self.options().assemble_options().clone();
        if options.get_flag(crate::AssemblingOptionFlags::SnaSymb) {
            let ace_chunk = self.symbols_output.build_ace_snapshot_chunk(self.symbols());
            self.sna.add_chunk(ace_chunk);
        }

        if options.get_flag(crate::AssemblingOptionFlags::SnaRemu) {
            self.symbols_output
                .fill_remu_snapshot_chunk(self.symbols(), remu.as_mut().unwrap());
        }

        Ok(())
    }

    /// We handle breakpoint ONLY for the pages stored in the snapshot
    /// as they are stored inside a chunk of the snapshot:
    /// If one day another export is coded, we could export the others too.
    fn handle_breakpoints(
        &mut self,
        remu: &mut Option<&mut RemuChunk>,
        wabp: &mut Option<&mut WabpChunk>
    ) -> Result<(), Box<AssemblerError>> {
        let mut winape_chunk = if self
            .options()
            .assemble_options()
            .get_flag(crate::AssemblingOptionFlags::SnaBrks)
        {
            Some(WinapeBreakPointChunk::empty())
        }
        else {
            None
        };
        let mut ace_chunk = if self
            .options()
            .assemble_options()
            .get_flag(crate::AssemblingOptionFlags::SnaBrkc)
        {
            Some(AceBreakPointChunk::empty())
        }
        else {
            None
        };

        let pages_mmr = MMR_PAGES_SELECTION;
        for (activepage, _page) in pages_mmr[0..self.sna.pages_info.len()].iter().enumerate() {
            for brk in self.sna.pages_info[activepage].collect_breakpoints() {
                let info = &brk.info;
                self.observer().emit_stderr(&format!("{info}"));

                if let Some(chunk) = winape_chunk.as_mut()
                    && let Some(brk) = brk.winape()
                {
                    chunk.add_breakpoint(brk);
                }
                if let Some(chunk) = ace_chunk.as_mut()
                    && let Some(brk) = brk.ace()
                {
                    chunk.add_breakpoint(brk);
                }

                if let Some(chunk) = remu.as_mut() {
                    chunk.add_entry(&brk.remu().into());
                }

                if let Some(chunk) = wabp.as_mut() {
                    chunk.add_breakpoint(brk.wabp());
                }
            }
        }

        if let Some(chunk) = winape_chunk
            && chunk.nb_breakpoints() > 0
        {
            self.sna.add_chunk(chunk);
        }

        if let Some(chunk) = ace_chunk
            && chunk.nb_breakpoints() > 0
        {
            self.sna.add_chunk(chunk);
        }

        Ok(())
    }

    fn handle_assert(&mut self) -> Result<(), Box<AssemblerError>> {
        let backup = self.ga_mmr;

        // ga values to properly switch the pages
        let pages_mmr = MMR_PAGES_SELECTION;

        let mut assert_failures: Option<Box<AssemblerError>> = None;

        let mut handle_page = |page: &PageInformation| {
            let l_errors: Result<(), Box<AssemblerError>> = page.collect_assert_failure();
            match (&mut assert_failures, l_errors) {
                (_, Ok(_)) => {
                    // nothing to do
                },
                (Some(existing), Err(new_err)) => {
                    match (existing.as_mut(), *new_err) {
                        (
                            AssemblerError::MultipleErrors { errors: e1 },
                            AssemblerError::MultipleErrors { errors: mut e2 }
                        ) => {
                            e1.append(&mut e2);
                        },
                        _ => unimplemented!()
                    }
                },
                (None, Err(l_errors)) => {
                    assert_failures = Some(l_errors);
                },
                _ => unimplemented!()
            }
        };

        for (activepage, page) in pages_mmr[0..self.sna.pages_info.len()].iter().enumerate() {
            self.ga_mmr = *page;
            let page = &self.sna.pages_info[activepage];
            handle_page(page);
        }

        for page in self.free_banks.page_infos() {
            handle_page(page);
        }

        if let Some(cpr) = self.cpr.as_ref() {
            for page in cpr.page_infos() {
                handle_page(page)
            }
        }

        self.ga_mmr = backup;

        // All possible messages have been printed.
        // Errors are generated for the others
        if let Some(errors) = assert_failures {
            Err(errors)
        }
        else {
            Ok(())
        }
    }

    pub fn observer(&self) -> Arc<dyn EnvEventObserver> {
        Arc::clone(&self.options().observer)
    }

    pub fn handle_print(&mut self) -> Result<(), Box<AssemblerError>> {
        let backup = self.ga_mmr;

        // ga values to properly switch the pages
        let pages_mmr = MMR_PAGES_SELECTION;

        let mut print_errors: Option<Box<AssemblerError>> = None;
        let observer = self.observer();
        let dry_run = self.options().assemble_options().dry_run();

        let mut handle_page_info = |page: &PageInformation| {
            let l_errors: Result<(), Box<AssemblerError>> =
                page.execute_print_or_pause(observer.deref(), dry_run);
            match (&mut print_errors, l_errors) {
                (_, Ok(_)) => {
                    // nothing to do
                },
                (Some(existing), Err(new_err)) => {
                    match (existing.as_mut(), *new_err) {
                        (
                            AssemblerError::MultipleErrors { errors: e1 },
                            AssemblerError::MultipleErrors { errors: mut e2 }
                        ) => {
                            e1.append(&mut e2);
                        },
                        _ => unreachable!()
                    }
                },
                (None, Err(l_errors)) => {
                    print_errors = Some(l_errors);
                },
                _ => unreachable!()
            }
        };

        // Print from the snapshot
        for (activepage, page) in pages_mmr[0..self.sna.pages_info.len()].iter().enumerate() {
            self.ga_mmr = *page;
            let page_info = &self.sna.pages_info[activepage];

            handle_page_info(page_info);
        }
        self.ga_mmr = backup;

        // Print free banks
        for page in self.free_banks.page_infos() {
            handle_page_info(page);
        }

        // Print from CPR
        if let Some(cpr) = self.cpr.as_ref() {
            for page in cpr.page_infos() {
                handle_page_info(page);
            }
        }

        // All possible messages have been printed.
        // Errors are generated for the others
        if let Some(errors) = print_errors {
            Err(errors)
        }
        else {
            Ok(())
        }
    }

    fn handle_file_save(&mut self) -> Result<Vec<SavedFile>, Box<AssemblerError>> {
        let backup = self.ga_mmr;

        // ga values to properly switch the pages
        let pages_mmr = MMR_PAGES_SELECTION;

        let mut saved_files = Vec::new();

        // count the number of files to save to build the process bar
        let nb_files_to_save = {
            let mut nb_files_to_save: u64 = 0;
            nb_files_to_save += pages_mmr[0..self.sna.pages_info.len()]
                .iter()
                .enumerate()
                .map(|(activepage, page)| {
                    self.ga_mmr = *page;
                    self.sna.pages_info[activepage].nb_files_to_save() as u64
                })
                .sum::<u64>();
            nb_files_to_save += self
                .free_banks
                .pages
                .iter()
                .map(|b| b.1.nb_files_to_save() as u64)
                .sum::<u64>();

            nb_files_to_save
        };

        if self.options.show_progress() {
            #[cfg(not(target_arch = "wasm32"))]
            Progress::progress().create_save_bar(nb_files_to_save);
        }

        // save from snapshot. cannot be done in parallel
        for (activepage, _page) in pages_mmr[0..self.sna.pages_info.len()].iter().enumerate() {
            //  eprintln!("ACTIVEPAGE. {:x}", &activepage);
            //  eprintln!("PAGE. {:x}", &page);

            for mma in self.sna.pages_info[activepage].get_save_mmrs() {
                self.ga_mmr = mma;
                let mut saved = self.sna.pages_info[activepage].execute_save(self, mma)?;
                saved_files.append(&mut saved);
            }
        }

        // save from extra memory / can be done in parallel as it does not concerns memory
        self.ga_mmr = 0xC0;

        #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
        let iter = {
            let can_save_in_parallel = self
                .free_banks
                .pages
                .iter()
                .all(|b| b.1.can_save_in_parallel());
            CondIterator::new(&self.free_banks.pages, can_save_in_parallel)
        };
        #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
        let iter = self.free_banks.pages.iter();
        let (saved, errors): (Vec<Vec<SavedFile>>, Vec<Box<AssemblerError>>) = iter
            .map(|bank| bank.1.execute_save(self, self.ga_mmr))
            .partition_map(|res| {
                match res {
                    Ok(val) => Either::Left(val),
                    Err(e) => Either::Right(e)
                }
            });
        if !errors.is_empty() {
            return Err(Box::new(AssemblerError::MultipleErrors { errors }));
        }
        for mut s in saved {
            saved_files.append(&mut s);
        }

        if self.options().show_progress() {
            #[cfg(not(target_arch = "wasm32"))]
            Progress::progress().finish_save();
        }
        // restor memory conf
        self.ga_mmr = backup;
        Ok(saved_files)
    }
}

/// Output handling
impl Env {
    /// The `Snapshot`-mode active page index, computed from
    /// `(output_address, ga_mmr)` - the only two fields it actually depends
    /// on - and memoized in `active_page_index_cache` against exactly that
    /// pair. Self-invalidating by construction: whichever of the two last
    /// changed, the cached key simply stops matching and this recomputes -
    /// there is no separate "remember to invalidate" step to get wrong, so
    /// this can never observe a stale page. `output_byte` alone reads the
    /// active page (directly or via `logical_output_address`/
    /// `physical_output_address`/etc., which all route through
    /// `active_page_info`) closer to a dozen times without either field
    /// changing in between, so this turns what was a dozen re-derivations
    /// of the same value into one.
    fn active_page_index(&self) -> usize {
        let mut cache = self.active_page_index_cache.lock().unwrap();
        if let Some((cached_addr, cached_mmr, cached_idx)) = *cache
            && cached_addr == self.output_address
            && cached_mmr == self.ga_mmr
        {
            return cached_idx;
        }

        let idx = self
            .logical_to_physical_address(self.output_address)
            .to_memory()
            .page() as usize;
        *cache = Some((self.output_address, self.ga_mmr, idx));
        idx
    }

    /// TODO
    fn active_page_info(&self) -> &PageInformation {
        match self.output_kind() {
            OutputKind::Snapshot => &self.sna.pages_info[self.active_page_index()],
            OutputKind::Cpr => {
                self.cpr
                    .as_ref()
                    .unwrap()
                    .selected_active_page_info()
                    .unwrap()
            },
            OutputKind::FreeBank => self.free_banks.selected_active_page_info().unwrap()
        }
    }

    fn active_page_info_mut(&mut self) -> &mut PageInformation {
        match self.output_kind() {
            OutputKind::Snapshot => {
                let active_page = self.active_page_index();
                &mut self.sna.pages_info[active_page]
            },
            OutputKind::Cpr => {
                let cpr = self.cpr.as_mut().unwrap();
                cpr.selected_active_page_info_mut().unwrap()
            },
            OutputKind::FreeBank => self.free_banks.selected_active_page_info_mut().unwrap()
        }
    }

    fn page_info_for_logical_address_mut(
        &mut self,
        address: u16
    ) -> Result<&mut PageInformation, Box<AssemblerError>> {
        match self.output_kind() {
            OutputKind::Snapshot => {
                let active_page =
                    self.logical_to_physical_address(address).to_memory().page() as usize;
                Ok(&mut self.sna.pages_info[active_page])
            },
            OutputKind::Cpr => {
                let cpr = self.cpr.as_mut().ok_or_else(|| {
                    Box::new(AssemblerError::BugInAssembler {
                        file: file!(),
                        line: line!(),
                        msg: "CPR is None when output_kind is Cpr".to_string()
                    })
                })?;
                cpr.selected_active_page_info_mut().ok_or_else(|| {
                    Box::new(AssemblerError::BugInAssembler {
                        file: file!(),
                        line: line!(),
                        msg: "No active page info in CPR".to_string()
                    })
                })
            },
            OutputKind::FreeBank => {
                self.free_banks
                    .selected_active_page_info_mut()
                    .ok_or_else(|| {
                        Box::new(AssemblerError::BugInAssembler {
                            file: file!(),
                            line: line!(),
                            msg: "No active page info in free banks".to_string()
                        })
                    })
            },
        }
    }

    fn written_bytes(&self) -> &BitVec {
        match self.output_kind() {
            OutputKind::Snapshot => &self.sna.written_bytes,
            OutputKind::Cpr => {
                self.cpr
                    .as_ref()
                    .unwrap()
                    .selected_written_bytes()
                    .expect("No bank selected")
            },
            OutputKind::FreeBank => {
                self.free_banks
                    .selected_written_bytes()
                    .expect("No bank selected")
            },
        }
    }

    /// Return the address where the next byte will be written
    pub fn logical_output_address(&self) -> u16 {
        self.active_page_info().logical_outputadr
    }

    /// The real assembled address of `span`, if `record_token_addresses` was
    /// enabled for this assemble (see that option's doc comment) **and**
    /// `span` came from the same parse this `Env` actually visited - a
    /// same-text-but-re-parsed span gets a fresh context and simply misses
    /// here, safely, rather than returning a wrong address (see
    /// `address_trace`'s own doc comment).
    pub fn address_of_span(&self, span: &Z80Span) -> Option<u16> {
        self.address_trace.get(&span.identity()).copied()
    }

    /// The real assembled address of the token starting at `offset` in `file`.
    ///
    /// Unlike [`Self::address_of_span`] this survives being asked from a
    /// different parse of the same file, which is what lets an editor assemble
    /// a project's entry point and then resolve addresses for a file that
    /// entry merely `include`s. The caller is responsible for the offsets
    /// still being meaningful - i.e. for the file not having changed since the
    /// assemble.
    /// Where each source line ended up, when
    /// [`AssemblingOptions::record_source_map`] asked for it.
    ///
    /// Populated during the listing pass, i.e. once, after the address passes
    /// have converged - so every address here is final.
    /// Every breakpoint the program asked for, across all pages.
    ///
    /// Exposed for debuggers: a `BREAKPOINT` directive is the author saying
    /// where they want to stop, and that is worth more than anything an editor
    /// can infer. What an emulator does with the richer forms is its own
    /// business - see `AssembledBreakpoint`.
    pub fn assembled_breakpoints(
        &self
    ) -> Vec<crate::assembler::delayed_command::AssembledBreakpoint> {
        self.sna
            .pages_info
            .iter()
            .flat_map(|page| page.collect_breakpoints())
            .map(|command| command.described())
            .collect()
    }

    pub fn source_map(&self) -> Option<crate::assembler::listing_output::RawSourceMap> {
        let builder = self.options().assemble_options().output_builder.as_ref()?;
        let mut output = builder.write().unwrap();
        // The listing accumulates a line and only emits it when the *next*
        // line starts, so the last one of the program - and the last iteration
        // of a `REPEAT`, whose body never "changes line" - is still pending
        // here. Flushing first is the difference between a map that accounts
        // for every emitted byte and one that quietly loses the tail.
        output.process_current_line();
        output.source_map_snapshot()
    }

    pub fn address_of_file_offset(&self, file: &std::path::Path, offset: usize) -> Option<u16> {
        let path = canonical_source_path(file.to_str()?)?;
        self.address_trace_by_file.get(&(path, offset)).copied()
    }

    pub fn physical_output_address(&self) -> PhysicalAddress {
        self.logical_to_physical_address(self.logical_output_address())
    }

    pub fn physical_code_address(&self) -> PhysicalAddress {
        self.logical_to_physical_address(self.logical_code_address())
    }

    /// Return the address of dollar
    pub fn logical_code_address(&self) -> u16 {
        self.active_page_info().logical_codeadr
    }

    pub fn output_limit_address(&self) -> u16 {
        self.active_page_info().output_limit
    }

    pub fn code_limit_address(&self) -> u16 {
        self.active_page_info().code_limit
    }

    pub fn start_address(&self) -> Option<u16> {
        self.active_page_info().startadr
    }

    pub fn maximum_address(&self) -> u16 {
        self.active_page_info().maxadr
    }

    /// . Update the value of $ in the symbol table in order to take the current  output address
    pub fn update_dollar(&mut self) {
        if let Some(cpr) = &self.cpr
            && cpr.is_empty()
        {
            return;
        }

        let code_addr = self.logical_to_physical_address(self.logical_code_address());
        let output_addr = self.logical_to_physical_address(self.logical_output_address());

        self.symbols.set_current_address(code_addr);
        self.symbols.set_current_output_address(output_addr);
    }

    /// Produce the memory for the required limits
    /// TODO check that the implementation is still correct with snapshot inclusion
    /// BUG  does not take into account extra bank configuration
    pub fn get_memory(&self, start: u16, size: u16) -> Vec<u8> {
        //     dbg!(self.ga_mmr);
        let mut mem = Vec::new();
        let start = start as u32;
        let size = size as u32;
        for pos in start..(start + size) {
            let address = self.logical_to_physical_address(pos as _);
            mem.push(self.peek(&address));
        }
        mem
    }

    /// Returns the stream of bytes produced for a 64k compilation
    pub fn produced_bytes(&self) -> Vec<u8> {
        let (start, length) = match self.start_address() {
            Some(start) => {
                if start > self.maximum_address() {
                    (0, 0)
                }
                else {
                    (start, self.maximum_address() as usize - start as usize + 1)
                }
            },
            None => (0, 0)
        };

        self.get_memory(start, length as _)
    }

    /// Returns the address of the 1st written byte
    pub fn loading_address(&self) -> Option<u16> {
        self.start_address()
    }

    /// Returns the address from when to start the program
    /// TODO really configure this address
    pub fn execution_address(&self) -> Option<u16> {
        self.start_address()
    }

    /// Output one byte either in the appropriate bank of the snapshot or in the temporary bank
    /// return true if it raised an override warning
    pub fn output_byte(&mut self, v: u8) -> Result<bool, Box<AssemblerError>> {
        //   dbg!(self.logical_output_address(), self.output_address);
        if self.logical_output_address() != self.output_address {
            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!(
                    "Sync issue with output address (0x{:x} != 0x{:x})",
                    self.logical_output_address(),
                    self.output_address
                )
            }));
        }

        // dbg!(self.output_address(), &v);
        let physical_output_address: PhysicalAddress = self.physical_output_address();
        let physical_code_address: PhysicalAddress = self.physical_code_address();

        // Check if it is legal to output the value
        // if self.logical_code_address() > self.limit_address() || (self.active_page_info().fail_next_write_if_zero && self.logical_code_address() == 0)
        if self.physical_output_address().address() > self.output_limit_address()
            || (self.active_page_info().fail_next_write_if_zero && self.logical_code_address() == 0)
        {
            return Err(Box::new(AssemblerError::OutputExceedsLimits(
                physical_output_address,
                self.output_limit_address() as _
            )));
        }

        if self.logical_code_address() > self.code_limit_address()
            || (self.active_page_info().fail_next_write_if_zero && self.logical_code_address() == 0)
        {
            return Err(Box::new(AssemblerError::OutputExceedsLimits(
                physical_code_address,
                self.code_limit_address() as _
            )));
        }
        for protected_area in &self.active_page_info().protected_areas {
            if protected_area.contains(&{ self.logical_code_address() }) {
                return Err(Box::new(AssemblerError::OutputProtected {
                    area: protected_area.clone(),
                    address: self.logical_code_address() as _
                }));
            }
        }

        self.byte_written = true;
        if let Some(commands) = self.assembling_control_current_output_commands.last_mut() {
            commands.store_byte(v);
        }

        // TODO move the next in a function to reuse when executing the command
        // update the maximm 64k position
        self.active_page_info_mut().maxadr =
            self.maximum_address().max(self.logical_output_address());
        if self.active_page_info_mut().startadr.is_none() {
            self.active_page_info_mut().startadr = Some(self.logical_output_address());
        };

        let abstract_address = physical_output_address.offset_in_cpc();
        let already_used = if let Some(access) = self.written_bytes().get(abstract_address as usize)
        {
            *access
        }
        else {
            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!(
                    "Wrong size of memory access {} > {}",
                    abstract_address,
                    self.written_bytes().len()
                )
            }));
        };

        let r#override = if already_used {
            let r#override = AssemblerWarning::OverrideMemory(physical_output_address, 1);
            if self.allow_memory_override() {
                self.add_warning(Box::new(r#override));
                true
            }
            else {
                return Err(Box::new(r#override));
            }
        }
        else {
            false
        };

        if self.free_banks.selected_index.is_none()
            && let Some(section) = &self.current_section
        {
            let section = section.read().unwrap();
            if !section.contains(physical_output_address.address()) {
                return Err(Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "SECTION error: write address 0x{:x} out of range [Ox{:}-Ox{:}]",
                        physical_output_address.address(),
                        section.start,
                        section.stop
                    )
                }));
            }
        }

        match self.output_kind() {
            OutputKind::Snapshot => {
                self.sna.set_byte(abstract_address, v);
            },
            OutputKind::Cpr => {
                self.cpr
                    .as_mut()
                    .unwrap()
                    .set_byte(self.output_address, v)?;
            },
            OutputKind::FreeBank => {
                self.free_banks.set_byte(self.output_address, v);
            }
        }

        // Add the byte to the listing space
        if self.listing_is_recording() {
            self.listing_trigger().unwrap().write_byte(v);
        }

        self.active_page_info_mut().logical_outputadr =
            self.logical_output_address().wrapping_add(1);
        self.output_address = self.logical_output_address();
        self.active_page_info_mut().logical_codeadr = self.logical_code_address().wrapping_add(1);

        // we have written all memory and are trying to restart
        if self.logical_output_address() == 0 {
            self.active_page_info_mut().fail_next_write_if_zero = true;
        }

        {
            let (output, code) = (
                self.active_page_info().logical_outputadr,
                self.active_page_info().logical_codeadr
            );

            if let Some(section) = &mut self.current_section {
                let mut section = section.write().unwrap();
                section.output_adr = output;
                section.code_adr = code;
                section.max_output_adr = section.max_output_adr.max(output);
            }
        }

        self.update_dollar();

        Ok(r#override)
    }

    pub fn allow_memory_override(&self) -> bool {
        !self.options().assemble_options().forbid_memory_override()
    }

    /// Write consecutives bytes
    pub fn output_bytes(&mut self, bytes: &[u8]) -> Result<(), Box<AssemblerError>> {
        //        dbg!(self.logical_output_address(), bytes);

        let mut previously_overrided = false;
        for b in bytes.iter() {
            let currently_overrided = self.output_byte(*b)?;

            if self.options().assemble_options().enable_warnings {
                match (previously_overrided, currently_overrided) {
                    (true, true) => {
                        // remove the latest warning as it is a duplicate
                        let extra_override_idx = self
                            .warnings
                            .iter_mut()
                            .rev()
                            .position(|w| {
                                if let AssemblerError::OverrideMemory(..) = &**w {
                                    true
                                }
                                else {
                                    false
                                }
                            })
                            .unwrap(); // cannot fail by construction
                        self.warnings
                            .remove(self.warnings.len() - 1 - extra_override_idx); // rev impose to change index order

                        // get the last override warning and update it
                        let r#override = self
                            .warnings
                            .iter_mut()
                            .rev()
                            .find(|w| {
                                if let AssemblerError::OverrideMemory(..) = &***w {
                                    true
                                }
                                else {
                                    false
                                }
                            })
                            .unwrap(); // cannot fail by construction

                        // increase its size
                        match &mut **r#override {
                            AssemblerError::OverrideMemory(_, size) => {
                                *size += 1;
                            },
                            _ => unreachable!()
                        };
                    },
                    _ => {
                        // nothing to do
                    }
                }
            }

            previously_overrided = currently_overrided;
        }

        Ok(())
    }

    pub fn peek(&self, address: &PhysicalAddress) -> u8 {
        // we assume that the physical address in argument matches the current configuration
        match self.output_kind() {
            OutputKind::Snapshot => {
                let address = address.to_memory().offset_in_cpc();
                self.sna.get_byte(address)
            },
            OutputKind::Cpr => {
                let address = address.to_cpr().address();
                self.cpr.as_ref().unwrap().get_byte(address as _).unwrap()
            },
            OutputKind::FreeBank => {
                let address = address.to_bank().address();
                self.free_banks.get_byte(address as _).unwrap()
            }
        }
    }

    pub fn poke(&mut self, byte: u8, address: &PhysicalAddress) -> Result<(), Box<AssemblerError>> {
        // need modification to work when the physical address is different
        match self.output_kind() {
            OutputKind::Snapshot => {
                let address = address.to_memory().offset_in_cpc();
                self.sna.set_byte(address, byte)
            },
            OutputKind::Cpr => {
                let address = address.to_cpr().address();
                self.cpr.as_mut().unwrap().set_byte(address as _, byte)?
            },
            OutputKind::FreeBank => {
                let address = address.to_bank().address();
                self.free_banks.set_byte(address as _, byte)
            }
        }

        Ok(())
    }

    /// Get the size of the generated binary.
    /// ATTENTION it can only work when geneating 0x10000 files
    pub fn size(&self) -> u16 {
        if self.start_address().is_none() {
            panic!("Unable to compute size now");
        }
        else {
            self.logical_output_address() - self.start_address().unwrap()
        }
    }

    /// Evaluate the expression according to the current state of the environment
    pub fn eval(&mut self, expr: &Expr) -> Result<ExprResult, Box<AssemblerError>> {
        self.resolve_isolated(expr)
    }

    pub fn sna(&self) -> &cpclib_sna::Snapshot {
        &self.sna
    }

    pub fn sna_version(&self) -> cpclib_sna::SnapshotVersion {
        self.sna_version
    }

    /// No-op under `dry_run` — guarantees `BUILDSNA` never writes a real file.
    pub fn save_sna<P: AsRef<Utf8Path>>(&self, fname: P) -> Result<(), std::io::Error> {
        if self.options().assemble_options().dry_run() {
            return Ok(());
        }
        self.sna().save(fname, self.sna_version())
    }

    /// No-op under `dry_run` — guarantees `BUILDCPR` never writes a real file.
    pub fn save_cpr<P: AsRef<Utf8Path>>(&self, fname: P) -> Result<(), Box<AssemblerError>> {
        if self.options().assemble_options().dry_run() {
            return Ok(());
        }
        let cpr_asm = self.cpr.as_ref().unwrap();
        let cpr = cpr_asm.build_cpr()?;
        Ok(cpr
            .save(fname)
            .map_err(|e| AssemblerError::IOError { msg: e.to_string() })?)
    }

    /// Compute the relative address. Is authorized to fail at first pass
    fn absolute_to_relative_may_fail_in_first_pass(
        &self,
        address: i32,
        opcode_delta: i32
    ) -> Result<u8, Box<AssemblerError>> {
        match absolute_to_relative(address, opcode_delta, self.symbols()) {
            Ok(value) => Ok(value),
            Err(error) => {
                if self.pass.is_first_pass() {
                    Ok(0)
                }
                else {
                    Err(Box::new(AssemblerError::RelativeAddressUncomputable {
                        address,
                        pass: self.pass,
                        error: Box::new(*error)
                    }))
                }
            },
        }
    }
}

impl Env {
    #[inline(always)]
    pub fn add_warning(&mut self, warning: Box<AssemblerWarning>) {
        let opts = self.options().assemble_options();
        if opts.enable_warnings && opts.is_warning_category_enabled(warning.warning_category()) {
            self.warnings.push(warning);
            self.warning_push_count += 1;
        }
    }

    /// Push each `cpclib_tokens::ExprWarning` as its own `Env::warnings`
    /// entry - `env.warnings` is itself the "stack", so no need to nest a
    /// `Vec` inside one `AssemblerError` value.
    #[inline(always)]
    pub fn add_expression_warnings(&mut self, warnings: Vec<cpclib_tokens::ExprWarning>) {
        for w in warnings {
            self.add_warning(Box::new(AssemblerError::ExpressionWarning(w)));
        }
    }

    /// How many warnings have been recorded so far - paired with
    /// `locate_warnings_since` to retroactively locate whatever a call
    /// pushed, the same idiom `visit_located_token` already uses at
    /// whole-statement granularity.
    #[inline(always)]
    pub(crate) fn warnings_len(&self) -> usize {
        self.warnings.len()
    }

    /// Locate (idempotently - a no-op for anything already located by a
    /// deeper call) every warning pushed since `from`.
    #[inline(always)]
    pub(crate) fn locate_warnings_since(&mut self, from: usize, span: Z80Span) {
        for warning in &mut self.warnings[from..] {
            **warning = warning.clone().locate_warning(span.clone());
        }
    }

    /// Truncate `value` into an 8-bit slot, warning first if it doesn't
    /// actually fit. Always returns the same truncated byte an unchecked
    /// `(value & 0xFF) as u8` would have - this only adds a warning, it
    /// never changes what gets assembled. The message text is a fixed
    /// format (`cpclib-lsp` parses it back out to enrich its own
    /// diagnostic, the same way it already treats "fake instruction" as an
    /// implicit contract string) - keep it in sync if changed.
    #[inline(always)]
    pub fn checked_byte(&mut self, value: i32) -> u8 {
        if !(-128..=255).contains(&value) {
            self.add_expression_warnings(vec![cpclib_tokens::ExprWarning {
                kind: cpclib_tokens::ExprWarningKind::Overflow,
                message: format!("value {value} does not fit in 8 bits")
            }]);
        }
        (value & 0xFF) as u8
    }

    /// Like `checked_byte`, for a 16-bit slot.
    #[inline(always)]
    pub fn checked_word(&mut self, value: i32) -> u16 {
        if !(-32768..=65535).contains(&value) {
            self.add_expression_warnings(vec![cpclib_tokens::ExprWarning {
                kind: cpclib_tokens::ExprWarningKind::Overflow,
                message: format!("value {value} does not fit in 16 bits")
            }]);
        }
        (value & 0xFFFF) as u16
    }

    /// Coerce `result` to `i32`, forwarding any truncation warnings it
    /// carries (i.e. `result` was a real/float value) as assembler
    /// warnings. The building block every call site that resolves a
    /// user-authored expression into assembled output should use instead of
    /// calling `ExprResult::int()` directly, so a stray float never gets
    /// silently rounded without the user being told.
    #[inline(always)]
    pub fn int_forward(&mut self, result: &ExprResult) -> Result<i32, ExpressionTypeError> {
        let (val, warnings) = result.int()?;
        self.add_expression_warnings(warnings);
        Ok(val)
    }
}

/// Visit directives
impl Env {
    fn visit_org<E: ExprElement + ExprEvaluationExt + Debug>(
        &mut self,
        address: &E,
        address2: Option<&E>
    ) -> Result<(), Box<AssemblerError>> {
        // org $ set org to the output address (cf. rasm)
        let code_adr = if address2.is_none() && address.is_label_value("$") {
            if self.start_address().is_none() {
                return Err(Box::new(AssemblerError::InvalidArgument {
                    msg: "ORG: $ cannot be used now".into()
                }));
            }
            self.logical_output_address() as i32
        }
        else {
            { let __r = self.resolve_expr_must_never_fail(address)?; self.int_forward(&__r)? }
        };

        let output_adr = if let Some(address2) = address2 {
            if address2.is_label_value("$") {
                self.logical_output_address() as i32 // XXX here is must be code not output. I do not understand ...
            }
            else {
                { let __r = self.resolve_expr_must_never_fail(address2)?; self.int_forward(&__r)? }
            }
        }
        else {
            code_adr
        };

        if let Some(commands) = self.assembling_control_current_output_commands.last_mut() {
            commands.store_org(code_adr as _, output_adr as _);
        }

        self.visit_org_set_arguments(code_adr as _, output_adr as _)
    }

    pub fn visit_org_set_arguments(
        &mut self,
        code_adr: u16,
        output_adr: u16
    ) -> Result<(), Box<AssemblerError>> {
        // TODO Check overlapping region
        let page_info = {
            let page_info = self.page_info_for_logical_address_mut(output_adr as _)?;
            page_info.logical_outputadr = output_adr as _;
            page_info.logical_codeadr = code_adr as _;
            page_info.fail_next_write_if_zero = false;
            page_info
        };

        // Specify start address at first use
        let logical_output_address = page_info.logical_outputadr;
        let start_address = match page_info.startadr {
            Some(val) => val.min(logical_output_address),
            None => logical_output_address
        };
        page_info.startadr = Some(start_address);

        self.output_address = output_adr as _;
        self.update_dollar();

        // update the erroneous information for the listing
        if self.listing_is_recording() {
            let output_adr = self.logical_to_physical_address(output_adr as _);
            let trigger = self.listing_trigger().unwrap();

            trigger.replace_code_address(&code_adr.into());
            trigger.replace_physical_address(output_adr);
        }

        if self.logical_output_address() != self.output_address {
            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!(
                    "BUG in assembler: 0x{:x}!=0x{:x} in pass {:?}",
                    self.logical_output_address(),
                    self.output_address,
                    self.pass
                )
            }));
        }

        Ok(())
    }

    fn visit_breakpoint<E: ExprEvaluationExt + ExprElement + MayHaveSpan>(
        &mut self,
        address: Option<&E>,
        r#type: Option<&RemuBreakPointType>,
        access: Option<&RemuBreakPointAccessMode>,
        run: Option<&RemuBreakPointRunMode>,
        mask: Option<&E>,
        size: Option<&E>,
        value: Option<&E>,
        value_mask: Option<&E>,
        condition: Option<&E>,
        name: Option<&E>,
        step: Option<&E>,
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>> {
        let brk = if r#type.is_none()
            && access.is_none()
            && run.is_none()
            && mask.is_none()
            && size.is_none()
            && value.is_none()
            && value_mask.is_none()
            && condition.is_none()
            && name.is_none()
            && step.is_none()
        {
            // here we manipulate a very simple breakpoint
            let (current_address, page): (u16, u8) = if let Some(exp) = address {
                if exp.is_label() {
                    let label = exp.label();
                    let symbols = self.symbols();
                    let value: &Value = symbols.any_value(label)?.unwrap();
                    match value {
                        Value::Expr(expr_result) => {
                            let expr_result = expr_result.clone();
                            (self.int_forward(&expr_result)? as _, 0)
                        },
                        Value::Address(physical_address) => {
                            (
                                physical_address.address(),
                                physical_address.remu_bank() as _
                            )
                        }, /* BUG we lost the differentiation between the different kind of addresses, */
                        _ => todo!()
                    }
                }
                else {
                    let current_address = { let __r = self.resolve_expr_must_never_fail(exp)?; self.int_forward(&__r)? };
                    let page = 0; // BUG should be dynamic and not hard coded !
                    (current_address as _, page)
                }
            }
            else {
                let current_address = self.logical_code_address();
                // ATM the breakpoints only work in SNA
                // To allow them in CPR there is a bit of work to do
                let page = match self
                    .logical_to_physical_address(current_address)
                    .to_memory()
                    .page()
                {
                    0 => 0,
                    1 => 1,
                    _ => {
                        return Err(Box::new(AssemblerError::BugInAssembler {
                            file: file!(),
                            line: line!(),
                            msg: format!(
                                "Page selection not handled 0x{:x}",
                                self.logical_to_physical_address(current_address)
                                    .to_memory()
                                    .page()
                            )
                        }));
                    }
                };

                (current_address, page)
            };

            BreakpointCommand::new_simple(current_address, page, span.cloned())
        }
        else {
            // here we manipulate an advanced breakpoint of Ace

            let mut brk = AdvancedRemuBreakPoint::default();
            brk.addr = if let Some(address) = address {
                ({ let __r = self.resolve_expr_must_never_fail(address)?; self.int_forward(&__r)? }) as u16
            }
            else {
                self.logical_code_address()
            };
            if let Some(r#type) = r#type {
                brk.brk_type = r#type.clone();
            }
            if let Some(access) = access {
                brk.access_mode = access.clone();
            }
            if let Some(run) = run {
                brk.run_mode = run.clone();
            }
            if let Some(mask) = mask {
                brk.mask = { let __r = self.resolve_expr_may_fail_in_first_pass(mask)?; self.int_forward(&__r)? } as u16;
            }
            if let Some(size) = size {
                brk.size = { let __r = self.resolve_expr_may_fail_in_first_pass(size)?; self.int_forward(&__r)? } as u16;
            }
            if let Some(value) = value {
                brk.value = { let __r = self.resolve_expr_may_fail_in_first_pass(value)?; self.int_forward(&__r)? } as u8;
            }
            if let Some(value_mask) = value_mask {
                let result = self.resolve_expr_may_fail_in_first_pass(value_mask)?;
                brk.val_mask = self.int_forward(&result)? as u8;
            }
            if let Some(step) = step {
                brk.step = Some({ let __r = self.resolve_expr_may_fail_in_first_pass(step)?; self.int_forward(&__r)? } as _);
            }
            if let Some(condition) = condition {
                let cond = self.resolve_expr_may_fail_in_first_pass(condition)?;
                let cond = cond.string()?;
                brk.condition.replace(String127::try_new(cond).map_err(|_| {
                    let e = AssemblerError::AssemblingError {
                        msg: "Condition is too long".into()
                    };
                    if condition.has_span() {
                        e.locate(condition.span().clone())
                    }
                    else {
                        e
                    }
                })?);
            }
            if let Some(name) = name {
                let n = self.resolve_expr_may_fail_in_first_pass(name)?;
                let n = n.string()?;
                brk.name.replace(String127::try_new(n).map_err(|_| {
                    let e = AssemblerError::AssemblingError {
                        msg: "Name is too long".into()
                    };
                    if name.has_span() {
                        e.locate(name.span().clone())
                    }
                    else {
                        e
                    }
                })?);
            }

            BreakpointCommand::from((brk, span.cloned()))
        };

        if self
            .options()
            .assemble_options()
            .get_flag(crate::AssemblingOptionFlags::BreakpointAsOpcode)
        {
            // XXX here we are dumb and add breakpoints unconditionnaly
            // TODO do it only for exec ones
            self.output_byte(0xED)?;
            self.output_byte(0xFF)?;
        }
        else {
            self.active_page_info_mut().add_breakpoint_command(brk);
        }

        Ok(())
    }
}

#[allow(missing_docs)]
impl Env {
    /// Get the output filename set by the OUTPUT directive
    pub fn output_filename(&self) -> Option<&str> {
        self.output_filename.as_deref()
    }

    /// Write in w the list of symbols
    pub fn generate_symbols_output<W: Write>(
        &mut self,
        w: &mut W,
        fmt: SymbolOutputFormat
    ) -> std::io::Result<()> {
        let warnings = self.symbols_output.generate(w, self.symbols(), fmt)?;
        self.add_expression_warnings(warnings);
        Ok(())
    }

    /// Visit all the tokens of the slice of tokens.
    /// Return true if an additional pass is requested
    pub fn visit_listing<T: ListingElement + Visited + MayHaveSpan>(
        &mut self,
        listing: &[T]
    ) -> Result<(), Box<AssemblerError>> {
        for token in listing.iter() {
            token.visited(self)?;
        }

        Ok(())
    }

    /// TODO set the limit for the current page
    fn visit_limit<E: ExprEvaluationExt>(&mut self, exp: &E) -> Result<(), Box<AssemblerError>> {
        let value = { let __r = self.resolve_expr_must_never_fail(exp)?; self.int_forward(&__r)? };
        let in_crunched_section = self.crunched_section_state.is_some();

        if value <= 0 {
            return Err(Box::new(AssemblerError::AssemblingError {
                msg: format!("It is a nonsense to define a limit of {value}")
            }));
        }

        if value > 0xFFFF {
            return Err(Box::new(AssemblerError::AssemblingError {
                msg: format!(
                    "It is a nonsense to define a limit of {value} that exceeds hardware limitations."
                )
            }));
        }

        if in_crunched_section {
            self.active_page_info_mut().code_limit = value as _;
            if self.code_limit_address() <= self.maximum_address() {
                return Err(Box::new(AssemblerError::OutputAlreadyExceedsLimits(
                    self.code_limit_address() as _
                )));
            }
            if self.code_limit_address() == 0 {
                eprintln!("[WARNING] Do you really want to set a limit of 0 ?");
            }
        }
        else {
            self.active_page_info_mut().output_limit = value as _;
            if self.output_limit_address() <= self.maximum_address() {
                return Err(Box::new(AssemblerError::OutputAlreadyExceedsLimits(
                    self.output_limit_address() as _
                )));
            }
            if self.output_limit_address() == 0 {
                eprintln!("[WARNING] Do you really want to set a limit of 0 ?");
            }
        }

        Ok(())
    }

    fn visit_map<E: ExprEvaluationExt>(&mut self, exp: &E) -> Result<(), Box<AssemblerError>> {
        let value = { let __r = self.resolve_expr_must_never_fail(exp)?; self.int_forward(&__r)? };
        self.map_counter = value;

        Ok(())
    }

    // Remove the global part if needed and change if if needed
    fn handle_global_and_local_labels<'s>(
        &mut self,
        label: &'s str
    ) -> Result<&'s str, Box<AssemblerError>> {
        let label = if let Some(dot_pos) = label[1..].find(".") {
            let global = &label[0..(dot_pos + 1)];
            let local = &label[(dot_pos + 1)..label.len()];
            let current = self.symbols().get_current_label().as_ref();
            if global != current {
                self.symbols_mut().set_current_label(global)?;
            }
            local
        }
        else {
            label
        };

        Ok(label)
    }

    fn visit_label<S: SourceString + MayHaveSpan>(
        &mut self,
        label_span: S
    ) -> Result<(), Box<AssemblerError>> {
        let label = self.symbols().normalize_symbol(label_span.as_str());
        let label = label.value();

        // Increment proximity counter BEFORE any checks if defining _ label
        if label == "_" {
            self.symbols_mut().increment_proximity_counter();
        }

        // A label cannot be defined multiple times
        let res = if self.symbols().contains_symbol(label)?
            && (self.pass.is_first_pass()
                || !(self.symbols().kind(label)? == "address"
                    || self.symbols().kind(label)? == "any"))
        {
            Err(Box::new(AssemblerError::AlreadyDefinedSymbol {
                symbol: self
                    .symbols()
                    .extend_local_and_patterns_for_symbol(label)
                    .map(std::convert::Into::<SmolStr>::into)
                    .unwrap_or_else(|_| SmolStr::from(label)),
                kind: self.symbols().kind(label)?.into(),
                here: self
                    .symbols()
                    .any_value(label)
                    .unwrap()
                    .unwrap()
                    .location()
                    .cloned()
            }))
        }
        else {
            // Remember how *this* definition was reached, in case a later
            // label with the same name conflicts with it - see
            // `Env::symbol_definition_chains`. Keyed by the same normalized
            // `label` the error branch above looks up with, *before*
            // `handle_global_and_local_labels` below resolves it further.
            if !self.active_frames.is_empty() {
                self.symbol_definition_chains
                    .insert(label.to_string(), self.active_frames_as_notes());
            }

            // TODO we should make the expansion right now because it is fucked up otherwise

            let label = self.handle_global_and_local_labels(label)?;
            // XXX limit: should not be done here as it may start by {...} that contains when interpreted.}
            if !label.starts_with('.') {
                let _ = self.symbols_mut().set_current_label(label);
            }

            // If the current address is not set up, we force it to be 0
            let value = self.symbols().current_address().unwrap_or_default();
            let addr = self.logical_to_physical_address(value);

            self.add_symbol_to_symbol_table(
                label,
                addr,
                label_span.possible_span().map(|s| s.into())
            )
        };

        // Try to fallback on a macro call - parser is not that much great
        if let Err(err) = &res
            && let AssemblerError::AlreadyDefinedSymbol { kind, .. } = err.as_ref()
            && (kind == "macro" || kind == "struct")
        {
            let message = AssemblerError::AssemblingError {
                    msg:
                        "Use (void) for macros or structs with no parameters to disambiguate them with labels"
                            .to_owned()
                };
            if self.options().assemble_options().force_void() {
                return Err(Box::new(message));
            }
            else {
                // self.add_warning(message);
            }

            // I'm really unsure of memory safety in case of bugs
            let macro_token = Token::MacroCall(label.into(), Default::default());
            let mut processed_token = build_processed_token(
                &macro_token,
                std::sync::Arc::new(std::sync::RwLock::new(self))
            )?;
            return processed_token.visited(self);
        }

        // Locate with *this* occurrence's own span, and attach how the
        // *original* definition was itself reached, only now - not as
        // fields of `AlreadyDefinedSymbol` itself, which would put a
        // variable number of extra lines between the "error: ..." line and
        // the "-->"/"┌─ file:line:col" line a consumer (cpclib-vscode's
        // `$basm` problem matcher) expects right after it. `with_chain_note`
        // instead appends after the whole codespan block already rendered
        // by locating first - see `AlreadyDefinedSymbol`'s doc comment.
        res.map_err(|e| {
            if matches!(e.as_ref(), AssemblerError::AlreadyDefinedSymbol { .. }) {
                let mut e = match label_span.possible_span() {
                    Some(span) => Box::new((*e).locate(span.clone())),
                    None => e
                };
                for note in self
                    .symbol_definition_chains
                    .get(label)
                    .cloned()
                    .unwrap_or_default()
                {
                    e = e.with_chain_note(note);
                }
                e
            }
            else {
                e
            }
        })
    }

    fn visit_noexport<S: AsRef<str> + Display>(
        &mut self,
        labels: &[S]
    ) -> Result<(), Box<AssemblerError>> {
        if labels.is_empty() {
            self.symbols_output.forbid_all_symbols();
        }
        else {
            labels
                .iter()
                .for_each(|l| self.symbols_output.forbid_symbol(l.as_ref()));
        }

        Ok(())
    }

    fn visit_export<S: AsRef<str> + Display>(
        &mut self,
        labels: &[S]
    ) -> Result<(), Box<AssemblerError>> {
        if labels.is_empty() {
            self.symbols_output.allow_all_symbols();
        }
        else {
            labels
                .iter()
                .for_each(|l| self.symbols_output.allow_symbol(l.as_ref()));
        }

        Ok(())
    }

    fn visit_multi_pushes<D: DataAccessElem>(
        &mut self,
        regs: &[D]
    ) -> Result<(), Box<AssemblerError>> {
        // pre-size assuming 2 bytes per push; actual size may vary slightly
        let (oks, errs): (Vec<Bytes>, Vec<Box<AssemblerError>>) = regs
            .iter()
            .map(|reg| self.assemble_push(reg))
            .partition_map(|res| {
                match res {
                    Ok(val) => Either::Left(val),
                    Err(e) => Either::Right(e)
                }
            });
        if !errs.is_empty() {
            return Err(Box::new(AssemblerError::MultipleErrors { errors: errs }));
        }
        let mut result = Vec::with_capacity(regs.len().saturating_mul(2));
        for ok in oks {
            result.extend_from_slice(&ok);
        }
        self.output_bytes(&result)
    }

    fn visit_multi_pops<D: DataAccessElem>(
        &mut self,
        regs: &[D]
    ) -> Result<(), Box<AssemblerError>> {
        // pre-size assuming 2 bytes per pop; actual size may vary slightly
        let (oks, errs): (Vec<Bytes>, Vec<Box<AssemblerError>>) = regs
            .iter()
            .map(|reg| self.assemble_pop(reg))
            .partition_map(|res| {
                match res {
                    Ok(val) => Either::Left(val),
                    Err(e) => Either::Right(e)
                }
            });
        if !errs.is_empty() {
            return Err(Box::new(AssemblerError::MultipleErrors { errors: errs }));
        }
        let mut result = Vec::with_capacity(regs.len().saturating_mul(2));
        for ok in oks {
            result.extend_from_slice(&ok);
        }
        self.output_bytes(&result)
    }

    // TODO move that part n processed_tokens ?
    pub fn visit_macro_definition(
        &mut self,
        name: &str,
        arguments: &[&str],
        code: &str,
        source: Option<&Z80Span>,
        flavor: AssemblerFlavor,
        has_variadic: bool
    ) -> Result<(), Box<AssemblerError>> {
        // ignore if it is the very same macro. That can happen with orgams
        if let Some(r#macro) = self.symbols().macro_value(name)? {
            if r#macro.code().trim() == code.trim() {
                return Ok(());
            }
            else {
                let diff = prettydiff::diff_lines(r#macro.code().trim(), code.trim())
                    .names("Previous macro", "Current macro")
                    .set_show_lines(true)
                    .set_diff_only(true)
                    .format();
                let msg = format!("Macro name `{name}` already exists. {diff}");
                return Err(Box::new(AssemblerError::AlreadyRenderedError(msg)));
            }
        }

        // raise an error if already exists
        if self.pass.is_first_pass() && self.symbols().contains_symbol(name)? {
            return Err(Box::new(AssemblerError::SymbolAlreadyExists {
                symbol: name.to_owned()
            }));
        }

        // Warn if the body looks like it contains a swallowed MACRO definition.
        // When ENDM/MEND is missing, the byte-level body scanner grabs the ENDM of the
        // *next* macro, silently swallowing that macro's definition into this body.
        let has_swallowed_macro = code.lines().any(|line| {
            let t = line.trim().to_ascii_uppercase();
            t.starts_with("MACRO ")
                || t.starts_with("MACRO\t")
                || t == "MACRO"
                || t.contains(" MACRO ")
                || t.contains("\tMACRO ")
                || t.contains(" MACRO\t")
                || t.ends_with(" MACRO")
                || t.ends_with("\tMACRO")
        });
        if has_swallowed_macro {
            self.add_warning(Box::new(AssemblerWarning::AlreadyRenderedError(format!(
                "Macro `{name}` body contains what looks like another MACRO definition — \
                 likely caused by a missing ENDM/MEND before `{name}`."
            ))));
        }

        let tokenized_content =
            cpclib_tokens::macro_segment::tokenize_macro_body(code, arguments, has_variadic);
        for index in
            crate::unused_bindings::unused_macro_parameter_indices(arguments, &tokenized_content)
        {
            let msg = format!("'{}' is never used in this macro's body", arguments[index]);
            match source {
                Some(source_span) => {
                    let (line, column) = source_span.relative_line_and_column();
                    let len = {
                        let text: &str = source_span.as_ref();
                        text.lines().next().map(str::len).unwrap_or(0)
                    };
                    self.add_warning(Box::new(
                        AssemblerWarning::AlreadyRenderedWarningWithLocation {
                            msg,
                            line: line as u32,
                            column: column as u32,
                            len: len as u32
                        }
                    ));
                },
                None => {
                    self.add_warning(Box::new(AssemblerWarning::AlreadyRenderedError(msg)));
                }
            }
        }

        let location: Option<SourceLocation> = source.map(|s| s.into());
        let source = source.map(|s| s.into());

        let r#macro = ValueMacro::new(
            name.into(),
            arguments,
            code.to_owned(),
            tokenized_content,
            source,
            flavor,
            has_variadic
        );
        self.symbols_mut()
            .set_symbol_to_value(name, ValueAndSource::new(r#macro, location))?;
        Ok(())
    }

    pub fn visit_waitnops<E: ExprEvaluationExt>(
        &mut self,
        count: &E
    ) -> Result<(), Box<AssemblerError>> {
        // TODO really use a clever way
        let bytes = self.assemble_nop(Mnemonic::Nop, Some(count))?;
        self.output_bytes(&bytes)?;

        let count = { let __r = self.resolve_expr_may_fail_in_first_pass(count)?; self.int_forward(&__r)? } as _;
        self.stable_counters.update_counters(count);
        Ok(())
    }

    pub fn visit_struct_definition<
        T: ListingElement + ToSimpleToken,
        S1: SourceString,
        S2: AsRef<str>
    >(
        &mut self,
        name: S1,
        content: &[(S2, T)],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>> {
        if self.pass.is_first_pass() && self.symbols().contains_symbol(name.as_str())? {
            return Err(Box::new(AssemblerError::SymbolAlreadyExists {
                symbol: name.as_str().to_owned()
            }));
        }

        let r#struct = Struct::new(name.as_str(), content, span.map(|s| s.into()));
        // add inner index BEFORE the structure. It should reduce infinite loops
        let mut index = 0;

        for (f, s) in r#struct.fields_size(self.symbols()) {
            self.symbols_mut()
                .set_symbol_to_value(format!("{name}.{f}"), ValueAndSource::new(index, span))?;
            index += s;
        }

        self.symbols_mut()
            .set_symbol_to_value(name.as_str(), ValueAndSource::new(r#struct, span))?;

        Ok(())
    }

    pub fn visit_buildcpr(&mut self) -> Result<(), Box<AssemblerError>> {
        if self.pass.is_first_pass() {
            self.cpr = Some(CprAssembler::default());
        }
        else {
            self.cpr.as_mut().unwrap().select(0);
        }

        self.free_banks.selected_index = None; // be sure free banks is not selected
        self.ga_mmr = 0xC0;

        Ok(())
    }

    pub fn visit_buildsna(
        &mut self,
        version: Option<&SnapshotVersion>
    ) -> Result<(), Box<AssemblerError>> {
        self.sna_version = version.cloned().unwrap_or(SnapshotVersion::V3);
        self.free_banks.selected_index = None;
        Ok(())
    }

    pub fn visit_assembler_control<C: AssemblerControlCommand>(
        &mut self,
        cmd: &C,
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>> {
        if cmd.is_restricted_assembling_environment() {
            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: "BUG in assembler. This has to be handled in processed_tokens".to_string()
            }));
        }
        else if cmd.is_print_at_parse_state() {
            // nothing to do here because printing as alrady been done
        }
        else {
            assert!(cmd.is_print_at_assembling_state());
            let print_or_error =
                match self.prepropress_string_formatted_expression(cmd.get_formatted_expr()) {
                    Ok(msg) => either::Either::Left(msg),
                    Err(error) => either::Either::Right(error)
                };

            let _ = PrintCommand {
                prefix: Some(format!("[PASS{}] ", self.pass)),
                span: span.cloned(),
                print_or_error
            }
            .execute(self.observer().deref()); // TODO use the true one
        }
        Ok(())
    }

    pub fn visit_align<E: ExprEvaluationExt>(
        &mut self,
        boundary: &E,
        fill: Option<&E>
    ) -> Result<(), Box<AssemblerError>> {
        let boundary = { let __r = self.resolve_expr_must_never_fail(boundary)?; self.int_forward(&__r)? } as u16;
        let fill = match fill {
            Some(fill) => ({ let __r = self.resolve_expr_may_fail_in_first_pass(fill)?; self.int_forward(&__r)? }) as u8,
            None => 0
        };

        const OUTPUT_ALIGN: bool = false; // TODO programmaticall change it

        while !(if OUTPUT_ALIGN {
            self.logical_output_address()
        }
        else {
            self.logical_code_address()
        })
        .is_multiple_of(boundary)
        {
            self.output_byte(fill)?;
        }

        Ok(())
    }

    fn get_section_description(&self, name: &str) -> Result<Section, Box<AssemblerError>> {
        match self.sections.get(name) {
            Some(section) => Ok(section.read().unwrap().clone()),
            None => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: format!("Section '{name}' does not exists")
                }))
            },
        }
    }

    fn visit_section<S: SourceString>(&mut self, name: S) -> Result<(), Box<AssemblerError>> {
        let section = match self.sections.get(name.as_str()) {
            Some(section) => section,
            None => {
                return Err(Box::new(AssemblerError::AssemblingError {
                    msg: format!("Section '{name}' does not exists")
                }));
            }
        };

        let (output_adr, code_adr, mmr, warning) = {
            let section = section.read().unwrap();

            let warning = if section.mmr != self.ga_mmr {
                Some(AssemblerError::AssemblingError {
                    msg: format!(
                        "Gate Array configuration is not coherent with the section. We  manually set it (0x{:x} expected instead of 0x{:x})",
                        section.mmr, self.ga_mmr
                    )
                })
            }
            else {
                None
            };

            (section.output_adr, section.code_adr, section.mmr, warning)
        };

        self.current_section = Some(Arc::clone(section));

        self.ga_mmr = mmr;
        self.output_address = output_adr;

        self.active_page_info_mut().logical_outputadr = output_adr;
        self.active_page_info_mut().logical_codeadr = code_adr;

        self.update_dollar();
        if let Some(o) = self.listing_trigger() {
            o.replace_code_address(&code_adr.into())
        }

        if let Some(warning) = warning {
            self.add_warning(Box::new(warning));
        }

        Ok(())
    }

    fn visit_range<E: ExprEvaluationExt, S: SourceString>(
        &mut self,
        name: S,
        start: &E,
        stop: &E
    ) -> Result<(), Box<AssemblerError>> {
        let start = { let __r = self.resolve_expr_must_never_fail(start)?; self.int_forward(&__r)? } as u16;
        let stop = { let __r = self.resolve_expr_must_never_fail(stop)?; self.int_forward(&__r)? } as u16;
        let mmr = self.ga_mmr;

        if let Some(section) = self.sections.get(name.as_str()) {
            let section = section.read().unwrap();
            if start != section.start
                || stop != section.stop
                || name.as_str() != section.name
                || mmr != section.mmr
            {
                return Err(Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "Section '{}' is already defined from 0x{:x} to 0x{:x} in 0x{:x}",
                        section.name, section.start, section.stop, section.mmr
                    )
                }));
            }
        }
        else {
            let section = Arc::new(RwLock::new(Section::new(name.as_str(), start, stop, mmr)));

            self.sections.insert(name.as_str().to_owned(), section);
        }

        Ok(())
    }

    fn visit_next_and_co<
        E: ExprElement + ExprEvaluationExt,
        S1: SourceString + MayHaveSpan,
        S2: SourceString
    >(
        &mut self,
        destination: S1,
        source: S2,
        delta: Option<&E>,
        can_override: bool
    ) -> Result<(), Box<AssemblerError>> {
        if !can_override
            && self.symbols.contains_symbol(destination.as_str())?
            && self.pass.is_first_pass()
        {
            let kind = self.symbols().kind(Symbol::from(destination.as_str()))?;
            return Err(Box::new(AssemblerError::AlreadyDefinedSymbol {
                symbol: destination.as_str().into(),
                kind: kind.into(),
                here: self
                    .symbols()
                    .any_value(destination.as_str())?
                    .and_then(|v| v.location().cloned())
            }));
        }

        // setup the value
        let value = self.resolve_expr_must_never_fail(&Expr::Label(source.as_str().into()))?;
        if can_override {
            self.symbols_mut()
                .assign_symbol_to_value(destination.as_str(), value.clone())?;
        }
        else {
            self.add_symbol_to_symbol_table(
                destination.as_str(),
                value.clone(),
                destination.possible_span().map(|s| s.into())
            )?;
        }
        if let Some(o) = self.listing_trigger() {
            o.replace_code_address(&value)
        }

        // increase next one
        let delta = match delta {
            Some(delta) => self.resolve_expr_must_never_fail(delta)?,
            None => 1.into()
        };
        let value = (value + delta)?;

        self.symbols_mut()
            .assign_symbol_to_value(source.as_str(), value)?;

        Ok(())
    }

    /// return the page and bank configuration for the given address at the current mmr configuration
    /// https://grimware.org/doku.php/documentations/devices/gatearray#mmr
    pub fn logical_to_physical_address(&self, address: u16) -> PhysicalAddress {
        match self.output_kind() {
            OutputKind::Snapshot => MemoryPhysicalAddress::new(address, self.ga_mmr).into(),
            OutputKind::Cpr => {
                CprPhysicalAddress::new(
                    address,
                    self.cpr.as_ref().unwrap().selected_bloc().unwrap()
                )
                .into()
            },
            OutputKind::FreeBank => {
                BankPhysicalAddress::new(address, self.free_banks.selected_index().unwrap()).into()
            },
        }
    }

    fn visit_skip<E: ExprEvaluationExt>(&mut self, exp: &E) -> Result<(), Box<AssemblerError>> {
        let amount = { let __r = self.resolve_expr_must_never_fail(exp)?; self.int_forward(&__r)? };

        // if amount < 0 {
        // return Err(AssemblerError::AlreadyRenderedError(format!("SKIP accept only positive values. {amount} is invalid")));
        // }

        let amount = amount as u16;

        let codeaddr = self
            .active_page_info()
            .logical_codeadr
            .wrapping_add(amount as _);
        let outputadr = self
            .active_page_info()
            .logical_outputadr
            .wrapping_add(amount as _);

        self.active_page_info_mut().logical_codeadr = codeaddr;
        self.active_page_info_mut().logical_outputadr = outputadr;

        self.update_dollar();
        self.output_address = outputadr;
        Ok(())
    }

    /// The keyword is named BANK, but in fact, it is a PAGE ...
    fn visit_page_or_bank<E: ExprEvaluationExt + Debug>(
        &mut self,
        exp: Option<&E>
    ) -> Result<(), Box<AssemblerError>> {
        if self.nested_rorg > 0 {
            return Err(Box::new(AssemblerError::NotAllowed));
        }

        let output_kind = self.output_kind();

        match exp {
            Some(exp) => {
                // prefix provided, we explicitely want one configuration
                let exp = { let __r = self.resolve_expr_must_never_fail(exp)?; self.int_forward(&__r)? };
                self.free_banks.selected_index = None;

                if output_kind == OutputKind::Cpr {
                    if !(0..=31).contains(&exp) {
                        return Err(Box::new(AssemblerError::AssemblingError {
                            msg: format!("Value {exp} is not compatible. [0-31]")
                        }));
                    }

                    if let Some(cpr) = &mut self.cpr {
                        cpr.select(exp as u8);

                        let page_info = self.active_page_info_mut();
                        page_info.logical_outputadr = 0;
                        page_info.logical_codeadr = 0;
                        self.ga_mmr = 0xC0;
                        self.output_address = 0
                    }
                }
                else {
                    // Snapshot output

                    let mmr = exp;
                    if !(0xC0..=0xC7).contains(&mmr) {
                        return Err(Box::new(AssemblerError::MMRError { value: mmr }));
                    }

                    let mmr = mmr as u8;
                    self.ga_mmr = mmr;

                    // ensure the page are present in the snapshot
                    if mmr >= 0xC4 && self.sna.pages_info.len() < 2 {
                        self.sna.resize(2.max(self.sna.pages_info.len()));
                    }

                    // we do not change the output address (there is no reason to do that)
                    // dbg!(self.output_address);
                }
            },
            None => {
                if output_kind == OutputKind::Cpr {
                    todo!("Need to implement this behavior")
                }

                // nothing provided, we write in a temporary area
                if self.pass.is_first_pass() {
                    self.free_banks.add_new_and_select();
                }
                else {
                    self.free_banks.select_next()?;
                }

                self.ga_mmr = 0xC0;
                self.output_address = 0;
                let page_info = self.active_page_info_mut();
                page_info.logical_outputadr = 0;
                page_info.logical_codeadr = 0;
            }
        }

        // BANK/BANKSET-like directives do not emit bytes, so refresh listing
        // coordinates after page/bank selection to avoid stale physical address.
        if self.listing_is_recording() {
            let code_adr = self.logical_code_address();
            let output_adr = self.logical_to_physical_address(self.logical_output_address());
            let trigger = self.listing_trigger().unwrap();

            trigger.replace_code_address(&(code_adr as i32).into());
            trigger.replace_physical_address(output_adr);
        }

        Ok(())
    }

    // total switch of page
    fn visit_pageset<E: ExprEvaluationExt>(&mut self, exp: &E) -> Result<(), Box<AssemblerError>> {
        if self.nested_rorg > 0 {
            return Err(Box::new(AssemblerError::NotAllowed));
        }

        let page = { let __r = self.resolve_expr_must_never_fail(exp)?; self.int_forward(&__r)? } as u8; // This value MUST be interpretable once executed

        //       eprintln!("Warning need to code sna memory extension if needed");
        self.select_page(page)?;
        Ok(())
    }

    fn select_page(&mut self, page: u8) -> Result<(), Box<AssemblerError>> {
        if self.nested_rorg > 0 {
            return Err(Box::new(AssemblerError::NotAllowed));
        }

        if
        // page < 0 ||
        page >= 8 {
            return Err(Box::new(AssemblerError::InvalidArgument {
                msg: format!("{page} is invalid. BANKSET only accept values from 0 to 7")
            }));
        }

        if page == 0 {
            self.ga_mmr = 0b1100_0000;
        }
        else {
            self.ga_mmr = 0b1100_0010 + ((page - 1) << 3);
        }

        let page = page as usize;
        let expected_nb_pages = self.sna.pages_info.len().max(page + 1);
        if expected_nb_pages > self.sna.pages_info.len() {
            self.sna.resize(expected_nb_pages);
        }
        debug_assert_eq!(self.sna.pages_info.len(), expected_nb_pages);

        self.output_address = self.logical_output_address();
        self.update_dollar();

        // Keep listing row addresses in sync for BANKSET directives.
        if self.listing_is_recording() {
            let code_adr = self.logical_code_address();
            let output_adr = self.logical_to_physical_address(self.logical_output_address());
            let trigger = self.listing_trigger().unwrap();

            trigger.replace_code_address(&(code_adr as i32).into());
            trigger.replace_physical_address(output_adr);
        }

        Ok(())
    }

    /// Remove the given variable from the table of symbols
    pub fn visit_undef<S: SourceString>(&mut self, label: S) -> Result<(), Box<AssemblerError>> {
        match self.symbols_mut().remove_symbol(label.as_str())? {
            Some(_) => Ok(()),
            None => {
                Err(Box::new(AssemblerError::UnknownSymbol {
                    symbol: label.as_str().into(),
                    closest: self
                        .symbols()
                        .closest_symbol(label.as_str(), SymbolFor::Number)?
                        .map(|s| s.into())
                }))
            },
        }
    }

    pub fn visit_protect<E: ExprEvaluationExt>(
        &mut self,
        start: &E,
        stop: &E
    ) -> Result<(), Box<AssemblerError>> {
        if self.pass.is_first_pass() {
            let start = { let __r = self.resolve_expr_must_never_fail(start)?; self.int_forward(&__r)? } as u16;
            let stop = { let __r = self.resolve_expr_must_never_fail(stop)?; self.int_forward(&__r)? } as u16;

            self.active_page_info_mut()
                .protected_areas
                .push(start..=stop);
        }

        Ok(())
    }

    #[inline]
    fn prepropress_string_formatted_expression(
        &mut self,
        info: &[FormattedExpr]
    ) -> Result<PreprocessedFormattedString, Box<AssemblerError>> {
        PreprocessedFormattedString::try_new(info, self)
    }

    /// Print the evaluation of the expression in the 2nd pass
    pub fn visit_print(&mut self, info: &[FormattedExpr], span: Option<&Z80Span>) {
        let print_or_error = match self.prepropress_string_formatted_expression(info) {
            Ok(msg) => either::Either::Left(msg),
            Err(error) => either::Either::Right(error)
        };

        self.active_page_info_mut().add_print_command(PrintCommand {
            prefix: None,
            span: span.cloned(),
            print_or_error
        })
    }

    pub fn visit_pause(&mut self, span: Option<&Z80Span>) {
        self.active_page_info_mut()
            .add_pause_command(span.cloned().into());
    }

    pub fn visit_fail(
        &mut self,
        info: Option<&[FormattedExpr]>
    ) -> Result<(), Box<AssemblerError>> {
        let repr = info
            .map(|info| self.prepropress_string_formatted_expression(info))
            .unwrap_or_else(|| Ok(Default::default()))?;
        Err(Box::new(AssemblerError::Fail {
            msg: repr.to_string()
        }))
    }

    pub fn visit_warning(
        &mut self,
        info: Option<&[FormattedExpr]>
    ) -> Result<(), Box<AssemblerError>> {
        let repr = info
            .map(|info| self.prepropress_string_formatted_expression(info))
            .unwrap_or_else(|| Ok(Default::default()))?;
        let warning = AssemblerWarning::AlreadyRenderedError(format!("Warning: {}", repr));
        self.add_warning(Box::new(warning));
        Ok(())
    }

    pub fn visit_output_file<E: ExprEvaluationExt>(
        &mut self,
        filename: &E
    ) -> Result<(), Box<AssemblerError>> {
        // Evaluate the filename expression
        let fname_result = self.resolve_expr_must_never_fail(filename)?;
        let fname = match fname_result {
            ExprResult::String(s) => s,
            ExprResult::Value(v) => v.to_string().into(),
            _ => {
                return Err(Box::new(AssemblerError::AssemblingError {
                    msg: "OUTPUT directive expects a string or value for the filename".into()
                }));
            }
        };

        // Store the output filename in the environment
        // This will be used later when saving the assembled output
        self.output_filename = Some(fname.to_string());
        Ok(())
    }

    // TODO better design the token to simplify this code and remove all ambigous cases
    pub fn visit_save<E: ExprEvaluationExt + Debug>(
        &mut self,
        amsdos_fname: &E,
        address: Option<&E>,
        size: Option<&E>,
        save_type: Option<&SaveType>,
        dsk_fname: Option<&E>,
        _side: Option<&E>
    ) -> Result<(), Box<AssemblerError>> {
        if cfg!(target_arch = "wasm32") {
            return Err(Box::new(AssemblerError::AssemblingError {
                msg: "SAVE directive is not allowed in a web-based assembling.".into()
            }));
        }

        let from = match address {
            Some(address) => {
                let address = { let __r = self.resolve_expr_must_never_fail(address)?; self.int_forward(&__r)? };
                if address < 0 {
                    return Err(Box::new(AssemblerError::AssemblingError {
                        msg: format!(
                            "Cannot SAVE {amsdos_fname} as the address ({address}) is invalid."
                        )
                    }));
                }
                Some(address)
            },
            None => None
        };

        let size = match size {
            Some(size) => {
                let size = { let __r = self.resolve_expr_must_never_fail(size)?; self.int_forward(&__r)? };
                if size < 0 {
                    return Err(Box::new(AssemblerError::AssemblingError {
                        msg: format!("Cannot SAVE {amsdos_fname} as the size ({size}) is invalid.")
                    }));
                }
                Some(size)
            },
            None => None
        };

        if let Some(from) = &from
            && let Some(size) = &size
            && 0x10000 - *from < *size
        {
            return Err(Box::new(AssemblerError::AssemblingError {
                msg: format!(
                    "Cannot SAVE {amsdos_fname} as the address+size (0x{:X}) is out of bounds.",
                    *from + *size
                )
            }));
        }

        let amsdos_fname = self.build_fname(amsdos_fname)?;
        let any_fname: AnyFileNameOwned = match dsk_fname {
            Some(dsk_fname) => {
                AnyFileNameOwned::new_in_image(self.build_fname(dsk_fname)?, amsdos_fname)
            },
            None => AnyFileNameOwned::from(amsdos_fname.as_str())
        };
        let any_fname = any_fname.as_any_filename();

        let (amsdos_fname, dsk_fname) = (any_fname.content_filename(), any_fname.image_filename());

        let amsdos_fname = Utf8PathBuf::from(amsdos_fname);
        let dsk_fname = dsk_fname.map(Utf8PathBuf::from);

        // Check filename validity
        if let Some(SaveType::Disc(disc)) = &save_type {
            let dsk_fname = dsk_fname.as_ref().unwrap();
            let lower_fname = dsk_fname.as_str().to_ascii_lowercase();
            match disc {
                DiscType::Dsk => {
                    if !(lower_fname.ends_with(".dsk") || lower_fname.ends_with(".edsk")) {
                        return Err(Box::new(AssemblerError::InvalidArgument {
                            msg: format!("{dsk_fname} has not a DSK compatible extension")
                        }));
                    }
                },
                DiscType::Hfe => {
                    if !lower_fname.ends_with(".hfe") {
                        return Err(Box::new(AssemblerError::InvalidArgument {
                            msg: format!("{dsk_fname} has not a HFE compatible extension")
                        }));
                    }

                    #[cfg(not(feature = "hfe"))]
                    Err(Box::new(AssemblerError::InvalidArgument {
                        msg: format!(
                            "{dsk_fname} cannot be saved. No HFE support is included with this version of basm"
                        )
                    }))?
                },
                DiscType::Auto => {
                    if !(lower_fname.ends_with(".dsk")
                        || lower_fname.ends_with(".edsk")
                        || lower_fname.ends_with(".hfe"))
                    {
                        return Err(Box::new(AssemblerError::InvalidArgument {
                            msg: format!("{dsk_fname} has not a DSK or HFE compatible extension")
                        }));
                    }

                    #[cfg(not(feature = "hfe"))]
                    if lower_fname.ends_with(".hfe") {
                        Err(Box::new(AssemblerError::InvalidArgument {
                            msg: format!(
                                "{dsk_fname} cannot be saved. No HFE support is included with this version of basm"
                            )
                        }))?
                    }
                }
            }
        }

        let file = match (save_type, dsk_fname, amsdos_fname) {
            (Some(save_type), Some(dsk_fname), amsdos_fname) => {
                let support = match save_type {
                    SaveType::Disc(_) => StorageSupport::Disc(dsk_fname),
                    SaveType::Tape => StorageSupport::Tape(dsk_fname),
                    _ => StorageSupport::Disc(dsk_fname)
                };
                let file_type = match save_type {
                    SaveType::AmsdosBas => FileType::AmsdosBas,
                    SaveType::AmsdosBin => FileType::AmsdosBin,
                    SaveType::Ascii => FileType::Ascii,
                    SaveType::Disc(_) | SaveType::Tape => FileType::Auto /* TODO handle vases based on file names */
                };
                SaveFile::new(support, (file_type, amsdos_fname))
            },
            (None, Some(dsk_fname), amsdos_fname) => {
                SaveFile::new(
                    StorageSupport::Disc(dsk_fname),
                    (FileType::Auto, amsdos_fname)
                )
            },
            (Some(save_type), None, amsdos_fname) => {
                let file_type = match save_type {
                    SaveType::AmsdosBas => FileType::AmsdosBas,
                    SaveType::AmsdosBin => FileType::AmsdosBin,
                    SaveType::Ascii => FileType::Ascii,
                    SaveType::Disc(_) | SaveType::Tape => {
                        unimplemented!("Handle the error message");
                    }
                };
                SaveFile::new(StorageSupport::Host, (file_type, amsdos_fname))
            },
            (None, None, amsdos_fname) => {
                SaveFile::new(StorageSupport::Host, (FileType::Ascii, amsdos_fname))
            },
        };

        //       eprintln!("MMR at save=0x{:x}", self.ga_mmr);
        let mmr = self.ga_mmr;
        let page_info = self.active_page_info_mut();
        page_info.add_save_command(SaveCommand::new(from, size, file, mmr));

        Ok(())
    }

    pub fn visit_charset(&mut self, format: &CharsetFormat) -> Result<(), Box<AssemblerError>> {
        let mut new_charset = CharsetEncoding::new();
        std::mem::swap(&mut new_charset, &mut self.charset_encoding);
        new_charset.update(&format.strengthen(&self.symbols), self)?;
        std::mem::swap(&mut new_charset, &mut self.charset_encoding); // XXX lost in case of error
        Ok(())
    }

    pub fn visit_snainit<E: ExprEvaluationExt + Debug>(
        &mut self,
        fname: &E
    ) -> Result<(), Box<AssemblerError>> {
        let fname = self.build_fname(fname)?;

        if !self.pass.is_first_pass() {
            return Ok(());
        }

        if self.byte_written {
            return Err(Box::new(AssemblerError::AssemblingError {
                msg: format!(
                    "Some bytes has already been produced; you cannot import the snapshot {fname}."
                )
            }));
        }
        // `inner://` names a snapshot compiled into cpclib itself. A program
        // that wants the firmware available - so `CALL &BB5A` does something -
        // must start from a booted machine, and requiring every project to
        // carry its own copy of one made the build depend on a binary blob
        // sitting beside the source.
        let loaded = match fname.as_str().strip_prefix("inner://") {
            Some(inner) => {
                Snapshot::from_embedded(inner).ok_or_else(|| {
                    AssemblerError::AssemblingError {
                        msg: format!(
                            "There is no snapshot embedded as \"inner://{inner}\". \
                             Available: {}.",
                            Snapshot::EMBEDDED
                                .iter()
                                .map(|name| format!("inner://{name}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                })?
            },
            None => Snapshot::load(fname)
        };

        self.sna.sna = loaded.map_err(|e| {
            AssemblerError::AssemblingError {
                msg: format!("Error while loading snapshot. {e}")
            }
        })?;

        self.sna.unwrap_memory_chunks();

        Ok(())
    }

    pub fn visit_snaset(
        &mut self,
        flag: &cpclib_sna::SnapshotFlag,
        value: &cpclib_sna::FlagValue
    ) -> Result<(), Box<AssemblerError>> {
        self.sna
            .set_value(*flag, value.as_u16().unwrap())
            .map_err(|e| e.into())
    }

    pub fn visit_incbin(&mut self, data: &[u8]) -> Result<(), Box<AssemblerError>> {
        self.output_bytes(data)
    }

    fn build_crunched_section_env(&mut self, span: Option<&Z80Span>) -> Self {
        let mut crunched_env = self.clone();
        crunched_env.crunched_section_state = CrunchedSectionState::new(span.cloned()).into();
        // codeadr stays the same
        crunched_env.active_page_info_mut().logical_outputadr = 0;
        crunched_env.active_page_info_mut().startadr = None; // reset the counter to obtain the bytes
        crunched_env.active_page_info_mut().maxadr = 0;
        crunched_env.active_page_info_mut().output_limit = 0xFFFF; // disable limit (to be redone in the area)
        crunched_env.active_page_info_mut().protected_areas.clear(); // remove protected areas
        crunched_env.output_address = 0;

        crunched_env
    }

    /// Handle a crunched section.
    /// bytes generated during previous pass or previous loop are provided TO NOT crunched them an additional time if they are similar
    pub fn visit_crunched_section<'tokens, T: Visited + ListingElement + MayHaveSpan + Sync>(
        &mut self,
        kind: &CrunchType,
        lst: &mut [ProcessedToken<'tokens, T>],
        previous_bytes_to_crunch: &mut Option<Vec<u8>>,
        previous_crunched_bytes: &mut Option<AssemblerCompressionResult>,
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement + Sync,
        ProcessedToken<'tokens, T>: FunctionBuilder,
        <<T as cpclib_tokens::ListingElement>::TestKind as cpclib_tokens::TestKindElement>::Expr:
            ExprEvaluationExt
    {
        // deactivated because there is no reason to do such thing
        // crunched section is disabled inside crunched section
        // if let Some(state) = & self.crunched_section_state {
        // let base = AssemblerError::AlreadyInCrunchedSection(state.crunched_section_start);
        // if let Some(span) = span {
        // return Err(AssemblerError::RelocatedError{error:base, span});
        // } else {
        // return Err(base);
        // }
        // }

        let could_display_warning_message = self.active_page_info().output_limit != 0xFFFF
            || !self.active_page_info().protected_areas.is_empty();

        // from here, the modifications to the memory will be forgotten afterwise.
        // for this reason everything is done in a cloned environnement
        // TODO to have a more stable memory function, see if we can keep some steps between the passes
        // TODO OR play all the passes directly now
        let mut crunched_env = self.build_crunched_section_env(span);

        if let Some(t) = self.listing_trigger() {
            t.enter_crunched_section()
        }

        // Try to assemble the crunched section
        let assembly_result = visit_processed_tokens(lst, &mut crunched_env);

        // Handle errors: some errors (like unknown symbols) can be deferred to next pass
        if let Err(e) = assembly_result {
            // Check if this is a recoverable error that might be resolved in a later pass
            let is_recoverable = match e.as_ref() {
                AssemblerError::UnknownSymbol { .. } => true,
                AssemblerError::RelocatedError { error, .. } => {
                    matches!(error.as_ref(), AssemblerError::UnknownSymbol { .. })
                },
                _ => false
            };

            // In first pass or if error is recoverable, defer it and request additional pass
            if crunched_env.pass.is_first_pass() || is_recoverable {
                // Mark that we need another pass to resolve this
                *self.request_additional_pass.write().unwrap() = true;
                *crunched_env.request_additional_pass.write().unwrap() = true;
                // Continue with empty bytes for now - will be computed in next pass
            }
            else {
                // Truly unrecoverable error - propagate it
                let e = AssemblerError::CrunchedSectionError { error: e };
                return Err(Box::new(match span {
                    Some(span) => {
                        AssemblerError::RelocatedError {
                            error: e.into(),
                            span: span.clone()
                        }
                    },
                    None => e
                }));
            }
        }

        if let Some(t) = self.listing_trigger() {
            t.leave_crunched_section()
        }

        // get the new data and crunch it if needed
        let new_bytes_to_crunch = crunched_env.produced_bytes();

        // indeed, there is no need to crunched again the same data
        let have_to_really_crunch = previous_bytes_to_crunch
            .as_ref()
            .map(|previous_bytes_to_crunch| {
                previous_bytes_to_crunch.as_slice() != new_bytes_to_crunch.as_slice()
            })
            .unwrap_or(true);
        let crunched_bytes = if have_to_really_crunch {
            if new_bytes_to_crunch.is_empty() {
                AssemblerCompressionResult::empty()
            }
            else {
                kind.compress(&new_bytes_to_crunch).map_err(|e| {
                    match span {
                        Some(span) => {
                            AssemblerError::RelocatedError {
                                error: Box::new(*e),
                                span: span.clone()
                            }
                        },
                        None => *e
                    }
                })?
            }
        }
        else {
            previous_crunched_bytes.as_ref().unwrap().clone() // safe because have_to_really_crunch is false only if previous_crunched_bytes is Some
        };

        // inject the crunched data
        self.visit_incbin(crunched_bytes.as_ref()).map_err(|e| {
            match span {
                Some(span) => {
                    AssemblerError::RelocatedError {
                        error: Box::new(*e),
                        span: span.clone()
                    }
                },
                None => *e
            }
        })?;

        // update the symbol table with the new symbols obtained in the crunched section
        std::mem::swap(self.symbols_mut(), crunched_env.symbols_mut());

        // appy the side effects
        crunched_bytes.apply_side_effects(self).map_err(|e| {
            match span {
                Some(span) => {
                    AssemblerError::RelocatedError {
                        error: Box::new(*e),
                        span: span.clone()
                    }
                },
                None => *e
            }
        })?;

        let can_skip_next_passes = *self.can_skip_next_passes.read().unwrap().deref()
            & *crunched_env.can_skip_next_passes.read().unwrap(); // report missing symbols from the crunched area to the current area
        let request_additional_pass = *self.request_additional_pass.read().unwrap().deref()
            | *crunched_env.request_additional_pass.read().unwrap();
        *self.can_skip_next_passes.write().unwrap() = can_skip_next_passes;
        *self.request_additional_pass.write().unwrap() = request_additional_pass;

        self.macro_seed = crunched_env.macro_seed;

        // TODO display ONLY if:
        // - no LIMIT/PROTECT has been used in the crunched area
        // - a possible forbidden write has been done (maybe too complex to implement)
        if could_display_warning_message {
            self.add_warning(Box::new(
                AssemblerWarning::AssemblingError{
                    msg: "Memory protection systems are disabled in crunched section. If you want to keep them, explicitely use LIMIT or PROTECT directives in the crunched section.".into()
                }
            ));
        }

        Ok(())
    }
}

impl Env {
    fn assemble_nop<E: ExprEvaluationExt>(
        &mut self,
        kind: Mnemonic,
        count: Option<&E>
    ) -> Result<Bytes, Box<AssemblerError>> {
        let count = match count {
            Some(count) => { let __r = self.resolve_expr_must_never_fail(count)?; self.int_forward(&__r)? },
            None => 1
        };
        let mut bytes = Bytes::new();
        for _i in 0..count {
            match kind {
                Mnemonic::Nop => {
                    bytes.push(0);
                },
                Mnemonic::Nop2 => {
                    bytes.push(0xED);
                    bytes.push(0xFF);
                },
                _ => unreachable!()
            }
        }
        Ok(bytes)
    }
}
/// Visit the tokens during several passes without providing a specific symbol table.
// pub fn visit_tokens_all_passes<
// 'token,
// T: 'token + Visited + ToSimpleToken + Debug + Sync + ListingElement + MayHaveSpan
// >(
// tokens: &'token [T]
// ) -> Result<Env, Box<AssemblerError>>
// where
// <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
// <<T as cpclib_tokens::ListingElement>::TestKind as cpclib_tokens::TestKindElement>::Expr:
// ExprEvaluationExt,
// ProcessedToken<'token, T>: FunctionBuilder
// {
// let options = EnvOptions::default();
// visit_tokens_all_passes_with_options(tokens, options).map(|r| r.1) // TODO really return both
// }
//

impl Env {
    pub fn new(options: EnvOptions) -> Self {
        let mut env = Self {
            lookup_directory_stack: Vec::with_capacity(3),
            pass: AssemblingPass::Uninitialized,
            options: EnvOptions::default(),
            stable_counters: StableTickerCounters::default(),
            ga_mmr: 0xC0, // standard memory configuration

            macro_seed: 0,
            active_frames: Vec::new(),
            macro_expansion_cache: Arc::new(RwLock::new(HashMap::new())),
            symbol_definition_chains: HashMap::new(),
            charset_encoding: CharsetEncoding::new(),
            sna: SnaAssembler::default(),
            sna_version: cpclib_sna::SnapshotVersion::V3,

            cpr: None,

            symbols: SymbolsTable::default(),
            run_options: None,
            byte_written: false,
            output_trigger: None,
            expression_depth: 0,
            symbols_output: Default::default(),

            crunched_section_state: None,

            warnings: Vec::new(),
            warning_push_count: 0,
            warnings_cleaned_up_to: 0,
            nested_rorg: 0,

            sections: HashMap::<String, Arc<RwLock<Section>>>::default(),
            current_section: None,
            output_address: 0,
            active_page_index_cache: std::sync::Mutex::new(None),
            free_banks: DecoratedPages::default(),

            real_nb_passes: 0,
            saved_files: None,
            can_skip_next_passes: true.into(),
            request_additional_pass: false.into(),

            if_token_adr_to_used_decision: HashMap::default(),
            if_token_adr_to_unused_decision: HashMap::default(),
            address_trace: HashMap::default(),
            address_trace_by_file: HashMap::default(),
            requested_additional_pass: false,

            functions: Default::default(),
            return_value: None,

            current_pass_discarded_errors: HashSet::default(),
            previous_pass_discarded_errors: HashSet::default(),

            included_paths: HashSet::default(),

            extra_print_from_function: Vec::new().into(),
            extra_failed_assert_from_function: Vec::new().into(),
            map_counter: 0,

            repeat_start: 1.into(),
            repeat_step: 1.into(),

            output_filename: None,

            assembling_control_current_output_commands: Vec::new()
        };

        env.options = options;

        // prefill the snapshot representation with something else than the default
        if let Some(sna) = env.options.assemble_options().snapshot_model() {
            env.sna.sna = sna.clone();
            env.sna_version = env.sna.version();
        }

        env.symbols = env
            .options()
            .symbols()
            .clone()
            .with_case_sensitive(env.options().case_sensitive());
        env.retrieve_options_symbols();

        if let Some(builder) = &env.options().assemble_options().output_builder {
            env.output_trigger = ListingOutputTrigger {
                token: None,
                bytes: Vec::new(),
                symbols: None,
                builder: builder.clone(),
                start: 0,
                physical_address: MemoryPhysicalAddress::new(0, 0).into()
            }
            .into();
        }
        env
    }

    pub fn pass(&self) -> &AssemblingPass {
        &self.pass
    }
}

// Functions related
impl Env {
    pub fn visit_return<E: ExprEvaluationExt>(&mut self, e: &E) -> Result<(), Box<AssemblerError>> {
        if self.return_value.is_some() {
            return dbg!(Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: "Return value is alread set up".into()
            })));
        }
        self.return_value = Some(self.resolve_expr_must_never_fail(e)?);
        Ok(())
    }

    pub fn user_defined_function(&self, name: &str) -> Result<&Function, Box<AssemblerError>> {
        match self.functions.get(name) {
            Some(f) => Ok(f),
            None => Err(Box::new(AssemblerError::FunctionUnknown(name.to_owned())))
        }
    }

    pub fn any_function<'res>(
        &'res self,
        name: &'res str
    ) -> Result<&'res Function, Box<AssemblerError>> {
        match HardCodedFunction::by_name(name) {
            Some(f) => Ok(f),
            None => self.user_defined_function(name)
        }
    }

    pub fn eval_any_function<'res, E:AsRef<ExprResult>+Clone>(
        &'res mut self,
        name: &'res str,
        params: &[E]
    ) -> Result<ExprResult, Box<AssemblerError>> {
        let f = match HardCodedFunction::by_name(name) {
            Some(f) => Ok(f),
            None => self.user_defined_function(name)
        }?;

        let f: *const Function = f as *const _; // XXX remove the link with environment
        unsafe { (*f).eval(self, params) }
    }
}

/// Visit the tokens during several passes by providing a specific symbol table.
/// Warning Listing output is only possible for LocatedToken
pub fn visit_tokens_all_passes_with_options<'token, T>(
    tokens: &'token [T],
    options: EnvOptions
) -> Result<
    (Vec<ProcessedToken<'token, T>>, Env),
    (
        Option<Vec<ProcessedToken<'token, T>>>,
        Env,
        Box<AssemblerError>
    )
>
where
    T: Visited + ToSimpleToken + Debug + Sync + ListingElement + MayHaveSpan,
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
        ExprEvaluationExt + ExprElement + Sync,
    ProcessedToken<'token, T>: FunctionBuilder,
    <T as cpclib_tokens::ListingElement>::Expr: Sync
{
    let mut env = Env::new(options);

    let mut tokens = match processed_token::build_processed_tokens_list(
        tokens,
        std::sync::Arc::new(std::sync::RwLock::new(&mut env))
    ) {
        Ok(tokens) => tokens,
        Err(e) => return Err((None, env, e))
    };
    loop {
        let res = env.start_new_pass();
        if let Err(e) = res {
            return Err((Some(tokens), env, e));
        }

        // println!("[pass] {:?}", env.pass);

        if env.pass.is_finished() {
            break;
        }

        let res = processed_token::visit_processed_tokens(&mut tokens, &mut env);
        if let Err(e) = res {
            return Err((Some(tokens), env, e));
        }
    }

    env.cleanup_warnings();

    Ok((tokens, env))
}

/// Visit the tokens during a single pass. Is deprecated in favor to the mulitpass version
#[deprecated(note = "use visit_tokens_one_pass")]
pub fn visit_tokens<T: Visited>(
    tokens: &[T],
    o: Arc<dyn EnvEventObserver>
) -> Result<Env, Box<AssemblerError>> {
    visit_tokens_one_pass(tokens, o)
}

/// Assemble the tokens doing one pass only (so symbols are not properly treated)
pub fn visit_tokens_one_pass<T: Visited>(
    tokens: &[T],
    o: Arc<dyn EnvEventObserver>
) -> Result<Env, Box<AssemblerError>> {
    let mut opt = EnvOptions::default();
    opt.observer = o;
    let mut env = Env::new(opt);

    for token in tokens.iter() {
        token.visited(&mut env)?;
    }

    Ok(env)
}

macro_rules! visit_token_impl {
    ($token:ident, $env:ident, $span:ident, $cls:tt) => {{
        $env.update_dollar();
        match &$token {
            $cls::Abyte(d, l) => $env.visit_abyte(d, l.as_ref()),
            $cls::Align(boundary, fill) => $env.visit_align(boundary, fill.as_ref()),
            $cls::Assert(exp, txt) => {
                $env.visit_assert(exp, txt.as_ref(), $span)?;
                Ok(())
            },
            $cls::AssemblerControl(cmd) => $env.visit_assembler_control(cmd, $span),
            $cls::Assign { label, expr, op } => $env.visit_assign(label, expr, op.as_ref()),

            $cls::Basic(variables, hidden_lines, code) => {
                $env.visit_basic(variables.as_deref(), hidden_lines.as_deref(), code)
            }, // TODO move in the processed tokens stuff
            $cls::Bank(exp) => $env.visit_page_or_bank(exp.as_ref()),
            $cls::Bankset(v) => $env.visit_pageset(v),
            $cls::Breakpoint {
                address,
                r#type,
                access,
                run,
                mask,
                size,
                value,
                value_mask,
                condition,
                name,
                step
            } => {
                $env.visit_breakpoint(
                    address.as_ref(),
                    r#type.as_ref(),
                    access.as_ref(),
                    run.as_ref(),
                    mask.as_ref(),
                    size.as_ref(),
                    value.as_ref(),
                    value_mask.as_ref(),
                    condition.as_ref(),
                    name.as_ref(),
                    step.as_ref(),
                    $span
                )
            },
            $cls::BuildCpr => $env.visit_buildcpr(),
            $cls::BuildSna(v) => $env.visit_buildsna(v.as_ref()),

            $cls::Charset(format) => $env.visit_charset(format),

            $cls::Comment(_) => Ok(()), // Nothing to do for a comment

            $cls::Defb(l) => $env.visit_db_or_dw_or_str(DbLikeKind::Defb, l.as_ref(), 0.into()),
            $cls::Defw(l) => $env.visit_db_or_dw_or_str(DbLikeKind::Defw, l.as_ref(), 0.into()),
            $cls::Defs(l) => $env.visit_defs(l),

            $cls::End => $env.visit_end(),
            $cls::Enum {
                prefix,
                start,
                step,
                fields
            } => {
                $env.visit_enum(
                    prefix.as_ref(),
                    start.as_ref(),
                    step.as_ref(),
                    fields.as_slice()
                )
            },
            $cls::Export(labels) => $env.visit_export(labels.as_slice()),
            $cls::Equ { label, expr } => $env.visit_equ(&label, expr),
            $cls::Even => $env.visit_even(),

            $cls::Fail(exp) => $env.visit_fail(exp.as_ref().map(|v| v.as_slice())),
            $cls::Warning(exp) => $env.visit_warning(exp.as_ref().map(|v| v.as_slice())),
            $cls::Field { label, expr, .. } => $env.visit_field(label, expr),

            $cls::Label(label) => $env.visit_label(label),
            $cls::Limit(exp) => $env.visit_limit(exp),
            $cls::List => {
                $env.output_trigger.as_mut().map(|l| {
                    l.on();
                });
                Ok(())
            },

            $cls::Map(exp) => $env.visit_map(exp),
            $cls::MultiPush(regs) => $env.visit_multi_pushes(regs),
            $cls::MultiPop(regs) => $env.visit_multi_pops(regs),

            $cls::Next {
                label,
                source,
                expr
            } => $env.visit_next_and_co(label, source, expr.as_ref(), false),
            $cls::NoExport(labels) => $env.visit_noexport(labels.as_slice()),
            $cls::NoList => {
                $env.output_trigger.as_mut().map(|l| {
                    l.off();
                });
                Ok(())
            },

            $cls::Org { val1, val2 } => $env.visit_org(val1, val2.as_ref()),
            $cls::OutputFile(filename) => $env.visit_output_file(filename),
            $cls::OpCode(mnemonic, arg1, arg2, arg3) => {
                $env.visit_opcode(*mnemonic, &arg1, &arg2, &arg3)?;
                // Compute duration only if it is necessary
                if !$env.stable_counters.is_empty() {
                    let duration = $token.to_token().estimated_duration()?;
                    $env.stable_counters.update_counters(duration);
                }
                Ok(())
            },

            $cls::Pause => {
                $env.visit_pause($span);
                Ok(())
            },
            $cls::Protect(start, end) => $env.visit_protect(start, end),
            $cls::Print(exp) => {
                $env.visit_print(exp.as_ref(), $span);
                Ok(())
            },

            $cls::Range(name, start, stop) => $env.visit_range(name, start, stop),
            $cls::Return(exp) => $env.visit_return(exp),

            $cls::Rorg(_exp, _code) => panic!("Is delegated to ProcessedToken"),
            $cls::Run(address, gate_array) => $env.visit_run(address, gate_array.as_ref()),

            $cls::SetN {
                label,
                source,
                expr
            } => $env.visit_next_and_co(label, source, expr.as_ref(), true),
            $cls::Save {
                filename,
                address,
                size,
                save_type,
                dsk_filename,
                side
            } => {
                $env.visit_save(
                    filename,
                    address.as_ref(),
                    size.as_ref(),
                    save_type.as_ref(),
                    dsk_filename.as_ref(),
                    side.as_ref()
                )
            },
            $cls::Section(name) => $env.visit_section(name),
            $cls::Skip(amount) => $env.visit_skip(amount),
            $cls::SnaInit(fname) => $env.visit_snainit(fname),
            $cls::SnaSet(flag, value) => $env.visit_snaset(flag, value),
            $cls::StableTicker(ticker) => $env.visit_stableticker(ticker),
            $cls::StartingIndex { start, step } => {
                $env.visit_starting_index(start.as_ref(), step.as_ref())
            },
            $cls::Str(l) => $env.visit_db_or_dw_or_str(DbLikeKind::Str, l.as_ref(), 0.into()),
            $cls::Struct(name, content) => {
                $env.visit_struct_definition(name, content.as_slice(), $span)
            },

            $cls::Undef(label) => $env.visit_undef(label),
            $cls::WaitNops(count) => $env.visit_waitnops(count),

            $cls::Include(..)
            | $cls::Incbin { .. }
            | $cls::If(..)
            | $cls::Repeat(..)
            | $cls::Macro { .. } => panic!("Should be handled by ProcessedToken"),

            _ => {
                Err(Box::new(AssemblerError::BugInAssembler {
                    file: file!(),
                    line: line!(),
                    msg: format!("Directive not handled: {:?}", $token)
                }))
            },
        }
    }};
}

impl Env {
    /// Apply the effect of the localized token. Most of the action is delegated to visit_token.
    /// The difference with the standard token is the ability to embed listing
    pub fn visit_located_token(
        &mut self,
        outer_token: &LocatedToken
    ) -> Result<(), Box<AssemblerError>> {
        let nb_warnings = self.warnings.len();

        // cheat on the lifetime of tokens
        let outer_token = unsafe { (outer_token as *const LocatedToken).as_ref().unwrap() };

        // Listing trigger is handled in ProcessedToken::visited to preserve
        // deferred/non-deferred token ordering.

        let span = Some(outer_token.span());

        if self.options().assemble_options().record_token_addresses() {
            let span = outer_token.span();
            let address = self.logical_output_address();
            self.address_trace.insert(span.identity(), address);
            // Recorded a second way so a *different* parse of the same file
            // can look it up - see `address_trace_by_file`. Canonicalised on
            // the way in, because how a file was reached (`include "x.asm"`
            // from one directory, an absolute path from another) must not
            // change its identity.
            if let Some(path) = canonical_source_path(span.filename()) {
                self.address_trace_by_file
                    .insert((path, span.offset_from_start()), address);
            }
        }

        // handle warning if any - in practice, `build_processed_token`
        // (processed_token.rs) already intercepts every `is_warning()`
        // token at classification time, unwrapping it via
        // `token.warning_token()` before it can ever reach this generic
        // dispatch with `is_warning()` still true, so this is not known to
        // be reachable through the normal multi-pass pipeline (the real,
        // reachable path is `ProcessedTokenState::Warning` in
        // processed_token.rs). Kept as a defensive fallback, in the same
        // plain-and-unlocated shape `checked_byte`/`checked_word` use
        // elsewhere in this file - the loop just below already relocates
        // any warning added since `nb_warnings`, so no eager span capture
        // is needed here either.
        if outer_token.is_warning() {
            self.add_warning(Box::new(AssemblerWarning::AssemblingError {
                msg: outer_token.warning_message().into()
            }));
        }

        // get the token to handle (after remobing handling wrapping)
        let token = outer_token.deref();

        visit_token_impl!(token, self, span, LocatedTokenInner)
            .map_err(|e| e.locate(span.unwrap().clone()))?;

        let span = outer_token.span();

        // Patch the warnings to inject them a location
        let nb_additional_warnings = self.warnings.len() - nb_warnings;
        for i in 0..nb_additional_warnings {
            let warning = &mut self.warnings[i + nb_warnings];
            **warning = warning.clone().locate_warning(span.clone());

            // TODO check why it has been done this way
            //      maybe source code is not retrained and there are random crashes ?
            //     anyway I comment it now because it breaks warning merge
            //
            //    *warning = AssemblerError::AssemblingError {
            //        msg: (*warning).to_string()
            //    }
        }

        self.move_delayed_commands_of_functions();

        Ok(())
    }

    /// Apply the effect of the token
    fn visit_token(&mut self, token: &Token) -> Result<(), Box<AssemblerError>> {
        let span = None;
        let _res = visit_token_impl!(token, self, span, Token);

        self.move_delayed_commands_of_functions();
        Ok(())
    }

    fn visit_defs<E: ExprEvaluationExt>(
        &mut self,
        l: &[(E, Option<E>)]
    ) -> Result<(), Box<AssemblerError>> {
        for (count, val) in l.iter() {
            let bytes = self.assemble_defs_item(count, val.as_ref())?;
            // Update stable ticker counters when active
            if !self.stable_counters.is_empty() {
                if bytes.iter().all(|&b| b == 0) {
                    self.stable_counters.update_counters(bytes.len());
                }
                else {
                    return Err(Box::new(AssemblerError::AssemblingError {
                        msg: "TICKER cannot compute timing for DEFS with non-zero fill value"
                            .to_string()
                    }));
                }
            }
            self.output_bytes(&bytes)?;
        }
        Ok(())
    }

    fn visit_end(&mut self) -> Result<(), Box<AssemblerError>> {
        if self.pass.is_first_pass() {
            *self.can_skip_next_passes.write().unwrap() = false;
        }
        Ok(())
    }
}

impl Env {
    pub fn visit_while<'token, E, T>(
        &mut self,
        cond: &E,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement + Sync,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        while self.resolve_expr_must_never_fail(cond)?.bool()? {
            // generate the bytes
            visit_processed_tokens(code, self).map_err(|e| {
                AssemblerError::WhileIssue {
                    error: Box::new(*e),
                    span: span.cloned()
                }
            })?;
        }

        Ok(())
    }

    /// Handle the iterate repetition directive
    /// Values is either a list of values or a Expression that represents a list
    pub fn visit_iterate<
        'token,
        E: ExprEvaluationExt + Sync,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync
    >(
        &mut self,
        counter_name: &str,
        values: either::Either<&Vec<E>, &E>,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        let counter_name = format!("{{{counter_name}}}");
        let counter_name = counter_name.as_str();
        if self.symbols().contains_symbol(counter_name)? {
            return Err(Box::new(AssemblerError::RepeatIssue {
                error: Box::new(AssemblerError::ExpressionError(ExpressionError::OwnError(
                    Box::new(AssemblerError::AssemblingError {
                        msg: format!("Counter {counter_name} already exists")
                    })
                ))),
                span: span.cloned(),
                repetition: 0
            }));
        }

        // Get the values (all args or list explosion)
        // BUG: iteration over values make the expressions progressively evaluated, while iteration over a list make its expressions evaluated at first loop
        match values {
            either::Either::Left(values) => {
                for (i, value) in values.iter().enumerate() {
                    let counter_value = self.resolve_expr_must_never_fail(value).map_err(|e| {
                        AssemblerError::RepeatIssue {
                            error: e,
                            span: span.cloned(),
                            repetition: i as _
                        }
                    })?;
                    self.inner_visit_repeat(
                        Some(counter_name),
                        Some(counter_value),
                        i as _,
                        code,
                        span
                    )?;
                }
            },
            either::Either::Right(values) => {
                match self.resolve_expr_must_never_fail(values)? {
                    ExprResult::List(values) => {
                        for (i, counter_value) in
                            std::sync::Arc::unwrap_or_clone(values).into_iter().enumerate()
                        {
                            self.inner_visit_repeat(
                                Some(counter_name),
                                Some(counter_value),
                                i as _,
                                code,
                                span
                            )?;
                        }
                    },

                    _ => {
                        let kind = values.r#type();
                        return Err(Box::new(AssemblerError::AssemblingError {
                            msg: format!(
                                "REPEAT issue: {values} is not a list or a matrix but a {kind}"
                            )
                        }));
                    }
                }
            },
        }

        // Apply the iteration

        self.warn_if_counter_unused(counter_name, span, "ITERATE loop");

        // TODO restore a previous value if any
        self.symbols_mut().remove_symbol(counter_name)?;

        Ok(())
    }

    pub fn visit_rorg<'token, T, E>(
        &mut self,
        address: &E,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        E: ExprEvaluationExt + Sync,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        // Get the next code address
        let result = self
            .resolve_expr_must_never_fail(address)
            .map_err(|error| {
                match span {
                    Some(span) => {
                        Box::new(AssemblerError::RelocatedError {
                            error: Box::new(*error),
                            span: span.clone()
                        })
                    },
                    None => error
                }
            })?;
        let address = self.int_forward(&result)?;

        // do not change the output address
        {
            let page_info = self.active_page_info_mut();
            page_info.logical_codeadr = address as _;
        }

        self.update_dollar();
        let value = self.active_page_info_mut().logical_codeadr;

        if let Some(o) = self.listing_trigger() {
            o.replace_code_address(&value.into())
        }

        // execute the listing
        self.nested_rorg += 1; // used to disable page functionalities
        visit_processed_tokens(code, self)?;
        self.nested_rorg -= 1;

        // restore the appropriate  address
        let page_info = self.active_page_info_mut();
        page_info.logical_codeadr = page_info.logical_outputadr;

        Ok(())
    }

    pub fn visit_confined<'token, E: ExprEvaluationExt, T>(
        &mut self,
        lst: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        E: ExprEvaluationExt + Sync,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        // Visit the confined section a first time
        // TODO: refactor this code with visit_crunched_section
        let mut confined_env = self.clone();
        confined_env.active_page_info_mut().logical_outputadr = 0;
        confined_env.active_page_info_mut().startadr = None; // reset the counter to obtain the bytes
        confined_env.active_page_info_mut().maxadr = 0;
        confined_env.active_page_info_mut().output_limit = 0xFFFF; // disable limit (to be redone in the area)
        confined_env.active_page_info_mut().protected_areas.clear(); // remove protected areas
        confined_env.output_address = 0;
        // TODO: forbid a subset of instructions to ensure it works properly
        visit_processed_tokens(lst, &mut confined_env).map_err(|e| {
            match span {
                Some(span) => e.locate(span.clone()),
                None => *e
            }
        })?;

        // compute its size
        let bytes = confined_env.produced_bytes();
        let bytes_len = bytes.len() as u16;

        if bytes_len > 256 {
            let e = AssemblerError::AssemblingError {
                msg: format!(
                    "CONFINED error: content uses {} bytes instead of a maximum of 256.",
                    bytes.len()
                )
            };
            match span {
                Some(span) => return Err(Box::new(e.locate(span.clone()))),
                None => return Err(Box::new(e))
            }
        }

        // Add the delta if needed and recompute the confined section a second time to properly setup the side effects
        if ((self.logical_code_address().wrapping_add(bytes_len)) & 0xFF00)
            != self.logical_code_address() & 0xFF00
        {
            while (self.logical_code_address() & 0x00FF) != 0x0000 {
                self.output_byte(0)?;
                self.update_dollar();
            }
        }

        visit_processed_tokens(lst, self)
    }

    /// Handle the for directive
    pub fn visit_for<'token, E, T>(
        &mut self,
        label: &str,
        start: &E,
        stop: &E,
        step: Option<&E>,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        E: ExprEvaluationExt + Sync,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        let counter_name = format!("{{{label}}}");
        if self.symbols().contains_symbol(&counter_name)? {
            return Err(Box::new(AssemblerError::ForIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: format!("Counter {} already exists", counter_name)
                    }
                )))
                .into(),
                span: span.cloned()
            }));
        }

        let mut counter_value = self.resolve_expr_must_never_fail(start)?;
        let stop = self.resolve_expr_must_never_fail(stop)?;
        let step = match step {
            Some(step) => self.resolve_expr_must_never_fail(step)?,
            None => 1i32.into()
        };

        let zero = ExprResult::from(0i32);

        if step == zero {
            return Err(Box::new(AssemblerError::ForIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: "Infinite loop".to_string()
                    }
                )))
                .into(),
                span: span.cloned()
            }));
        }

        if step < zero {
            return Err(Box::new(AssemblerError::ForIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: "Negative step is not yet handled".to_string()
                    }
                )))
                .into(),
                span: span.cloned()
            }));
        }

        let mut i = 1;
        while counter_value <= stop {
            self.inner_visit_repeat(
                Some(counter_name.as_str()),
                Some(counter_value.clone()),
                i as _,
                code,
                span
            )?;
            counter_value = (counter_value + &step)?;
            i += 1;
        }

        self.warn_if_counter_unused(&counter_name, span, "FOR loop");
        self.symbols_mut().remove_symbol(counter_name)?;

        Ok(())
    }

    /// Handle the standard repetition directive
    pub fn visit_repeat_until<'token, E, T>(
        &mut self,
        cond: &E,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        E: ExprEvaluationExt + Sync,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        let mut i = 0;
        loop {
            i += 1;
            self.inner_visit_repeat(None, None, i as _, code, span)?;
            let res = self.resolve_expr_must_never_fail(cond)?;
            if res.bool()? {
                break;
            }
        }

        Ok(())
    }

    pub fn visit_starting_index<E>(
        &mut self,
        start: Option<&E>,
        step: Option<&E>
    ) -> Result<(), Box<AssemblerError>>
    where
        E: ExprEvaluationExt
    {
        let start_value = start
            .map(|start| self.resolve_expr_must_never_fail(start))
            .unwrap_or(Ok(ExprResult::from(1)))?;
        let step_value = step
            .map(|step| self.resolve_expr_must_never_fail(step))
            .unwrap_or(Ok(ExprResult::from(1)))?;

        self.repeat_start = start_value;
        self.repeat_step = step_value;
        Ok(())
    }

    /// Handle the repetition of single opcode
    pub fn visit_repeat_token<'token, T, E>(
        &mut self,
        opcode: &mut ProcessedToken<'token, T>,
        count: &E
    ) -> Result<(), Box<AssemblerError>>
    where
        E: ExprEvaluationExt,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement + Sync,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        let repeat = self.resolve_expr_must_never_fail(count)?;
        let repeat = self.int_forward(&repeat)?;
        for _ in 0..repeat {
            opcode.visited(self)?;
        }
        Ok(())
    }

    /// Push a warning if `bracketed_counter_name` (e.g. `"{i}"`) was never
    /// referenced during the loop body that just finished assembling -
    /// shared by REPEAT/ITERATE/FOR, called right before each removes its
    /// own counter symbol. Mirrors `cpclib_asm::unused_bindings`'s own
    /// static check (used by the LSP) for these same three construct
    /// kinds - see that module's doc comment for why MACRO parameters
    /// can't use this real-time approach at all (pure text substitution,
    /// never touches the symbol table).
    fn warn_if_counter_unused(
        &mut self,
        bracketed_counter_name: &str,
        span: Option<&Z80Span>,
        construct: &str
    ) {
        if self.symbols().is_used(bracketed_counter_name) {
            return;
        }
        let name = bracketed_counter_name
            .trim_start_matches('{')
            .trim_end_matches('}');
        let msg = format!("'{name}' is never used in this {construct}'s body");
        match span {
            Some(span) => {
                let (line, column) = span.relative_line_and_column();
                let len = {
                    let text: &str = span.as_ref();
                    text.lines().next().map(str::len).unwrap_or(0)
                };
                self.add_warning(Box::new(
                    AssemblerError::AlreadyRenderedWarningWithLocation {
                        msg,
                        line: line as u32,
                        column: column as u32,
                        len: len as u32
                    }
                ));
            },
            None => {
                self.add_warning(Box::new(AssemblerError::AlreadyRenderedError(msg)));
            }
        }
    }

    /// Handle the standard repetition directive
    pub fn visit_repeat<'token, T, E>(
        &mut self,
        count: &E,
        code: &mut [ProcessedToken<'token, T>],
        counter_name: Option<&str>,
        counter_start: Option<&E>,
        counter_step: Option<&E>,
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        E: ExprEvaluationExt + Sync,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        // get the number of loops
        let count = { let __r = self.resolve_expr_must_never_fail(count)?; self.int_forward(&__r)? };

        // get the counter name of any
        let counter_name = counter_name
            .as_ref()
            .map(|counter| format!("{{{counter}}}"));
        let counter_name = counter_name.as_deref();
        if let Some(counter_name) = counter_name
            && self.symbols().contains_symbol(counter_name)?
        {
            return Err(Box::new(AssemblerError::RepeatIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: format!("Counter {counter_name} already exists")
                    }
                )))
                .into(),
                span: span.cloned(),
                repetition: 0
            }));
        }

        // get the first value
        let mut counter_value = counter_start
            .map(|start| self.resolve_expr_must_never_fail(start))
            .unwrap_or(Ok(self.repeat_start.clone()))?; // TODO use the one setup by STARTINGINDEX
        let step_value = counter_step
            .map(|step| self.resolve_expr_must_never_fail(step))
            .unwrap_or(Ok(self.repeat_step.clone()))?; // TODO use the one steup by STARTINGINDEX

        for i in 0..count {
            self.inner_visit_repeat(
                counter_name,
                Some(counter_value.clone()),
                i as _,
                code,
                span
            )?;
            // handle the counter update
            counter_value += step_value.clone();
        }

        if let Some(counter_name) = counter_name {
            self.warn_if_counter_unused(counter_name, span, "REPEAT loop");
            self.symbols_mut().remove_symbol(counter_name)?;
        }
        Ok(())
    }

    /// Handle the code generation for all the repetition variants
    fn inner_visit_repeat<'token, T: ListingElement + Visited + MayHaveSpan + Sync>(
        &mut self,
        counter_name: Option<&str>,
        counter_value: Option<ExprResult>,
        iteration: i32,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), Box<AssemblerError>>
    where
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + ExprElement + Sync,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        // handle symbols unicity
        {
            self.macro_seed += 1;
            let seed = self.macro_seed;
            self.symbols_mut().push_seed(seed);
        }

        // handle counter value update
        if let Some(counter_name) = counter_name {
            self.symbols_mut()
                .set_symbol_to_value(counter_name, counter_value.clone().unwrap())?;

            let depth = self.symbols().counter_depth() + 1;

            if self.pass.is_listing_pass()
                && let Some(trigger) = self.listing_trigger()
            {
                trigger.repeat_iteration(counter_name, counter_value.as_ref(), depth)
            }
        }
        else {
            let depth = self.symbols().counter_depth() + 1;
            if self.pass.is_listing_pass()
                && let Some(trigger) = self.listing_trigger()
            {
                trigger.repeat_iteration("<new iteration>", counter_value.as_ref(), depth)
            }
        }

        if let Some(counter_value) = &counter_value {
            self.symbols_mut().push_counter_value(counter_value.clone());
        }

        // generate the bytes
        visit_processed_tokens(code, self).map_err(|e| {
            Box::new(AssemblerError::RepeatIssue {
                error: e,
                span: span.cloned(),
                repetition: iteration as _
            })
        })?;

        // handle the end of visibility of unique labels
        self.symbols_mut().pop_seed();
        if let Some(_counter_value) = &counter_value {
            self.symbols_mut().pop_counter_value();
        }

        Ok(())
    }

    /// Generate a string that is helpfull for assertion understanding (i.e. show the operation and evaluate the rest)
    /// Crash if expression cannot be computed
    fn to_assert_string<E>(&mut self, exp: &E) -> String
    where
        E: ExprEvaluationExt + ExprElement,
        <E as ExprElement>::Expr: ExprEvaluationExt
    {
        let mut format = |oper, left, right| {
            format!(
                "0x{:x} {} 0x{:x}",
                self.resolve_expr_must_never_fail(left).unwrap(),
                oper,
                self.resolve_expr_must_never_fail(right).unwrap(),
            )
        };

        if exp.is_binary_operation() {
            let code = match exp.binary_operation() {
                BinaryOperation::Equal => Some("=="),
                BinaryOperation::GreaterOrEqual => Some(">="),
                BinaryOperation::StrictlyGreater => Some(">"),
                BinaryOperation::StrictlyLower => Some("<"),
                BinaryOperation::LowerOrEqual => Some("<="),
                _ => None
            };

            match code {
                Some(code) => {
                    let left = exp.arg1();
                    let right = exp.arg2();
                    format(code, left, right)
                },

                None => {
                    let d = self.resolve_expr_must_never_fail(exp).unwrap();
                    format!("0x{d:x}")
                }
            }
        }
        else {
            let d = self.resolve_expr_must_never_fail(exp).unwrap();
            format!("0x{d:x}")
        }
    }

    fn visit_run<E: ExprEvaluationExt>(
        &mut self,
        address: &E,
        ga: Option<&E>
    ) -> Result<(), Box<AssemblerError>> {
        let address = { let __r = self.resolve_expr_may_fail_in_first_pass(address)?; self.int_forward(&__r)? };

        if let Some(o) = self.listing_trigger() {
            o.replace_code_address(&address.into())
        }

        if self.run_options.is_some() {
            #[allow(unreachable_code)]
            return Err(Box::new(AssemblerError::RunAlreadySpecified));
            #[allow(unreachable_code)]
            return Err(Box::new(AssemblerError::RunAlreadySpecified));
        }
        self.sna
            .set_value(cpclib_sna::SnapshotFlag::Z80_PC, address as _)?;

        match ga {
            None => {
                self.run_options = Some((address as _, None));
            },
            Some(ga_expr) => {
                let ga_expr = { let __r = self.resolve_expr_may_fail_in_first_pass(ga_expr)?; self.int_forward(&__r)? };
                self.sna.set_value(SnapshotFlag::GA_RAMCFG, address as _)?;
                self.run_options = Some((address as _, Some(ga_expr as _)));
            }
        }
        Ok(())
    }
}

/// Macro related code
impl Env {
    pub fn inc_macro_seed(&mut self) {
        self.macro_seed += 1;
    }

    pub fn macro_seed(&self) -> usize {
        self.macro_seed
    }
}

/// Warnings related code
impl Env {
    fn merge_overriding_warnings(&mut self) {
        // Filter the warnings to merge overriding
        let mut current_warning_idx = 1; // index to the last warning to treat
        let mut previous_warning_idx = 0; // index to the previous warning treated (diff with current_warning_idx can be higher than 1 when there are several consecutive warnings for OverrideMemory)

        while current_warning_idx < self.warnings.len() {
            // Check if we need to fuse successive override memory warnings
            let (new_size, new_span) = match (
                &*self.warnings[previous_warning_idx],
                &*self.warnings[current_warning_idx]
            ) {
                // we fuse two consecutive override memory warnings
                (
                    AssemblerWarning::OverrideMemory(prev_addr, prev_size),
                    AssemblerWarning::OverrideMemory(curr_addr, curr_size)
                ) => {
                    if (prev_addr.offset_in_cpc() + *prev_size as u32) == curr_addr.offset_in_cpc()
                    {
                        (Some(*prev_size + *curr_size), None)
                    }
                    else {
                        (None, None)
                    }
                },

                (
                    AssemblerError::RelocatedWarning {
                        warning: prev_warning,
                        span: prev_span
                    },
                    AssemblerError::RelocatedWarning {
                        warning: curr_warning,
                        span: curr_span
                    }
                ) => {
                    if let (
                        AssemblerWarning::OverrideMemory(prev_addr, prev_size),
                        AssemblerWarning::OverrideMemory(curr_addr, curr_size)
                    ) = (prev_warning.as_ref(), curr_warning.as_ref())
                        && (prev_addr.offset_in_cpc() + *prev_size as u32
                            == curr_addr.offset_in_cpc())
                        && std::ptr::eq(
                            prev_span.complete_source().as_ptr(),
                            curr_span.complete_source().as_ptr()
                        )
                    {
                        let new_size = *prev_size + *curr_size;

                        let start_str = prev_span.as_str();
                        let end_str = curr_span.as_str();
                        let start_str = start_str.as_bytes();
                        let end_str = end_str.as_bytes();

                        let start_ptr = &start_str[0] as *const u8;
                        let end_last_ptr = &end_str[end_str.len() - 1] as *const u8;
                        assert!(end_last_ptr > start_ptr);
                        let txt = unsafe {
                            let slice = std::slice::from_raw_parts(
                                start_ptr,
                                end_last_ptr.offset_from(start_ptr) as _
                            );
                            std::str::from_utf8(slice).unwrap()
                        };

                        let new_span = Z80Span::from(prev_span.0.update_slice(txt.as_bytes()));

                        (Some(new_size), Some(new_span))
                    }
                    else {
                        (None, None)
                    }
                },

                _ => {
                    // nothing to do ATM
                    (None, None)
                }
            };

            if let Some(new_size) = new_size {
                if let Some(new_span) = new_span {
                    if let AssemblerError::RelocatedWarning { warning, span } =
                        &mut *self.warnings[previous_warning_idx]
                        && let AssemblerWarning::OverrideMemory(_prev_addr, prev_size) =
                            warning.as_mut()
                    {
                        *prev_size = new_size;
                        *span = new_span;
                    }
                }
                else if let AssemblerWarning::OverrideMemory(_prev_addr, prev_size) =
                    &mut *self.warnings[previous_warning_idx]
                {
                    *prev_size = new_size;
                }
            }
            else {
                previous_warning_idx += 1;
                if previous_warning_idx != current_warning_idx {
                    self.warnings
                        .swap(previous_warning_idx, current_warning_idx);
                }
            }

            current_warning_idx += 1;
        }
        // change the length  of the vector to remove all eated ones
        self.warnings.truncate(previous_warning_idx + 1);
    }

    fn render_warnings(&mut self) {
        // Transform the warnings as strings. This must stay unconditional for
        // anything still holding a live `Z80Span` (`RelocatedWarning` and
        // friends): some spans (e.g. ones pointing into a macro-expansion
        // scratch buffer) are not safe to read from once a later pass has
        // reused/overwritten the buffer they point into, despite `Z80Span`'s
        // `'static`-shaped internals - rendering promptly, here, is what
        // keeps that access safe. `AlreadyRenderedWarningWithLocation` is the
        // one exception that doesn't need it: it holds no span, only plain
        // owned data (message + line/column/len captured eagerly at
        // construction time).
        //
        // `RelocatedWarning` specifically gets *converted into*
        // `AlreadyRenderedWarningWithLocation` here (not just rendered to a
        // bare `AssemblingError`, which was this function's original
        // behavior): its span is still valid right now, so this is the last
        // safe point to capture its structured line/column/len before the
        // span itself is discarded - losing that structure meant every
        // warning routed through this generic path (e.g. `OverrideMemory`,
        // or a `checked_byte`/`checked_word` overflow warning routed through
        // `visit_located_token`'s auto-locate wrapper) rendered with no
        // usable location at all downstream (the LSP fell back to a
        // location-less diagnostic for every one of them).
        self.warnings.iter_mut().for_each(|w| {
            match &**w {
                AssemblerError::AssemblingError { .. }
                | AssemblerError::AlreadyRenderedWarningWithLocation { .. } => {
                    // already in a final, safe-to-keep shape
                },
                AssemblerError::RelocatedWarning { span, .. } => {
                    let (line, column) = span.relative_line_and_column();
                    let len = span.as_str().len();
                    let msg = (**w).to_string();
                    **w = AssemblerError::AlreadyRenderedWarningWithLocation {
                        msg,
                        line: line as u32,
                        column: column as u32,
                        len: len as u32
                    };
                },
                _ => {
                    **w = AssemblerWarning::AssemblingError {
                        msg: (**w).to_string()
                    }
                },
            }
        });
    }

    /// Merges adjacent `OverrideMemory` warnings and renders every warning
    /// still holding a live `Z80Span` into an owned, safe-to-keep shape -
    /// see `render_warnings`'s doc comment for why the rendering itself
    /// can't be skipped or deferred.
    ///
    /// Called at the end of *every* `visit_processed_tokens` (`processed_token.rs`)
    /// - once per macro/struct expansion, `INCLUDE`, `IF` branch and `REPEAT`
    /// iteration visited, not just once per pass - because any of those can
    /// be the last chance to render a warning before the buffer its span
    /// points into gets reused. Both `merge_overriding_warnings` and
    /// `render_warnings` are full `O(warnings.len())` scans, so calling this
    /// unconditionally made total cost scale with
    /// `(nested visits) × (warnings accumulated so far)` - largely wasted
    /// work, since a real demoscene source visits far more nested blocks
    /// than it ever emits warnings (confirmed as the single largest cost in
    /// this crate under profiling: ~12% of all instructions assembling a
    /// real project, versus a low single-digit warning count). Skipping
    /// outright when nothing was pushed since the last call is always safe:
    /// with no new warning, there is nothing unrendered that could go
    /// dangling, and the merge has nothing new to merge.
    pub fn cleanup_warnings(&mut self) {
        if !self.options().assemble_options().enable_warnings {
            debug_assert!(self.warnings.is_empty());
            return;
        }

        if self.warning_push_count == self.warnings_cleaned_up_to {
            return;
        }
        self.warnings_cleaned_up_to = self.warning_push_count;

        self.merge_overriding_warnings();
        self.render_warnings();
    }
}

impl Env {
    pub fn visit_equ<E: ExprEvaluationExt + ExprElement + Debug, S: SourceString + MayHaveSpan>(
        &mut self,
        label_span: &S,
        exp: &E
    ) -> Result<(), Box<AssemblerError>> {
        if self.symbols().contains_symbol(label_span.as_str())? && self.pass.is_first_pass() {
            let key = self
                .symbols()
                .normalize_symbol(label_span.as_str())
                .value()
                .to_string();
            let error = AssemblerError::AlreadyDefinedSymbol {
                symbol: label_span.as_str().into(),
                kind: self.symbols().kind(label_span.as_str())?.into(),
                // The *original* definition's location, not this (the
                // conflicting, about-to-fail) occurrence's - same pattern as
                // `visit_label` above. Using `label_span.possible_span()`
                // here reported the current line twice (once as prose, once
                // as the codespan block), which is useless for finding the
                // actual duplicate.
                here: self
                    .symbols()
                    .any_value(label_span.as_str())?
                    .and_then(|v| v.location().cloned())
            };
            // Locate with *this* occurrence's own span first, then attach
            // how the *original* definition was itself reached (e.g. via
            // `INCLUDE`) - just as important for tracking down a duplicate
            // as this occurrence's own chain (already shown by the codespan
            // block), and otherwise invisible (`here` is only a flat
            // file:line:col). Appended *after* locating, not a field of
            // `AlreadyDefinedSymbol` itself - see its doc comment for why.
            let mut error = match label_span.possible_span() {
                Some(span) => Box::new(error.locate(span.clone())),
                None => Box::new(error)
            };
            for note in self
                .symbol_definition_chains
                .get(&key)
                .cloned()
                .unwrap_or_default()
            {
                error = error.with_chain_note(note);
            }
            Err(error)
        }
        else {
            let label = self.handle_global_and_local_labels(label_span.as_str())?;
            let normalized_label = self.symbols().normalize_symbol(label);
            let normalized_label_value = normalized_label.value();
            let normalized_braced_label = format!("{{{}}}", normalized_label_value);

            // Remember how *this* definition was reached, in case a later
            // `equ` for the same name conflicts with it - see
            // `Env::symbol_definition_chains`.
            if !self.active_frames.is_empty() {
                let key = self
                    .symbols()
                    .normalize_symbol(label_span.as_str())
                    .value()
                    .to_string();
                self.symbol_definition_chains
                    .insert(key, self.active_frames_as_notes());
            }

            // Forbid self-referential EQU (e.g. `x equ y - x`), otherwise first-pass fallback
            // can silently inject a bogus value and make the symbol look valid.
            if exp.symbols_used().iter().any(|used| {
                let normalized_used = self.symbols().normalize_symbol(used.as_ref());
                let used_value = normalized_used.value();
                used_value == normalized_label_value || used_value == normalized_braced_label
            }) {
                return Err(Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "Invalid self-referential EQU for symbol '{}'",
                        normalized_label_value
                    )
                }));
            }

            if label.starts_with(".") {
                // `AssemblerWarning` is `AssemblerError` (type alias) and
                // `Display for AssemblingError` is just `write!(f, "{msg}")`
                // - building an AssemblerError here and then re-rendering
                // it via `.to_string()` into a second, identically-shaped
                // value produced the exact same text through a wasted
                // allocate-format-parse round trip. One construction is
                // enough.
                let warning = AssemblerWarning::AssemblingError {
                    msg: format!(
                        "{} is not a local label. A better name without the dot would be better",
                        label
                    )
                };
                self.add_warning(Box::new(warning));
            }

            // XXX Disabled behavior the 12/01/2024
            // if !label.starts_with('.') {
            // self.symbols_mut().set_current_label(label)?;
            // }
            let value = self.resolve_expr_may_fail_in_first_pass(exp)?;
            if let Some(o) = self.listing_trigger() {
                o.replace_code_address(&value)
            }
            self.add_symbol_to_symbol_table(
                label,
                value,
                label_span.possible_span().map(|s| s.into())
            )
        }
    }

    pub fn visit_even(&mut self) -> Result<(), Box<AssemblerError>> {
        // EVEN is shorthand for ALIGN 2
        let boundary = Expr::Value(2);
        self.visit_align(&boundary, None)
    }

    /// Assign sequentially increasing values to each label in an ENUM block.
    /// prefix:  optional prefix prepended to each label name (with underscore separator)
    /// start:   optional starting value expression (defaults to 0)
    /// step:    optional increment expression (defaults to 1)
    /// fields:  list of (label_name, optional_override_value)
    pub fn visit_enum<
        S: SourceString,
        E: ExprEvaluationExt + ExprElement + Debug,
        L: SourceString,
        V: ExprEvaluationExt + ExprElement + Debug
    >(
        &mut self,
        prefix: Option<&S>,
        start: Option<&E>,
        step: Option<&E>,
        fields: &[(L, Option<V>)]
    ) -> Result<(), Box<AssemblerError>> {
        let mut counter: i32 = if let Some(s) = start {
            { let __r = self.resolve_expr_must_never_fail(s)?; self.int_forward(&__r)? }
        }
        else {
            0
        };
        let step_val: i32 = if let Some(s) = step {
            { let __r = self.resolve_expr_must_never_fail(s)?; self.int_forward(&__r)? }
        }
        else {
            1
        };
        for (label, override_val) in fields {
            if let Some(ov) = override_val {
                counter = { let __r = self.resolve_expr_must_never_fail(ov)?; self.int_forward(&__r)? };
            }
            let symbol_name: String = if let Some(p) = prefix {
                format!("{}_{}", p.as_str(), label.as_str())
            }
            else {
                label.as_str().to_owned()
            };
            let value: ExprResult = counter.into();
            self.add_symbol_to_symbol_table(&symbol_name, value, None)?;
            counter = counter.wrapping_add(step_val);
        }
        Ok(())
    }

    fn visit_field<
        E: ExprEvaluationExt + ExprElement + Debug + MayHaveSpan,
        S: SourceString + MayHaveSpan
    >(
        &mut self,
        label_span: S,
        exp: &E
    ) -> Result<(), Box<AssemblerError>> {
        if self.symbols().contains_symbol(label_span.as_str())? && self.pass.is_first_pass() {
            Err(Box::new(AssemblerError::AlreadyDefinedSymbol {
                symbol: label_span.as_str().into(),
                kind: self.symbols().kind(label_span.as_str())?.into(),
                here: self
                    .symbols()
                    .any_value(label_span.as_str())?
                    .and_then(|v| v.location().cloned())
            }))
        }
        else {
            let delta = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
            if delta < 0 {
                let mut e = AssemblerError::AlreadyRenderedError(format!(
                    "FIELD argument must be positive ({delta} is a wrong value)."
                ));
                if let Some(span) = exp.possible_span() {
                    e = e.locate(span.clone());
                }

                return Err(Box::new(e));
            }

            let label = self.handle_global_and_local_labels(label_span.as_str())?;
            if !label.starts_with('.') {
                self.symbols_mut().set_current_label(label)?;
            }

            let value: ExprResult = self.map_counter.into();
            if let Some(o) = self.listing_trigger() {
                o.replace_code_address(&value)
            }
            self.add_symbol_to_symbol_table(
                label,
                value,
                label_span.possible_span().map(|l| l.into())
            )?;

            self.map_counter = self.map_counter.wrapping_add(delta);

            Ok(())
        }
    }

    pub fn visit_assign<'e, E: ExprEvaluationExt + ExprElement + Clone, S: AsRef<str>>(
        &mut self,
        label: S,
        exp: &E,
        op: Option<&BinaryOperation>
    ) -> Result<(), Box<AssemblerError>> {
        let label = label.as_ref();
        let value = if let Some(op) = op {
            let new_exp = Expr::BinaryOperation(
                *op,
                Box::new(Expr::Label(label.into())),
                Box::new(exp.to_expr().into_owned())
            );
            self.resolve_expr_must_never_fail(&new_exp)?
        }
        else {
            self.resolve_expr_may_fail_in_first_pass(exp)?
        };

        if let Some(o) = self.listing_trigger() {
            o.replace_code_address(&value)
        }

        let label = self.handle_global_and_local_labels(label)?;
        // XXX Disabled behavior the 12/01/2024
        // if !label.starts_with('.') {
        // self.symbols_mut().set_current_label(label)?;
        // }
        self.symbols_mut().assign_symbol_to_value(label, value)?;
        Ok(())
    }
}

// visit_defs and visit_end have been moved into the Env impl block above.

pub enum DbLikeKind {
    Defb,
    Defw,
    Str
}

impl From<&Token> for DbLikeKind {
    fn from(token: &Token) -> Self {
        match token {
            Token::Defb(..) => Self::Defb,
            Token::Defw(..) => Self::Defw,
            Token::Str(..) => Self::Str,
            _ => unreachable!()
        }
    }
}

impl DbLikeKind {
    fn mask(&self) -> u16 {
        match self {
            DbLikeKind::Defb => 0xFF,
            DbLikeKind::Defw => 0xFFFF,
            DbLikeKind::Str => 0xFF
        }
    }
}

impl Env {
    pub fn visit_abyte<
        E1: ExprEvaluationExt + ExprElement + Debug,
        E2: ExprEvaluationExt + ExprElement + Debug
    >(
        &mut self,
        delta: &E1,
        exprs: &[E2]
    ) -> Result<(), Box<AssemblerError>> {
        let delta = self.resolve_expr_may_fail_in_first_pass(delta)?;
        self.visit_db_or_dw_or_str(DbLikeKind::Defb, exprs, delta)
    }

    // TODO refactor code with assemble_opcode or other functions manipulating bytes
    pub fn visit_db_or_dw_or_str<E: ExprEvaluationExt + ExprElement + Debug>(
        &mut self,
        kind: DbLikeKind,
        exprs: &[E],
        delta: ExprResult
    ) -> Result<(), Box<AssemblerError>> {
        let env = self;

        let delta = env.int_forward(&delta)?;

        let mask = kind.mask();

        fn output(
            env: &mut Env,
            val: i32,
            delta: i32,
            mask: u16
        ) -> Result<(), Box<AssemblerError>> {
            let val: i32 = val + delta;

            if mask == 0xFF {
                let b = env.checked_byte(val);
                env.output_byte(b)?;
            }
            else {
                let w = env.checked_word(val);
                let high = (w >> 8) as u8;
                let low = (w & 0xFF) as u8;
                env.output_byte(low)?;
                env.output_byte(high)?;
            }
            Ok(())
        }

        fn output_expr_result(
            env: &mut Env,
            expr: &ExprResult,
            delta: i32,
            mask: u16
        ) -> Result<(), Box<AssemblerError>> {
            match &expr {
                ExprResult::Float(_) | ExprResult::Value(_) | ExprResult::Bool(_) => {
                    let raw = env.int_forward(expr)?;
                    output(env, raw, delta, mask)
                },
                ExprResult::Char(c) => {
                    // XXX here it is problematci c shold be a char and not a byte
                    let _c = env.charset_encoding.transform_char(*c as char);
                    let raw = env.int_forward(expr)?;
                    output(env, raw, delta, mask)
                },
                ExprResult::String(s) => {
                    let bytes = env.charset_encoding.transform_string(s);

                    for c in bytes {
                        output(env, c as _, delta, mask)?;
                    }
                    Ok(())
                },
                ExprResult::List(l) => {
                    for c in l.iter() {
                        output_expr_result(env, c, delta, mask)?;
                    }
                    Ok(())
                },
                ExprResult::Matrix { .. } => {
                    for row in expr.matrix_rows() {
                        for c in row.list_content() {
                            output_expr_result(env, c, delta, mask)?;
                        }
                    }
                    Ok(())
                }
            }
        }

        let backup_address = env.logical_output_address();
        for exp in exprs.iter() {
            let exp = env.resolve_expr_may_fail_in_first_pass(exp)?;
            output_expr_result(env, &exp, delta, mask)?;
            env.update_dollar();
        }

        // Patch the last char of a str
        if matches!(kind, DbLikeKind::Str) && backup_address < env.logical_output_address() {
            let last_address = env.logical_output_address() - 1;
            let last_address = env.logical_to_physical_address(last_address as _);
            let last_value = env.peek(&last_address);
            let patched_last_value = last_value | 0x80;
            let _ = env.poke(patched_last_value, &last_address);

            // Keep listing bytes aligned with actual emitted bytes when STR patches
            // the last character with bit 7 set.
            if env.pass.is_listing_pass()
                && let Some(last) = env
                    .listing_trigger()
                    .and_then(|trigger| trigger.bytes.last_mut())
            {
                *last = patched_last_value;
            }
        }

        // Update stable ticker counters when active
        if !env.stable_counters.is_empty() {
            let num_bytes = env.logical_output_address().wrapping_sub(backup_address) as usize;

            // TODO add that in a function to reuse it with DEFS
            // collect the bytes
            let mut bytes = (0..num_bytes)
                .into_iter()
                .map(|i| {
                    let addr = backup_address.wrapping_add(i as u16);
                    let phy = env.logical_to_physical_address(addr);
                    env.peek(&phy)
                })
                .collect_vec();

            // disassemble the bytes
            let obtained_listing = disassemble(&bytes);

            // check there is only compatible mnemonics
            obtained_listing.iter().try_for_each(|token| {
                    match token.mnemonic() {
                        Some(Mnemonic::Cpir | Mnemonic::Cpdr | Mnemonic::Ldir | Mnemonic::Lddr | Mnemonic::Otdr | Mnemonic::Otir) => {
                            Err(Box::new(AssemblerError::AssemblingError {
                                msg: format!("TICKER cannot compute timing for looping instruction. 
                                Here we have {}", token
                                )
                            }))
                        }
                        Some(_) => Ok(()),
                        None => {
                    Err(Box::new(AssemblerError::AssemblingError {
                        msg: format!("TICKER cannot compute timing for DB/DW directive when not generating valid mnemmonics. 
                        Here we have {}", token
                        )
                    }))
                }
            }
        })?;

            let listing_duration = obtained_listing.estimated_duration().unwrap();
            env.stable_counters.update_counters(listing_duration);
        }

        Ok(())
    }
}

impl Env {
    // TODO find a more efficient way; there a tons of copies there...
    fn move_delayed_commands_of_functions(&mut self) {
        {
            let prints = self.extra_print_from_function.read().unwrap().clone();
            for print in prints.into_iter() {
                self.active_page_info_mut()
                    .add_print_or_pause_command(print);
            }
            self.extra_print_from_function.write().unwrap().clear();
        }

        {
            let asserts = self
                .extra_failed_assert_from_function
                .read()
                .unwrap()
                .clone();
            for assert in asserts.into_iter() {
                self.active_page_info_mut()
                    .add_failed_assert_command(assert);
            }
            self.extra_failed_assert_from_function
                .write()
                .unwrap()
                .clear();
        }
    }
}

#[allow(missing_docs)]
impl Env {
    pub fn visit_basic<S: SourceString, S2: SourceString, E: ExprEvaluationExt>(
        &mut self,
        variables: Option<&[S]>,
        hidden_lines: Option<&[E]>,
        code: S2
    ) -> Result<(), Box<AssemblerError>> {
        let bytes = self.assemble_basic(variables, hidden_lines, code)?;

        // If the basic directive is the VERY first thing to output,
        // we assume startadr is 0x170 as for any basic program
        if self.start_address().is_none() {
            self.active_page_info_mut().logical_outputadr = 0x170;
            self.active_page_info_mut().logical_codeadr = self.logical_output_address();
            self.active_page_info_mut().startadr = Some(self.logical_output_address());
            self.output_address = 0x170;

            // Keep listing token start/end addresses aligned with the effective
            // BASIC load address when LOCOMOTIVE is the first emitted content.
            if self.listing_is_recording() {
                let code_adr = self.logical_code_address();
                let output_adr = self.logical_to_physical_address(self.logical_output_address());
                let trigger = self.listing_trigger().unwrap();

                trigger.replace_code_address(&code_adr.into());
                trigger.replace_physical_address(output_adr);
            }
        }

        self.output_bytes(&bytes)
    }

    pub fn assemble_basic<S: SourceString, S2: SourceString, E: ExprEvaluationExt>(
        &mut self,
        variables: Option<&[S]>,
        hidden_lines: Option<&[E]>,
        code: S2
    ) -> Result<Vec<u8>, Box<AssemblerError>> {
        let hidden_lines: Option<Vec<u16>> = if let Some(lines) = hidden_lines {
            let mut resolved = Vec::with_capacity(lines.len());
            for expr in lines {
                let val = { let __r = self.resolve_expr_must_never_fail(expr)?; self.int_forward(&__r)? };
                resolved.push(val as u16);
            }
            Some(resolved)
        }
        else {
            None
        };

        // Build the final basic code by replacing variables by value
        // Hexadecimal is used to ensure a consistent 2 bytes representation
        let basic_src = {
            let mut basic = code.as_str().to_owned();
            if let Some(arguments) = variables {
                for argument in arguments {
                    let key = format!("{{{}}}", argument.as_str());
                    let value = format!(
                        "&{:X}",
                        self.resolve_expr_may_fail_in_first_pass(&Expr::from(argument.as_str()))?
                    );
                    basic = basic.replace(&key, &value);
                }
            }
            basic
        };

        // build the basic tokens
        let mut basic = BasicProgram::parse(basic_src)?;
        if let Some(hidden_lines) = hidden_lines {
            basic.hide_lines(&hidden_lines)?;
        }
        Ok(basic.as_bytes())
    }
}

fn visit_token(token: &Token, env: &mut Env) -> Result<(), Box<AssemblerError>> {
    let span = None;
    let _res = visit_token_impl!(token, env, span, Token);

    env.move_delayed_commands_of_functions();
    Ok(())
}

/// Assemble DEFS directive
impl Env {
    pub fn assemble_defs_item<E: ExprEvaluationExt>(
        &mut self,
        expr: &E,
        fill: Option<&E>
    ) -> Result<Bytes, Box<AssemblerError>> {
        let count = match self.resolve_expr_must_never_fail(expr) {
            Ok(amount) => self.int_forward(&amount)?,
            Err(e) => {
                self.add_error_discardable_one_pass(e)?;
                *self.request_additional_pass.write().unwrap() = true; // we expect to obtain this value later
                0
            }
        };

        if count < 0 {
            return Err(Box::new(AssemblerError::AssemblingError {
                msg: format!("DEFS count must be positive ({count} is an invalid value)")
            }));
        }

        let value = if fill.is_none() {
            0
        }
        else {
            let result = self.resolve_expr_may_fail_in_first_pass(fill.unwrap())?;
            let raw = self.int_forward(&result)?;
            self.checked_byte(raw)
        };

        let mut bytes = Bytes::with_capacity(count as usize);
        bytes.resize_with(count as _, || value);

        Ok(bytes)
    }

    /// Assemble align directive. It can only work if current address is known...
    pub fn assemble_align(
        &mut self,
        expr: &Expr,
        fill: Option<&Expr>
    ) -> Result<Bytes, Box<AssemblerError>> {
        let expression = { let __r = self.resolve_expr_must_never_fail(expr)?; self.int_forward(&__r)? } as u16;
        let current = self.symbols().current_address()?;
        let value = if fill.is_none() {
            0
        }
        else {
            let result = self.resolve_expr_may_fail_in_first_pass(fill.unwrap())?;
            let raw = self.int_forward(&result)?;
            self.checked_byte(raw)
        };

        // compute the number of 0 to put
        let mut until = current;
        while !until.is_multiple_of(expression) {
            until += 1;
        }

        // Create the vector
        let hole = (until - current) as usize;
        let mut bytes = Bytes::with_capacity(hole);
        for _i in 0..hole {
            bytes.push(value);
        }

        // and return it
        Ok(bytes)
    }
}

/// Assemble the opcode and inject in the environement
impl Env {
    pub fn visit_opcode<D: DataAccessElem>(
        &mut self,
        mnemonic: Mnemonic,
        arg1: &Option<D>,
        arg2: &Option<D>,
        arg3: &Option<Register8>
    ) -> Result<(), Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        // TODO update $ in the symbol table
        let bytes = self.assemble_opcode_impl(mnemonic, arg1, arg2, arg3)?;
        for b in bytes.iter() {
            self.output_byte(*b)?;
        }

        Ok(())
    }

    /// Assemble an opcode and returns the generated bytes or the error message if it is impossible to
    /// assemble.
    /// We assume the opcode is properly coded. Panic occurs if it is not the case
    pub(crate) fn assemble_opcode_impl<D: DataAccessElem>(
        &mut self,
        mnemonic: Mnemonic,
        arg1: &Option<D>,
        arg2: &Option<D>,
        arg3: &Option<Register8>
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        match mnemonic {
            // `parse_logical_operator` always populates `arg2` (the real
            // operand) - `arg1` is only ever the optional explicit `A,`
            // prefix, which never affects encoding. Fall back to `arg1`
            // defensively rather than panicking if some other construction
            // path ever produces the pre-fix shape.
            Mnemonic::And | Mnemonic::Or | Mnemonic::Xor => {
                self.assemble_logical_operator(mnemonic, arg2.as_ref().or(arg1.as_ref()).unwrap())
            },
            Mnemonic::Add | Mnemonic::Adc => {
                self.assemble_add_or_adc::<_, Token>(
                    mnemonic,
                    arg1.as_ref(),
                    arg2.as_ref().unwrap()
                )
            },
            // `parse_cp` always populates `arg2` (the compared value) -
            // `arg1` is only ever the optional explicit `A,` prefix, which
            // never affects encoding. Fall back to `arg1` defensively
            // rather than panicking if some other construction path ever
            // produces the pre-fix shape (compared value in `arg1`, `arg2`
            // empty).
            Mnemonic::Cp => self.assemble_cp(arg2.as_ref().or(arg1.as_ref()).unwrap()),
            Mnemonic::ExMemSp => self.assemble_ex_memsp(arg1.as_ref().unwrap()),
            Mnemonic::Dec | Mnemonic::Inc => {
                self.assemble_inc_dec(mnemonic, arg1.as_ref().unwrap())
            },
            Mnemonic::Djnz => self.assemble_djnz(arg1.as_ref().unwrap()),
            Mnemonic::In => self.assemble_in(arg1.as_ref().unwrap(), arg2.as_ref().unwrap()),
            Mnemonic::Ld => {
                self.assemble_ld::<_, Token>(arg1.as_ref().unwrap(), arg2.as_ref().unwrap())
            },
            Mnemonic::Ldi
            | Mnemonic::Ldd
            | Mnemonic::Ldir
            | Mnemonic::Lddr
            | Mnemonic::Outi
            | Mnemonic::Outd
            | Mnemonic::Ei
            | Mnemonic::Di
            | Mnemonic::ExAf
            | Mnemonic::ExHlDe
            | Mnemonic::Exx
            | Mnemonic::Halt
            | Mnemonic::Ind
            | Mnemonic::Indr
            | Mnemonic::Ini
            | Mnemonic::Inir
            | Mnemonic::Rla
            | Mnemonic::Rlca
            | Mnemonic::Rrca
            | Mnemonic::Rra
            | Mnemonic::Reti
            | Mnemonic::Retn
            | Mnemonic::Scf
            | Mnemonic::Ccf
            | Mnemonic::Cpd
            | Mnemonic::Cpdr
            | Mnemonic::Cpi
            | Mnemonic::Cpir
            | Mnemonic::Cpl
            | Mnemonic::Daa
            | Mnemonic::Neg
            | Mnemonic::Otdr
            | Mnemonic::Otir
            | Mnemonic::Rld
            | Mnemonic::Rrd => Env::assemble_no_arg(mnemonic),
            Mnemonic::Out => self.assemble_out(arg1.as_ref().unwrap(), arg2.as_ref().unwrap()),
            Mnemonic::Jr | Mnemonic::Jp | Mnemonic::Call => {
                self.assemble_call_jr_or_jp(mnemonic, arg1.as_ref(), arg2.as_ref().unwrap())
            },
            Mnemonic::Jq => self.assemble_jq(arg1.as_ref(), arg2.as_ref().unwrap()),
            Mnemonic::Pop => self.assemble_pop(arg1.as_ref().unwrap()),
            Mnemonic::Push => self.assemble_push(arg1.as_ref().unwrap()),
            Mnemonic::Bit | Mnemonic::Res | Mnemonic::Set => {
                self.assemble_bit_res_or_set(
                    mnemonic,
                    arg1.as_ref().unwrap(),
                    arg2.as_ref().unwrap(),
                    arg3.as_ref()
                )
            },
            Mnemonic::Ret => self.assemble_ret(arg1.as_ref()),
            Mnemonic::Rst => {
                if let Some(arg2) = arg2.as_ref() {
                    self.assemble_rst_fake(arg1.as_ref().unwrap(), arg2)
                }
                else {
                    // normal RST
                    self.assemble_rst(arg1.as_ref().unwrap())
                }
            },
            Mnemonic::Im => self.assemble_im(arg1.as_ref().unwrap()),
            Mnemonic::Nop => {
                self.assemble_nop(
                    Mnemonic::Nop,
                    arg1.as_ref().map(|v| v.get_expression().unwrap())
                )
            },
            Mnemonic::Nop2 => self.assemble_nop::<Expr>(Mnemonic::Nop2, None),
            Mnemonic::Sub => self.assemble_sub::<_, Token>(arg1.as_ref(), arg2.as_ref()),
            Mnemonic::Sbc => self.assemble_sbc::<_, Token>(arg1.as_ref(), arg2.as_ref().unwrap()),
            Mnemonic::Sla
            | Mnemonic::Sra
            | Mnemonic::Srl
            | Mnemonic::Sl1
            | Mnemonic::Rl
            | Mnemonic::Rr
            | Mnemonic::Rlc
            | Mnemonic::Rrc => {
                self.assemble_shift::<_, Token>(mnemonic, arg1.as_ref().unwrap(), arg2.as_ref())
            },
            Mnemonic::Srl8 => self.assemble_srl8::<_, Token>(arg1.as_ref().unwrap())
        }
    }
}

impl Env {
    fn assemble_no_arg(mnemonic: Mnemonic) -> Result<Bytes, Box<AssemblerError>> {
        let bytes: &[u8] = match mnemonic {
            Mnemonic::Ldi => &[0xED, 0xA0],
            Mnemonic::Ldd => &[0xED, 0xA8],
            Mnemonic::Lddr => &[0xED, 0xB8],
            Mnemonic::Ldir => &[0xED, 0xB0],
            Mnemonic::Di => &[0xF3],
            Mnemonic::ExAf => &[0x08],
            Mnemonic::ExHlDe => &[0xEB],
            Mnemonic::Exx => &[0xD9],
            Mnemonic::Ei => &[0xFB],
            Mnemonic::Halt => &[0x76],
            Mnemonic::Ind => &[0xED, 0xAA],
            Mnemonic::Indr => &[0xED, 0xBA],
            Mnemonic::Ini => &[0xED, 0xA2],
            Mnemonic::Inir => &[0xED, 0xB2],
            Mnemonic::Outd => &[0xED, 0xAB],
            Mnemonic::Outi => &[0xED, 0xA3],
            Mnemonic::Rla => &[0x17],
            Mnemonic::Rlca => &[0x07],
            Mnemonic::Rrca => &[0x0F],
            Mnemonic::Rra => &[0x1F],
            Mnemonic::Reti => &[0xED, 0x4D],
            Mnemonic::Retn => &[0xED, 0x45],
            Mnemonic::Scf => &[0x37],
            Mnemonic::Ccf => &[0x3F],
            // added
            Mnemonic::Cpd => &[0xED, 0xA9],
            Mnemonic::Cpdr => &[0xED, 0xB9],
            Mnemonic::Cpi => &[0xED, 0xA1],
            Mnemonic::Cpir => &[0xED, 0xB1],
            Mnemonic::Cpl => &[0x2F],
            Mnemonic::Daa => &[0x27],
            Mnemonic::Neg => &[0xED, 0x44],
            Mnemonic::Otdr => &[0xED, 0xBB],
            Mnemonic::Otir => &[0xED, 0xB3],
            Mnemonic::Rld => &[0xED, 0x6F],
            Mnemonic::Rrd => &[0xED, 0x67],
            _ => {
                return Err(Box::new(AssemblerError::BugInAssembler {
                    file: file!(),
                    line: line!(),
                    msg: format!("{mnemonic} not treated")
                }));
            }
        };

        Ok(Bytes::from_slice(bytes))
    }

    fn assemble_fake_listing(
        &mut self,
        listing: &[(Mnemonic, Option<DataAccess>, Option<DataAccess>)]
    ) -> Result<Bytes, Box<AssemblerError>> {
        let mut bytes = Bytes::new();
        for (mnemonic, arg1, arg2) in listing {
            let op_bytes = self.assemble_opcode_impl::<DataAccess>(*mnemonic, arg1, arg2, &None)?;
            bytes.extend(op_bytes);
        }

        Ok(bytes)
    }
}

/// Converts an absolute address to a relative one (relative to $)
pub fn absolute_to_relative<T: AsRef<SymbolsTable>>(
    address: i32,
    opcode_delta: i32,
    sym: T
) -> Result<u8, Box<AssemblerError>> {
    match sym.as_ref().current_address() {
        Err(_msg) => Err(Box::new(AssemblerError::UnknownAssemblingAddress)),
        Ok(root) => {
            let delta = (address - i32::from(root)) - opcode_delta;
            if !(-128..=127).contains(&delta) {
                Err(Box::new(AssemblerError::InvalidArgument {
                    msg: format!("Address 0x{address:x} relative to 0x{root:x} is too far {delta}")
                }))
            }
            else {
                let res = (delta & 0xFF) as u8;
                Ok(res)
            }
        }
    }
}

#[allow(missing_docs)]
impl Env {
    pub fn assemble_cp<D: DataAccessElem>(
        &mut self,
        arg: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        if arg.is_register8() {
            let reg = arg.get_register8().unwrap();
            {
                add_byte(&mut bytes, 0b1011_1000 + register8_to_code(reg));
            }
        }
        else if arg.is_indexregister8() {
            let reg = arg.get_indexregister8().unwrap();
            {
                add_byte(&mut bytes, indexed_register16_to_code(reg.complete()));
                add_byte(&mut bytes, 0b1011_1000 + indexregister8_to_code(reg));
            }
        }
        else if arg.is_expression() {
            let exp = arg.get_expression().unwrap();
            {
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let val = self.checked_byte(raw);
                add_byte(&mut bytes, 0xFE);
                add_byte(&mut bytes, val);
            }
        }
        else if arg.is_address_in_register16() && arg.get_register16().unwrap() == Register16::Hl
        {
            {
                add_byte(&mut bytes, 0xBE);
            }
        }
        else if arg.is_indexregister_with_index() {
            let reg = arg.get_indexregister16().unwrap();
            let idx = arg.get_index().unwrap();
            {
                add_byte(&mut bytes, indexed_register16_to_code(reg));
                add_byte(&mut bytes, 0xBE);
                add_byte(
                    &mut bytes,
                    { let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } as _
                );
            }
        }
        else {
            unreachable!()
        }

        Ok(bytes)
    }

    pub fn assemble_sub<D: DataAccessElem, T: ListingElement>(
        &mut self,
        arg1: Option<&D>,
        arg2: Option<&D>
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        // Fake 16-bit form (`SUB DE,rr`/`SUB HL,rr`): here `arg1` really is
        // `DE`/`HL` itself (see `parse_sub` in cpclib-asm) - dispatch to the
        // fake-instruction expansion before treating `arg1`/`arg2` as the
        // normal 8-bit case's optional-`A,`-prefix/real-operand pair below.
        if arg1.is_some_and(|a| a.is_register_de() || a.is_register_hl()) {
            if let Some(listing) =
                <T as ListingElement>::fake_to_listing_from_access(Mnemonic::Sub, arg1, arg2, None)
            {
                return self.assemble_fake_listing(&listing);
            }
            unreachable!();
        }

        // Normal SUB: `arg1` is the optional explicit `A,` prefix (never
        // affects encoding), `arg2` is the real 8-bit operand. `parse_sub`
        // always populates `arg2` for this case, but fall back to `arg1`
        // defensively rather than erroring if some other construction path
        // ever produces the pre-fix shape (compared value in `arg1`, `arg2`
        // empty).
        let arg = arg2.or(arg1).ok_or_else(|| {
            Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: "SUB requires at least one argument".into()
            })
        })?;

        if arg.is_expression() {
            let exp = arg.get_expression().unwrap();
            {
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let val = self.checked_byte(raw);
                bytes.push(0xD6);
                bytes.push(val);
            }
        }
        else if arg.is_register8() {
            let reg = arg.get_register8().unwrap();
            {
                bytes.push(0b10010000 + (register8_to_code(reg)));
            }
        }
        else if arg.is_indexregister8() {
            let reg = arg.get_indexregister8().unwrap();
            {
                bytes.push(indexed_register16_to_code(reg.complete()));
                bytes.push(0b10010000 + (indexregister8_to_code(reg)));
            }
        }
        else if arg.is_address_in_register16() {
            assert_eq!(arg.get_register16().unwrap(), Register16::Hl);
            {
                bytes.push(0x96);
            }
        }
        else if arg.is_indexregister_with_index() {
            let reg = arg.get_indexregister16().unwrap();
            let idx = arg.get_index().unwrap();

            {
                let val = ({ let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } & 0xFF) as u8;

                bytes.push(indexed_register16_to_code(reg));
                bytes.push(0x96);
                bytes.push(val);
            }
        }
        else {
            // Try fake instruction expansion (Sub with De|Hl + r16)
            if let Some(listing) =
                <T as ListingElement>::fake_to_listing_from_access(Mnemonic::Sub, arg1, arg2, None)
            {
                return self.assemble_fake_listing(&listing);
            }

            unreachable!();
        }

        Ok(bytes)
    }

    pub fn assemble_sbc<D: DataAccessElem, T: ListingElement>(
        &mut self,
        arg1: Option<&D>,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        if arg1.as_ref().map(|arg| arg.is_register_a()).unwrap_or(true) {
            if arg2.is_register8() {
                let reg = arg2.get_register8().unwrap();
                {
                    bytes.push(0b10011000 + register8_to_code(reg));
                }
            }
            else if arg2.is_indexregister8() {
                let reg = arg2.get_indexregister8().unwrap();
                {
                    bytes.push(indexed_register16_to_code(reg.complete()));
                    bytes.push(0b10011000 + indexregister8_to_code(reg));
                }
            }
            else if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                {
                    let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                    let val = self.checked_byte(raw);
                    bytes.push(0xDE);
                    bytes.push(val);
                }
            }
            else if arg2.is_address_in_register16() {
                assert_eq!(arg2.get_register16().unwrap(), Register16::Hl);
                {
                    bytes.push(0x9E);
                }
            }
            else if arg2.is_indexregister_with_index() {
                let reg = arg2.get_indexregister16().unwrap();
                let idx = arg2.get_index().unwrap();
                {
                    bytes.push(indexed_register16_to_code(reg));
                    bytes.push(0x9E);
                    let val = { let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } as u8;
                    bytes.push(val);
                }
            }
            else {
                unreachable!()
            }
        }
        else {
            // Try fake instruction expansion (Sbc with De + r16)
            if let Some(listing) = <T as ListingElement>::fake_to_listing_from_access(
                Mnemonic::Sbc,
                arg1,
                Some(arg2),
                None
            ) {
                return self.assemble_fake_listing(&listing);
            }

            // If not a fake, must be HL + r16
            assert!(arg1.unwrap().is_register_hl());
            assert!(arg2.is_register16());
            let reg = arg2.get_register16().unwrap();
            bytes.push(0xED);
            bytes.push(0b0100_0010 | (register16_to_code_with_sp(reg) << 4));
        }

        Ok(bytes)
    }

    pub fn assemble_shift<D: DataAccessElem, T: ListingElement>(
        &mut self,
        mne: Mnemonic,
        target: &D,
        hidden: Option<&D>
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        if target.is_register8() {
            let reg = target.get_register8().unwrap();
            add_byte(&mut bytes, 0xCB);
            let byte = if mne.is_sla() {
                0b0010_0000
            }
            else if mne.is_sra() {
                0b0010_1000
            }
            else if mne.is_srl() {
                0b0011_1000
            }
            else if mne.is_rlc() {
                0b0000_0000
            }
            else if mne.is_rrc() {
                0b0000_1000
            }
            else if mne.is_rl() {
                0b0001_0000
            }
            else if mne.is_rr() {
                0b0001_1000
            }
            else if mne.is_sl1() {
                0b0011_0000
            }
            else {
                unreachable!()
            } + register8_to_code(reg);
            add_byte(&mut bytes, byte);
        }
        else if target.is_register16() {
            if let Some(listing) =
                <T as ListingElement>::fake_to_listing_from_access(mne, Some(target), None, None)
            {
                return self.assemble_fake_listing(&listing);
            }

            unreachable!();
        }
        else {
            assert!(target.is_address_in_register16() || target.is_indexregister_with_index());

            // add prefix for ix/iy
            if target.is_indexregister_with_index() {
                let reg = target.get_indexregister16().unwrap();
                let idx = target.get_index().unwrap();

                {
                    let val = { let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } as u8;
                    bytes.push(indexed_register16_to_code(reg));
                    add_byte(&mut bytes, 0xCB);
                    bytes.push(val);
                }
            }
            else if target.is_address_in_register16() {
                assert_eq!(target.get_register16().unwrap(), Register16::Hl);
                {
                    add_byte(&mut bytes, 0xCB);
                }
            }
            else {
                return Err(Box::new(AssemblerError::InvalidArgument {
                    msg: format!("{mne} cannot take {target} as argument")
                }));
            };

            // some hidden opcode modify this byte
            let mut byte: u8 = if mne.is_sla() {
                0x26
            }
            else if mne.is_sra() {
                0x2E
            }
            else if mne.is_srl() {
                0x3E
            }
            else if mne.is_rlc() {
                0x06
            }
            else if mne.is_rrc() {
                0x0E
            }
            else if mne.is_rl() {
                0x16
            }
            else if mne.is_rr() {
                0x1E
            }
            else if mne.is_sl1() {
                0x36
            }
            else {
                unreachable!()
            };

            if hidden.is_some() {
                let delta: i8 = match hidden.unwrap().get_register8().unwrap() {
                    Register8::A => 1,
                    Register8::L => -1,
                    Register8::H => -2,
                    Register8::E => -3,
                    Register8::D => -4,
                    Register8::C => -5,
                    Register8::B => -6
                };
                if delta < 0 {
                    byte -= delta.unsigned_abs();
                }
                else {
                    byte += delta as u8;
                }
            }
            bytes.push(byte);
        }

        Ok(bytes)
    }

    /// Assemble SRL8 rr (fake instruction): shifts rr right by 8 bits.
    /// Delegates to fake_to_listing_from_access for consistent expansion logic.
    pub fn assemble_srl8<D: DataAccessElem + Debug, T: ListingElement>(
        &mut self,
        arg: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        // Try fake instruction expansion
        if let Some(listing) = <T as ListingElement>::fake_to_listing_from_access(
            Mnemonic::Srl8,
            Some(arg),
            None,
            None
        ) {
            return self.assemble_fake_listing(&listing);
        }

        unreachable!()
    }

    pub fn assemble_ex_memsp<D: DataAccessElem>(
        &mut self,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>> {
        let mut bytes = Bytes::new();

        if let Some(reg) = arg1.get_indexregister16() {
            bytes.push(indexed_register16_to_code(reg));
        }

        bytes.push(0xE3);
        Ok(bytes)
    }

    pub fn assemble_pop<D: DataAccessElem>(
        &mut self,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>> {
        let mut bytes = Bytes::new();

        if arg1.is_register16() {
            let reg = arg1.get_register16().unwrap();
            let byte = 0b1100_0001 | (register16_to_code_with_af(reg) << 4);
            bytes.push(byte);
        }
        else if arg1.is_indexregister16() {
            let reg = arg1.get_indexregister16().unwrap();
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(0xE1);
        }
        else {
            return Err(Box::new(AssemblerError::InvalidArgument {
                msg: format!("POP: not implemented for {arg1:?}")
            }));
        }

        Ok(bytes)
    }

    pub fn assemble_push<D: DataAccessElem>(
        &mut self,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>> {
        let mut bytes = Bytes::new();

        if arg1.is_register16() {
            let reg = arg1.get_register16().unwrap();
            let byte = 0b1100_0101 | (register16_to_code_with_af(reg) << 4);
            bytes.push(byte);
        }
        else if arg1.is_indexregister16() {
            let reg = arg1.get_indexregister16().unwrap();
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(0xE5);
        }
        else {
            return Err(Box::new(AssemblerError::InvalidArgument {
                msg: format!("PUSH: not implemented for {arg1:?}")
            }));
        }

        Ok(bytes)
    }

    pub fn assemble_inc_dec<D: DataAccessElem>(
        &mut self,
        mne: Mnemonic,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        let is_inc = match mne {
            Mnemonic::Inc => true,
            Mnemonic::Dec => false,
            _ => panic!("Impossible case")
        };

        if arg1.is_register16() {
            let reg = arg1.get_register16().unwrap();
            {
                let base = if is_inc { 0b0000_0011 } else { 0b0000_1011 };
                let byte = base | (register16_to_code(reg) << 4);
                bytes.push(byte);
            }
        }
        else if arg1.is_indexregister16() {
            let reg = arg1.get_indexregister16().unwrap();
            {
                bytes.push(indexed_register16_to_code(reg));
                bytes.push(if is_inc { 0x23 } else { 0x2B });
            }
        }
        else if arg1.is_register8() {
            let reg = arg1.get_register8().unwrap();
            {
                bytes.push(
                    if is_inc { 0b0000_0100 } else { 0b0000_0101 } | (register8_to_code(reg) << 3)
                );
            }
        }
        else if arg1.is_indexregister8() {
            let reg = arg1.get_indexregister8().unwrap();
            {
                bytes.push(indexed_register16_to_code(reg.complete()));
                bytes.push(
                    if is_inc { 0b0000_0100 } else { 0b0000_0101 }
                        | (indexregister8_to_code(reg) << 3)
                );
            }
        }
        else if arg1.is_address_in_register16()
            && arg1.get_register16().unwrap() == Register16::Hl
        {
            {
                bytes.push(if is_inc { 0x34 } else { 0x35 });
            }
        }
        else if arg1.is_indexregister_with_index() {
            let reg = arg1.get_indexregister16().unwrap();
            let idx = arg1.get_index().unwrap();
            {
                let res = self.resolve_index_may_fail_in_first_pass(idx)?;
                let val = (self.int_forward(&res)? & 0xFF) as u8;

                bytes.push(indexed_register16_to_code(reg));
                bytes.push(if is_inc { 0x34 } else { 0x35 });
                bytes.push(val);
            }
        }
        else {
            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!(
                    "{}: not implemented for {:?}",
                    mne.to_string().to_owned(),
                    arg1
                )
            }));
        }
        Ok(bytes)
    }

    pub fn assemble_djnz<D: DataAccessElem>(
        &mut self,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        if let Some(expr) = arg1.get_expression() {
            let mut bytes = Bytes::new();
            let address = { let __r = self.resolve_expr_may_fail_in_first_pass(expr)?; self.int_forward(&__r)? };
            let relative = if expr.is_relative() {
                address as u8
            }
            else {
                self.absolute_to_relative_may_fail_in_first_pass(address, 1 + 1)?
            };
            bytes.push(0x10);
            bytes.push(relative);

            Ok(bytes)
        }
        else {
            unreachable!()
        }
    }

    pub fn assemble_logical_operator<D: DataAccessElem>(
        &mut self,
        mnemonic: Mnemonic,
        arg1: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        let memory_code = || {
            match mnemonic {
                Mnemonic::And => 0xA6,
                Mnemonic::Or => 0xB6,
                Mnemonic::Xor => 0xAE,
                _ => unreachable!()
            }
        };

        if arg1.is_register8() {
            let reg = arg1.get_register8().unwrap();
            {
                let base = match mnemonic {
                    Mnemonic::And => 0b1010_0000,
                    Mnemonic::Or => 0b1011_0000,
                    Mnemonic::Xor => 0b1010_1000,
                    _ => unreachable!()
                };
                bytes.push(base + register8_to_code(reg));
            }
        }
        else if arg1.is_indexregister8() {
            let reg = arg1.get_indexregister8().unwrap();
            {
                bytes.push(indexed_register16_to_code(reg.complete()));
                let base = match mnemonic {
                    Mnemonic::And => 0b1010_0000,
                    Mnemonic::Or => 0b1011_0000,
                    Mnemonic::Xor => 0b1010_1000,
                    _ => unreachable!()
                };
                bytes.push(base + indexregister8_to_code(reg));
            }
        }
        else if arg1.is_expression() {
            let exp = arg1.get_expression().unwrap();

            {
                let base = match mnemonic {
                    Mnemonic::And => 0xE6,
                    Mnemonic::Or => 0xF6,
                    Mnemonic::Xor => 0xEE,
                    _ => unreachable!()
                };
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let value = self.checked_byte(raw);
                bytes.push(base);
                bytes.push(value);
            }
        }
        else if arg1.is_address_in_register16() {
            assert_eq!(arg1.get_register16(), Some(Register16::Hl));

            {
                bytes.push(memory_code());
            }
        }
        else if arg1.is_indexregister_with_index() {
            let reg = arg1.get_indexregister16().unwrap();
            let idx = arg1.get_index().unwrap();

            {
                let value = { let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } & 0xFF;
                bytes.push(indexed_register16_to_code(reg));
                bytes.push(memory_code());
                bytes.push(value as u8);
            }
        }
        else {
            unreachable!()
        }

        Ok(bytes)
    }

    pub fn assemble_add_or_adc<D: DataAccessElem, T: ListingElement>(
        &mut self,
        mnemonic: Mnemonic,
        arg1: Option<&D>,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();
        let is_add = match mnemonic {
            Mnemonic::Add => true,
            Mnemonic::Adc => false,
            _ => panic!("Impossible case")
        };

        if arg1.is_none() || arg1.as_ref().map(|arg1| arg1.is_register_a()).unwrap() {
            if arg2.is_address_in_hl() {
                if is_add {
                    bytes.push(0b1000_0110);
                }
                else {
                    bytes.push(0b1000_1110);
                }
            }
            else if arg2.is_indexregister_with_index() {
                let reg = arg2.get_indexregister16().unwrap();
                let idx = arg2.get_index().unwrap();

                {
                    let val = { let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? };

                    bytes.push(indexed_register16_to_code(reg));
                    if is_add {
                        bytes.push(0b1000_0110);
                    }
                    else {
                        bytes.push(0x8E);
                    }
                    add_index(&mut bytes, val, self)?;
                }
            }
            else if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                {
                    let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                    let val = self.checked_byte(raw);
                    if is_add {
                        bytes.push(0b1100_0110);
                    }
                    else {
                        bytes.push(0xCE);
                    }
                    bytes.push(val);
                }
            }
            else if arg2.is_register8() {
                let reg = arg2.get_register8().unwrap();
                {
                    let base = if is_add { 0b1000_0000 } else { 0b1000_1000 };
                    bytes.push(base | register8_to_code(reg));
                }
            }
            else if arg2.is_indexregister8() {
                let reg = arg2.get_indexregister8().unwrap();

                {
                    bytes.push(indexed_register16_to_code(reg.complete()));
                    let base = if is_add { 0b1000_0000 } else { 0b1000_1000 };
                    bytes.push(base | indexregister8_to_code(reg));
                }
            }
        }
        else if arg1.as_ref().unwrap().is_register_hl() {
            if arg2.is_register16() {
                let reg = arg2.get_register16().unwrap();
                let base = if is_add {
                    0b0000_1001
                }
                else {
                    bytes.push(0xED);
                    0b0100_1010
                };

                bytes.push(base | (register16_to_code_with_sp(reg) << 4));
            }
        }
        else if arg1.as_ref().unwrap().is_indexregister16() {
            let reg1 = arg1.as_ref().unwrap().get_indexregister16().unwrap();
            {
                if arg2.is_register16() {
                    let reg2 = arg2.get_register16().unwrap();
                    {
                        bytes.push(indexed_register16_to_code(reg1));
                        let base = if is_add {
                            0b0000_1001
                        }
                        else {
                            panic!();
                        };
                        bytes.push(
                            base | (register16_to_code_with_indexed(&DataAccess::Register16(reg2))
                                << 4)
                        )
                    }
                }
                else if arg2.is_indexregister16() {
                    let reg2 = arg2.get_indexregister16().unwrap();

                    {
                        if reg1 != reg2 {
                            return Err(Box::new(AssemblerError::InvalidArgument {
                                msg: "Unable to add different indexed registers".into()
                            }));
                        }
                        bytes.push(indexed_register16_to_code(reg1));
                        let base = if is_add {
                            0b0000_1001
                        }
                        else {
                            panic!();
                        };
                        bytes.push(
                            base | (register16_to_code_with_indexed(&DataAccess::IndexRegister16(
                                reg2
                            )) << 4)
                        )
                    }
                }
            }
        }

        // Try fake instruction expansion as a fallback
        if bytes.is_empty() {
            if let Some(listing) =
                <T as ListingElement>::fake_to_listing_from_access(mnemonic, arg1, Some(arg2), None)
            {
                return self.assemble_fake_listing(&listing);
            }

            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("{mnemonic:?} not implemented for {arg1:?} {arg2:?}")
            }));
        }

        Ok(bytes)
    }

    pub fn assemble_in<D: DataAccessElem>(
        &mut self,
        arg1: &D,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        if arg1.is_expression() {
            assert_eq!(
                self.resolve_expr_must_never_fail(arg1.get_expression().unwrap())?,
                ExprResult::from(0)
            );
            assert!(arg2.is_port_c());
            bytes.push(0xED);
            bytes.push(0x70);
        }
        else if arg2.is_port_c() && arg1.is_register8() {
            let reg = arg1.get_register8().unwrap();
            {
                bytes.push(0xED);
                bytes.push(0b0100_0000 | (register8_to_code(reg) << 3))
            }
        }
        else if arg2.is_port_n() {
            let exp = arg2.get_expression().unwrap();
            {
                if arg1.is_register_a() {
                    let val = ({ let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? } & 0xFF) as u8;
                    bytes.push(0xDB);
                    bytes.push(val);
                }
            }
        }

        if bytes.is_empty() {
            Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("IN: not properly implemented for '{arg1:?}, {arg2:?}'")
            }))
        }
        else {
            Ok(bytes)
        }
    }

    pub fn assemble_out<D: DataAccessElem>(
        &mut self,
        arg1: &D,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        if arg2.is_expression() {
            assert_eq!(
                self.resolve_expr_must_never_fail(arg2.get_expression().unwrap())?,
                0.into()
            );
            assert!(arg1.is_port_c());
            bytes.push(0xED);
            bytes.push(0x71);
        }
        else if arg1.is_port_c() {
            if arg2.is_register8() {
                let reg = arg2.get_register8().unwrap();
                bytes.push(0xED);
                bytes.push(0b0100_0001 | (register8_to_code(reg) << 3))
            }
        }
        else if arg1.is_port_n() {
            let exp = arg1.get_expression().unwrap();
            {
                if arg2.is_register_a() {
                    let val = ({ let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? } & 0xFF) as u8;
                    bytes.push(0xD3);
                    bytes.push(val);
                }
            }
        }

        if bytes.is_empty() {
            Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("OUT: not properly implemented for '{arg1:?}, {arg2:?}'")
            }))
        }
        else {
            Ok(bytes)
        }
    }

    pub fn assemble_bit_res_or_set<D: DataAccessElem>(
        &mut self,
        mnemonic: Mnemonic,
        arg1: &D,
        arg2: &D,
        hidden: Option<&Register8>
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt
    {
        let mut bytes = Bytes::new();

        let bit = match arg1.get_expression() {
            Some(e) => {
                let bit = ({ let __r = self.resolve_expr_may_fail_in_first_pass(e)?; self.int_forward(&__r)? } & 0xFF) as u8;
                if bit > 7 {
                    return Err(Box::new(AssemblerError::InvalidArgument {
                        msg: format!("{mnemonic}: {bit} is an invalid value")
                    }));
                }
                bit
            },
            _ => unreachable!()
        };

        let code = match mnemonic {
            Mnemonic::Res => 0b1000_0000,
            Mnemonic::Set => 0b1100_0000,
            Mnemonic::Bit => 0b0100_0000,
            _ => unreachable!()
        };

        if let Some(ref reg) = arg2.get_register8() {
            bytes.push(0xCB);
            bytes.push(code | (bit << 3) | register8_to_code(*reg))
        }
        else {
            assert!(arg2.is_address_in_register16() || arg2.is_indexregister_with_index());
            let mut code = code + 0b0110;

            if arg2.is_indexregister_with_index() {
                let reg = arg2.get_indexregister16().unwrap();
                let idx = arg2.get_index().unwrap();

                bytes.push(indexed_register16_to_code(reg));
                add_byte(&mut bytes, 0xCB);
                let delta = ({ let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } & 0xFF) as u8;
                add_byte(&mut bytes, delta);

                if hidden.is_some() {
                    let fix: i8 = match hidden.unwrap() {
                        Register8::A => 1,
                        Register8::L => -1,
                        Register8::H => -2,
                        Register8::E => -3,
                        Register8::D => -4,
                        Register8::C => -5,
                        Register8::B => -6
                    };
                    if fix < 0 {
                        code -= fix.unsigned_abs();
                    }
                    else {
                        code += fix as u8;
                    }
                }
            }
            else {
                bytes.push(0xCB);
            }

            bytes.push(code | (bit << 3));
        }

        Ok(bytes)
    }

    pub fn assemble_call_jr_or_jp<D: DataAccessElem>(
        &mut self,
        mne: Mnemonic,
        arg1: Option<&D>,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        let is_jr = match mne {
            Mnemonic::Jr => true,
            Mnemonic::Jp | Mnemonic::Call => false,
            _ => unreachable!()
        };

        let is_call = match mne {
            Mnemonic::Call => true,
            Mnemonic::Jp | Mnemonic::Jr => false,
            _ => unreachable!()
        };

        let is_jp = !(is_call || is_jr);

        let flag_code = if let Some(arg1) = arg1 {
            match arg1.get_flag_test() {
                Some(test) => Some(flag_test_to_code(test)),
                _ => {
                    return Err(Box::new(AssemblerError::InvalidArgument {
                        msg: format!(
                            "{}: wrong flag argument",
                            mne.to_string().to_ascii_uppercase()
                        )
                    }));
                }
            }
        }
        else {
            None
        };

        if arg2.is_expression() {
            let e = arg2.get_expression().unwrap();
            let address = { let __r = self.resolve_expr_may_fail_in_first_pass(e)?; self.int_forward(&__r)? };
            if is_jr {
                let relative = if e.is_relative() {
                    address as u8
                }
                else {
                    self.absolute_to_relative_may_fail_in_first_pass(address, 2)?
                };
                if flag_code.is_some() {
                    add_byte(&mut bytes, 0b0010_0000 | (flag_code.unwrap() << 3));
                }
                else {
                    add_byte(&mut bytes, 0b0001_1000);
                }
                add_byte(&mut bytes, relative);
            }
            else if is_call {
                match flag_code {
                    Some(flag) => add_byte(&mut bytes, 0b1100_0100 | (flag << 3)),
                    None => add_byte(&mut bytes, 0xCD)
                }
                add_word(&mut bytes, address as u16);
            }
            else {
                if flag_code.is_some() {
                    add_byte(&mut bytes, 0b1100_0010 | (flag_code.unwrap() << 3))
                }
                else {
                    add_byte(&mut bytes, 0xC3);
                }
                add_word(&mut bytes, address as u16);
            }
        }
        else if arg2.is_address_in_register16() {
            assert_eq!(arg2.get_register16(), Some(Register16::Hl));
            assert!(is_jp);
            add_byte(&mut bytes, 0xE9);
        }
        else if arg2.is_address_in_indexregister16() {
            assert!(is_jp);
            let reg = arg2.get_indexregister16().unwrap();
            add_byte(&mut bytes, indexed_register16_to_code(reg));
            add_byte(&mut bytes, 0xE9);
        }
        else {
            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("{mne}: parameter {arg2:?} not treated")
            }));
        }

        Ok(bytes)
    }

    /// Assemble the `JQ` fake instruction: try a `JR` encoding first, and
    /// fall back to a `JP` encoding only once the target is confirmed too
    /// far for a relative jump - no further reachability analysis. On the
    /// first pass, an unresolved forward reference gets the same optimistic
    /// 0-relative placeholder a real `JR` already gets; it's refined once
    /// addresses are known on a later pass.
    pub fn assemble_jq<D: DataAccessElem>(
        &mut self,
        arg1: Option<&D>,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        let flag_code = if let Some(arg1) = arg1 {
            match arg1.get_flag_test() {
                Some(test) => Some(flag_test_to_code(test)),
                _ => {
                    return Err(Box::new(AssemblerError::InvalidArgument {
                        msg: "JQ: wrong flag argument".to_owned()
                    }));
                }
            }
        }
        else {
            None
        };

        if !arg2.is_expression() {
            return Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("JQ: parameter {arg2:?} not treated")
            }));
        }

        let e = arg2.get_expression().unwrap();
        let address = { let __r = self.resolve_expr_may_fail_in_first_pass(e)?; self.int_forward(&__r)? };

        // A forward-referenced target's resolved value lags one pass behind
        // this instruction's own size decision (the symbol table only holds
        // the value computed the last time the target's definition site was
        // actually visited) - and since JQ's own size depends on that value,
        // a forward JQ can take an extra pass to settle. Keep asking for one
        // more pass until the value we read stops changing from one pass to
        // the next. Once stable, `repr` must still be re-recorded every pass
        // (not just the first time it's seen) so a later pass - possibly
        // triggered for a reason unrelated to this JQ - doesn't see an empty
        // history and spuriously conclude the value is "new" again, asking
        // for pointless extra passes forever.
        if !self.pass.is_first_pass() {
            let pc = self.symbols().current_address().unwrap_or(0);
            let repr = format!("JQ@{pc:x}: target=0x{address:x}");
            if !self.previous_pass_discarded_errors.contains(&repr) {
                *self.request_additional_pass.write().unwrap() = true;
            }
            self.current_pass_discarded_errors.insert(repr);
        }

        let relative = if e.is_relative() {
            Some(address as u8)
        }
        else {
            match absolute_to_relative(address, 2, self.symbols()) {
                Ok(relative) => Some(relative),
                // Confirmed too far for a relative jump - true regardless of
                // pass, including on the first pass for a backward
                // reference (already fully resolved, no placeholder
                // involved). Checked before the first-pass fallback below,
                // which is only for a target that isn't resolvable *at all*
                // yet.
                Err(err) if matches!(*err, AssemblerError::InvalidArgument { .. }) => None,
                Err(_) if self.pass.is_first_pass() => Some(0),
                Err(err) => return Err(err)
            }
        };

        match relative {
            Some(relative) => {
                add_byte(
                    &mut bytes,
                    flag_code.map_or(0b0001_1000, |flag| 0b0010_0000 | (flag << 3))
                );
                add_byte(&mut bytes, relative);
            },
            None => {
                add_byte(
                    &mut bytes,
                    flag_code.map_or(0xC3, |flag| 0b1100_0010 | (flag << 3))
                );
                add_word(&mut bytes, address as u16);
            }
        }

        Ok(bytes)
    }

    pub fn assemble_ld<D: DataAccessElem + Debug, T: ListingElement>(
        &mut self,
        arg1: &D,
        arg2: &D
    ) -> Result<Bytes, Box<AssemblerError>>
    where
        <D as cpclib_tokens::DataAccessElem>::Expr: ExprEvaluationExt + ExprElement
    {
        let mut bytes = Bytes::new();

        // Destination is 8bits register
        if arg1.is_register8() {
            let dst = register8_to_code(arg1.get_register8().unwrap());
            if arg2.is_register8() {
                let src = arg2.get_register8().unwrap();
                let src = register8_to_code(src);
                let code = 0b0100_0000 + (dst << 3) + src;
                bytes.push(code);
            }
            else if arg2.is_indexregister8() {
                let src = arg2.get_indexregister8().unwrap();
                bytes.push(indexed_register16_to_code(src.complete()));
                let src = indexregister8_to_code(src);
                let code = 0b0100_0000 + (dst << 3) + src;
                bytes.push(code);
            }
            else if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let val = self.checked_byte(raw);
                bytes.push(0b0000_0110 | (dst << 3));
                bytes.push(val);
            }
            else if arg2.is_indexregister_with_index() {
                let reg = arg2.get_indexregister16().unwrap();
                let idx = arg2.get_index().unwrap();
                let val = { let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? };
                add_index_register_code(&mut bytes, reg);
                add_byte(&mut bytes, 0b0100_0110 | (dst << 3));
                add_index(&mut bytes, val, self)?;
            }
            else if arg2.is_address_in_register16() {
                match arg2.get_register16().unwrap() {
                    Register16::Hl => {
                        add_byte(&mut bytes, 0b0100_0110 | (dst << 3));
                    },
                    memreg => {
                        assert!(arg1.is_register_a());
                        let byte = match memreg {
                            Register16::Bc => 0x0A,
                            Register16::De => 0x1A,
                            _ => unreachable!()
                        };
                        add_byte(&mut bytes, byte);
                    }
                }
            }
            else if arg2.is_address_in_indexregister16() {
                let reg = arg2.get_indexregister16().unwrap();
                add_index_register_code(&mut bytes, reg);
                add_byte(&mut bytes, 0b0100_0110 | (dst << 3));
            }
            else if arg2.is_memory() {
                // dst is A
                let expr = arg2.get_expression().unwrap();
                let val = { let __r = self.resolve_expr_may_fail_in_first_pass(expr)?; self.int_forward(&__r)? };
                add_byte(&mut bytes, 0x3A);
                add_word(&mut bytes, val as _);
            }
            else if arg2.is_register_i() {
                assert!(arg1.is_register_a());
                bytes.push(0xED);
                bytes.push(0x57);
            }
            else if arg2.is_register_r() {
                assert!(arg1.is_register_a());
                bytes.push(0xED);
                bytes.push(0x5F);
            }
        }
        // Destination is 16 bits register
        else if arg1.is_register16() {
            let dst = arg1.get_register16().unwrap();
            let dst_code = register16_to_code(dst);

            if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let val = self.checked_word(raw);
                add_byte(&mut bytes, 0b0000_0001 | (dst_code << 4));
                add_word(&mut bytes, val);
            }
            else if arg2.is_register_hl() && dst.is_sp() {
                add_byte(&mut bytes, 0xF9);
            }
            else if arg2.is_indexregister16() && dst.is_sp() {
                let reg = arg2.get_indexregister16().unwrap();
                add_byte(&mut bytes, indexed_register16_to_code(reg));
                add_byte(&mut bytes, 0xF9);
            }
            else if arg2.is_memory() {
                let expr = arg2.get_expression().unwrap();
                let val = ({ let __r = self.resolve_expr_may_fail_in_first_pass(expr)?; self.int_forward(&__r)? } & 0xFFFF) as u16;

                if let Register16::Hl = dst {
                    add_byte(&mut bytes, 0x2A);
                    add_word(&mut bytes, val);
                }
                else {
                    add_byte(&mut bytes, 0xED);
                    add_byte(
                        &mut bytes,
                        (register16_to_code_with_sp(dst) << 4) + 0b0100_1011
                    );
                    add_word(&mut bytes, val);
                }
            }
        }
        else if arg1.is_indexregister8() {
            let dst = arg1.get_indexregister8().unwrap();
            add_byte(&mut bytes, indexed_register16_to_code(dst.complete()));

            if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                let val = ({ let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? } & 0xFF) as u8;
                bytes.push(0b0000_0110 | (indexregister8_to_code(dst) << 3));
                bytes.push(val);
            }
            else if arg2.is_register8() {
                let src = arg2.get_register8().unwrap();
                let code = register8_to_code(src);

                let code = if dst.is_high() {
                    0b0110_0000 + code
                }
                else {
                    0x68 + code
                };
                bytes.push(code);
            }
            else if arg2.is_indexregister8() {
                let src = arg2.get_indexregister8().unwrap();
                assert_eq!(dst.complete(), src.complete());

                let byte = match (dst.is_low(), src.is_low()) {
                    (false, false) => 0x64,
                    (false, true) => 0x65,
                    (true, false) => 0x6C,
                    (true, true) => 0x6D
                };
                bytes.push(byte)
            }
        }
        // Destination  is 16 bits indexed register
        else if arg1.is_indexregister16() {
            let dst = arg1.get_indexregister16().unwrap();
            let code = indexed_register16_to_code(dst);

            if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let val = self.checked_word(raw);
                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x21);
                add_word(&mut bytes, val);
            }
            else if arg2.is_memory() {
                let exp = arg2.get_expression().unwrap();

                let val = ({ let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? } & 0xFFFF) as u16;
                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x2A);
                add_word(&mut bytes, val);
            }
        }
        // Destination is memory indexed by register
        else if arg1.is_address_in_register16() {
            let dst = arg1.get_register16().unwrap();
            // Want to store in memory pointed by register
            match dst {
                Register16::Hl => {
                    if arg2.is_register8() {
                        let src = arg2.get_register8().unwrap();
                        let src = register8_to_code(src);
                        let code = 0b0111_0000 | src;
                        bytes.push(code);
                    }
                    else if arg2.is_expression() {
                        let exp = arg2.get_expression().unwrap();
                        let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                        let val = self.checked_byte(raw);
                        bytes.push(0x36);
                        bytes.push(val);
                    }
                },

                Register16::De if arg2.is_register_a() => {
                    bytes.push(0b0001_0010);
                },

                Register16::Bc if arg2.is_register_a() => {
                    bytes.push(0b0000_0010);
                },

                _ => {}
            }
        }
        else if arg1.is_address_in_indexregister16() {
            let dst = arg1.get_indexregister16().unwrap();
            add_index_register_code(&mut bytes, dst);

            if arg2.is_register8() {
                let src = arg2.get_register8().unwrap();
                let src = register8_to_code(src);
                let code = 0b0111_0000 | src;
                bytes.push(code);
                bytes.push(0);
            }
            else if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let val = self.checked_byte(raw);
                bytes.push(0x36);
                bytes.push(val);
            }
        }
        // Destination is memory form ix/iy + n
        else if arg1.is_indexregister_with_index() {
            let reg = arg1.get_indexregister16().unwrap();
            let idx = arg1.get_index().unwrap();

            if arg2.is_expression() {
                let exp = arg2.get_expression().unwrap();
                let raw = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };
                let value = self.checked_byte(raw);
                add_byte(&mut bytes, indexed_register16_to_code(reg));
                let delta = ({ let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } & 0xFF) as u8;
                add_byte(&mut bytes, 0x36);
                add_byte(&mut bytes, delta);
                add_byte(&mut bytes, value);
            }
            else if arg2.is_register8() {
                let src = arg2.get_register8().unwrap();
                add_byte(&mut bytes, indexed_register16_to_code(reg));
                let delta = ({ let __r = self.resolve_index_may_fail_in_first_pass(idx)?; self.int_forward(&__r)? } & 0xFF) as u8;
                add_byte(&mut bytes, 0x70 + register8_to_code(src));
                add_byte(&mut bytes, delta);
            }
        }
        // Destination is memory
        else if arg1.is_memory() {
            let exp = arg1.get_expression().unwrap();
            let address = { let __r = self.resolve_expr_may_fail_in_first_pass(exp)?; self.int_forward(&__r)? };

            if arg2.is_indexregister16() {
                match arg2.get_indexregister16().unwrap() {
                    IndexRegister16::Ix => {
                        bytes.push(DD);
                        bytes.push(0b0010_0010);
                        add_word(&mut bytes, address as _);
                    },
                    IndexRegister16::Iy => {
                        bytes.push(FD);
                        bytes.push(0b0010_0010);
                        add_word(&mut bytes, address as _);
                    }
                }
            }
            else if arg2.is_register_hl() {
                bytes.push(0b0010_0010);
                add_word(&mut bytes, address as _);
            }
            else if arg2.is_register16() {
                let reg = arg2.get_register16().unwrap();
                bytes.push(0xED);
                bytes.push(0b0100_0011 | (register16_to_code(reg) << 4));
                add_word(&mut bytes, address as _);
            }
            else if arg2.is_register_a() {
                bytes.push(0x32);
                add_word(&mut bytes, address as _);
            }
        }
        else if arg1.is_register_i() {
            assert!(arg2.is_register_a());
            bytes.push(0xED);
            bytes.push(0x47)
        }
        else if arg1.is_register_r() {
            assert!(arg2.is_register_a());
            bytes.push(0xED);
            bytes.push(0x4F)
        }

        // handle fake instructions
        if bytes.is_empty()
            && let Some(listing) = <T as ListingElement>::fake_to_listing_from_access(
                Mnemonic::Ld,
                Some(arg1),
                Some(arg2),
                None
            )
        {
            return self.assemble_fake_listing(&listing);
        }

        if bytes.is_empty() {
            Err(Box::new(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("LD: not properly implemented for '{arg1:?}, {arg2:?}'")
            }))
        }
        else {
            Ok(bytes)
        }
    }

    /// Human-readable "how we got here" notes for `self.active_frames`,
    /// innermost first - flattened to plain owned text (via
    /// `macro_chain_note`/`include_chain_note`) so it is safe to keep around
    /// after the `Z80Span`s in `active_frames` stop being valid (see
    /// `Env::symbol_definition_chains`).
    fn active_frames_as_notes(&self) -> Vec<String> {
        self.active_frames
            .iter()
            .rev()
            .map(|frame| {
                match frame {
                    ActiveFrame::Expansion(expansion) => {
                        macro_chain_note(&expansion.name, &expansion.location, &expansion.call_site)
                    },
                    ActiveFrame::Include(include) => include_chain_note(&include.call_site)
                }
            })
            .collect()
    }

    pub fn visit_assert<E: ExprEvaluationExt + ExprElement>(
        &mut self,
        exp: &E,
        txt: Option<&Vec<FormattedExpr>>,
        span: Option<&Z80Span>
    ) -> Result<bool, Box<AssemblerError>>
    where
        <E as cpclib_tokens::ExprElement>::Expr:
            crate::implementation::expression::ExprEvaluationExt
    {
        if let Some(commands) = self.assembling_control_current_output_commands.last_mut() {
            commands.store_assert(exp.to_expr().into_owned(), txt.cloned(), span.cloned());
        }

        let res = match self.resolve_expr_must_never_fail(exp) {
            Err(e) => Err(e),

            Ok(value) => {
                if !value.bool()? {
                    Err(Box::new(AssemblerError::AssertionFailed {
                        msg: (if txt.is_some() {
                            self.prepropress_string_formatted_expression(txt.unwrap())?
                                .to_string()
                        }
                        else {
                            "".to_owned()
                        }),
                        test: exp.to_string(),
                        guidance: self.to_assert_string(exp)
                    }))
                }
                else {
                    Ok(())
                }
            },
        };

        if let Err(assert_error) = res {
            let assert_error = if let Some(span) = span {
                assert_error.locate(span.clone())
            }
            else {
                *assert_error
            };

            // An `assert` never propagates as an `Err` (so every assert in a
            // file can be collected in one run), so it never goes through
            // the wrapping `ProcessedToken::visited`'s `MacroCallOrBuildStruct`/
            // `Include` arms apply automatically to a macro-body/included-file
            // error that *does* propagate. Do it by hand here, innermost
            // frame first: a macro call gets a whole extra codespan block
            // (its call arguments are worth seeing - the same reason
            // `MacroCallOrBuildStruct` gets one for a propagated `Err`), an
            // `INCLUDE` gets a compact trailing note instead (there's
            // nothing but the path to see in an `INCLUDE "..."` line the
            // note doesn't already say - see `WithChainNotes`).
            let mut assert_error = Box::new(assert_error);
            for frame in self.active_frames.iter().rev() {
                assert_error = match frame {
                    ActiveFrame::Expansion(expansion) => {
                        Box::new(AssemblerError::RelocatedError {
                            span: expansion.call_site.clone(),
                            error: Box::new(AssemblerError::MacroError {
                                name: expansion.name.clone(),
                                root: assert_error,
                                location: expansion.location.clone()
                            })
                        })
                    },
                    ActiveFrame::Include(include) => {
                        assert_error.with_chain_note(include_chain_note(&include.call_site))
                    }
                };
            }

            // Keep whichever macro/struct-expansion buffer(s) `assert_error`'s
            // spans point into alive for as long as this command lives (i.e.
            // as long as `self` lives) - see `Env::active_frames` and
            // `FailedAssertCommand`. `Include` frames need no such keep-alive
            // (see `IncludeFrame`'s doc comment), so only `Expansion` frames
            // contribute here.
            let keep_alive = self
                .active_frames
                .iter()
                .filter_map(|frame| {
                    match frame {
                        ActiveFrame::Expansion(expansion) => Some(expansion.listing.clone()),
                        ActiveFrame::Include(_) => None
                    }
                })
                .collect();
            self.active_page_info_mut()
                .add_failed_assert_command(FailedAssertCommand {
                    failure: assert_error,
                    _keep_alive: keep_alive
                });
            Ok(false)
        }
        else {
            Ok(true)
        }
    }

    pub fn visit_stableticker<S: AsRef<str>>(
        &mut self,
        stable: &StableTickerAction<S>
    ) -> Result<(), Box<AssemblerError>> {
        match stable {
            StableTickerAction::Start(name) => {
                let expanded = self
                    .symbols()
                    .extend_local_and_patterns_for_symbol(name.as_ref())?;
                self.stable_counters.add_counter(&expanded)?;
                Ok(())
            },
            StableTickerAction::Stop(stop) => {
                let release_result = if let Some(stop) = stop {
                    let expanded = self
                        .symbols()
                        .extend_local_and_patterns_for_symbol(stop.as_ref())?;
                    self.stable_counters.release_counter(expanded.value())
                }
                else {
                    self.stable_counters.release_last_counter()
                };
                if let Some((label, count)) = release_result {
                    if !self.pass.is_listing_pass()
                        && self.symbols().contains_symbol(&label)?
                        && self.symbols().int_value(&label).unwrap().unwrap() != count as i32
                    {
                        self.add_warning(Box::new(AssemblerWarning::AlreadyRenderedError(
                            format!("Symbol {label} has been overwritten")
                        )));
                    }

                    // force the injection of the value
                    self.symbols_mut()
                        .set_symbol_to_value(label, Value::from(count))?;
                    Ok(())
                }
                else {
                    Err(Box::new(AssemblerError::NoActiveCounter))
                }
            }
        }
    }
}

// Removed: now implemented as a method on Env

fn indexed_register16_to_code(reg: IndexRegister16) -> u8 {
    match reg {
        IndexRegister16::Ix => DD,
        IndexRegister16::Iy => FD
    }
}

/// Return the code that represents a 8bits register.
/// A: 0b111
/// B: 0b000
/// C: 0b001
/// D: 0b010
/// E: 0b011
/// H: 0b100
/// L: 0b101
#[inline]
fn register8_to_code(reg: Register8) -> u8 {
    match reg {
        Register8::A => 0b111,
        Register8::B => 0b000,
        Register8::C => 0b001,
        Register8::D => 0b010,
        Register8::E => 0b011,
        Register8::H => 0b100,
        Register8::L => 0b101
    }
}

#[inline]
fn indexregister8_to_code(reg: IndexRegister8) -> u8 {
    match reg {
        IndexRegister8::Ixh | IndexRegister8::Iyh => register8_to_code(Register8::H),
        IndexRegister8::Ixl | IndexRegister8::Iyl => register8_to_code(Register8::L)
    }
}

/// Return the code that represents a 16 bits register
fn register16_to_code_with_af(reg: Register16) -> u8 {
    match reg {
        Register16::Bc => 0b00,
        Register16::De => 0b01,
        Register16::Hl => 0b10,
        Register16::Af => 0b11,
        _ => panic!("no mapping for {reg:?}")
    }
}

fn register16_to_code_with_sp(reg: Register16) -> u8 {
    match reg {
        Register16::Bc => 0b00,
        Register16::De => 0b01,
        Register16::Hl => 0b10,
        Register16::Sp => 0b11,
        _ => panic!("no mapping for {reg:?}")
    }
}

fn register16_to_code(reg: Register16) -> u8 {
    match reg {
        Register16::Bc => 0b00,
        Register16::De => 0b01,
        Register16::Hl => 0b10,
        Register16::Sp | Register16::Af => 0b11
    }
}

fn register16_to_code_with_indexed<D: DataAccessElem>(reg: &D) -> u8 {
    if reg.is_register_bc() {
        0b00
    }
    else if reg.is_register_de() {
        0b01
    }
    else if reg.is_indexregister16() {
        0b10
    }
    else if reg.is_register_sp() {
        0b11
    }
    else {
        panic!("no mapping for {reg:?}")
    }
}

fn flag_test_to_code(flag: FlagTest) -> u8 {
    match flag {
        FlagTest::NZ => 0b000,
        FlagTest::Z => 0b001,
        FlagTest::NC => 0b010,
        FlagTest::C => 0b011,

        // the following flags are not used for jr
        FlagTest::PO => 0b100,
        FlagTest::PE => 0b101,
        FlagTest::P => 0b110,
        FlagTest::M => 0b111
    }
}

// All these tests are deactivated because there are too many compilation issues at the moment
#[cfg(test_to_clean)]
#[allow(deprecated)]
mod test {

    use super::processed_token::build_processed_token;
    use super::*;

    fn visit_token(token: &Token, env: &mut Env) -> Result<(), Box<AssemblerError>> {
        let mut processed =
            build_processed_token(token, std::sync::Arc::new(std::sync::RwLock::new(env)))?;
        processed.visited(env)
    }

    fn visit_tokens(tokens: &[Token]) -> Result<Env, Box<AssemblerError>> {
        let mut env = Env::default();
        for t in tokens {
            visit_token(t, &mut env)?;
        }
        Ok(env)
    }

    #[test]
    pub fn test_inc_b() {
        let mut env = Env::default();
        let res = assemble_inc_dec(
            Mnemonic::Inc,
            &DataAccess::Register8(Register8::B),
            &mut env
        )
        .unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x04);
    }

    #[test]
    pub fn test_pop() {
        let res = assemble_pop(&DataAccess::Register16(Register16::Af)).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0b1111_0001);
    }

    #[test]
    fn test_jump() {
        let mut env = Env::default();
        let res = env
            .assemble_call_jr_or_jp(
                Mnemonic::Jp,
                Some(&DataAccess::FlagTest(FlagTest::Z)),
                &DataAccess::Expression(Expr::Value(0x1234))
            )
            .unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], 0b1100_1010);
        assert_eq!(res[1], 0x34);
        assert_eq!(res[2], 0x12);
    }

    #[test]
    pub fn test_assert() {
        let mut env = Env::default();
        env.start_new_pass();

        assert!(
            env.visit_assert(
                &Expr::BinaryOperation(
                    BinaryOperation::Equal,
                    Box::new(0i32.into()),
                    Box::new(0i32.into())
                ),
                None,
                None
            )
            .unwrap()
        );
        assert!(
            !env.visit_assert(
                &Expr::BinaryOperation(
                    BinaryOperation::Equal,
                    Box::new(1i32.into()),
                    Box::new(0i32.into())
                ),
                None,
                &mut env,
                None
            )
            .unwrap()
        );
    }

    #[test]
    pub fn test_undef() {
        let mut env = Env::default();
        env.start_new_pass();

        env.visit_label("toto").unwrap();
        assert!(env.symbols().contains_symbol("toto").unwrap());
        env.visit_undef("toto").unwrap();
        assert!(!env.symbols().contains_symbol("toto").unwrap());
        assert!(env.visit_undef("toto").is_err());
    }

    #[test]
    pub fn test_inc_dec() {
        let mut env = Env::default();
        let res = env
            .assemble_inc_dec(Mnemonic::Inc, &DataAccess::Register16(Register16::De))
            .unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x13);

        let res = env
            .assemble_inc_dec(Mnemonic::Dec, &DataAccess::Register8(Register8::B))
            .unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x05);
    }

    #[test]
    pub fn test_res() {
        let mut env = Env::default();
        let res = env
            .assemble_bit_res_or_set(
                Mnemonic::Res,
                &DataAccess::Expression(0.into()),
                &DataAccess::Register8(Register8::B),
                None
            )
            .unwrap();

        assert_eq!(res.as_ref(), &[0xCB, 0b10000000]);

        let mut env = Env::default();
        let res = env
            .assemble_bit_res_or_set(
                Mnemonic::Res,
                &DataAccess::Expression(2.into()),
                &DataAccess::Register8(Register8::C),
                None
            )
            .unwrap();

        assert_eq!(res.as_ref(), &[0xCB, 0b10010001]);

        let mut env = Env::default();
        let res = env
            .assemble_bit_res_or_set(
                Mnemonic::Res,
                &DataAccess::Expression(2.into()),
                &DataAccess::MemoryRegister16(Register16::Hl),
                None
            )
            .unwrap();

        assert_eq!(res.as_ref(), &[0xCB, 0b10010110]);

        let mut env = Env::default();
        let res = env
            .assemble_bit_res_or_set(
                Mnemonic::Res,
                &DataAccess::Expression(2.into()),
                &DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, 3.into()),
                None
            )
            .unwrap();

        assert_eq!(res.as_ref(), &[DD, 0xCB, 3, 0b10010110]);

        let mut env = Env::default();
        let res = env
            .assemble_bit_res_or_set(
                Mnemonic::Res,
                &DataAccess::Expression(2.into()),
                &DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, 3.into()),
                Some(&Register8::B)
            )
            .unwrap();

        assert_eq!(res.as_ref(), &[DD, 0xCB, 3, 0b10010000]);
    }

    #[test]
    pub fn test_ld() {
        let mut env = Env::default();
        let res = env
            .assemble_ld(
                &DataAccess::Register16(Register16::De),
                &DataAccess::Expression(Expr::Value(0x1234))
            )
            .unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], 0x11);
        assert_eq!(res[1], 0x34);
        assert_eq!(res[2], 0x12);
    }

    #[test]
    #[should_panic]
    pub fn test_ld_fail() {
        let _res = Env::default()
            .assemble_ld(
                &DataAccess::Register16(Register16::Af),
                &DataAccess::Expression(Expr::Value(0x1234))
            )
            .unwrap();
    }

    #[test]
    pub fn test_ld_r16_r16() {
        let res = assemble_ld(
            &DataAccess::Register16(Register16::De),
            &DataAccess::Register16(Register16::Hl),
            &Env::default()
        )
        .unwrap();
        assert_eq!(res.len(), 2);
    }

    #[test]
    pub fn test_repeat() {
        let tokens = vec![
            Token::Org(0.into(), None),
            Token::Repeat(
                10.into(),
                vec![Token::OpCode(Mnemonic::Nop, None, None, None)].into(),
                None,
                None
            ),
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 10);
    }

    #[test]
    pub fn test_double_repeat() {
        let tokens = vec![
            Token::Org(0.into(), None),
            Token::Repeat(
                10.into(),
                vec![Token::Repeat(
                    10.into(),
                    vec![Token::OpCode(Mnemonic::Nop, None, None, None)].into(),
                    None,
                    None
                )]
                .into(),
                None,
                None
            ),
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 100);
    }

    #[test]
    pub fn test_assemble_logical_operator() {
        let operators = [Mnemonic::And, Mnemonic::Or, Mnemonic::Xor];
        let operands = [
            DataAccess::Register8(Register8::A),
            DataAccess::Expression(0.into()),
            DataAccess::MemoryRegister16(Register16::Hl),
            DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, 2.into())
        ];

        for operator in &operators {
            for operand in &operands {
                let token = Token::OpCode(*operator, Some(operand.clone()), None, None);
                visit_tokens(&[token]).unwrap();
            }
        }
    }

    #[test]
    pub fn test_count() {
        let tokens = vec![
            Token::Org(0.into(), None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
            Token::OpCode(Mnemonic::Nop, None, None, None),
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 10);
    }

    #[test]
    pub fn test_stableticker() {
        let tokens = vec![
            Token::StableTicker(StableTickerAction::Start("myticker".into())),
            Token::OpCode(
                Mnemonic::Inc,
                Some(DataAccess::Register16(Register16::Hl)),
                None,
                None
            ),
            Token::StableTicker(StableTickerAction::Stop),
        ];

        let env = visit_tokens(&tokens);
        assert!(env.is_ok());
        let env = env.unwrap();

        let val = env.symbols().int_value("myticker");
        assert_eq!(val.unwrap().unwrap(), 2);
    }

    #[test]
    pub fn basic_no_variable() {
        let tokens = vec![Token::Basic(None, None, "10 PRINT &DEAD".to_owned())];

        let env = visit_tokens(&tokens);
        println!("{:?}", env);
        assert!(env.is_ok());
    }

    #[test]
    pub fn basic_variable_unset() {
        let tokens = vec![Token::Basic(
            Some(vec!["STUFF".into()]),
            None,
            "10 PRINT {STUFF}".to_owned()
        )];

        let env = visit_tokens(&tokens);
        println!("{:?}", env);
        assert!(env.is_err());
    }

    #[test]
    pub fn basic_variable_set() {
        let tokens = vec![
            Token::Label("STUFF".into()),
            Token::Basic(Some(vec!["STUFF".into()]), None, "10 PRINT {STUFF}".into()),
        ];

        let env = visit_tokens(&tokens);
        println!("{:?}", env);
        assert!(env.is_ok());
    }

    #[test]
    pub fn test_duration() {
        let tokens = vec![Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(Expr::UnaryTokenOperation(
                UnaryTokenOperation::Duration,
                Box::new(Token::OpCode(
                    Mnemonic::Inc,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None,
                    None
                ))
            ))),
            None
        )];

        let env = visit_tokens(&tokens);
        assert!(env.is_ok());
        let env = env.unwrap();
        let bytes = env.memory(0, 2);
        assert_eq!(bytes[1], 2);
    }

    #[test]
    pub fn test_opcode() {
        let tokens = vec![Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(Expr::UnaryTokenOperation(
                UnaryTokenOperation::Opcode,
                Box::new(Token::OpCode(
                    Mnemonic::Inc,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None,
                    None
                ))
            ))),
            None
        )];

        let env = visit_tokens(&tokens);
        assert!(env.is_ok());
        let mut env = env.unwrap();
        let bytes = env.memory(0, 2);
        assert_eq!(
            bytes[1],
            env.assemble_inc_dec(Mnemonic::Inc, &DataAccess::Register16(Register16::Hl))
                .unwrap()[0]
        );
    }

    #[test]
    pub fn test_bytes() {
        let mut m = Bytes::new();

        add_byte(&mut m, 2);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0], 2);

        add_word(&mut m, 0x1234);
        assert_eq!(m.len(), 3);
        assert_eq!(m[1], 0x34);
        assert_eq!(m[2], 0x12);
    }

    #[test]
    pub fn test_labels() {
        let mut env = Env::default();
        let res = visit_token(&Token::Org(0x4000.into(), None), &mut env);
        assert!(res.is_ok());
        assert!(!env.symbols().contains_symbol("hello").unwrap());
        let res = visit_token(&Token::Label("hello".into()), &mut env);
        assert!(res.is_ok());
        assert!(env.symbols().contains_symbol("hello").unwrap());
        assert_eq!(env.symbols().int_value("hello").unwrap(), 0x4000.into());
    }

    #[test]
    pub fn test_jr() {
        let res = dbg!(visit_tokens_all_passes(
            &[
                Token::Org(0x4000.into(), None),
                Token::OpCode(
                    Mnemonic::Jr,
                    None,
                    Some(DataAccess::Expression(Expr::Label("$".into()))),
                    None,
                ),
            ],
            ctx()
        ));

        assert!(res.is_ok());
        let env = res.unwrap();

        assert_eq!(
            env.memory(0x4000, 2),
            &[0x18, 0u8.wrapping_sub(1).wrapping_sub(1)]
        );
    }

    /// Check if  label already exists
    #[test]
    pub fn label_exists() {
        let res = visit_tokens_all_passes(
            &[
                Token::Org(0x4000.into(), None),
                Token::Label("hello".into()),
                Token::Label("hello".into())
            ],
            ctx()
        );
        assert!(res.is_err());
    }

    #[test]
    pub fn test_rorg() {
        let res = visit_tokens_all_passes(
            &[
                Token::Org(0x4000i32.into(), None),
                Token::Rorg(
                    0x8000i32.into(),
                    vec![Token::Defb(vec![Expr::Label("$".into())])].into()
                )
            ],
            ctx()
        );
        assert!(res.is_ok());
    }

    #[test]
    pub fn test_two_passes() {
        let tokens = vec![
            Token::Org(0x123i32.into(), None),
            Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register16(Register16::Hl)),
                Some(DataAccess::Expression(Expr::Label("test".into()))),
                None
            ),
            Token::Label("test".into()),
        ];
        let env = visit_tokens(&tokens);
        assert!(env.is_err());

        let env = visit_tokens_all_passes(&tokens, ctx());
        assert!(env.is_ok());
        let env = env.ok().unwrap();

        let count = env.size();
        assert_eq!(count, 3);

        assert_eq!(
            env.symbols()
                .int_value(&"test".to_owned())
                .unwrap()
                .unwrap(),
            0x123 + 3
        );
        let buffer = env.memory(0x123, 3);
        assert_eq!(buffer[1], 0x23 + 3);
        assert_eq!(buffer[2], 0x1);
    }

    #[test]
    fn test_read_bytes() {
        let tokens = vec![
            Token::Org(0x100.into(), None),
            Token::Defb(vec![1.into(), 2.into()]),
            Token::Defb(vec![3.into(), 4.into()]),
        ];

        let env = visit_tokens(&tokens).unwrap();
        let bytes = env.memory(0x100, 4);
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    pub fn test_undocumented_rlc() {
        let res = visit_tokens_all_passes(
            &[
                Token::Org(0x100.into(), None),
                Token::OpCode(
                    Mnemonic::Rlc,
                    Some(DataAccess::IndexRegister16WithIndex(
                        IndexRegister16::Iy,
                        2.into()
                    )),
                    Some(DataAccess::Register8(Register8::C)),
                    None
                )
            ],
            ctx()
        );
        assert!(res.is_ok());
        let env = res.unwrap();
        let bytes = env.memory(0x100, 4);
        assert_eq!(bytes, vec![0xFD, 0xCB, 0x2, 0x1]);
    }

    #[test]
    pub fn test_undocumented_res() {
        // normal case
        let res = visit_tokens_all_passes(
            &[
                Token::Org(0x100.into(), None),
                Token::OpCode(
                    Mnemonic::Res,
                    Some(DataAccess::Expression(4.into())),
                    Some(DataAccess::MemoryRegister16(Register16::Hl)),
                    None
                )
            ],
            ctx()
        );
        assert!(res.is_ok());
        let env = res.unwrap();
        let bytes = env.memory(0x100, 2);
        assert_eq!(bytes, vec![0xCB, 0xA6]);

        let res = visit_tokens_one_pass(
            &[
                Token::Org(0x100.into(), None),
                Token::OpCode(
                    Mnemonic::Res,
                    Some(DataAccess::Expression(4.into())),
                    Some(DataAccess::IndexRegister16WithIndex(
                        IndexRegister16::Iy,
                        2.into()
                    )),
                    Some(Register8::A)
                )
            ],
            ctx()
        );
        assert!(res.is_ok());
        let env = res.unwrap();
        let bytes = env.memory(0x100, 4);
        assert_eq!(bytes, vec![0xFD, 0xCB, 0x2, 0xA7]);
    }
}

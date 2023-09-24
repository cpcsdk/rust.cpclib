pub mod delayed_command;
pub mod file;
pub mod function;
pub mod list;
pub mod listing_output;
pub mod r#macro;
pub mod matrix;
pub mod page_info;
pub mod report;
pub mod save_command;
pub mod section;
pub mod stable_ticker;
pub mod symbols_output;

pub mod embedded;
pub mod processed_token;

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use cpclib_basic::*;
use cpclib_common::bitvec::prelude::BitVec;
use cpclib_common::itertools::Itertools;
use cpclib_common::lazy_static::__Deref;
#[cfg(not(target_arch = "wasm32"))]
use cpclib_common::rayon::prelude::*;
use cpclib_common::smallvec::SmallVec;
use cpclib_sna::*;
use cpclib_tokens::ToSimpleToken;

use self::function::{Function, FunctionBuilder, HardCodedFunction};
use self::listing_output::*;
use self::processed_token::ProcessedToken;
use self::report::SavedFile;
use self::symbols_output::{SymbolOutputGenerator, SymbolOutputFormat};
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
use crate::{AssemblingOptions, PhysicalAddress};
use cpclib_common::smol_str::SmolStr;

/// Use smallvec to put stuff on the stack not the heap and (hope so) speed up assembling
const MAX_SIZE: usize = 4;
const REPEAT_START_VALUE: i32 = 1;
const MMR_PAGES_SELECTION: [u8; 9] = [
    0xC0,
    0b11_000_0_01,
    0b11_001_0_01,
    0b11_010_0_01,
    0b11_011_0_01,
    0b11_100_0_01,
    0b11_101_0_01,
    0b11_110_0_01,
    0b11_111_0_01
];

#[allow(missing_docs)]
pub type Bytes = SmallVec<[u8; MAX_SIZE]>;

#[derive(Clone, Default, Debug)]
pub struct EnvOptions {
    parse: ParserOptions,
    assemble: AssemblingOptions
}

impl From<AssemblingOptions> for EnvOptions {
    fn from(opt: AssemblingOptions) -> EnvOptions {
        EnvOptions {
            parse: ParserOptions::default(),
            assemble: opt
        }
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
        }
    }

    pub fn new(parse: ParserOptions, assemble: AssemblingOptions) -> Self {
        Self { parse, assemble }
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

/// Add the encoding of an indexed structure
fn add_index(m: &mut Bytes, idx: i32) -> Result<(), AssemblerError> {
    if idx < -127 || idx > 128 {
        // Err(format!("Index error {}", idx).into())
        eprintln!("Index error {}", idx);
    }
    // else {
    let val = (idx & 0xFF) as u8;
    add_byte(m, val);
    Ok(())
    // }
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

trait MyDefault {
    fn default() -> Self;
}

/// ! Lots of things will probably be inspired from RASM
type Bank = [u8; 0x1_0000];
impl MyDefault for Bank {
    fn default() -> Bank {
        [0; 0x1_0000]
    }
}

/// Number of banks allowed in a snapshot
const NB_BANKS: usize = 9;

/// The Banks of interest
/// only one is added at first. Others are created on demand
type Banks = Vec<Bank>;
impl MyDefault for Banks {
    fn default() -> Self {
        vec![Bank::default()]
    }
}

/// Several passes are needed to properly assemble a source file.
/// This structure allows to code which pass is going to be analysed.
/// First pass consists in collecting the various labels to manipulate and so on. Some labels stay unknown at this moment.
/// Second pass serves to get the final values
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssemblingPass {
    Uninitialized,
    FirstPass,
    SecondPass, // and subsequent
    Finished,
    ListingPass // pass dedicated to the listing production
}
impl fmt::Display for AssemblingPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = match self {
            AssemblingPass::Uninitialized => "Uninitialized",
            AssemblingPass::FirstPass => "1",
            AssemblingPass::SecondPass => "2",
            AssemblingPass::Finished => "Finished",
            AssemblingPass::ListingPass => "Listing"
        };
        write!(f, "{}", content)
    }
}

#[allow(missing_docs)]
#[allow(unused)]
impl AssemblingPass {
    fn is_uninitialized(self) -> bool {
        match self {
            AssemblingPass::Uninitialized => true,
            _ => false
        }
    }

    pub fn is_finished(self) -> bool {
        match self {
            AssemblingPass::Finished => true,
            _ => false
        }
    }

    pub fn is_first_pass(self) -> bool {
        match self {
            AssemblingPass::FirstPass => true,
            _ => false
        }
    }

    pub fn is_second_pass(self) -> bool {
        match self {
            AssemblingPass::SecondPass => true,
            _ => false
        }
    }

    pub fn is_listing_pass(self) -> bool {
        match self {
            AssemblingPass::ListingPass => true,
            _ => false
        }
    }

    fn next_pass(self) -> Self {
        match self {
            AssemblingPass::Uninitialized => AssemblingPass::FirstPass,
            AssemblingPass::FirstPass => AssemblingPass::SecondPass,
            AssemblingPass::SecondPass => AssemblingPass::Finished,
            AssemblingPass::Finished | AssemblingPass::ListingPass => panic!()
        }
    }
}

/// Trait to implement for each type of token.
/// it allows to drive the appropriate data vonversion
pub trait Visited {
    /// Make all the necessary for the given token
    fn visited(&self, env: &mut Env) -> Result<(), AssemblerError>;
}

impl Visited for Token {
    fn visited(&self, env: &mut Env) -> Result<(), AssemblerError> {
        visit_token(self, env)
    }
}

impl Visited for LocatedToken {
    fn visited(&self, env: &mut Env) -> Result<(), AssemblerError> {
        // dbg!(env.output_address, self.as_token());
        visit_located_token(self, env).map_err(|e| e.locate(self.span().clone()))
    }
}

type AssemblerWarning = AssemblerError;

/// Store all the necessary information when handling a crunched section
#[derive(Clone)]
struct CrunchedSectionState {
    /// Start of the crunched section for code assembled from the sources.
    /// None for code assembled from tokens
    // mainly usefull for error messages; nothing more
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

    pub fn update(&mut self, spec: &CharsetFormat, env: &Env) -> Result<(), AssemblerError> {
        match spec {
            CharsetFormat::Reset => self.reset(),
            CharsetFormat::CharsList(l, s) => {
                let mut s = env.resolve_expr_must_never_fail(s)?.int()?;
                for c in l.iter() {
                    self.lut.insert(*c, s);
                    s += 1;
                }
            }
            CharsetFormat::Char(c, i) => {
                let c = env.resolve_expr_must_never_fail(c)?.char()?;
                let i = env.resolve_expr_must_never_fail(i)?.int()?;
                self.lut.insert(c, i);
            }
            CharsetFormat::Interval(a, b, s) => {
                let a = env.resolve_expr_must_never_fail(a)?.char()?;
                let b = env.resolve_expr_must_never_fail(b)?.char()?;
                let mut s = env.resolve_expr_must_never_fail(s)?.int()?;
                for c in a..=b {
                    self.lut.insert(c, s);
                    s += 1;
                }
            }
        }

        Ok(())
    }

    pub fn transform_char(&self, c: char) -> u8 {
        self.lut.get(&c).cloned().unwrap_or(c as i32) as _
    }

    pub fn transform_string(&self, s: &str) -> Vec<u8> {
        s.chars().map(|c| self.transform_char(c)).collect_vec()
    }
}

/// Environment of the assembly
#[allow(missing_docs)]
pub struct Env {
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

    /// Ensemble of pages (2 for a stock CPC) for the snapshot
    pages_info_sna: Vec<PageInformation>,

    /// Memory configuration is controlled by the underlying snapshot.
    /// It will ease the generation of snapshots but may complexify the generation of files
    sna: cpclib_sna::Snapshot,
    // TODO remove it as it is store within the sna
    sna_version: cpclib_sna::SnapshotVersion,

    /// List of banks (temporary memory)
    banks: Vec<(Bank, PageInformation, BitVec)>,
    /// Some functionalities may be disabled once assembler is in a bank (TODO determine which ones)
    selected_bank: Option<usize>,

    /// Counter for the unique labels within macros
    macro_seed: usize,

    charset_encoding: CharsetEncoding,

    /// Track where bytes has been written to forbid overriding them when generating data
    /// BUG: should be stored individually in each bank ?
    written_bytes: BitVec,
    byte_written: bool,

    symbols: SymbolsTableCaseDependent,

    /// Return value of the currently executed function. Is almost always None
    return_value: Option<ExprResult>,
    functions: BTreeMap<String, Arc<Function>>,

    /// Set only if the run instruction has been used
    run_options: Option<(u16, Option<u16>)>,

    /// optional object that manages the listing output
    output_trigger: Option<ListingOutputTrigger>,
    /// Listing of symbols generator
    symbols_output: SymbolOutputGenerator,

    string_warning_done: bool,
    warnings: Vec<AssemblerWarning>,

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

    included_paths: HashSet<PathBuf>,

    // temporary stuff
    extra_print_from_function: RwLock<Vec<PrintOrPauseCommand>>,
    extra_failed_assert_from_function: RwLock<Vec<FailedAssertCommand>>
}

impl Clone for Env {
    fn clone(&self) -> Self {
        Self {
            options: self.options.clone(),
            can_skip_next_passes: (*self.can_skip_next_passes.read().unwrap().deref()).into(),
            request_additional_pass: (*self.request_additional_pass.read().unwrap().deref()).into(),
            pass: self.pass.clone(),
            real_nb_passes: self.real_nb_passes.clone(),
            crunched_section_state: self.crunched_section_state.clone(),
            stable_counters: self.stable_counters.clone(),
            ga_mmr: self.ga_mmr.clone(),
            output_address: self.output_address.clone(),
            pages_info_sna: self.pages_info_sna.clone(),
            sna: self.sna.clone(),
            sna_version: self.sna_version.clone(),
            banks: self.banks.clone(),
            selected_bank: self.selected_bank.clone(),
            macro_seed: self.macro_seed.clone(),
            charset_encoding: self.charset_encoding.clone(),
            written_bytes: self.written_bytes.clone(),
            byte_written: self.byte_written.clone(),
            symbols: self.symbols.clone(),
            run_options: self.run_options.clone(),
            output_trigger: self.output_trigger.clone(),
            symbols_output: self.symbols_output.clone(),
            string_warning_done: self.string_warning_done.clone(),
            warnings: self.warnings.clone(),
            nested_rorg: self.nested_rorg.clone(),
            sections: self.sections.clone(),
            current_section: self.current_section.clone(),
            saved_files: self.saved_files.clone(),

            if_token_adr_to_used_decision: self.if_token_adr_to_used_decision.clone(),
            if_token_adr_to_unused_decision: self.if_token_adr_to_unused_decision.clone(),
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
                .into()
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

impl Default for Env {
    fn default() -> Self {
        Self {
            pass: AssemblingPass::Uninitialized,
            options: EnvOptions::default(),
            stable_counters: StableTickerCounters::default(),
            pages_info_sna: vec![Default::default(); 2],
            ga_mmr: 0xC0, // standard memory configuration

            macro_seed: 0,
            charset_encoding: CharsetEncoding::new(),
            sna: {
                let mut sna = Snapshot::default(); // Snapshot::new_6128().unwrap();
                sna.unwrap_memory_chunks();
                sna
            },
            sna_version: cpclib_sna::SnapshotVersion::V3,

            symbols: SymbolsTableCaseDependent::default(),
            run_options: None,
            written_bytes: BitVec::repeat(false, 0x4000 * 2 * 4),
            byte_written: false,
            output_trigger: None,
            symbols_output: Default::default(),

            crunched_section_state: None,

            string_warning_done: false,
            warnings: Vec::new(),
            nested_rorg: 0,

            sections: HashMap::<String, Arc<RwLock<Section>>>::default(),
            current_section: None,
            output_address: 0,
            banks: Vec::new(),
            selected_bank: None,

            real_nb_passes: 0,
            saved_files: None,
            can_skip_next_passes: true.into(),
            request_additional_pass: false.into(),

            if_token_adr_to_used_decision: HashMap::default(),
            if_token_adr_to_unused_decision: HashMap::default(),
            requested_additional_pass: false,

            functions: Default::default(),
            return_value: None,

            current_pass_discarded_errors: HashSet::default(),
            previous_pass_discarded_errors: HashSet::default(),

            included_paths: HashSet::default(),

            extra_print_from_function: Vec::new().into(),
            extra_failed_assert_from_function: Vec::new().into()
        }
    }
}

/// Symbols handling
impl Env {
    pub fn options(&self) -> &EnvOptions {
        &self.options
    }

    pub fn symbols(&self) -> &SymbolsTableCaseDependent {
        &self.symbols
    }

    pub fn symbols_mut(&mut self) -> &mut SymbolsTableCaseDependent {
        &mut self.symbols
    }

    /// Compute the expression thanks to the symbol table of the environment.
    /// If the expression is not solvable in first pass, 0 is returned.
    /// If the expression is not solvable in second pass, an error is returned
    ///
    /// However, when assembling in a crunched section, the expression MUST NOT fail. edit: why ? I do not get it now and I have removed this limitation
    pub fn resolve_expr_may_fail_in_first_pass<E: ExprEvaluationExt>(
        &self,
        exp: &E
    ) -> Result<ExprResult, AssemblerError> {
        self.resolve_expr_may_fail_in_first_pass_with_default(exp, 0)
    }

    pub fn resolve_expr_may_fail_in_first_pass_with_default<
        E: ExprEvaluationExt,
        R: Into<ExprResult>
    >(
        &self,
        exp: &E,
        r: R
    ) -> Result<ExprResult, AssemblerError> {
        match exp.resolve(self) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.pass.is_first_pass() {
                    *self.can_skip_next_passes.write().unwrap() = false;
                    Ok(r.into())
                }
                else {
                    Err(e)
                }
            }
        }
    }

    /// Compute the expression thanks to the symbol table of the environment.
    /// An error is systematically raised if the expression is not solvable (i.e., labels are unknown)
    fn resolve_expr_must_never_fail<E: ExprEvaluationExt>(
        &self,
        exp: &E
    ) -> Result<ExprResult, AssemblerError> {
        match exp.resolve(self) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.pass.is_first_pass() {
                    *self.can_skip_next_passes.write().unwrap() = false;
                    Err(e)
                }
                else {
                    Err(e)
                }
            }
        }
    }

    pub(crate) fn add_function_parameter_to_symbols_table<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<(), AssemblerError> {
        let symbol = symbol.into();
        // // we do not test that, otherwise it is impossible to do recursive functions
        // if self.symbols().contains_symbol(symbol.clone())? {
        // return Err(AssemblerError::IncoherentCode{msg: format!("Function parameter {} already present", symbol)})
        // }
        self.symbols.set_symbol_to_value(symbol, value)?;
        Ok(())
    }

    /// Add a symbol to the symbol table.
    /// In pass 1: the label MUST be absent
    /// In pass 2: the label MUST be present and of the same value
    fn add_symbol_to_symbol_table<E: Into<Value>>(
        &mut self,
        label: &str,
        value: E
    ) -> Result<(), AssemblerError> {
        let already_present = self.symbols().contains_symbol(label)?;
        let value = value.into();

        match (already_present, self.pass) {
            (true, AssemblingPass::FirstPass) => Err(AssemblerError::SymbolAlreadyExists {
                symbol: label.to_string(),
            }),
            (false, AssemblingPass::SecondPass ) => {
                // here we weaken the test to allow multipass stuff
                if ! self.requested_additional_pass &&
                   ! *self.request_additional_pass.read().unwrap()
                {
                        Err(AssemblerError::IncoherentCode{msg: format!(
                        "Label {} is not present in the symbol table in pass {}. There is an issue with some  conditional code.",
                        label, self.pass
                    )})
                } else {
                    self.symbols_mut()
                        .set_symbol_to_value(label, value)?;
                    Ok(())
                }
             },
            (false,  AssemblingPass::ListingPass) => {
                panic!();
                Err(AssemblerError::IncoherentCode{msg: format!(
                "Label {} is not present in the symbol table in pass {}. There is an issue with some  conditional code.",
                label, self.pass
            )})
            },
            (false, AssemblingPass::FirstPass) | (false, AssemblingPass::Uninitialized) => {
                self.symbols_mut()
                    .set_symbol_to_value(label, value)?;
                Ok(())
            }
            (true, AssemblingPass::SecondPass | AssemblingPass::ListingPass) => {
                self.symbols_mut()
                    .update_symbol_to_value(&label.to_owned(), value)?;
                Ok(())
            }
            (_, _) => panic!(
                "add_symbol_to_symbol_table / unmanaged case {}, {}, {} {:#?}",
                self.pass, label, already_present, value
            ),
        }
    }

    /// Track the symbols for an expression that has been properly executed
    fn track_used_symbols(&mut self, e: &Expr) {
        e.symbols_used()
            .into_iter()
            .for_each(|symbol| self.symbols.use_symbol(symbol))
    }
}
/// Report handling
impl Env {
    pub fn report(&self, start: &Instant) -> Report {
        Report::from((self, start))
    }
}

/// Include once handling {
impl Env {
    fn has_included(&self, path: &PathBuf) -> bool {
        self.included_paths.contains(path)
    }

    fn mark_included(&mut self, path: PathBuf) {
        self.included_paths.insert(path);
    }
}

/// Error handling
impl Env {
    /// If the error has not been raised at the previous pass, store it and do not propagate it. Otherwise, propagate it
    pub fn add_error_discardable_one_pass(
        &mut self,
        e: AssemblerError
    ) -> Result<(), AssemblerError> {
        let repr = SimplerAssemblerError(&e).to_string();
        if self.previous_pass_discarded_errors.contains(&repr) {
            return Err(e);
        }
        else {
            self.current_pass_discarded_errors.insert(repr);
            return Ok(());
        }
    }
}
/// Namespace handling
impl Env {
    fn enter_namespace(&mut self, namespace: &str) -> Result<(), AssemblerError> {
        if namespace.contains(".") {
            return Err(AssemblerError::AssemblingError {
                msg: format!("Invalid namespace \"{}\"", namespace)
            });
        }
        self.symbols_mut().enter_namespace(namespace);
        Ok(())
    }

    fn leave_namespace(&mut self) -> Result<Symbol, AssemblerError> {
        self.symbols_mut().leave_namespace().map_err(|e| e.into())
    }
}

#[allow(missing_docs)]
impl Env {
    /// Create an environment that embeds a copy of the given table and is configured to be in the latest pass.
    /// Mainly used for tests.
    pub fn with_table(symbols: &SymbolsTable) -> Self {
        let mut env = Self::default();
        env.symbols.set_table(symbols.clone());
        env.pass = AssemblingPass::SecondPass;
        env
    }

    pub fn with_table_case_dependent(symbols: &SymbolsTableCaseDependent) -> Self {
        let mut env = Self::default();
        env.symbols = symbols.clone();
        env.pass = AssemblingPass::SecondPass;
        env
    }

    pub fn warnings(&self) -> &[AssemblerWarning] {
        &self.warnings
    }

    /// Manage the play with data for the output listing
    fn handle_output_trigger(&mut self, new: &LocatedToken) {
        if self.pass.is_listing_pass() && self.output_trigger.is_some() {
            let code_addr = self.logical_code_address();
            let phy_addr = self.logical_to_physical_address(self.logical_output_address());

            let kind = if self.crunched_section_state.is_some() {
                AddressKind::CrunchedArea
            }
            else {
                AddressKind::Address
            };

            let trig = self.output_trigger.as_mut().unwrap();

            trig.new_token(new, code_addr as _, kind, phy_addr);
        }
    }

    /// Start a new pass by cleaning up datastructures.
    /// The only thing to keep is the symbol table
    pub(crate) fn start_new_pass(&mut self) {
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
                    AssemblingPass::SecondPass
                }
                else {
                    self.pass.next_pass()
                }
            }
            else {
                if !*self.request_additional_pass.read().unwrap() {
                    AssemblingPass::Finished
                }
                else {
                    AssemblingPass::SecondPass
                }
            };
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

            self.stable_counters = StableTickerCounters::default();
            self.run_options = None;
            self.written_bytes().fill(false);
            self.warnings.retain(|elem| !elem.is_override_memory());
            self.pages_info_sna.iter_mut().for_each(|p| p.new_pass());

            self.sections
                .iter_mut()
                .for_each(|s| s.1.write().unwrap().new_pass());
            self.current_section = None;

            self.banks.iter_mut().for_each(|bank| {
                bank.1.new_pass();
                bank.2.fill(false);
            });
            self.selected_bank = None;

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
            self.symbols.new_pass();

            if self.options.show_progress() {
                #[cfg(not(target_arch = "wasm32"))]
                Progress::progress().new_pass();
            }
        }
    }

    /// Handle the actions to do after assembling.
    /// ATM it is only the save of data for each page
    pub fn handle_post_actions(&mut self) -> Result<(), AssemblerError> {
        self.handle_print()?;
        self.handle_assert()?;
        self.handle_breakpoints()?;
        self.saved_files = Some(self.handle_file_save()?);
        Ok(())
    }

    /// We handle breakpoint ONLY for the pages stored in the snapshot
    /// as they are stored inside a chunck of the snapshot:
    /// If one day another export is coded, we could export the others too.
    fn handle_breakpoints(&mut self) -> Result<(), AssemblerError> {
        let mut winape_raw = Vec::new();

        let pages_mmr = MMR_PAGES_SELECTION;
        for (activepage, _page) in pages_mmr[0..self.pages_info_sna.len()].iter().enumerate() {
            for brk in self.pages_info_sna[activepage].collect_breakpoints() {
                let info = AssemblerError::RelocatedInfo {
                    info: Box::new(AssemblerError::AssemblingError {
                        msg: format!("Add a breakpoint in 0x{:04x}", brk.address)
                    }),
                    span: brk.span.clone()
                };
                eprint!("{}", info);

                winape_raw.extend(brk.winape_raw());
            }
        }

        assert_eq!(winape_raw.len() % 5, 0);

        let winape_chunk = WinapeBreakPointChunk::from([b'B', b'R', b'K', b'S'], winape_raw);

        if winape_chunk.nb_breakpoints() > 0 {
            self.sna.add_chunk(winape_chunk);
        }

        Ok(())
    }

    fn handle_assert(&mut self) -> Result<(), AssemblerError> {
        let backup = self.ga_mmr;

        // ga values to properly switch the pages
        let pages_mmr = MMR_PAGES_SELECTION;

        let mut assert_failures: Option<AssemblerError> = None;

        // Print from the snapshot
        for (activepage, page) in pages_mmr[0..self.pages_info_sna.len()].iter().enumerate() {
            self.ga_mmr = *page;
            let mut l_errors = self.pages_info_sna[activepage].collect_assert_failure();
            match (&mut assert_failures, &mut l_errors) {
                (_, Ok(_)) => {
                    // nothing to do
                }
                (
                    Some(AssemblerError::MultipleErrors { errors: e1 }),
                    Err(AssemblerError::MultipleErrors { errors: e2 })
                ) => {
                    e1.append(e2);
                }
                (None, Err(l_errors)) => {
                    assert_failures = Some(l_errors.clone());
                }
                _ => unreachable!()
            }
        }

        for bank in self.banks.iter() {
            let mut l_errors = bank.1.collect_assert_failure();
            match (&mut assert_failures, &mut l_errors) {
                (_, Ok(_)) => {
                    // nothing to do
                }
                (
                    Some(AssemblerError::MultipleErrors { errors: e1 }),
                    Err(AssemblerError::MultipleErrors { errors: e2 })
                ) => {
                    e1.append(e2);
                }
                (None, Err(l_errors)) => {
                    assert_failures = Some(l_errors.clone());
                }
                _ => unreachable!()
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

    pub fn handle_print(&mut self) -> Result<(), AssemblerError> {
        let backup = self.ga_mmr;

        // ga values to properly switch the pages
        let pages_mmr = MMR_PAGES_SELECTION;

        let mut print_errors: Option<AssemblerError> = None;
        let mut writer = std::io::stdout();

        // Print from the snapshot
        for (activepage, page) in pages_mmr[0..self.pages_info_sna.len()].iter().enumerate() {
            self.ga_mmr = *page;
            let mut l_errors = self.pages_info_sna[activepage].execute_print_or_pause(&mut writer);
            match (&mut print_errors, &mut l_errors) {
                (_, Ok(_)) => {
                    // nothing to do
                }
                (
                    Some(AssemblerError::MultipleErrors { errors: e1 }),
                    Err(AssemblerError::MultipleErrors { errors: e2 })
                ) => {
                    e1.append(e2);
                }
                (None, Err(l_errors)) => {
                    print_errors = Some(l_errors.clone());
                }
                _ => unreachable!()
            }
        }

        for bank in self.banks.iter() {
            let mut l_errors = bank.1.execute_print_or_pause(&mut writer);
            match (&mut print_errors, &mut l_errors) {
                (_, Ok(_)) => {
                    // nothing to do
                }
                (
                    Some(AssemblerError::MultipleErrors { errors: e1 }),
                    Err(AssemblerError::MultipleErrors { errors: e2 })
                ) => {
                    e1.append(e2);
                }
                (None, Err(l_errors)) => {
                    print_errors = Some(l_errors.clone());
                }
                _ => unreachable!()
            }
        }

        self.ga_mmr = backup;

        // All possible messages have been printed.
        // Errors are generated for the others
        if let Some(errors) = print_errors {
            Err(errors)
        }
        else {
            Ok(())
        }
    }

    fn handle_file_save(&mut self) -> Result<Vec<SavedFile>, AssemblerError> {
        let backup = self.ga_mmr;

        // ga values to properly switch the pages
        let pages_mmr = MMR_PAGES_SELECTION;

        let mut saved_files = Vec::new();

        // count the number of files to save to build the process bar
        let mut nb_files_to_save: u64 = 0;
        nb_files_to_save += pages_mmr[0..self.pages_info_sna.len()]
            .iter()
            .enumerate()
            .map(|(activepage, page)| {
                self.ga_mmr = *page;
                self.pages_info_sna[activepage].nb_files_to_save() as u64
            })
            .sum::<u64>() as u64;
        nb_files_to_save += self
            .banks
            .iter()
            .map(|b| b.1.nb_files_to_save() as u64)
            .sum::<u64>() as u64;

        if self.options.show_progress() {
                #[cfg(not(target_arch = "wasm32"))]
                Progress::progress().create_save_bar(nb_files_to_save);
        }

        // save from snapshot
        for (activepage, page) in pages_mmr[0..self.pages_info_sna.len()].iter().enumerate() {
            //  eprintln!("ACTIVEPAGE. {:x}", &activepage);
            //  eprintln!("PAGE. {:x}", &page);

            self.ga_mmr = *page;
            let mut saved = self.pages_info_sna[activepage].execute_save(self)?;
            saved_files.append(&mut saved);
        }

        // save from extra memory / can be done in parallal
        self.ga_mmr = 0xC0;

        #[cfg(not(target_arch = "wasm32"))]
        let iter = self.banks.par_iter();
        #[cfg(target_arch = "wasm32")]
        let iter = self.banks.iter();
        let mut saved = iter
            .map(|bank| bank.1.execute_save(self))
            .collect::<Result<Vec<_>, AssemblerError>>()?;
        for s in &mut saved {
            saved_files.append(s);
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
    /// TODO
    fn active_page_info(&self) -> &PageInformation {
        match &self.selected_bank {
            Some(idx) => &self.banks[*idx].1,
            None => {
                let active_page =
                    self.logical_to_physical_address(self.output_address).page() as usize;
                &self.pages_info_sna[active_page]
            }
        }
    }

    /// TODO remove this method and its calls
    fn active_page_info_mut(&mut self) -> &mut PageInformation {
        match &self.selected_bank {
            Some(idx) => &mut self.banks[*idx].1,
            None => {
                let active_page =
                    self.logical_to_physical_address(self.output_address).page() as usize;
                &mut self.pages_info_sna[active_page]
            }
        }
    }

    fn page_info_for_logical_address_mut(&mut self, address: u16) -> &mut PageInformation {
        match &self.selected_bank {
            Some(idx) => &mut self.banks[*idx].1, // TODO check if this code is valid
            None => {
                let active_page =
                    self.logical_to_physical_address(address).page() as usize;
                &mut self.pages_info_sna[active_page]
            }
        }       
    }

    fn written_bytes(&mut self) -> &mut BitVec {
        match &self.selected_bank {
            Some(idx) => &mut self.banks[*idx].2,
            None => &mut self.written_bytes
        }
    }

    /// Return the address where the next byte will be written
    pub fn logical_output_address(&self) -> u16 {
        self.active_page_info().logical_outputadr
    }

    /// Return the address of dollar
    pub fn logical_code_address(&self) -> u16 {
        self.active_page_info().logical_codeadr
    }

    pub fn limit_address(&self) -> u16 {
        self.active_page_info().limit
    }

    pub fn start_address(&self) -> Option<u16> {
        self.active_page_info().startadr
    }

    pub fn maximum_address(&self) -> u16 {
        self.active_page_info().maxadr
    }

    /// . Update the value of $ in the symbol table in order to take the current  output address
    pub fn update_dollar(&mut self) {
        let addr = self.logical_to_physical_address(self.logical_code_address());
        self.symbols.set_current_address(addr);

        let addr = self.logical_to_physical_address(self.logical_output_address());
        self.symbols.set_current_output_address(addr);
    }

    /// Produce the memory for the required limits
    /// TODO check that the implementation is still correct with snapshot inclusion
    /// BUG  does not take into account extra bank configuration
    pub fn memory(&self, start: u16, size: u16) -> Vec<u8> {
        //     dbg!(self.ga_mmr);
        let mut mem = Vec::new();
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
            }
            None => (0, 0)
        };

        self.memory(start, length as _)
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

    /// Output one byte either in the appropriate bank of the snapshot or in the termporary bank
    /// return true if it raised an override warning
    pub fn output(&mut self, v: u8) -> Result<bool, AssemblerError> {
        //   dbg!(self.logical_output_address(), self.output_address);
        if self.logical_output_address() != self.output_address {
            return Err(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!(
                    "Sync issue with output address (0x{:x} != 0x{:x})",
                    self.logical_output_address(),
                    self.output_address
                )
            });
        }

        // dbg!(self.output_address(), &v);
        let physical_address =
            self.logical_to_physical_address(self.logical_output_address() as u16);

        // Check if it is legal to output the value
        if self.logical_code_address() > self.limit_address()
            || (self.active_page_info().fail_next_write_if_zero && self.logical_code_address() == 0)
        {
            // dbg!(self.logical_output_address() > self.limit_address(), self.active_page_info().fail_next_write_if_zero && self.logical_output_address()==0);

            return Err(AssemblerError::OutputExceedsLimits(
                physical_address,
                self.limit_address() as _
            ));
        }
        for protected_area in &self.active_page_info().protected_areas {
            if protected_area.contains(&(self.logical_code_address() as u16)) {
                return Err(AssemblerError::OutputProtected {
                    area: protected_area.clone(),
                    address: self.logical_code_address() as _
                });
            }
        }

        self.byte_written = true;

        // update the maximm 64k position
        self.active_page_info_mut().maxadr =
            self.maximum_address().max(self.logical_output_address());
        if self.active_page_info().startadr.is_none() {
            self.active_page_info_mut().startadr = Some(self.logical_output_address());
        }

        let abstract_address = physical_address.offset_in_cpc();
        let already_used = *self.written_bytes().get(abstract_address as usize).unwrap();

        let r#override = if already_used {
            let r#override = AssemblerWarning::OverrideMemory(physical_address.clone(), 1);
            if self.allow_memory_override() {
                self.warnings.push(r#override);
                true
            }
            else {
                return Err(r#override);
            }
        }
        else {
            false
        };

        if self.selected_bank.is_none() {
            if let Some(section) = &self.current_section {
                let section = section.read().unwrap();
                if !section.contains(physical_address.address()) {
                    return Err(AssemblerError::AssemblingError {
                        msg: format!(
                            "SECTION error: write address 0x{:x} out of range [Ox{:}-Ox{:}]",
                            physical_address.address(),
                            section.start,
                            section.stop
                        )
                    });
                }
            }
        }

        match &self.selected_bank {
            Some(idx) => {
                self.banks[*idx].0[self.output_address as usize] = v;
            }
            None => {
                self.sna.set_byte(abstract_address, v);
            }
        }
        self.written_bytes().set(abstract_address as _, true);

        // Add the byte to the listing space
        if self.pass.is_listing_pass() && self.output_trigger.is_some() {
            self.output_trigger.as_mut().unwrap().write_byte(v);
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
        true // TODO parametrize it in the options (and set false by default)
    }

    /// Write consecutives bytes
    pub fn output_bytes(&mut self, bytes: &[u8]) -> Result<(), AssemblerError> {
        //        dbg!(self.logical_output_address(), bytes);

        let mut previously_overrided = false;
        for b in bytes.iter() {
            let currently_overrided = self.output(*b)?;

            match (previously_overrided, currently_overrided) {
                (true, true) => {
                    // remove the latestwarning as it is a duplicate
                    let extra_override_idx = self
                        .warnings
                        .iter_mut()
                        .rev()
                        .position(|w| {
                            if let AssemblerError::OverrideMemory(..) = w {
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
                            if let AssemblerError::OverrideMemory(..) = w {
                                true
                            }
                            else {
                                false
                            }
                        })
                        .unwrap(); // cannot fail by construction

                    // increase its size
                    match r#override {
                        AssemblerError::OverrideMemory(_, ref mut size) => {
                            *size += 1;
                        }
                        _ => unreachable!()
                    };
                }
                _ => {
                    // nothing to do
                }
            }

            previously_overrided = currently_overrided;
        }

        Ok(())
    }

    pub fn peek(&self, address: &PhysicalAddress) -> u8 {
        let address = address.offset_in_cpc();
        match &self.selected_bank {
            Some(idx) => self.banks[*idx].0[address as usize],
            None => self.sna.get_byte(address)
        }
    }

    pub fn poke(&mut self, byte: u8, address: &PhysicalAddress) {
        let address = address.offset_in_cpc();
        match &self.selected_bank {
            Some(idx) => {
                self.banks[*idx].0[address as usize] = byte;
            }
            None => {
                self.sna.set_byte(address, byte);
            }
        }
    }

    /// Get the size of the generated binary.
    /// ATTENTION it can only work when geneating 0x10000 files
    pub fn size(&self) -> u16 {
        if self.start_address().is_none() {
            panic!("Unable to compute size now");
        }
        else {
            (self.logical_output_address() - self.start_address().unwrap()) as u16
        }
    }

    /// Evaluate the expression according to the current state of the environment
    pub fn eval(&self, expr: &Expr) -> Result<ExprResult, AssemblerError> {
        expr.resolve(self)
    }

    pub fn sna(&self) -> &cpclib_sna::Snapshot {
        &self.sna
    }

    pub fn sna_version(&self) -> cpclib_sna::SnapshotVersion {
        self.sna_version
    }

    pub fn save_sna<P: AsRef<std::path::Path>>(&self, fname: P) -> Result<(), std::io::Error> {
        self.sna().save(fname, self.sna_version())
    }

    /// Compute the relative address. Is authorized to fail at first pass
    fn absolute_to_relative_may_fail_in_first_pass(
        &self,
        address: i32,
        opcode_delta: i32
    ) -> Result<u8, AssemblerError> {
        match absolute_to_relative(address, opcode_delta, self.symbols()) {
            Ok(value) => Ok(value),
            Err(error) => {
                if self.pass.is_first_pass() {
                    Ok(0)
                }
                else {
                    Err(AssemblerError::RelativeAddressUncomputable {
                        address,
                        pass: self.pass,
                        error: Box::new(error)
                    })
                }
            }
        }
    }
}

impl Env {
    pub fn add_warning(&mut self, warning: AssemblerWarning) {
        self.warnings.push(warning);
    }
}

/// Visit directives
impl Env {

    fn visit_org<E: ExprElement + ExprEvaluationExt >(&mut self, address: &E, address2: Option<&E>) -> Result<(), AssemblerError> {

        // org $ set org to the output address (cf. rasm)
        let code_adr = if address2.is_none() && address.is_label_value("$") {
            if self.start_address().is_none() {
                return Err(AssemblerError::InvalidArgument {
                    msg: "ORG: $ cannot be used now".into()
                });
            }
            self.logical_output_address() as i32
        }
        else {
            self.resolve_expr_must_never_fail(address)?.int()?
        };
    
        let output_adr = if address2.is_some() {
            self.resolve_expr_must_never_fail(address2.unwrap())?.int()?
        }
        else {
            code_adr.clone()
        };
    
        // TODO Check overlapping region
        {
            let page_info = self.page_info_for_logical_address_mut(output_adr as _);
            page_info.logical_outputadr = output_adr as _;
            page_info.logical_codeadr = code_adr as _;
            page_info.fail_next_write_if_zero = false;
        }
    
        // Specify start address at first use
        self.page_info_for_logical_address_mut(output_adr as _).startadr = match self.start_address() {
            Some(val) => val.min(self.logical_output_address()),
            None => self.logical_output_address()
        }
        .into();
    
    
        self.output_address = output_adr as _;
        self.update_dollar();
    
        // update the erroneous information for the listing
        if self.pass.is_listing_pass() && self.output_trigger.is_some() {
            let output_adr = self.logical_to_physical_address(output_adr as _);
            let trigger = self.output_trigger.as_mut().unwrap();
    
            trigger.replace_code_address(code_adr.into());
            trigger.replace_physical_address(output_adr);
        }
    
        if self.logical_output_address() != self.output_address {
            return Err(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!(
                    "BUG in assembler: 0x{:x}!=0x{:x} in pass {:?}",
                    self.logical_output_address(),
                    self.output_address,
                    self.pass
                )
            });
        }
        Ok(())
    }



    fn visit_breakpoint(
        &mut self,
        exp: Option<&Expr>,
        span: Option<Z80Span>
    ) -> Result<(), AssemblerError> {
        if exp.is_some() {
            return Err(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("Breakpoint with an expression is not yet implemented")
            });
        }

        let current_address = self.logical_code_address();
        let page = match self.logical_to_physical_address(current_address).page() {
            0 => 0,
            1 => 1,
            _ => {
                return Err(AssemblerError::BugInAssembler {
                    file: file!(),
                    line: line!(),
                    msg: format!(
                        "Page selection not handled 0x{:x}",
                        self.logical_to_physical_address(current_address).page()
                    )
                })
            }
        };

        let brk = BreakpointCommand::new(current_address, page, span.unwrap());
        self.active_page_info_mut().add_breakpoint_command(brk);

        Ok(())
    }
}

#[allow(missing_docs)]
impl Env {
    /// Write in w the list of symbols
    pub fn generate_symbols_output<W: Write>(&self, w: &mut W, fmt: SymbolOutputFormat) -> std::io::Result<()> {
        self.symbols_output.generate(w, self.symbols(), fmt)
    }

    /// Visit all the tokens of the slice of tokens.
    /// Return true if an additional pass is requested
    pub fn visit_listing<T: ListingElement + Visited + MayHaveSpan>(
        &mut self,
        listing: &[T]
    ) -> Result<(), AssemblerError> {
        for token in listing.iter() {
            let _res = token.visited(self)?;
        }

        Ok(())
    }

    /// TODO set the limit for the current page
    fn visit_limit(&mut self, exp: &Expr) -> Result<(), AssemblerError> {
        let value = self.resolve_expr_must_never_fail(exp)?.int()?;
        self.active_page_info_mut().limit = value as _;

        if self.limit_address() <= self.maximum_address() {
            return Err(AssemblerError::OutputAlreadyExceedsLimits(
                self.limit_address() as _
            ));
        }
        if self.limit_address() == 0 {
            eprintln!("[WARNING] Do you really want to set a limit of 0 ?");
        }
        Ok(())
    }

    fn visit_label(&mut self, label: &str) -> Result<(), AssemblerError> {
        let label = self.symbols().normalize_symbol(label);
        let label = label.value();

        // A label cannot be defined multiple times
        let res = if self.symbols().contains_symbol(label)?
            && (self.pass.is_first_pass()
                || !(self.symbols().kind(label)? == "address"
                    || self.symbols().kind(label)? == "any"))
        {
            Err(AssemblerError::AlreadyDefinedSymbol {
                symbol: self.symbols()
                            .extend_local_and_patterns_for_symbol(label)
                            .map(|s| std::convert::Into::<SmolStr>::into(s))
                            .unwrap_or_else(|_| SmolStr::from(label)),
                kind: self.symbols().kind(label)?.into()
            })
        }
        else {
            if !label.starts_with('.') {
                self.symbols_mut().set_current_label(label)?;
            }

            // If the current address is not set up, we force it to be 0
            let value = match self.symbols().current_address() {
                Ok(address) => address,
                Err(_) => 0
            };
            let addr = self.logical_to_physical_address(value);

            self.add_symbol_to_symbol_table(label, addr)
        };

        // Try to fallback on a macro call - parser is not that much great
        if let Err(AssemblerError::AlreadyDefinedSymbol { symbol: _, kind }) = &res {
            if kind == "macro" || kind == "struct" {
                return Err(AssemblerError::AssemblingError {
                    msg:
                        "Use (void) for macros with no parameters to disambiguate them with labels"
                            .to_owned()
                });
            // self.visit_call_macro_or_build_struct(&Token::MacroCall(
            // label.into(),
            // Default::default()
            // ))
            }
            else {
                res
            }
        }
        else {
            res
        }
    }

    fn visit_noexport<S: Borrow<str> + Display>(
        &mut self,
        labels: &[S]
    ) -> Result<(), AssemblerError> {
        if labels.is_empty() {
            self.symbols_output.forbid_all_symbols();
        }
        else {
            labels
                .iter()
                .for_each(|l| self.symbols_output.forbid_symbol(l.borrow()));
        }

        Ok(())
    }

    fn visit_export<S: Borrow<str> + Display>(
        &mut self,
        labels: &[S]
    ) -> Result<(), AssemblerError> {
        if labels.is_empty() {
            self.symbols_output.allow_all_symbols();
        }
        else {
            labels
                .iter()
                .for_each(|l| self.symbols_output.allow_symbol(l.borrow()));
        }

        Ok(())
    }

    fn visit_multi_pushes(&mut self, regs: &[DataAccess]) -> Result<(), AssemblerError> {
        let result = regs
            .iter()
            .map(|reg| assemble_push(reg))
            .collect::<Result<Vec<_>, AssemblerError>>()?;
        let result = result.into_iter().flatten().collect_vec();
        self.output_bytes(&result)
    }

    fn visit_multi_pops(&mut self, regs: &[DataAccess]) -> Result<(), AssemblerError> {
        let result = regs
            .iter()
            .map(|reg| assemble_pop(reg))
            .collect::<Result<Vec<_>, AssemblerError>>()?;
        let result = result.into_iter().flatten().collect_vec();
        self.output_bytes(&result)
    }

    pub fn visit_macro_definition(
        &mut self,
        name: &str,
        arguments: &[&str],
        code: &str,
        source: Option<&Z80Span>
    ) -> Result<(), AssemblerError> {
        if self.pass.is_first_pass() && self.symbols().contains_symbol(name)? {
            return Err(AssemblerError::SymbolAlreadyExists {
                symbol: name.to_owned()
            });
        }

        let source = source.map(|s| s.into());

        self.symbols_mut().set_symbol_to_value(
            name,
            Macro::new(name.into(), arguments, code.to_owned(), source)
        )?;
        Ok(())
    }

    pub fn visit_waitnops(&mut self, count: &Expr) -> Result<(), AssemblerError> {
        // TODO really use a clever way
        let bytes = self.assemble_nop(Mnemonic::Nop, Some(count))?;
        self.output_bytes(&bytes)?;

        self.stable_counters
            .update_counters(self.resolve_expr_may_fail_in_first_pass(count)?.int()? as _);
        Ok(())
    }

    pub fn visit_struct_definition<T: ListingElement + ToSimpleToken, S: Borrow<str>>(
        &mut self,
        name: &str,
        content: &[(S, T)],
        source: Option<&Z80Span>
    ) -> Result<(), AssemblerError> {
        if self.pass.is_first_pass() && self.symbols().contains_symbol(name)? {
            return Err(AssemblerError::SymbolAlreadyExists {
                symbol: name.to_owned()
            });
        }

        let r#struct = Struct::new(name, content, source.map(|s| s.into()));
        // add inner index BEFORE the structure. It should reduce infinite loops
        let mut index = 0;
        for (f, s) in r#struct.fields_size(self.symbols()) {
            self.symbols_mut()
                .set_symbol_to_value(format!("{}.{}", name, f), index)?;
            index += s;
        }
        self.symbols_mut().set_symbol_to_value(name, r#struct)?;

        Ok(())
    }

    pub fn visit_buildsna(
        &mut self,
        version: Option<&SnapshotVersion>
    ) -> Result<(), AssemblerError> {
        self.sna_version = version.cloned().unwrap_or(SnapshotVersion::V3);
        Ok(())
    }

    pub fn visit_align(
        &mut self,
        boundary: &Expr,
        fill: Option<&Expr>
    ) -> Result<(), AssemblerError> {
        let boundary = self.resolve_expr_must_never_fail(boundary)?.int()? as u16;
        let fill = match fill {
            Some(fill) => self.resolve_expr_may_fail_in_first_pass(fill)?.int()? as u8,
            None => 0
        };

        while self.logical_output_address() as u16 % boundary != 0 {
            self.output(fill)?;
        }

        Ok(())
    }

    fn get_section_description(&self, name: &str) -> Result<Section, AssemblerError> {
        match self.sections.get(name) {
            Some(section) => Ok(section.read().unwrap().clone()),
            None => {
                Err(AssemblerError::AssemblingError {
                    msg: format!("Section '{}' does not exists", name)
                })
            }
        }
    }

    fn visit_section(&mut self, name: &str) -> Result<(), AssemblerError> {
        let section = match self.sections.get(name) {
            Some(section) => section,
            None => {
                return Err(AssemblerError::AssemblingError {
                    msg: format!("Section '{}' does not exists", name)
                });
            }
        };

        let (output_adr, code_adr, mmr) = {
            let section = section.read().unwrap();

            if section.mmr != self.ga_mmr {
                self.warnings.push(AssemblerError::AssemblingError{
                    msg: format!("Gate Array configuration is not coherent with the section. We  manually set it (0x{:x} expected instead of 0x{:x})", section.mmr, self.ga_mmr)
                });
            }

            (section.output_adr, section.code_adr, section.mmr)
        };

        self.current_section = Some(Arc::clone(section));

        self.ga_mmr = mmr;
        self.output_address = output_adr;

        self.active_page_info_mut().logical_outputadr = output_adr;
        self.active_page_info_mut().logical_codeadr = code_adr;

        self.update_dollar();
        self.output_trigger
            .as_mut()
            .map(|o| o.replace_code_address(code_adr.into()));
        Ok(())
    }

    fn visit_range(&mut self, name: &str, start: &Expr, stop: &Expr) -> Result<(), AssemblerError> {
        let start = self.resolve_expr_must_never_fail(start)?.int()? as u16;
        let stop = self.resolve_expr_must_never_fail(stop)?.int()? as u16;
        let mmr = self.ga_mmr;

        if let Some(section) = self.sections.get(name) {
            let section = section.read().unwrap();
            if start != section.start
                || stop != section.stop
                || name != section.name
                || mmr != section.mmr
            {
                return Err(AssemblerError::AssemblingError {
                    msg: format!(
                        "Section '{}' is already defined from 0x{:x} to 0x{:x} in 0x{:x}",
                        section.name, section.start, section.stop, section.mmr
                    )
                });
            }
        }
        else {
            let section = Arc::new(RwLock::new(Section::new(name, start, stop, mmr)));

            self.sections.insert(name.to_owned(), section);
        }

        Ok(())
    }

    fn visit_next_and_co<E: ExprElement + ExprEvaluationExt> (
        &mut self,
        destination: &str,
        source: &str,
        delta: Option<&E>,
        can_override: bool
    ) -> Result<(), AssemblerError> {
        if !can_override && self.symbols.contains_symbol(destination)? && self.pass.is_first_pass()
        {
            let kind = self.symbols().kind(Symbol::from(destination))?;
            return Err(AssemblerError::AlreadyDefinedSymbol {
                symbol: destination.into(),
                kind: kind.into()
            });
        }

        // setup the value
        let value = self.resolve_expr_must_never_fail(&Expr::Label(source.into()))?;
        if can_override {
            self.symbols_mut()
                .assign_symbol_to_value(destination, value.clone())?;
        }
        else {
            self.add_symbol_to_symbol_table(destination, value.clone())?;
        }
        self.output_trigger
            .as_mut()
            .map(|o| o.replace_code_address(value.clone()));

        // increase next one
        let delta = match delta {
            Some(delta) => self.resolve_expr_must_never_fail(delta)?,
            None => 1.into()
        };
        let value = (value + delta)?;

        self.symbols_mut().assign_symbol_to_value(source, value)?;

        Ok(())
    }

    /// return the page and bank configuration for the given address at the current mmr configuration
    /// https://grimware.org/doku.php/documentations/devices/gatearray#mmr
    pub fn logical_to_physical_address(&self, address: u16) -> PhysicalAddress {
        PhysicalAddress::new(address, self.ga_mmr)
    }

    fn visit_bank(&mut self, exp: Option<&Expr>) -> Result<(), AssemblerError> {
        if self.nested_rorg > 0 {
            return Err(AssemblerError::NotAllowed);
        }

        match exp {
            Some(exp) => {
                // prefix provided, we explicitely want one configuration
                let mmr = self.resolve_expr_must_never_fail(exp)?.int()?;

                if mmr < 0xC0 || mmr > 0xC7 {
                    return Err(AssemblerError::MMRError { value: mmr });
                }

                let mmr = mmr as u8;
                self.ga_mmr = mmr;

                // we do not change the output address (there is no reason to do that)
            }
            None => {
                // nothing provided, we write in a temporary area
                if self.pass.is_first_pass() {
                    self.selected_bank = Some(self.banks.len());
                    self.banks.push((
                        Bank::default(),
                        PageInformation::default(),
                        BitVec::repeat(false, 0x4000 * 4)
                    ));
                }
                else {
                    self.selected_bank = self.selected_bank.map(|v| v + 1).or(Some(0));
                    if *self.selected_bank.as_ref().unwrap() >= self.banks.len() {
                        return Err(AssemblerError::AssemblingError {
                            msg: "There were less banks in previous pass".to_owned()
                        });
                    }
                }

                self.ga_mmr = 0xC0;
                self.output_address = 0;
                let page_info = self.active_page_info_mut();
                page_info.logical_outputadr = 0;
                page_info.logical_codeadr = 0;
            }
        }

        Ok(())
    }

    // total switch of page
    fn visit_bankset(&mut self, exp: &Expr) -> Result<(), AssemblerError> {
        if self.nested_rorg > 0 {
            return Err(AssemblerError::NotAllowed);
        }

        let page = self.resolve_expr_must_never_fail(exp)?.int()? as u8; // This value MUST be interpretable once executed

        //       eprintln!("Warning need to code sna memory extension if needed");
        self.select_page(page)?;
        Ok(())
    }

    fn select_page(&mut self, page: u8) -> Result<(), AssemblerError> {
        if self.nested_rorg > 0 {
            return Err(AssemblerError::NotAllowed);
        }

        if
        // page < 0 ||
        page >= 8 {
            return Err(AssemblerError::InvalidArgument {
                msg: format!(
                    "{} is invalid. BANKSET only accept values from 0 to 7",
                    page
                )
                .into()
            });
        }

        if page == 0 {
            self.ga_mmr = 0b11_000_0_00;
        }
        else {
            self.ga_mmr = 0b11_000_0_10 + ((page - 1) << 3);
        }

        let page = page as usize;
        let nb_pages = self.pages_info_sna.len();
        let expected_nb = nb_pages.max(page + 1);
        if expected_nb > nb_pages {
            self.pages_info_sna.resize(expected_nb, Default::default());
            self.written_bytes().resize(expected_nb * 0x1_0000, false);
        }

        self.output_address = self.logical_output_address();
        self.update_dollar();
        Ok(())
    }

    /// Remove the given variable from the table of symbols
    pub fn visit_undef(&mut self, label: &str) -> Result<(), AssemblerError> {
        match self.symbols_mut().remove_symbol(label)? {
            Some(_) => Ok(()),
            None => {
                Err(AssemblerError::UnknownSymbol {
                    symbol: label.into(),
                    closest: self.symbols().closest_symbol(label, SymbolFor::Number)?
                })
            }
        }
    }

    pub fn visit_protect(&mut self, start: &Expr, stop: &Expr) -> Result<(), AssemblerError> {
        if self.pass.is_first_pass() {
            let start = self.resolve_expr_must_never_fail(start)?.int()? as u16;
            let stop = self.resolve_expr_must_never_fail(stop)?.int()? as u16;

            self.active_page_info_mut()
                .protected_areas
                .push(start..=stop);
        }

        Ok(())
    }

    fn build_string_from_formatted_expression(
        &self,
        info: &[FormattedExpr]
    ) -> Result<String, AssemblerError> {
        let mut repr = String::default();
        for (_idx, current) in info.iter().enumerate() {
            // if idx != 0 {
            // repr += " ";
            // }
            // we do not want the space anymore
            match current {
                FormattedExpr::Raw(Expr::String(string)) => {
                    repr += string;
                }
                FormattedExpr::Raw(Expr::Char(char)) => {
                    repr += &char.to_string();
                }
                FormattedExpr::Raw(expr) => {
                    let value = self.resolve_expr_may_fail_in_first_pass(expr)?;
                    repr += &value.to_string();
                }
                FormattedExpr::Formatted(format, expr) => {
                    let value = self.resolve_expr_may_fail_in_first_pass(expr)?.int()? as i32;
                    repr += &format.string_representation(value);
                }
            }
        }

        Ok(repr)
    }

    /// Print the evaluation of the expression in the 2nd pass
    pub fn visit_print(&mut self, info: &[FormattedExpr], span: Option<Z80Span>) {
        let print_or_error = match self.build_string_from_formatted_expression(info) {
            Ok(msg) => either::Either::Left(msg),
            Err(error) => either::Either::Right(error)
        };

        self.active_page_info_mut().add_print_command(PrintCommand {
            span,
            print_or_error
        })
    }

    pub fn visit_pause(&mut self, span: Option<Z80Span>) {
        self.active_page_info_mut().add_pause_command(span.into());
    }

    pub fn visit_fail(&self, info: Option<&[FormattedExpr]>) -> Result<(), AssemblerError> {
        let repr = info
            .map(|info| self.build_string_from_formatted_expression(info))
            .unwrap_or_else(|| Ok("".to_owned()))?;
        dbg!(Err(AssemblerError::Fail { msg: repr }))
    }

    // BUG the file is saved in any case EVEN if there is a crash in the assembler later
    // TODO delay the save but retreive the data now
    pub fn visit_save(
        &mut self,
        filename: &str,
        address: Option<&Expr>,
        size: Option<&Expr>,
        save_type: Option<&SaveType>,
        dsk_filename: Option<&String>,
        _side: Option<&Expr>
    ) -> Result<(), AssemblerError> {
        if cfg!(target_arch = "wasm32") {
            return Err(AssemblerError::AssemblingError {
                msg: "SAVE directive is not allowed in a web-based assembling.".to_owned()
            });
        }

        let from = match address {
            Some(address) => Some(self.resolve_expr_must_never_fail(address)?.int()?),
            None => None
        };
        let size = match size {
            Some(size) => Some(self.resolve_expr_must_never_fail(size)?.int()?),
            None => None
        };

        let page_info = self.active_page_info_mut();
        page_info.add_save_command(SaveCommand::new(
            from,
            size,
            filename.to_owned(),
            save_type.cloned(),
            dsk_filename.cloned()
        ));

        Ok(())
    }

    pub fn visit_charset(&mut self, format: &CharsetFormat) -> Result<(), AssemblerError> {
        let mut new_charset = CharsetEncoding::new();
        std::mem::swap(&mut new_charset, &mut self.charset_encoding);
        new_charset.update(format, self)?;
        std::mem::swap(&mut new_charset, &mut self.charset_encoding); // XXX lost in case of error
        Ok(())
    }

    pub fn visit_snainit(&mut self, fname: &str) -> Result<(), AssemblerError> {
        if !self.pass.is_first_pass() {
            return Ok(());
        }

        if self.byte_written {
            return Err(AssemblerError::AssemblingError {
                msg: format!(
                    "Some bytes has alrady been produced; you cannot import the snapshot {}.",
                    fname
                )
            });
        }
        self.sna = Snapshot::load(fname).map_err(|e| {
            AssemblerError::AssemblingError {
                msg: format!("Error while loading snapshot. {}", e)
            }
        })?;
        dbg!(self.sna.memory_size_header());
        dbg!(self
            .sna
            .chunks()
            .iter()
            .map(|c| String::from_utf8_lossy(c.code()))
            .collect_vec());
        self.sna.unwrap_memory_chunks();
        dbg!(self.sna.memory_size_header());
        dbg!(self
            .sna
            .chunks()
            .iter()
            .map(|c| String::from_utf8_lossy(c.code()))
            .collect_vec());
        Ok(())
    }

    pub fn visit_snaset(
        &mut self,
        flag: &cpclib_sna::SnapshotFlag,
        value: &cpclib_sna::FlagValue
    ) -> Result<(), AssemblerError> {
        self.sna
            .set_value(*flag, value.as_u16().unwrap())
            .map_err(|e| e.into())
    }

    pub fn visit_incbin(&mut self, data: &[u8]) -> Result<(), AssemblerError> {
        self.output_bytes(data)
    }

    /// Handle a crunched section.
    /// bytes generated during previous pass or previous loop are provided TO NOT crunched them an additional time if they are similar
    pub fn visit_crunched_section<'tokens, T: Visited + ListingElement + MayHaveSpan + Sync>(
        &mut self,
        kind: &CrunchType,
        lst: &mut [ProcessedToken<'tokens, T>],
        previous_bytes: &mut Option<Vec<u8>>,
        previous_crunched_bytes: &mut Option<Vec<u8>>,
        span: Option<&Z80Span>
    ) -> Result<(), AssemblerError>
    where
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
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

        let could_display_warning_message = self.active_page_info().limit != 0xFFFF
            || !self.active_page_info().protected_areas.is_empty();

        // from here, the modifications to the memory will be forgotten afterwise.
        // for this reason everything is done in a cloned environnement
        // TODO to have a more stable memory function, see if we can keep some steps between the passes
        // TODO OR play all the passes directly now
        let mut crunched_env = self.clone();
        crunched_env.crunched_section_state = CrunchedSectionState::new(span.cloned()).into();
        // codeadr stays the same
        crunched_env.active_page_info_mut().logical_outputadr = 0; // relocated output at 0 to be sure to have 64kb available
                                                                   // XXX probably a wrong behavior/to see with users

        crunched_env.active_page_info_mut().startadr = None; // reset the counter to obtain the bytes
        crunched_env.active_page_info_mut().maxadr = 0;
        crunched_env.active_page_info_mut().limit = 0xFFFF; // disable limit (to be redone in the area)
        crunched_env.active_page_info_mut().protected_areas.clear(); // remove protected areas
        crunched_env.output_address = 0;

        self.output_trigger
            .as_mut()
            .map(|t| t.enter_crunched_section());

        visit_processed_tokens(lst, &mut crunched_env).map_err(|e| {
            dbg!(&self.pass, &crunched_env.pass);
            let e = AssemblerError::CrunchedSectionError { error: e.into() };
            match span {
                Some(span) => {
                    AssemblerError::RelocatedError {
                        error: e.into(),
                        span: span.clone()
                    }
                }
                None => e
            }
        })?;
        self.output_trigger
            .as_mut()
            .map(|t| t.leave_crunched_section());

        // get the new data and crunch it if needed
        let bytes = crunched_env.produced_bytes();

        let must_crunch = previous_bytes
            .as_ref()
            .map(|b| b.as_slice() != bytes.as_slice())
            .unwrap_or(true);
        if must_crunch {
            let crunched: Vec<u8> = if bytes.is_empty() {
                Vec::new()
            }
            else {
                kind.crunch(&bytes).map_err(|e| {
                    match span {
                        Some(span) => {
                            AssemblerError::RelocatedError {
                                error: e.into(),
                                span: span.clone()
                            }
                        }
                        None => e
                    }
                })?
            };
            previous_crunched_bytes.replace(crunched);
            previous_bytes.replace(bytes);
        }
        else {
        }

        let _bytes = previous_bytes.as_ref().unwrap();
        let crunched = previous_crunched_bytes.as_ref().unwrap();

        // inject the crunched data
        self.visit_incbin(&crunched).map_err(|e| {
            match span {
                Some(span) => {
                    AssemblerError::RelocatedError {
                        error: e.into(),
                        span: span.clone()
                    }
                }
                None => e
            }
        })?;

        // update the symbol table with the new symbols obtained in the crunched section
        std::mem::swap(self.symbols_mut(), crunched_env.symbols_mut());
        let can_skip_next_passes = (*self.can_skip_next_passes.read().unwrap().deref()
            & *crunched_env.can_skip_next_passes.read().unwrap())
        .into(); // report missing symbols from the crunched area to the current area
        let request_additional_pass = (*self.request_additional_pass.read().unwrap().deref()
            | *crunched_env.request_additional_pass.read().unwrap())
        .into();
        *self.can_skip_next_passes.write().unwrap() = can_skip_next_passes;
        *self.request_additional_pass.write().unwrap() = request_additional_pass;

        self.macro_seed = crunched_env.macro_seed;

        // TODO display ONLY if:
        // - no LIMIT/PROTECT has been used in the crunched area
        // - a possible forbidden write has been done (maybe too complex to implement)
        if could_display_warning_message {
            self.warnings.push(
                AssemblerWarning::AssemblingError{
                    msg: "Memory protection systems are disabled in crunched section. If you want to keep them, explicitely use LIMIT or PROTECT directives in the crunched section.".to_owned()
                }
            );
        }

        Ok(())
    }
}

impl Env {
    fn assemble_nop(&self, kind: Mnemonic, count: Option<&Expr>) -> Result<Bytes, AssemblerError> {
        let count = match count {
            Some(count) => self.resolve_expr_must_never_fail(count)?.int()?,
            None => 1
        };
        let mut bytes = Bytes::new();
        for _i in 0..count {
            match kind {
                Mnemonic::Nop => {
                    bytes.push(0);
                }
                Mnemonic::Nop2 => {
                    bytes.push(0xED);
                    bytes.push(0xFF);
                }
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
// ) -> Result<Env, AssemblerError>
// where
// <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
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
        let mut env = Env::default();
        env.options = options;

        // prefill the snapshot representation with something else than the default
        if let Some(sna) =  env.options.assemble_options().snapshot_model() {
            env.sna = sna.clone();
            env.sna_version = env.sna.version();
        }

        env.symbols = SymbolsTableCaseDependent::new(
            env.options().symbols().clone(),
            env.options().case_sensitive()
        );

        if let Some(builder) = &env.options().assemble_options().output_builder {
            env.output_trigger = ListingOutputTrigger {
                token: None,
                bytes: Vec::new(),
                builder: builder.clone(),
                start: 0,
                physical_address: PhysicalAddress::new(0, 0)
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
    pub fn visit_return(&mut self, e: &Expr) -> Result<(), AssemblerError> {
        debug_assert!(self.return_value.is_none());
        self.return_value = Some(self.resolve_expr_must_never_fail(e)?);
        Ok(())
    }

    pub fn user_defined_function(&self, name: &str) -> Result<&Function, AssemblerError> {
        match self.functions.get(name) {
            Some(f) => Ok(f),
            None => Err(AssemblerError::FunctionUnknown(name.to_owned()))
        }
    }

    pub fn any_function<'res>(
        &'res self,
        name: &'res str
    ) -> Result<&'res Function, AssemblerError> {
        match HardCodedFunction::by_name(name) {
            Some(f) => Ok(f),
            None => self.user_defined_function(name)
        }
    }
}

/// Visit the tokens during several passes by providing a specific symbol table.
/// Warning Listing output is only possible for LocatedToken
pub fn visit_tokens_all_passes_with_options<'token, T>(
    tokens: &'token [T],
    options: EnvOptions
) -> Result<
    (Vec<ProcessedToken<'token, T>>, Env),
    (Vec<ProcessedToken<'token, T>>, Env, AssemblerError)
>
where
    T: Visited + ToSimpleToken + Debug + Sync + ListingElement + MayHaveSpan,
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    assert!(!tokens.is_empty());

    let mut env = Env::new(options);
    let mut tokens = processed_token::build_processed_tokens_list(tokens, &mut env);
    loop {
        env.start_new_pass();
        // println!("[pass] {:?}", env.pass);

        if env.pass.is_finished() {
            break;
        }

        let res = processed_token::visit_processed_tokens(&mut tokens, &mut env);
        if let Err(e) = res {
            return Err((tokens, env, e));
        }
    }



    // Add an additional pass to build the listing (this way it is built only one time)
    if env.options().assemble_options().output_builder.is_some() {
        env.pass = AssemblingPass::ListingPass;
        env.start_new_pass();
        processed_token::visit_processed_tokens(&mut tokens, &mut env)
            .map_err(|e| eprintln!("{}", e))
            .expect("No error can arise in listing output mode; there is a bug somewhere");
    }


    env.cleanup_warnings();

    if let Some(trigger) = env.output_trigger.as_mut() {
        trigger.finish()
    }

    Ok((tokens, env))
}

/// Visit the tokens during a single pass. Is deprecated in favor to the mulitpass version
#[deprecated(note = "use visit_tokens_one_pass")]
pub fn visit_tokens<T: Visited>(tokens: &[T]) -> Result<Env, AssemblerError> {
    visit_tokens_one_pass(tokens)
}

/// Assemble the tokens doing one pass only (so symbols are not properly treated)
pub fn visit_tokens_one_pass<T: Visited>(tokens: &[T]) -> Result<Env, AssemblerError> {
    let mut env = Env::default();

    for token in tokens.iter() {
        token.visited(&mut env)?;
    }

    Ok(env)
}

/// Apply the effect of the localized token. Most of the action is delegated to visit_token.
/// The difference with the standard token is the ability to embed listing
pub fn visit_located_token(
    outer_token: &LocatedToken,
    env: &mut Env
) -> Result<(), AssemblerError> {
    let nb_warnings = env.warnings.len();

    // cheat on the lifetime of tokens
    let outer_token = unsafe { (outer_token as *const LocatedToken).as_ref().unwrap() };

    // XXX Maybe we have to uncomment it if some tokens are not included within the listing
    //env.handle_output_trigger(outer_token);

    let span = outer_token.span();
    match outer_token {
        LocatedToken::Standard { token, span } => {
            match token {
                Token::Assert(exp, txt) => {
                    visit_assert(exp, txt.as_ref(), env, Some(span.clone()))?;
                    Ok(())
                }

                Token::Breakpoint(expr) => env.visit_breakpoint(expr.as_ref(), Some(span.clone())),

                Token::Pause => {
                    env.visit_pause(Some(span.clone()));
                    Ok(())
                }

                Token::Print(ref exp) => {
                    env.visit_print(exp.as_ref(), Some(span.clone()));
                    Ok(())
                }

                Token::Struct(name, content) => {
                    env.visit_struct_definition(name, content.as_slice(), Some(span))
                        .map_err(|e| e.locate(span.clone()))
                }
                Token::Incbin { .. } | Token::Include(..) => {
                    panic!("Should never be called");
                    // if content.read().unwrap().is_none() {
                    // outer_token
                    // .read_referenced_file(&outer_token.context().1)
                    // .and_then(|_| visit_located_token(outer_token, env))
                    // .map_err(|e| e.locate(span.clone()))
                    // }
                    // else {
                    // env.visit_incbin(content.read().unwrap().as_ref().unwrap())
                    // }
                    // .map_err(|err| {
                    // AssemblerError::IncludedFileError {
                    // span: span.clone(),
                    // error: Box::new(err)
                    // }
                    // })
                }
                _ => {
                    token.visited(env).map_err(|err| {
                        AssemblerError::RelocatedError {
                            error: Box::new(err),
                            span: span.clone()
                        }
                    })
                }
            }
        }
        LocatedToken::Comment(..) => Ok(()),
        LocatedToken::Org { val1, val2, span } => {
            env.visit_org(val1, val2.as_ref())
            .map_err(|e| e.locate(span.clone()))
        }
        LocatedToken::Assign{label, expr, op, span} => {
            env.visit_assign(label, expr, op.as_ref())
            .map_err(|e| e.locate(span.clone()))
        }
        LocatedToken::Equ{label, expr, span} => {
                env.visit_equ(label, expr)
                .map_err(|e| e.locate(span.clone()))
        }
        LocatedToken::SetN{label, source, expr, span } => {
            env.visit_next_and_co(label, source, expr.as_ref(), true)
            .map_err(|e| e.locate(span.clone()))
        }
        LocatedToken::Next{label, source, expr, span} => {
            env.visit_next_and_co(label, source, expr.as_ref(), false)
            .map_err(|e| e.locate(span.clone()))
        }

        LocatedToken::Label(label) => {
            env.visit_label(label.as_str())
                .map_err(|e| e.locate(label.clone()))
        }

        LocatedToken::Struct(name, args, span) => {
            env.visit_struct_definition(name.as_str(), args, Some(span))
        }
        LocatedToken::Defb(l, span) => {
            visit_db_or_dw_or_str(DbLikeKind::Defb, l.as_ref(), env)
                .map_err(|e| e.locate(span.clone()))
        }
        LocatedToken::Defw(l, span) => {
            visit_db_or_dw_or_str(DbLikeKind::Defw, l.as_ref(), env)
                .map_err(|e| e.locate(span.clone()))
        }
        LocatedToken::Str(l, span) => {
            visit_db_or_dw_or_str(DbLikeKind::Str, l.as_ref(), env)
                .map_err(|e| e.locate(span.clone()))
        }

        LocatedToken::Module(..)
        | LocatedToken::WarningWrapper(..)
        | LocatedToken::CrunchedSection(..)
        | LocatedToken::Function(..)
        | LocatedToken::If(..)
        | LocatedToken::MacroCall(..)
        | LocatedToken::Repeat(..)
        | LocatedToken::RepeatUntil(..)
        | LocatedToken::Rorg(..)
        | LocatedToken::While(..)
        | LocatedToken::Iterate(..)
        | LocatedToken::For { .. }
        | LocatedToken::Switch(..)
        | LocatedToken::Confined(..)
        | LocatedToken::Include(..)
        | LocatedToken::Incbin { .. }
        | LocatedToken::Macro { .. } => panic!("Should never been called {:?}", outer_token)
    }?;

    // Patch the warnings to inject them a location
    let nb_additional_warnings = env.warnings.len() - nb_warnings;
    for i in 0..nb_additional_warnings {
        let warning = &mut env.warnings[i + nb_warnings];
        if !warning.is_located() {
            *warning = AssemblerError::RelocatedWarning {
                warning: Box::new(warning.clone()),
                span: span.clone()
            };
        }
        // TODO check why it has been done this way
        //      maybe source code is not retrained and there are random crashes ?
        //     anyway I comment it now because it breaks warning merge
    //    
    //    *warning = AssemblerError::AssemblingError {
    //        msg: (*warning).to_string()
    //    }
    }

    env.move_delayed_commands_of_functions();

    Ok(())
}

/// Apply the effect of the token
fn visit_token(token: &Token, env: &mut Env) -> Result<(), AssemblerError> {
    env.update_dollar();
    // dbg!(token, env.active_page_info());
    match token {
        Token::Align(ref boundary, ref fill) => env.visit_align(boundary, fill.as_ref()),
        Token::Assert(ref exp, ref txt) => {
            visit_assert(exp, txt.as_ref(), env, None)?;
            Ok(())
        }
        Token::Basic(ref variables, ref hidden_lines, ref code) => {
            env.visit_basic(variables.as_ref(), hidden_lines.as_ref(), code)
        }
        Token::BuildSna(ref v) => env.visit_buildsna(v.as_ref()),
        Token::Bank(ref exp) => env.visit_bank(exp.as_ref()),
        Token::Bankset(ref v) => env.visit_bankset(v),
        Token::Breakpoint(ref exp) => env.visit_breakpoint(exp.as_ref(), None),
        Token::Org(ref address, ref address2) => env.visit_org(address, address2.as_ref()),
        Token::Defb(l) => visit_db_or_dw_or_str(DbLikeKind::Defb, l.as_ref(), env),
        Token::Defw(l) => visit_db_or_dw_or_str(DbLikeKind::Defw, l.as_ref(), env),
        Token::Str(l) => visit_db_or_dw_or_str(DbLikeKind::Str, l.as_ref(), env),
        Token::Defs(_) => visit_defs(token, env),
        Token::End => visit_end(env),
        Token::OpCode(ref mnemonic, ref arg1, ref arg2, ref arg3) => {
            visit_opcode(*mnemonic, &arg1, &arg2, &arg3, env)?;
            // Compute duration only if it is necessary
            if !env.stable_counters.is_empty() {
                let duration = token.estimated_duration()?;
                env.stable_counters.update_counters(duration);
            }
            Ok(())
        }
        Token::Comment(_) => Ok(()), // Nothing to do for a comment
        Token::List => {
            env.output_trigger.as_mut().map(|l| {
                l.on();
            });
            Ok(())
        }
        Token::NoList => {
            env.output_trigger.as_mut().map(|l| {
                l.off();
            });
            Ok(())
        }
        Token::Include(_, _namespace, _once) => {
            panic!("ERROR - Should never be executed");/*
            if *once {
                unimplemented!("ONCE on hardcoded tokens");
            }

            if let Some(namespace) = namespace.as_ref() {
                env.enter_namespace(namespace)?;
            }
            env.visit_listing(cell.read().unwrap().as_ref().unwrap())?;
            if namespace.is_some() {
                env.leave_namespace()?;
            }
            */
            Ok(())
        }
        Token::Include(_fname, _namespace, _once) => {
            panic!("ERROR - Should never be executed");
        }
        Token::Incbin {
            fname: _,
            offset: _,
            length: _,
            extended_offset: _,
            off: _,
            transformation: _
        } => panic!("Error - should never be called"),

        Token::If(ref _cases, ref _other) => {
            panic!("Should be handled by ProcessedToken")
        }
        Token::Label(ref label) => env.visit_label(label),
        Token::Limit(ref exp) => env.visit_limit(exp),
        Token::MultiPush(ref regs) => env.visit_multi_pushes(regs),
        Token::MultiPop(ref regs) => env.visit_multi_pops(regs),
        Token::NoExport(ref labels) => env.visit_noexport(labels.as_slice()),
        Token::Export(ref labels) => env.visit_export(labels.as_slice()),
        Token::Equ(ref label, ref exp) => env.visit_equ(label, exp),
        Token::Assign(ref label, ref exp, op) => env.visit_assign(label, exp, op.as_ref()),
        Token::Pause => {
            env.visit_pause(None);
            Ok(())
        }
        Token::Protect(ref start, ref end) => env.visit_protect(start, end),
        Token::Print(ref exp) => {
            env.visit_print(exp.as_ref(), None);
            Ok(())
        }
        Token::Fail(ref exp) => env.visit_fail(exp.as_ref().map(|v| v.as_slice())),
        Token::Repeat(..) => {
            panic!("Should never be called")
        }
        Token::Run(address, gate_array) => env.visit_run(address, gate_array.as_ref()),
        Token::Rorg(ref _exp, ref _code) => panic!("Is delegated to ProcessedToken"),
        Token::Save {
            filename,
            address,
            size,
            save_type,
            dsk_filename,
            side
        } => {
            env.visit_save(
                filename,
                address.as_ref(),
                size.as_ref(),
                save_type.as_ref(),
                dsk_filename.as_ref(),
                side.as_ref()
            )
        }
        Token::Charset(format) => env.visit_charset(format),
        Token::SnaSet(flag, value) => env.visit_snaset(flag, value),
        Token::StableTicker(ref ticker) => visit_stableticker(ticker, env),
        Token::Undef(ref label) => env.visit_undef(label),
        Token::Macro(_name, _arguments, _code) => {
            panic!("Is delegated to ProcessedToken")
        }
        Token::MacroCall(_name, _parameters) => panic!("Should never been called"),
        Token::Struct(name, content) => env.visit_struct_definition(name, content.as_slice(), None),
        Token::WaitNops(count) => env.visit_waitnops(count),
        Token::Next(label, source, delta) => {
            env.visit_next_and_co(label, source, delta.as_ref(), false)
        }
        Token::SnaInit(fname) => env.visit_snainit(fname),
        Token::SetN(label, source, delta) => {
            env.visit_next_and_co(label, source, delta.as_ref(), true)
        }
        Token::Range(name, start, stop) => env.visit_range(name, start, stop),
        Token::Section(name) => env.visit_section(name),

        Token::Return(exp) => env.visit_return(exp),
        _ => unimplemented!("{:?}", token)
    }?;

    env.move_delayed_commands_of_functions();
    Ok(())
}

/// No error is generated here; everything is delayed at the end of assembling.
/// Returns false in case of assert failure
fn visit_assert(
    exp: &Expr,
    txt: Option<&Vec<FormattedExpr>>,
    env: &mut Env,
    span: Option<Z80Span>
) -> Result<bool, AssemblerError> {
    let res = match env.resolve_expr_must_never_fail(exp) {
        Err(e) => Err(e),

        Ok(value) => {
            if !value.bool()? {
                let _symbols = env.symbols();
                let oper = |left: &Expr, right: &Expr, oper: &str| -> String {
                    let res_left = left.resolve(env).unwrap();
                    let res_right = right.resolve(env).unwrap();

                    match (&res_left, &res_right) {
                        (ExprResult::Char(c1), ExprResult::Char(c2)) => {
                            format!("['{}' {} '{}']", *c1 as char, oper, *c2 as char)
                        }
                        _ => {
                            format!("[{} {} {}] ", res_left, oper, res_right)
                                + format!("[0x{:x} {} 0x{:x}] ", res_left, oper, res_right).as_str()
                        }
                    }
                };

                let prefix = match exp {
                    Expr::BinaryOperation(BinaryOperation::Equal, box left, box right) => {
                        oper(left, right, "==")
                    }

                    Expr::BinaryOperation(BinaryOperation::LowerOrEqual, box left, box right) => {
                        oper(left, right, "<=")
                    }

                    Expr::BinaryOperation(BinaryOperation::GreaterOrEqual, box left, box right) => {
                        oper(left, right, ">=")
                    }

                    Expr::BinaryOperation(BinaryOperation::StrictlyLower, box left, box right) => {
                        oper(left, right, "<")
                    }

                    Expr::BinaryOperation(
                        BinaryOperation::StrictlyGreater,
                        box left,
                        box right
                    ) => oper(left, right, ">"),

                    _ => "".to_string()
                };

                Err(AssemblerError::AssertionFailed {
                    msg: prefix
                        + (if txt.is_some() {
                            env.build_string_from_formatted_expression(txt.unwrap())?
                        }
                        else {
                            "".to_owned()
                        })
                        .as_str(),
                    test: exp.to_string(),
                    guidance: env.to_assert_string(exp)
                })
            }
            else {
                Ok(())
            }
        }
    };

    if let Err(assert_error) = res {
        let assert_error = if let Some(span) = span {
            assert_error.locate(span)
        }
        else {
            assert_error
        };
        env.active_page_info_mut()
            .add_failed_assert_command(assert_error.into());
        Ok(false)
    }
    else {
        Ok(true)
    }
}

impl Env {
    pub fn visit_while<'token, E, T>(
        &mut self,
        cond: &E,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), AssemblerError>
    where
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        while self.resolve_expr_must_never_fail(cond)?.bool()? {
            // generate the bytes
            visit_processed_tokens(code, self).map_err(|e| {
                AssemblerError::WhileIssue {
                    error: Box::new(e),
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
        E: ExprEvaluationExt,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync
    >(
        &mut self,
        counter_name: &str,
        values: either::Either<&Vec<E>, &E>,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), AssemblerError>
    where
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        let counter_name = format!("{{{}}}", counter_name);
        let counter_name = counter_name.as_str();
        if self.symbols().contains_symbol(counter_name)? {
            return Err(AssemblerError::RepeatIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: format!("Counter {} already exists", counter_name)
                    }
                )))
                .into(),
                span: span.cloned(),
                repetition: 0
            });
        }

        // Get the values (all args or list explosion)
        // BUG: iteration over values make the expressions progressively evaluated, while iteration over a list make its expressions evaluated at first loop
        match values {
            either::Either::Left(values) => {
                for (i, value) in values.iter().enumerate() {
                    let counter_value = self.resolve_expr_must_never_fail(value).map_err(|e| {
                        AssemblerError::RepeatIssue {
                            error: Box::new(e),
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
            }
            either::Either::Right(values) => {
                match self.resolve_expr_must_never_fail(values)? {
                    ExprResult::List(values) => {
                        for (i, counter_value) in values.into_iter().enumerate() {
                            self.inner_visit_repeat(
                                Some(counter_name),
                                Some(counter_value),
                                i as _,
                                code,
                                span.clone()
                            )?;
                        }
                    }
                    _ => {
                        return Err(AssemblerError::AssemblingError {
                            msg: format!("REPEAT issue: {} is not a list", values)
                        })
                    }
                }
            }
        }

        // Apply the iteration

        // TODO restore a previous value if any
        self.symbols_mut().remove_symbol(counter_name)?;

        Ok(())
    }

    pub fn visit_rorg<'token, T, E>(
        &mut self,
        address: &E,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), AssemblerError>
    where
        E: ExprEvaluationExt,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        // Get the next code address
        let address = self
            .resolve_expr_must_never_fail(address)
            .map_err(|error| {
                match span {
                    Some(span) => {
                        AssemblerError::RelocatedError {
                            error: Box::new(error),
                            span: span.clone()
                        }
                    }
                    None => error
                }
            })?
            .int()?;

        // do not change the output address
        {
            let page_info = self.active_page_info_mut();
            page_info.logical_codeadr = address as _;
        }

        self.update_dollar();
        let value = self.active_page_info().logical_codeadr;

        self.output_trigger
            .as_mut()
            .map(|o| o.replace_code_address(value.into()));

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
    ) -> Result<(), AssemblerError>
    where
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
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
        confined_env.active_page_info_mut().limit = 0xFFFF; // disable limit (to be redone in the area)
        confined_env.active_page_info_mut().protected_areas.clear(); // remove protected areas
        confined_env.output_address = 0;
        // TODO: forbid a subset of instructions to ensure it works properly
        visit_processed_tokens(lst, &mut confined_env).map_err(|e| {
            panic!("{:?}", e);
            match span {
                Some(span) => e.locate(span.clone()),
                None => e
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
                Some(span) => return Err(e.locate(span.clone())),
                None => return Err(e)
            }
        }

        // Add the delta if needed and recompute the confined section a second time to properly setup the side effects
        if ((self.logical_code_address().wrapping_add(bytes_len)) & 0xFF00)
            != self.logical_code_address() & 0xFF00
        {
            while (self.logical_code_address() & 0x00FF) != 0x0000 {
                self.output(0)?;
                self.update_dollar();
            }
        }

        visit_processed_tokens(lst, self)
    }

    /// Handle the for directive
    pub fn visit_for<'token, E: ExprEvaluationExt, T>(
        &mut self,
        label: &str,
        start: &E,
        stop: &E,
        step: Option<&E>,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), AssemblerError>
    where
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        let counter_name = format!("{{{}}}", label);
        if self.symbols().contains_symbol(&counter_name)? {
            return Err(AssemblerError::ForIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: format!("Counter {} already exists", &counter_name)
                    }
                )))
                .into(),
                span: span.cloned()
            });
        }

        let mut counter_value = self.resolve_expr_must_never_fail(start)?;
        let stop = self.resolve_expr_must_never_fail(stop)?;
        let step = match step {
            Some(step) => self.resolve_expr_must_never_fail(step)?,
            None => 1i32.into()
        };

        let zero = ExprResult::from(0i32);

        if step == zero {
            return Err(AssemblerError::ForIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: format!("Infinite loop")
                    }
                )))
                .into(),
                span: span.cloned()
            });
        }

        if step < zero {
            return Err(AssemblerError::ForIssue {
                error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                    AssemblerError::AssemblingError {
                        msg: format!("Negative step is not yet handled")
                    }
                )))
                .into(),
                span: span.cloned()
            });
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

        self.symbols_mut().remove_symbol(counter_name)?;

        Ok(())
    }

    /// Handle the standard repetition directive
    pub fn visit_repeat_until<'token, E, T>(
        &mut self,
        cond: &E,
        code: &mut [ProcessedToken<'token, T>],
        span: Option<&Z80Span>
    ) -> Result<(), AssemblerError>
    where
        E: ExprEvaluationExt,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        let mut i = 0;
        loop {
            i = i + 1;
            self.inner_visit_repeat(None, None, i as _, code, span.clone())?;
            let res = self.resolve_expr_must_never_fail(cond)?;
            if res.bool()? {
                break;
            }
        }

        Ok(())
    }

    /// Handle the statndard repetition directive
    pub fn visit_repeat<'token, T, E>(
        &mut self,
        count: &E,
        code: &mut [ProcessedToken<'token, T>],
        counter: Option<&str>,
        counter_start: Option<&E>,
        span: Option<&Z80Span>
    ) -> Result<(), AssemblerError>
    where
        E: ExprEvaluationExt,
        T: ListingElement<Expr = E> + Visited + MayHaveSpan + Sync,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        ProcessedToken<'token, T>: FunctionBuilder
    {
        // get the number of loops
        let count = self.resolve_expr_must_never_fail(count)?.int()?;

        // get the counter name of any
        let counter_name = counter.as_ref().map(|counter| format!("{{{}}}", counter));
        let counter_name = counter_name.as_ref().map(|s| s.as_str());
        if let Some(counter_name) = counter_name {
            if self.symbols().contains_symbol(counter_name)? {
                return Err(AssemblerError::RepeatIssue {
                    error: AssemblerError::ExpressionError(ExpressionError::OwnError(Box::new(
                        AssemblerError::AssemblingError {
                            msg: format!("Counter {} already exists", counter_name)
                        }
                    )))
                    .into(),
                    span: span.cloned(),
                    repetition: 0
                });
            }
        }

        // get the first value
        let mut counter_value = counter_start
            .map(|start| self.resolve_expr_must_never_fail(start))
            .unwrap_or(Ok(REPEAT_START_VALUE.into()))?;

        for i in 0..count {
            self.inner_visit_repeat(
                counter_name,
                Some(counter_value.clone()),
                i as _,
                code,
                span.clone()
            )?;
            // handle the counter update
            counter_value += 1i32.into();
        }

        if let Some(counter_name) = counter_name {
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
    ) -> Result<(), AssemblerError>
    where
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
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
        }

        // generate the bytes
        visit_processed_tokens(code, self).map_err(|e| {
            let e = if let AssemblerError::RelocatedError {
                error: box AssemblerError::UnknownSymbol { closest: _, symbol },
                span
            } = &e
            {
                dbg!(symbol, counter_name);
                if let Some(counter_name) = counter_name {
                    if counter_name == &format!("{{{}}}", symbol) {
                        AssemblerError::RelocatedError {
                            error: Box::new(AssemblerError::UnknownSymbol {
                                closest: Some(counter_name.into()),
                                symbol: symbol.clone()
                            }),
                            span: span.clone()
                        }
                    }
                    else {
                        e
                    }
                }
                else {
                    e
                }
            }
            else {
                e
            };

            AssemblerError::RepeatIssue {
                error: Box::new(e),
                span: span.cloned(),
                repetition: iteration as _
            }
        })?;

        // handle the end of visibility of unique labels
        self.symbols_mut().pop_seed();

        Ok(())
    }

    /// Generate a string that is helpfull for assertion understanding (i.e. show the operation and evaluate the rest)
    /// Crash if expression cannot be computed
    fn to_assert_string(&self, exp: &Expr) -> String {
        let format = |oper, left, right| {
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
                }

                None => format!("0x{:x}", self.resolve_expr_must_never_fail(exp).unwrap())
            }
        }
        else {
            format!("0x{:x}", self.resolve_expr_must_never_fail(exp).unwrap())
        }
    }

    fn visit_run(&mut self, address: &Expr, ga: Option<&Expr>) -> Result<(), AssemblerError> {
        let address = self.resolve_expr_may_fail_in_first_pass(address)?.int()?;

        self.output_trigger
            .as_mut()
            .map(|o| o.replace_code_address(address.into()));

        if self.run_options.is_some() {
            return Err(AssemblerError::RunAlreadySpecified);
        }
        self.sna
            .set_value(cpclib_sna::SnapshotFlag::Z80_PC, address as _)?;

        match ga {
            None => {
                self.run_options = Some((address as _, None));
            }
            Some(ga_expr) => {
                let ga_expr = self.resolve_expr_may_fail_in_first_pass(ga_expr)?.int()?;
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

    pub fn cleanup_warnings(&mut self) {
        // Filter the warnings to merge overriding
        let mut current_warning_idx = 1; // index to the last warning to treat
        let mut previous_warning_idx = 0; // index to the previous warning treated (diff with current_warning_idx can be higher than 1 when there are several consecutive warnings for OverrideMemory)

        while current_warning_idx < self.warnings.len() {
            // Check if we need to fuse successive override memory warnings
            let (new_size, new_span) = match (&self.warnings[previous_warning_idx], &self.warnings[current_warning_idx]) {

                // we fuse two consecutive override memory warnings
                (
                    AssemblerWarning::OverrideMemory(prev_addr, prev_size),
                    AssemblerWarning::OverrideMemory(curr_addr, curr_size),
                ) => {
                    if (prev_addr.offset_in_cpc() + *prev_size as u32)  == curr_addr.offset_in_cpc() {
                        (Some(*prev_size + *curr_size), None)
                    } else {
                        (None, None)
                    }
                },

                (
                    AssemblerError::RelocatedWarning {warning: box AssemblerWarning::OverrideMemory(prev_addr, prev_size), span:prev_span},
                    AssemblerError::RelocatedWarning {warning: box AssemblerWarning::OverrideMemory(curr_addr, curr_size),
                    span: curr_span}
                ) => {
                    if prev_addr.offset_in_cpc() + *prev_size as u32  == curr_addr.offset_in_cpc() {

                        let new_size = *prev_size + *curr_size;

                        let start_str = prev_span.fragment();
                        let end_str = curr_span.fragment();
                        let start_str = start_str.as_bytes();
                        let end_str = end_str.as_bytes();

                        let start_ptr = &start_str[0] as *const u8;
                        let end_last_ptr = &end_str[end_str.len()-1] as *const u8;
                        assert!(end_last_ptr > start_ptr);
                        let txt = unsafe { 
                            let slice = std::slice::from_raw_parts(start_ptr, end_last_ptr.offset_from(start_ptr) as _);
                            std::str::from_utf8(slice).unwrap()
                         };

                        let new_span = unsafe { 
                            use cpclib_common::LocatedSpan;
                            Z80Span(LocatedSpan::new_from_raw_offset(
                                prev_span.location_offset(),
                                prev_span.location_line(),
                                txt,
                                prev_span.0.extra
                            ))
                        };

                        (Some(new_size), Some(new_span))
                    } else {
                        (None, None)
                    }
                },

                _ => {/* nothing to do ATM */
                    (None, None)
                }
            };

            if let Some(new_size) = new_size {

                if let Some(new_span) = new_span {
                    if let AssemblerError::RelocatedWarning {warning: box  AssemblerWarning::OverrideMemory(_prev_addr, ref mut prev_size), ref mut span} = &mut self.warnings[previous_warning_idx] {
                        *prev_size = new_size;
                        *span = new_span;
                    }
                } else {
                    if let AssemblerWarning::OverrideMemory(_prev_addr, ref mut prev_size) = &mut self.warnings[previous_warning_idx] {
                        *prev_size = new_size;
                    }
                }

            } else {
                previous_warning_idx += 1;
                if previous_warning_idx != current_warning_idx {
                    self.warnings.swap(previous_warning_idx, current_warning_idx);
                }
            }

            current_warning_idx += 1;
        }
        // change the length  of the vector to remove all eated ones
        self.warnings.truncate(previous_warning_idx+1);


        // transform the warnings as strings
        self.warnings.iter_mut()
            .for_each(|w| *w = AssemblerWarning::AssemblingError {
                        msg: (*w).to_string()
            } );


    }
}

impl Env {
fn visit_equ<E: ExprEvaluationExt + ExprElement + Debug>(&mut self, label: &str, exp: &E) -> Result<(), AssemblerError> {
    if self.symbols().contains_symbol(label)? && self.pass.is_first_pass() {
        Err(AssemblerError::AlreadyDefinedSymbol {
            symbol: label.into(),
            kind: self.symbols().kind(label)?.into()
        })
    }
    else {
        let value = self.resolve_expr_may_fail_in_first_pass(exp)?;
        self.output_trigger
            .as_mut()
            .map(|o| o.replace_code_address(value.clone()));
        self.add_symbol_to_symbol_table(label, value)
    }
}



fn visit_assign<'e, E: ExprEvaluationExt + ExprElement + Clone>(
    &mut self,
    label: &str,
    exp: &E,
    op: Option<&BinaryOperation>,
) -> Result<(), AssemblerError> 
{
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

    self.output_trigger
        .as_mut()
        .map(|o| o.replace_code_address(value.clone()));

    self.symbols_mut().assign_symbol_to_value(label, value)?;

    Ok(())
}
}

fn visit_defs(token: &Token, env: &mut Env) -> Result<(), AssemblerError> {
    match token {
        Token::Defs(l) => {
            for (e, f) in l.iter() {
                let bytes = assemble_defs_item(e, f.as_ref(), env)?;
                env.output_bytes(&bytes)?;
            }
            Ok(())
        }
        _ => unreachable!()
    }
}

fn visit_end(_env: &mut Env) -> Result<(), AssemblerError> {
    eprintln!("END directive is not implemented");
    Ok(())
}

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

// TODO refactor code with assemble_opcode or other functions manipulating bytes
pub fn visit_db_or_dw_or_str<E: ExprEvaluationExt + ExprElement>(
    kind: DbLikeKind,
    exprs: &[E],
    env: &mut Env
) -> Result<(), AssemblerError> {
    let mask = kind.mask();

    let output = |env: &mut Env, val: i32, mask: u16| -> Result<(), AssemblerError> {
        if mask == 0xFF {
            env.output(val as u8)?;
        }
        else {
            let high = ((val & 0xFF00) >> 8) as u8;
            let low = (val & 0xFF) as u8;
            env.output(low)?;
            env.output(high)?;
        }
        Ok(())
    };

    let output_expr_result = |env: &mut Env, expr: ExprResult, mask: u16| {
        match &expr {
            ExprResult::Float(_)
            | ExprResult::Value(_)
            | ExprResult::Bool(_)
            | ExprResult::Char(_) => output(env, expr.int()?, mask),
            ExprResult::String(s) => {
                for c in s.chars() {
                    output(env, c as _, mask)?;
                }
                Ok(())
            }
            ExprResult::List(l) => {
                for c in l {
                    output(env, c.int()?, mask)?;
                }
                Ok(())
            }
            ExprResult::Matrix { .. } => {
                for row in expr.matrix_rows() {
                    for c in row.list_content() {
                        output(env, c.int()?, mask)?;
                    }
                }
                Ok(())
            }
        }
    };

    let backup_address = env.logical_output_address();
    for exp in exprs.iter() {
        if exp.is_string() {
            let s = exp.string();
            let bytes = env.charset_encoding.transform_string(s);
            for b in &bytes {
                output(env, *b as _, mask)?
            }
            env.update_dollar();
        }
        else if exp.is_char() {
            let c = exp.char();
            let b = env.charset_encoding.transform_char(c);
            output(env, b as _, mask)?;
            env.update_dollar();
        }
        else {
            let val = env.resolve_expr_may_fail_in_first_pass(exp)?;
            output_expr_result(env, val, mask)?;
            env.update_dollar();
        }
    }

    // Patch the last char of a str
    if matches!(kind, DbLikeKind::Str) && backup_address < env.logical_output_address() {
        let last_address = env.logical_output_address() - 1;
        let last_address = env.logical_to_physical_address(last_address as _);
        let last_value = env.peek(&last_address);
        env.poke(last_value | 0x80, &last_address);
    }

    Ok(())
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
    pub fn visit_basic<S: Borrow<str> + Display>(
        &mut self,
        variables: Option<&Vec<S>>,
        hidden_lines: Option<&Vec<u16>>,
        code: &str
    ) -> Result<(), AssemblerError> {
        let bytes = self.assemble_basic(variables, hidden_lines, code)?;

        // If the basic directive is the VERY first thing to output,
        // we assume startadr is 0x170 as for any basic program
        if self.start_address().is_none() {
            self.active_page_info_mut().logical_outputadr = 0x170;
            self.active_page_info_mut().logical_codeadr = self.logical_output_address();
            self.active_page_info_mut().startadr = Some(self.logical_output_address());
            self.output_address = 0x170;
        }

        self.output_bytes(&bytes)
    }

    pub fn assemble_basic<S: Borrow<str> + Display>(
        &mut self,
        variables: Option<&Vec<S>>,
        hidden_lines: Option<&Vec<u16>>,
        code: &str
    ) -> Result<Vec<u8>, AssemblerError> {
        // Build the final basic code by replacing variables by value
        // Hexadecimal is used to ensure a consistent 2 bytes representation
        let basic_src = {
            let mut basic = code.to_owned();
            match variables {
                None => {}
                Some(arguments) => {
                    for argument in arguments {
                        let key = format!("{{{}}}", argument);
                        let value = format!(
                            "&{:X}",
                            self.resolve_expr_may_fail_in_first_pass(&Expr::from(
                                argument.borrow()
                            ))?
                        );
                        basic = basic.replace(&key, &value);
                    }
                }
            }
            basic
        };

        // build the basic tokens
        let mut basic = BasicProgram::parse(basic_src)?;
        if hidden_lines.is_some() {
            basic.hide_lines(hidden_lines.unwrap())?;
        }
        Ok(basic.as_bytes())
    }
}

/// When visiting a repetition, we unroll the loop and stream the tokens
/// TODO reimplement it in a similar way that the LocatedToken version that is better
pub fn visit_repeat(rept: &Token, env: &mut Env) -> Result<(), AssemblerError> {
    let tokens = rept.unroll(env).unwrap()?;

    for token in &tokens {
        visit_token(token, env)?;
    }

    Ok(())
}

/// Manage the stable ticker stuff.
/// - Start: register a counter
/// - Stop: store counter count
#[allow(clippy::cast_possible_wrap)]
pub fn visit_stableticker(
    ticker: &StableTickerAction,
    env: &mut Env
) -> Result<(), AssemblerError> {
    match ticker {
        StableTickerAction::Start(ref name) => {
            env.stable_counters.add_counter(name)?;
            Ok(())
        }
        StableTickerAction::Stop => {
            match env.stable_counters.release_last_counter() {
                None => Err(AssemblerError::NoActiveCounter),
                Some((label, count)) => {
                    if env.pass.is_listing_pass() {
                        // force the injection of the value
                        env.symbols_mut()
                            .set_symbol_to_value(label, count)?;
                        Ok(())
                    } else {
                        env.add_symbol_to_symbol_table(&label, count)
                    }
                }
            }
        }
    }
}

/// Assemble DEFS directive
pub fn assemble_defs_item(
    expr: &Expr,
    fill: Option<&Expr>,
    env: &mut Env
) -> Result<Bytes, AssemblerError> {
    let count = match env.resolve_expr_must_never_fail(expr) {
        Ok(amount) => amount.int()?,
        Err(e) => {
            env.add_error_discardable_one_pass(e)?;
            *env.request_additional_pass.write().unwrap() = true; // we expect to obtain this value later
            0.into()
        }
    };
    let value = if fill.is_none() {
        0
    }
    else {
        let value = env
            .resolve_expr_may_fail_in_first_pass(fill.unwrap())?
            .int()?;
        (value & 0xFF) as u8
    };

    let mut bytes = Bytes::with_capacity(count as usize);
    bytes.resize_with(count as _, || value);

    Ok(bytes)
}

/// Assemble align directive. It can only work if current address is known...
pub fn assemble_align(
    expr: &Expr,
    fill: Option<&Expr>,
    env: &Env
) -> Result<Bytes, AssemblerError> {
    let expression = env.resolve_expr_must_never_fail(expr)?.int()? as u16;
    let current = env.symbols().current_address()?;
    let value = if fill.is_none() {
        0
    }
    else {
        let value = env
            .resolve_expr_may_fail_in_first_pass(fill.unwrap())?
            .int()?;
        (value & 0xFF) as u8
    };

    // compute the number of 0 to put
    let mut until = current;
    while until % expression != 0 {
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

/// Assemble the opcode and inject in the environement
pub(crate) fn visit_opcode(
    mnemonic: Mnemonic,
    arg1: &Option<DataAccess>,
    arg2: &Option<DataAccess>,
    arg3: &Option<Register8>,
    env: &mut Env
) -> Result<(), AssemblerError> {
    // TODO update $ in the symbol table
    let bytes = assemble_opcode(mnemonic, arg1, arg2, arg3, env)?;
    for b in bytes.iter() {
        env.output(*b)?;
    }

    Ok(())
}

/// Assemble an opcode and returns the generated bytes or the error message if it is impossible to
/// assemblea.
/// We assum the opcode is properlt coded. Panic occurs if it is not the case
pub fn assemble_opcode(
    mnemonic: Mnemonic,
    arg1: &Option<DataAccess>,
    arg2: &Option<DataAccess>,
    arg3: &Option<Register8>,
    env: &mut Env
) -> Result<Bytes, AssemblerError> {
    match mnemonic {
        Mnemonic::And | Mnemonic::Or | Mnemonic::Xor => {
            assemble_logical_operator(mnemonic, arg1.as_ref().unwrap(), env)
        }
        Mnemonic::Add | Mnemonic::Adc => {
            assemble_add_or_adc(
                mnemonic,
                arg1.as_ref().unwrap(),
                arg2.as_ref().unwrap(),
                env
            )
        }
        Mnemonic::Cp => env.assemble_cp(arg1.as_ref().unwrap()),
        Mnemonic::ExMemSp => assemble_ex_memsp(arg1.as_ref().unwrap()),
        Mnemonic::Dec | Mnemonic::Inc => assemble_inc_dec(mnemonic, arg1.as_ref().unwrap(), env),
        Mnemonic::Djnz => assemble_djnz(arg1.as_ref().unwrap(), env),
        Mnemonic::In => assemble_in(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), env),
        Mnemonic::Ld => assemble_ld(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), env),
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
        | Mnemonic::Rrd => assemble_no_arg(mnemonic),
        Mnemonic::Out => assemble_out(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), env),
        Mnemonic::Jr | Mnemonic::Jp | Mnemonic::Call => {
            assemble_call_jr_or_jp(mnemonic, arg1.as_ref(), arg2.as_ref().unwrap(), env)
        }
        Mnemonic::Pop => assemble_pop(arg1.as_ref().unwrap()),
        Mnemonic::Push => assemble_push(arg1.as_ref().unwrap()),
        Mnemonic::Bit | Mnemonic::Res | Mnemonic::Set => {
            assemble_bit_res_or_set(
                mnemonic,
                arg1.as_ref().unwrap(),
                arg2.as_ref().unwrap(),
                arg3.as_ref(),
                env
            )
        }
        Mnemonic::Ret => assemble_ret(arg1),
        Mnemonic::Rst => assemble_rst(arg1.as_ref().unwrap(), env),
        Mnemonic::Im => assemble_im(arg1.as_ref().unwrap(), env),
        Mnemonic::Nop => env.assemble_nop(Mnemonic::Nop, arg1.as_ref().map(|v| v.expr().unwrap())),
        Mnemonic::Nop2 => env.assemble_nop(Mnemonic::Nop2, None),

        Mnemonic::Sub => env.assemble_sub(arg1.as_ref().unwrap()),
        Mnemonic::Sbc => env.assemble_sbc(arg1.as_ref().unwrap(), arg2.as_ref().unwrap()),
        Mnemonic::Sla
        | Mnemonic::Sra
        | Mnemonic::Srl
        | Mnemonic::Sl1
        | Mnemonic::Rl
        | Mnemonic::Rr
        | Mnemonic::Rlc
        | Mnemonic::Rrc => env.assemble_shift(mnemonic, arg1.as_ref().unwrap(), arg2.as_ref())
    }
}



fn assemble_no_arg(mnemonic: Mnemonic) -> Result<Bytes, AssemblerError> {
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
            return Err(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!("{} not treated", mnemonic)
            });
        }
    };

    Ok(Bytes::from_slice(bytes))
}

fn assemble_inc_dec(mne: Mnemonic, arg1: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    let is_inc = match mne {
        Mnemonic::Inc => true,
        Mnemonic::Dec => false,
        _ => panic!("Impossible case")
    };

    match arg1 {
        DataAccess::Register16(ref reg) => {
            let base = if is_inc { 0b0000_0011 } else { 0b0000_1011 };
            let byte = base | (register16_to_code_with_sp(*reg) << 4);
            bytes.push(byte);
        }

        DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(*reg));
            bytes.push(if is_inc { 0x23 } else { 0x2B });
        }

        DataAccess::Register8(ref reg) => {
            bytes.push(
                if is_inc { 0b0000_0100 } else { 0b0000_0101 } | (register8_to_code(*reg) << 3)
            );
        }

        DataAccess::IndexRegister8(ref reg) => {
            bytes.push(indexed_register16_to_code(reg.complete()));
            bytes.push(
                if is_inc { 0b0000_0100 } else { 0b0000_0101 }
                    | (indexregister8_to_code(*reg) << 3)
            );
        }

        DataAccess::MemoryRegister16(Register16::Hl) => {
            bytes.push(if is_inc { 0x34 } else { 0x35 });
        }

        DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
            let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;

            bytes.push(indexed_register16_to_code(*reg));
            bytes.push(if is_inc { 0x34 } else { 0x35 });
            bytes.push(val);
        }
        _ => {
            return Err(AssemblerError::BugInAssembler {
                file: file!(),
                line: line!(),
                msg: format!(
                    "{}: not implemented for {:?}",
                    mne.to_string().to_owned(),
                    arg1
                )
            });
        }
    }

    Ok(bytes)
}

/// Converts an absolute address to a relative one (relative to $)
pub fn absolute_to_relative<T: AsRef<SymbolsTable>>(
    address: i32,
    opcode_delta: i32,
    sym: T
) -> Result<u8, AssemblerError> {
    match sym.as_ref().current_address() {
        Err(_msg) => Err(AssemblerError::UnknownAssemblingAddress),
        Ok(root) => {
            let delta = (address - i32::from(root)) - opcode_delta;
            if delta > 128 || delta < -127 {
                Err(AssemblerError::InvalidArgument {
                    msg: format!(
                        "Address 0x{:x} relative to 0x{:x} is too far {}",
                        address, root, delta
                    )
                })
            }
            else {
                let res = (delta & 0xFF) as u8;
                Ok(res)
            }
        }
    }
}

fn assemble_ret(arg1: &Option<DataAccess>) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    if arg1.is_some() {
        if let Some(&DataAccess::FlagTest(ref test)) = arg1.as_ref() {
            let flag = flag_test_to_code(*test);
            bytes.push(0b1100_0000 | (flag << 3));
        }
        else {
            return Err(AssemblerError::InvalidArgument {
                msg: format!("RET: wrong argument for ret")
            });
        }
    }
    else {
        bytes.push(0xC9);
    };

    Ok(bytes)
}

fn assemble_rst(arg1: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();
    let val = env
        .resolve_expr_may_fail_in_first_pass(arg1.get_expression().unwrap())?
        .int()?;

    let p = match val {
        0x00 => 0b000,
        0x08 => 0b001,
        0x10 => 0b010,
        0x18 => 0b011,
        0x20 => 0b100,
        0x28 => 0b101,
        0x30 => 0b110,
        0x38 => 0b111,

        // just for convenience
        10 => 0b010,
        18 => 0b011,
        20 => 0b100,
        28 => 0b101,
        30 => 0b110,
        38 => 0b111,
        _ => {
            return Err(AssemblerError::InvalidArgument {
                msg: format!("RST cannot take {} as argument.", val)
            })
        }
    };

    bytes.push(0b11000111 | p << 3);
    Ok(bytes)
}

fn assemble_im(arg1: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();
    let val = env
        .resolve_expr_may_fail_in_first_pass(arg1.get_expression().unwrap())?
        .int()?;

    let code = match val {
        0x00 => 0x46,
        0x01 => 0x56,
        0x02 => 0x5E,
        _ => {
            return Err(AssemblerError::InvalidArgument {
                msg: format!("IM cannot take {} as argument.", val)
            })
        }
    };

    bytes.push(0xED);
    bytes.push(code);
    Ok(bytes)
}

/// arg1 contains the tests
/// arg2 contains the information
pub fn assemble_call_jr_or_jp(
    mne: Mnemonic,
    arg1: Option<&DataAccess>,
    arg2: &DataAccess,
    env: &mut Env
) -> Result<Bytes, AssemblerError> {
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

    // compute the flag code if any
    // TODO raise an error if the flag test for jr is wrong
    let flag_code = if arg1.is_some() {
        match arg1.as_ref() {
            Some(DataAccess::FlagTest(ref test)) => Some(flag_test_to_code(*test)),
            _ => {
                return Err(AssemblerError::InvalidArgument {
                    msg: format!(
                        "{}: wrong flag argument",
                        mne.to_string().to_ascii_uppercase()
                    )
                })
            }
        }
    }
    else {
        None
    };

    // Treat address
    if let DataAccess::Expression(ref e) = arg2 {
        let address = env.resolve_expr_may_fail_in_first_pass(e)?.int()?;
        if is_jr {
            let relative = if e.is_relative() {
                address as u8
            }
            else {
                env.absolute_to_relative_may_fail_in_first_pass(address, 2)? as u8
            };
            if flag_code.is_some() {
                // jr - flag
                add_byte(&mut bytes, 0b0010_0000 | (flag_code.unwrap() << 3));
            }
            else {
                // jr - no flag
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
                // jp - flag
                add_byte(&mut bytes, 0b1100_0010 | (flag_code.unwrap() << 3))
            }
            else {
                // jp - no flag
                add_byte(&mut bytes, 0xC3);
            }
            add_word(&mut bytes, address as u16);
        }

        env.track_used_symbols(e);
    }
    else if let DataAccess::MemoryRegister16(Register16::Hl) = arg2 {
        assert!(is_jp);
        add_byte(&mut bytes, 0xE9);
    }
    else if let DataAccess::MemoryIndexRegister16(ref reg) = arg2 {
        assert!(is_jp);
        add_byte(&mut bytes, indexed_register16_to_code(*reg));
        add_byte(&mut bytes, 0xE9);
    }
    else {
        return Err(AssemblerError::BugInAssembler {
            file: file!(),
            line: line!(),
            msg: format!("{}: parameter {:?} not treated", mne, arg2)
        });
    }

    Ok(bytes)
}

fn assemble_djnz(arg1: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    if let DataAccess::Expression(ref expr) = arg1 {
        let mut bytes = Bytes::new();
        let address = env.resolve_expr_may_fail_in_first_pass(expr)?.int()?;
        let relative = if expr.is_relative() {
            address as u8
        }
        else {
            env.absolute_to_relative_may_fail_in_first_pass(address, 1 + 1)? as u8
        };
        bytes.push(0x10);
        bytes.push(relative);

        Ok(bytes)
    }
    else {
        unreachable!()
    }
}

#[allow(missing_docs)]
impl Env {
    pub fn assemble_cp(&mut self, arg: &DataAccess) -> Result<Bytes, AssemblerError> {
        let mut bytes = Bytes::new();

        match arg {
            DataAccess::Register8(ref reg) => {
                add_byte(&mut bytes, 0b1011_1000 + register8_to_code(*reg));
            }

            DataAccess::IndexRegister8(ref reg) => {
                add_byte(&mut bytes, indexed_register16_to_code(reg.complete()));
                add_byte(&mut bytes, 0b1011_1000 + indexregister8_to_code(*reg));
            }
            DataAccess::Expression(ref exp) => {
                add_byte(&mut bytes, 0xFE);
                add_byte(
                    &mut bytes,
                    self.resolve_expr_may_fail_in_first_pass(exp)?.int()? as _
                );
            }

            DataAccess::MemoryRegister16(Register16::Hl) => {
                add_byte(&mut bytes, 0xBE);
            }

            DataAccess::IndexRegister16WithIndex(ref reg, ref idx) => {
                add_byte(&mut bytes, indexed_register16_to_code(*reg));
                add_byte(&mut bytes, 0xBE);
                add_byte(
                    &mut bytes,
                    self.resolve_expr_may_fail_in_first_pass(idx)?.int()? as _
                );
            }

            _ => unreachable!()
        }

        Ok(bytes)
    }

    pub fn assemble_sub(&mut self, arg: &DataAccess) -> Result<Bytes, AssemblerError> {
        let mut bytes = Bytes::new();

        match arg {
            DataAccess::Expression(ref exp) => {
                let val = (self.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;
                bytes.push(0xD6);
                bytes.push(val);
            }

            DataAccess::Register8(ref reg) => {
                bytes.push(0b10010000 + (register8_to_code(*reg)));
            }

            DataAccess::IndexRegister8(ref reg) => {
                bytes.push(indexed_register16_to_code(reg.complete()));
                bytes.push(0b10010000 + (indexregister8_to_code(*reg)));
            }

            DataAccess::MemoryRegister16(Register16::Hl) => {
                bytes.push(0x96);
            }

            DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                let val = (self.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;

                bytes.push(indexed_register16_to_code(*reg));
                bytes.push(0x96);
                bytes.push(val);
            }
            _ => {
                unreachable!();
            }
        }

        Ok(bytes)
    }

    pub fn assemble_sbc(
        &mut self,
        arg1: &DataAccess,
        arg2: &DataAccess
    ) -> Result<Bytes, AssemblerError> {
        let mut bytes = Bytes::new();

        if arg1.is_register_a() {
            match arg2 {
                DataAccess::Register8(ref reg) => {
                    bytes.push(0b10011000 + register8_to_code(*reg));
                }

                DataAccess::IndexRegister8(ref reg) => {
                    bytes.push(indexed_register16_to_code(reg.complete()));
                    bytes.push(0b10011000 + indexregister8_to_code(*reg));
                }

                DataAccess::Expression(ref exp) => {
                    let val = self.resolve_expr_may_fail_in_first_pass(exp)?.int()? as u8;
                    bytes.push(0xDE);
                    bytes.push(val);
                }

                DataAccess::MemoryRegister16(Register16::Hl) => {
                    bytes.push(0x9E);
                }

                DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                    bytes.push(indexed_register16_to_code(*reg));
                    bytes.push(0x9E);
                    let val = self.resolve_expr_may_fail_in_first_pass(exp)?.int()? as u8;
                    bytes.push(val);
                }

                _ => unreachable!()
            }
        }
        else {
            assert!(arg1.is_register_hl());

            match arg2 {
                DataAccess::Register16(ref reg) => {
                    bytes.push(0xED);
                    bytes.push(0b0100_0010 | register16_to_code_with_sp(*reg) << 4);
                }
                _ => unreachable!()
            }
        }

        Ok(bytes)
    }

    pub fn assemble_shift(
        &mut self,
        mne: Mnemonic,
        target: &DataAccess,
        hidden: Option<&DataAccess>
    ) -> Result<Bytes, AssemblerError> {
        let mut bytes = Bytes::new();

        if let DataAccess::Register8(ref reg) = target {
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
            } + register8_to_code(*reg);
            add_byte(&mut bytes, byte);
        }
        else {
            assert!(match target {
                DataAccess::MemoryRegister16(Register16::Hl) => true,
                DataAccess::IndexRegister16WithIndex(..) => true,
                _ => false
            });

            // add prefix for ix/iy
            match target {
                DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                    let val = self.resolve_expr_may_fail_in_first_pass(exp)?.int()? as u8;
                    bytes.push(indexed_register16_to_code(*reg));
                    add_byte(&mut bytes, 0xCB);
                    bytes.push(val);
                }

                DataAccess::MemoryRegister16(Register16::Hl) => {
                    add_byte(&mut bytes, 0xCB);
                }

                _ => {
                    return Err(AssemblerError::InvalidArgument {
                        msg: format!("{} cannot take {} as argument", mne, target)
                    })
                }
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
                    byte -= delta.abs() as u8;
                }
                else {
                    byte += delta as u8;
                }
            }
            bytes.push(byte);
        }

        Ok(bytes)
    }
}

fn assemble_ld(arg1: &DataAccess, arg2: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    // Destination is 8bits register
    if let DataAccess::Register8(ref dst) = arg1 {
        let dst = register8_to_code(*dst);
        match arg2 {
            DataAccess::Register8(ref src) => {
                // R. Zaks p 297
                let src = register8_to_code(*src);

                let code = 0b0100_0000 + (dst << 3) + src;
                bytes.push(code);
            }

            DataAccess::IndexRegister8(ref src) => {
                bytes.push(indexed_register16_to_code(src.complete()));
                let src = indexregister8_to_code(*src);
                let code = 0b0100_0000 + (dst << 3) + src;
                bytes.push(code);
            }

            DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;

                bytes.push(0b0000_0110 | (dst << 3));
                bytes.push(val);
            }

            DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                let val = env.resolve_expr_may_fail_in_first_pass(exp)?.int()?;

                add_index_register_code(&mut bytes, *reg);
                add_byte(&mut bytes, 0b0100_0110 | (dst << 3));
                add_index(&mut bytes, val)?;
            }

            DataAccess::MemoryRegister16(Register16::Hl) => {
                add_byte(&mut bytes, 0b0100_0110 | (dst << 3));
            }
            DataAccess::MemoryIndexRegister16(reg) => {
                add_index_register_code(&mut bytes, *reg);
                add_byte(&mut bytes, 0b0100_0110 | (dst << 3));
            }

            DataAccess::MemoryRegister16(ref memreg) if arg1.is_register_a() => {
                let byte = match memreg {
                    Register16::Bc => 0x0A,
                    Register16::De => 0x1A,
                    _ => unreachable!()
                };
                add_byte(&mut bytes, byte);
            }

            DataAccess::Memory(ref expr) => {
                // dst is A
                let val = env.resolve_expr_may_fail_in_first_pass(expr)?.int()?;
                add_byte(&mut bytes, 0x3A);
                add_word(&mut bytes, val as _);
            }

            DataAccess::SpecialRegisterI => {
                assert!(arg1.is_register_a());
                bytes.push(0xED);
                bytes.push(0x57);
            }

            DataAccess::SpecialRegisterR => {
                assert!(arg1.is_register_a());
                bytes.push(0xED);
                bytes.push(0x5F);
            }

            _ => {
                return Err(AssemblerError::BugInAssembler {
                    file: file!(),
                    line: line!(),
                    msg: format!("LD: not properly implemented for '{:?}, {:?}'", arg1, arg2)
                });
            }
        }
    }
    // Destination is 16 bits register
    else if let DataAccess::Register16(ref dst) = arg1 {
        let dst_code = register16_to_code_with_sp(*dst);

        match arg2 {
            DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFFFF) as u16;

                add_byte(&mut bytes, 0b0000_0001 | (dst_code << 4));
                add_word(&mut bytes, val);
            }

            DataAccess::Register16(Register16::Hl) if dst.is_sp() => {
                add_byte(&mut bytes, 0xF9);
            }

            DataAccess::IndexRegister16(ref reg) if dst.is_sp() => {
                add_byte(&mut bytes, indexed_register16_to_code(*reg));
                add_byte(&mut bytes, 0xF9);
            }

            // Fake instruction splitted in 2 bits operations
            DataAccess::Register16(ref src) => {
                println!("{:?}, {:?}", dst.split(), src.split());
                let bytes_high = assemble_ld(
                    &DataAccess::Register8(dst.high().unwrap()),
                    &DataAccess::Register8(src.high().unwrap()),
                    env
                )
                .unwrap();
                let bytes_low = assemble_ld(
                    &DataAccess::Register8(dst.low().unwrap()),
                    &DataAccess::Register8(src.low().unwrap()),
                    env
                )
                .unwrap();

                bytes.extend_from_slice(&bytes_low);
                bytes.extend_from_slice(&bytes_high);
            }

            DataAccess::Memory(ref expr) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(expr)?.int()? & 0xFFFF) as u16;

                if let Register16::Hl = dst {
                    add_byte(&mut bytes, 0x2A);
                    add_word(&mut bytes, val);
                }
                else {
                    add_byte(&mut bytes, 0xED);
                    add_byte(
                        &mut bytes,
                        (register16_to_code_with_sp(*dst) << 4) + 0b0100_1011
                    );
                    add_word(&mut bytes, val);
                }
            }

            _ => {}
        }
    }
    else if let DataAccess::IndexRegister8(ref dst) = arg1 {
        add_byte(&mut bytes, indexed_register16_to_code(dst.complete()));
        match arg2 {
            DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;
                bytes.push(0b0000_0110 | (indexregister8_to_code(*dst) << 3));
                bytes.push(val);
            }

            DataAccess::Register8(ref src) => {
                let code = register8_to_code(*src);

                let code = if dst.is_high() {
                    0b0110_0000 + code
                }
                else {
                    0x68 + code
                };
                bytes.push(code);
            }

            DataAccess::IndexRegister8(ref src) => {
                assert_eq!(dst.complete(), src.complete());

                let byte = match (dst.is_low(), src.is_low()) {
                    (false, false) => 0x64,
                    (false, true) => 0x65,
                    (true, false) => 0x6C,
                    (true, true) => 0x6D
                };
                bytes.push(byte)
            }

            _ => unreachable!()
        }
    }
    // Distinatin is 16 bits indexed register
    else if let DataAccess::IndexRegister16(ref dst) = arg1 {
        let code = indexed_register16_to_code(*dst);

        match arg2 {
            DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFFFF) as u16;

                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x21);
                add_word(&mut bytes, val);
            }

            DataAccess::Memory(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFFFF) as u16;

                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x2A);
                add_word(&mut bytes, val);
            }
            _ => {}
        }
    }
    // Destination is memory indexed by register
    else if let DataAccess::MemoryRegister16(ref dst) = arg1 {
        // Want to store in memory pointed by register
        match dst {
            Register16::Hl => {
                if let DataAccess::Register8(ref src) = arg2 {
                    let src = register8_to_code(*src);
                    let code = 0b0111_0000 | src;
                    bytes.push(code);
                }
                else if let DataAccess::Expression(ref exp) = arg2 {
                    let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;
                    bytes.push(0x36);
                    bytes.push(val);
                }
            }

            Register16::De if DataAccess::Register8(Register8::A) == *arg2 => {
                bytes.push(0b0001_0010);
            }

            Register16::Bc if DataAccess::Register8(Register8::A) == *arg2 => {
                bytes.push(0b0000_0010);
            }

            _ => {}
        }
    }
    else if let DataAccess::MemoryIndexRegister16(ref dst) = arg1 {
        add_index_register_code(&mut bytes, *dst);
        add_byte(&mut bytes, indexed_register16_to_code(*dst));

        if let DataAccess::Register8(ref src) = arg2 {
            let src = register8_to_code(*src);
            let code = 0b0111_0000 | src;
            bytes.push(code);
        }
        else if let DataAccess::Expression(ref exp) = arg2 {
            let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;
            bytes.push(0x36);
            bytes.push(val);
        }
    }
    // Destination is memory form ix/iy + n
    else if let DataAccess::IndexRegister16WithIndex(ref reg, ref exp) = arg1 {
        add_byte(&mut bytes, indexed_register16_to_code(*reg));
        let delta = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;

        match arg2 {
            DataAccess::Expression(ref exp) => {
                let value = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;
                add_byte(&mut bytes, 0x36);
                add_byte(&mut bytes, delta);
                add_byte(&mut bytes, value);
            }

            DataAccess::Register8(ref src) => {
                add_byte(&mut bytes, 0x70 + register8_to_code(*src));
                add_byte(&mut bytes, delta);
            }
            _ => {
                // possible fake instruction
                bytes.clear();
            }
        }
    }
    // Destination is memory
    else if let DataAccess::Memory(ref exp) = arg1 {
        let address = env.resolve_expr_may_fail_in_first_pass(exp)?.int()?;

        match arg2 {
            DataAccess::IndexRegister16(IndexRegister16::Ix) => {
                bytes.push(0xDD);
                bytes.push(0b0010_0010);
                add_word(&mut bytes, address as _);
            }
            DataAccess::IndexRegister16(IndexRegister16::Iy) => {
                bytes.push(0xFD);
                bytes.push(0b0010_0010);
                add_word(&mut bytes, address as _);
            }
            DataAccess::Register16(Register16::Hl) => {
                bytes.push(0b0010_0010);
                add_word(&mut bytes, address as _);
            }
            DataAccess::Register16(ref reg) => {
                bytes.push(0xED);
                bytes.push(0b0100_0011 | (register16_to_code_with_sp(*reg) << 4));
                add_word(&mut bytes, address as _);
            }
            DataAccess::Register8(Register8::A) => {
                bytes.push(0x32);
                add_word(&mut bytes, address as _);
            }

            _ => {}
        }
    }
    else if let DataAccess::SpecialRegisterI = arg1 {
        if let DataAccess::Register8(Register8::A) = arg2 {
            bytes.push(0xED);
            bytes.push(0x47)
        }
        else {
            unreachable!();
        }
    }
    else if let DataAccess::SpecialRegisterR = arg1 {
        if let DataAccess::Register8(Register8::A) = arg2 {
            bytes.push(0xED);
            bytes.push(0x4F)
        }
        else {
            unreachable!();
        }
    }

    // handle fake instructions
    if bytes.is_empty() {
        match (arg1, arg2) {
            (DataAccess::Register16(dst), DataAccess::Register16(src)) => {
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.low().unwrap()),
                        &DataAccess::Register8(src.low().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.high().unwrap()),
                        &DataAccess::Register8(src.high().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
            }

            (DataAccess::Register16(Register16::Hl), DataAccess::IndexRegister16(_))
            | (DataAccess::IndexRegister16(_), DataAccess::Register16(Register16::Hl))
            | (DataAccess::IndexRegister16(_), DataAccess::IndexRegister16(_)) => {
                bytes.extend(assemble_push(arg2)?);
                bytes.extend(assemble_pop(arg1)?);
            }

            // general registers from indexed
            (DataAccess::Register16(dst), DataAccess::IndexRegister16(src)) => {
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.low().unwrap()),
                        &DataAccess::IndexRegister8(src.low()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.high().unwrap()),
                        &DataAccess::IndexRegister8(src.high()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
            }
            // general > indexed
            (DataAccess::IndexRegister16(dst), DataAccess::Register16(src)) => {
                bytes.extend(
                    assemble_ld(
                        &DataAccess::IndexRegister8(dst.low()),
                        &DataAccess::Register8(src.low().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(
                    assemble_ld(
                        &DataAccess::IndexRegister8(dst.high()),
                        &DataAccess::Register8(src.high().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
            }

            (DataAccess::Register16(dst), DataAccess::IndexRegister16WithIndex(src, index)) => {
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.low().unwrap()),
                        &DataAccess::IndexRegister16WithIndex(src.clone(), index.clone()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.high().unwrap()),
                        &DataAccess::IndexRegister16WithIndex(src.clone(), index.add(1)),
                        env
                    )?
                    .iter()
                    .cloned()
                );
            }
            (DataAccess::IndexRegister16WithIndex(dst, index), DataAccess::Register16(src)) => {
                bytes.extend(
                    assemble_ld(
                        &DataAccess::IndexRegister16WithIndex(dst.clone(), index.clone()),
                        &DataAccess::Register8(src.low().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(
                    assemble_ld(
                        &DataAccess::IndexRegister16WithIndex(dst.clone(), index.add(1)),
                        &DataAccess::Register8(src.high().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
            }

            (DataAccess::Register16(dst), DataAccess::MemoryRegister16(Register16::Hl)) => {
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.low().unwrap()),
                        &DataAccess::MemoryRegister16(Register16::Hl),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(assemble_inc_dec(
                    Mnemonic::Inc,
                    &DataAccess::Register16(Register16::Hl),
                    env
                )?);
                bytes.extend(
                    assemble_ld(
                        &DataAccess::Register8(dst.high().unwrap()),
                        &DataAccess::MemoryRegister16(Register16::Hl),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(assemble_inc_dec(
                    Mnemonic::Dec,
                    &DataAccess::Register16(Register16::Hl),
                    env
                )?);
            }
            (DataAccess::MemoryRegister16(Register16::Hl), DataAccess::Register16(src)) => {
                bytes.extend(
                    assemble_ld(
                        &DataAccess::MemoryRegister16(Register16::Hl),
                        &DataAccess::Register8(src.low().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(assemble_inc_dec(
                    Mnemonic::Inc,
                    &DataAccess::Register16(Register16::Hl),
                    env
                )?);
                bytes.extend(
                    assemble_ld(
                        &DataAccess::MemoryRegister16(Register16::Hl),
                        &DataAccess::Register8(src.high().unwrap()),
                        env
                    )?
                    .iter()
                    .cloned()
                );
                bytes.extend(assemble_inc_dec(
                    Mnemonic::Dec,
                    &DataAccess::Register16(Register16::Hl),
                    env
                )?);
            }

            _ => {}
        }
    }

    if bytes.is_empty() {
        Err(AssemblerError::BugInAssembler {
            file: file!(),
            line: line!(),
            msg: format!("LD: not properly implemented for '{:?}, {:?}'", arg1, arg2)
        })
    }
    else {
        Ok(bytes)
    }
}

fn assemble_in(arg1: &DataAccess, arg2: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    if *arg1 == DataAccess::Expression(0.into()) {
        assert_eq!(arg2, &DataAccess::PortC);
        bytes.push(0xED);
        bytes.push(0x70);
    }
    else {
        match arg2 {
            DataAccess::PortC => {
                match arg1 {
                    DataAccess::Register8(ref reg) => {
                        bytes.push(0xED);
                        bytes.push(0b0100_0000 | (register8_to_code(*reg) << 3))
                    }
                    _ => panic!()
                }
            }

            DataAccess::PortN(ref exp) => {
                if let DataAccess::Register8(Register8::A) = arg1 {
                    let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;
                    bytes.push(0xDB);
                    bytes.push(val);
                }
            }

            _ => panic!("{:?}", arg2)
        };
    }

    if bytes.is_empty() {
        Err(AssemblerError::BugInAssembler {
            file: file!(),
            line: line!(),
            msg: format!("IN: not properly implemented for '{:?}, {:?}'", arg1, arg2)
        })
    }
    else {
        Ok(bytes)
    }
}

fn assemble_out(arg1: &DataAccess, arg2: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    if *arg2 == DataAccess::Expression(0.into()) {
        assert_eq!(arg1, &DataAccess::PortC);
        bytes.push(0xED);
        bytes.push(0x71);
    }
    else {
        match arg1 {
            DataAccess::PortC => {
                if let DataAccess::Register8(ref reg) = arg2 {
                    bytes.push(0xED);
                    bytes.push(0b0100_0001 | (register8_to_code(*reg) << 3))
                }

                if let DataAccess::Expression(Expr::Value(0)) = arg2 {
                    bytes.push(0xED);
                    bytes.push(0x71);
                }
            }

            DataAccess::PortN(ref exp) => {
                if let DataAccess::Register8(Register8::A) = arg2 {
                    let val = (env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF) as u8;
                    bytes.push(0xD3);
                    bytes.push(val);
                }
            }
            _ => {}
        };
    }

    if bytes.is_empty() {
        Err(AssemblerError::BugInAssembler {
            file: file!(),
            line: line!(),
            msg: format!("OUT: not properly implemented for '{:?}, {:?}'", arg1, arg2)
        })
    }
    else {
        Ok(bytes)
    }
}

fn assemble_pop(arg1: &DataAccess) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    match arg1 {
        DataAccess::Register16(ref reg) => {
            let byte = 0b1100_0001 | (register16_to_code_with_af(*reg) << 4);
            bytes.push(byte);
        }
        DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(*reg));
            bytes.push(0xE1);
        }
        _ => {
            return Err(AssemblerError::InvalidArgument {
                msg: format!("POP: not implemented for {:?}", arg1)
            });
        }
    }

    Ok(bytes)
}

fn assemble_push(arg1: &DataAccess) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    match arg1 {
        DataAccess::Register16(ref reg) => {
            let byte = 0b1100_0101 | (register16_to_code_with_af(*reg) << 4);
            bytes.push(byte);
        }
        DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(*reg));
            bytes.push(0xE5);
        }
        _ => {
            return Err(AssemblerError::InvalidArgument {
                msg: format!("PUSH: not implemented for {:?}", arg1)
            });
        }
    }

    Ok(bytes)
}

fn assemble_logical_operator(
    mnemonic: Mnemonic,
    arg1: &DataAccess,
    env: &Env
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    let memory_code = || {
        match mnemonic {
            Mnemonic::And => 0xA6,
            Mnemonic::Or => 0xB6,
            Mnemonic::Xor => 0xAE,
            _ => unreachable!()
        }
    };

    match arg1 {
        DataAccess::Register8(ref reg) => {
            let base = match mnemonic {
                Mnemonic::And => 0b1010_0000,
                Mnemonic::Or => 0b1011_0000,
                Mnemonic::Xor => 0b1010_1000,
                _ => unreachable!()
            };
            bytes.push(base + register8_to_code(*reg));
        }

        DataAccess::IndexRegister8(ref reg) => {
            bytes.push(indexed_register16_to_code(reg.complete()));
            let base = match mnemonic {
                Mnemonic::And => 0b1010_0000,
                Mnemonic::Or => 0b1011_0000,
                Mnemonic::Xor => 0b1010_1000,
                _ => unreachable!()
            };
            bytes.push(base + indexregister8_to_code(*reg));
        }

        DataAccess::Expression(ref exp) => {
            let base = match mnemonic {
                Mnemonic::And => 0xE6,
                Mnemonic::Or => 0xF6,
                Mnemonic::Xor => 0xEE,
                _ => unreachable!()
            };
            let value = env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF;
            bytes.push(base);
            bytes.push(value as u8);
        }

        DataAccess::MemoryRegister16(Register16::Hl) => {
            bytes.push(memory_code());
        }

        DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
            let value = env.resolve_expr_may_fail_in_first_pass(exp)?.int()? & 0xFF;
            bytes.push(indexed_register16_to_code(*reg));
            bytes.push(memory_code());
            bytes.push(value as u8);
        }
        _ => unreachable!()
    }

    Ok(bytes)
}

fn assemble_ex_memsp(arg1: &DataAccess) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    if let DataAccess::IndexRegister16(ref reg) = arg1 {
        bytes.push(indexed_register16_to_code(*reg));
    }

    bytes.push(0xE3);
    Ok(bytes)
}

fn assemble_add_or_adc(
    mnemonic: Mnemonic,
    arg1: &DataAccess,
    arg2: &DataAccess,
    env: &Env
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();
    let is_add = match mnemonic {
        Mnemonic::Add => true,
        Mnemonic::Adc => false,
        _ => panic!("Impossible case")
    };

    match arg1 {
        DataAccess::Register8(Register8::A) => {
            match arg2 {
                DataAccess::MemoryRegister16(Register16::Hl) => {
                    if is_add {
                        bytes.push(0b1000_0110);
                    }
                    else {
                        bytes.push(0b1000_1110);
                    }
                }

                DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                    let val = env.resolve_expr_may_fail_in_first_pass(exp)?.int()?;

                    // TODO check if the code is ok
                    bytes.push(indexed_register16_to_code(*reg));
                    if is_add {
                        bytes.push(0b1000_0110);
                    }
                    else {
                        bytes.push(0x8E);
                    }
                    add_index(&mut bytes, val)?;
                }

                DataAccess::Expression(ref exp) => {
                    let val = env.resolve_expr_may_fail_in_first_pass(exp)?.int()? as u8;
                    if is_add {
                        bytes.push(0b1100_0110);
                    }
                    else {
                        bytes.push(0xCE);
                    }
                    bytes.push(val);
                }

                DataAccess::Register8(ref reg) => {
                    let base = if is_add { 0b1000_0000 } else { 0b1000_1000 };
                    bytes.push(base | register8_to_code(*reg));
                }

                DataAccess::IndexRegister8(ref reg) => {
                    bytes.push(indexed_register16_to_code(reg.complete()));
                    let base = if is_add { 0b1000_0000 } else { 0b1000_1000 };
                    bytes.push(base | indexregister8_to_code(*reg));
                }
                _ => {}
            }
        }

        DataAccess::Register16(Register16::Hl) => {
            if let DataAccess::Register16(ref reg) = arg2 {
                let base = if is_add {
                    0b0000_1001
                }
                else {
                    bytes.push(0xED);
                    0b0100_1010
                };

                bytes.push(base | (register16_to_code_with_sp(*reg) << 4));
            }
        }

        DataAccess::IndexRegister16(ref reg1) => {
            match arg2 {
                DataAccess::Register16(ref reg2) => {
                    // TODO Error if reg2 = HL
                    bytes.push(indexed_register16_to_code(*reg1));
                    let base = if is_add {
                        0b0000_1001
                    }
                    else {
                        panic!();
                    };
                    bytes.push(
                        base | (register16_to_code_with_indexed(&DataAccess::Register16(*reg2))
                            << 4)
                    )
                }

                DataAccess::IndexRegister16(ref reg2) => {
                    if reg1 != reg2 {
                        return Err(AssemblerError::InvalidArgument {
                            msg: "Unable to add different indexed registers".to_owned()
                        });
                    }

                    bytes.push(indexed_register16_to_code(*reg1));
                    let base = if is_add {
                        0b0000_1001
                    }
                    else {
                        panic!();
                    };
                    bytes.push(
                        base | (register16_to_code_with_indexed(&DataAccess::IndexRegister16(
                            *reg2
                        )) << 4)
                    )
                }

                _ => {}
            }
        }
        _ => {}
    }

    if bytes.is_empty() {
        Err(AssemblerError::BugInAssembler {
            file: file!(),
            line: line!(),
            msg: format!("{:?} not implemented for {:?} {:?}", mnemonic, arg1, arg2)
        })
    }
    else {
        Ok(bytes)
    }
}

fn assemble_bit_res_or_set(
    mnemonic: Mnemonic,
    arg1: &DataAccess,
    arg2: &DataAccess,
    hidden: Option<&Register8>,
    env: &Env
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    // Get the bit of interest
    let bit = match arg1 {
        DataAccess::Expression(ref e) => {
            let bit = (env.resolve_expr_may_fail_in_first_pass(e)?.int()? & 0xFF) as u8;
            if bit > 7 {
                return Err(AssemblerError::InvalidArgument {
                    msg: format!("{}: {} is an invalid value", mnemonic.to_string(), bit)
                });
            }
            bit
        }
        _ => unreachable!()
    };

    // Get the code to differentiate the instructions
    // the value can be modified by some hidden instructions
    let code = match mnemonic {
        Mnemonic::Res => 0b1000_0000,
        Mnemonic::Set => 0b1100_0000,
        Mnemonic::Bit => 0b0100_0000,
        _ => unreachable!()
    };

    // Apply it to the right thing
    if let DataAccess::Register8(ref reg) = arg2 {
        //    let mut code = code + 0b0110;

        bytes.push(0xCB);
        bytes.push(code | (bit << 3) | register8_to_code(*reg))
    }
    else {
        assert!(match arg2 {
            DataAccess::MemoryRegister16(Register16::Hl) => true,
            DataAccess::IndexRegister16WithIndex(..) => true,
            _ => false
        });

        let mut code = code + 0b0110;

        if let DataAccess::IndexRegister16WithIndex(ref reg, delta) = arg2 {
            bytes.push(indexed_register16_to_code(*reg));
            add_byte(&mut bytes, 0xCB);
            let delta = (env.resolve_expr_may_fail_in_first_pass(delta)?.int()? & 0xFF) as u8;
            add_byte(&mut bytes, delta);

            // patch the code for hidden opcode
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
                    code -= fix.abs() as u8;
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
        _ => panic!("no mapping for {:?}", reg)
    }
}

fn register16_to_code_with_sp(reg: Register16) -> u8 {
    match reg {
        Register16::Bc => 0b00,
        Register16::De => 0b01,
        Register16::Hl => 0b10,
        Register16::Sp => 0b11,
        _ => panic!("no mapping for {:?}", reg)
    }
}

fn register16_to_code_with_indexed(reg: &DataAccess) -> u8 {
    match reg {
        DataAccess::Register16(Register16::Bc) => 0b00,
        DataAccess::Register16(Register16::De) => 0b01,
        DataAccess::IndexRegister16(_) => 0b10,
        DataAccess::Register16(Register16::Sp) => 0b11,
        _ => panic!("no mapping for {:?}", reg)
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

    fn visit_token(token: &Token, env: &mut Env) -> Result<(), AssemblerError> {
        let mut processed = build_processed_token(token, env);
        processed.visited(env)
    }

    fn visit_tokens(tokens: &[Token]) -> Result<Env, AssemblerError> {
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
        let res = assemble_call_jr_or_jp(
            Mnemonic::Jp,
            Some(&DataAccess::FlagTest(FlagTest::Z)),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &mut Env::default()
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

        assert!(visit_assert(
            &Expr::BinaryOperation(
                BinaryOperation::Equal,
                Box::new(0i32.into()),
                Box::new(0i32.into())
            ),
            None,
            &mut env,
            None
        )
        .unwrap());
        assert!(!visit_assert(
            &Expr::BinaryOperation(
                BinaryOperation::Equal,
                Box::new(1i32.into()),
                Box::new(0i32.into())
            ),
            None,
            &mut env,
            None
        )
        .unwrap());
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
        let env = Env::default();
        let res =
            assemble_inc_dec(Mnemonic::Inc, &DataAccess::Register16(Register16::De), &env).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x13);

        let res =
            assemble_inc_dec(Mnemonic::Dec, &DataAccess::Register8(Register8::B), &env).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x05);
    }

    #[test]
    pub fn test_res() {
        let env = Env::default();
        let res = assemble_bit_res_or_set(
            Mnemonic::Res,
            &DataAccess::Expression(0.into()),
            &DataAccess::Register8(Register8::B),
            None,
            &env
        )
        .unwrap();

        assert_eq!(res.as_ref(), &[0xCB, 0b10000000]);

        let env = Env::default();
        let res = assemble_bit_res_or_set(
            Mnemonic::Res,
            &DataAccess::Expression(2.into()),
            &DataAccess::Register8(Register8::C),
            None,
            &env
        )
        .unwrap();

        assert_eq!(res.as_ref(), &[0xCB, 0b10010001]);

        let env = Env::default();
        let res = assemble_bit_res_or_set(
            Mnemonic::Res,
            &DataAccess::Expression(2.into()),
            &DataAccess::MemoryRegister16(Register16::Hl),
            None,
            &env
        )
        .unwrap();

        assert_eq!(res.as_ref(), &[0xCB, 0b10010110]);

        let env = Env::default();
        let res = assemble_bit_res_or_set(
            Mnemonic::Res,
            &DataAccess::Expression(2.into()),
            &DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, 3.into()),
            None,
            &env
        )
        .unwrap();

        assert_eq!(res.as_ref(), &[0xDD, 0xCB, 3, 0b10010110]);

        let env = Env::default();
        let res = assemble_bit_res_or_set(
            Mnemonic::Res,
            &DataAccess::Expression(2.into()),
            &DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, 3.into()),
            Some(&Register8::B),
            &env
        )
        .unwrap();

        assert_eq!(res.as_ref(), &[0xDD, 0xCB, 3, 0b10010000]);
    }

    #[test]
    pub fn test_ld() {
        let res = assemble_ld(
            &DataAccess::Register16(Register16::De),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &Env::default()
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
        let _res = assemble_ld(
            &DataAccess::Register16(Register16::Af),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &Env::default()
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
        let env = env.unwrap();
        let bytes = env.memory(0, 2);
        assert_eq!(
            bytes[1],
            assemble_inc_dec(Mnemonic::Inc, &DataAccess::Register16(Register16::Hl), &env).unwrap()
                [0]
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

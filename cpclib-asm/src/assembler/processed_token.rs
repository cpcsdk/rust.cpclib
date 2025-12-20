use std::borrow::{Borrow, Cow};
use std::cell::OnceCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Formatter};
use std::ptr;
use std::sync::Arc;

use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::prelude::*;
use cpclib_disc::amsdos::AmsdosFileType;
use cpclib_tokens::symbols::{SymbolFor, SymbolsTableTrait};
use cpclib_tokens::{
    AssemblerControlCommand, AssemblerFlavor, BinaryTransformation, ListingElement,
    MacroParamElement, TestKindElement, ToSimpleToken, Token
};
use ouroboros::*;

use super::AssemblerWarning;
use super::control::ControlOutputStore;
use super::file::{get_filename_to_read, load_file, read_source};
use super::function::{Function, FunctionBuilder};
use super::r#macro::Expandable;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::instructions::Compressor;
use crate::preamble::{LocatedListing, MayHaveSpan, Z80Span};
use crate::progress::{self, Progress};
use crate::{
    AssemblerCompressionResult, AssemblerError, Env, LocatedToken, Visited, r#macro,
    parse_z80_with_context_builder
};

/// Tokens are read only elements extracted from the parser
/// ProcessedTokens allow to maintain their state during assembling
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedToken<'token, T: Visited + Debug + ListingElement + Sync> {
    /// The token being processed by the assembler
    token: &'token T,
    state: Option<ProcessedTokenState<'token, T>>
}

/// Specific state to maintain for the current token
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcessedTokenState<'token, T: Visited + ListingElement + Debug + Sync> {
    RestrictedAssemblingEnvironment {
        listing: SimpleListingState<'token, T>,
        commands: Option<ControlOutputStore>
    },
    Confined(SimpleListingState<'token, T>),
    CrunchedSection {
        /// The token to assemble
        listing: SimpleListingState<'token, T>,
        // The bytes previously generated - to be compared to avoid a second slow assembling
        previous_bytes: Option<Vec<u8>>,
        // The previous compressed flux - to reuse if needed
        previous_compressed_bytes: Option<AssemblerCompressionResult>
    },
    For(SimpleListingState<'token, T>),
    FunctionDefinition(FunctionDefinitionState),
    /// If state encodes previous choice
    If(IfState<'token, T>),
    /// Included file must read at some moment the file to handle
    Include(IncludeState),
    /// Included binary needs to be read
    /// TODO add parameters
    Incbin(IncbinState),

    Iterate(SimpleListingState<'token, T>),
    MacroCallOrBuildStruct(ExpandState),
    Module(SimpleListingState<'token, T>),
    RepeatToken(
        SingleTokenState<'token, T>,
        &'token <T as ListingElement>::Expr
    ),
    Repeat(SimpleListingState<'token, T>),
    RepeatUntil(SimpleListingState<'token, T>),
    While(SimpleListingState<'token, T>),
    Rorg(SimpleListingState<'token, T>),
    Switch(SwitchState<'token, T>),
    Warning(Box<ProcessedToken<'token, T>>)
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
struct IncbinState {
    contents: BTreeMap<Utf8PathBuf, Vec<u8>>
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct SingleTokenState<'token, T: Visited + ListingElement + Debug + Sync> {
    token: Box<ProcessedToken<'token, T>>
}

#[derive(PartialEq, Eq, Clone)]
struct SimpleListingState<'token, T: Visited + ListingElement + Debug + Sync> {
    processed_tokens: Vec<ProcessedToken<'token, T>>,
    span: Option<Z80Span>
}

impl<T: Visited + ListingElement + Debug + Sync> Debug for SimpleListingState<'_, T> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "SimpleListingState")
    }
}

impl<'token, T: Visited + ListingElement + Debug + Sync + MayHaveSpan> SimpleListingState<'token, T>
where <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    fn build(
        tokens: &'token [T],
        span: Option<Z80Span>,
        env: &mut Env
    ) -> Result<Self, AssemblerError> {
        Ok(Self {
            processed_tokens: build_processed_tokens_list(tokens, env)?,
            span
        })
    }

    fn tokens_mut(&mut self) -> &mut [ProcessedToken<'token, T>] {
        &mut self.processed_tokens
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionDefinitionState(Option<Arc<Function>>);

#[derive(PartialEq, Eq, Clone, Debug)]
struct SwitchState<'token, T: Visited + ListingElement + Debug + Sync> {
    cases: Vec<SimpleListingState<'token, T>>,
    default: Option<SimpleListingState<'token, T>>
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
struct IncludeState(BTreeMap<Utf8PathBuf, IncludeStateInner>);

impl IncludeState {
    /// By constructon fname exists and is correct
    fn retreive_listing(
        &mut self,
        env: &mut Env,
        fname: &Utf8PathBuf
    ) -> Result<&mut IncludeStateInner, AssemblerError> {
        if cfg!(target_arch = "wasm32") {
            return Err(AssemblerError::AssemblingError {
                msg: "INCLUDE-like directives are not allowed in a web-based assembling."
                    .to_owned()
            });
        }

        // Build the state if needed / retreive it otherwise
        let state: &mut IncludeStateInner = if !self.0.contains_key(fname) {
            let content = read_source(fname.clone(), env.options().parse_options())?;

            if env.options().show_progress() {
                Progress::progress().add_parse(progress::normalize(fname));
            }

            let builder = env
                .options()
                .clone()
                .context_builder()
                .set_current_filename(fname.clone());

            let listing = parse_z80_with_context_builder(content, builder)?;

            // Remove the progression
            if env.options().show_progress() {
                Progress::progress().remove_parse(progress::normalize(fname));
            }

            let include_state = IncludeStateInnerTryBuilder {
                listing,
                processed_tokens_builder: |listing: &LocatedListing| {
                    build_processed_tokens_list(listing.as_slice(), env)
                }
            }
            .try_build()?;

            self.0.try_insert(fname.clone(), include_state)
                .expect("BUG: fname should not already be in include map")
        }
        else {
            self.0.get_mut(fname)
                .expect("BUG: fname should exist in include map")
        };

        // handle the listing
        env.included_marks_add(fname.clone());

        Ok(state)
    }

    fn handle(
        &mut self,
        env: &mut Env,
        fname: &str,
        namespace: Option<&str>,
        once: bool
    ) -> Result<(), AssemblerError> {
        let fname = get_filename_to_read(fname, env.options().parse_options(), Some(env))?;

        let need_to_include = !once || !env.included_marks_includes(&fname);

        // Process the inclusion only if necessary
        if need_to_include {
            // most of the time, file has been loaded
            let state = self.retreive_listing(env, &fname)?;

            // handle module if necessary
            if let Some(namespace) = namespace {
                env.enter_namespace(namespace)?;
                // TODO handle the locating of error
                //.map_err(|e| e.locate(span.clone()))?;
            }

            // Visit the included listing
            env.enter_current_working_file(fname);
            let res = state.with_processed_tokens_mut(|tokens| {
                let tokens: &mut [ProcessedToken<'_, LocatedToken>] = &mut tokens[..];
                visit_processed_tokens::<LocatedToken>(tokens, env)
            });
            env.leave_current_working_file();
            res?;

            // Remove module if necessary
            if namespace.is_some() {
                env.leave_namespace()?;
                //.map_err(|e| e.locate(span.clone()))?;
            }
        }

        Ok(())
    }
}

#[self_referencing]
struct IncludeStateInner {
    listing: LocatedListing,
    #[borrows(listing)]
    #[covariant]
    processed_tokens: Vec<ProcessedToken<'this, LocatedToken>>
}

impl Clone for IncludeStateInner {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl PartialEq for IncludeStateInner {
    fn eq(&self, other: &Self) -> bool {
        self.with_listing(|l1| other.with_listing(|l2| l1.eq(l2)))
    }
}

impl Eq for IncludeStateInner {}

impl Debug for IncludeStateInner {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "IncludeState")
    }
}

#[self_referencing]
struct ExpandState {
    listing: LocatedListing,
    #[borrows(listing)]
    #[covariant]
    processed_tokens: Vec<ProcessedToken<'this, LocatedToken>>
}

impl PartialEq for ExpandState {
    fn eq(&self, other: &Self) -> bool {
        self.with_listing(|l1| other.with_listing(|l2| l1.eq(l2)))
    }
}

impl Eq for ExpandState {}

impl Clone for ExpandState {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl Debug for ExpandState {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "ExpandState")
    }
}

/// Store for each branch (if passed at some point) the test result and the listing
#[derive(Debug, Clone, PartialEq, Eq)]
struct IfState<'token, T: Visited + Debug + ListingElement + Sync> {
    // The token that contains the tests and listings
    token: &'token T,
    if_token_adr_to_used_decision: std::collections::HashMap<usize, bool>,
    if_token_adr_to_unused_decision: std::collections::HashMap<usize, bool>,
    // Processed listing build on demand
    tests_listing: HashMap<usize, Vec<ProcessedToken<'token, T>>>,
    // else listing build on demand
    else_listing: Option<Vec<ProcessedToken<'token, T>>>
}

impl<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan> IfState<'token, T>
where <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    fn new(token: &'token T) -> Self {
        Self {
            token,
            if_token_adr_to_used_decision: Default::default(),
            if_token_adr_to_unused_decision: Default::default(),
            tests_listing: Default::default(),
            else_listing: None
        }
    }

    /// Helper method to evaluate expression-based tests (IF/IFNOT)
    fn evaluate_expression_test(
        &self,
        test: &<T as ListingElement>::TestKind,
        env: &mut Env,
        flag_failure: &cpclib_tokens::ExprResult
    ) -> Result<Option<bool>, AssemblerError>
    where
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt
    {
        let exp = test.expr_unchecked();
        let value = env.resolve_expr_may_fail_in_first_pass_with_default(exp, flag_failure.clone())?;
        
        if &value == flag_failure {
            // Test cannot be evaluated, return None
            return Ok(None);
        }
        
        let result = value.bool()?;
        Ok(Some(if test.is_true_test() { result } else { !result }))
    }

    /// Helper method to evaluate label usage tests (IFUSED/IFNUSED)
    fn evaluate_label_usage_test(
        &mut self,
        test: &<T as ListingElement>::TestKind,
        token_adr: usize,
        env: &Env,
        request_additional_pass: &mut bool
    ) -> Result<bool, AssemblerError> {
        let label = test.label_unchecked();
        let is_used = env.symbols().is_used(label);
        let decision = if test.is_label_used_test() { is_used } else { !is_used };

        let map = if test.is_label_used_test() {
            &mut self.if_token_adr_to_used_decision
        } else {
            &mut self.if_token_adr_to_unused_decision
        };

        // Add an extra pass if the test result differs from previous
        if let Some(&previous) = map.get(&token_adr) {
            if previous != decision {
                *request_additional_pass = true;
            }
        }

        map.insert(token_adr, decision);
        Ok(decision)
    }

    /// Helper method to evaluate label existence tests (IFDEF/IFNDEF)
    fn evaluate_label_existence_test(
        &self,
        test: &<T as ListingElement>::TestKind,
        env: &Env
    ) -> Result<bool, AssemblerError> {
        let label = test.label_unchecked();
        let exists = env.symbols().symbol_exist_in_current_pass(label)?;
        Ok(if test.is_label_exists_test() { exists } else { !exists })
    }

    fn choose_listing_to_assemble(
        &mut self,
        env: &mut Env
    ) -> Result<Option<&mut [ProcessedToken<'token, T>]>, AssemblerError>
    where
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
    {
        let mut selected_idx = None;
        let mut request_additional_pass = false;
        use cpclib_tokens::ExprResult;
        let flag_failure: OnceCell<ExprResult> = OnceCell::new();
        let flag_failure =
            flag_failure.get_or_init(|| "__BASM_INNER_TEST_FAILURE__".to_owned().into());

        for idx in 0..self.token.if_nb_tests() {
            let (test, _) = self.token.if_test(idx);
            // Use safe pointer conversion for stable address
            let token_adr = ptr::from_ref(test) as usize;

            let test_passed = if test.is_true_test() || test.is_false_test() {
                // Expression-based tests (IF/IFNOT) — only evaluate once
                match self.evaluate_expression_test(test, env, flag_failure)? {
                    Some(result) => result,
                    None => return Ok(None)  // Unresolvable expression, abort
                }
            } else if test.is_label_used_test() || test.is_label_nused_test() {
                // Label usage tests (IFUSED/IFNUSED)
                self.evaluate_label_usage_test(test, token_adr, env, &mut request_additional_pass)?
            } else {
                // Label existence tests (IFDEF/IFNDEF)
                self.evaluate_label_existence_test(test, env)?
            };

            if test_passed {
                selected_idx = Some(idx);
                break;
            }
        }

        let selected_listing = match selected_idx {
            Some(selected_idx) => {
                // Build the listing if never done, using entry API to avoid double lookup
                use std::collections::hash_map::Entry;
                match self.tests_listing.entry(selected_idx) {
                    Entry::Occupied(e) => Some(e.into_mut()),
                    Entry::Vacant(e) => {
                        let listing = self.token.if_test(selected_idx).1;
                        let listing = build_processed_tokens_list(listing, env)?;
                        Some(e.insert(listing))
                    }
                }
            },
            None => {
                // Build else listing on-demand if it exists and hasn't been built
                if self.else_listing.is_none() {
                    self.else_listing = self.token.if_else()
                        .map(|listing| build_processed_tokens_list(listing, env))
                        .transpose()?;
                }
                self.else_listing.as_mut()
            }
        };

        // update env to request an additional pass
        if request_additional_pass {
            *env.request_additional_pass.write().unwrap() = true;
        }

        Ok(selected_listing.map(|l| l.as_mut_slice()))
    }
}

impl<T: Visited + Debug + ListingElement + Sync + ToSimpleToken> ToSimpleToken
    for ProcessedToken<'_, T>
where <T as ListingElement>::Expr: ExprEvaluationExt
{
    fn as_simple_token(&self) -> Cow<'_, Token> {
        self.token.as_simple_token()
    }
}

pub type AssemblerInfo = AssemblerError;

/// Helper: Extract and clone the span from a token if present
fn get_token_span<T: MayHaveSpan>(token: &T) -> Option<Z80Span> {
    token.possible_span().cloned()
}

/// Helper: Build SimpleListingState from a listing, span, and environment
fn build_simple_listing_state<'token, T: Visited + Debug + Sync + ListingElement + MayHaveSpan>(
    listing: &'token [T],
    span: Option<Z80Span>,
    env: &mut Env
) -> Result<SimpleListingState<'token, T>, AssemblerError>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    SimpleListingState::build(listing, span, env)
}

/// Helper: Relocate print commands from a given index with a specific span
fn relocate_print_commands(env: &mut Env, nb_prints: Vec<usize>, span: &Z80Span) {
    env.sna
        .pages_info
        .iter_mut()
        .zip(nb_prints.into_iter())
        .for_each(|(ti, count)| {
            ti.print_commands_mut()[count..]
                .iter_mut()
                .for_each(|cmd| cmd.relocate(span.clone()))
        });
}

/// Helper: Relocate error with span if available
fn relocate_error_with_span<T: MayHaveSpan>(error: AssemblerError, token: &T) -> AssemblerError {
    match token.possible_span() {
        Some(span) => AssemblerError::RelocatedError {
            error: error.into(),
            span: span.clone()
        },
        None => error
    }
}

/// Build a processed token based on the base token
pub fn build_processed_token<'token, T: Visited + Debug + Sync + ListingElement + MayHaveSpan>(
    token: &'token T,
    env: &mut Env
) -> Result<ProcessedToken<'token, T>, AssemblerError>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    let span = get_token_span(token);
    let state = if token.is_confined() {
        Some(ProcessedTokenState::Confined(
            build_simple_listing_state(
                token.confined_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_if() {
        let state = IfState::new(token);
        Some(ProcessedTokenState::If(state))
    }
    else if token.is_include() {
        // we cannot use the real method onf IncludeState because it modifies env and here wa cannot
        let fname = token.include_fname();
        let fname = env.build_fname(fname)?;
        let options = env.options().parse_options();
        match get_filename_to_read(fname, options, Some(env)) {
            Ok(fname) => {
                match read_source(fname.clone(), options) {
                    Ok(content) => {
                        let ctx_builder = options
                            .clone()
                            .context_builder()
                            .set_current_filename(fname.clone());

                        match parse_z80_with_context_builder(content, ctx_builder) {
                            Ok(listing) => {
                                // Filename has already been added
                                if token.include_is_standard_include()
                                    && env.options().show_progress()
                                {
                                    Progress::progress().remove_parse(progress::normalize(&fname));
                                }

                                let include_state = IncludeStateInnerTryBuilder {
                                    listing,
                                    processed_tokens_builder: |listing: &LocatedListing| {
                                        build_processed_tokens_list(listing, env)
                                    }
                                }
                                .try_build()?;

                                let mut map = BTreeMap::new();
                                map.insert(fname, include_state);

                                Some(ProcessedTokenState::Include(IncludeState(map)))
                            },
                            Err(_) => Some(ProcessedTokenState::Include(Default::default()))
                        }
                    },
                    Err(_) => Some(ProcessedTokenState::Include(Default::default()))
                }
            },
            Err(_) => Some(ProcessedTokenState::Include(Default::default())) /* we were unable to get the filename with the provided information */
        }
    }
    else if token.is_incbin() {
        Some(ProcessedTokenState::Incbin(Default::default()))
    }
    else if token.is_crunched_section() {
        Some(ProcessedTokenState::CrunchedSection {
            listing: build_simple_listing_state(
                token.crunched_section_listing(),
                span.clone(),
                env
            )?,
            previous_bytes: None,
            previous_compressed_bytes: None
        })
    }
    else if token.is_for() {
        Some(ProcessedTokenState::For(
            build_simple_listing_state(
                token.for_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_function_definition() {
        Some(ProcessedTokenState::FunctionDefinition(
            FunctionDefinitionState(None)
        ))
    }
    else if token.is_iterate() {
        Some(ProcessedTokenState::Iterate(
            build_simple_listing_state(
                token.iterate_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_module() {
        Some(ProcessedTokenState::Module(
            build_simple_listing_state(
                token.module_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_repeat() {
        Some(ProcessedTokenState::Repeat(
            build_simple_listing_state(
                token.repeat_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_repeat_token() {
        Some(ProcessedTokenState::RepeatToken(
            SingleTokenState {
                token: Box::new(build_processed_token(token.repeat_token(), env)?)
            },
            token.repeat_count()
        ))
    }
    else if token.is_assembler_control()
        && token
            .assembler_control_command()
            .is_restricted_assembling_environment()
    {
        assert!(
            token.assembler_control_get_max_passes().is_some(),
            "We currently only support a maximum number of passes, so it as to be provided ..."
        );

        let passes = match token.assembler_control_get_max_passes() {
            Some(passes) => Some(env.resolve_expr_must_never_fail(passes)?.int()? as u8),
            None => None
        };

        let tokens = token.assembler_control_get_listing();
        Some(ProcessedTokenState::RestrictedAssemblingEnvironment {
            listing: build_simple_listing_state(
                tokens,
                span.clone(),
                env
            )?,
            commands: Some(ControlOutputStore::with_passes(
                passes.expect("BUG: passes should be Some after assertion")
            ))
        })
    }
    else if token.is_repeat_until() {
        Some(ProcessedTokenState::RepeatUntil(
            build_simple_listing_state(
                token.repeat_until_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_rorg() {
        Some(ProcessedTokenState::Rorg(
            build_simple_listing_state(
                token.rorg_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_switch() {
        // todo setup properly the spans
        Some(ProcessedTokenState::Switch(SwitchState {
            cases: token
                .switch_cases()
                .map(|(_v, l, _b)| {
                    build_simple_listing_state(l, span.clone(), env)
                })
                .collect::<Result<Vec<_>, _>>()?,

            default: token
                .switch_default()
                .map(|l| build_simple_listing_state(l, span.clone(), env))
                .transpose()?
        }))
    }
    else if token.is_warning() {
        Some(ProcessedTokenState::Warning(Box::new(
            build_processed_token(token.warning_token(), env)?
        )))
    }
    else if token.is_while() {
        Some(ProcessedTokenState::While(
            build_simple_listing_state(
                token.while_listing(),
                span.clone(),
                env
            )?
        ))
    }
    else if token.is_call_macro_or_build_struct() {
        // one day, we may whish to maintain a state
        None
    }
    else {
        None
    };

    Ok(ProcessedToken { token, state })
}

pub fn build_processed_tokens_list<
    'token,
    T: Visited + Debug + Sync + ListingElement + MayHaveSpan
>(
    tokens: &'token [T],
    env: &mut Env
) -> Result<Vec<ProcessedToken<'token, T>>, AssemblerError>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    let show_progress = env.options().parse_options().show_progress;
    if show_progress {
        // // temporarily deactivate parallel processing while i have not found a way to compile it
        // #[cfg(not(target_arch = "wasm32"))]
        // let iter = tokens.par_iter();
        // #[cfg(target_arch = "wasm32")]
        let iter = tokens.iter();

        // get filename of files that will be read in parallel
        let mut include_fnames: Vec<String> = Vec::with_capacity(tokens.len());
        include_fnames.extend(
            iter.filter(|t| t.include_is_standard_include()).flat_map(|t| {
                let fname = t.include_fname();
                let fname = env.build_fname(fname)?;
                get_filename_to_read(fname, env.options().parse_options(), Some(env))
            })
            .map(|path| progress::normalize(&path).to_string())
        );

        // inform the progress bar in one go
        Progress::progress().add_parses(include_fnames.iter().map(String::as_str));
    }

    // the files will be read here while token are built
    // this is really important to keep this place parallelized
    #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
    let iter = tokens.par_iter();
    #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
    let iter = tokens.iter();
    iter.map(|t| build_processed_token(t, env))
        .collect::<Result<Vec<_>, _>>()
}

/// Visit all the tokens until an error occurs
pub fn visit_processed_tokens<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan>(
    tokens: &mut [ProcessedToken<'token, T>],
    env: &mut Env
) -> Result<(), AssemblerError>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    // Ignore if function has already returned (mainly a workaround for switch case)
    if env.return_value.is_some() {
        return Ok(());
    }

    let options = env.options();

    if options.show_progress() {
        // setup the amount of tokens that will be processed
        Progress::progress().add_expected_to_pass(tokens.len() as _);
        for chunk in &tokens.iter_mut().chunks(64) {
            let mut visited = 0;
            for token in chunk {
                token.visited(env)?;
                visited += 1;
            }

            Progress::progress().add_visited_to_pass(visited);
        }
    }
    else {
        // normal iteration
        for token in tokens.iter_mut() {
            token.visited(env)?;
        }
    }

    env.cleanup_warnings();

    Ok(())
}

impl<T: Visited + Debug + ListingElement + Sync + MayHaveSpan> MayHaveSpan for ProcessedToken<'_, T>
where <T as ListingElement>::Expr: ExprEvaluationExt
{
    fn possible_span(&self) -> Option<&Z80Span> {
        self.token.possible_span()
    }

    fn span(&self) -> &Z80Span {
        self.token.span()
    }

    fn has_span(&self) -> bool {
        self.token.has_span()
    }
}

impl<T: Visited + Debug + ListingElement + Sync + MayHaveSpan> ProcessedToken<'_, T>
where <T as ListingElement>::Expr: ExprEvaluationExt
{
    /// Generate the tokens needed for the macro or the struct
    #[inline]
    pub fn update_macro_or_struct_state(&mut self, env: &mut Env) -> Result<(), AssemblerError>
    where <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt {
        let caller = self.token;
        let name = caller.macro_call_name();
        let parameters = caller.macro_call_arguments();
        let mut padded_struct_params: Option<Vec<T::MacroParam>> = None;

        let listing = {
            // Retreive the macro or structure definition
            let r#macro = env
                .symbols()
                .macro_value(name)?
                .map(|m| r#macro::MacroWithArgs::build(m, parameters))
                .transpose()?;
            let r#struct = if r#macro.is_none() {
                env.symbols()
                    .struct_value(name)?
                    .map(|s| {
                        let needed = s.nb_args();
                        let args_slice = if parameters.len() < needed {
                            let mut buf = parameters.to_vec();
                            buf.resize(needed, T::MacroParam::empty());
                            padded_struct_params = Some(buf);
                            padded_struct_params
                                .as_deref()
                                .expect("padded params must be present when needed")
                        }
                        else {
                            parameters
                        };

                        r#macro::StructWithArgs::build(s, args_slice)
                    })
                    .transpose()?
            }
            else {
                None
            };

            // Leave now if it corresponds to no macro or struct
            if r#macro.is_none() && r#struct.is_none() {
                let e = AssemblerError::UnknownMacro {
                    symbol: name.into(),
                    closest: env
                        .symbols()
                        .closest_symbol(name, SymbolFor::Macro)?
                        .map(|s| s.into())
                };
                return Err(relocate_error_with_span(e, self.token));
            }

            // get the generated code
            // TODO handle some errors there
            let (source, code, _flavor) = if let Some(r#macro) = &r#macro {
                let source = r#macro.source();
                let flavor = r#macro.flavor();
                let code = r#macro.expand(env)?;
                (source, code, flavor)
            }
            else {
                let r#struct = r#struct
                    .as_ref()
                    .expect("BUG: r#struct should be Some when r#macro is None");
                (
                    r#struct.source(),
                    r#struct.expand(env)?,
                    AssemblerFlavor::Basm
                )
            };

            // Tokenize with the same parsing  parameters and context when possible

            match self.token.possible_span() {
                Some(span) => {
                    use crate::ParserContextBuilder;
                    let ctx_builder = ParserContextBuilder::default() // nothing is specified
                        //                    from(span.state.clone())
                        .set_state(span.state.state)
                        .set_options(span.state.options.clone())
                        .set_context_name(format!(
                            "{}:{}:{} > {} {}:",
                            source.map(|s| s.fname()).unwrap_or_else(|| "???"),
                            source.map(|s| s.line()).unwrap_or(0),
                            source.map(|s| s.column()).unwrap_or(0),
                            if r#macro.is_some() { "MACRO" } else { "STRUCT" },
                            name,
                        ));
                    parse_z80_with_context_builder(code, ctx_builder)?
                },
                _ => {
                    use crate::parse_z80_str;
                    parse_z80_str(&code)?
                }
            }
        };

        let expand_state = ExpandStateTryBuilder {
            listing,
            processed_tokens_builder: |listing: &LocatedListing| {
                build_processed_tokens_list(listing, env)
            }
        }
        .try_build()?;

        self.state = Some(ProcessedTokenState::MacroCallOrBuildStruct(expand_state));

        Ok(())
    }
}

impl<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan> ProcessedToken<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    /// Due to the state management, the signature requires mutability
    pub fn visited(&mut self, env: &mut Env) -> Result<(), AssemblerError> {
        let possible_span = self.possible_span().cloned();

        let mut really_does_the_job = move |possible_span: Option<&Z80Span>| {
            let deferred = self.token.defer_listing_output();
            if !deferred {
                // dbg!(&self.token, deferred);
                // SAFETY: This transmute is only safe when T is LocatedToken.
                // This is guaranteed by the type system at the call site.
                let outer_token = unsafe {
                    std::mem::transmute::<&T, &LocatedToken>(self.token)
                };

                env.handle_output_trigger(outer_token);
            }

            // Generate the code of a macro/struct
            if self.token.is_call_macro_or_build_struct() {
                self.update_macro_or_struct_state(env)?;
            }

            // Behavior based on the token
            if self.token.is_macro_definition() {
                // TODO really implement logic here
                let name = self.token.macro_definition_name();
                let arguments = self.token.macro_definition_arguments();
                let code = self.token.macro_definition_code();
                env.visit_macro_definition(
                    name,
                    &arguments,
                    code,
                    self.possible_span(),
                    self.token.macro_flavor()
                )
            }
            // Behavior based on the state (for ease of writting)
            else {
                // Handle the tokens depending on their specific state
                match &mut self.state {
                    Some(ProcessedTokenState::RestrictedAssemblingEnvironment {
                        listing,
                        commands
                    }) => {
                        let mut new_commands = commands.take()
                            .expect("BUG: commands should be Some for RestrictedAssemblingEnvironment");

                        if !new_commands.has_remaining_passes() {
                            new_commands.execute(env)?;
                        }
                        else {
                            // TODO move that code directly inside ControlOutputStore
                            new_commands.new_pass();
                            env.assembling_control_current_output_commands
                                .push(new_commands);
                            visit_processed_tokens(&mut listing.processed_tokens, env)?;
                            new_commands = env
                                .assembling_control_current_output_commands
                                .pop()
                                .expect("BUG: output commands stack should not be empty");
                        }
                        commands.replace(new_commands);
                        Ok(())
                    },

                    Some(ProcessedTokenState::Confined(SimpleListingState {
                        processed_tokens,
                        span
                    })) => env.visit_confined(processed_tokens, span.as_ref()),
                    Some(ProcessedTokenState::CrunchedSection {
                        listing:
                            SimpleListingState {
                                processed_tokens,
                                span
                            },
                        previous_bytes,
                        previous_compressed_bytes
                    }) => {
                        env.visit_crunched_section(
                            self.token.crunched_section_kind(),
                            processed_tokens,
                            previous_bytes,
                            previous_compressed_bytes,
                            span.as_ref()
                        )
                    },

                    Some(ProcessedTokenState::For(SimpleListingState {
                        processed_tokens,
                        span
                    })) => {
                        env.visit_for(
                            self.token.for_label(),
                            self.token.for_start(),
                            self.token.for_stop(),
                            self.token.for_step(),
                            processed_tokens,
                            span.as_ref()
                        )
                    },

                    Some(ProcessedTokenState::RepeatToken(state, count)) => {
                        env.visit_repeat_token(&mut state.token, count)
                    },

                    Some(ProcessedTokenState::FunctionDefinition(FunctionDefinitionState(
                        Some(_fun)
                    ))) => {
                        // TODO check if the funtion has already been defined during this pass
                        Ok(())
                    },
                    Some(ProcessedTokenState::FunctionDefinition(FunctionDefinitionState(
                        option
                    ))) => {
                        let name = self.token.function_definition_name();
                        if !env.functions.contains_key(name) {
                            let inner = self.token.function_definition_inner();
                            let params = self.token.function_definition_params();

                            let inner = build_processed_tokens_list(inner, env)?;
                            let f =
                                Arc::new(unsafe { FunctionBuilder::new(&name, &params, inner) }?);
                            option.replace(f.clone());

                            env.functions.insert(name.to_owned(), f);
                        }
                        else {
                            // TODO raise an error ?
                        }
                        Ok(())
                    },

                    Some(ProcessedTokenState::Incbin(IncbinState { contents })) => {
                        if cfg!(target_arch = "wasm32") {
                            return Err(AssemblerError::AssemblingError { msg:
                                "INCBIN-like directives are not allowed in a web-based assembling.".into()
                            });
                        }

                        // Handle file loading
                        let fname = self.token.incbin_fname();
                        let fname = env.build_fname(fname)?;
                        let to_print_fname = &fname;
                        let fname =
                            get_filename_to_read(&fname, env.options().parse_options(), Some(env))?;

                        // Add a warning when incbin is used on a possible assembly file
                        if let Some(extension) = fname.extension() {
                            match extension.to_ascii_uppercase().as_str() {
                                "ASM" | "Z80" => {
                                    let warning = format!(
                                        "{} seems to be a source code and not a binary file.",
                                        &fname
                                    );
                                    env.add_warning(dbg!(AssemblerWarning::AssemblingError {
                                        msg: warning
                                    }));
                                },
                                _ => {}
                            }
                        }

                        // get the data for the given file
                        let data = if !contents.contains_key(&fname) {
                            // need to load the file

                            let (data, header) =
                                load_file(fname.as_path(), env.options().parse_options())?;

                            if let Some(header) = header {
                                let ams_fname = header
                                    .amsdos_filename()
                                    .map(|ams_fname| ams_fname.filename_with_user())
                                    .unwrap_or_else(|_| "<WRONG FILENAME>".to_owned());
                                let txt = match header.file_type() {
                                    Ok(AmsdosFileType::Binary) => {
                                        format! {"{to_print_fname}|{ams_fname} BINARY  L:0x{:x} X:0x{:x}", header.loading_address(), header.execution_address()}
                                    },
                                    Ok(AmsdosFileType::Protected) => {
                                        format! {"{to_print_fname}|{ams_fname} PROTECTED L:0x{:x} X:0x{:x}", header.loading_address(), header.execution_address()}
                                    },
                                    Ok(AmsdosFileType::Basic) => format!("{ams_fname} BASIC"),
                                    Err(_) => format!("{ams_fname} <WRONG FILETYPE>")
                                };

                                let warning = AssemblerWarning::AssemblingError {
                                    msg: format!("Header has been removed for {txt}")
                                };
                                let warning = if let Some(span) = possible_span {
                                    warning.locate_warning(span.clone())
                                }
                                else {
                                    warning
                                };

                                env.add_warning(warning)
                            }

                            contents.try_insert(fname.clone(), data.into())
                                .expect("BUG: fname should not already be in incbin contents")
                        }
                        else {
                            contents.get(&fname)
                                .expect("BUG: fname should exist in incbin contents")
                        };

                        let mut data = data.as_slice();

                        // Extract the appropriate content to the file
                        let offset = self.token.incbin_offset();
                        let length = self.token.incbin_length();
                        let transformation = self.token.incbin_transformation();

                        if let Some(offset) = offset {
                            let offset = env.resolve_expr_must_never_fail(offset)?.int()? as usize;
                            if offset > data.len() {
                                return Err(AssemblerError::AssemblingError {
                                    msg: format!(
                                        "Unable to skip {} bytes in  {}. Only {} bytes  are available",
                                        offset,
                                        self.token.incbin_fname(),
                                        data.len()
                                    )
                                });
                            }
                            data = &data[offset..];
                        }

                        if let Some(length) = length {
                            let length = env.resolve_expr_must_never_fail(length)?.int()? as usize;
                            if data.len() < length {
                                return Err(AssemblerError::AssemblingError {
                                    msg: format!(
                                        "Unable to read {} bytes in {}. Only {} bytes are available",
                                        length,
                                        self.token.incbin_fname(),
                                        data.len()
                                    )
                                });
                            }
                            data = &data[..length];
                        }

                        let data = match transformation {
                            BinaryTransformation::None => Cow::Borrowed(data),

                            other => {
                                if data.is_empty() {
                                    return Err(AssemblerError::EmptyBinaryFile(
                                        self.token.incbin_fname().to_string()
                                    ));
                                }

                                let crunch_type = other.crunch_type()
                                    .expect("BUG: crunch_type should return Some for non-None transformation");
                                let result = crunch_type.compress(data)?;
                                Cow::Owned(result.to_vec()) // TODO store the delta somewhere to allow a reuse
                            }
                        };

                        env.visit_incbin(data.borrow())
                    },

                    Some(ProcessedTokenState::Include(state)) => {
                        let fname = env.build_fname(self.token.include_fname())?;

                        state.handle(
                            env,
                            &fname,
                            self.token.include_namespace(),
                            self.token.include_once()
                        )
                    },

                    Some(ProcessedTokenState::If(if_state)) => {
                        let listing = if_state.choose_listing_to_assemble(env)?;

                        if let Some(listing) = listing {
                            visit_processed_tokens(listing, env)?;
                        }

                        Ok(())
                    },

                    Some(ProcessedTokenState::Iterate(SimpleListingState {
                        processed_tokens,
                        span
                    })) => {
                        env.visit_iterate(
                            self.token.iterate_counter_name(),
                            self.token.iterate_values(),
                            processed_tokens,
                            span.as_ref()
                        )
                    },

                    Some(ProcessedTokenState::MacroCallOrBuildStruct(state)) => {
                        let name = self.token.macro_call_name();

                        // Increment macro seed for this macro invocation
                        env.inc_macro_seed();
                        let macro_seed = env.macro_seed();
                        env.symbols_mut().push_seed(macro_seed);

                        // save the number of prints to patch the ones added by the macro
                        // to properly locate them
                        let nb_prints = env
                            .sna
                            .pages_info
                            .iter()
                            .map(|ti| ti.print_commands().len())
                            .collect();

                        // Process tokens - if error occurs, we still need to pop_seed (see cleanup below)
                        let process_result = state
                            .with_processed_tokens_mut(|listing| {
                                let tokens: &mut [ProcessedToken<'_, LocatedToken>] =
                                    &mut listing[..];
                                visit_processed_tokens::<LocatedToken>(tokens, env)
                            });

                        // Always pop the seed, even if an error occurred (RAII principle)
                        env.symbols_mut().pop_seed();

                        let result = process_result.map_err(|e| {
                            let location = env
                                .symbols()
                                .any_value(name)
                                .ok()
                                .flatten()
                                .expect("BUG: macro name should exist in symbol table")
                                .location()
                                .cloned();

                            let e = AssemblerError::MacroError {
                                name: name.into(),
                                root: Box::new(e),
                                location
                            };
                            let caller_span = self.possible_span();
                            relocate_error_with_span(e, self.token)
                        })?;

                        let caller_span = self.possible_span();
                        if let Some(span) = caller_span {
                            relocate_print_commands(env, nb_prints, span);
                        }

                        Ok(())
                    },

                    Some(ProcessedTokenState::Module(SimpleListingState {
                        processed_tokens,
                        ..
                    })) => {
                        env.enter_namespace(self.token.module_name())?;
                        visit_processed_tokens(processed_tokens, env)?;
                        env.leave_namespace()?;

                        Ok(())
                    },

                    Some(ProcessedTokenState::Repeat(SimpleListingState {
                        processed_tokens,
                        ..
                    })) => {
                        env.visit_repeat(
                            self.token.repeat_count(),
                            processed_tokens,
                            self.token.repeat_counter_name(),
                            self.token.repeat_counter_start(),
                            self.token.repeat_counter_step(),
                            self.token.possible_span()
                        )
                    },

                    Some(ProcessedTokenState::RepeatUntil(SimpleListingState {
                        processed_tokens,
                        ..
                    })) => {
                        env.visit_repeat_until(
                            self.token.repeat_until_condition(),
                            processed_tokens,
                            self.token.possible_span()
                        )
                    },

                    Some(ProcessedTokenState::Rorg(SimpleListingState {
                        processed_tokens,
                        span
                    })) => env.visit_rorg(self.token.rorg_expr(), processed_tokens, span.as_ref()),

                    Some(ProcessedTokenState::Switch(state)) => {
                        let value = env.resolve_expr_must_never_fail(self.token.switch_expr())?;
                        let mut met = false;
                        let mut broken = false;
                        for (case, listing, r#break) in state
                            .cases
                            .iter_mut()
                            .zip(self.token.switch_cases())
                            .map(|(pt, t)| (t.0, pt.tokens_mut(), t.2))
                        {
                            // check if case must be executed
                            let case = env.resolve_expr_must_never_fail(case)?;
                            met |= case == value;

                            // inject code if needed and leave if break is present
                            if met {
                                visit_processed_tokens(listing, env)?;
                                if r#break {
                                    broken = true;
                                    break;
                                }
                            }
                        }

                        // execute default if any
                        if (!met || !broken)
                            && let Some(default) = &mut state.default
                        {
                            visit_processed_tokens(&mut default.processed_tokens, env)?;
                        }

                        Ok(())
                    },

                    Some(ProcessedTokenState::Warning(box token)) => {
                        let warning = AssemblerError::RelocatedWarning {
                            warning: Box::new(AssemblerError::AssemblingError {
                                msg: self.token.warning_message().to_owned()
                            }),
                            span: self.token.possible_span()
                                .expect("BUG: warning token should have a span")
                                .clone()
                        };
                        let warning = AssemblerError::AlreadyRenderedError(warning.to_string());

                        env.add_warning(warning);
                        token.visited(env)
                    },
                    Some(ProcessedTokenState::While(SimpleListingState {
                        processed_tokens,
                        ..
                    })) => {
                        env.visit_while(
                            self.token.while_expr(),
                            processed_tokens,
                            self.token.possible_span()
                        )
                    },

                    // no state implies a standard visit
                    None => self.token.visited(env)
                }
            }?;

            if !self.token.is_buildcpr() {
                // we lack of some datastructures
                env.update_dollar();
            }

            if deferred {
                // SAFETY: This transmute is only safe when T is LocatedToken.
                // This is guaranteed by the type system at the call site.
                let outer_token = unsafe {
                    std::mem::transmute::<&T, &LocatedToken>(self.token)
                };

                env.handle_output_trigger(outer_token);
            }
            Ok(())
        };

        really_does_the_job(possible_span.as_ref()).map_err(|e| {
            let e = match possible_span {
                Some(span) => e.locate(span.clone()),
                None => e
            };
            AssemblerError::AlreadyRenderedError(e.to_string())
        })
    }
}

// let fname = span
// .context()
// .get_path_for(fname)
// .unwrap_or("will_fail".into());
// if (!*once) || (!env.has_included(&fname)) {
// env.mark_included(fname);
//
// if cell.read().unwrap().is_some() {
// if let Some(namespace) = namespace {
// env.enter_namespace(namespace)
// .map_err(|e| e.locate(span.clone()))?;
// }
//
// env.visit_listing(cell.read().unwrap().as_ref().unwrap())?;
//
// if namespace.is_some() {
// env.leave_namespace().map_err(|e| e.locate(span.clone()))?;
// }
// Ok(())
// }
// else {
// outer_token
// .read_referenced_file(&outer_token.context().1)
// .and_then(|_| visit_located_token(outer_token, env))
// .map_err(|e| e.locate(span.clone()))
// }
// .map_err(|err| {
// AssemblerError::IncludedFileError {
// span: span.clone(),
// error: Box::new(err)
// }
// })
// }
// else {
// Ok(()) // we include nothing
// }

#[cfg(test)]
mod test_super {
    use super::*;

    #[test]
    fn test_located_include() {
        use crate::parse_z80;

        let src = "include \"toto\"";

        let tokens = parse_z80(src).unwrap();
        let token = &tokens[0];
        let mut env = Env::default();

        let processed = build_processed_token(token, &mut env);
        assert!(matches!(
            processed.unwrap().state,
            Some(ProcessedTokenState::Include(..))
        ));
    }
}

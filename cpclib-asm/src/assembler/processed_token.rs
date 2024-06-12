use std::borrow::{Borrow, Cow};
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::prelude::*;
use cpclib_disc::amsdos::AmsdosHeader;
use cpclib_tokens::symbols::{SymbolFor, SymbolsTableTrait};
use cpclib_tokens::{
    AssemblerControlCommand, AssemblerFlavor, BinaryTransformation, ExprElement, ListingElement,
    MacroParamElement, TestKindElement, ToSimpleToken, Token
};
use either::Either;
use ouroboros::*;

use super::control::ControlOutputStore;
use super::file::{get_filename, load_binary, read_source};
use super::function::{Function, FunctionBuilder};
use super::r#macro::Expandable;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::instructions::Cruncher;
use crate::preamble::{LocatedListing, MayHaveSpan, Z80Span};
use crate::progress::{self, Progress};
use crate::{parse_z80_with_context_builder, r#macro, AssemblerError, Env, LocatedToken, Visited};

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
        previous_compressed_bytes: Option<Vec<u8>>
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
    Repeat(SimpleListingState<'token, T>),
    RepeatUntil(SimpleListingState<'token, T>),
    While(SimpleListingState<'token, T>),
    Rorg(SimpleListingState<'token, T>),
    Switch(SwitchState<'token, T>),
    Warning(Box<ProcessedToken<'token, T>>)
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
struct IncbinState {
    contents: BTreeMap<PathBuf, Vec<u8>>
}

#[derive(PartialEq, Eq, Clone)]
struct SimpleListingState<'token, T: Visited + ListingElement + Debug + Sync> {
    processed_tokens: Vec<ProcessedToken<'token, T>>,
    span: Option<Z80Span>
}

impl<'token, T: Visited + ListingElement + Debug + Sync> Debug for SimpleListingState<'token, T> {
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
        env: &Env
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

#[derive(PartialEq, Eq, Clone, Debug)]
struct IncludeState(BTreeMap<PathBuf, IncludeStateInner>);

impl IncludeState {
    /// By constructon fname exists and is correct
    /// 
    fn retreive_listing(
        &mut self,
        env: &mut Env,
        fname: &PathBuf
    ) -> Result<&mut IncludeStateInner, AssemblerError> {
        if cfg!(target_arch = "wasm32") {
            return Err(AssemblerError::AssemblingError {
                msg: "INCLUDE-like directives are not allowed in a web-based assembling."
                    .to_owned()
            });
        }

        let options = env.options();

        // Build the state if needed / retreive it otherwise
        let state: &mut IncludeStateInner = if !self.0.contains_key(fname) {
            let content = read_source(fname.clone(), options.parse_options())?;

            if options.show_progress() {
                Progress::progress().add_parse(progress::normalize(fname));
            }

            let builder = options
                .clone()
                .context_builder()
                .set_current_filename(fname.clone());

            let listing = parse_z80_with_context_builder(content, builder)?;

            // Remove the progression
            if options.show_progress() {
                Progress::progress().remove_parse(progress::normalize(fname));
            }

            let include_state = IncludeStateInnerTryBuilder {
                listing,
                processed_tokens_builder: |listing: &LocatedListing| {
                    build_processed_tokens_list(listing.as_slice(), env)
                }
            }
            .try_build()?;

            self.0.try_insert(fname.clone(), include_state).unwrap()
        }
        else {
            self.0.get_mut(fname).unwrap()
        };



        // handle the listing
        env.mark_included(fname.clone());

        Ok(state)
    }

    fn handle(
        &mut self,
        env: &mut Env,
        fname: &str,
        namespace: Option<&str>,
        once: bool
    ) -> Result<(), AssemblerError> {
        let fname = get_filename(fname, &env.options().parse_options(), Some(env))?;

        // Process the inclusion only if necessary
        if (!once) || (!env.has_included(&fname)) {
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
                visit_processed_tokens::<'_, LocatedToken>(tokens, env)
            });
            env.leave_current_working_file();
            res?;

            // Remove module if necessary
            if namespace.is_some() {
                env.leave_namespace()?;
                //.map_err(|e| e.locate(span.clone()))?;
            }

            Ok(())
        }
        else {
            Ok(())
        }
    }
}

impl Default for IncludeState {
    fn default() -> Self {
        IncludeState(BTreeMap::default())
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

    fn choose_listing_to_assemble(
        &mut self,
        env: &Env
    ) -> Result<Option<&mut [ProcessedToken<'token, T>]>, AssemblerError>
    where
        <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr:
            ExprEvaluationExt,
        <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
    {
        let mut selected_idx = None;
        let mut request_additional_pass = false;
        use cpclib_tokens::ExprResult;
        let FLAG_FAILURE: ExprResult = "__BASM_INNER_TEST_FAILURE__".to_owned().into();

        for idx in 0..self.token.if_nb_tests() {
            let (test, _) = self.token.if_test(idx);
            let token_adr = test as *const _ as usize;

            // Expression must be true
            if test.is_true_test() {
                let exp = test.expr_unchecked();
                // Expression must be true
                let value = env
                    .resolve_expr_may_fail_in_first_pass_with_default(exp, FLAG_FAILURE.clone())?;
                if value == FLAG_FAILURE {
                    // no code is executed if the test cannot be done
                    return Ok(None);
                }
                if value.bool()? {
                    selected_idx = Some(idx);
                    break;
                }
            }
            // Expression must be false
            else if test.is_false_test() {
                let exp = test.expr_unchecked();
                let value = env
                    .resolve_expr_may_fail_in_first_pass_with_default(exp, FLAG_FAILURE.clone())?;
                if value == FLAG_FAILURE {
                    // no code is executed if the test cannot be done
                    return Ok(None);
                }
                if !value.bool()? {
                    selected_idx = Some(idx);
                    break;
                }
            }
            else if test.is_label_used_test() {
                let label = test.label_unchecked();
                let decision = env.symbols().is_used(label);

                // Add an extra pass if the test differ
                if let Some(res) = self.if_token_adr_to_used_decision.get(&token_adr) {
                    if *res != decision {
                        request_additional_pass = true;
                    }
                }

                // replace the previously stored value
                self.if_token_adr_to_used_decision
                    .insert(token_adr.clone(), decision);

                if decision {
                    selected_idx = Some(idx);
                    break;
                }
            }
            else if test.is_label_nused_test() {
                let label = test.label_unchecked();
                let decision = !env.symbols().is_used(label);

                // Add an extra pass if the test differ
                if let Some(res) = self.if_token_adr_to_unused_decision.get(&token_adr) {
                    if *res != decision {
                        request_additional_pass = true;
                    }
                }

                // replace the previously stored value
                self.if_token_adr_to_unused_decision
                    .insert(token_adr.clone(), decision);

                if decision {
                    selected_idx = Some(idx);
                    break;
                }
            }
            // Label must exist at this specific moment
            else if test.is_label_exists_test() {
                let label = test.label_unchecked();
                if env.symbols().contains_symbol(label)? {
                    selected_idx = Some(idx);
                    break;
                }
            }
            // Label must not exist at this specific moment
            else {
                let label = test.label_unchecked();
                if !env.symbols().contains_symbol(label)? {
                    selected_idx = Some(idx);
                    break;
                }
            }
        }

        let selected_listing = match selected_idx {
            Some(selected_idx) => {
                // build the listing if never done
                if self.tests_listing.get(&selected_idx).is_none() {
                    let listing = self.token.if_test(selected_idx).1;
                    let listing = build_processed_tokens_list(listing, env)?;
                    self.tests_listing.insert(selected_idx, listing);
                }
                self.tests_listing.get_mut(&selected_idx)
            },
            None => {
                // build else listing if needed
                if self.else_listing.is_none() && self.token.if_else().is_some() {
                    let listing = self.token.if_else();
                    self.else_listing = listing
                        .map(|listing| build_processed_tokens_list(listing, env))
                        .transpose()?;
                }
                self.else_listing.as_mut()
            }
        };

        // update env to request an additional pass
        let request_additional_pass =
            *env.request_additional_pass.read().unwrap().deref() | request_additional_pass;
        *env.request_additional_pass.write().unwrap() = request_additional_pass;

        Ok(selected_listing.map(|l| l.as_mut_slice()))
    }
}

impl<'token, T: Visited + Debug + ListingElement + Sync + ToSimpleToken> ToSimpleToken
    for ProcessedToken<'token, T>
where <T as ListingElement>::Expr: ExprEvaluationExt
{
    fn as_simple_token(&self) -> Cow<Token> {
        self.token.as_simple_token()
    }
}

pub type AssemblerInfo = AssemblerError;

/// Build a processed token based on the base token
pub fn build_processed_token<'token, T: Visited + Debug + Sync + ListingElement + MayHaveSpan>(
    token: &'token T,
    env: &Env
) -> Result<ProcessedToken<'token, T>, AssemblerError>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    let state = if token.is_confined() {
        Some(ProcessedTokenState::Confined(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.confined_listing(), env)?,
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_if() {
        let state = IfState::new(token);
        Some(ProcessedTokenState::If(state))
    }
    else if token.is_include() {
        // we cannot use the real method onf IncludeState because it modifies env and here wa cannot
        let fname = token.include_fname();
        let options = env.options().parse_options();
        match get_filename(fname, options, Some(env)) {
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
                                if token.include_is_standard_include() && options.show_progress {
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
            listing: SimpleListingState {
                processed_tokens: build_processed_tokens_list(
                    token.crunched_section_listing(),
                    env
                )?,
                span: token.possible_span().cloned()
            },
            previous_bytes: None,
            previous_compressed_bytes: None
        })
    }
    else if token.is_for() {
        Some(ProcessedTokenState::For(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.for_listing(), env)?,
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_function_definition() {
        Some(ProcessedTokenState::FunctionDefinition(
            FunctionDefinitionState(None)
        ))
    }
    else if token.is_iterate() {
        Some(ProcessedTokenState::Iterate(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.iterate_listing(), env)?,
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_module() {
        Some(ProcessedTokenState::Module(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.module_listing(), env)?,
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_repeat() {
        Some(ProcessedTokenState::Repeat(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.repeat_listing(), env)?,
            span: token.possible_span().cloned()
        }))
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
            listing: SimpleListingState {
                processed_tokens: build_processed_tokens_list(tokens, env)?,
                span: token.possible_span().cloned()
            },
            commands: Some(ControlOutputStore::with_passes(passes.unwrap()))
        })
    }
    else if token.is_repeat_until() {
        Some(ProcessedTokenState::RepeatUntil(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.repeat_until_listing(), env)?,
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_rorg() {
        Some(ProcessedTokenState::Rorg(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.rorg_listing(), env)?,
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_switch() {
        // todo setup properly the spans
        Some(ProcessedTokenState::Switch(SwitchState {
            cases: token
                .switch_cases()
                .map(|(_v, l, _b)| {
                    SimpleListingState::build(l, token.possible_span().cloned(), env)
                })
                .collect::<Result<Vec<_>, _>>()?,

            default: token
                .switch_default()
                .map(|l| SimpleListingState::build(l, token.possible_span().cloned(), env))
                .transpose()?
        }))
    }
    else if token.is_warning() {
        Some(ProcessedTokenState::Warning(Box::new(
            build_processed_token(token.warning_token(), env)?
        )))
    }
    else if token.is_while() {
        Some(ProcessedTokenState::While(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.while_listing(), env)?,
            span: token.possible_span().cloned()
        }))
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
    env: &Env
) -> Result<Vec<ProcessedToken<'token, T>>, AssemblerError>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    let options = env.options().parse_options();
    if options.show_progress {
        // // temporarily deactivate parallel processing while i have not found a way to compile it
        // #[cfg(not(target_arch = "wasm32"))]
        // let iter = tokens.par_iter();
        // #[cfg(target_arch = "wasm32")]
        let iter = tokens.iter();

        // get filename of files that will be read in parallel
        let include_fnames = iter
            .filter(|t| t.include_is_standard_include())
            .map(|t| get_filename(t.include_fname(), options, Some(env)))
            .filter(|f| f.is_ok())
            .map(|f| f.unwrap())
            .collect::<Vec<_>>();
        let include_fnames = include_fnames.iter().map(|t| progress::normalize(&t));

        // inform the progress bar
        // add all fnames in one time
        Progress::progress().add_parses(include_fnames);
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

    Ok(())
}

impl<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan> MayHaveSpan
    for ProcessedToken<'token, T>
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

impl<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan> ProcessedToken<'token, T>
where <T as ListingElement>::Expr: ExprEvaluationExt
{
    /// Generate the tokens needed for the macro or the struct
    #[inline]
    pub fn update_macro_or_struct_state(&mut self, env: &Env) -> Result<(), AssemblerError>
    where <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt {
        let caller = self.token;
        let name = caller.macro_call_name();
        let parameters = caller.macro_call_arguments();

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
                    .map(|s| r#macro::StructWithArgs::build(s, parameters))
                    .transpose()?
            }
            else {
                None
            };

            // Leave now if it corresponds to no macro or struct
            if r#macro.is_none() && r#struct.is_none() {
                let e = AssemblerError::UnknownMacro {
                    symbol: name.into(),
                    closest: env.symbols().closest_symbol(name, SymbolFor::Macro)?
                };
                return match self.possible_span() {
                    Some(span) => {
                        Err(AssemblerError::RelocatedError {
                            error: e.into(),
                            span: span.clone()
                        })
                    },
                    None => Err(e)
                };
            }

            // get the generated code
            // TODO handle some errors there
            let (source, code, flavor) = if let Some(ref r#macro) = r#macro {
                (r#macro.source(), r#macro.expand(env)?, r#macro.flavor())
            }
            else {
                let r#struct = r#struct.as_ref().unwrap();
                let mut parameters = parameters.to_vec();
                parameters.resize(r#struct.r#struct().nb_args(), T::MacroParam::empty());
                (
                    r#struct.source(),
                    r#struct.expand(env)?,
                    AssemblerFlavor::Basm
                )
            };

            // Tokenize with the same parsing  parameters and context when possible
            let listing = match self.token.possible_span() {
                Some(span) => {
                    use crate::ParserContextBuilder;
                    let ctx_builder = ParserContextBuilder::default() // nothing is specified
                        //                    from(span.state.clone())
                        .set_state(span.state.state.clone())
                        .set_options(span.state.options.clone())
                        .set_context_name(&format!(
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
            };
            listing
        };

        let expand_state = ExpandStateTryBuilder {
            listing,
            processed_tokens_builder: |listing: &LocatedListing| {
                build_processed_tokens_list(listing, env)
            }
        }
        .try_build()?;

        self.state = Some(ProcessedTokenState::MacroCallOrBuildStruct(expand_state));

        return Ok(());
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
        let mut really_does_the_job = move || {
            let deferred = self.token.defer_listing_output();
            if !deferred {
                // dbg!(&self.token, deferred);
                let outer_token = unsafe {
                    (self.token as *const T as *const LocatedToken)
                        .as_ref()
                        .unwrap()
                };

                env.handle_output_trigger(outer_token);
            }

            // Generate the code of a macro/struct
            if self.token.is_call_macro_or_build_struct() {
                self.update_macro_or_struct_state(env)?;
            }

            // Behavior based on the token
            let res = if self.token.is_macro_definition() {
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
                let options = env.options();
                // Handle the tokens depending on their specific state
                match &mut self.state {
                    Some(ProcessedTokenState::RestrictedAssemblingEnvironment {
                        listing,
                        commands
                    }) => {
                        let mut new_commands = commands.take().unwrap();

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
                                .unwrap();
                        }
                        commands.replace(new_commands);
                        Ok(())
                    },

                    Some(ProcessedTokenState::Confined(SimpleListingState {
                        ref mut processed_tokens,
                        span
                    })) => env.visit_confined(processed_tokens, span.as_ref()),
                    Some(ProcessedTokenState::CrunchedSection {
                        listing:
                            SimpleListingState {
                                ref mut processed_tokens,
                                span
                            },
                        ref mut previous_bytes,
                        ref mut previous_compressed_bytes
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
                                "INCBIN-like directives are not allowed in a web-based assembling.".to_owned()
                            });
                        }

                        // Handle file loading
                        let fname = self.token.incbin_fname();
                        let fname = get_filename(fname, options.parse_options(), Some(env))?;

                        // get the data for the given file
                        let data = if !contents.contains_key(&fname) {
                            // need to load the file

                            let data =
                                load_binary(Either::Left(fname.as_ref()), options.parse_options())?;
                            // get a slice on the data to ease its cut
                            let mut data = &data[..];

                            if data.len() >= 128 {
                                let header = AmsdosHeader::from_buffer(&data);
                                let _info = Some(if header.represent_a_valid_file() {
                                    dbg!("TODO add a message explainng that header has been removed for", &fname);
                                    data = &data[128..];

                                    AssemblerError::AssemblingError{
                                    msg: format!("{:?} is a valid Amsdos file. It is included without its header.", fname)
                                }
                                }
                                else {
                                    AssemblerError::AssemblingError{
                                            msg: format!("{:?} does not contain a valid Amsdos file. It is fully included.", fname)
                                        }
                                });
                            }

                            contents.try_insert(fname.clone(), data.to_vec()).unwrap()
                        }
                        else {
                            contents.get(&fname).unwrap()
                        };

                        let mut data = data.as_slice();

                        // Extract the appropriate content to the file
                        let offset = self.token.incbin_offset();
                        let length = self.token.incbin_length();
                        let transformation = self.token.incbin_transformation();

                        match offset {
                            Some(offset) => {
                                let offset =
                                    env.resolve_expr_must_never_fail(offset)?.int()? as usize;
                                if offset >= data.len() {
                                    return Err(AssemblerError::AssemblingError {
                                        msg: format!(
                                            "Unable to read {:?}. Only {} are available",
                                            self.token.incbin_fname(),
                                            data.len()
                                        )
                                    });
                                }
                                data = &data[offset..];
                            },
                            None => {}
                        }

                        match length {
                            Some(length) => {
                                let length =
                                    env.resolve_expr_must_never_fail(length)?.int()? as usize;
                                if data.len() < length {
                                    return Err(AssemblerError::AssemblingError {
                                        msg: format!(
                                            "Unable to read {:?}. Only {} bytes are available ({} expected)",
                                            self.token.incbin_fname(),
                                            data.len(),
                                            length
                                        )
                                    });
                                }
                                data = &data[..length];
                            },
                            None => {}
                        }

                        let data = match transformation {
                            BinaryTransformation::None => Cow::Borrowed(data),

                            other => {
                                if data.len() == 0 {
                                    return Err(AssemblerError::EmptyBinaryFile(
                                        self.token.incbin_fname().to_string()
                                    ));
                                }

                                let crunch_type = other.crunch_type().unwrap();
                                Cow::Owned(crunch_type.crunch(&data)?)
                            }
                        };

                        env.visit_incbin(data.borrow())
                    },

                    Some(ProcessedTokenState::Include(ref mut state)) => {
                        state.handle(
                            env,
                            self.token.include_fname(),
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

                        env.inc_macro_seed();
                        let seed = env.macro_seed();
                        env.symbols_mut().push_seed(seed);

                        // save the number of prints to patch the ones added by the macro
                        // to properly locate them
                        let nb_prints = env
                            .sna.pages_info
                            .iter()
                            .map(|ti| ti.print_commands().len())
                            .collect_vec();

                        state
                            .with_processed_tokens_mut(|listing| {
                                let tokens: &mut [ProcessedToken<'_, LocatedToken>] =
                                    &mut listing[..];
                                visit_processed_tokens::<'_, LocatedToken>(tokens, env)
                            })
                            .or_else(|e| {
                                let e = AssemblerError::MacroError {
                                    name: name.into(),
                                    root: Box::new(e)
                                };
                                let caller_span = self.possible_span();
                                match caller_span {
                                    Some(span) => {
                                        Err(AssemblerError::RelocatedError {
                                            error: e.into(),
                                            span: span.clone()
                                        })
                                    },
                                    None => Err(e)
                                }
                            })?;

                        let caller_span = self.possible_span();
                        if let Some(span) = caller_span {
                            env.sna.pages_info
                                .iter_mut()
                                .zip(nb_prints.into_iter())
                                .for_each(|(ti, count)| {
                                    ti.print_commands_mut()[count..]
                                        .iter_mut()
                                        .for_each(|cmd| cmd.relocate(span.clone()))
                                });
                        }

                        env.symbols_mut().pop_seed();

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

                    Some(ProcessedTokenState::Switch(ref mut state)) => {
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
                        if !met || !broken {
                            if let Some(ref mut default) = state.default {
                                visit_processed_tokens(&mut default.processed_tokens, env)?;
                            }
                        }

                        Ok(())
                    },

                    Some(ProcessedTokenState::Warning(box token)) => {
                        let warning = AssemblerError::RelocatedWarning {
                            warning: Box::new(AssemblerError::AssemblingError {
                                msg: self.token.warning_message().to_owned()
                            }),
                            span: self.token.possible_span().unwrap().clone()
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

            if ! self.token.is_buildcpr() { // we lack of some datastructures
                env.update_dollar();
            }

            if deferred {
                let outer_token = unsafe {
                    (self.token as *const T as *const LocatedToken)
                        .as_ref()
                        .unwrap()
                };

                env.handle_output_trigger(outer_token);
            }
            Ok(res)
        };

        really_does_the_job().map_err(|e| {
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
        let env = Env::default();

        let processed = build_processed_token(token, &env);
        assert!(matches!(
            processed.unwrap().state,
            Some(ProcessedTokenState::Include(..))
        ));
    }
}

use std::any::Any;
use std::borrow::{Cow, Borrow};
use std::collections::{BTreeMap, HashMap};
use std::collections::btree_map::Entry;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cpclib_common::itertools::Itertools;
use cpclib_disc::amsdos::AmsdosHeader;
use cpclib_tokens::symbols::{Macro, SymbolFor, SymbolsTableTrait};
use cpclib_tokens::{
    BinaryTransformation, Listing, ListingElement, MacroParamElement, TestKindElement,
    ToSimpleToken, Token
};
use either::Either;
use ouroboros::*;

use super::file::{load_binary, get_filename};
use super::function::{Function, FunctionBuilder};
use super::r#macro::Expandable;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::instructions::Cruncher;
use crate::preamble::{
    parse_z80_str, parse_z80_str_with_context, LocatedListing, MayHaveSpan, Z80Span
};
use crate::{r#macro, AssemblerError, Env, LocatedToken, ParserContext, Visited};

#[cfg(not(target_arch = "wasm32"))]
use cpclib_common::rayon::prelude::*;

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
    Repeat(SimpleListingState<'token, T>),
    RepeatUntil(SimpleListingState<'token, T>),
    Rorg(SimpleListingState<'token, T>)
}


#[derive(PartialEq, Eq, Clone, Debug, Default)]
struct IncbinState{
    contents: BTreeMap<PathBuf, Vec<u8>>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionDefinitionState(Option<Arc<Function>>);


#[derive(PartialEq, Eq, Clone, Debug)]
struct IncludeState(
    BTreeMap<PathBuf, IncludeStateInner>
);

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

        for idx in 0..self.token.if_nb_tests() {
            let (test, _) = self.token.if_test(idx);
            let token_adr = test as *const _ as usize;

            // Expression must be true
            if test.is_true_test() {
                let exp = test.expr_unchecked();
                // Expression must be true
                let value = env.resolve_expr_must_never_fail(exp)?;
                if value.bool()? {
                    selected_idx = Some(idx);
                    break;
                }
            }
            // Expression must be false
            else if test.is_false_test() {
                let exp = test.expr_unchecked();
                let value = env.resolve_expr_must_never_fail(exp)?;
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
            // Label must exist
            else if test.is_label_exists_test() {
                let label = test.label_unchecked();
                if env.symbols().symbol_exist_in_current_pass(label)? {
                    selected_idx = Some(idx);
                    break;
                }
            }
            // Label must not exist
            else {
                let label = test.label_unchecked();
                if !env.symbols().symbol_exist_in_current_pass(label)? {
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
                    let listing = build_processed_tokens_list(listing, env);
                    self.tests_listing.insert(selected_idx, listing);
                }
                self.tests_listing.get_mut(&selected_idx)
            }
            None => {
                // build else listing if needed
                if self.else_listing.is_none() && self.token.if_else().is_some() {
                    let listing = self.token.if_else();
                    self.else_listing =
                        listing.map(|listing| build_processed_tokens_list(listing, env));
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
) -> ProcessedToken<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{
    let state = if token.is_if() {
        let state = IfState::new(token);
        Some(ProcessedTokenState::If(state))
    }
    else if token.is_include(){
        let fname = token.include_fname();
        let ctx = &env.ctx;
        match get_filename(fname, ctx, Some(env)) {
            Ok(fname) => {
                 match read_source(fname.clone(), ctx, Some(env)) {
                    Ok(content) => {
                        let new_ctx = {
                            let mut new_ctx = ctx.clone();
                            new_ctx.set_current_filename(fname.clone());
                            new_ctx
                        };
            
                        match parse_z80_str_with_context(content, new_ctx) {
                            Ok(listing) => {
                                let include_state = IncludeStateInnerBuilder {
                                    listing,
                                    processed_tokens_builder: |listing: &LocatedListing| {
                                        build_processed_tokens_list(listing.as_slice(), env)
                                    }
                                }
                                .build();

                                let mut map = BTreeMap::new();
                                map.insert(fname, include_state);
                
                                Some(ProcessedTokenState::Include(IncludeState(map))) 
                            },
                            Err(_) => Some(ProcessedTokenState::Include(Default::default())),
                        }

                    },
                    Err(_) => Some(ProcessedTokenState::Include(Default::default())),
                }
            },
            Err(_) =>  Some(ProcessedTokenState::Include(Default::default())) // we were unable to get the filename with the provided information
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
                ),
                span: token.possible_span().cloned()
            },
            previous_bytes: None,
            previous_compressed_bytes: None
        })
    }
    else if token.is_for() {
        Some(ProcessedTokenState::For(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.for_listing(), env),
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
            processed_tokens: build_processed_tokens_list(token.iterate_listing(), env),
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_repeat() {
        Some(ProcessedTokenState::Repeat(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.repeat_listing(), env),
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_repeat_until() {
        Some(ProcessedTokenState::RepeatUntil(SimpleListingState {
            processed_tokens: build_processed_tokens_list(token.repeat_until_listing(), env),
            span: token.possible_span().cloned()
        }))
    }
    else if token.is_rorg() {
        Some(ProcessedTokenState::Rorg(SimpleListingState{
            processed_tokens: build_processed_tokens_list(token.rorg_listing(), env),
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

    ProcessedToken { token, state }
}

pub fn build_processed_tokens_list<
    'token,
    T: Visited + Debug + Sync + ListingElement + MayHaveSpan
>(
    tokens: &'token [T],
    env: &Env
) -> Vec<ProcessedToken<'token, T>>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt
{

    #[cfg(not(target_arch = "wasm32"))]
    let iter = tokens.par_iter();
    #[cfg(target_arch = "wasm32")]
    let iter = tokens.iter();

    iter.map(|t| build_processed_token(t, env))
        .collect::<Vec<_>>()
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
    for token in tokens.iter_mut() {
        token.visited(env)?;
    }

    Ok(())
}

impl<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan> MayHaveSpan
    for ProcessedToken<'token, T>
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

impl<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan> ProcessedToken<'token, T> {
    /// Generate the tokens needed for the macro or the struct
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
            let r#struct = env
                .symbols()
                .struct_value(name)?
                .map(|s| r#macro::StructWithArgs::build(s, parameters))
                .transpose()?;

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
                    }
                    None => Err(e)
                };
            }

            // get the generated code
            // TODO handle some errors there
            let (source, code) = if let Some(ref r#macro) = r#macro {
                (r#macro.source(), r#macro.expand(env)?)
            }
            else {
                let r#struct = r#struct.as_ref().unwrap();
                let mut parameters = parameters.to_vec();
                parameters.resize(r#struct.r#struct().nb_args(), T::MacroParam::empty());
                (r#struct.source(), r#struct.expand(env)?)
            };

            // Tokenize with the same parsing  parameters and context when possible
            let listing = match self.token.possible_span() {
                Some(span) => {
                    let mut ctx = span.extra.deref().clone();
                    ctx.remove_filename();
                    ctx.set_context_name(&format!(
                        "{}:{}:{} > {} {}:",
                        source.map(|s| s.fname()).unwrap_or_else(|| "???"),
                        source.map(|s| s.line()).unwrap_or(0),
                        source.map(|s| s.column()).unwrap_or(0),
                        if r#macro.is_some() { "MACRO" } else { "STRUCT" },
                        name,
                    ));
                    let code = Box::new(code);
                    parse_z80_str_with_context(code.as_ref(), ctx)?
                }
                _ => parse_z80_str(&code)?
            };
            listing
        };

        let expandState = ExpandStateBuilder {
            listing,
            processed_tokens_builder: |listing: &LocatedListing| {
                build_processed_tokens_list(listing.as_slice(), env)
            }
        }
        .build();

        self.state = Some(ProcessedTokenState::MacroCallOrBuildStruct(expandState));

        return Ok(());
    }

 
}

/// Read the content of the source file.
/// Uses the context to obtain the appropriate file other the included directories
pub fn read_source<P: AsRef<Path>>(fname: P, ctx: &ParserContext, env: Option<&Env>) -> Result<String, AssemblerError> {

    let fname = fname.as_ref();
    let mut f = File::open(&fname).map_err(|e| {
        AssemblerError::IOError {
            msg: format!("Unable to open {:?}. {}", fname, e)
        }
    })?;

    let mut content = Vec::new();
    f.read_to_end(&mut content).map_err(|e| {
        AssemblerError::IOError {
            msg: format!("Unable to read {:?}. {}", fname, e.to_string())
        }
    })?;

    let result = chardet::detect(&content);
    let coder =
        encoding::label::encoding_from_whatwg_label(chardet::charset2encoding(&result.0));

    let content = match coder {
        Some(coder) => {
            let utf8reader = coder
                .decode(&content, encoding::DecoderTrap::Ignore)
                .expect("Error");
            utf8reader.to_string()
        }
        None => {
            return Err(AssemblerError::IOError {
                msg: format!("Encoding error for {:?}.", fname)
            });
        }
    };

    Ok(content)
    
}

impl<'token, T: Visited + Debug + ListingElement + Sync + MayHaveSpan> ProcessedToken<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    /// Due to the state management, the signature requires mutability
    pub fn visited(&mut self, env: &mut Env) -> Result<(), AssemblerError> {

        let mut really_does_the_job = move || {
        let ctx = &env.ctx;

            {

                // Generate the code of a macro/struct
                if self.token.is_call_macro_or_build_struct() {
                    self.update_macro_or_struct_state(env)?;
                }
            }

            // Behavior based on the token
            let res = if self.token.is_macro_definition() {
                // TODO really implement logic here
                let name = self.token.macro_definition_name();
                let arguments = self.token.macro_definition_arguments();
                let code = self.token.macro_definition_code();
                env.visit_macro_definition(name, &arguments, code, self.possible_span())
            }
            // Behavior based on the state (for ease of writting)
            else {
                // Handle the tokens depending on their specific state
                match &mut self.state {
                    Some(ProcessedTokenState::CrunchedSection{listing:  SimpleListingState {
                        ref mut processed_tokens,
                        span
                    }, 
                    ref mut previous_bytes, 
                    ref mut previous_compressed_bytes}) => {
                        env.visit_crunched_section(
                            self.token.crunched_section_kind(),
                            processed_tokens,
                            previous_bytes,
                            previous_compressed_bytes,
                            span.as_ref()
                        )
                    }

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
                    }

                    Some(ProcessedTokenState::FunctionDefinition(FunctionDefinitionState(
                        Some(fun)
                    ))) => {
                        // TODO check if the funtion has already been defined during this pass
                        Ok(())
                    }
                    Some(ProcessedTokenState::FunctionDefinition(FunctionDefinitionState(
                        option
                    ))) => {
                        let name = self.token.function_definition_name();
                        if !env.functions.contains_key(name) {
                            let inner = self.token.function_definition_inner();
                            let params = self.token.function_definition_params();

                            let inner = build_processed_tokens_list(inner, env);
                            let f =
                                Arc::new(unsafe { FunctionBuilder::new(&name, &params, inner) }?);
                            option.replace(f.clone());

                            env.functions.insert(name.to_owned(), f);
                        }
                        else {
                            // TODO raise an error ?
                        }
                        Ok(())
                    }

                    Some(ProcessedTokenState::Incbin(IncbinState{contents})) => {
                        if cfg!(target_arch = "wasm32") {
                            return Err(AssemblerError::AssemblingError { msg: 
                                "INCBIN-like directives are not allowed in a web-based assembling.".to_owned()
                            });
                        }

                        // Handle file loading
                        let fname = self.token.incbin_fname();
                        let fname = get_filename(fname, ctx, Some(env))?;

                        // get the data for the given file
                        let data = if !contents.contains_key(&fname){
                            // need to load the file
                           
                            let data = load_binary(Either::Left(fname.as_ref()), ctx, env)?;
                            // get a slice on the data to ease its cut
                            let mut data = &data[..];

                            if data.len() >= 128 {
                                let header = AmsdosHeader::from_buffer(&data);
                                let info = Some(if header.is_checksum_valid() {
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
                                let offset = env.resolve_expr_must_never_fail(offset)?.int()? as usize;
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
                            }
                            None => {}
                        }

                        match length {
                            Some(length) => {
                                let length = env.resolve_expr_must_never_fail(length)?.int()? as usize;
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
                            }
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


                    Some(ProcessedTokenState::Include(IncludeState(ref mut contents))) => {
                        if cfg!(target_arch = "wasm32") {
                            return Err(AssemblerError::AssemblingError { msg: 
                                "INCLUDE-like directives are not allowed in a web-based assembling.".to_owned()
                            });
                        }

                        let fname = self.token.include_fname();
                        let fname = get_filename(fname, ctx, Some(env))?;

                        let namespace = self.token.include_namespace();
                        let once = self.token.include_once();

                        // Process the inclusion only if necessary
                        if (!once) || (!env.has_included(&fname)) {
                            // Build the state if needed / retreive it otherwhise
                            let state: &mut IncludeStateInner = if !contents.contains_key(&fname) {
                                let content = read_source(fname.clone(), ctx, Some(env))?;
                    
                                let new_ctx = {
                                    let mut new_ctx = ctx.clone();
                                    new_ctx.set_current_filename(fname.clone());
                                    new_ctx
                                };
                    
                                let listing = parse_z80_str_with_context(content, new_ctx)?;
                                let include_state = IncludeStateInnerBuilder {
                                    listing,
                                    processed_tokens_builder: |listing: &LocatedListing| {
                                        build_processed_tokens_list(listing.as_slice(), env)
                                    }
                                }
                                .build();

                                contents.try_insert(fname.clone(), include_state).unwrap()
                            } else {
                                contents.get_mut(&fname).unwrap()
                            };


                            // handle the listing
                            env.mark_included(fname);

                            // handle module if necessary
                            if let Some(namespace) = namespace {
                                env.enter_namespace(namespace)?;
                                // TODO handle the locating of error
                                //.map_err(|e| e.locate(span.clone()))?;
                            }

                            // Visit the included listing
                            state.with_processed_tokens_mut(|tokens| {
                                let tokens: &mut [ProcessedToken<'_, LocatedToken>] =
                                    &mut tokens[..];
                                visit_processed_tokens::<'_, LocatedToken>(tokens, env)
                            })?;

                            // Remove module if necessary
                            if namespace.is_some() {
                                env.leave_namespace()?;
                                //.map_err(|e| e.locate(span.clone()))?;
                            }

                            Ok(())
                        } else {
                            Ok(())
                        }
                    
                    }



                    Some(ProcessedTokenState::If(if_state)) => {
                        let listing = if_state.choose_listing_to_assemble(env)?;

                        if let Some(listing) = listing {
                            visit_processed_tokens(listing, env)?;
                        }

                        Ok(())
                    }

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
                    }

                    Some(ProcessedTokenState::MacroCallOrBuildStruct(state)) => {
                        let name = self.token.macro_call_name();

                        env.inc_macro_seed();
                        let seed = env.macro_seed();
                        env.symbols_mut().push_seed(seed);

                        // save the number of prints to patch the ones added by the macro
                        // to properly locate them
                        let nb_prints = env
                            .pages_info_sna
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
                                    }
                                    None => Err(e)
                                }
                            })?;

                        let caller_span = self.possible_span();
                        if let Some(span) = caller_span {
                            env.pages_info_sna
                                .iter_mut()
                                .zip(nb_prints.into_iter())
                                .for_each(|(ti, count)| {
                                    ti.print_commands_mut()[count..]
                                        .iter_mut()
                                        .for_each(|cmd| cmd.relocate(span.clone()))
                                });
                        }

                        env.symbols_mut().pop_seed();
                        //   dbg!("done");

                        Ok(())
                    }

                    Some(ProcessedTokenState::Repeat(SimpleListingState {
                        processed_tokens,
                        ..
                    })) => {
                        env.visit_repeat(
                            self.token.repeat_count(),
                            processed_tokens,
                            self.token.repeat_counter_name(),
                            self.token.repeat_counter_start(),
                            self.token.possible_span()
                        )
                    }

                    Some(ProcessedTokenState::RepeatUntil(SimpleListingState {
                        processed_tokens,
                        ..
                    })) => {
                        env.visit_repeat_until(
                            self.token.repeat_until_condition(),
                            processed_tokens,
                            self.token.possible_span()
                        )
                    }


                    Some(ProcessedTokenState::Rorg(SimpleListingState {
                        processed_tokens,
                        span
                    })) => {
                        env.visit_rorg(
                            self.token.rorg_expr(),
                            processed_tokens,
                            span.as_ref()
                        )
                    }

                    // no state implies a standard visit
                    None => self.token.visited(env)
                }
            }?;

            env.update_dollar();
            Ok(res)

        };

        really_does_the_job().map_err(|e| AssemblerError::AlreadyRenderedError(e.to_string()))
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
    use crate::preamble::{parse_include, Z80Span};

    #[test]
    fn test_located_include() {
        let src = " include \"toto\"";
        let mut ctx = ParserContext::default();
        ctx.source = Some(src);

        let span = Z80Span::new_extra(src, &ctx);

        let token = parse_include(span).unwrap().1;
        let env = Env::default();

        dbg!(&token);

        let processed = build_processed_token(&token, &env);
        dbg!(&processed);
        assert!(matches!(
            processed.state,
            Some(ProcessedTokenState::Include(..))
        ));
    }
}

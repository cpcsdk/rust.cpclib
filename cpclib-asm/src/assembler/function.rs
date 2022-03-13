use std::borrow::Borrow;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};

use cpclib_common::itertools::Itertools;
use cpclib_common::lazy_static;
use cpclib_tokens::{Expr, ExprResult, ListingElement, TestKindElement, ToSimpleToken, Token};

use super::list::{
    list_argsort, list_get, list_len, list_push, list_sort, list_sublist, string_from_list,
    string_new, string_push
};
use super::matrix::{
    matrix_col, matrix_get, matrix_height, matrix_row, matrix_set_col, matrix_set_row, matrix_width
};
use super::processed_token::{build_processed_tokens_list, ProcessedToken};
use super::{file, Env};
use crate::assembler::list::{list_new, list_set};
use crate::assembler::matrix::{matrix_new, matrix_set};
use crate::error::{AssemblerError, ExpressionError};
use crate::implementation::expression::ExprEvaluationExt;
use crate::preamble::{LocatedToken, MayHaveSpan, ParserContext, ParsingState};
use crate::Visited;

/// Returns the expression of the RETURN directive
pub trait ReturnExpr {
    fn return_expr(&self) -> Option<&Expr>;
}

impl ReturnExpr for Token {
    fn return_expr(&self) -> Option<&Expr> {
        match self {
            Token::Return(exp) => Some(exp),
            _ => None
        }
    }
}

impl ReturnExpr for LocatedToken {
    fn return_expr(&self) -> Option<&Expr> {
        match self {
            LocatedToken::Standard { token, .. } => token.return_expr(),
            _ => None
        }
    }
}

#[derive(Debug)]
pub struct AnyFunction<'token, T: ListingElement + Visited + ToSimpleToken + Sync + MayHaveSpan>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    name: String,
    args: Vec<String>,
    inner: RwLock<Vec<ProcessedToken<'token, T>>>
}

impl<'token, T: ListingElement + Visited + ToSimpleToken + Sync + MayHaveSpan> Clone
    for AnyFunction<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            args: self.args.clone(),
            inner: todo!()
        }
    }
}

impl<'token, T: ListingElement + Visited + ToSimpleToken + Sync + MayHaveSpan> PartialEq
    for AnyFunction<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.args == other.args
    }
}

impl<'token, T: ListingElement + Visited + ToSimpleToken + Sync + MayHaveSpan> Eq
    for AnyFunction<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
}

impl<'token, T: ListingElement + Visited + ToSimpleToken + Sync + MayHaveSpan>
    AnyFunction<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder
{
    fn new<S1: AsRef<str>, S2: Borrow<str>>(
        name: S1,
        args: &[S2],
        inner: Vec<ProcessedToken<'token, T>>
    ) -> Self {
        AnyFunction {
            name: name.as_ref().to_owned(),
            args: args.iter().map(|s| s.borrow().into()).collect_vec(),
            inner: inner.into()
        }
    }
}

impl<'token, T: ListingElement + Visited + ToSimpleToken + Sync + ReturnExpr + MayHaveSpan>
    AnyFunction<'token, T>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt,
    <<T as cpclib_tokens::ListingElement>::TestKind as TestKindElement>::Expr: ExprEvaluationExt,
    ProcessedToken<'token, T>: FunctionBuilder + Clone
{
    pub fn eval(
        &self,
        init_env: &Env,
        params: &[ExprResult]
    ) -> Result<ExprResult, AssemblerError> {
        if self.args.len() != params.len() {
            return Err(AssemblerError::FunctionWithWrongNumberOfArguments(
                self.name.clone(),
                self.args.len(),
                params.len()
            ));
        }
        // we copy the environement to be sure no bug can modify it
        // and to keep the symbol table fixed.
        // a better alternative would be to backup the symbol table
        let mut env = init_env.clone();

        // set the parameters
        for param in self.args.iter().zip(params.iter()) {
            // TODO modify the code according to the value
            env.add_function_parameter_to_symbols_table(
                format!("{{{}}}", param.0),
                param.1.clone()
            )
            .unwrap();
        }

        let inner = self.inner.read().unwrap();
        let mut inner = inner.iter().cloned().collect_vec(); // BUG: memory issue in case of error generated
        for token in inner.iter_mut() {
            token
                .visited(&mut env)
                .map_err(|e| AssemblerError::FunctionError(self.name.clone(), box e))?;

            if env.return_value.is_some() {
                let extra_print = &env.active_page_info().print_commands()
                    [init_env.active_page_info().print_commands().len()..];
                let extra_assert = &env.active_page_info().failed_assert_commands()
                    [init_env.active_page_info().failed_assert_commands().len()..];

                init_env
                    .extra_print_from_function
                    .write()
                    .unwrap()
                    .extend_from_slice(extra_print);
                init_env
                    .extra_failed_assert_from_function
                    .write()
                    .unwrap()
                    .extend_from_slice(extra_assert);

                return Ok(env.return_value.take().unwrap());
            }
        }

        Err(AssemblerError::FunctionWithoutReturn(self.name.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Function {
    Located(AnyFunction<'static, LocatedToken>), // Here we cheat, it should not be 'static ...
    Standard(AnyFunction<'static, Token>),
    HardCoded(HardCodedFunction)
}

lazy_static::lazy_static! {
     static ref HARD_CODED_FUNCTIONS: HashMap<&'static str, Function> = velcro::hash_map! {
        "mode0_byte_to_pen_at": Function::HardCoded(HardCodedFunction::Mode0ByteToPenAt),
        "mode1_byte_to_pen_at": Function::HardCoded(HardCodedFunction::Mode1ByteToPenAt),
        "mode2_byte_to_pen_at": Function::HardCoded(HardCodedFunction::Mode2ByteToPenAt),
        "pen_at_mode0_byte": Function::HardCoded(HardCodedFunction::PenAtToMode0Byte),
        "pen_at_mode1_byte":Function::HardCoded(HardCodedFunction::PenAtToMode1Byte),
        "pen_at_mode2_byte": Function::HardCoded(HardCodedFunction::PenAtToMode2Byte),
        "pens_to_mode0_byte": Function::HardCoded(HardCodedFunction::PensToMode0Byte),
        "pens_to_mode1_byte":
        Function::HardCoded(HardCodedFunction::PensToMode1Byte),
        "pens_to_mode2_byte": Function::HardCoded(HardCodedFunction::PensToMode2Byte),
        "list_new": Function::HardCoded(HardCodedFunction::ListNew),
        "list_get": Function::HardCoded(HardCodedFunction::ListGet),
        "list_set": Function::HardCoded(HardCodedFunction::ListSet),
        "list_len": Function::HardCoded(HardCodedFunction::ListLen),
        "list_sublist": Function::HardCoded(HardCodedFunction::ListSublist),
        "list_sort": Function::HardCoded(HardCodedFunction::ListSort),
        "list_argsort": Function::HardCoded(HardCodedFunction::ListArgsort),
        "list_push": Function::HardCoded(HardCodedFunction::ListPush),
        "string_new": Function::HardCoded(HardCodedFunction::StringNew),
        "string_push": Function::HardCoded(HardCodedFunction::StringPush),
        "string_concat": Function::HardCoded(HardCodedFunction::StringConcat),
        "string_from_list": Function::HardCoded(HardCodedFunction::StringFromList),
        "assemble": Function::HardCoded(HardCodedFunction::Assemble),
        "matrix_new": Function::HardCoded(HardCodedFunction::MatrixNew),
        "matrix_set": Function::HardCoded(HardCodedFunction::MatrixSet),
        "matrix_get": Function::HardCoded(HardCodedFunction::MatrixGet),
        "matrix_col": Function::HardCoded(HardCodedFunction::MatrixCol),
        "matrix_row": Function::HardCoded(HardCodedFunction::MatrixRow),
        "matrix_set_row": Function::HardCoded(HardCodedFunction::MatrixSetRow),
        "matrix_set_col": Function::HardCoded(HardCodedFunction::MatrixSetCol),
        "matrix_width": Function::HardCoded(HardCodedFunction::MatrixWidth),
        "matrix_height": Function::HardCoded(HardCodedFunction::MatrixHeight),
        "load": Function::HardCoded(HardCodedFunction::Load)
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardCodedFunction {
    Mode0ByteToPenAt,
    Mode1ByteToPenAt,
    Mode2ByteToPenAt,

    PenAtToMode0Byte,
    PenAtToMode1Byte,
    PenAtToMode2Byte,

    PensToMode0Byte,
    PensToMode1Byte,
    PensToMode2Byte,

    ListNew,
    ListSet,
    ListGet,
    ListSublist,
    ListLen,
    ListPush,
    ListSort,
    ListArgsort,

    MatrixNew,
    MatrixSet,
    MatrixGet,
    MatrixCol,
    MatrixRow,
    MatrixSetRow,
    MatrixSetCol,
    MatrixWidth,
    MatrixHeight,

    StringNew,
    StringPush,
    StringConcat,
    StringFromList,

    Load,
    Assemble
}

impl HardCodedFunction {
    pub fn nb_expected_params(&self) -> Option<usize> {
        match self {
            HardCodedFunction::Mode0ByteToPenAt => Some(2),
            HardCodedFunction::Mode1ByteToPenAt => Some(2),
            HardCodedFunction::Mode2ByteToPenAt => Some(2),

            HardCodedFunction::PenAtToMode0Byte => Some(2),
            HardCodedFunction::PenAtToMode1Byte => Some(2),
            HardCodedFunction::PenAtToMode2Byte => Some(2),

            HardCodedFunction::PensToMode0Byte => Some(2),
            HardCodedFunction::PensToMode1Byte => Some(4),
            HardCodedFunction::PensToMode2Byte => Some(8),

            HardCodedFunction::ListNew => Some(2),
            HardCodedFunction::ListSet => Some(3),
            HardCodedFunction::ListGet => Some(2),
            HardCodedFunction::ListSublist => Some(3),
            HardCodedFunction::ListLen => Some(1),
            HardCodedFunction::ListSort => Some(1),
            HardCodedFunction::ListArgsort => Some(1),
            HardCodedFunction::ListPush => Some(2),

            HardCodedFunction::StringNew => Some(2),
            HardCodedFunction::StringPush => Some(2),
            HardCodedFunction::StringFromList => Some(1),
            HardCodedFunction::StringConcat => None,

            HardCodedFunction::Assemble => Some(1),

            HardCodedFunction::MatrixNew => Some(3),
            HardCodedFunction::MatrixSet => Some(4),
            HardCodedFunction::MatrixCol => Some(2),
            HardCodedFunction::MatrixRow => Some(2),
            HardCodedFunction::MatrixGet => Some(3),
            HardCodedFunction::MatrixSetRow => Some(3),
            HardCodedFunction::MatrixSetCol => Some(3),

            HardCodedFunction::MatrixWidth => Some(1),
            HardCodedFunction::MatrixHeight => Some(1),

            HardCodedFunction::Load => Some(1)
        }
    }

    pub fn by_name(name: &str) -> Option<&Function> {
        HARD_CODED_FUNCTIONS.get(name.to_lowercase().as_str())
    }

    pub fn name(&self) -> &str {
        HARD_CODED_FUNCTIONS
            .iter()
            .find_map(|(k, v)| {
                match v {
                    Function::HardCoded(v) => {
                        if v == self {
                            Some(k)
                        }
                        else {
                            None
                        }
                    }
                    _ => None
                }
            })
            .unwrap() // Cannot fail by definition
    }

    pub fn eval(&self, env: &Env, params: &[ExprResult]) -> Result<ExprResult, AssemblerError> {
        match self.nb_expected_params() {
            Some(nb) => {
                if nb != params.len() {
                    return Err(AssemblerError::FunctionWithWrongNumberOfArguments(
                        self.name().into(),
                        nb,
                        params.len()
                    ));
                }
            }
            _ => {}
        }

        match self {
            HardCodedFunction::Mode0ByteToPenAt => {
                Ok(
                    cpclib_image::pixels::mode0::byte_to_pens(params[0].int()? as _)
                        [params[1].int()? as usize % 2]
                        .number()
                        .into()
                )
            }
            HardCodedFunction::Mode1ByteToPenAt => {
                Ok(
                    cpclib_image::pixels::mode1::byte_to_pens(params[0].int()? as _)
                        [params[1].int()? as usize % 4]
                        .number()
                        .into()
                )
            }
            HardCodedFunction::Mode2ByteToPenAt => {
                Ok(
                    cpclib_image::pixels::mode2::byte_to_pens(params[0].int()? as _)
                        [params[1].int()? as usize % 8]
                        .number()
                        .into()
                )
            }

            HardCodedFunction::PenAtToMode0Byte => {
                Ok(cpclib_image::pixels::mode0::pen_to_pixel_byte(
                    (params[0].int()? as u8 % 16).into(),
                    (params[1].int()? as u8 % 2).into()
                )
                .into())
            }
            HardCodedFunction::PenAtToMode1Byte => {
                Ok(cpclib_image::pixels::mode1::pen_to_pixel_byte(
                    (params[0].int()? as u8 % 4).into(),
                    (params[1].int()? as u8 % 4).into()
                )
                .into())
            }

            HardCodedFunction::PenAtToMode2Byte => {
                Ok(cpclib_image::pixels::mode2::pen_to_pixel_byte(
                    (params[0].int()? as u8 % 2).into(),
                    (params[1].int()? as u8 % 8).into()
                )
                .into())
            }

            HardCodedFunction::PensToMode0Byte => {
                Ok(cpclib_image::pixels::mode0::pens_to_byte(
                    params[0].int()?.into(),
                    params[1].int()?.into()
                )
                .into())
            }
            HardCodedFunction::PensToMode1Byte => {
                Ok(cpclib_image::pixels::mode1::pens_to_byte(
                    params[0].int()?.into(),
                    params[1].int()?.into(),
                    params[2].int()?.into(),
                    params[3].int()?.into()
                )
                .into())
            }
            HardCodedFunction::PensToMode2Byte => {
                Ok(cpclib_image::pixels::mode2::pens_to_byte(
                    params[0].int()?.into(),
                    params[1].int()?.into(),
                    params[2].int()?.into(),
                    params[3].int()?.into(),
                    params[4].int()?.into(),
                    params[5].int()?.into(),
                    params[6].int()?.into(),
                    params[7].int()?.into()
                )
                .into())
            }
            HardCodedFunction::ListNew => Ok(list_new(params[0].int()? as _, params[1].clone())),
            HardCodedFunction::ListSet => {
                list_set(params[0].clone(), params[1].int()? as _, params[2].clone())
            }
            HardCodedFunction::ListGet => list_get(&params[0], params[1].int()? as _),
            HardCodedFunction::ListPush => list_push(params[0].clone(), params[1].clone()),

            HardCodedFunction::StringNew => string_new(params[0].int()? as _, params[1].clone()),
            HardCodedFunction::ListLen => list_len(&params[0]),
            HardCodedFunction::ListSublist => {
                list_sublist(&params[0], params[1].int()? as _, params[2].int()? as _)
            }

            HardCodedFunction::StringPush => string_push(params[0].clone(), params[1].clone()),

            HardCodedFunction::StringFromList => string_from_list(params[0].clone()),

            HardCodedFunction::Assemble => assemble(params[0].clone(), env),
            HardCodedFunction::StringConcat => {
                let mut base = params[0].clone();
                for i in 1..params.len() {
                    base = string_push(base, params[i].clone())?
                }
                Ok(base)
            }
            HardCodedFunction::ListSort => list_sort(params[0].clone()),
            HardCodedFunction::ListArgsort => list_argsort(&params[0]),

            HardCodedFunction::MatrixNew => {
                Ok(matrix_new(
                    params[0].int()? as _,
                    params[1].int()? as _,
                    params[2].clone()
                ))
            }
            HardCodedFunction::MatrixSet => {
                matrix_set(
                    params[0].clone(),
                    params[1].int()? as _,
                    params[2].int()? as _,
                    params[3].clone()
                )
            }
            HardCodedFunction::MatrixGet => {
                matrix_get(&params[0], params[1].int()? as _, params[2].int()? as _)
            }
            HardCodedFunction::MatrixCol => matrix_col(&params[0], params[1].int()? as _),
            HardCodedFunction::MatrixRow => matrix_row(&params[0], params[1].int()? as _),
            HardCodedFunction::MatrixSetRow => {
                matrix_set_row(params[0].clone(), params[1].int()? as _, &params[2])
            }

            HardCodedFunction::MatrixSetCol => {
                matrix_set_col(params[0].clone(), params[1].int()? as _, &params[2])
            }
            HardCodedFunction::MatrixWidth => matrix_width(&params[0]),
            HardCodedFunction::MatrixHeight => matrix_height(&params[0]),
            HardCodedFunction::Load => {
                let fname = params[0].string()?;
                let ctx = &env.ctx;
                let data = file::load_binary(fname, ctx)?;
                Ok(ExprResult::from(data.as_slice()))
            }
        }
    }
}

impl Function {
    /// Be sure the function lives shorter than inner
    pub unsafe fn new_located<'token, S1: AsRef<str>, S2: Borrow<str>>(
        name: &S1,
        args: &[S2],
        inner: Vec<ProcessedToken<'token, LocatedToken>>
    ) -> Result<Self, AssemblerError> {
        if inner.is_empty() {
            return Err(AssemblerError::FunctionWithEmptyBody(
                name.as_ref().to_owned()
            ));
        }

        let inner = std::mem::transmute(inner);

        return Ok(Function::Located(AnyFunction::new(name, args, inner)));
    }

    pub unsafe fn new_standard<'token, S1: AsRef<str>, S2: Borrow<str>>(
        name: &S1,
        args: &[S2],
        inner: Vec<ProcessedToken<'token, Token>>
    ) -> Result<Self, AssemblerError> {
        if inner.is_empty() {
            return Err(AssemblerError::FunctionWithEmptyBody(
                name.as_ref().to_owned()
            ));
        }

        let inner = std::mem::transmute(inner);

        return Ok(Function::Standard(AnyFunction::new(name, args, inner)));
    }

    /// Be sure the function lives shorter than inner

    pub fn eval(&self, env: &Env, params: &[ExprResult]) -> Result<ExprResult, AssemblerError> {
        match self {
            Self::Located(f) => f.eval(env, params),
            Self::Standard(f) => f.eval(env, params),
            Self::HardCoded(f) => f.eval(env, params)
        }
    }
}

pub trait FunctionBuilder {
    unsafe fn new<S1: AsRef<str>, S2: Borrow<str>>(
        name: &S1,
        args: &[S2],
        inner: Vec<Self>
    ) -> Result<Function, AssemblerError>
    where
        Self: Sized;
}

impl<'token> FunctionBuilder for ProcessedToken<'token, LocatedToken> {
    unsafe fn new<S1: AsRef<str>, S2: Borrow<str>>(
        name: &S1,
        args: &[S2],
        inner: Vec<Self>
    ) -> Result<Function, AssemblerError>
    where
        Self: Sized
    {
        Function::new_located(name, args, inner)
    }
}

impl<'token> FunctionBuilder for ProcessedToken<'token, Token> {
    unsafe fn new<S1: AsRef<str>, S2: Borrow<str>>(
        name: &S1,
        args: &[S2],
        inner: Vec<Self>
    ) -> Result<Function, AssemblerError>
    where
        Self: Sized
    {
        Function::new_standard(name, args, inner)
    }
}

/// Assemble a simple listing with no directives.
/// Warning !!! As the env is read only, we cannot assemble directly inside
/// To overcome that, we use a bank in a copied Env. It has not been deeply tested
pub fn assemble(code: ExprResult, base_env: &Env) -> Result<ExprResult, AssemblerError> {
    let code = match code {
        ExprResult::String(code) => code,
        _ => {
            return Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                box AssemblerError::AssemblingError {
                    msg: "Wrong type".to_owned()
                }
            )));
        }
    };

    let mut parser_context = ParserContext::default();
    parser_context.state = ParsingState::GeneratedLimited;
    parser_context.context_name = Some("Generated source".to_owned());
    let tokens = crate::parse_z80_str_with_context(code, parser_context)?;

    let mut env = Env::default();
    env.symbols = base_env.symbols().clone();
    env.start_new_pass();
    env.visit_bank(None)?; // assemble in a new bank
    env.visit_listing(&tokens)?;
    let bank_info = env.banks.pop().unwrap();
    match &bank_info.1.startadr {
        Some(startadr) => {
            let bytes = bank_info.0[*startadr as _..=bank_info.1.maxadr as _]
                .iter()
                .map(|b| ExprResult::from(*b))
                .collect_vec();
            Ok(ExprResult::List(bytes))
        }
        None => Ok(ExprResult::List(Default::default()))
    }
}

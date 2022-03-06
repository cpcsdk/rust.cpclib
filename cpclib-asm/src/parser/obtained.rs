use std::borrow::Cow;
use std::ops::Deref;

use cpclib_common::itertools::Itertools;
use cpclib_common::nom::bytes::complete::tag;
use cpclib_common::nom::character::complete::space0;
use cpclib_common::nom::combinator::{cut, opt};
use cpclib_common::nom::error::{context, ErrorKind, VerboseError};
use cpclib_common::nom::multi::fold_many0;
use cpclib_common::nom::sequence::{delimited, preceded};
use cpclib_common::nom::{Err, IResult, InputLength, InputTake};
use cpclib_common::nom_locate::LocatedSpan;
use cpclib_common::rayon::prelude::*;
use cpclib_tokens::ordered_float::OrderedFloat;
use cpclib_tokens::{
    BaseListing, BinaryFunction, BinaryOperation, CrunchType, Expr, ExprResult, LabelPrefix,
    ListingElement, MacroParam, MacroParamElement, TestKind, TestKindElement, ToSimpleToken, Token,
    UnaryFunction, UnaryOperation, UnaryTokenOperation
};
use ouroboros::self_referencing;

use super::{parse_z80_line, ParserContext, Z80Span};
use crate::assembler::Env;
use crate::error::AssemblerError;
/// ! This crate is related to the adaptation of tokens and listing for the case where they are parsed
use crate::error::ExpressionError;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::tokens::TokenExt;
use crate::preamble::{parse_end_directive, parse_z80_str};
use crate::{resolve_impl, BinaryTransformation, ExprElement, ParsingState, SymbolFor};
#[derive(Debug, Clone)]
pub enum LocatedExpr {
    RelativeDelta(i8, Z80Span),
    Value(i32, Z80Span),
    Float(OrderedFloat<f64>, Z80Span),
    Char(char, Z80Span),
    Bool(bool, Z80Span),

    String(Z80Span),
    Label(Z80Span),

    List(Vec<LocatedExpr>, Z80Span),

    PrefixedLabel(LabelPrefix, Z80Span, Z80Span),

    Paren(Box<LocatedExpr>, Z80Span),

    UnaryFunction(UnaryFunction, Box<LocatedExpr>, Z80Span),
    UnaryOperation(UnaryOperation, Box<LocatedExpr>, Z80Span),
    UnaryTokenOperation(UnaryTokenOperation, Box<LocatedToken>, Z80Span),
    BinaryFunction(BinaryFunction, Box<LocatedExpr>, Box<LocatedExpr>, Z80Span),
    BinaryOperation(BinaryOperation, Box<LocatedExpr>, Box<LocatedExpr>, Z80Span),

    /// Function supposely coded by the user
    AnyFunction(Z80Span, Vec<LocatedExpr>, Z80Span),

    /// Random value
    Rnd(Z80Span)
}

impl std::fmt::Display for LocatedExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.span())
    }
}

impl ExprElement for LocatedExpr {
    type ResultExpr = Expr;
    type Token = LocatedToken;

    fn is_negated(&self) -> bool {
        todo!()
    }

    fn is_relative(&self) -> bool {
        todo!()
    }

    fn relative_delta(&self) -> i8 {
        todo!()
    }

    fn is_value(&self) -> bool {
        todo!()
    }

    fn value(&self) -> i32 {
        todo!()
    }

    fn is_char(&self) -> bool {
        todo!()
    }

    fn char(&self) -> char {
        todo!()
    }

    fn is_bool(&self) -> bool {
        todo!()
    }

    fn bool(&self) -> bool {
        todo!()
    }

    fn is_string(&self) -> bool {
        todo!()
    }

    fn string(&self) -> &str {
        todo!()
    }

    fn is_float(&self) -> bool {
        todo!()
    }

    fn float(&self) -> OrderedFloat<f64> {
        todo!()
    }

    fn is_list(&self) -> bool {
        todo!()
    }

    fn list(&self) -> &[Self] {
        todo!()
    }

    fn is_label(&self) -> bool {
        todo!()
    }

    fn label(&self) -> &str {
        todo!()
    }

    fn is_token_operation(&self) -> bool {
        todo!()
    }

    fn token_operation(&self) -> &UnaryTokenOperation {
        todo!()
    }

    fn token(&self) -> &Self::Token {
        todo!()
    }

    fn is_prefix_label(&self) -> bool {
        todo!()
    }

    fn prefix(&self) -> &LabelPrefix {
        todo!()
    }

    fn is_binary_operation(&self) -> bool {
        todo!()
    }

    fn binary_operation(&self) -> BinaryOperation {
        todo!()
    }

    fn is_unary_operation(&self) -> bool {
        todo!()
    }

    fn unary_operation(&self) -> UnaryOperation {
        todo!()
    }

    fn is_unary_function(&self) -> bool {
        todo!()
    }

    fn unary_function(&self) -> UnaryFunction {
        todo!()
    }

    fn is_binary_function(&self) -> bool {
        todo!()
    }

    fn binary_function(&self) -> BinaryFunction {
        todo!()
    }

    fn is_paren(&self) -> bool {
        todo!()
    }

    fn is_rnd(&self) -> bool {
        todo!()
    }

    fn is_any_function(&self) -> bool {
        todo!()
    }

    fn function_name(&self) -> &str {
        todo!()
    }

    fn function_args(&self) -> &[Self] {
        todo!()
    }

    fn arg1(&self) -> &Self {
        todo!()
    }

    fn arg2(&self) -> &Self {
        todo!()
    }

    fn neg(&self) -> Self::ResultExpr {
        todo!()
    }

    fn not(&self) -> Self::ResultExpr {
        todo!()
    }

    fn add<E: Into<Self::ResultExpr>>(&self, v: E) -> Self::ResultExpr {
        todo!()
    }

    fn is_context_independant(&self) -> bool {
        todo!()
    }

    fn fix_relative_value(&mut self) {
        todo!()
    }
}

impl ExprEvaluationExt for LocatedExpr {
    /// Resolve by adding localisation in case of error
    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        resolve_impl!(self, env).map_err(|e| e.locate(self.span().clone()))
    }

    /// Be sure it is always synchronized with Expr
    fn symbols_used(&self) -> Vec<&str> {
        match self {
            LocatedExpr::RelativeDelta(..)
            | LocatedExpr::Value(..)
            | LocatedExpr::Float(..)
            | LocatedExpr::Char(..)
            | LocatedExpr::Bool(..)
            | LocatedExpr::String(..)
            | LocatedExpr::Rnd(_) => Vec::new(),

            LocatedExpr::Label(label) | LocatedExpr::PrefixedLabel(_, label, _) => {
                vec![label.as_str()]
            }

            LocatedExpr::BinaryFunction(_, box a, box b, _)
            | LocatedExpr::BinaryOperation(_, box a, box b, _) => {
                a.symbols_used()
                    .into_iter()
                    .chain(b.symbols_used().into_iter())
                    .collect_vec()
            }

            LocatedExpr::Paren(a, _)
            | LocatedExpr::UnaryFunction(_, a, _)
            | LocatedExpr::UnaryOperation(_, a, _) => a.symbols_used(),

            LocatedExpr::AnyFunction(_, l, _) | LocatedExpr::List(l, _) => {
                l.iter().map(|e| e.symbols_used()).flatten().collect_vec()
            }

            LocatedExpr::UnaryTokenOperation(_, box t, _) => {
                unimplemented!("Need to retreive the symbols from the operation")
            }
        }
    }
}

impl LocatedExpr {
    pub fn span(&self) -> &Z80Span {
        todo!()
    }

    /// Build a expr representation of the expression
    pub fn to_expr(&self) -> Expr {
        match self {
            LocatedExpr::RelativeDelta(d, _) => Expr::RelativeDelta(*d),
            LocatedExpr::Value(v, _) => Expr::Value(*v),
            LocatedExpr::Float(f, _) => Expr::Float(*f),
            LocatedExpr::Char(c, _) => Expr::Char(*c),
            LocatedExpr::Bool(b, _) => Expr::Bool(*b),
            LocatedExpr::String(s) => Expr::String(s.into()),
            LocatedExpr::Label(l) => Expr::Label(l.into()),
            LocatedExpr::List(l, _) => Expr::List(l.iter().map(|e| e.to_expr()).collect_vec()),
            LocatedExpr::PrefixedLabel(p, l, _) => Expr::PrefixedLabel(*p, l.into()),
            LocatedExpr::Paren(box p, _) => Expr::Paren(box p.to_expr()),
            LocatedExpr::UnaryFunction(f, box e, _) => Expr::UnaryFunction(*f, box e.to_expr()),
            LocatedExpr::UnaryOperation(o, box e, _) => Expr::UnaryOperation(*o, box e.to_expr()),
            LocatedExpr::UnaryTokenOperation(o, box t, _) => {
                Expr::UnaryTokenOperation(*o, box t.to_token().into_owned())
            }
            LocatedExpr::BinaryFunction(f, box e1, box e2, _) => {
                Expr::BinaryFunction(*f, box e1.to_expr(), box e2.to_expr())
            }
            LocatedExpr::BinaryOperation(o, box e1, box e2, _) => {
                Expr::BinaryOperation(*o, box e1.to_expr(), box e2.to_expr())
            }
            LocatedExpr::AnyFunction(n, a, _) => {
                Expr::AnyFunction(n.into(), a.iter().map(|e| e.to_expr()).collect_vec())
            }
            LocatedExpr::Rnd(_) => Expr::Rnd
        }
    }
}

#[derive(Debug, Clone)]
pub enum LocatedMacroParam {
    Empty,
    /// Standard argument
    Single(Z80Span),
    /// A list of argument that will be provided in a nested macro call
    List(Vec<Box<LocatedMacroParam>>)
}

impl MacroParamElement for LocatedMacroParam {
    fn empty() -> Self {
        Self::Empty
    }

    fn is_single(&self) -> bool {
        matches!(self, LocatedMacroParam::Single(..))
    }

    fn is_list(&self) -> bool {
        matches!(self, LocatedMacroParam::List(..))
    }

    fn single_argument(&self) -> &str {
        match self {
            LocatedMacroParam::Empty => "",
            LocatedMacroParam::Single(s) => s,
            LocatedMacroParam::List(_) => unreachable!()
        }
    }

    fn list_argument(&self) -> &[Box<Self>] {
        match self {
            LocatedMacroParam::List(l) => l,
            _ => unreachable!()
        }
    }
}

impl LocatedMacroParam {
    pub fn to_macro_param(&self) -> MacroParam {
        match self {
            LocatedMacroParam::Empty => MacroParam::Single("".to_string()),
            LocatedMacroParam::Single(text) => MacroParam::Single(text.fragment().to_string()),
            LocatedMacroParam::List(params) => {
                MacroParam::List(params.iter().map(|p| box p.to_macro_param()).collect_vec())
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            LocatedMacroParam::Empty => true,
            LocatedMacroParam::Single(text) => text.is_empty(),
            _ => false
        }
    }

    pub fn span(&self) -> Z80Span {
        match self {
            LocatedMacroParam::Single(span) => span.clone(),
            LocatedMacroParam::List(_) => todo!(),
            LocatedMacroParam::Empty => panic!()
        }
    }
}

#[derive(Debug, Clone)]
pub enum LocatedTestKind {
    // Test succeed if it is an expression that returns True
    True(LocatedExpr),
    // Test succeed if it is an expression that returns False
    False(LocatedExpr),
    // Test succeed if it is an existing label
    LabelExists(Z80Span),
    // Test succeed if it is a missing label
    LabelDoesNotExist(Z80Span),
    LabelUsed(Z80Span),
    LabelNused(Z80Span)
}

impl LocatedTestKind {
    pub fn to_test_kind(&self) -> TestKind {
        match self {
            LocatedTestKind::True(e) => TestKind::True(e.to_expr().clone()),
            LocatedTestKind::False(e) => TestKind::False(e.to_expr().clone()),
            LocatedTestKind::LabelExists(l) => TestKind::LabelExists(l.into()),
            LocatedTestKind::LabelDoesNotExist(l) => TestKind::LabelDoesNotExist(l.into()),
            LocatedTestKind::LabelUsed(l) => TestKind::LabelUsed(l.into()),
            LocatedTestKind::LabelNused(l) => TestKind::LabelNused(l.into())
        }
    }
}

impl TestKindElement for LocatedTestKind {
    type Expr = LocatedExpr;

    fn is_true_test(&self) -> bool {
        matches!(self, LocatedTestKind::True(_))
    }

    fn is_false_test(&self) -> bool {
        matches!(self, LocatedTestKind::False(_))
    }

    fn is_label_used_test(&self) -> bool {
        matches!(self, LocatedTestKind::LabelUsed(_))
    }

    fn is_label_nused_test(&self) -> bool {
        matches!(self, LocatedTestKind::LabelNused(_))
    }

    fn is_label_exists_test(&self) -> bool {
        matches!(self, LocatedTestKind::LabelExists(_))
    }

    fn is_label_nexists_test(&self) -> bool {
        matches!(self, LocatedTestKind::LabelDoesNotExist(_))
    }

    fn expr_unchecked(&self) -> &Self::Expr {
        match self {
            LocatedTestKind::True(exp) | LocatedTestKind::True(exp) => exp,
            _ => panic!()
        }
    }

    fn label_unchecked(&self) -> &str {
        match self {
            LocatedTestKind::LabelExists(l)
            | LocatedTestKind::LabelDoesNotExist(l)
            | LocatedTestKind::LabelUsed(l)
            | LocatedTestKind::LabelNused(l) => l.as_str(),
            _ => panic!()
        }
    }
}

#[derive(Debug)]
/// Add span information for a Token.
/// This hierarchy is a mirror of the original token one
pub enum LocatedToken {
    /// A token without any listing embedding
    Standard {
        /// The original token without any span information
        token: Token,
        /// The span that correspond to the token
        span: Z80Span
    },
    Defb(Vec<LocatedExpr>, Z80Span),
    Defw(Vec<LocatedExpr>, Z80Span),
    Str(Vec<LocatedExpr>, Z80Span),

    For {
        label: Z80Span,
        start: LocatedExpr,
        stop: LocatedExpr,
        step: Option<LocatedExpr>,
        listing: LocatedListing,
        span: Z80Span
    },
    Function(Z80Span, Vec<Z80Span>, LocatedListing, Z80Span),
    CrunchedSection(CrunchType, LocatedListing, Z80Span),
    Include(Z80Span, Option<Z80Span>, bool, Z80Span),
    Incbin {
        fname: Z80Span,
        offset: Option<LocatedExpr>,
        length: Option<LocatedExpr>,
        extended_offset: Option<LocatedExpr>,
        off: bool,
        transformation: BinaryTransformation,
        span: Z80Span
    },
    If(
        Vec<(LocatedTestKind, LocatedListing)>,
        Option<LocatedListing>,
        Z80Span
    ),
    Label(Z80Span),
    /// Name, Parameters, FullSpan
    MacroCall(Z80Span, Vec<LocatedMacroParam>, Z80Span),
    Repeat(
        LocatedExpr,
        LocatedListing,
        Option<Z80Span>,
        Option<LocatedExpr>,
        Z80Span
    ),
    Iterate(
        Z80Span,
        either::Either<Vec<LocatedExpr>, LocatedExpr>,
        LocatedListing,
        Z80Span
    ),
    RepeatUntil(LocatedExpr, LocatedListing, Z80Span),
    Rorg(LocatedExpr, LocatedListing, Z80Span),
    /// Name, Parameters, FullSpan
    Struct(Z80Span, Vec<(Z80Span, LocatedToken)>, Z80Span),
    Switch(
        LocatedExpr,
        Vec<(LocatedExpr, LocatedListing, bool)>,
        Option<LocatedListing>,
        Z80Span
    ),
    While(LocatedExpr, LocatedListing, Z80Span),
    Module(Z80Span, LocatedListing, Z80Span)
}

impl std::fmt::Display for LocatedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.span())
    }
}

impl ToSimpleToken for LocatedToken {
    fn as_simple_token(&self) -> Cow<Token> {
        self.to_token()
    }
}

/// Trait to handle the span of listing elements
pub trait MayHaveSpan: ListingElement {
    fn possible_span(&self) -> Option<&Z80Span>;
    fn span(&self) -> &Z80Span;
    fn has_span(&self) -> bool;
}

impl MayHaveSpan for Token {
    fn possible_span(&self) -> Option<&Z80Span> {
        None
    }

    fn span(&self) -> &Z80Span {
        panic!()
    }

    fn has_span(&self) -> bool {
        false
    }
}

impl MayHaveSpan for LocatedToken {
    fn has_span(&self) -> bool {
        true
    }

    fn possible_span(&self) -> Option<&Z80Span> {
        Some(self.span())
    }

    /// Get the span of the current token
    fn span(&self) -> &Z80Span {
        match self {
            LocatedToken::Standard { span, .. }
            | LocatedToken::CrunchedSection(_, _, span)
            | LocatedToken::For { span, .. }
            | LocatedToken::Function(_, _, _, span)
            | LocatedToken::If(_, _, span)
            | LocatedToken::Label(span)
            | LocatedToken::MacroCall(_, _, span)
            | LocatedToken::Module(_, _, span)
            | LocatedToken::Iterate(_, _, _, span)
            | LocatedToken::Repeat(_, _, _, _, span)
            | LocatedToken::RepeatUntil(_, _, span)
            | LocatedToken::Rorg(_, _, span)
            | LocatedToken::Struct(_, _, span)
            | LocatedToken::Switch(_, _, _, span)
            | LocatedToken::While(_, _, span)
            | LocatedToken::Defb(_, span)
            | LocatedToken::Defw(_, span)
            | LocatedToken::Str(_, span)
            | LocatedToken::Include(_, _, _, span) => span,
            LocatedToken::Incbin { span, .. } => span
        }
    }
}

impl Clone for LocatedToken {
    fn clone(&self) -> Self {
        unimplemented!();
        // match self {
        // LocatedToken::Standard { token, span } => {
        // LocatedToken::Standard {
        // token: token.clone(),
        // span: span.clone()
        // }
        // }
        // LocatedToken::CrunchedSection(a, b, c) => {
        // LocatedToken::CrunchedSection(a.clone(), b.clone(), c.clone())
        // }
        // LocatedToken::Function(a, b, c, d) => {
        // LocatedToken::Function(a.clone(), b.clone(), c.clone(), d.clone())
        // }
        // LocatedToken::If(a, b, c) => LocatedToken::If(a.clone(), b.clone(), c.clone()),
        // LocatedToken::Repeat(a, b, c, d, e) => {
        // LocatedToken::Repeat(a.clone(), b.clone(), c.clone(), d.clone(), e.clone())
        // }
        // LocatedToken::Iterate(a, b, c, d) => {
        // LocatedToken::Iterate(a.clone(), b.clone(), c.clone(), d.clone())
        // }
        // LocatedToken::RepeatUntil(..) => todo!(),
        // LocatedToken::Rorg(a, b, c) => LocatedToken::Rorg(a.clone(), b.clone(), c.clone()),
        // LocatedToken::Switch(value, cases, default, span) => {
        // LocatedToken::Switch(value.clone(), cases.clone(), default.clone(), span.clone())
        // }
        // LocatedToken::While(a, b, c) => LocatedToken::While(a.clone(), b.clone(), c.clone()),
        // LocatedToken::Module(..) => todo!(),
        // LocatedToken::For {
        // label,
        // start,
        // stop,
        // step,
        // listing,
        // span
        // } => {
        // LocatedToken::For {
        // label: label.clone(),
        // start: start.clone(),
        // stop: stop.clone(),
        // step: step.clone(),
        // span: span.clone(),
        // listing: listing.clone()
        // }
        // }
        // }
    }
}

impl Deref for LocatedToken {
    type Target = Token;

    fn deref(&self) -> &Self::Target {
        match self.token() {
            Ok(t) => t,
            Err(_) => {
                panic!("{:?} cannot be dereferenced as it contains a listing", self)
            }
        }
    }
}

impl LocatedToken {
    /// We can obtain a token only for "standard ones". Those that rely on listing need to be handled differently
    pub fn token(&self) -> Result<&Token, ()> {
        match self {
            Self::Standard { token, .. } => Ok(token),
            _ => Err(())
        }
    }

    pub fn context(&self) -> &ParserContext {
        &self.span().extra
    }
}

impl LocatedToken {
    /// Transform the located token in a raw token.
    /// Warning, this is quite costly when strings or vec are involved
    pub fn to_token(&self) -> Cow<Token> {
        unimplemented!();
        match self {
            LocatedToken::Standard { token, .. } => Cow::Borrowed(token),
            LocatedToken::CrunchedSection(c, l, _span) => {
                Cow::Owned(Token::CrunchedSection(*c, l.as_listing()))
            }
            LocatedToken::Function(name, params, inner, _span) => {
                Cow::Owned(Token::Function(
                    name.into(),
                    params.iter().map(|p| p.into()).collect_vec(),
                    inner.as_listing()
                ))
            }
            LocatedToken::If(v, e, _span) => {
                Cow::Owned(Token::If(
                    v.iter()
                        .map(|(k, l)| (k.to_test_kind(), l.as_listing()))
                        .collect_vec(),
                    e.as_ref().map(|l| l.as_listing())
                ))
            }
            LocatedToken::Repeat(e, l, s, start, _span) => {
                Cow::Owned(Token::Repeat(
                    e.to_expr(),
                    l.as_listing(),
                    s.as_ref().map(|s| s.into()),
                    start.map(|e| e.to_expr())
                ))
            }
            LocatedToken::RepeatUntil(e, l, _span) => {
                Cow::Owned(Token::RepeatUntil(e.to_expr(), l.as_listing()))
            }
            LocatedToken::Rorg(e, l, _span) => Cow::Owned(Token::Rorg(e.to_expr(), l.as_listing())),
            LocatedToken::Switch(v, c, d, _span) => {
                Cow::Owned(Token::Switch(
                    v.to_expr(),
                    c.iter()
                        .map(|(e, l, b)| (e.to_expr(), l.as_listing(), b.clone()))
                        .collect_vec(),
                    d.as_ref().map(|d| d.as_listing())
                ))
            }
            LocatedToken::While(e, l, _span) => {
                Cow::Owned(Token::While(e.to_expr(), l.as_listing()))
            }
            LocatedToken::Iterate(name, values, code, span) => {
                todo!()
            }
            LocatedToken::Module(..) => todo!(),
            LocatedToken::For {
                label,
                start,
                stop,
                step,
                listing,
                span
            } => {
                Cow::Owned(Token::For {
                    label: label.into(),
                    start: start.to_expr(),
                    stop: stop.to_expr(),
                    step: step.map(|e| e.to_expr()),
                    listing: listing.as_listing()
                })
            }
            LocatedToken::Label(label) => Cow::Owned(Token::Label(label.into())),
            LocatedToken::MacroCall(name, params, _) => {
                Cow::Owned(Token::MacroCall(
                    name.into(),
                    params.iter().map(|p| p.to_macro_param()).collect_vec()
                ))
            }
            LocatedToken::Struct(name, params, _) => {
                Cow::Owned(Token::Struct(
                    name.into(),
                    params
                        .iter()
                        .map(|(label, p)| (label.into(), p.as_simple_token().into_owned()))
                        .collect_vec()
                ))
            }
            LocatedToken::Defb(..) => todo!(),
            LocatedToken::Defw(..) => todo!(),
            LocatedToken::Str(..) => todo!(),
            LocatedToken::Include(..) => todo!(),
            LocatedToken::Incbin {
                fname,
                offset,
                length,
                extended_offset,
                off,
                transformation,
                span
            } => todo!()
        }
    }

    pub fn parse_token(value: &str) -> Result<LocatedToken, String> {
        let tokens = {
            let res = parse_z80_str(value);
            match res {
                Ok(tokens) => tokens,
                Err(_e) => {
                    return Err("ERROR -- need to code why ...".to_owned());
                }
            }
        };
        match tokens.len() {
            0 => Err("No ASM found.".to_owned()),
            1 => {
                let token = tokens[0].clone();
                Ok(token)
            }
            _ => {
                Err(format!(
                    "{} tokens are present instead of one",
                    tokens.len()
                ))
            }
        }
    }

    // fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
    // match self {
    // LocatedToken::Standard { token, span } => {
    // token.fix_local_macro_labels_with_seed(seed)
    // },
    // LocatedToken::CrunchedSection(_, _, _) => todo!(),
    // LocatedToken::Include(_, _, _) => todo!(),
    //
    // Self::If(v, o, _) => {
    // v.iter_mut()
    // .map(|(t, l)| l)
    // .for_each(|l| l.fix_local_macro_labels_with_seed(seed));
    // o.as_mut().map(|l| l.fix_local_macro_labels_with_seed(seed));
    // }
    //
    // Self::Switch(l, _) => {
    // l.iter_mut().for_each(|(e, l)| {
    // e.fix_local_macro_labels_with_seed(seed);
    // l.fix_local_macro_labels_with_seed(seed);
    // });
    // }
    //
    //
    // Self::RepeatUntil(e, l, _)
    // | Self::Rorg(e, l, _)
    // | Self::While(e, l, _) => {
    // e.fix_local_macro_labels_with_seed(seed);
    // l.fix_local_macro_labels_with_seed(seed);
    // }
    //
    // Self::Repeat(e, l, _, s, _) => {
    //
    // e.fix_local_macro_labels_with_seed(seed);
    // l.fix_local_macro_labels_with_seed(seed);
    // s.as_mut().map(|s| s.fix_local_macro_labels_with_seed(seed));
    // }
    // }
    // }
}
/// Implement this trait for type previousy defined without source location.

pub trait Locate {
    type Output;

    fn locate(self, span: Z80Span) -> Self::Output;
}

impl Locate for Token {
    type Output = LocatedToken;

    fn locate(self, span: Z80Span) -> LocatedToken {
        if self.has_at_least_one_listing() {
            // /
            // match self {
            // Token::CrunchedSection(a, b) => {
            // LocatedToken::CrunchedSection(a, b, span)
            // },
            // Token::Include(a,b) => {
            // LocatedToken::Include(a, b, span)
            // },
            // Token::If(a, b) => {
            // LocatedToken::If(a, b, span)
            // },
            // Token::Repeat(a,b, c,) => {
            // LocatedToken::Repeat(a,b,c,span)
            // },
            // Token::RepeatUntil(a, b) => {
            // LocatedToken::RepeatUntil(a, b, span)
            // },
            // Token::Rorg(a, b) => {
            // LocatedToken::Rorg(a, b, span)
            // },
            // Token::Switch(a) => {
            // LocatedToken::Switch(a, span)
            // },
            // Token::While(a, b) => {
            // LocatedToken::While(a, b, span)
            // },
            // _ => unreachable!()
            //
            // }
            unreachable!()
        }
        else {
            LocatedToken::Standard { token: self, span }
        }
    }
}

impl ListingElement for LocatedToken {
    type Expr = LocatedExpr;
    type MacroParam = LocatedMacroParam;
    type TestKind = LocatedTestKind;

    fn macro_call_name(&self) -> &str {
        match self {
            Self::MacroCall(name, ..) => name.as_str(),
            _ => panic!()
        }
    }

    fn macro_call_arguments(&self) -> &[Self::MacroParam] {
        match self {
            Self::MacroCall(_, args, _) => args,
            _ => panic!()
        }
    }

    fn is_if(&self) -> bool {
        match self {
            Self::If(..) => true,
            _ => false
        }
    }

    fn if_nb_tests(&self) -> usize {
        match self {
            Self::If(tests, ..) => tests.len(),
            _ => panic!()
        }
    }

    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]) {
        match self {
            Self::If(tests, ..) => {
                let data = &tests[idx];
                (&data.0, &data.1)
            }
            _ => panic!()
        }
    }

    fn if_else(&self) -> Option<&[Self]> {
        match self {
            Self::If(_, r#else, _) => r#else.as_ref().map(|l| l.as_slice()),
            _ => panic!()
        }
    }

    fn is_include(&self) -> bool {
        match self {
            Self::Standard {
                token: Token::Include(..),
                ..
            } => true,
            _ => false
        }
    }

    fn is_incbin(&self) -> bool {
        match self {
            Self::Incbin { .. } => true,
            _ => false
        }
    }

    fn incbin_fname(&self) -> &str {
        match self {
            Self::Incbin { fname, .. } => fname,
            _ => unimplemented!()
        }
    }

    fn incbin_offset(&self) -> Option<&Self::Expr> {
        match self {
            Self::Incbin { offset, .. } => offset.as_ref(),
            _ => unimplemented!()
        }
    }

    fn incbin_length(&self) -> Option<&Self::Expr> {
        match self {
            Self::Incbin { length, .. } => length.as_ref(),
            _ => unimplemented!()
        }
    }

    fn incbin_transformation(&self) -> &cpclib_tokens::BinaryTransformation {
        match self {
            Self::Incbin { transformation, .. } => transformation,
            _ => unimplemented!()
        }
    }

    fn include_fname(&self) -> &str {
        match self {
            Self::Include(fname, ..) => fname,
            _ => unreachable!()
        }
    }
}

pub type InnerLocatedListing = BaseListing<LocatedToken>;

/// Represents a Listing of located tokens
/// Lifetimes 'src and 'ctx are in fact the same and correspond to hte lifetime of the object itself
#[self_referencing]
pub struct LocatedListing {
    /// Its source code. We want it to live as long as possible.
    /// A string is copied for the very beginning of the file parsing, while a span is used for the inner blocs. As this field is immutable and build before the listing, we do not store the span here
    src: Option<String>,

    /// Its Parsing Context whose source targets LocatedListing
    #[borrows(src)]
    ctx: ParserContext,

    /// The real listing whose tokens come from src
    #[borrows(src, ctx)]
    pub(crate) parse_result: ParseResult
}

impl std::fmt::Debug for LocatedListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with_parse_result(|p| {
            f.debug_struct("LocatedListing")
                .field("parse_result", p)
                .finish()
        })
    }
}

#[derive(Debug)]
pub(crate) enum ParseResult {
    /// Success for a complete file
    SuccessComplete(InnerLocatedListing),
    /// Success for an inner block
    SuccessInner {
        /// The real listing of LocatedTokens
        listing: InnerLocatedListing,
        /// The code of the inner block
        inner_span: Z80Span,
        /// The code of the next span
        next_span: Z80Span
    },
    FailureInner(Err<VerboseError<Z80Span>>),
    FailureComplete(AssemblerError)
}

#[derive(Debug)]
pub(crate) enum ParseResultFirstStage {
    Sucess {
        listing: Option<InnerLocatedListing>,
        remaining_span: Option<Z80Span>
    },
    Failure(VerboseError<Z80Span>)
}

impl LocatedListing {
    /// Build the listing from the current code and context
    /// In case of error, the listing is provided as error message refer to its owned source code... FInal version should behave differently
    /// The listing embeds the error
    pub fn new_complete_source(
        code: String,
        mut ctx: ParserContext
    ) -> Result<LocatedListing, LocatedListing> {
        // generate the listing
        let listing = LocatedListingBuilder {
            // source code is a string owned by the listing
            src: Some(code),

            // context keeps a reference on the full listing (but is it really needed yet ?)
            ctx_builder: |src| {
                ctx.source = src
                    .as_ref()
                    .map(|s| s.as_str())
                    .map(|s| unsafe { &*(s as *const str) as &'static str });
                ctx
            },

            // tokens depend both on the source and context. However source can be obtained from context so we do not use it here (it is usefull for the inner case)
            parse_result_builder: |_, ctx| {
                let src = ctx.source.as_ref().unwrap();
                let input_start = Z80Span::new_extra(src, ctx);

                // really make the parsing
                let res = fold_many0(
                    parse_z80_line,
                    || Vec::new(),
                    |mut source_tokens, mut line_tokens| {
                        source_tokens.append(&mut line_tokens);
                        source_tokens
                    }
                )(input_start.clone());

                // analyse result and can generate error even if parsing was ok
                let res = match res {
                    Ok((input_stop, tokens)) => {
                        if input_stop.trim().is_empty() {
                            // no more things to assemble
                            Ok(InnerLocatedListing::from(tokens))
                        }
                        else {
                            // Everything should have been consumed
                            std::result::Result::Err(Err::Error(
                                cpclib_common::nom::error::ParseError::<Z80Span>::from_error_kind(
                                    input_start,
                                    ErrorKind::Many0
                                )
                            ))
                        }
                    }
                    Err(e) => {
                        // Propagate the error (that is located)
                        std::result::Result::Err(e)
                    }
                };

                // Build the result
                let res = match res {
                    Ok(listing) => ParseResult::SuccessComplete(listing),
                    Err(e) => {
                        match e {
                            cpclib_common::nom::Err::Error(e) | Err::Failure(e) => {
                                ParseResult::FailureComplete(AssemblerError::SyntaxError {
                                    error: e
                                })
                            }
                            cpclib_common::nom::Err::Incomplete(_) => {
                                ParseResult::FailureComplete(AssemblerError::BugInParser {
                                    error: "Bug in the parser".to_owned(),
                                    context: ctx.deref().clone()
                                })
                            }
                        }
                    }
                };

                return res;
            }
        }
        .build();

        match listing.borrow_parse_result() {
            ParseResult::SuccessComplete(_) => Ok(listing),
            ParseResult::FailureComplete(_) => Err(listing),
            _ => unreachable!()
        }
    }

    /// By definition code is store in a Z80Span because the original string is Already contained in another Listing as a String
    /// As the code is already owned by another LocatedListing, we can return error messages that refer it
    pub fn parse_inner(
        input_code: Z80Span,
        new_state: ParsingState
    ) -> IResult<Z80Span, LocatedListing, VerboseError<Z80Span>> {
        // The context is similar to the initial one ...
        let mut ctx = input_code.extra.clone();
        // ... but the state can be modified to forbid some keywords
        ctx.state = new_state;

        // we do not change ctx.source that must be the very same than the parent
        //       let input_fragment = input_code.fragment();
        //       ctx.source = Some(input_fragment);

        let inner_listing = LocatedListingBuilder {
            // No need to specify an input as it is already embedded in the parent listing
            src: None,

            // Context source has already been provided before. Its state as also been properly set
            ctx_builder: move |_src| ctx,

            parse_result_builder: |_, ctx| {
                // build a span with the appropriate novel context
                let ctx = unsafe { &*(ctx as *const ParserContext) as &'static ParserContext }; // the context is store within the object; so it is safe to set its lifetime to static

                // Build the span that will be parsed to collect inner tokens.
                // It has a length of input_length.
                let mut inner_code = Z80Span(unsafe {
                    LocatedSpan::new_from_raw_offset(
                        input_code.location_offset(),
                        input_code.location_line(),
                        &*(input_code.as_str() as *const str) as &'static str,
                        ctx
                    )
                });
                // keep a track of the very beginning of the span
                let inner_start: Z80Span = inner_code.clone();

                let mut tokens = Vec::new(); // container of the parsed tokens
                let mut error = None; // container of the potential parse error

                // we parse until we met an error or the end of the parse
                loop {
                    // check if the line needs to be parsed (ie there is no end directive)
                    let must_break = inner_code.trim().is_empty() || {
                        // TODO take into account potential label
                        let maybe_keyword = opt(preceded(
                            delimited(space0, opt(tag(":")), space0),
                            parse_end_directive
                        ))(inner_code.clone());
                        match maybe_keyword {
                            Ok((_, Some(_))) => true,
                            _ => false
                        }
                    };
                    if must_break {
                        break;
                    };

                    // really parse the line
                    match cut(context("[DBG] Inner loop", parse_z80_line))(inner_code.clone()) {
                        Ok((next_input, mut tok)) => {
                            inner_code = next_input; // ensure next line parsing starts at the right place{}
                            tokens.append(&mut tok); // add the collected tokens to the complete tokens list
                        }
                        Err(e) => {
                            error = Some(e);
                            break;
                        }
                    }
                }

                // here we may have left because of an error or the end of parsing.
                // Generate the appropriate parse result
                match error {
                    // Parse error
                    Some(e) => ParseResult::FailureInner(e),
                    // Correct parsing
                    None => {
                        // restore the appropriate context to the next_span (the original context in fact)
                        let mut next_span = inner_code;
                        next_span.extra = input_code.extra;

                        // shorten the inner_code
                        let inner_span =
                            inner_start.take(inner_start.input_len() - next_span.input_len());

                        ParseResult::SuccessInner {
                            inner_span,
                            next_span,
                            listing: InnerLocatedListing::from(tokens)
                        }
                    }
                }
            }
        }
        .build();

        match inner_listing.borrow_parse_result() {
            ParseResult::SuccessInner { next_span, .. } => Ok((next_span.clone(), inner_listing)),
            ParseResult::FailureInner(e) => Err(e.clone()),
            _ => unreachable!()
        }
    }
}

impl LocatedListing {
    /// Make sense only when the listing as been properly parsed. May crash otherwhise
    pub fn src(&self) -> &str {
        self.with_src(|src| src.as_ref().map(|s| s.as_str()))
            .unwrap_or_else(|| {
                self.with_parse_result(|parse_result| {
                    match parse_result {
                        ParseResult::SuccessInner { inner_span, .. } => inner_span.as_str(),
                        _ => unreachable!()
                    }
                })
            })
    }

    /// Lie a bit for inner listing as the provided source is too long
    pub fn ctx(&self) -> &ParserContext {
        self.with_ctx(|ctx| ctx)
    }

    /// Return the span of the listing
    pub fn span(&self) -> Z80Span {
        self.with_parse_result(|parse_result| {
            match parse_result {
                ParseResult::SuccessComplete(_) => {
                    let src = self.src();
                    let ctx = self.ctx();
                    Z80Span::new_extra(src, ctx)
                }
                ParseResult::SuccessInner { inner_span, .. } => inner_span.clone(),
                _ => panic!("No listing available")
            }
        })
    }

    pub fn nom_error_unchecked(&self) -> &Err<VerboseError<Z80Span>> {
        self.with_parse_result(|parse_result| {
            match parse_result {
                ParseResult::FailureInner(e) => e,
                _ => unreachable!()
            }
        })
    }

    pub fn cpclib_error_unchecked(&self) -> &AssemblerError {
        self.with_parse_result(|parse_result| {
            match parse_result {
                ParseResult::FailureComplete(e) => e,
                _ => unreachable!()
            }
        })
    }

    pub fn parse_ok(&self) -> bool {
        self.with_parse_result(|parse_result| {
            match parse_result {
                ParseResult::SuccessComplete(_) | ParseResult::SuccessInner { .. } => true,
                ParseResult::FailureInner(_) | ParseResult::FailureComplete(_) => false
            }
        })
    }

    // pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
    // self.iter_mut()
    // .for_each(|e| e.fix_local_macro_labels_with_seed(seed));
    // }
}

impl Deref for LocatedListing {
    type Target = InnerLocatedListing;

    fn deref(&self) -> &Self::Target {
        self.with_parse_result(|parse_result| {
            match parse_result {
                ParseResult::SuccessComplete(listing) => listing,
                ParseResult::SuccessInner { listing, .. } => listing,
                _ => panic!("No listing available.")
            }
        })
    }
}

// No more possible as the listing MUST be created BEFORE the tokens
// impl TryFrom<Vec<LocatedToken>> for LocatedListing {
// type Error = ();
//
// Conversion fails only when the vec is empty.
// In this case a workaround has to be used
// TODO shorten the listing the src does not seems appropriate at all
// fn try_from(tokens: Vec<LocatedToken>) -> Result<Self, Self::Error> {
// match tokens.first() {
// Some(token) => {
// let extra = &token.span().extra;
// let src = Arc::clone(&extra.0);
// let ctx = Arc::clone(&extra.1);
// Ok(LocatedListing {
// listing: tokens.into(),
// ctx,
// src
// })
// }
// None => Err(())
// }
// }
// }

impl LocatedListing {
    // pub fn as_cowed_listing(&self) -> BaseListing<Cow<Token>> {
    // self.deref()
    // .par_iter()
    // .map(|lt| lt.to_token())
    // .collect::<Vec<_>>()
    // .into()
    // }

    pub fn as_listing(&self) -> BaseListing<Token> {
        self.deref()
            .par_iter()
            .map(|lt| lt.to_token())
            .map(|c| -> Token { c.into_owned() })
            .collect::<Vec<Token>>()
            .into()
    }
}

pub trait ParseToken {
    type Output: ListingElement;
    fn parse_token(src: &str) -> Result<Self::Output, String>;
}

impl ParseToken for Token {
    type Output = Token;

    fn parse_token(src: &str) -> Result<Self::Output, String> {
        let token = LocatedToken::parse_token(src);
        token.map(|lt| lt.to_token().into_owned())
    }
}

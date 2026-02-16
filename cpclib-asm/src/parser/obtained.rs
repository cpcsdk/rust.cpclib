// SAFETY: All fields of LocatedExpr are Sync (primitives, Box, Vec, OrderedFloat, UnescapedString, Z80Span, etc.)
unsafe impl Sync for LocatedExpr {}

// SAFETY: All fields of UnescapedString are Sync (String, Z80Span)
unsafe impl Sync for UnescapedString {}
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;
use std::sync::Arc;

use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::prelude::*;
use cpclib_common::smallvec::SmallVec;
use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::combinator::cut_err;
use cpclib_common::winnow::error::ErrMode;
use cpclib_common::winnow::stream::{AsBStr, AsBytes, Offset, Stream, UpdateSlice};
use cpclib_common::winnow::token::take;
use cpclib_common::winnow::{BStr, ModalResult, Parser};
use cpclib_sna::{
    FlagValue, RemuBreakPointAccessMode, RemuBreakPointRunMode, RemuBreakPointType, SnapshotFlag,
    SnapshotVersion
};
use cpclib_tokens::macro_segment::TokenizedMacroContent;
use cpclib_tokens::ordered_float::OrderedFloat;
use cpclib_tokens::{
    AssemblerControlCommand, AssemblerFlavor, BaseListing, BinaryOperation, CharsetFormat,
    CrunchType, DataAccess, DataAccessElem, Expr, ExprResult, FlagTest, FormattedExpr,
    IndexRegister8, IndexRegister16, LabelPrefix, ListingElement, MacroParam, MacroParamElement,
    Mnemonic, Register8, Register16, SaveType, StableTickerAction, TestKind, TestKindElement,
    ToSimpleToken, Token, UnaryOperation, UnaryTokenOperation, data_access_impl_most_methods,
    data_access_is_any_indexregister8, data_access_is_any_indexregister16,
    data_access_is_any_register8, data_access_is_any_register16, listing_element_impl_most_methods
};
use ouroboros::self_referencing;

use super::{
    InnerZ80Span, ParserContext, SourceString, Z80ParserError, Z80Span, build_span,
    my_many0_nocollect, parse_lines, parse_single_token, parse_z80_line_complete
};
use crate::assembler::Env;
use crate::error::AssemblerError;
/// ! This crate is related to the adaptation of tokens and listing for the case where they are parsed
use crate::error::ExpressionError;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::listing::ListingExt;
use crate::implementation::tokens::TokenExt;
use crate::preamble::parse_z80_str;
use crate::{
    BinaryTransformation, ExprElement, ParserContextBuilder, ParsingState, SymbolFor,
    ensure_orgams_type, resolve_impl
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocatedExpr {
    RelativeDelta(i8, Z80Span),
    Value(i32, Z80Span),
    Float(OrderedFloat<f64>, Z80Span),
    Char(char, Z80Span),
    Bool(bool, Z80Span),

    String(UnescapedString),
    Label(Z80Span),

    List(Vec<LocatedExpr>, Z80Span),

    PrefixedLabel(LabelPrefix, Z80Span, Z80Span),

    Paren(Box<LocatedExpr>, Z80Span),

    UnaryOperation(UnaryOperation, Box<LocatedExpr>, Z80Span),
    UnaryTokenOperation(UnaryTokenOperation, Box<LocatedToken>, Z80Span),
    BinaryOperation(BinaryOperation, Box<LocatedExpr>, Box<LocatedExpr>, Z80Span),

    /// Ternary conditional: condition ? true_value : false_value
    Ternary(
        Box<LocatedExpr>,
        Box<LocatedExpr>,
        Box<LocatedExpr>,
        Z80Span
    ),

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnescapedString(pub(crate) String, pub(crate) Z80Span);

impl AsRef<str> for UnescapedString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for UnescapedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl SourceString for &UnescapedString {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl ExprElement for LocatedExpr {
    type Expr = LocatedExpr;
    type ResultExpr = Expr;
    type Token = LocatedToken;

    fn to_expr(&self) -> Cow<'_, Expr> {
        let expr = match self {
            LocatedExpr::RelativeDelta(d, _) => Expr::RelativeDelta(*d),
            LocatedExpr::Value(v, _) => Expr::Value(*v),
            LocatedExpr::Float(f, _) => Expr::Float(*f),
            LocatedExpr::Char(c, _) => Expr::Char(*c),
            LocatedExpr::Bool(b, _) => Expr::Bool(*b),
            LocatedExpr::String(s) => Expr::String(s.as_ref().into()),
            LocatedExpr::Label(l) => Expr::Label(l.into()),
            LocatedExpr::List(l, _) => {
                Expr::List(l.iter().map(|e| e.to_expr().into_owned()).collect_vec())
            },
            LocatedExpr::PrefixedLabel(p, l, _) => Expr::PrefixedLabel(*p, l.into()),
            LocatedExpr::Paren(box p, _) => Expr::Paren(Box::new(p.to_expr().into_owned())),
            LocatedExpr::UnaryOperation(o, box e, _) => {
                Expr::UnaryOperation(*o, Box::new(e.to_expr().into_owned()))
            },
            LocatedExpr::UnaryTokenOperation(o, box t, _) => {
                Expr::UnaryTokenOperation(*o, Box::new(t.to_token().into_owned()))
            },

            LocatedExpr::BinaryOperation(o, box e1, box e2, _) => {
                Expr::BinaryOperation(
                    *o,
                    Box::new(e1.to_expr().into_owned()),
                    Box::new(e2.to_expr().into_owned())
                )
            },
            LocatedExpr::Ternary(box cond, box true_expr, box false_expr, _) => {
                Expr::Ternary(
                    Box::new(cond.to_expr().into_owned()),
                    Box::new(true_expr.to_expr().into_owned()),
                    Box::new(false_expr.to_expr().into_owned())
                )
            },
            LocatedExpr::AnyFunction(n, a, _) => {
                Expr::AnyFunction(
                    n.into(),
                    a.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                )
            },
            LocatedExpr::Rnd(_) => Expr::Rnd
        };

        Cow::Owned(expr)
    }

    fn is_negated(&self) -> bool {
        match self {
            Self::UnaryOperation(UnaryOperation::Neg, ..) => true,
            _ => false
        }
    }

    fn is_relative(&self) -> bool {
        match self {
            Self::RelativeDelta(..) => true,
            _ => false
        }
    }

    fn relative_delta(&self) -> i8 {
        match self {
            Self::RelativeDelta(val, _) => *val,
            _ => unreachable!()
        }
    }

    fn neg(&self) -> Self::ResultExpr {
        Expr::UnaryOperation(UnaryOperation::Neg, Box::new(self.to_expr().into_owned()))
    }

    fn add<E: Into<Expr>>(&self, v: E) -> Self::ResultExpr {
        Self::ResultExpr::BinaryOperation(
            BinaryOperation::Add,
            Box::new(self.to_expr().into_owned()),
            v.into().into()
        )
    }

    /// Check if it is necessary to read within a symbol table
    fn is_context_independant(&self) -> bool {
        match self {
            Self::Label(..) => false,
            _ => true
        }
    }

    /// When disassembling an instruction with relative expressions, the contained value needs to be transformed as an absolute value
    fn fix_relative_value(&mut self) {
        if let Self::Value(val, span) = self {
            let mut new_expr = Self::RelativeDelta(*val as i8, span.clone());
            std::mem::swap(self, &mut new_expr);
        }
    }

    fn not(&self) -> Self::ResultExpr {
        todo!()
    }

    fn is_value(&self) -> bool {
        match self {
            Self::Value(..) => true,
            _ => false
        }
    }

    fn value(&self) -> i32 {
        match self {
            Self::Value(v, _) => *v,
            _ => unreachable!()
        }
    }

    fn is_char(&self) -> bool {
        match self {
            Self::Char(..) => true,
            _ => false
        }
    }

    fn char(&self) -> char {
        match self {
            Self::Char(v, _) => *v,
            _ => unreachable!()
        }
    }

    fn is_bool(&self) -> bool {
        match self {
            Self::Bool(..) => true,
            _ => false
        }
    }

    fn bool(&self) -> bool {
        match self {
            Self::Bool(v, _) => *v,
            _ => unreachable!()
        }
    }

    fn is_string(&self) -> bool {
        match self {
            Self::String(..) => true,
            _ => false
        }
    }

    fn string(&self) -> &str {
        match self {
            Self::String(v) => &v.0,
            _ => unreachable!()
        }
    }

    fn is_float(&self) -> bool {
        match self {
            Self::Float(..) => true,
            _ => false
        }
    }

    fn float(&self) -> OrderedFloat<f64> {
        match self {
            Self::Float(v, _) => *v,
            _ => unreachable!()
        }
    }

    fn is_list(&self) -> bool {
        match self {
            Self::List(..) => true,
            _ => false
        }
    }

    fn list(&self) -> &[Self] {
        match self {
            Self::List(v, _) => v.as_slice(),
            _ => unreachable!()
        }
    }

    fn is_label(&self) -> bool {
        match self {
            Self::Label(..) => true,
            _ => false
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Label(v) => v.as_str(),
            Self::PrefixedLabel(_, v, _) => v.as_str(),
            _ => unreachable!()
        }
    }

    fn is_token_operation(&self) -> bool {
        match self {
            Self::UnaryTokenOperation(..) => true,
            _ => false
        }
    }

    fn token_operation(&self) -> &UnaryTokenOperation {
        match self {
            Self::UnaryTokenOperation(op, __, _) => op,
            _ => unreachable!()
        }
    }

    fn token(&self) -> &Self::Token {
        match self {
            Self::UnaryTokenOperation(_, box token, _) => token,
            _ => unreachable!()
        }
    }

    fn is_prefix_label(&self) -> bool {
        match self {
            Self::PrefixedLabel(..) => true,
            _ => false
        }
    }

    fn prefix(&self) -> &LabelPrefix {
        match self {
            Self::PrefixedLabel(prefix, ..) => prefix,
            _ => unreachable!()
        }
    }

    fn is_binary_operation(&self) -> bool {
        match self {
            Self::BinaryOperation(..) => true,
            _ => false
        }
    }

    fn binary_operation(&self) -> BinaryOperation {
        match self {
            Self::BinaryOperation(op, ..) => *op,
            _ => unreachable!()
        }
    }

    fn is_ternary(&self) -> bool {
        matches!(self, Self::Ternary(..))
    }

    fn ternary_condition(&self) -> &Self::Expr {
        match self {
            Self::Ternary(cond, ..) => cond.as_ref(),
            _ => unreachable!()
        }
    }

    fn ternary_true(&self) -> &Self::Expr {
        match self {
            Self::Ternary(_, true_expr, ..) => true_expr.as_ref(),
            _ => unreachable!()
        }
    }

    fn ternary_false(&self) -> &Self::Expr {
        match self {
            Self::Ternary(_, _, false_expr, _) => false_expr.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_unary_operation(&self) -> bool {
        match self {
            Self::UnaryOperation(..) => true,
            _ => false
        }
    }

    fn unary_operation(&self) -> UnaryOperation {
        match self {
            Self::UnaryOperation(op, ..) => *op,
            _ => unreachable!()
        }
    }

    fn is_paren(&self) -> bool {
        match self {
            Self::Paren(..) => true,
            _ => false
        }
    }

    fn is_rnd(&self) -> bool {
        match self {
            Self::Rnd(_) => true,
            _ => false
        }
    }

    fn is_any_function(&self) -> bool {
        match self {
            Self::AnyFunction(..) => true,
            _ => false
        }
    }

    fn function_name(&self) -> &str {
        match self {
            Self::AnyFunction(n, ..) => n.as_str(),
            _ => unreachable!()
        }
    }

    fn function_args(&self) -> &[Self] {
        match self {
            Self::AnyFunction(_, args, _) => args.as_slice(),
            _ => unreachable!()
        }
    }

    fn arg1(&self) -> &Self {
        match self {
            Self::BinaryOperation(_, box arg1, ..) => arg1,
            Self::UnaryOperation(_, box arg, _) => arg,
            Self::Paren(box p, _) => p,

            _ => unreachable!()
        }
    }

    fn arg2(&self) -> &Self {
        match self {
            Self::BinaryOperation(_, _, box arg2, _) => arg2,
            _ => unreachable!()
        }
    }

    fn symbols(&self) -> std::collections::HashSet<String> {
        // Delegate to the Expr implementation after converting
        self.to_expr().symbols()
    }
}

impl ExprEvaluationExt for LocatedExpr {
    /// Resolve by adding localisation in case of error
    fn resolve(&self, env: &mut Env) -> Result<ExprResult, Box<AssemblerError>> {
        let res = resolve_impl!(self, env).map_err(|e| e.locate(self.span().clone()))?;
        ensure_orgams_type(res, env)
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
            },

            LocatedExpr::BinaryOperation(_, box a, box b, _) => {
                a.symbols_used()
                    .into_iter()
                    .chain(b.symbols_used())
                    .collect_vec()
            },

            LocatedExpr::Paren(a, _) | LocatedExpr::UnaryOperation(_, a, _) => a.symbols_used(),

            LocatedExpr::AnyFunction(_, l, _) | LocatedExpr::List(l, _) => {
                l.iter().flat_map(|e| e.symbols_used()).collect_vec()
            },

            LocatedExpr::UnaryTokenOperation(_, box _t, _) => {
                eprintln!("symbols_used is not implemented for UnaryTokenOperation");
                vec![]
            },

            LocatedExpr::Ternary(box cond, box true_expr, box false_expr, _) => {
                cond.symbols_used()
                    .into_iter()
                    .chain(true_expr.symbols_used())
                    .chain(false_expr.symbols_used())
                    .collect_vec()
            },
        }
    }
}

impl MayHaveSpan for LocatedExpr {
    fn has_span(&self) -> bool {
        true
    }

    fn possible_span(&self) -> Option<&Z80Span> {
        Some(self.span())
    }

    fn span(&self) -> &Z80Span {
        match self {
            LocatedExpr::RelativeDelta(_, span)
            | LocatedExpr::Value(_, span)
            | LocatedExpr::Float(_, span)
            | LocatedExpr::Char(_, span)
            | LocatedExpr::Bool(_, span)
            | LocatedExpr::String(UnescapedString(_, span))
            | LocatedExpr::Label(span)
            | LocatedExpr::List(_, span)
            | LocatedExpr::PrefixedLabel(_, _, span)
            | LocatedExpr::Paren(_, span)
            | LocatedExpr::UnaryOperation(_, _, span)
            | LocatedExpr::UnaryTokenOperation(_, _, span)
            | LocatedExpr::BinaryOperation(_, _, _, span)
            | LocatedExpr::Ternary(_, _, _, span)
            | LocatedExpr::AnyFunction(_, _, span)
            | LocatedExpr::Rnd(span) => span
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LocatedDataAccess {
    /// We are using an indexed register associated to its index
    IndexRegister16WithIndex(IndexRegister16, BinaryOperation, LocatedExpr, Z80Span),
    IndexRegister16(IndexRegister16, Z80Span),
    IndexRegister8(IndexRegister8, Z80Span),
    /// Represents a standard 16 bits register
    Register16(Register16, Z80Span),
    /// Represents a standard 8 bits register
    Register8(Register8, Z80Span),
    /// Represents a memory access indexed by a register
    MemoryRegister16(Register16, Z80Span),
    MemoryIndexRegister16(IndexRegister16, Z80Span),
    /// Represents any expression
    Expression(LocatedExpr),
    /// Represents an address
    Memory(LocatedExpr),
    /// Represnts the test of bit flag
    FlagTest(FlagTest, Z80Span),
    /// Special register I
    SpecialRegisterI(Z80Span),
    /// Special register R
    SpecialRegisterR(Z80Span),
    /// Used for in/out instructions
    PortC(Z80Span),
    /// Used for in/out instructions
    PortN(LocatedExpr, Z80Span)
}

impl From<LocatedExpr> for LocatedDataAccess {
    fn from(value: LocatedExpr) -> Self {
        Self::Expression(value)
    }
}

impl From<LocatedDataAccess> for DataAccess {
    fn from(val: LocatedDataAccess) -> Self {
        val.to_data_access()
    }
}

impl LocatedDataAccess {
    pub fn to_data_access(self) -> DataAccess {
        match self {
            LocatedDataAccess::IndexRegister16WithIndex(r, b, e, _) => {
                DataAccess::IndexRegister16WithIndex(r, b, e.to_expr().into_owned())
            },
            LocatedDataAccess::IndexRegister16(i, _) => DataAccess::IndexRegister16(i),
            LocatedDataAccess::IndexRegister8(i, _) => DataAccess::IndexRegister8(i),
            LocatedDataAccess::Register16(r, _) => DataAccess::Register16(r),
            LocatedDataAccess::Register8(r, _) => DataAccess::Register8(r),
            LocatedDataAccess::MemoryRegister16(r, _) => DataAccess::MemoryRegister16(r),
            LocatedDataAccess::MemoryIndexRegister16(i, _) => DataAccess::MemoryIndexRegister16(i),
            LocatedDataAccess::Expression(e) => DataAccess::Expression(e.to_expr().into_owned()),
            LocatedDataAccess::Memory(a) => DataAccess::Memory(a.to_expr().into_owned()),
            LocatedDataAccess::FlagTest(f, _) => DataAccess::FlagTest(f),
            LocatedDataAccess::SpecialRegisterI(_) => DataAccess::SpecialRegisterI,
            LocatedDataAccess::SpecialRegisterR(_) => DataAccess::SpecialRegisterI,
            LocatedDataAccess::PortC(_) => DataAccess::PortC,
            LocatedDataAccess::PortN(p, _) => DataAccess::PortN(p.to_expr().into_owned())
        }
    }
}

impl LocatedDataAccess {
    pub fn span(&self) -> &Z80Span {
        match self {
            LocatedDataAccess::IndexRegister16WithIndex(_, _, _, span)
            | LocatedDataAccess::IndexRegister16(_, span)
            | LocatedDataAccess::IndexRegister8(_, span)
            | LocatedDataAccess::Register16(_, span)
            | LocatedDataAccess::Register8(_, span)
            | LocatedDataAccess::MemoryRegister16(_, span)
            | LocatedDataAccess::MemoryIndexRegister16(_, span)
            | LocatedDataAccess::FlagTest(_, span)
            | LocatedDataAccess::SpecialRegisterI(span)
            | LocatedDataAccess::SpecialRegisterR(span)
            | LocatedDataAccess::PortC(span)
            | LocatedDataAccess::PortN(_, span) => span,

            LocatedDataAccess::Memory(e) | LocatedDataAccess::Expression(e) => e.span()
        }
    }
}

impl Display for LocatedDataAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.span())
    }
}

impl From<LocatedExpr> for Expr {
    fn from(val: LocatedExpr) -> Self {
        val.to_expr().into_owned()
    }
}

impl DataAccessElem for LocatedDataAccess {
    type Expr = LocatedExpr;

    data_access_impl_most_methods!();

    fn to_data_access_for_low_register(&self) -> Option<Self> {
        match self {
            Self::IndexRegister16(reg, span) => {
                Some(LocatedDataAccess::IndexRegister8(reg.low(), span.clone()))
            },
            Self::Register16(reg, span) => {
                reg.low()
                    .map(|reg| LocatedDataAccess::Register8(reg, span.clone()))
            },
            _ => None
        }
    }

    fn to_data_access_for_high_register(&self) -> Option<Self> {
        match self {
            Self::IndexRegister16(reg, span) => {
                Some(LocatedDataAccess::IndexRegister8(reg.high(), span.clone()))
            },
            Self::Register16(reg, span) => {
                reg.high()
                    .map(|reg| LocatedDataAccess::Register8(reg, span.clone()))
            },
            _ => None
        }
    }

    fn is_port_c(&self) -> bool {
        match self {
            Self::PortC(..) => true,
            _ => false
        }
    }

    fn is_register_i(&self) -> bool {
        match self {
            Self::SpecialRegisterI(..) => true,
            _ => false
        }
    }

    fn is_register_r(&self) -> bool {
        match self {
            Self::SpecialRegisterR(..) => true,
            _ => false
        }
    }

    fn to_data_access(&self) -> Cow<'_, DataAccess> {
        Cow::Owned(match self {
            LocatedDataAccess::IndexRegister16WithIndex(a, b, c, _) => {
                DataAccess::IndexRegister16WithIndex(*a, *b, c.to_expr().into_owned())
            },
            LocatedDataAccess::IndexRegister16(a, _) => DataAccess::IndexRegister16(*a),
            LocatedDataAccess::IndexRegister8(a, _) => DataAccess::IndexRegister8(*a),
            LocatedDataAccess::Register16(a, _) => DataAccess::Register16(*a),
            LocatedDataAccess::Register8(a, _) => DataAccess::Register8(*a),
            LocatedDataAccess::MemoryRegister16(a, _) => DataAccess::MemoryRegister16(*a),
            LocatedDataAccess::MemoryIndexRegister16(a, _) => DataAccess::MemoryIndexRegister16(*a),
            LocatedDataAccess::Expression(e) => DataAccess::Expression(e.to_expr().into_owned()),
            LocatedDataAccess::Memory(a) => DataAccess::Memory(a.to_expr().into_owned()),
            LocatedDataAccess::FlagTest(a, _) => DataAccess::FlagTest(*a),
            LocatedDataAccess::SpecialRegisterI(_) => DataAccess::SpecialRegisterI,
            LocatedDataAccess::SpecialRegisterR(_) => DataAccess::SpecialRegisterR,
            LocatedDataAccess::PortC(_) => DataAccess::PortC,
            LocatedDataAccess::PortN(e, _) => DataAccess::PortN(e.to_expr().into_owned())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocatedMacroParam {
    Empty,
    /// Standard argument directly propagated
    RawArgument(Z80Span),
    /// Standard argument evaluated before exapansion
    EvaluatedArgument(Z80Span),
    /// A list of argument that will be provided in a nested macro call
    List(Vec<Box<LocatedMacroParam>>)
}

impl MacroParamElement for LocatedMacroParam {
    fn empty() -> Self {
        Self::Empty
    }

    fn must_be_evaluated(&self) -> bool {
        matches!(self, LocatedMacroParam::EvaluatedArgument(..))
    }

    fn is_single(&self) -> bool {
        matches!(
            self,
            LocatedMacroParam::RawArgument(..)
                | LocatedMacroParam::EvaluatedArgument(..)
                | LocatedMacroParam::Empty
        )
    }

    fn is_list(&self) -> bool {
        matches!(self, LocatedMacroParam::List(..))
    }

    fn single_argument(&self) -> beef::lean::Cow<'_, str> {
        match &self {
            LocatedMacroParam::Empty => beef::lean::Cow::borrowed(""),
            LocatedMacroParam::RawArgument(s) | LocatedMacroParam::EvaluatedArgument(s) => {
                beef::lean::Cow::borrowed(s.as_str())
            },
            LocatedMacroParam::List(_) => unreachable!()
        }
    }

    fn list_argument(&self) -> &[Box<Self>] {
        match self {
            LocatedMacroParam::List(l) => l,
            _ => &[]
        }
    }
}

impl LocatedMacroParam {
    pub fn to_macro_param(&self) -> MacroParam {
        match self {
            LocatedMacroParam::List(params) => {
                MacroParam::List(
                    params
                        .iter()
                        .map(|p| p.to_macro_param())
                        .map(Box::new)
                        .collect_vec()
                )
            },

            LocatedMacroParam::RawArgument(_) | LocatedMacroParam::Empty => {
                MacroParam::RawArgument(self.single_argument().to_string())
            },

            LocatedMacroParam::EvaluatedArgument(_) => {
                MacroParam::EvaluatedArgument(self.single_argument().to_string())
            },
        }
    }

    pub fn span(&self) -> Z80Span {
        match self {
            LocatedMacroParam::RawArgument(span) | LocatedMacroParam::EvaluatedArgument(span) => {
                span.clone()
            },
            LocatedMacroParam::List(_) => todo!(),
            LocatedMacroParam::Empty => panic!()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            LocatedTestKind::True(e) => TestKind::True(e.to_expr().into_owned()),
            LocatedTestKind::False(e) => TestKind::False(e.to_expr().into_owned()),
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
            LocatedTestKind::True(exp) | LocatedTestKind::False(exp) => exp,
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

#[derive(Debug, PartialEq, Eq)]
pub enum LocatedAssemblerControlCommand {
    RestrictedAssemblingEnvironment {
        passes: Option<LocatedExpr>,
        lst: LocatedListing
    },
    PrintAtParsingState(Vec<FormattedExpr>), // completely ignored during assembling
    PrintAtAssemblingState(Vec<FormattedExpr>)
}

impl AssemblerControlCommand for LocatedAssemblerControlCommand {
    type Expr = LocatedExpr;
    type T = LocatedToken;

    fn is_restricted_assembling_environment(&self) -> bool {
        matches!(
            self,
            LocatedAssemblerControlCommand::RestrictedAssemblingEnvironment { .. }
        )
    }

    fn is_print_at_parse_state(&self) -> bool {
        matches!(self, LocatedAssemblerControlCommand::PrintAtParsingState(_))
    }

    fn is_print_at_assembling_state(&self) -> bool {
        matches!(
            self,
            LocatedAssemblerControlCommand::PrintAtAssemblingState(_)
        )
    }

    fn get_max_nb_passes(&self) -> Option<&Self::Expr> {
        match self {
            LocatedAssemblerControlCommand::RestrictedAssemblingEnvironment { passes, .. } => {
                passes.as_ref()
            },
            _ => unreachable!()
        }
    }

    fn get_listing(&self) -> &[Self::T] {
        match self {
            LocatedAssemblerControlCommand::RestrictedAssemblingEnvironment { lst, .. } => lst,
            _ => unreachable!()
        }
    }

    fn get_formatted_expr(&self) -> &[FormattedExpr] {
        match self {
            LocatedAssemblerControlCommand::PrintAtAssemblingState(e)
            | LocatedAssemblerControlCommand::PrintAtParsingState(e) => e,
            _ => unreachable!()
        }
    }
}

// Encode the LocatedToken BEFORE computing its span
#[derive(Debug, PartialEq, Eq)]
pub enum LocatedTokenInner {
    Abyte(LocatedExpr, Vec<LocatedExpr>),
    Align(LocatedExpr, Option<LocatedExpr>),
    Assert(LocatedExpr, Option<Vec<FormattedExpr>>),
    AssemblerControl(LocatedAssemblerControlCommand),
    Assign {
        label: Z80Span,
        expr: LocatedExpr,
        op: Option<BinaryOperation>
    },

    Bank(Option<LocatedExpr>),
    Bankset(LocatedExpr),
    Basic(Option<Vec<Z80Span>>, Option<Vec<LocatedExpr>>, Z80Span),
    Break,
    /// Breakpoints are quite biased toward Ace-Dl representation
    // for each field (span to the filed name, value with potential span)
    Breakpoint {
        address: Option<Box<LocatedExpr>>,
        r#type: Option<RemuBreakPointType>,
        access: Option<RemuBreakPointAccessMode>,
        run: Option<RemuBreakPointRunMode>,
        mask: Option<Box<LocatedExpr>>,
        size: Option<Box<LocatedExpr>>,
        value: Option<Box<LocatedExpr>>,
        value_mask: Option<Box<LocatedExpr>>,
        condition: Option<Box<LocatedExpr>>,
        name: Option<Box<LocatedExpr>>,
        step: Option<Box<LocatedExpr>>
    },
    BuildCpr,
    BuildSna(Option<SnapshotVersion>),

    Charset(CharsetFormat),
    Comment(Z80Span),
    Confined(LocatedListing),
    CrunchedSection(CrunchType, LocatedListing),
    Defb(Vec<LocatedExpr>),
    Defs(Vec<(LocatedExpr, Option<LocatedExpr>)>),
    Defw(Vec<LocatedExpr>),
    End,
    Equ {
        label: Z80Span,
        expr: LocatedExpr
    },
    Even,
    Export(Vec<Z80Span>),

    Fail(Option<Vec<FormattedExpr>>),
    Field {
        label: Z80Span,
        expr: LocatedExpr
    },
    For {
        label: Z80Span,
        start: Box<LocatedExpr>,
        stop: Box<LocatedExpr>,
        step: Option<Box<LocatedExpr>>,
        listing: Box<LocatedListing>
    },
    Function(Z80Span, Vec<Z80Span>, LocatedListing),
    If(
        Vec<(LocatedTestKind, LocatedListing)>,
        Option<LocatedListing>
    ),
    Incbin {
        fname: LocatedExpr,
        offset: Option<LocatedExpr>,
        length: Option<LocatedExpr>,
        extended_offset: Option<LocatedExpr>,
        off: bool,
        transformation: BinaryTransformation
    },
    Include(LocatedExpr, Option<Z80Span>, bool),
    Iterate(
        Z80Span,
        either::Either<Vec<LocatedExpr>, LocatedExpr>,
        LocatedListing
    ),

    Label(Z80Span),
    Let(Z80Span, Expr),
    Limit(LocatedExpr),
    List,

    Macro {
        name: Z80Span,
        params: Vec<Z80Span>,
        content: Z80Span,
        flavor: AssemblerFlavor,
        tokenized_content: TokenizedMacroContent
    },
    /// Name, Parameters, FullSpan
    MacroCall(Z80Span, Vec<LocatedMacroParam>),
    Map(LocatedExpr),
    Module(Z80Span, LocatedListing),
    MultiPop(Vec<LocatedDataAccess>),
    MultiPush(Vec<LocatedDataAccess>),

    Next {
        label: Z80Span,
        source: Z80Span,
        expr: Option<LocatedExpr>
    },
    NoExport(Vec<Z80Span>),
    NoList,

    OpCode(
        Mnemonic,
        Option<LocatedDataAccess>,
        Option<LocatedDataAccess>,
        Option<Register8>
    ),

    Org {
        val1: LocatedExpr,
        val2: Option<LocatedExpr>
    },
    OutputFile(LocatedExpr),
    Pause,
    Print(Vec<FormattedExpr>),
    Protect(LocatedExpr, LocatedExpr),

    Range(Z80Span, LocatedExpr, LocatedExpr),
    Repeat(
        LocatedExpr,         // amount
        LocatedListing,      // code
        Option<Z80Span>,     // Name of the counter TODO check why it is optional
        Option<LocatedExpr>, // Start value
        Option<LocatedExpr>  // Step value
    ),
    RepeatToken {
        token: Box<LocatedToken>,
        repeat: LocatedExpr
    },

    RepeatUntil(LocatedExpr, LocatedListing),
    Return(LocatedExpr),
    Rorg(LocatedExpr, LocatedListing),
    Run(LocatedExpr, Option<LocatedExpr>),

    Save {
        filename: LocatedExpr,
        address: Option<LocatedExpr>,
        size: Option<LocatedExpr>,
        save_type: Option<SaveType>,
        dsk_filename: Option<LocatedExpr>,
        side: Option<LocatedExpr>
    },
    Section(Z80Span),
    SetN {
        label: Z80Span,
        source: Z80Span,
        expr: Option<LocatedExpr>
    },
    Skip(LocatedExpr),
    SnaInit(LocatedExpr),
    SnaSet(SnapshotFlag, FlagValue),
    StableTicker(StableTickerAction<Z80Span>),
    StartingIndex {
        start: Option<LocatedExpr>,
        step: Option<LocatedExpr>
    },
    Str(Vec<LocatedExpr>),
    Struct(Z80Span, Vec<(Z80Span, LocatedToken)>),
    Switch(
        LocatedExpr,
        Vec<(LocatedExpr, LocatedListing, bool)>,
        Option<LocatedListing>
    ),
    Undef(Z80Span),

    WaitNops(LocatedExpr),
    Warning(Option<Vec<FormattedExpr>>),
    WarningWrapper(Box<Self>, String),
    While(LocatedExpr, LocatedListing)
}

impl LocatedTokenInner {
    pub fn new_opcode(
        mne: Mnemonic,
        arg1: Option<LocatedDataAccess>,
        arg2: Option<LocatedDataAccess>
    ) -> Self {
        LocatedTokenInner::OpCode(mne, arg1, arg2, None)
    }

    pub fn into_located_token_direct(self) -> LocatedToken {
        let span = match &self {
            Self::Label(span) | Self::Comment(span) => span.clone(),

            _ => todo!("not coded yet or impossible {:?}", self)
        };

        LocatedToken {
            inner: either::Either::Left(self),
            span
        }
    }

    pub fn into_located_token_at(self, span: InnerZ80Span) -> LocatedToken {
        match self {
            Self::WarningWrapper(token, msg) => {
                let warned = Box::new(token.into_located_token_at(span));
                LocatedToken {
                    inner: either::Right((warned, msg)),
                    span: span.into()
                }
            },

            _ => {
                LocatedToken {
                    inner: either::Either::Left(self),
                    span: span.into()
                }
            },
        }
    }

    /// start: checkpoint for the start of the token
    /// stop: checkpoint for the stop of the token
    /// i: stream after parse: is copied
    #[inline]
    pub fn into_located_token_between(
        self,
        start_checkpoint: &<InnerZ80Span as Stream>::Checkpoint,
        mut i: InnerZ80Span
    ) -> LocatedToken {
        let input = i;

        i.reset(start_checkpoint);
        let start_eof_offset: usize = i.eof_offset();

        let span = build_span(start_eof_offset, start_checkpoint, input);

        self.into_located_token_at(span)
    }
}

/// Add span information for a Token.
#[derive(Debug, PartialEq, Eq)]
pub struct LocatedToken {
    // The token of interest of a warning with the token of interest
    pub(crate) inner: either::Either<LocatedTokenInner, (Box<LocatedToken>, String)>,
    pub(crate) span: Z80Span
}

macro_rules! is_stuff_delegate {
    ($($name: ident)*) => {
        $(
            #[inline(always)]
            fn $name(&self) -> bool {
                self.inner.as_ref().left()
                    .map(|inner| inner.$name())
                    .unwrap_or(false)
            }
        )*
    };
}

macro_rules! any_delegate {
    ( $(fn $name:ident(&self) -> $return:ty);+ ;) => {
        $(
        #[inline(always)]
        fn $name(&self) -> $return {
            self.inner.as_ref().left().unwrap().$name()
        }
    )*
    };
}

impl ListingElement for LocatedToken {
    type AssemblerControlCommand = LocatedAssemblerControlCommand;
    type DataAccess = LocatedDataAccess;
    type Expr = LocatedExpr;
    type MacroParam = LocatedMacroParam;
    type TestKind = LocatedTestKind;

    is_stuff_delegate!(
        is_print is_buildcpr is_label is_equ
        is_assign is_module is_directive is_rorg
        is_iterate is_for is_repeat_until is_repeat
        is_macro_definition is_if is_include is_incbin
        is_call_macro_or_build_struct is_function_definition
        is_crunched_section is_confined is_switch
        is_db is_dw is_str is_set is_comment is_org
        is_assembler_control is_while is_assert
        is_run is_breakpoint is_save
        is_repeat_token
    );

    any_delegate!(
        fn assign_symbol(&self) -> &str;
        fn comment(&self) -> &str;
        fn assign_value(&self) -> &Self::Expr;
        fn equ_symbol(&self) -> &str;
        fn label_symbol(&self) -> &str;
        fn equ_value(&self) -> &Self::Expr;
        fn module_name(&self) -> &str;
        fn while_expr(&self) -> &Self::Expr;
        fn mnemonic(&self) -> Option<&Mnemonic>;
        fn mnemonic_arg1(&self) -> Option<&Self::DataAccess>;
        fn mnemonic_arg2(&self) -> Option<&Self::DataAccess>;
        fn rorg_expr(&self) -> &Self::Expr;
        fn iterate_counter_name(&self) -> &str;
        fn iterate_values(&self) -> either::Either<&Vec<Self::Expr>, &Self::Expr>;
        fn for_label(&self) -> &str;
        fn for_start(&self) -> &Self::Expr;
        fn for_stop(&self) -> &Self::Expr;
        fn for_step(&self) -> Option<&Self::Expr>;
        fn repeat_until_condition(&self) -> &Self::Expr;
        fn repeat_count(&self) -> &Self::Expr;
        fn repeat_counter_name(&self) -> Option<&str>;
        fn repeat_counter_start(&self) -> Option<&Self::Expr>;
        fn repeat_counter_step(&self) -> Option<&Self::Expr>;
        fn macro_definition_name(&self) -> &str;
        fn macro_definition_arguments(&self) -> SmallVec<[&str; 4]>;
        fn macro_definition_code(&self) -> &str;
        fn macro_call_name(&self) -> &str;
        fn macro_call_arguments(&self) -> &[Self::MacroParam];
        fn if_nb_tests(&self) -> usize;
        fn incbin_fname(&self) -> &Self::Expr;
        fn incbin_offset(&self) -> Option<&Self::Expr>;
        fn incbin_length(&self) -> Option<&Self::Expr>;
        fn incbin_transformation(&self) -> &cpclib_tokens::BinaryTransformation;
        fn include_fname(&self) -> &Self::Expr;
        fn include_namespace(&self) -> Option<&str>;
        fn include_once(&self) -> bool;
        fn function_definition_name(&self) -> &str;
        fn function_definition_params(&self) -> SmallVec<[&str; 4]>;
        fn crunched_section_kind(&self) -> &CrunchType;
        fn switch_expr(&self) -> &Self::Expr;
        fn data_exprs(&self) -> &[Self::Expr];
        fn assembler_control_command(&self) -> &Self::AssemblerControlCommand;
        fn assembler_control_get_max_passes(&self) -> Option<&Self::Expr>;
        fn macro_flavor(&self) -> AssemblerFlavor;
        fn run_expr(&self) -> &Self::Expr;
        fn org_first(&self) -> &Self::Expr;
        fn org_second(&self) -> Option<&Self::Expr>;
    );

    fn to_token(&self) -> Cow<'_, cpclib_tokens::Token> {
        match &self.inner {
            either::Either::Left(inner) => inner.to_token(),
            either::Either::Right((_inner, _msg)) => {
                unimplemented!("is it necessary to implement it ?")
            }
        }
    }

    fn is_warning(&self) -> bool {
        self.inner.is_right()
    }

    fn warning_message(&self) -> &str {
        match &self.inner {
            either::Right((_inner, msg)) => msg.as_str(),
            _ => unreachable!()
        }
    }

    fn while_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::While(_, lst, ..)) => lst,
            _ => unreachable!()
        }
    }

    fn module_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Module(_, lst, ..)) => lst,
            _ => unreachable!()
        }
    }

    fn mnemonic_arg1_mut(&mut self) -> Option<&mut Self::DataAccess> {
        match &mut self.inner {
            either::Left(LocatedTokenInner::OpCode(_, arg1, ..)) => arg1.as_mut(),

            _ => None
        }
    }

    fn mnemonic_arg2_mut(&mut self) -> Option<&mut Self::DataAccess> {
        match &mut self.inner {
            either::Left(LocatedTokenInner::OpCode(_, _, arg2, _)) => arg2.as_mut(),

            _ => None
        }
    }

    fn rorg_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Rorg(_, lst)) => lst,
            _ => unreachable!()
        }
    }

    fn iterate_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Iterate(_, _, listing, ..)) => listing,
            _ => unreachable!()
        }
    }

    fn for_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::For { listing, .. }) => listing,
            _ => unreachable!()
        }
    }

    fn repeat_until_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::RepeatUntil(_, code, ..)) => code,
            _ => unreachable!()
        }
    }

    fn repeat_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Repeat(_, listing, ..)) => listing,
            _ => unreachable!()
        }
    }

    fn repeat_token(&self) -> &Self {
        match &self.inner {
            either::Left(LocatedTokenInner::RepeatToken { token, .. }) => token,
            _ => unreachable!()
        }
    }

    fn function_definition_inner(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Function(_, _, inner)) => inner,
            _ => unreachable!()
        }
    }

    fn crunched_section_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::CrunchedSection(_, lst)) => lst,
            _ => unreachable!()
        }
    }

    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]) {
        match &self.inner {
            either::Left(LocatedTokenInner::If(tests, ..)) => {
                let data = &tests[idx];
                (&data.0, &data.1)
            },
            _ => panic!()
        }
    }

    fn if_else(&self) -> Option<&[Self]> {
        match &self.inner {
            either::Left(LocatedTokenInner::If(_, r#else)) => r#else.as_ref().map(|l| l.as_slice()),
            _ => panic!()
        }
    }

    fn confined_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Confined(lst)) => lst,
            _ => unreachable!()
        }
    }

    fn switch_cases(&self) -> Box<dyn Iterator<Item = (&Self::Expr, &[Self], bool)> + '_> {
        match &self.inner {
            either::Left(LocatedTokenInner::Switch(_, cases, ..)) => {
                Box::new(cases.iter().map(|c| (&c.0, c.1.as_slice(), c.2)))
            },
            _ => unreachable!()
        }
    }

    fn switch_default(&self) -> Option<&[Self]> {
        match &self.inner {
            either::Left(LocatedTokenInner::Switch(_, _, default, ..)) => {
                default.as_ref().map(|l| l.as_slice())
            },
            _ => unreachable!()
        }
    }

    fn warning_token(&self) -> &Self {
        match &self.inner {
            either::Either::Left(_) => unreachable!(),
            either::Either::Right((inner, _msg)) => inner
        }
    }

    fn assembler_control_get_listing(&self) -> &[Self] {
        self.assembler_control_command().get_listing()
    }

    /// Override symbols() to safely extract symbols without going through problematic delegates
    fn symbols(&self) -> std::collections::HashSet<String> {
        use std::collections::HashSet;

        // Skip comments and labels - they're definitions, not references
        if self.is_comment() || self.is_label() || self.is_macro_definition() {
            return HashSet::new();
        }

        // Extract symbols by pattern matching on the inner token directly
        match &self.inner {
            either::Left(token) => {
                // Delegate to Token's ListingElement impl which has the default implementation
                token.symbols()
            },
            either::Right(_) => {
                // Comments/warnings wrapped in Right - no user symbols
                HashSet::new()
            }
        }
    }
}

// Several methodsare not implemented because their return type is wrong
// it does not really matter because we never have to call them
impl ListingElement for LocatedTokenInner {
    type AssemblerControlCommand = LocatedAssemblerControlCommand;
    type DataAccess = LocatedDataAccess;
    type Expr = LocatedExpr;
    type MacroParam = LocatedMacroParam;
    type TestKind = LocatedTestKind;

    listing_element_impl_most_methods!();

    /// Override symbols() to extract symbols without requiring full to_token() conversion
    fn symbols(&self) -> std::collections::HashSet<String> {
        use std::collections::HashSet;

        let mut symbols = HashSet::new();

        // Extract symbols based on the variant type directly
        match self {
            // Skip comments and label definitions - they're definitions, not references
            Self::Comment(_) | Self::Label(_) | Self::Macro { .. } => {},

            // Expression-based tokens
            Self::Org { val1, val2 } => {
                symbols.extend(val1.symbols());
                if let Some(val2) = val2 {
                    symbols.extend(val2.symbols());
                }
            },
            Self::Equ { expr, .. } => {
                symbols.extend(expr.symbols());
            },
            Self::Assign { expr, .. } => {
                symbols.extend(expr.symbols());
            },
            Self::OpCode(_, arg1, arg2, _) => {
                if let Some(arg1) = arg1 {
                    if let Some(expr) = arg1.get_expression() {
                        symbols.extend(expr.symbols());
                    }
                }
                if let Some(arg2) = arg2 {
                    if let Some(expr) = arg2.get_expression() {
                        symbols.extend(expr.symbols());
                    }
                }
            },
            Self::While(expr, _) => {
                symbols.extend(expr.symbols());
            },
            Self::Switch(expr, cases, _) => {
                symbols.extend(expr.symbols());
                for (case_expr, ..) in cases {
                    symbols.extend(case_expr.symbols());
                }
            },
            Self::Iterate(_, values, _) => {
                match values {
                    either::Either::Left(exprs) => {
                        for expr in exprs {
                            symbols.extend(expr.symbols());
                        }
                    },
                    either::Either::Right(expr) => {
                        symbols.extend(expr.symbols());
                    }
                }
            },
            Self::For {
                start, stop, step, ..
            } => {
                symbols.extend(start.symbols());
                symbols.extend(stop.symbols());
                if let Some(step) = step {
                    symbols.extend(step.symbols());
                }
            },
            Self::Repeat(expr, _, _, start, step) => {
                symbols.extend(expr.symbols());
                if let Some(start) = start {
                    symbols.extend(start.symbols());
                }
                if let Some(step) = step {
                    symbols.extend(step.symbols());
                }
            },
            Self::RepeatUntil(expr, _) => {
                symbols.extend(expr.symbols());
            },
            Self::Rorg(expr, _) => {
                symbols.extend(expr.symbols());
            },
            Self::Incbin {
                fname,
                offset,
                length,
                extended_offset,
                ..
            } => {
                symbols.extend(fname.symbols());
                if let Some(offset) = offset {
                    symbols.extend(offset.symbols());
                }
                if let Some(length) = length {
                    symbols.extend(length.symbols());
                }
                if let Some(extended_offset) = extended_offset {
                    symbols.extend(extended_offset.symbols());
                }
            },
            Self::Include(fname, ..) => {
                symbols.extend(fname.symbols());
            },
            Self::Defb(exprs) | Self::Defw(exprs) => {
                for expr in exprs {
                    symbols.extend(expr.symbols());
                }
            },
            Self::Str(exprs) => {
                for expr in exprs {
                    symbols.extend(expr.symbols());
                }
            },
            Self::Run(expr, expr2) => {
                symbols.extend(expr.symbols());
                if let Some(expr2) = expr2 {
                    symbols.extend(expr2.symbols());
                }
            },
            Self::MacroCall(name, _) => {
                // Macro name is a symbol reference
                symbols.insert(name.to_string());
            },
            Self::Return(expr) => {
                symbols.extend(expr.symbols());
            },
            Self::Assert(expr, _) => {
                symbols.extend(expr.symbols());
            },
            Self::Fail(_) | Self::Warning(_) => {},
            Self::OutputFile(expr) => {
                symbols.extend(expr.symbols());
            },
            Self::Breakpoint {
                address,
                mask,
                size,
                value,
                value_mask,
                condition,
                name,
                step,
                ..
            } => {
                if let Some(address) = address {
                    symbols.extend(address.symbols());
                }
                if let Some(mask) = mask {
                    symbols.extend(mask.symbols());
                }
                if let Some(size) = size {
                    symbols.extend(size.symbols());
                }
                if let Some(value) = value {
                    symbols.extend(value.symbols());
                }
                if let Some(value_mask) = value_mask {
                    symbols.extend(value_mask.symbols());
                }
                if let Some(condition) = condition {
                    symbols.extend(condition.symbols());
                }
                if let Some(name) = name {
                    symbols.extend(name.symbols());
                }
                if let Some(step) = step {
                    symbols.extend(step.symbols());
                }
            },
            // For other tokens, skip - they don't contain symbol references
            _ => {}
        }

        symbols
    }

    /// Transform the located token in a raw token.
    /// Warning, this is quite costly when strings or vec are involved
    fn to_token(&self) -> Cow<'_, Token> {
        match self {
            Self::OpCode(mne, arg1, arg2, arg3) => {
                Cow::Owned(Token::OpCode(
                    *mne,
                    arg1.as_ref().map(|d| d.to_data_access().into_owned()),
                    arg2.as_ref().map(|d| d.to_data_access().into_owned()),
                    *arg3
                ))
            },
            Self::Comment(cmt) => Cow::Owned(Token::Comment(cmt.to_string())),
            Self::Org { val1, val2 } => {
                Cow::Owned(Token::Org {
                    val1: val1.to_expr().into_owned(),
                    val2: val2.as_ref().map(|val2| val2.to_expr().into_owned())
                })
            },
            Self::CrunchedSection(c, l) => Cow::Owned(Token::CrunchedSection(*c, l.as_listing())),
            Self::Function(name, params, inner) => {
                Cow::Owned(Token::Function(
                    name.into(),
                    params.iter().map(|p| p.into()).collect_vec(),
                    inner.as_listing()
                ))
            },
            Self::If(v, e) => {
                Cow::Owned(Token::If(
                    v.iter()
                        .map(|(k, l)| (k.to_test_kind(), l.as_listing()))
                        .collect_vec(),
                    e.as_ref().map(|l| l.as_listing())
                ))
            },
            Self::Repeat(_e, _l, _s, _start, _step) => {
                unimplemented!("step");
                #[allow(unreachable_code)]
                {
                    Cow::Owned(Token::Repeat(
                        _e.to_expr().into_owned(),
                        _l.as_listing(),
                        _s.as_ref().map(|s| s.into()),
                        _start.as_ref().map(|e| e.to_expr().into_owned())
                    ))
                }
            },
            Self::RepeatUntil(e, l) => {
                Cow::Owned(Token::RepeatUntil(e.to_expr().into_owned(), l.as_listing()))
            },
            Self::Rorg(e, l) => Cow::Owned(Token::Rorg(e.to_expr().into_owned(), l.as_listing())),
            Self::Switch(v, c, d) => {
                Cow::Owned(Token::Switch(
                    v.to_expr().into_owned(),
                    c.iter()
                        .map(|(e, l, b)| (e.to_expr().into_owned(), l.as_listing(), *b))
                        .collect_vec(),
                    d.as_ref().map(|d| d.as_listing())
                ))
            },
            Self::While(e, l) => Cow::Owned(Token::While(e.to_expr().into_owned(), l.as_listing())),
            Self::Iterate(_name, _values, _code) => {
                todo!()
            },
            Self::Module(..) => todo!(),
            Self::For {
                label,
                start,
                stop,
                step,
                listing
            } => {
                Cow::Owned(Token::For {
                    label: label.into(),
                    start: Box::new(start.to_expr().into_owned()),
                    stop: Box::new(stop.to_expr().into_owned()),
                    step: step.as_ref().map(|e| Box::new(e.to_expr().into_owned())),
                    listing: Box::new(listing.as_listing())
                })
            },
            Self::Label(label) => Cow::Owned(Token::Label(label.into())),
            Self::MacroCall(name, params) => {
                Cow::Owned(Token::MacroCall(
                    name.into(),
                    params.iter().map(|p| p.to_macro_param()).collect_vec()
                ))
            },
            Self::Struct(name, params) => {
                Cow::Owned(Token::Struct(
                    name.into(),
                    params
                        .iter()
                        .map(|(label, p)| (label.into(), p.as_simple_token().into_owned()))
                        .collect_vec()
                ))
            },
            Self::Defb(exprs) => {
                Cow::Owned(Token::Defb(
                    exprs.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                ))
            },
            Self::Defw(exprs) => {
                Cow::Owned(Token::Defw(
                    exprs.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                ))
            },
            Self::Str(exprs) => {
                Cow::Owned(Token::Str(
                    exprs.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                ))
            },

            Self::Include(..) => todo!(),
            Self::Incbin {
                fname,
                offset,
                length,
                extended_offset,
                off,
                transformation
            } => {
                Cow::Owned(Token::Incbin {
                    fname: fname.to_expr().into_owned(),
                    offset: offset.as_ref().map(|e| e.to_expr().into_owned()),
                    length: length.as_ref().map(|e| e.to_expr().into_owned()),
                    extended_offset: extended_offset.as_ref().map(|e| e.to_expr().into_owned()),
                    off: *off,
                    transformation: *transformation
                })
            },
            Self::Macro {
                name,
                params,
                content,
                flavor,
                tokenized_content
            } => {
                Cow::Owned(Token::Macro {
                    name: name.into(),
                    params: params.iter().map(|p| p.into()).collect_vec(),
                    content: content.as_str().to_owned(),
                    flavor: *flavor,
                    tokenized_content: tokenized_content.clone()
                })
            },
            Self::Confined(..) => todo!(),
            Self::WarningWrapper(..) => todo!(),
            Self::Assign {
                label: _,
                expr: _,
                op: _
            } => todo!(),
            Self::Equ { label, expr } => {
                Cow::Owned(Token::Equ {
                    label: label.as_str().into(),
                    expr: expr.to_expr().into_owned()
                })
            },
            Self::Even => Cow::Borrowed(&Token::Even),
            Self::SetN {
                label: _,
                source: _,
                expr: _
            } => todo!(),
            Self::Next {
                label: _,
                source: _,
                expr: _
            } => todo!(),

            Self::Assert(test, print) => {
                Cow::Owned(Token::Assert(test.to_expr().into_owned(), print.clone()))
            },

            Self::Fail(msg) => Cow::Owned(Token::Fail(msg.clone())),
            Self::Warning(msg) => Cow::Owned(Token::Warning(msg.clone())),
            Self::OutputFile(filename) => {
                Cow::Owned(Token::OutputFile(filename.to_expr().into_owned()))
            },
            Self::Breakpoint {
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
                Cow::Owned(Token::Breakpoint {
                    address: address.as_ref().map(|e| Box::new(e.to_expr().into_owned())),
                    r#type: r#type.clone(),
                    access: access.clone(),
                    run: run.clone(),
                    mask: mask.as_ref().map(|e| Box::new(e.to_expr().into_owned())),
                    size: size.as_ref().map(|e| Box::new(e.to_expr().into_owned())),
                    value: value.as_ref().map(|e| Box::new(e.to_expr().into_owned())),
                    value_mask: value_mask
                        .as_ref()
                        .map(|e| Box::new(e.to_expr().into_owned())),
                    condition: condition
                        .as_ref()
                        .map(|e| Box::new(e.to_expr().into_owned())),
                    name: name.as_ref().map(|e| Box::new(e.to_expr().into_owned())),
                    step: step.as_ref().map(|e| Box::new(e.to_expr().into_owned()))
                })
            },
            _ => todo!("Need to implement conversion  for {:?}", self)
        }
    }

    fn is_warning(&self) -> bool {
        todo!()
    }

    fn warning_token(&self) -> &Self {
        todo!()
    }

    fn warning_message(&self) -> &str {
        match &self {
            LocatedTokenInner::WarningWrapper(_token, message) => message.as_str(),
            _ => unreachable!()
        }
    }

    fn module_listing(&self) -> &[Self] {
        unimplemented!()
    }

    fn while_listing(&self) -> &[Self] {
        unreachable!()
    }

    fn mnemonic_arg1(&self) -> Option<&Self::DataAccess> {
        match &self {
            LocatedTokenInner::OpCode(_, arg1, ..) => arg1.as_ref(),
            _ => None
        }
    }

    fn rorg_listing(&self) -> &[Self] {
        todo!()
    }

    fn iterate_listing(&self) -> &[Self] {
        unreachable!()
    }

    fn for_listing(&self) -> &[Self] {
        todo!()
    }

    fn repeat_until_listing(&self) -> &[Self] {
        todo!()
    }

    fn repeat_listing(&self) -> &[Self] {
        todo!()
    }

    fn if_test(&self, _idx: usize) -> (&Self::TestKind, &[Self]) {
        unreachable!()
    }

    fn if_else(&self) -> Option<&[Self]> {
        unreachable!()
    }

    fn function_definition_inner(&self) -> &[Self] {
        todo!()
    }

    fn crunched_section_listing(&self) -> &[Self] {
        todo!()
    }

    fn is_confined(&self) -> bool {
        match &self {
            LocatedTokenInner::Confined(..) => true,
            _ => false
        }
    }

    fn confined_listing(&self) -> &[Self] {
        todo!()
    }

    fn switch_cases(&self) -> Box<dyn Iterator<Item = (&Self::Expr, &[Self], bool)> + '_> {
        todo!()
    }

    fn switch_default(&self) -> Option<&[Self]> {
        todo!()
    }

    fn repeat_counter_step(&self) -> Option<&Self::Expr> {
        match self {
            LocatedTokenInner::Repeat(_, _, _, _, step) => step.as_ref(),
            _ => unreachable!()
        }
    }

    fn assembler_control_command(&self) -> &Self::AssemblerControlCommand {
        match &self {
            LocatedTokenInner::AssemblerControl(cmd) => cmd,
            _ => unreachable!()
        }
    }

    fn defer_listing_output(&self) -> bool {
        false // self.is_equ() | self.is_set()
    }

    fn include_is_standard_include(&self) -> bool {
        dbg!("include_is_standard_include is no more accurate and needs to be updated/removed");

        self.is_include() &&
       /* !self.include_fname().contains('{') && */ // no expansion
        !self.include_once()
    }

    fn assembler_control_get_max_passes(&self) -> Option<&Self::Expr> {
        self.assembler_control_command().get_max_nb_passes()
    }

    fn assembler_control_get_listing(&self) -> &[Self] {
        unreachable!()
    }

    fn macro_flavor(&self) -> AssemblerFlavor {
        match self {
            LocatedTokenInner::Macro { flavor, .. } => *flavor,
            _ => unreachable!()
        }
    }
}

impl std::fmt::Display for LocatedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.span())
    }
}

impl ToSimpleToken for LocatedToken {
    fn as_simple_token(&self) -> Cow<'_, Token> {
        self.to_token()
    }
}

/// Trait to handle the span of listing elements
pub trait MayHaveSpan {
    fn possible_span(&self) -> Option<&Z80Span>;
    fn span(&self) -> &Z80Span;
    fn has_span(&self) -> bool;
}

impl<T> MayHaveSpan for Box<T>
where T: MayHaveSpan
{
    fn possible_span(&self) -> Option<&Z80Span> {
        (**self).possible_span()
    }

    fn span(&self) -> &Z80Span {
        (**self).span()
    }

    fn has_span(&self) -> bool {
        (**self).has_span()
    }
}

impl MayHaveSpan for Token {
    fn possible_span(&self) -> Option<&Z80Span> {
        None
    }

    fn span(&self) -> &Z80Span {
        panic!("A raw Token does not have a span")
    }

    fn has_span(&self) -> bool {
        false
    }
}

impl MayHaveSpan for Expr {
    fn possible_span(&self) -> Option<&Z80Span> {
        None
    }

    fn has_span(&self) -> bool {
        false
    }

    fn span(&self) -> &Z80Span {
        panic!()
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
        &self.span
    }
}

impl MayHaveSpan for &str {
    fn has_span(&self) -> bool {
        false
    }

    fn possible_span(&self) -> Option<&Z80Span> {
        None
    }

    fn span(&self) -> &Z80Span {
        unreachable!()
    }
}

impl MayHaveSpan for SmolStr {
    fn has_span(&self) -> bool {
        false
    }

    fn possible_span(&self) -> Option<&Z80Span> {
        None
    }

    fn span(&self) -> &Z80Span {
        unreachable!()
    }
}

impl MayHaveSpan for &SmolStr {
    fn has_span(&self) -> bool {
        false
    }

    fn possible_span(&self) -> Option<&Z80Span> {
        None
    }

    fn span(&self) -> &Z80Span {
        unreachable!()
    }
}

impl MayHaveSpan for &Z80Span {
    fn has_span(&self) -> bool {
        true
    }

    fn possible_span(&self) -> Option<&Z80Span> {
        Some(self)
    }

    fn span(&self) -> &Z80Span {
        self
    }
}

impl MayHaveSpan for Z80Span {
    fn has_span(&self) -> bool {
        true
    }

    fn possible_span(&self) -> Option<&Z80Span> {
        Some(self)
    }

    fn span(&self) -> &Z80Span {
        self
    }
}

impl Clone for LocatedToken {
    fn clone(&self) -> Self {
        unimplemented!()
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

// impl Deref for LocatedToken {
// type Target = Token;
//
// fn deref(&self) -> &Self::Target {
// match self.token() {
// Ok(t) => t,
// Err(_) => {
// panic!("{:?} cannot be dereferenced as it contains a listing", self)
// }
// }
// }
// }

impl LocatedToken {
    // We can obtain a token only for "standard ones". Those that rely on listing need to be handled differently
    // TODO remove that
    // pub fn token(&self) -> Result<&Token, ()> {
    // match self {
    // Self::Standard { token, .. } => Ok(token),
    // _ => Err(())
    // }
    // }
    pub fn context(&self) -> &ParserContext {
        self.span().context()
    }
}

impl LocatedToken {
    pub fn parse_token(_value: &str) -> Result<(), String> {
        unimplemented!("Should return a LocatedToken reference + its  LocatedListing")
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

    fn locate(self, span: Z80Span, size: usize) -> Self::Output;
}

impl<T> Locate for Box<T>
where T: Locate
{
    type Output = T::Output;

    fn locate(self, span: Z80Span, size: usize) -> Self::Output {
        (*self).locate(span, size)
    }
}
// /
// impl Locate for Token {
// type Output = LocatedToken;
//
// fn locate(self, span: Z80Span, size: usize) -> LocatedToken {
// if self.has_at_least_one_listing() {
// unreachable!()
// }
// else {
// LocatedToken::Standard {
// token: self,
// span: span.take(size)
// }
// }
// }
// }

impl TokenExt for LocatedToken {
    fn estimated_duration(&self) -> Result<usize, Box<AssemblerError>> {
        self.to_token().estimated_duration()
    }

    fn unroll(&self, _env: &mut crate::Env) -> Option<Result<Vec<&Self>, Box<AssemblerError>>> {
        todo!()
    }

    fn disassemble_data(&self) -> Result<cpclib_tokens::Listing, String> {
        todo!()
    }

    fn fallback_number_of_bytes(&self) -> Result<usize, String> {
        todo!()
    }
}

impl Deref for LocatedToken {
    type Target = LocatedTokenInner;

    fn deref(&self) -> &Self::Target {
        match &self.inner {
            either::Either::Left(inner) => inner,
            either::Either::Right((inner, _)) => inner.deref()
        }
    }
}

pub type InnerLocatedListing = BaseListing<LocatedToken>;

/// Represents a Listing of located tokens
/// Lifetimes 'src and 'ctx are in fact the same and correspond to hte lifetime of the object itself
#[derive(Eq)]
#[self_referencing]
pub struct LocatedListing {
    /// Its source code. We want it to live as long as possible.
    /// A string is copied for the very beginning of the file parsing, while a span is used for the inner blocs. As this field is immutable and build before the listing, we do not store the span here
    src: Option<std::sync::Arc<String>>,

    /// Its Parsing Context whose source targets LocatedListing
    #[borrows(src)]
    ctx: ParserContext,

    /// The real listing whose tokens come from src
    #[borrows(src, ctx)]
    pub(crate) parse_result: ParseResult
}

// impl ListingTrait for LocatedListing {
// type Element = LocatedToken;
//
// fn as_slice(&self) -> &[Self::Element] {
// self.with_parse_result(|result|{
// match result {
// ParseResult::SuccessComplete(listing) => listing,
// ParseResult::SuccessInner { listing, inner_span, next_span } => listing,
// ParseResult::FailureInner(_) => unreachable!(),
// ParseResult::FailureComplete(_) => unreachable!(),
// }
// })
// }
// }

impl PartialEq for LocatedListing {
    fn eq(&self, other: &Self) -> bool {
        self.borrow_src() == other.borrow_src()
    }
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParseResult {
    /// Assembling is successful for a complete file
    SuccessComplete(InnerLocatedListing),
    /// Assembling is successfull for an inner block inside a complete file
    SuccessInner {
        /// The real listing of LocatedTokens
        listing: InnerLocatedListing,
        /// The code of the inner block
        inner_span: Z80Span
    },
    FailureInner(ErrMode<Z80ParserError>),
    FailureComplete(AssemblerError) // TODO use Z80ParserError there
}

#[derive(Debug)]
pub(crate) enum _ParseResultFirstStage {
    Success {
        listing: Option<InnerLocatedListing>,
        remaining_span: Option<Z80Span>
    },
    Failure(Z80ParserError)
}

impl LocatedListing {
    /// Build the listing from the current code and context
    /// In case of error, the listing is provided as error message refer to its owned source code... FInal version should behave differently
    /// The listing embeds the error
    #[inline]
    pub fn new_complete_source<S: Into<String>>(
        code: S,
        builder: ParserContextBuilder
    ) -> Result<LocatedListing, LocatedListing> {
        // generate the listing
        let listing = LocatedListingBuilder {
            // source code is a string owned by the listing
            src: Some(code.into().into()),

            // context keeps a reference on the full listing (but is it really needed yet ?)
            ctx_builder: move |src: &Option<Arc<String>>| {
                let source = src
                    .as_ref()
                    .map(|arc| arc.deref())
                    .map(|s| s.as_str())
                    .map(|s| unsafe { &*(s as *const str) as &'static str })
                    .unwrap();
                builder.build(source)
            },

            // tokens depend both on the source and context. However source can be obtained from context so we do not use it here (it is usefull for the inner case)
            parse_result_builder: |_, ctx| {
                let src: &BStr = ctx.source;
                let input_start = Z80Span::new_extra(src, ctx);

                // really make the parsing
                let res: Result<
                    Vec<LocatedToken>,
                    cpclib_common::winnow::error::ParseError<
                        cpclib_common::winnow::Stateful<
                            cpclib_common::winnow::stream::LocatingSlice<&BStr>,
                            &ParserContext
                        >,
                        Z80ParserError
                    >
                > = parse_lines.parse(input_start.0);

                // analyse result and can generate error even if parsing was ok
                // Ok only if everything is parsed
                let res: Result<InnerLocatedListing, Z80ParserError> = match res {
                    Ok(tokens) => {
                        // no more things to assemble
                        Ok(InnerLocatedListing::from(tokens))
                    },
                    Err(e) => {
                        // Propagate the error (that is located)
                        let e = e.into_inner();
                        std::result::Result::Err(e)
                    }
                };

                // Build the result

                match res {
                    Ok(listing) => ParseResult::SuccessComplete(listing),
                    Err(e) => ParseResult::FailureComplete(AssemblerError::SyntaxError { error: e })
                }
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
    #[inline]
    pub fn parse_inner(
        input_code: &mut InnerZ80Span,
        new_state: ParsingState,
        only_one_instruction: bool
    ) -> ModalResult<Arc<LocatedListing>, Z80ParserError> {
        let mut tokens = Vec::with_capacity(20);

        let ctx_moved_in_builder = input_code.state.clone_with_state(new_state);

        // we do not change ctx.source that must be the very same than the parent
        //       let input_fragment = input_code.fragment();
        //       ctx.source = Some(input_fragment);

        let inner_listing = LocatedListingBuilder {
            // No need to specify an input as it is already embedded in the parent listing
            src: None,

            // Context source has already been provided before. Its state as also been properly set
            ctx_builder: move |_src| {
                // we ignore the provided code
                ctx_moved_in_builder
            },

            parse_result_builder: |_src, lst_ctx| {
                // build a span with the appropriate novel context
                let lst_ctx =
                    unsafe { &*(lst_ctx as *const ParserContext) as &'static ParserContext }; // the context is stored within the object; so it is safe to set its lifetime to static

                let src = unsafe { &*( std::str::from_utf8_unchecked(input_code.as_bstr()) as *const str) } as &'static str;

                // Build the span that will be parsed to collect inner tokens.
                // It has a length of input_length.
                let mut inner_span = Z80Span::new_extra(
                    src,
                    lst_ctx
                );
                let inner_code_ptr = &mut inner_span;
                /*
                (unsafe {
                    LocatedSpan::new_from_raw_offset(
                        input_code.location_offset(),
                        input_code.location_line(),
                        &*(input_code.as_str() as *const str) as &'static str,
                        lst_ctx
                    )
                });
                */
                // keep a track of the very beginning of the span
                let inner_start = inner_code_ptr.checkpoint();

                let res = if only_one_instruction {
                    match parse_single_token.parse_next(inner_code_ptr) {
                        Ok(token) => {
                            tokens.push(token);
                            Ok(())
                        }
                        Err(e) => {
                            Err(e)
                        }
                    }
                } else {
                    cut_err(my_many0_nocollect(parse_z80_line_complete(&mut tokens))).parse_next(
                        inner_code_ptr
                    )
                };
                match res {
                    Ok(_) => {
                        let inner_length = inner_code_ptr.offset_from(&inner_start);
                        let inner_span: &'static BStr = unsafe{std::mem::transmute(&input_code.as_bstr().as_bytes()[..inner_length])}; // remove the bytes eaten by the inner parser

                        let inner_span = (*input_code).update_slice(inner_span);

                        take::<_,_, Z80ParserError>(inner_length).parse_next(input_code).expect("BUG in parser"); // really consume from the input

                        ParseResult::SuccessInner {
                            inner_span: inner_span.into(),
                            listing: InnerLocatedListing::from(tokens)
                        }
                    },
                    Err(e) => ParseResult::FailureInner(e)
                }
            }
        }
        .build();

        let inner_listing = Arc::new(inner_listing);

        if let ParseResult::SuccessInner { .. } = inner_listing.borrow_parse_result() {
            return Ok(inner_listing);
        }

        if let ParseResult::FailureInner(e) = inner_listing.borrow_parse_result() {
            match e {
                ErrMode::Incomplete(e) => {
                    return Err(ErrMode::Incomplete(*e));
                },
                ErrMode::Backtrack(e) => {
                    return Err(ErrMode::Backtrack(Z80ParserError::from_inner_error(
                        input_code,
                        inner_listing.clone(),
                        Box::new(e.clone())
                    )));
                },
                ErrMode::Cut(e) => {
                    return Err(ErrMode::Cut(Z80ParserError::from_inner_error(
                        input_code,
                        inner_listing.clone(),
                        Box::new(e.clone())
                    )));
                }
            }
        }

        unreachable!();
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
                },
                ParseResult::SuccessInner { inner_span, .. } => inner_span.clone(),
                _ => panic!("No listing available")
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
        #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
        let iter = self.deref().par_iter();
        #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
        let iter = self.deref().iter();

        iter.map(|lt| lt.to_token())
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
        let tokens = {
            let res = parse_z80_str(src);
            match res {
                Ok(tokens) => tokens,
                Err(_e) => {
                    return Err("ERROR -- need to code why ...".to_owned());
                }
            }
        };
        match tokens.len() {
            0 => Err("No ASM found.".to_owned()),
            1 => Ok(tokens[0].to_token().into_owned()),
            _ => {
                Err(format!(
                    "{} tokens are present instead of one",
                    tokens.len()
                ))
            },
        }
    }
}

impl ListingExt for LocatedListing {
    fn add_code<S: AsRef<str> + core::fmt::Display>(
        &mut self,
        _code: S
    ) -> Result<(), Box<AssemblerError>> {
        panic!("Cannot be used in this context");
    }

    fn to_bytes_with_options(
        &self,
        options: crate::assembler::EnvOptions
    ) -> Result<Vec<u8>, Box<AssemblerError>> {
        let (_, env) =
            crate::assembler::visit_tokens_all_passes_with_options(self.listing(), options)
                .map_err(|(_, _, e)| AssemblerError::AlreadyRenderedError(e.to_string()))?;
        Ok(env.produced_bytes())
    }

    fn estimated_duration(&self) -> Result<usize, Box<AssemblerError>> {
        todo!()
    }

    fn to_string(&self) -> String {
        todo!()
    }

    fn to_enhanced_string(&self) -> String {
        todo!()
    }

    fn inject_labels<S: Borrow<str>>(&mut self, _labels: HashMap<u16, S>) {
        todo!()
    }

    fn fallback_number_of_bytes(&self) -> Result<usize, String> {
        todo!()
    }
}

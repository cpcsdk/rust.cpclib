use std::borrow::Cow;
use std::fmt::Display;
use std::ops::Deref;
use std::sync::Arc;

use cpclib_common::itertools::Itertools;
use cpclib_common::nom::combinator::{cut, eof};
use cpclib_common::nom::error::{context, ErrorKind, VerboseError};
use cpclib_common::nom::{Err, IResult, InputLength, InputTake};
use cpclib_common::nom_locate::LocatedSpan;
#[cfg(not(target_arch = "wasm32"))]
use cpclib_common::rayon::prelude::*;
use cpclib_common::smallvec::SmallVec;
use cpclib_sna::{SnapshotVersion, SnapshotFlag, FlagValue};
use cpclib_tokens::ordered_float::OrderedFloat;
use cpclib_tokens::{
    data_access_impl_most_methods,
    data_access_is_any_indexregister16, data_access_is_any_indexregister8,
    data_access_is_any_register16, data_access_is_any_register8, BaseListing, BinaryFunction,
    BinaryOperation, CharsetFormat, CrunchType, DataAccess, DataAccessElem, Expr, ExprResult,
    FlagTest, FormattedExpr, IndexRegister16, IndexRegister8, LabelPrefix, ListingElement,
    MacroParam, MacroParamElement, Mnemonic, Register16, Register8, TestKind, TestKindElement, UnaryTokenOperation, UnaryOperation, UnaryFunction, StableTickerAction, SaveType
};
use cpclib_tokens::ToSimpleToken;
use cpclib_tokens::Token;
use ouroboros::self_referencing;

use super::{
    my_many0_nocollect, my_many_till_nocollect, parse_z80_line_complete, ParserContext,
    Z80ParserError, Z80Span
};
use crate::assembler::Env;
use crate::error::AssemblerError;
/// ! This crate is related to the adaptation of tokens and listing for the case where they are parsed
use crate::error::ExpressionError;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::tokens::TokenExt;
use crate::preamble::parse_z80_str;
use crate::{
    resolve_impl, BinaryTransformation, ExprElement, ParserContextBuilder, ParsingState, SymbolFor
};

#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn to_expr(&self) -> Cow<Expr> {
        let expr = match self {
            LocatedExpr::RelativeDelta(d, _) => Expr::RelativeDelta(*d),
            LocatedExpr::Value(v, _) => Expr::Value(*v),
            LocatedExpr::Float(f, _) => Expr::Float(*f),
            LocatedExpr::Char(c, _) => Expr::Char(*c),
            LocatedExpr::Bool(b, _) => Expr::Bool(*b),
            LocatedExpr::String(s) => Expr::String(s.into()),
            LocatedExpr::Label(l) => Expr::Label(l.into()),
            LocatedExpr::List(l, _) => {
                Expr::List(l.iter().map(|e| e.to_expr().into_owned()).collect_vec())
            }
            LocatedExpr::PrefixedLabel(p, l, _) => Expr::PrefixedLabel(*p, l.into()),
            LocatedExpr::Paren(box p, _) => Expr::Paren(Box::new(p.to_expr().into_owned())),
            LocatedExpr::UnaryFunction(f, box e, _) => {
                Expr::UnaryFunction(*f, Box::new(e.to_expr().into_owned()))
            }
            LocatedExpr::UnaryOperation(o, box e, _) => {
                Expr::UnaryOperation(*o, Box::new(e.to_expr().into_owned()))
            }
            LocatedExpr::UnaryTokenOperation(o, box t, _) => {
                Expr::UnaryTokenOperation(*o, Box::new(t.to_token().into_owned()))
            }
            LocatedExpr::BinaryFunction(f, box e1, box e2, _) => {
                Expr::BinaryFunction(
                    *f,
                    Box::new(e1.to_expr().into_owned()),
                    Box::new(e2.to_expr().into_owned())
                )
            }
            LocatedExpr::BinaryOperation(o, box e1, box e2, _) => {
                Expr::BinaryOperation(
                    *o,
                    Box::new(e1.to_expr().into_owned()),
                    Box::new(e2.to_expr().into_owned())
                )
            }
            LocatedExpr::AnyFunction(n, a, _) => {
                Expr::AnyFunction(
                    n.into(),
                    a.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                )
            }
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
            Self::String(v) => v.as_str(),
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

    fn is_unary_function(&self) -> bool {
        match self {
            Self::UnaryFunction(..) => true,
            _ => false
        }
    }

    fn unary_function(&self) -> UnaryFunction {
        match self {
            Self::UnaryFunction(f, ..) => *f,
            _ => unreachable!()
        }
    }

    fn is_binary_function(&self) -> bool {
        match self {
            Self::BinaryFunction(..) => true,
            _ => false
        }
    }

    fn binary_function(&self) -> BinaryFunction {
        match self {
            Self::BinaryFunction(f, ..) => *f,
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
            Self::UnaryFunction(_f, ..) => todo!(),
            Self::BinaryFunction(_f, ..) => todo!(),
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
            Self::UnaryFunction(_, box arg, _) => arg,
            Self::BinaryFunction(_, box arg1, ..) => arg1,
            Self::Paren(box p, _) => p,

            _ => unreachable!()
        }
    }

    fn arg2(&self) -> &Self {
        match self {
            Self::BinaryOperation(_, _, box arg2, _) => arg2,
            Self::BinaryFunction(_, _, box arg2, _) => arg2,

            _ => unreachable!()
        }
    }
}

impl ExprEvaluationExt for LocatedExpr {
    /// Resolve by adding localisation in case of error
    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        resolve_impl!(self, env)
            .map_err(|e| e.locate(self.span().clone()))
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

            LocatedExpr::UnaryTokenOperation(_, box _t, _) => {
                unimplemented!("Need to retreive the symbols from the operation")
            }
        }
    }
}

impl LocatedExpr {
    pub fn span(&self) -> &Z80Span {
        match self {
            LocatedExpr::RelativeDelta(_, span)
            | LocatedExpr::Value(_, span)
            | LocatedExpr::Float(_, span)
            | LocatedExpr::Char(_, span)
            | LocatedExpr::Bool(_, span)
            | LocatedExpr::String(span)
            | LocatedExpr::Label(span)
            | LocatedExpr::List(_, span)
            | LocatedExpr::PrefixedLabel(_, _, span)
            | LocatedExpr::Paren(_, span)
            | LocatedExpr::UnaryFunction(_, _, span)
            | LocatedExpr::UnaryOperation(_, _, span)
            | LocatedExpr::UnaryTokenOperation(_, _, span)
            | LocatedExpr::BinaryFunction(_, _, _, span)
            | LocatedExpr::BinaryOperation(_, _, _, span)
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

impl Into<DataAccess> for LocatedDataAccess {
    fn into(self) -> DataAccess {
        self.to_data_access()
    }
}

impl LocatedDataAccess {
    pub fn to_data_access(self) -> DataAccess {
        match self {
            LocatedDataAccess::IndexRegister16WithIndex(r, b, e, _) => {
                DataAccess::IndexRegister16WithIndex(r, b, e.to_expr().into_owned())
            }
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

impl Into<Expr> for LocatedExpr {
    fn into(self) -> Expr {
        self.to_expr().into_owned()
    }
}

impl DataAccessElem for LocatedDataAccess {
    type Expr = LocatedExpr;

    data_access_impl_most_methods!();

    fn to_data_access_for_low_register(&self) -> Option<Self> {
        match self {
            Self::IndexRegister16(reg, span) => Some(LocatedDataAccess::IndexRegister8(reg.low(), span.clone())),
            Self::Register16(reg, span) => 
                reg.low()
                    .map(|reg| {
                        LocatedDataAccess::Register8(reg, span.clone())
                    }),
            _ => None
        }
    }

    fn to_data_access_for_high_register(&self) -> Option<Self> {
        match self {
            Self::IndexRegister16(reg, span) => Some(LocatedDataAccess::IndexRegister8(reg.high(), span.clone())),
            Self::Register16(reg, span) =>  
            reg.high()
                .map(|reg| {
                    LocatedDataAccess::Register8(reg, span.clone())
                }),
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

    fn to_data_access(&self) -> Cow<DataAccess> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
                MacroParam::List(
                    params
                        .iter()
                        .map(|p| p.to_macro_param())
                        .map(|p| Box::new(p))
                        .collect_vec()
                )
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

// Encode the LocatedToken BEFORE computing its span
#[derive(Debug, PartialEq, Eq)]
pub enum LocatedTokenInner {
    Align(LocatedExpr, Option<LocatedExpr>),
    Assert(LocatedExpr, Option<Vec<FormattedExpr>>),
    Assign {
        label: Z80Span,
        expr: LocatedExpr,
        op: Option<BinaryOperation>
    },

    Bank(Option<LocatedExpr>),
    Bankset(LocatedExpr),
    Basic(Option<Vec<Z80Span>>, Option<Vec<u16>>, Z80Span),
    Break,
    Breakpoint(Option<LocatedExpr>),
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
    Export(Vec<Z80Span>),

    Fail(Option<Vec<FormattedExpr>>),
    For {
        label: Z80Span,
        start: LocatedExpr,
        stop: LocatedExpr,
        step: Option<LocatedExpr>,
        listing: LocatedListing
    },
    Function(Z80Span, Vec<Z80Span>, LocatedListing),
    If(
        Vec<(LocatedTestKind, LocatedListing)>,
        Option<LocatedListing>
    ),
    Incbin {
        fname: Z80Span,
        offset: Option<LocatedExpr>,
        length: Option<LocatedExpr>,
        extended_offset: Option<LocatedExpr>,
        off: bool,
        transformation: BinaryTransformation
    },
    Include(Z80Span, Option<Z80Span>, bool),
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
        content: Z80Span
    },
    /// Name, Parameters, FullSpan
    MacroCall(Z80Span, Vec<LocatedMacroParam>),
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
    Pause,
    Print(Vec<FormattedExpr>),
    Protect(LocatedExpr, LocatedExpr),

    Range(Z80Span, LocatedExpr, LocatedExpr),
    Repeat(
        LocatedExpr,
        LocatedListing,
        Option<Z80Span>,
        Option<LocatedExpr>
    ),
    RepeatUntil(LocatedExpr, LocatedListing),
    Return(LocatedExpr),
    Rorg(LocatedExpr, LocatedListing),
    Run(LocatedExpr, Option<LocatedExpr>),

    Save{
        filename: Z80Span,
        address: Option<LocatedExpr>,
        size: Option<LocatedExpr>,
        save_type: Option<SaveType>,
        dsk_filename: Option<Z80Span>,
        side: Option<LocatedExpr>
    },
    Section(Z80Span),
    SetN {
        label: Z80Span,
        source: Z80Span,
        expr: Option<LocatedExpr>
    },
    SnaInit(Z80Span),
    SnaSet(SnapshotFlag, FlagValue),
    StableTicker(StableTickerAction),
    Str(Vec<LocatedExpr>),
    Struct(Z80Span, Vec<(Z80Span, LocatedToken)>),
    Switch(
        LocatedExpr,
        Vec<(LocatedExpr, LocatedListing, bool)>,
        Option<LocatedListing>
    ),
    Undef(Z80Span),

    WaitNops(LocatedExpr),
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
            Self::Label(span) | Self::Comment(span) => {
                span.clone()
            },
            
            _ => todo!("not coded yet or impossible {:?}", self)
        };

        LocatedToken {
            inner: either::Either::Left(self),
            span,
        }
    }


    pub fn into_located_token_at(self, span: &Z80Span) -> LocatedToken {
        match self {
            Self::WarningWrapper(token,msg) => {
                let warned = Box::new(token.into_located_token_at(span));
                LocatedToken { inner: either::Right((warned, msg)), span: span.clone() }

            }

            _ => {
                LocatedToken { inner: either::Either::Left(self), span: span.clone()}
            }
        }
    }

    pub fn into_located_token_between(self, start: &Z80Span, stop: &Z80Span) -> LocatedToken {
        let count = start.input_len() - stop.input_len();
        let span = start.take(count);

        self.into_located_token_at(&span)
        
    }

 
}

/// Add span information for a Token.
#[derive(Debug, PartialEq, Eq)]
pub struct LocatedToken {
    // The token of interest of a warning with the token of interest
    pub(crate) inner: either::Either<LocatedTokenInner, (Box<LocatedToken>, String)>,
    pub(crate) span: Z80Span,
}

impl ListingElement for LocatedToken {
    type DataAccess = LocatedDataAccess;
    type Expr = LocatedExpr;
    type MacroParam = LocatedMacroParam;
    type TestKind = LocatedTestKind;

   
   fn to_token(&self) -> Cow<cpclib_tokens::Token> {
        match &self.inner {
            either::Either::Left(inner) => inner.to_token(),
            either::Either::Right((inner, msg)) => {
                unimplemented!("is it necessary to implement it ?")
            }
        }
   }

    fn is_equ(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Equ { .. }) => true,
            _ => false
        }
    }

    fn equ_symbol(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Equ { label, .. }) => label.as_str(),
            _ => unreachable!()
        }
    }

    fn equ_value(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::Equ { expr, .. }) => expr,
            _ => unreachable!()
        }
    }

    fn is_warning(&self) -> bool {
       self.inner.is_right()
    }


    fn warning_message(&self) -> &str {
        match &self.inner {
            either::Right((inner, msg)) => msg.as_str(),
            _ => unreachable!()
        }
    }

    fn is_module(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Module(..)) => true,
            _ => false
        }
    }

    fn module_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Module(_, lst, ..)) => lst,
            _ => unreachable!()
        }
    }

    fn module_name(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Module(name, ..)) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn is_while(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::While(..)) => true,
            _ => false
        }
    }

    fn while_expr(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::While(expr, ..)) => expr,
            _ => unreachable!()
        }
    }

    fn while_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::While(_, lst, ..)) => lst,
            _ => unreachable!()
        }
    }

    fn mnemonic(&self) -> Option<&Mnemonic> {
        match &self.inner {
            either::Left(LocatedTokenInner::OpCode(mne, ..)) => Some(mne),
            _ => None
        }
    }

    fn mnemonic_arg1(&self) -> Option<&Self::DataAccess> {
        match &self.inner {
            either::Left(LocatedTokenInner::OpCode(_, arg1, ..)) => arg1.as_ref(),
            _ => None
        }
    }

    fn mnemonic_arg2(&self) -> Option<&Self::DataAccess> {
        match &self.inner {
            either::Left(LocatedTokenInner::OpCode(_, _, arg2, _)) => arg2.as_ref(),
            _ => None
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

    fn is_directive(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::OpCode(..)) => false,
            _ => true
        }
    }

    fn is_rorg(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Rorg(..)) => true,
            _ => false
        }
    }

    fn rorg_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Rorg(_, lst)) => lst,
            _ => unreachable!()
        }
    }

    fn rorg_expr(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::Rorg(exp, ..)) => exp,
            _ => unreachable!()
        }
    }

    fn is_iterate(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Iterate(..)) => true,
            _ => false
        }
    }

    fn iterate_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Iterate(_, _, listing, ..)) => listing,
            _ => unreachable!()
        }
    }

    fn iterate_counter_name(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Iterate(name, ..)) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn iterate_values(&self) -> either::Either<&Vec<Self::Expr>, &Self::Expr> {
        match &self.inner {
            either::Left(LocatedTokenInner::Iterate(_, values, ..)) => values.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_for(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::For { .. }) => true,
            _ => false
        }
    }

    fn for_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::For { listing, .. }) => listing,
            _ => unreachable!()
        }
    }

    fn for_label(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::For { label, .. }) => label.as_ref(),
            _ => unreachable!()
        }
    }

    fn for_start(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::For { start, .. }) => start,
            _ => unreachable!()
        }
    }

    fn for_stop(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::For { stop, .. }) => stop,
            _ => unreachable!()
        }
    }

    fn for_step(&self) -> Option<&Self::Expr> {
        match &self.inner {
            either::Left(LocatedTokenInner::For { step, .. }) => step.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_repeat_until(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::RepeatUntil(..)) => true,
            _ => false
        }
    }

    fn repeat_until_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::RepeatUntil(_, code, ..)) => code,
            _ => unreachable!()
        }
    }

    fn repeat_until_condition(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::RepeatUntil(cond, ..)) => cond,
            _ => unreachable!()
        }
    }

    fn is_repeat(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Repeat(..)) => true,
            _ => false
        }
    }

    fn repeat_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Repeat(_, listing, ..)) => listing,
            _ => unreachable!()
        }
    }

    fn repeat_count(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::Repeat(e, ..)) => e,
            _ => unreachable!()
        }
    }

    fn repeat_counter_name(&self) -> Option<&str> {
        match &self.inner {
            either::Left( LocatedTokenInner::Repeat(_, _, counter_name, ..)) => counter_name.as_ref().map(|c| c.as_str()),
            _ => unreachable!()
        }
    }

    fn repeat_counter_start(&self) -> Option<&Self::Expr> {
        match &self.inner {
            either::Left(LocatedTokenInner::Repeat(_, _, _, start)) => start.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_macro_definition(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Macro { .. }) => true,
            _ => false
        }
    }

    fn macro_definition_name(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Macro { name, .. }) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn macro_definition_arguments(&self) -> SmallVec<[&str; 4]> {
        match &self.inner {
            either::Left(LocatedTokenInner::Macro { params, .. }) => params.iter().map(|a| a.as_str()).collect(),
            _ => unreachable!()
        }
    }

    fn macro_definition_code(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Macro { content, .. }) => content.as_str(),
            _ => unreachable!()
        }
    }

    fn macro_call_name(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::MacroCall(name, ..)) => name.as_str(),
            _ => panic!()
        }
    }

    fn macro_call_arguments(&self) -> &[Self::MacroParam] {
        match &self.inner {
            either::Left(LocatedTokenInner::MacroCall(_, args)) => args,
            _ => panic!()
        }
    }

    fn is_if(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::If(..)) => true,
            _ => false
        }
    }

    fn if_nb_tests(&self) -> usize {
        match &self.inner {
            either::Left(LocatedTokenInner::If(tests, ..)) => tests.len(),
            _ => panic!()
        }
    }

    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]) {
        match &self.inner {
            either::Left(LocatedTokenInner::If(tests, ..)) => {
                let data = &tests[idx];
                (&data.0, &data.1)
            }
            _ => panic!()
        }
    }

    fn if_else(&self) -> Option<&[Self]> {
        match &self.inner {
            either::Left(LocatedTokenInner::If(_, r#else)) => r#else.as_ref().map(|l| l.as_slice()),
            _ => panic!()
        }
    }

    fn is_include(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Include(..)) => true,
            _ => false
        }
    }

    fn is_incbin(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Incbin { .. }) => true,
            _ => false
        }
    }

    fn incbin_fname(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Incbin { fname, .. }) => fname,
            _ => unimplemented!()
        }
    }

    fn incbin_offset(&self) -> Option<&Self::Expr> {
        match &self.inner {
            either::Left(LocatedTokenInner::Incbin { offset, .. }) => offset.as_ref(),
            _ => unimplemented!()
        }
    }

    fn incbin_length(&self) -> Option<&Self::Expr> {
        match &self.inner {
            either::Left(LocatedTokenInner::Incbin { length, .. }) => length.as_ref(),
            _ => unimplemented!()
        }
    }

    fn incbin_transformation(&self) -> &cpclib_tokens::BinaryTransformation {
        match &self.inner {
            either::Left(LocatedTokenInner::Incbin { transformation, .. }) => transformation,
            _ => unimplemented!()
        }
    }

    fn include_fname(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Include(fname, ..)) => fname,
            _ => unreachable!()
        }
    }

    fn include_namespace(&self) -> Option<&str> {
        match &self.inner {
            either::Left(LocatedTokenInner::Include(_, module, ..)) => module.as_ref().map(|s| s.as_str()),
            _ => unreachable!()
        }
    }

    fn include_once(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Include(_, _, once)) => *once,
            _ => unreachable!()
        }
    }

    fn is_call_macro_or_build_struct(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::MacroCall(..)) => true,
            _ => false
        }
    }

    fn is_function_definition(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Function(..)) => true,
            _ => false
        }
    }

    fn function_definition_name(&self) -> &str {
        match &self.inner {
            either::Left(LocatedTokenInner::Function(name, ..)) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn function_definition_params(&self) -> SmallVec<[&str; 4]> {
        match &self.inner {
            either::Left(LocatedTokenInner::Function(_name, params, ..)) => params.iter().map(|v| v.as_str()).collect(),
            _ => unreachable!()
        }
    }

    fn function_definition_inner(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Function(_, _, inner)) => inner,
            _ => unreachable!()
        }
    }

    fn is_crunched_section(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::CrunchedSection(..)) => true,
            _ => false
        }
    }

    fn crunched_section_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::CrunchedSection(_, lst)) => lst,
            _ => unreachable!()
        }
    }

    fn crunched_section_kind(&self) -> &CrunchType {
        match &self.inner {
            either::Left(LocatedTokenInner::CrunchedSection(kind, ..)) => kind,
            _ => unreachable!()
        }
    }

    fn is_confined(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Confined(..)) => true,
            _ => false
        }
    }

    fn confined_listing(&self) -> &[Self] {
        match &self.inner {
            either::Left(LocatedTokenInner::Confined(lst)) => lst,
            _ => unreachable!()
        }
    }

    fn is_switch(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Switch(..)) => true,
            _ => false
        }
    }

    fn switch_expr(&self) -> &Self::Expr {
        match &self.inner {
            either::Left(LocatedTokenInner::Switch(expr, ..)) => expr,
            _ => unreachable!()
        }
    }

    fn switch_cases(&self) -> Box<dyn Iterator<Item = (&Self::Expr, &[Self], bool)> + '_> {
        match &self.inner {
            either::Left(LocatedTokenInner::Switch(_, cases, ..)) => {
                Box::new(cases.iter().map(|c| (&c.0, c.1.as_slice(), c.2)))
            }
            _ => unreachable!()
        }
    }

    fn switch_default(&self) -> Option<&[Self]> {
        match &self.inner {
            either::Left(LocatedTokenInner::Switch(_, _, default, ..)) => default.as_ref().map(|l| l.as_slice()),
            _ => unreachable!()
        }
    }

    fn is_db(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Defb(..)) => true,
            _ => false
        }
    }

    fn is_dw(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Defw(..)) => true,
            _ => false
        }
    }

    fn is_str(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Str(..)) => true,
            _ => false
        }
    }

    fn data_exprs(&self) -> &[Self::Expr] {
        match &self.inner {
            either::Left(LocatedTokenInner::Defb(e, ..)) | either::Left(LocatedTokenInner::Defw(e, ..)) | either::Left(LocatedTokenInner::Str(e, ..)) => e,
            _ => unreachable!()
        }
    }

    fn is_set(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Assign { .. }) => true,
            _ => false
        }
    }

    fn is_comment(&self) -> bool {
        match &self.inner {
            either::Left(LocatedTokenInner::Comment(..)) => true,
            _ => false
        }
    }

    fn warning_token(&self) -> &Self {
        match &self.inner {
            either::Either::Left(_) => unreachable!(),
            either::Either::Right((inner, msg)) => inner,
        }
    }
}



// Several methodsare not implemented because their return type is wrong
// it soes not really matter because we never have to call them
impl ListingElement for LocatedTokenInner {
    type DataAccess = LocatedDataAccess;
    type Expr = LocatedExpr;
    type MacroParam = LocatedMacroParam;
    type TestKind = LocatedTestKind;


        /// Transform the located token in a raw token.
    /// Warning, this is quite costly when strings or vec are involved
    fn to_token(&self) -> Cow<Token> {
        match self {
            Self::OpCode(mne, arg1, arg2, arg3) => {
                Cow::Owned(Token::OpCode(
                    *mne,
                    arg1.as_ref().map(|d| d.to_data_access().into_owned()),
                    arg2.as_ref().map(|d| d.to_data_access().into_owned()),
                    arg3.clone()
                ))
            }
            Self::Comment(cmt) => Cow::Owned(Token::Comment(cmt.to_string())),
            Self::Org { val1, val2 } => {
                Cow::Owned(Token::Org{
                    val1: val1.to_expr().into_owned(),
                    val2: val2.as_ref().map(|val2| val2.to_expr().into_owned())
            })
            }
            Self::CrunchedSection(c, l) => Cow::Owned(Token::CrunchedSection(*c, l.as_listing())),
            Self::Function(name, params, inner) => {
                Cow::Owned(Token::Function(
                    name.into(),
                    params.iter().map(|p| p.into()).collect_vec(),
                    inner.as_listing()
                ))
            }
            Self::If(v, e) => {
                Cow::Owned(Token::If(
                    v.iter()
                        .map(|(k, l)| (k.to_test_kind(), l.as_listing()))
                        .collect_vec(),
                    e.as_ref().map(|l| l.as_listing())
                ))
            }
            Self::Repeat(e, l, s, start) => {
                Cow::Owned(Token::Repeat(
                    e.to_expr().into_owned(),
                    l.as_listing(),
                    s.as_ref().map(|s| s.into()),
                    start.as_ref().map(|e| e.to_expr().into_owned())
                ))
            }
            Self::RepeatUntil(e, l) => {
                Cow::Owned(Token::RepeatUntil(e.to_expr().into_owned(), l.as_listing()))
            }
            Self::Rorg(e, l) => Cow::Owned(Token::Rorg(e.to_expr().into_owned(), l.as_listing())),
            Self::Switch(v, c, d) => {
                Cow::Owned(Token::Switch(
                    v.to_expr().into_owned(),
                    c.iter()
                        .map(|(e, l, b)| (e.to_expr().into_owned(), l.as_listing(), b.clone()))
                        .collect_vec(),
                    d.as_ref().map(|d| d.as_listing())
                ))
            }
            Self::While(e, l) => Cow::Owned(Token::While(e.to_expr().into_owned(), l.as_listing())),
            Self::Iterate(_name, _values, _code) => {
                todo!()
            }
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
                    start: start.to_expr().into_owned(),
                    stop: stop.to_expr().into_owned(),
                    step: step.as_ref().map(|e| e.to_expr().into_owned()),
                    listing: listing.as_listing()
                })
            }
            Self::Label(label) => Cow::Owned(Token::Label(label.into())),
            Self::MacroCall(name, params) => {
                Cow::Owned(Token::MacroCall(
                    name.into(),
                    params.iter().map(|p| p.to_macro_param()).collect_vec()
                ))
            }
            Self::Struct(name, params) => {
                Cow::Owned(Token::Struct(
                    name.into(),
                    params
                        .iter()
                        .map(|(label, p)| (label.into(), p.as_simple_token().into_owned()))
                        .collect_vec()
                ))
            }
            Self::Defb(exprs) => {
                Cow::Owned(Token::Defb(
                    exprs.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                ))
            }
            Self::Defw(exprs) => {
                Cow::Owned(Token::Defw(
                    exprs.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                ))
            }
            Self::Str(exprs) => {
                Cow::Owned(Token::Str(
                    exprs.iter().map(|e| e.to_expr().into_owned()).collect_vec()
                ))
            }

            Self::Include(..) => todo!(),
            Self::Incbin {
                fname: _,
                offset: _,
                length: _,
                extended_offset: _,
                off: _,
                transformation: _
            } => todo!(),
            Self::Macro {
                name,
                params,
                content
            } => {
                Cow::Owned(Token::Macro{
                   name: name.into(),
                   params: params.iter().map(|p| p.into()).collect_vec(),
                  content:  content.as_str().to_owned()
            })
            }
            Self::Confined(..) => todo!(),
            Self::WarningWrapper(..) => todo!(),
            Self::Assign {
                label: _,
                expr: _,
                op: _
            } => todo!(),
            Self::Equ { label: _, expr: _ } => todo!(),
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

            _ => todo!()
        }
    }


    fn is_equ(&self) -> bool {
        match &self{
            LocatedTokenInner::Equ { .. } => true,
            _ => false
        }
    }

    fn equ_symbol(&self) -> &str {
        match &self{
            LocatedTokenInner::Equ { label, .. } => label.as_str(),
            _ => unreachable!()
        }
    }

    fn equ_value(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::Equ { expr, .. } => expr,
            _ => unreachable!()
        }
    }

    fn is_warning(&self) -> bool {
        todo!()
    }

    fn warning_token(&self) -> &Self {
        todo!()
    }

    fn warning_message(&self) -> &str {
        match &self{
            LocatedTokenInner::WarningWrapper(_token, message) => message.as_str(),
            _ => unreachable!()
        }
    }

    fn is_module(&self) -> bool {
        match &self{
            LocatedTokenInner::Module(..) => true,
            _ => false
        }
    }

    fn module_listing(&self) -> &[Self] {
        todo!()
    }

    fn module_name(&self) -> &str {
        match &self{
            LocatedTokenInner::Module(name, ..) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn is_while(&self) -> bool {
        match &self{
            LocatedTokenInner::While(..) => true,
            _ => false
        }
    }

    fn while_expr(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::While(expr, ..) => expr,
            _ => unreachable!()
        }
    }

    fn while_listing(&self) -> &[Self] {
        todo!()
    }

    fn mnemonic(&self) -> Option<&Mnemonic> {
        match &self{
            LocatedTokenInner::OpCode(mne, ..) => Some(mne),
            _ => None
        }
    }

    fn mnemonic_arg1(&self) -> Option<&Self::DataAccess> {
        match &self{
            LocatedTokenInner::OpCode(_, arg1, ..) => arg1.as_ref(),
            _ => None
        }
    }

    fn mnemonic_arg2(&self) -> Option<&Self::DataAccess> {
        match &self{
            LocatedTokenInner::OpCode(_, _, arg2, _) => arg2.as_ref(),
            _ => None
        }
    }

    fn mnemonic_arg1_mut(&mut self) -> Option<&mut Self::DataAccess> {
        match self{
            LocatedTokenInner::OpCode(_, arg1, ..) => arg1.as_mut(),

            _ => None
        }
    }

    fn mnemonic_arg2_mut(&mut self) -> Option<&mut Self::DataAccess> {
        match self{
            LocatedTokenInner::OpCode(_, _, arg2, _) => arg2.as_mut(),

            _ => None
        }
    }

    fn is_directive(&self) -> bool {
        match &self{
            LocatedTokenInner::OpCode(..) => false,
            _ => true
        }
    }

    fn is_rorg(&self) -> bool {
        match &self{
            LocatedTokenInner::Rorg(..) => true,
            _ => false
        }
    }

    fn rorg_listing(&self) -> &[Self] {
        todo!()
    }

    fn rorg_expr(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::Rorg(exp, ..) => exp,
            _ => unreachable!()
        }
    }

    fn is_iterate(&self) -> bool {
        match &self{
            LocatedTokenInner::Iterate(..) => true,
            _ => false
        }
    }

    fn iterate_listing(&self) -> &[Self] {
        todo!()
    }

    fn iterate_counter_name(&self) -> &str {
        match &self{
            LocatedTokenInner::Iterate(name, ..) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn iterate_values(&self) -> either::Either<&Vec<Self::Expr>, &Self::Expr> {
        match &self{
            LocatedTokenInner::Iterate(_, values, ..) => values.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_for(&self) -> bool {
        match &self{
            LocatedTokenInner::For { .. } => true,
            _ => false
        }
    }

    fn for_listing(&self) -> &[Self] {
        todo!()
    }

    fn for_label(&self) -> &str {
        match &self{
            LocatedTokenInner::For { label, .. } => label.as_ref(),
            _ => unreachable!()
        }
    }

    fn for_start(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::For { start, .. } => start,
            _ => unreachable!()
        }
    }

    fn for_stop(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::For { stop, .. } => stop,
            _ => unreachable!()
        }
    }

    fn for_step(&self) -> Option<&Self::Expr> {
        match &self{
            LocatedTokenInner::For { step, .. } => step.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_repeat_until(&self) -> bool {
        match &self{
            LocatedTokenInner::RepeatUntil(..) => true,
            _ => false
        }
    }

    fn repeat_until_listing(&self) -> &[Self] {
        todo!()
    }

    fn repeat_until_condition(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::RepeatUntil(cond, ..) => cond,
            _ => unreachable!()
        }
    }

    fn is_repeat(&self) -> bool {
        match &self{
            LocatedTokenInner::Repeat(..) => true,
            _ => false
        }
    }

    fn repeat_listing(&self) -> &[Self] {
        todo!()
    }

    fn repeat_count(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::Repeat(e, ..) => e,
            _ => unreachable!()
        }
    }

    fn repeat_counter_name(&self) -> Option<&str> {
        match &self{
            LocatedTokenInner::Repeat(_, _, counter_name, ..) => counter_name.as_ref().map(|c| c.as_str()),
            _ => unreachable!()
        }
    }

    fn repeat_counter_start(&self) -> Option<&Self::Expr> {
        match &self{
            LocatedTokenInner::Repeat(_, _, _, start) => start.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_macro_definition(&self) -> bool {
        match &self{
            LocatedTokenInner::Macro { .. } => true,
            _ => false
        }
    }

    fn macro_definition_name(&self) -> &str {
        match &self{
            LocatedTokenInner::Macro { name, .. } => name.as_str(),
            _ => unreachable!()
        }
    }

    fn macro_definition_arguments(&self) -> SmallVec<[&str; 4]> {
        match &self{
            LocatedTokenInner::Macro { params, .. } => params.iter().map(|a| a.as_str()).collect(),
            _ => unreachable!()
        }
    }

    fn macro_definition_code(&self) -> &str {
        match &self{
            LocatedTokenInner::Macro { content, .. } => content.as_str(),
            _ => unreachable!()
        }
    }

    fn macro_call_name(&self) -> &str {
        match &self{
            LocatedTokenInner::MacroCall(name, ..) => name.as_str(),
            _ => panic!()
        }
    }

    fn macro_call_arguments(&self) -> &[Self::MacroParam] {
        match &self{
            LocatedTokenInner::MacroCall(_, args) => args,
            _ => panic!()
        }
    }

    fn is_if(&self) -> bool {
        match &self{
            LocatedTokenInner::If(..) => true,
            _ => false
        }
    }

    fn if_nb_tests(&self) -> usize {
        match &self{
            LocatedTokenInner::If(tests, ..) => tests.len(),
            _ => panic!()
        }
    }

    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]) {
        todo!()
    }

    fn if_else(&self) -> Option<&[Self]> {
        todo!()
    }

    fn is_include(&self) -> bool {
        match &self{
            LocatedTokenInner::Include(..) => true,
            _ => false
        }
    }

    fn is_incbin(&self) -> bool {
        match &self{
            LocatedTokenInner::Incbin { .. } => true,
            _ => false
        }
    }

    fn incbin_fname(&self) -> &str {
        match &self{
            LocatedTokenInner::Incbin { fname, .. } => fname,
            _ => unimplemented!()
        }
    }

    fn incbin_offset(&self) -> Option<&Self::Expr> {
        match &self{
            LocatedTokenInner::Incbin { offset, .. } => offset.as_ref(),
            _ => unimplemented!()
        }
    }

    fn incbin_length(&self) -> Option<&Self::Expr> {
        match &self{
            LocatedTokenInner::Incbin { length, .. } => length.as_ref(),
            _ => unimplemented!()
        }
    }

    fn incbin_transformation(&self) -> &cpclib_tokens::BinaryTransformation {
        match &self{
            LocatedTokenInner::Incbin { transformation, .. } => transformation,
            _ => unimplemented!()
        }
    }

    fn include_fname(&self) -> &str {
        match &self{
            LocatedTokenInner::Include(fname, ..) => fname,
            _ => unreachable!()
        }
    }

    fn include_namespace(&self) -> Option<&str> {
        match &self{
            LocatedTokenInner::Include(_, module, ..) => module.as_ref().map(|s| s.as_str()),
            _ => unreachable!()
        }
    }

    fn include_once(&self) -> bool {
        match &self{
            LocatedTokenInner::Include(_, _, once) => *once,
            _ => unreachable!()
        }
    }

    fn is_call_macro_or_build_struct(&self) -> bool {
        match &self{
            LocatedTokenInner::MacroCall(..) => true,
            _ => false
        }
    }

    fn is_function_definition(&self) -> bool {
        match &self{
            LocatedTokenInner::Function(..) => true,
            _ => false
        }
    }

    fn function_definition_name(&self) -> &str {
        match &self{
            LocatedTokenInner::Function(name, ..) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn function_definition_params(&self) -> SmallVec<[&str; 4]> {
        match &self{
            LocatedTokenInner::Function(_name, params, ..) => params.iter().map(|v| v.as_str()).collect(),
            _ => unreachable!()
        }
    }

    fn function_definition_inner(&self) -> &[Self] {
        todo!()
    }

    fn is_crunched_section(&self) -> bool {
        match &self{
            LocatedTokenInner::CrunchedSection(..) => true,
            _ => false
        }
    }

    fn crunched_section_listing(&self) -> &[Self] {
        todo!()
    }

    fn crunched_section_kind(&self) -> &CrunchType {
        match &self{
            LocatedTokenInner::CrunchedSection(kind, ..) => kind,
            _ => unreachable!()
        }
    }

    fn is_confined(&self) -> bool {
        match &self{
            LocatedTokenInner::Confined(..) => true,
            _ => false
        }
    }

    fn confined_listing(&self) -> &[Self] {
        todo!()
    }

    fn is_switch(&self) -> bool {
        match &self{
            LocatedTokenInner::Switch(..) => true,
            _ => false
        }
    }

    fn switch_expr(&self) -> &Self::Expr {
        match &self{
            LocatedTokenInner::Switch(expr, ..) => expr,
            _ => unreachable!()
        }
    }

    fn switch_cases(&self) -> Box<dyn Iterator<Item = (&Self::Expr, &[Self], bool)> + '_> {
        todo!()
    }

    fn switch_default(&self) -> Option<&[Self]> {
        todo!()
    }

    fn is_db(&self) -> bool {
        match &self{
            LocatedTokenInner::Defb(..) => true,
            _ => false
        }
    }

    fn is_dw(&self) -> bool {
        match &self{
            LocatedTokenInner::Defw(..) => true,
            _ => false
        }
    }

    fn is_str(&self) -> bool {
        match &self{
            LocatedTokenInner::Str(..) => true,
            _ => false
        }
    }

    fn data_exprs(&self) -> &[Self::Expr] {
        match &self{
            LocatedTokenInner::Defb(e, ..) | LocatedTokenInner::Defw(e, ..) | LocatedTokenInner::Str(e, ..) => e,
            _ => unreachable!()
        }
    }

    fn is_set(&self) -> bool {
        match &self{
            LocatedTokenInner::Assign { .. } => true,
            _ => false
        }
    }

    fn is_comment(&self) -> bool {
        match &self{
            LocatedTokenInner::Comment(..) => true,
            _ => false
        }
    }
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
pub trait MayHaveSpan {
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
        &self.span
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
        &self.span().extra
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

impl TokenExt for LocatedToken{
    fn estimated_duration(&self) -> Result<usize, AssemblerError> {
        todo!("Move back the implementation of the feature directly into the TokenExt to have it available whatever is the true implementation");
    }

    fn unroll(&self, _env: &crate::Env) -> Option<Result<Vec<&Self>, AssemblerError>> {
        todo!()
    }

    fn disassemble_data(&self) -> Result<cpclib_tokens::Listing, String> {
        todo!()
    }

}

impl Deref for LocatedToken {
    type Target = LocatedTokenInner;

    fn deref(&self) -> &Self::Target {
        match &self.inner {
            either::Either::Left(inner) => inner,
            either::Either::Right((inner, _)) => inner.deref(),
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

/*
impl ListingTrait for LocatedListing {
    type Element = LocatedToken;

    fn as_slice(&self) -> &[Self::Element] {
       self.with_parse_result(|result|{
            match result {
                ParseResult::SuccessComplete(listing) => listing,
                ParseResult::SuccessInner { listing, inner_span, next_span } => listing,
                ParseResult::FailureInner(_) => unreachable!(),
                ParseResult::FailureComplete(_) => unreachable!(),
            }
       })
    }
}
*/

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
    FailureInner(Err<Z80ParserError>),
    FailureComplete(AssemblerError) // TODO use Z80ParserError there
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
    #[inline]
    pub fn new_complete_source(
        code: String,
        builder: ParserContextBuilder
    ) -> Result<LocatedListing, LocatedListing> {
        // generate the listing
        let listing = LocatedListingBuilder {
            // source code is a string owned by the listing
            src: Some(code.into()),

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
                let src = ctx.source;
                let input_start = Z80Span::new_extra(src, ctx);

                let mut tokens = Vec::with_capacity(100);
                // really make the parsing
                let res = my_many_till_nocollect(parse_z80_line_complete(&mut tokens), eof)(
                    input_start.clone()
                );

                // analyse result and can generate error even if parsing was ok
                let res = match res {
                    Ok((input_stop, _)) => {
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
                                    context: ctx.clone()
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
    #[inline]
    pub fn parse_inner(
        input_code: Z80Span,
        new_state: ParsingState
    ) -> IResult<Z80Span, Arc<LocatedListing>, Z80ParserError> {
        // let mut tokens = RefCell::new(Vec::new());
        let mut tokens = Vec::with_capacity(20);

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
            ctx_builder: move |_src| {
                // we have already build the context
                ctx
            },

            parse_result_builder: |_src, lst_ctx| {
                // build a span with the appropriate novel context
                let lst_ctx =
                    unsafe { &*(lst_ctx as *const ParserContext) as &'static ParserContext }; // the context is stored within the object; so it is safe to set its lifetime to static

                // Build the span that will be parsed to collect inner tokens.
                // It has a length of input_length.
                let inner_code = Z80Span(unsafe {
                    LocatedSpan::new_from_raw_offset(
                        input_code.location_offset(),
                        input_code.location_line(),
                        &*(input_code.as_str() as *const str) as &'static str,
                        lst_ctx
                    )
                });
                // keep a track of the very beginning of the span
                let inner_start: Z80Span = inner_code.clone();

                let res = cut(context(
                    "[DBG] Inner loop",
                    my_many0_nocollect(parse_z80_line_complete(&mut tokens))
                ))(inner_code.clone());
                match res {
                    Ok((next_input, _)) => {
                        let mut next_span = next_input;
                        next_span.extra = input_code.extra;

                        // shorten the inner_code
                        let inner_span =
                            inner_start.take(inner_start.input_len() - next_span.input_len());

                         // Properly setup the source of the context
                         /*
                        {
                            let lst_ctx =
                                unsafe { 
                                    &mut *((lst_ctx as *const ParserContext) 
                                    as *mut ParserContext)
                                    as &'static mut  ParserContext
                                }; 
                            let inner_src = inner_span.as_str() as *const str;
                            let inner_src = unsafe{&*inner_src as &'static str};
                            lst_ctx.source.replace(inner_src);
                        }
                        */

                        ParseResult::SuccessInner {
                            inner_span,
                            next_span,
                            listing: InnerLocatedListing::from(tokens)
                        }
                    }
                    Err(e) => ParseResult::FailureInner(e)
                }
            }
        }
        .build();
        let inner_listing = Arc::new(inner_listing);

        match inner_listing.borrow_parse_result() {
            ParseResult::SuccessInner { next_span, .. } => Ok((next_span.clone(), inner_listing)),
            ParseResult::FailureInner(e) => {
                match e {
                    Err::Error(e) => {
                        Err(Err::Error(Z80ParserError::from_inner_error(
                            input_code,
                            inner_listing.clone(),
                            Box::new(e.clone())
                        )))
                    }
                    Err::Failure(e) => {
                        Err(Err::Failure(Z80ParserError::from_inner_error(
                            input_code,
                            inner_listing.clone(),
                            Box::new(e.clone())
                        )))
                    }
                    Err::Incomplete(e) => Err(Err::Incomplete(*e))
                }
            }

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

    pub fn nom_error_unchecked(&self) -> &Err<Z80ParserError> {
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
        #[cfg(not(target_arch = "wasm32"))]
        let iter = self.deref().par_iter();
        #[cfg(target_arch = "wasm32")]
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
            }
        }
    }
}

use std::borrow::{Borrow, Cow};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Deref, Sub};

use cpclib_common::smol_str::SmolStr;
use ordered_float::OrderedFloat;

use crate::ListingElement;
use crate::tokens::Token;

// SAFETY: All fields of Expr are Sync (Vec, Box, SmolStr, primitives, etc.)
unsafe impl Sync for Expr {}

/// Expression nodes.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[allow(missing_docs)]
pub enum Expr {
    /// Only used for disassembled code
    RelativeDelta(i8),

    /// 32 bits integer value (should be able to include any integer value manipulated by the assember.
    Value(i32),
    // 64bits float for all the mathematical operations,
    Float(OrderedFloat<f64>),
    /// Char
    Char(char),
    /// Boolean
    Bool(bool),
    /// String (for db directive)
    String(SmolStr),
    /// Label
    Label(SmolStr),
    /// List of expression
    List(Vec<Expr>),

    /// `a..b` (exclusive of `b`) / `a..=b` (inclusive) - matches Rust's own
    /// `Range`/`RangeInclusive` semantics exactly, including being empty
    /// when `a > b` (no auto-descending). Evaluates to `ExprResult::Range`,
    /// a genuine runtime type distinct from `ExprResult::List` - see that
    /// variant's own doc comment for why. The trailing `Option<Box<Expr>>`
    /// is a step, always `None` from parsing (no `a..step..b` syntax
    /// exists) - kept for structural symmetry with `ExprResult::Range`,
    /// whose `step` field the `range_step_by` builtin can set at runtime.
    ///
    /// Not to be confused with [`crate::tokens::Token::Range`], the
    /// unrelated `RANGE start, stop, label` memory-region assertion
    /// directive - different enum, same name, no relation.
    Range(Box<Expr>, Box<Expr>, bool, Option<Box<Expr>>),

    /// `target[i]` / `target[a..b]` / `target[x, y]` - postfix indexing.
    /// One index: element access when it evaluates to `Value` (`List`,
    /// `Range`, or `String`), or a slice when it evaluates to `Range`
    /// (`List` or `String` only - slicing a `Range` by a `Range` is not
    /// supported, materialize first). Two indices: `Matrix` `[x, y]`
    /// access only - there is no flattened single-index form for a
    /// `Matrix`. Binds as tightly as possible, directly to the preceding
    /// factor, before any binary operator - `a[0] + b[1]` is `(a[0]) +
    /// (b[1])`. Chains: `a[0][1]` is `Subscript(Subscript(a, [0]), [1])`.
    Subscript(Box<Expr>, Vec<Expr>),

    /// Label with a prefix
    PrefixedLabel(LabelPrefix, SmolStr),

    Paren(Box<Expr>),

    UnaryOperation(UnaryOperation, Box<Expr>),
    UnaryTokenOperation(UnaryTokenOperation, Box<Token>),
    BinaryOperation(BinaryOperation, Box<Expr>, Box<Expr>),

    /// Ternary conditional: condition ? true_value : false_value
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),

    /// Function supposely coded by the user
    AnyFunction(SmolStr, Vec<Expr>),

    /// Random value
    Rnd
}

impl From<Expr> for Cow<'_, Expr> {
    fn from(val: Expr) -> Self {
        Cow::Owned(val)
    }
}

impl<'e> From<&'e Expr> for Cow<'e, Expr> {
    fn from(val: &'e Expr) -> Self {
        Cow::Borrowed(val)
    }
}

impl<T> ExprElement for Box<T>
where T: ExprElement
{
    type Expr = T::Expr;
    type ResultExpr = T::ResultExpr;
    type Token = T::Token;

    fn to_expr(&self) -> Cow<'_, Expr> {
        self.as_ref().to_expr()
    }

    fn is_negated(&self) -> bool {
        self.as_ref().is_negated()
    }

    fn is_relative(&self) -> bool {
        self.as_ref().is_relative()
    }

    fn relative_delta(&self) -> i8 {
        self.as_ref().relative_delta()
    }

    fn is_value(&self) -> bool {
        self.as_ref().is_value()
    }

    fn value(&self) -> i32 {
        self.as_ref().value()
    }

    fn is_char(&self) -> bool {
        self.as_ref().is_char()
    }

    fn char(&self) -> char {
        self.as_ref().char()
    }

    fn is_bool(&self) -> bool {
        self.as_ref().is_bool()
    }

    fn bool(&self) -> bool {
        self.as_ref().bool()
    }

    fn is_string(&self) -> bool {
        self.as_ref().is_string()
    }

    fn string(&self) -> &str {
        self.as_ref().string()
    }

    fn is_float(&self) -> bool {
        self.as_ref().is_float()
    }

    fn float(&self) -> OrderedFloat<f64> {
        self.as_ref().float()
    }

    fn is_list(&self) -> bool {
        self.as_ref().is_list()
    }

    fn list(&self) -> &[Self::Expr] {
        self.as_ref().list()
    }

    fn is_label(&self) -> bool {
        self.as_ref().is_label()
    }

    fn label(&self) -> &str {
        self.as_ref().label()
    }

    fn is_token_operation(&self) -> bool {
        self.as_ref().is_token_operation()
    }

    fn token_operation(&self) -> &UnaryTokenOperation {
        self.as_ref().token_operation()
    }

    fn token(&self) -> &Self::Token {
        self.as_ref().token()
    }

    fn is_prefix_label(&self) -> bool {
        self.as_ref().is_prefix_label()
    }

    fn prefix(&self) -> &LabelPrefix {
        self.as_ref().prefix()
    }

    fn is_binary_operation(&self) -> bool {
        self.as_ref().is_binary_operation()
    }

    fn binary_operation(&self) -> BinaryOperation {
        self.as_ref().binary_operation()
    }

    fn is_ternary(&self) -> bool {
        self.as_ref().is_ternary()
    }

    fn ternary_condition(&self) -> &Self::Expr {
        self.as_ref().ternary_condition()
    }

    fn ternary_true(&self) -> &Self::Expr {
        self.as_ref().ternary_true()
    }

    fn ternary_false(&self) -> &Self::Expr {
        self.as_ref().ternary_false()
    }

    fn is_unary_operation(&self) -> bool {
        self.as_ref().is_unary_operation()
    }

    fn unary_operation(&self) -> UnaryOperation {
        self.as_ref().unary_operation()
    }

    fn is_paren(&self) -> bool {
        self.as_ref().is_paren()
    }

    fn is_range(&self) -> bool {
        self.as_ref().is_range()
    }

    fn range_inclusive(&self) -> bool {
        self.as_ref().range_inclusive()
    }

    fn is_subscript(&self) -> bool {
        self.as_ref().is_subscript()
    }

    fn subscript_target(&self) -> &Self::Expr {
        self.as_ref().subscript_target()
    }

    fn subscript_indices(&self) -> &[Self::Expr] {
        self.as_ref().subscript_indices()
    }

    fn is_rnd(&self) -> bool {
        self.as_ref().is_rnd()
    }

    fn is_any_function(&self) -> bool {
        self.as_ref().is_any_function()
    }

    fn function_name(&self) -> &str {
        self.as_ref().function_name()
    }

    fn function_args(&self) -> &[Self::Expr] {
        self.as_ref().function_args()
    }

    fn arg1(&self) -> &Self::Expr {
        self.as_ref().arg1()
    }

    fn arg2(&self) -> &Self::Expr {
        self.as_ref().arg2()
    }

    fn neg(&self) -> Self::ResultExpr {
        self.as_ref().neg()
    }

    fn not(&self) -> Self::ResultExpr {
        self.as_ref().not()
    }

    fn add<E: Into<Self::ResultExpr>>(&self, v: E) -> Self::ResultExpr {
        self.as_ref().add(v)
    }

    fn is_context_independant(&self) -> bool {
        self.as_ref().is_context_independant()
    }

    fn fix_relative_value(&mut self) {
        self.as_mut().fix_relative_value()
    }

    fn symbols(&self) -> std::collections::HashSet<String> {
        self.as_ref().symbols()
    }
}

/// All methods are unchecked
pub trait ExprElement: Sized {
    type Expr: ExprElement;
    type ResultExpr: ExprElement;
    type Token: ListingElement;

    fn is_label_value(&self, label: &str) -> bool {
        self.is_label() && self.label() == label
    }

    fn is_negated(&self) -> bool;

    fn is_relative(&self) -> bool;
    fn relative_delta(&self) -> i8;

    fn is_value(&self) -> bool;
    fn value(&self) -> i32;

    fn is_char(&self) -> bool;
    fn char(&self) -> char;

    fn is_bool(&self) -> bool;
    fn bool(&self) -> bool;

    fn is_string(&self) -> bool;
    fn string(&self) -> &str;

    fn is_float(&self) -> bool;
    fn float(&self) -> OrderedFloat<f64>;

    fn is_list(&self) -> bool;
    fn list(&self) -> &[Self::Expr];

    fn is_label(&self) -> bool;
    fn label(&self) -> &str;

    fn is_token_operation(&self) -> bool;
    fn token_operation(&self) -> &UnaryTokenOperation;
    fn token(&self) -> &Self::Token;

    fn is_prefix_label(&self) -> bool;
    fn prefix(&self) -> &LabelPrefix;

    fn is_binary_operation(&self) -> bool;
    fn binary_operation(&self) -> BinaryOperation;

    fn is_ternary(&self) -> bool;
    fn ternary_condition(&self) -> &Self::Expr;
    fn ternary_true(&self) -> &Self::Expr;
    fn ternary_false(&self) -> &Self::Expr;

    fn is_unary_operation(&self) -> bool;
    fn unary_operation(&self) -> UnaryOperation;

    // Removed is_unary_function, unary_function, is_binary_function, binary_function

    fn is_paren(&self) -> bool;

    /// `a..b`/`a..=b` - operands accessible via `arg1()`/`arg2()`, same as
    /// `BinaryOperation`.
    fn is_range(&self) -> bool;
    fn range_inclusive(&self) -> bool;

    /// `target[i]`/`target[a..b]`/`target[x, y]` - a variable-length index
    /// list, so it gets its own accessors rather than reusing `arg1()`/
    /// `arg2()`.
    fn is_subscript(&self) -> bool;
    fn subscript_target(&self) -> &Self::Expr;
    fn subscript_indices(&self) -> &[Self::Expr];

    fn is_rnd(&self) -> bool;

    fn is_any_function(&self) -> bool;
    fn function_name(&self) -> &str;
    fn function_args(&self) -> &[Self::Expr];

    fn arg1(&self) -> &Self::Expr;
    fn arg2(&self) -> &Self::Expr;

    fn neg(&self) -> Self::ResultExpr;
    fn not(&self) -> Self::ResultExpr;
    fn add<E: Into<Self::ResultExpr>>(&self, v: E) -> Self::ResultExpr;

    fn is_context_independant(&self) -> bool;
    fn fix_relative_value(&mut self);

    fn to_expr(&self) -> Cow<'_, Expr>;

    /// Returns all symbol names (labels) used in this expression
    fn symbols(&self) -> std::collections::HashSet<String>;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
/// Represents a prefix that provides information related to banks for a label
pub enum LabelPrefix {
    /// We want the bank of the label
    Bank,
    /// We want the page of the label
    Page,
    /// We want the Gate array configuration for the label
    Pageset
}

impl Display for LabelPrefix {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr: &'static str = match self {
            Self::Bank => "{bank}",
            Self::Page => "{page}",
            Self::Pageset => "{pageset}"
        };
        write!(format, "{repr}")
    }
}

/// Format to represent an expression
/// Stolen documentation of rasm
/// Write text, variables or the result of evaluation of an expression during assembly.
/// By default, numerical values are formatted as
// oating point values, but you may use prexes to change
/// this behaviour:
///  fhexg Display in hexadecimal format. If the value is less than #FF two digits will be displayed.
/// If less than #FFFF, the display will be forced to 4 digits.
///  fhex2g, fhex4g, fhex8g to force hex display with 2, 4 or 8 digits.
///  fbing Display a binary value. If the value is less than #FF 8 bits will be displayed. Otherwise if
/// it is less than #FFFF 16 bits will be printed. Any negative 32 bits value with all 16 upper bits
/// set to 1 will be displayed as a 16 bits value.
///  fbin8g,fbin16g,fbin32g Force binary display with 8, 16 or 32 bits.
///  fintg Display value as integer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ExprFormat {
    Hex(Option<u8>),
    Bin(Option<u8>),
    Int
}

impl Display for ExprFormat {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr: &'static str = match self {
            Self::Hex(None) => "{hex}",
            Self::Bin(None) => "{bin}",

            Self::Int => "{int}",

            Self::Hex(Some(2)) => "{hex2}",
            Self::Hex(Some(4)) => "{hex4}",
            Self::Hex(Some(8)) => "{hex8}",

            Self::Bin(Some(8)) => "{bin8}",
            Self::Bin(Some(16)) => "{bin16}",
            Self::Bin(Some(32)) => "{bin32}",

            _ => unreachable!()
        };
        write!(format, "{repr}")
    }
}

impl ExprFormat {
    /// Generate the string representation of the given value
    pub fn string_representation(&self, val: i32) -> String {
        match self {
            Self::Hex(None) => format!("0x{val:x}"),
            Self::Bin(None) => format!("0b{val:b}"),

            Self::Int => format!("{val}"),

            Self::Hex(Some(2)) => format!("0x{val:0>2x}"),
            Self::Hex(Some(4)) => format!("0x{val:0>4x}"),
            Self::Hex(Some(8)) => format!("0x{val:0>8x}"),

            Self::Bin(Some(8)) => format!("0b{val:0>8b}"),
            Self::Bin(Some(16)) => format!("0b{val:0>16b}"),
            Self::Bin(Some(32)) => format!("0b{val:0>32b}"),

            _ => unreachable!()
        }
    }
}

/// Expression for a print expression
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FormattedExpr {
    // A raw expression is represented as it is
    Raw(Expr),
    // A formatted expression has a representatio nthat depends on its format
    Formatted(ExprFormat, Expr)
}

impl FormattedExpr {
    // pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
    // match self {
    // FormattedExpr::Raw(e) | FormattedExpr::Formatted(_, e) => e.fix_local_macro_labels_with_seed(seed),
    // }
    // }
}

impl Display for FormattedExpr {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw(expr) => write!(formatter, "{expr}"),
            Self::Formatted(format, expr) => write!(formatter, "{format}{expr}")
        }
    }
}

impl From<Expr> for FormattedExpr {
    fn from(e: Expr) -> Self {
        Self::Raw(e)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum UnaryOperation {
    Neg,
    Not,
    BinaryNot
}

impl Display for UnaryOperation {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            UnaryOperation::Neg => "-",
            UnaryOperation::Not => "!",
            UnaryOperation::BinaryNot => "~"
        };
        write!(format, "{repr}")
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum UnaryTokenOperation {
    Duration,
    Opcode
}

impl Display for UnaryTokenOperation {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            UnaryTokenOperation::Duration => "DURATION",
            UnaryTokenOperation::Opcode => "OPCODE"
        };
        write!(format, "{repr}")
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum BinaryOperation {
    RightShift,
    LeftShift,

    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,

    BinaryAnd,
    BinaryOr,
    BinaryXor,

    BooleanAnd,
    BooleanOr,

    Equal,
    Different,
    LowerOrEqual,
    GreaterOrEqual,
    StrictlyGreater,
    StrictlyLower
}

impl Display for BinaryOperation {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        use BinaryOperation::*;
        let repr = match self {
            RightShift => ">>",
            LeftShift => "<<",

            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            IntDiv => "//",
            Mod => "%",

            BinaryAnd => "&",
            BinaryOr => "|",
            BinaryXor => "^",

            BooleanAnd => "&&",
            BooleanOr => "||",

            Equal => "==",
            Different => "!=",
            LowerOrEqual => "<=",
            GreaterOrEqual => ">=",
            StrictlyGreater => ">",
            StrictlyLower => "<"
        };
        write!(format, "{repr}")
    }
}

impl From<&str> for Expr {
    fn from(src: &str) -> Self {
        Expr::Label(src.into())
    }
}

// Macro to generate all the converters from one number to an expression
macro_rules! convert_number_to_expr {
        ( $($i:ty)* ) => {
            $(
                #[allow(trivial_numeric_casts)]
                impl From<$i> for Expr {
                    fn from(src: $i) -> Self {
                        Expr::Value(src as _)
                    }
                }
            )*
        };
    }

convert_number_to_expr!(i32 i16 i8 u8 u16 u32 usize);

#[allow(missing_docs)]
impl ExprElement for Expr {
    type Expr = Expr;
    type ResultExpr = Expr;
    type Token = Token;

    fn to_expr(&self) -> Cow<'_, Expr> {
        Cow::Borrowed(self)
    }

    fn is_negated(&self) -> bool {
        matches!(self, Expr::UnaryOperation(UnaryOperation::Neg, _))
    }

    fn is_relative(&self) -> bool {
        matches!(self, Expr::RelativeDelta(_))
    }

    fn relative_delta(&self) -> i8 {
        match self {
            Expr::RelativeDelta(val) => *val,
            _ => unreachable!()
        }
    }

    fn neg(&self) -> Self {
        Expr::UnaryOperation(UnaryOperation::Neg, Box::new(self.clone()))
    }

    fn add<E: Into<Expr>>(&self, v: E) -> Self {
        Expr::BinaryOperation(
            BinaryOperation::Add,
            Box::new(self.clone()),
            v.into().into()
        )
    }

    /// Check if it is necessary to read within a symbol table
    fn is_context_independant(&self) -> bool {
        matches!(self, Expr::Label(_))
    }

    /// When disassembling an instruction with relative expressions, the contained value needs to be transformed as an absolute value
    fn fix_relative_value(&mut self) {
        panic!("i am planning to remove this code, it should not be called");
        // if let Expr::Value(val) = self {
        // let mut new_expr = Expr::RelativeDelta(*val as i8);
        // std::mem::swap(self, &mut new_expr);
        // }
    }

    fn not(&self) -> Self::ResultExpr {
        Expr::UnaryOperation(UnaryOperation::Not, Box::new(self.clone()))
    }

    fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    fn value(&self) -> i32 {
        match self {
            Self::Value(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    fn char(&self) -> char {
        match self {
            Self::Char(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    fn bool(&self) -> bool {
        match self {
            Self::Bool(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    fn string(&self) -> &str {
        match self {
            Self::String(v) => v.as_str(),
            _ => unreachable!()
        }
    }

    fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    fn float(&self) -> OrderedFloat<f64> {
        match self {
            Self::Float(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    fn list(&self) -> &[Self] {
        match self {
            Self::List(v) => v.as_slice(),
            _ => unreachable!()
        }
    }

    fn is_label(&self) -> bool {
        matches!(self, Self::Label(_))
    }

    fn label(&self) -> &str {
        match self {
            Self::Label(v) => v.as_str(),
            Self::PrefixedLabel(_, v) => v.as_str(),
            _ => unreachable!()
        }
    }

    fn is_token_operation(&self) -> bool {
        matches!(self, Self::UnaryTokenOperation(..))
    }

    fn token_operation(&self) -> &UnaryTokenOperation {
        match self {
            Self::UnaryTokenOperation(op, _) => op,
            _ => unreachable!()
        }
    }

    fn token(&self) -> &Self::Token {
        match self {
            Self::UnaryTokenOperation(_, token) => token.deref(),
            _ => unreachable!()
        }
    }

    fn is_prefix_label(&self) -> bool {
        matches!(self, Self::PrefixedLabel(..))
    }

    fn prefix(&self) -> &LabelPrefix {
        match self {
            Self::PrefixedLabel(prefix, _) => prefix,
            _ => unreachable!()
        }
    }

    fn is_binary_operation(&self) -> bool {
        matches!(self, Self::BinaryOperation(..))
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
            Self::Ternary(_, true_expr, _) => true_expr.as_ref(),
            _ => unreachable!()
        }
    }

    fn ternary_false(&self) -> &Self::Expr {
        match self {
            Self::Ternary(_, _, false_expr) => false_expr.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_unary_operation(&self) -> bool {
        matches!(self, Self::UnaryOperation(..))
    }

    fn unary_operation(&self) -> UnaryOperation {
        match self {
            Self::UnaryOperation(op, _) => *op,
            _ => unreachable!()
        }
    }

    fn is_paren(&self) -> bool {
        matches!(self, Self::Paren(..))
    }

    fn is_range(&self) -> bool {
        matches!(self, Self::Range(..))
    }

    fn range_inclusive(&self) -> bool {
        match self {
            Self::Range(_, _, inclusive, _) => *inclusive,
            _ => unreachable!()
        }
    }

    fn is_subscript(&self) -> bool {
        matches!(self, Self::Subscript(..))
    }

    fn subscript_target(&self) -> &Self {
        match self {
            Self::Subscript(target, _) => target,
            _ => unreachable!()
        }
    }

    fn subscript_indices(&self) -> &[Self] {
        match self {
            Self::Subscript(_, indices) => indices.as_slice(),
            _ => unreachable!()
        }
    }

    fn is_rnd(&self) -> bool {
        matches!(self, Self::Rnd)
    }

    fn is_any_function(&self) -> bool {
        matches!(self, Self::AnyFunction(..))
    }

    fn function_name(&self) -> &str {
        match self {
            Self::AnyFunction(n, _) => n.as_str(),
            _ => unreachable!()
        }
    }

    fn function_args(&self) -> &[Self] {
        match self {
            Self::AnyFunction(_, args) => args.as_slice(),
            _ => unreachable!()
        }
    }

    fn arg1(&self) -> &Self {
        match self {
            Self::BinaryOperation(_, arg1, _) => arg1,
            Self::UnaryOperation(_, arg) => arg,
            Self::Paren(p) => p,
            Self::Range(start, ..) => start,

            _ => unreachable!()
        }
        .deref()
    }

    fn arg2(&self) -> &Self {
        match self {
            Self::BinaryOperation(_, _, arg2) => arg2.deref(),
            Self::Range(_, end, _, _) => end.deref(),
            _ => unreachable!()
        }
    }

    fn symbols(&self) -> std::collections::HashSet<String> {
        use std::collections::HashSet;

        let mut symbols = HashSet::new();

        match self {
            // Base cases: no symbols
            Self::Value(_)
            | Self::Float(_)
            | Self::Char(_)
            | Self::Bool(_)
            | Self::String(_)
            | Self::RelativeDelta(_)
            | Self::Rnd => {},

            // Label is a symbol
            Self::Label(label) | Self::PrefixedLabel(_, label) => {
                symbols.insert(label.to_string());
            },

            // Recursive cases
            Self::List(exprs) => {
                for expr in exprs {
                    symbols.extend(expr.symbols());
                }
            },
            Self::Paren(expr) => {
                symbols.extend(expr.symbols());
            },
            Self::UnaryOperation(_, expr) => {
                symbols.extend(expr.symbols());
            },
            Self::UnaryTokenOperation(..) => {
                // Token operations don't contain user symbols
            },
            Self::BinaryOperation(_, expr1, expr2) => {
                symbols.extend(expr1.symbols());
                symbols.extend(expr2.symbols());
            },
            Self::Range(start, end, _, step) => {
                symbols.extend(start.symbols());
                symbols.extend(end.symbols());
                if let Some(step) = step {
                    symbols.extend(step.symbols());
                }
            },
            Self::Subscript(target, indices) => {
                symbols.extend(target.symbols());
                for index in indices {
                    symbols.extend(index.symbols());
                }
            },
            Self::Ternary(cond, true_expr, false_expr) => {
                symbols.extend(cond.symbols());
                symbols.extend(true_expr.symbols());
                symbols.extend(false_expr.symbols());
            },
            Self::AnyFunction(name, args) => {
                // Function name could be a symbol reference
                symbols.insert(name.to_string());
                for arg in args {
                    symbols.extend(arg.symbols());
                }
            }
        }

        symbols
    }
}

impl Expr {
    pub fn to_simplified_string(&self) -> String {
        let exp = self.to_string();
        let exp = exp.trim();

        let exp = if exp.starts_with('(') && exp.ends_with(')') {
            let exp = exp.strip_prefix('(').unwrap_or(exp);

            exp.strip_suffix(')').unwrap_or(exp)
        }
        else {
            exp
        };

        exp.to_owned()
    }
}
impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Value(val) => write!(f, "0x{val:x}"),
            Expr::Float(val) => write!(f, "{val}"),
            Expr::Char(c) => write!(f, "'{c}'"),
            Expr::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Expr::String(string) => write!(f, "\"{string}\""),
            Expr::List(l) => {
                write!(
                    f,
                    "[{}]",
                    l.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            Expr::Label(label) => write!(f, "{label}"),
            Expr::PrefixedLabel(prefix, label) => write!(f, "{prefix}{label}"),
            Expr::Paren(expr) => write!(f, "({expr})"),
            Expr::AnyFunction(name, args) => {
                write!(
                    f,
                    "{}({})",
                    name,
                    args.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            Expr::UnaryOperation(op, exp) => write!(f, "{op}{exp}"),
            Expr::UnaryTokenOperation(op, tok) => write!(f, "{op}({tok})"),
            Expr::BinaryOperation(op, exp1, exp2) => write!(f, "({exp1} {op} {exp2})"),
            Expr::Ternary(cond, true_expr, false_expr) => {
                write!(f, "({cond} ? {true_expr} : {false_expr})")
            },
            Expr::RelativeDelta(val) => write!(f, "$ + {val} + 2"),
            Expr::Range(start, end, inclusive, _) => {
                write!(f, "{start}..{}{end}", if *inclusive { "=" } else { "" })
            },
            Expr::Subscript(target, indices) => {
                write!(
                    f,
                    "{target}[{}]",
                    indices.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ")
                )
            },
            Expr::Rnd => write!(f, "RND()")
        }
    }
}

// impl Debug for Expr {
// fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
// use self::Expr::*;
// match *self {
// Value(val) => write!(format, "{}", val),
// String( string) => write!(format, "\"{}\"", string),
// Label( label) => write!(format, "{}", label),
// Duration( token) => write!(format, "DURATION({:?})", token),
// OpCode( token) => write!(format, "OPCODE({:?})", token),
//
// Add( left,  right) => write!(format, "({:?} + {:?})", left, right),
// Sub( left,  right) => write!(format, "({:?} - {:?})", left, right),
// Mul( left,  right) => write!(format, "({:?} * {:?})", left, right),
// Mod( left,  right) => write!(format, "({:?} % {:?})", left, right),
// Div( left,  right) => write!(format, "({:?} / {:?})", left, right),
//
// BinaryAnd( left,  right) => write!(format, "({:?} & {:?})", left, right),
// BinaryOr( left,  right) => write!(format, "({:?} | {:?})", left, right),
// BinaryXor( left,  right) => write!(format, "({:?} ^ {:?})", left, right),
//
// Neg( e) => write!(format, "Neg({:?})", e),
//
// Paren( expr) => write!(format, "[{:?}]", expr),
//
// Equal( left,  right) => write!(format, "{:?} == {:?}", left, right),
// GreaterOrEqual( left,  right) => write!(format, "{:?} >= {:?}", left, right),
// StrictlyGreater( left,  right) => write!(format, "{:?} > {:?}", left, right),
// StrictlyLower( left,  right) => write!(format, "{:?} < {:?}", left, right),
// LowerOrEqual( left,  right) => write!(format, "{:?} <= {:?}", left, right),
//
// High( inner) => write!(format, "HI({:?})", inner),
// Low( inner) => write!(format, "LO({:?})", inner),
// }
// }
// }
impl Expr {
    // pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
    // use Expr::*;
    // match self {
    // RelativeDelta(_) | Value(_) | String(_) | Char(_)=> {}
    //
    // Label(s) | PrefixedLabel(_, s) => {
    // Self::do_apply_macro_labels_modification(s, seed);
    // }
    //
    // Duration(t) | OpCode(t) => {
    // t.fix_local_macro_labels_with_seed(seed);
    // }
    //
    // Neg(b) | Paren(b) | UnaryFunction(_, b) => {
    // b.fix_local_macro_labels_with_seed(seed);
    // }
    //
    // RightShift(b1, b2)
    // |LeftShift(b1, b2)
    // |Add(b1, b2)
    // | Sub(b1, b2)
    // | Mul(b1, b2)
    // | Div(b1, b2)
    // | Mod(b1, b2)
    // | BinaryAnd(b1, b2)
    // | BinaryOr(b1, b2)
    // | BinaryXor(b1, b2)
    // | BooleanAnd(b1, b2)
    // | BooleanOr(b1, b2)
    // | Equal(b1, b2)
    // | Different(b1, b2)
    // | LowerOrEqual(b1, b2)
    // | GreaterOrEqual(b1, b2)
    // | StrictlyGreater(b1, b2)
    // | StrictlyLower(b1, b2)
    // | BinaryFunction(_, b1, b2) => {
    // b1.fix_local_macro_labels_with_seed(seed);
    // b2.fix_local_macro_labels_with_seed(seed);
    // }
    // }
    // }

    pub fn do_apply_macro_labels_modification(s: &mut Box<str>, seed: usize) {
        assert!(!s.is_empty());
        if s.starts_with('@') {
            // Sized and allocated exactly up front rather than
            // `format!(...).into()`: format!'s own capacity heuristic
            // targets avoiding reallocation *while writing*, not landing on
            // the final length, so it routinely leaves spare capacity that
            // `.into_boxed_str()` would otherwise have to
            // reallocate-and-copy away. Writing through a cursor straight
            // into a `Box<str>` allocated at its exact size skips both
            // `String` and `Vec` entirely.
            fn digits(mut n: usize) -> usize {
                if n == 0 {
                    return 1;
                }
                let mut count = 0;
                while n > 0 {
                    count += 1;
                    n /= 10;
                }
                count
            }

            const PREFIX: &str = "__macro__";
            const SEP: &str = "__";
            let exact_len = PREFIX.len() + digits(seed) + SEP.len() + s.len();
            let mut new = crate::boxed_str_builder::build_boxed_str(exact_len, |cursor| {
                let _ = std::io::Write::write_fmt(cursor, format_args!("{PREFIX}{seed}{SEP}{s}"));
            });
            std::mem::swap(&mut new, s);
        }
    }

    pub fn negate(self) -> Self {
        Expr::UnaryOperation(UnaryOperation::Neg, Box::new(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionTypeError(String);

impl Display for ExpressionTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ExpressionTypeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PureExprEvalError {
    NeedsContext,
    HasSideEffects,
    Type(ExpressionTypeError)
}

impl Display for PureExprEvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PureExprEvalError::NeedsContext => {
                write!(
                    f,
                    "Expression evaluation requires symbol/context resolution"
                )
            },
            PureExprEvalError::HasSideEffects => {
                write!(
                    f,
                    "Expression evaluation requires side-effecting assembler state"
                )
            },
            PureExprEvalError::Type(err) => write!(f, "{err}")
        }
    }
}

impl std::error::Error for PureExprEvalError {}

impl From<ExpressionTypeError> for PureExprEvalError {
    fn from(value: ExpressionTypeError) -> Self {
        Self::Type(value)
    }
}

pub fn try_eval_expr_without_context(expr: &Expr) -> Result<ExprResult, PureExprEvalError> {
    use std::ops::Neg;

    match expr {
        Expr::RelativeDelta(_) => Err(PureExprEvalError::NeedsContext),
        Expr::Value(v) => Ok((*v).into()),
        Expr::Float(v) => Ok((*v).into()),
        Expr::Char(v) => Ok((*v).into()),
        Expr::Bool(v) => Ok((*v).into()),
        Expr::String(v) => Ok(v.clone().into()),
        Expr::Label(_) | Expr::PrefixedLabel(..) => Err(PureExprEvalError::NeedsContext),
        Expr::List(items) => {
            Ok(ExprResult::List(
                items
                    .iter()
                    .map(try_eval_expr_without_context)
                    .collect::<Result<Vec<_>, _>>()?
                    .into()
            ))
        },
        Expr::Paren(inner) => try_eval_expr_without_context(inner),
        Expr::UnaryOperation(op, inner) => {
            let value = try_eval_expr_without_context(inner)?;
            match op {
                UnaryOperation::BinaryNot => value.binary_not().map_err(PureExprEvalError::from),
                UnaryOperation::Not => value.not().map_err(PureExprEvalError::from),
                UnaryOperation::Neg => value.neg().map_err(PureExprEvalError::from)
            }
        },
        Expr::UnaryTokenOperation(..) => Err(PureExprEvalError::HasSideEffects),
        Expr::BinaryOperation(op, left, right) => {
            let a = try_eval_expr_without_context(left)?;
            let b = try_eval_expr_without_context(right)?;
            match op {
                BinaryOperation::Add => (a + b).map_err(PureExprEvalError::from),
                BinaryOperation::Sub => (a - b).map_err(PureExprEvalError::from),
                BinaryOperation::Div => (a / b).map_err(PureExprEvalError::from),
                BinaryOperation::IntDiv => {
                    a.int_div(b)
                        .map(|(v, _)| v)
                        .map_err(PureExprEvalError::from)
                },
                BinaryOperation::Mod => (a % b).map_err(PureExprEvalError::from),
                BinaryOperation::Mul => (a * b).map_err(PureExprEvalError::from),
                BinaryOperation::RightShift => (a >> b).map_err(PureExprEvalError::from),
                BinaryOperation::LeftShift => (a << b).map_err(PureExprEvalError::from),
                BinaryOperation::BinaryAnd => (a & b).map_err(PureExprEvalError::from),
                BinaryOperation::BinaryOr => (a | b).map_err(PureExprEvalError::from),
                BinaryOperation::BinaryXor => (a ^ b).map_err(PureExprEvalError::from),
                BinaryOperation::BooleanAnd => Ok(ExprResult::from(a.bool()? && b.bool()?)),
                BinaryOperation::BooleanOr => Ok(ExprResult::from(a.bool()? || b.bool()?)),
                BinaryOperation::Equal => Ok((a == b).into()),
                BinaryOperation::Different => Ok((a != b).into()),
                BinaryOperation::LowerOrEqual => a.le_checked(&b).map_err(PureExprEvalError::from),
                BinaryOperation::StrictlyLower => a.lt_checked(&b).map_err(PureExprEvalError::from),
                BinaryOperation::GreaterOrEqual => a.ge_checked(&b).map_err(PureExprEvalError::from),
                BinaryOperation::StrictlyGreater => a.gt_checked(&b).map_err(PureExprEvalError::from)
            }
        },
        Expr::Ternary(cond, when_true, when_false) => {
            if try_eval_expr_without_context(cond)?.bool()? {
                try_eval_expr_without_context(when_true)
            }
            else {
                try_eval_expr_without_context(when_false)
            }
        },
        Expr::AnyFunction(..) => Err(PureExprEvalError::HasSideEffects),
        Expr::Range(start, end, inclusive, step) => {
            let start = try_eval_expr_without_context(start)?.range_bound()?;
            let end = try_eval_expr_without_context(end)?.range_bound()?;
            let step = match step {
                Some(step) => try_eval_expr_without_context(step)?.range_bound()?,
                None => 1
            };
            if step == 0 {
                return Err(PureExprEvalError::Type(ExpressionTypeError(
                    "Range step must not be 0".to_string()
                )));
            }
            Ok(ExprResult::Range { start, end, inclusive: *inclusive, step })
        },
        Expr::Subscript(target, indices) => {
            let target = try_eval_expr_without_context(target)?;
            let indices = indices
                .iter()
                .map(try_eval_expr_without_context)
                .collect::<Result<Vec<_>, _>>()?;
            target.subscript(&indices).map_err(PureExprEvalError::from)
        },
        Expr::Rnd => Err(PureExprEvalError::HasSideEffects)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprWarningKind {
    PrecisionLoss,
    Overflow
}

/// A non-fatal issue found while evaluating an expression - e.g. a real
/// value silently rounded to fit an integer-only context, or a value that
/// doesn't fit the width it's being stored into. This crate has no notion
/// of source spans (that's a `cpclib-asm`-side concept, via `LocatedExpr`);
/// `cpclib-asm` locates it using the same generic warning-relocation
/// mechanism every other warning kind already uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprWarning {
    pub kind: ExprWarningKind,
    pub message: String
}

/// The successful result of an evaluation.
/// Embeds  a real,  an integer or a string
///
/// `List`/`Matrix` wrap their payload in `Arc` rather than storing it
/// directly: a symbol's value is looked up and `clone()`d on every
/// reference to it (`resolve()` has to return an owned `ExprResult`), and
/// without this a large or nested list literal - e.g. `EQU`'d once, then
/// read inside a `REPEAT`-unrolled macro thousands of times - paid a full
/// recursive deep-copy on every single read. With `Arc`, `clone()` is an
/// atomic refcount bump; mutation (`list_set`, `matrix_set`, and friends in
/// `cpclib-asm`'s `list.rs`/`matrix.rs`) goes through `Arc::make_mut`/
/// `Arc::try_unwrap`, which still clones - but only when the value is
/// genuinely shared, not on every read. `Arc` (not `Rc`) because `Env`
/// crosses thread boundaries via `Arc<RwLock<&mut Env>>` in a few places
/// (parallel token-tree construction) - `ExprResult` has to stay `Send`.
#[derive(Eq, Debug, Clone)]
pub enum ExprResult {
    Float(OrderedFloat<f64>),
    Value(i32),
    Char(u8),
    Bool(bool),
    String(SmolStr),
    List(std::sync::Arc<Vec<ExprResult>>),
    Matrix {
        width: usize,
        height: usize,
        content: std::sync::Arc<Vec<ExprResult>>
    },
    /// `a..b`/`a..=b`, kept as a genuine runtime type rather than eagerly
    /// expanded into a `List` - `0..1000000` must not allocate a
    /// million-element `Vec` just because it was evaluated. `len()`/
    /// `nth_value()` (below) are pure arithmetic over these four fields, no
    /// allocation regardless of range size. `step` is `1` for a plain
    /// `a..b`; the `range_step_by` builtin (`cpclib-asm/src/assembler/
    /// list.rs`) is the only thing that ever sets it to something else.
    /// `ITERATE ... IN`, `DB`/`DEFW`/`STR`/`ABYTE` emission, and
    /// `list_len`/`list_get` walk/compute against this directly; every
    /// other list-consuming builtin and the broadcasting operators
    /// materialize it into a `List` at their own boundary first, since most
    /// of them (sort, reverse, filter, map, fold, ...) have no O(1) formula
    /// over a range anyway.
    Range {
        start: i32,
        end: i32,
        inclusive: bool,
        step: i32
    }
}

impl ExprResult {
    /// Number of values a `Range` denotes - `0` when `start > end` under a
    /// positive step (no auto-descending, matches Rust's own empty-range
    /// behavior) rather than panicking or wrapping.
    pub fn range_len(start: i32, end: i32, inclusive: bool, step: i32) -> usize {
        debug_assert!(step != 0, "range_step_by must reject a zero step");
        let span = if inclusive { end - start + 1 } else { end - start };
        if span <= 0 {
            0
        }
        else {
            // Manual ceiling division - `i32::div_ceil` is still unstable.
            ((span + step - 1) / step) as usize
        }
    }

    /// The `n`th value of a `Range` (0-based) - `start + n * step`, not
    /// bounds-checked against `range_len` (callers that need a checked
    /// lookup, e.g. the `list_get` builtin, check first and report their
    /// own `InvalidSize`-style error).
    pub fn range_nth_value(start: i32, step: i32, n: usize) -> i32 {
        start + (n as i32) * step
    }

    /// Materializes a `Range` into a `List` of `Value`s - the boundary every
    /// non-bespoke `Range` consumer (broadcasting, and every `list_*`/
    /// `matrix_*` builtin without an O(1) formula) converts through. A
    /// no-op `.clone()` for anything that isn't a `Range`.
    pub fn materialize(&self) -> Self {
        match self {
            Self::Range { start, end, inclusive, step } => {
                let len = Self::range_len(*start, *end, *inclusive, *step);
                Self::List(
                    (0..len)
                        .map(|n| Self::Value(Self::range_nth_value(*start, *step, n)))
                        .collect::<Vec<_>>()
                        .into()
                )
            },
            other => other.clone()
        }
    }

    /// `target[i]`/`target[a..b]`/`target[x, y]` - see `Expr::Subscript`'s
    /// own doc comment for the full semantics (one index: element or slice
    /// depending on whether it's a `Value` or a `Range`; two indices:
    /// `Matrix[x, y]` only). Self-contained (no `cpclib-asm` dependency),
    /// used by `try_eval_expr_without_context`. `cpclib-asm`'s own
    /// `Env`-aware evaluation instead calls the real `list_get`/
    /// `string_get`/`list_sublist_by_range`/`matrix_get` functions in
    /// `cpclib-asm/src/assembler/{list,matrix}.rs`, which give more
    /// detailed bounds-checked error messages - this version intentionally
    /// trades some of that detail for staying dependency-free, matching how
    /// `range_bound` already does.
    pub fn subscript(&self, indices: &[ExprResult]) -> Result<ExprResult, ExpressionTypeError> {
        fn as_usize(v: &ExprResult) -> Result<usize, ExpressionTypeError> {
            let i = v.range_bound()?;
            usize::try_from(i)
                .map_err(|_| ExpressionTypeError(format!("Subscript index {i} must not be negative")))
        }

        match indices {
            [ExprResult::Range { start, end, inclusive, step }] => {
                let len = Self::range_len(*start, *end, *inclusive, *step);
                match self {
                    Self::List(_) | Self::Range { .. } => {
                        let list = self.materialize();
                        let mut out = Vec::with_capacity(len);
                        for n in 0..len {
                            let i = as_usize(&Self::Value(Self::range_nth_value(*start, *step, n)))?;
                            if i >= list.list_len() {
                                return Err(ExpressionTypeError(format!(
                                    "Subscript index {i} out of range (length {})",
                                    list.list_len()
                                )));
                            }
                            out.push(list.list_get(i).clone());
                        }
                        Ok(Self::List(out.into()))
                    },
                    Self::String(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        let mut out = String::with_capacity(len);
                        for n in 0..len {
                            let i = as_usize(&Self::Value(Self::range_nth_value(*start, *step, n)))?;
                            let c = chars.get(i).ok_or_else(|| {
                                ExpressionTypeError(format!(
                                    "Subscript index {i} out of range (length {})",
                                    chars.len()
                                ))
                            })?;
                            out.push(*c);
                        }
                        Ok(Self::String(out.into()))
                    },
                    _ => {
                        Err(ExpressionTypeError(format!(
                            "{self} cannot be sliced by a range"
                        )))
                    }
                }
            },
            [index] => {
                let i = as_usize(index)?;
                match self {
                    Self::List(_) | Self::Range { .. } => {
                        let list = self.materialize();
                        if i >= list.list_len() {
                            return Err(ExpressionTypeError(format!(
                                "Subscript index {i} out of range (length {})",
                                list.list_len()
                            )));
                        }
                        Ok(list.list_get(i).clone())
                    },
                    Self::String(s) => {
                        s.chars().nth(i).map(|c| Self::Char(c as u8)).ok_or_else(|| {
                            ExpressionTypeError(format!(
                                "Subscript index {i} out of range (length {})",
                                s.chars().count()
                            ))
                        })
                    },
                    _ => Err(ExpressionTypeError(format!("{self} cannot be indexed"))),
                }
            },
            [x, y] => {
                match self {
                    Self::Matrix { .. } => {
                        // User-facing `matrix[x, y]` is (column, row) - the
                        // internal `matrix_get`/`matrix_rows` convention is
                        // row-major (row index first), so this swaps order
                        // on the way in.
                        let x = as_usize(x)?;
                        let y = as_usize(y)?;
                        if y >= self.matrix_height() {
                            return Err(ExpressionTypeError(format!(
                                "Subscript row {y} out of range (height {})",
                                self.matrix_height()
                            )));
                        }
                        if x >= self.matrix_width() {
                            return Err(ExpressionTypeError(format!(
                                "Subscript column {x} out of range (width {})",
                                self.matrix_width()
                            )));
                        }
                        Ok(self.matrix_get(y, x).clone())
                    },
                    _ => {
                        Err(ExpressionTypeError(format!(
                            "{self} needs exactly one index (or a range), not two - `[x, y]` only \
                             applies to a Matrix"
                        )))
                    }
                }
            },
            _ => {
                Err(ExpressionTypeError(format!(
                    "Wrong number of subscript indices ({}) - expected 1 or 2",
                    indices.len()
                )))
            }
        }
    }
}

impl From<String> for ExprResult {
    fn from(f: String) -> Self {
        ExprResult::String(f.into())
    }
}

impl From<&SmolStr> for ExprResult {
    fn from(f: &SmolStr) -> Self {
        ExprResult::String(f.clone())
    }
}

impl From<SmolStr> for ExprResult {
    fn from(f: SmolStr) -> Self {
        ExprResult::String(f)
    }
}

impl From<f64> for ExprResult {
    fn from(f: f64) -> Self {
        ExprResult::Float(f.into())
    }
}

impl From<bool> for ExprResult {
    fn from(b: bool) -> Self {
        ExprResult::Bool(b)
    }
}

impl From<OrderedFloat<f64>> for ExprResult {
    fn from(f: OrderedFloat<f64>) -> Self {
        ExprResult::Float(f)
    }
}

impl From<usize> for ExprResult {
    fn from(i: usize) -> Self {
        ExprResult::Value(i as _)
    }
}

impl From<i32> for ExprResult {
    fn from(i: i32) -> Self {
        ExprResult::Value(i)
    }
}

impl From<u16> for ExprResult {
    fn from(i: u16) -> Self {
        ExprResult::Value(i as _)
    }
}

impl From<u8> for ExprResult {
    fn from(i: u8) -> Self {
        ExprResult::Value(i as _)
    }
}

impl From<i8> for ExprResult {
    fn from(i: i8) -> Self {
        ExprResult::Value(i as _)
    }
}
impl From<char> for ExprResult {
    fn from(i: char) -> Self {
        ExprResult::Char(i as _)
    }
}

impl<T: Into<ExprResult> + Clone> From<&[T]> for ExprResult {
    fn from(slice: &[T]) -> Self {
        ExprResult::List(
            slice
                .iter()
                .cloned()
                .map(|e| e.into())
                .collect::<Vec<_>>()
                .into()
        )
    }
}

impl AsRef<ExprResult> for ExprResult {
    fn as_ref(&self) -> &ExprResult {
        self
    }
}

impl ExprResult {
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    pub fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    pub fn is_matrix(&self) -> bool {
        matches!(self, Self::Matrix { .. })
    }

    pub fn as_type(&self, other: &Self) -> Result<Self, ExpressionTypeError> {
        if other.is_char() {
            self.char().map(|e| e.into())
        }
        else if other.is_float() {
            self.float().map(|e| e.into())
        }
        else if other.is_int() {
            self.int().map(|(i, _)| i.into())
        }
        else {
            unimplemented!();
        }
    }

    pub fn string(&self) -> Result<&str, ExpressionTypeError> {
        match self {
            ExprResult::String(s) => Ok(s.borrow()),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {self} as an string"
                )))
            },
        }
    }

    /// Converts to an integer. If `self` was a `Float` that doesn't already
    /// exactly encode the rounded integer (e.g. `6.0` truncating to `6` loses
    /// nothing and warns about nothing; `3.5` truncating to `4` does), the
    /// returned `Vec` carries one `ExprWarning` describing it - empty
    /// otherwise. Callers must consciously decide whether to forward these
    /// warnings (e.g. via `Env::add_expression_warnings`) or explicitly
    /// discard them (via `int_value()`) - the changed return type is what
    /// forces every call site to make that decision instead of silently
    /// losing precision.
    pub fn int(&self) -> Result<(i32, Vec<ExprWarning>), ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => {
                let raw = f.into_inner();
                let i = (raw + 0.5).floor() as i32; /* ensure 2.9 is treated as 3 */
                let warnings = if raw == i as f64 {
                    vec![]
                } else {
                    vec![ExprWarning {
                        kind: ExprWarningKind::PrecisionLoss,
                        message: format!("real value {raw} truncated to integer {i}")
                    }]
                };
                Ok((i, warnings))
            },
            ExprResult::Value(i) => Ok((*i, vec![])),
            ExprResult::Char(i) => Ok((*i as i32, vec![])),
            ExprResult::Bool(b) => Ok((if *b { 1 } else { 0 }, vec![])),
            ExprResult::List(l) if l.len() == 1 => {
                // Single-element lists can be converted to int (e.g., opcode(inc e))
                l[0].int()
            },
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {self} as an integer"
                )))
            },
        }
    }

    /// Convenience for call sites that intentionally do not forward the
    /// truncation warnings from `int()` (no `Env` reachable, or truncation is
    /// the point, e.g. the `INT()` builtin). Prefer this over `.int()?.0` so
    /// the intent ("discarding on purpose") is visible at the call site.
    pub fn int_value(&self) -> Result<i32, ExpressionTypeError> {
        self.int().map(|(i, _)| i)
    }

    /// Integer ("//") division: always truncates toward zero and always
    /// returns `ExprResult::Value` (i32) - unlike `/`, never promotes to
    /// float. Any truncation warnings picked up while coercing either operand
    /// to int are kept distinct (not merged into one message) and returned
    /// rather than dropped - the caller decides whether to forward them.
    pub fn int_div<T: AsRef<Self> + std::fmt::Display>(
        self,
        rhs: T
    ) -> Result<(Self, Vec<ExprWarning>), ExpressionTypeError> {
        let rhs_ref = rhs.as_ref();
        let (a, mut warnings) = self.int()?;
        let (b, w2) = rhs_ref.int()?;
        warnings.extend(w2);
        if b == 0 {
            return Err(ExpressionTypeError(format!(
                "Integer division by zero: {self} // {rhs_ref}"
            )));
        }
        Ok(((a / b).into(), warnings))
    }

    pub fn float(&self) -> Result<f64, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.into_inner()),
            ExprResult::Value(i) => Ok(*i as f64),
            ExprResult::Char(i) => Ok(*i as f64),
            ExprResult::Bool(b) => Ok(if *b { 1_f64 } else { 0 as f64 }),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {self} as a float"
                )))
            },
        }
    }

    pub fn char(&self) -> Result<char, ExpressionTypeError> {
        match self {
            ExprResult::Char(u) => Ok(*u as char),
            ExprResult::Float(f) => Ok(f.into_inner() as u8 as char),
            ExprResult::Value(v) => Ok(*v as u8 as char),
            ExprResult::Bool(b) => Ok(if *b { 'T' } else { 'F' }),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {self} as a char"
                )))
            },
        }
    }

    /// Strict integer extraction for a `Range` bound/step - unlike `int()`,
    /// does NOT round a `Float`: a non-`Value` bound is an assembly-time
    /// error, not a silent coercion (a range of floats has no obvious
    /// meaning). Shared by both the context-free evaluator
    /// (`try_eval_expr_without_context`) and `cpclib-asm`'s `Env`-aware
    /// `resolve_impl!` macro, so the two never disagree about what counts
    /// as a valid range bound.
    pub fn range_bound(&self) -> Result<i32, ExpressionTypeError> {
        match self {
            ExprResult::Value(i) => Ok(*i),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Range bound must be an integer, found {self}"
                )))
            },
        }
    }

    pub fn bool(&self) -> Result<bool, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(*f != 0.),
            ExprResult::Value(i) => Ok(*i != 0),
            ExprResult::Char(i) => Ok(*i != 0),
            ExprResult::Bool(b) => Ok(*b),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {self} as a bool"
                )))
            },
        }
    }

    pub fn not(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Bool(b) => Ok(Self::from(!*b)),
            ExprResult::Value(i) => Ok(Self::from(if *i == 0 { 1 } else { 0 })),
            ExprResult::Float(f) => Ok(Self::from(if *f == 0.0 { 1.0 } else { 0.0 })),
            _ => {
                Err(ExpressionTypeError(format!(
                    "NOT is not an operation for {self}"
                )))
            },
        }
    }
}

impl ExprResult {
    pub fn list_content(&self) -> &[ExprResult] {
        match self {
            ExprResult::List(content, ..) => content,
            _ => panic!("not a list")
        }
    }

    pub fn list_len(&self) -> usize {
        self.list_content().len()
    }

    pub fn list_get(&self, pos: usize) -> &ExprResult {
        &self.list_content()[pos]
    }

    /// Clones the backing `Vec` only if it's genuinely shared (`Arc::make_mut`) -
    /// the common case, a value nobody else references, mutates in place.
    pub fn list_set(&mut self, pos: usize, value: ExprResult) {
        match self {
            ExprResult::List(content, ..) => std::sync::Arc::make_mut(content)[pos] = value,
            _ => panic!("not a list")
        }
    }
}

impl ExprResult {
    pub fn matrix_set(&mut self, y: usize, x: usize, value: ExprResult) {
        match self {
            ExprResult::Matrix { content, .. } => {
                std::sync::Arc::make_mut(content)[y].list_set(x, value)
            },
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_get(&self, y: usize, x: usize) -> &ExprResult {
        self.matrix_rows()[y].list_get(x)
    }

    pub fn matrix_height(&self) -> usize {
        match self {
            ExprResult::Matrix { .. } => self.matrix_rows().len(),
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_width(&self) -> usize {
        match self {
            ExprResult::Matrix { .. } => {
                self.matrix_rows()
                    .first()
                    .map(|r| r.list_len())
                    .unwrap_or(0)
            },
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_rows(&self) -> &[ExprResult] {
        match self {
            ExprResult::Matrix { content, .. } => content.as_slice(),
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_col(&self, x: usize) -> ExprResult {
        let l: Vec<ExprResult> = (0..self.matrix_height())
            .map(|row| self.matrix_rows()[row].list_get(x))
            .cloned()
            .collect();
        ExprResult::List(l.into())
    }

    pub fn matrix_set_col(&mut self, x: usize, values: &[ExprResult]) {
        debug_assert!(x < self.matrix_width());

        for (y, val) in values.iter().enumerate() {
            self.matrix_set(y, x, val.clone())
        }
    }

    pub fn matrix_row(&self, y: usize) -> &ExprResult {
        &self.matrix_rows()[y]
    }

    pub fn matrix_transpose(&self) -> ExprResult {
        match self {
            ExprResult::Matrix { width, height, .. } => {
                let mut cols = vec![Vec::new(); *width];
                for row in self.matrix_rows() {
                    for (col_idx, col_val) in row.list_content().iter().enumerate() {
                        cols[col_idx].push(col_val.clone())
                    }
                }
                let cols: Vec<ExprResult> = cols
                    .into_iter()
                    .map(|c| ExprResult::List(c.into()))
                    .collect();
                ExprResult::Matrix {
                    content: cols.into(),
                    width: *height,
                    height: *width
                }
            },
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_cols(&self) -> Vec<ExprResult> {
        let t = self.matrix_transpose();
        t.matrix_rows().to_vec()
    }
}

impl ExprResult {
    pub fn floor(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.floor().into()),
            ExprResult::Value(v) => Ok((*v).into()),
            _ => Err(ExpressionTypeError(format!("Try to apply floor to {self}")))
        }
    }

    pub fn ceil(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.ceil().into()),
            ExprResult::Value(v) => Ok((*v).into()),
            _ => Err(ExpressionTypeError(format!("Try to apply ceil to {self}")))
        }
    }

    pub fn frac(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.fract().into()),
            ExprResult::Value(_v) => Ok(0.into()),
            _ => Err(ExpressionTypeError(format!("Try to apply frac to {self}")))
        }
    }

    pub fn sin(&self) -> Result<Self, ExpressionTypeError> {
        Ok((self.float()? * 3.1415926545 / 180.0).sin().into())
    }

    pub fn cos(&self) -> Result<Self, ExpressionTypeError> {
        Ok((self.float()? * 3.1415926545 / 180.0).cos().into())
    }

    pub fn asin(&self) -> Result<Self, ExpressionTypeError> {
        Ok((self.float()? * 180.0 / 3.1415926545).asin().into())
    }

    pub fn acos(&self) -> Result<Self, ExpressionTypeError> {
        Ok((self.float()? * 180.0 / 3.1415926545).acos().into())
    }

    pub fn atan(&self) -> Result<Self, ExpressionTypeError> {
        Ok((self.float()? * 180.0 / 3.1415926545).atan().into())
    }

    pub fn abs(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.abs().into()),
            ExprResult::Value(v) => Ok(v.abs().into()),
            ExprResult::Bool(_b) => Ok(self.clone()),
            _ => Err(ExpressionTypeError(format!("Try to apply abs to {self}")))
        }
    }

    pub fn ln(&self) -> Result<Self, ExpressionTypeError> {
        Ok(self.float()?.ln().into())
    }

    pub fn log10(&self) -> Result<Self, ExpressionTypeError> {
        Ok(self.float()?.log10().into())
    }

    pub fn exp(&self) -> Result<Self, ExpressionTypeError> {
        Ok(self.float()?.exp().into())
    }

    pub fn sqrt(&self) -> Result<Self, ExpressionTypeError> {
        Ok(self.float()?.sqrt().into())
    }

    pub fn binary_not(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(_) => {
                Err(ExpressionTypeError(
                    "Float are not compatible with ~ operator".to_owned()
                ))
            },
            ExprResult::Value(i) => Ok((!*i).into()),
            ExprResult::Bool(b) => Ok((!*b).into()),
            _ => Err(ExpressionTypeError(format!("Try to apply floor to {self}")))
        }
    }
}

impl std::ops::Neg for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn neg(self) -> Self::Output {
        match self {
            ExprResult::Float(f) => Ok(f.neg().into()),
            ExprResult::Value(i) => Ok(i.neg().into()),
            ExprResult::Bool(b) => Ok((!b).into()),
            _ => Err(ExpressionTypeError(format!("Try to substract {self}")))
        }
    }
}

/// Shared list/range broadcasting dispatch, used by every operator that
/// supports it (`+ - * / %`, `&`, `|`, and the comparison methods below).
/// `op` is the operator's own scalar-capable method (e.g. `ExprResult::add`
/// itself), so a nested list (`[[1,2],[3,4]] + 1`, already parseable and
/// evaluable independent of this feature) broadcasts through every level
/// automatically - `op` recurses back into `broadcast` on its own next call.
///
/// A `Range` operand materializes into a `List` first (via
/// `ExprResult::materialize`): a scaled/shifted range generally isn't a
/// contiguous range any more (`(0..1000) * 2` has no `Range` representation),
/// so there is no way to stay lazy through arbitrary broadcasting the way
/// `ITERATE`/`DB` emission/`list_len`/`list_get` can for a *plain* range walk.
///
/// Returns `None` when neither operand is list-like, so the caller falls
/// straight through to its own existing scalar-only match arms, unchanged -
/// broadcasting only ever adds a `List`-shaped wrapper in front of the
/// existing scalar logic, never replaces it. `List`/`List` of mismatched
/// length is a hard error, same as every arm this replaces already
/// enforced individually.
fn broadcast(
    lhs: &ExprResult,
    rhs: &ExprResult,
    op: impl Fn(ExprResult, ExprResult) -> Result<ExprResult, ExpressionTypeError>
) -> Option<Result<ExprResult, ExpressionTypeError>> {
    if !matches!(lhs, ExprResult::List(_) | ExprResult::Range { .. })
        && !matches!(rhs, ExprResult::List(_) | ExprResult::Range { .. })
    {
        return None;
    }
    let lhs = lhs.materialize();
    let rhs = rhs.materialize();

    let combine = |pairs: Vec<(ExprResult, ExprResult)>| -> Result<ExprResult, ExpressionTypeError> {
        let values = pairs
            .into_iter()
            .map(|(a, b)| op(a, b))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ExprResult::List(values.into()))
    };

    Some(match (&lhs, &rhs) {
        (ExprResult::List(l), ExprResult::List(r)) => {
            if l.len() != r.len() {
                Err(ExpressionTypeError(format!(
                    "Cannot combine lists of different lengths ({} and {})",
                    l.len(),
                    r.len()
                )))
            }
            else {
                combine(l.iter().cloned().zip(r.iter().cloned()).collect())
            }
        },
        (ExprResult::List(l), _) => combine(l.iter().cloned().map(|a| (a, rhs.clone())).collect()),
        (_, ExprResult::List(r)) => combine(r.iter().cloned().map(|b| (lhs.clone(), b)).collect()),
        _ => unreachable!("at least one side materializes to a List, per the guard above")
    })
}

/// As [`broadcast`], for an operator whose scalar form also threads
/// [`ExprWarning`]s (`bitand_checked`/`bitor_checked`) - every element's
/// warnings are concatenated in order.
fn broadcast_checked(
    lhs: &ExprResult,
    rhs: &ExprResult,
    op: impl Fn(ExprResult, ExprResult) -> Result<(ExprResult, Vec<ExprWarning>), ExpressionTypeError>
) -> Option<Result<(ExprResult, Vec<ExprWarning>), ExpressionTypeError>> {
    if !matches!(lhs, ExprResult::List(_) | ExprResult::Range { .. })
        && !matches!(rhs, ExprResult::List(_) | ExprResult::Range { .. })
    {
        return None;
    }
    let lhs = lhs.materialize();
    let rhs = rhs.materialize();

    let combine = |pairs: Vec<(ExprResult, ExprResult)>| -> Result<(ExprResult, Vec<ExprWarning>), ExpressionTypeError> {
        let mut warnings = Vec::new();
        let mut values = Vec::with_capacity(pairs.len());
        for (a, b) in pairs {
            let (v, w) = op(a, b)?;
            warnings.extend(w);
            values.push(v);
        }
        Ok((ExprResult::List(values.into()), warnings))
    };

    Some(match (&lhs, &rhs) {
        (ExprResult::List(l), ExprResult::List(r)) => {
            if l.len() != r.len() {
                Err(ExpressionTypeError(format!(
                    "Cannot combine lists of different lengths ({} and {})",
                    l.len(),
                    r.len()
                )))
            }
            else {
                combine(l.iter().cloned().zip(r.iter().cloned()).collect())
            }
        },
        (ExprResult::List(l), _) => combine(l.iter().cloned().map(|a| (a, rhs.clone())).collect()),
        (_, ExprResult::List(r)) => combine(r.iter().cloned().map(|b| (lhs.clone(), b)).collect()),
        _ => unreachable!("at least one side materializes to a List, per the guard above")
    })
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Add<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    /// TODO Allow "string" + &80 to add a number to the very last string
    fn add(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        if let Some(result) = broadcast(&self, rhs, |a, b| a.add(b)) {
            return result;
        }
        match (self, rhs) {
            (any, ExprResult::Bool(_)) => {
                let b = rhs.as_type(&any)?;
                any.add(b)
            },
            (ExprResult::Bool(_), any) => {
                let b = rhs.as_type(any)?;
                b.add(any)
            },

            (ExprResult::Float(f1), ExprResult::Float(f2)) => {
                Ok((f1.into_inner() + f2.into_inner()).into())
            },
            (ExprResult::Float(f1), ExprResult::Value(_)) => Ok((f1 + rhs.float()?).into()),
            (any @ (ExprResult::Value(_) | ExprResult::Char(_)), ExprResult::Float(f2)) => {
                Ok((any.float()? + f2.into_inner()).into())
            },
            (ExprResult::Value(v1), ExprResult::Value(v2)) => Ok((v1 + v2).into()),
            (ExprResult::Char(v1), ExprResult::Char(v2)) => Ok((v1 + v2).into()),
            (ExprResult::Value(v1), ExprResult::Char(v2)) => Ok((v1 + *v2 as i32).into()),
            (ExprResult::Char(v1), ExprResult::Value(v2)) => Ok((v1 as i32 + *v2).into()),

            (ExprResult::String(s), _) if s.len() == 1 => {
                ExprResult::Char(s.chars().next().unwrap() as u8) + rhs.clone()
            },

            (ExprResult::String(s), _) => {
                // This is better to use string_concat
                let rhs_str = rhs.to_string();
                Ok(ExprResult::String(format!("{s}{rhs_str}").into()))
            },

            (ExprResult::Char(c), _) => ExprResult::Value(c as _) + rhs.clone(),

            (any, ExprResult::String(s)) if s.len() == 1 => {
                any + ExprResult::Char(s.chars().next().unwrap() as u8)
            },

            (any, ExprResult::String(s)) => {
                let any_str = any.to_string();
                Ok(ExprResult::String(format!("{any_str}{s}").into()))
            },

            (any, ExprResult::Char(c)) => any + ExprResult::Value(*c as _),

            (any, _) => {
                Err(ExpressionTypeError(format!(
                    "Impossible addition between {any} and {rhs}. Fill an issue if you need it <https://github.com/cpcsdk/rust.cpclib/issues>"
                )))
            },
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Sub<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn sub(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        if let Some(result) = broadcast(&self, rhs, |a, b| a.sub(b)) {
            return result;
        }
        match (self, rhs) {
            (any, ExprResult::Bool(_)) => {
                let b = rhs.as_type(&any)?;
                any.sub(b)
            },
            (ExprResult::Bool(_), any) => {
                let b = rhs.as_type(any)?;
                b.sub(any)
            },

            (ExprResult::Float(f1), ExprResult::Float(f2)) => {
                Ok((f1.into_inner() - f2.into_inner()).into())
            },
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() - rhs.float()?).into())
            },
            (any @ ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((any.float()? - f2.into_inner()).into())
            },
            (ExprResult::Value(v1), ExprResult::Value(v2)) => Ok((v1 - v2).into()),

            (ExprResult::String(s), _) if s.len() == 1 => {
                ExprResult::Char(s.chars().next().unwrap() as u8) - rhs.clone()
            },
            (ExprResult::Char(c), any) => ExprResult::Value(c as _) - any,

            (any, ExprResult::String(s)) if s.len() == 1 => {
                any - ExprResult::Char(s.chars().next().unwrap() as u8)
            },
            (any, ExprResult::Char(c)) => any - ExprResult::Value(*c as _),

            (any, rhs) => {
                Err(ExpressionTypeError(format!(
                    "Impossible substraction between {any} and {rhs}"
                )))
            },
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Mul<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn mul(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        if let Some(result) = broadcast(&self, rhs, |a, b| a.mul(b)) {
            return result;
        }
        match (&self, rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => {
                Ok((f1.into_inner() * f2.into_inner()).into())
            },
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() * rhs.float()?).into())
            },
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((self.float()? * f2.into_inner()).into())
            },
            (ExprResult::Value(v1), ExprResult::Value(v2)) => Ok((*v1 * *v2).into()),

            (ExprResult::Value(v1), ExprResult::Char(v2))
            | (ExprResult::Char(v2), ExprResult::Value(v1)) => Ok((*v1 * (*v2 as i32)).into()),

            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible multiplication between {self} and {rhs}"
                )))
            },
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Div<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn div(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        if let Some(result) = broadcast(&self, rhs, |a, b| a.div(b)) {
            return result;
        }
        match (&self, rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => {
                Ok((f1.into_inner() / f2.into_inner()).into())
            },
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() / rhs.float()?).into())
            },
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((self.float()? / f2.into_inner()).into())
            },
            (ExprResult::Value(_), ExprResult::Value(_)) => {
                Ok((self.float()? / rhs.float()?).into())
            },

            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible division between {self} and {rhs}"
                )))
            },
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Rem<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn rem(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        if let Some(result) = broadcast(&self, rhs, |a, b| a.rem(b)) {
            return result;
        }
        match (&self, &rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => {
                Ok((f1.into_inner() % f2.into_inner()).into())
            },
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() % rhs.float()?).into())
            },
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((self.float()? % f2.into_inner()).into())
            },
            (ExprResult::Value(v1), ExprResult::Value(v2)) => {
                if *v2 == 0 {
                    return Err(ExpressionTypeError(format!(
                        "Integer remainder by zero: {self} % {rhs}"
                    )));
                }
                // Native i32 %, truncates toward zero - matches `//`'s own
                // convention. The previous `(v1 as u32 % v2 as u32) as i32`
                // formula gave different (always-non-negative-ish) results
                // for negative operands and panicked on a zero divisor; no
                // test fixture in this workspace was found to depend on it.
                Ok((v1 % v2).into())
            },

            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible reminder between {self} and {rhs}"
                )))
            },
        }
    }
}

impl ExprResult {
    /// `>>`, warning-carrying form: the `>>` operator itself (below) discards
    /// any truncation warnings, since it's a fixed-signature `std::ops` trait
    /// impl with no room for an extra return value and no `Env` to forward
    /// to anyway. `cpclib-asm`'s `resolve_impl!` macro calls this form
    /// directly instead, since it *does* have `Env` in scope.
    pub fn shr_checked(self, rhs: Self) -> Result<(Self, Vec<ExprWarning>), ExpressionTypeError> {
        let (a, mut warnings) = self.int()?;
        let (b, w2) = rhs.int()?;
        warnings.extend(w2);
        Ok((a.wrapping_shr(b as _).into(), warnings))
    }

    /// `<<`, warning-carrying form - see `shr_checked`.
    pub fn shl_checked(self, rhs: Self) -> Result<(Self, Vec<ExprWarning>), ExpressionTypeError> {
        let (a, mut warnings) = self.int()?;
        let (b, w2) = rhs.int()?;
        warnings.extend(w2);
        Ok((a.wrapping_shl(b as u32).into(), warnings))
    }

    /// `&`, warning-carrying form - see `shr_checked`. The `List`/`List` case
    /// never touches `int()` and so never warns.
    pub fn bitand_checked(self, rhs: Self) -> Result<(Self, Vec<ExprWarning>), ExpressionTypeError> {
        if let Some(result) = broadcast_checked(&self, &rhs, |a, b| a.bitand_checked(b)) {
            return result;
        }
        let (a, mut warnings) = self.int()?;
        let (b, w2) = rhs.int()?;
        warnings.extend(w2);
        Ok(((a & b).into(), warnings))
    }

    /// `|`, warning-carrying form - see `bitand_checked`.
    pub fn bitor_checked(self, rhs: Self) -> Result<(Self, Vec<ExprWarning>), ExpressionTypeError> {
        if let Some(result) = broadcast_checked(&self, &rhs, |a, b| a.bitor_checked(b)) {
            return result;
        }
        let (a, mut warnings) = self.int()?;
        let (b, w2) = rhs.int()?;
        warnings.extend(w2);
        Ok(((a | b).into(), warnings))
    }

    /// `^`, warning-carrying form - see `shr_checked`.
    pub fn bitxor_checked(self, rhs: Self) -> Result<(Self, Vec<ExprWarning>), ExpressionTypeError> {
        let (a, mut warnings) = self.int()?;
        let (b, w2) = rhs.int()?;
        warnings.extend(w2);
        Ok(((a ^ b).into(), warnings))
    }

    /// `==`, warning-carrying form - see `shr_checked`. Every non-truncating
    /// comparison arm (matching variants directly, or the `String`/`List`
    /// cross-compare) never touches `int()` and so never warns.
    pub fn eq_checked(&self, other: &Self) -> (bool, Vec<ExprWarning>) {
        match (self, other) {
            (Self::Float(l0), Self::Float(r0)) => (l0 == r0, vec![]),
            (Self::Value(l0), Self::Value(r0)) => (l0 == r0, vec![]),
            (Self::String(l0), Self::String(r0)) => (l0 == r0, vec![]),
            (Self::List(l0), Self::List(r0)) => (l0 == r0, vec![]),
            (Self::Matrix { content: l0, .. }, Self::Matrix { content: r0, .. }) => (l0 == r0, vec![]),
            (
                Self::Range { start: s0, end: e0, inclusive: i0, step: st0 },
                Self::Range { start: s1, end: e1, inclusive: i1, step: st1 }
            ) => (s0 == s1 && e0 == e1 && i0 == i1 && st0 == st1, vec![]),
            (Self::Range { .. }, _) | (_, Self::Range { .. }) => (false, vec![]),

            (Self::String(s), Self::List(l)) | (Self::List(l), Self::String(s)) => {
                let s = s.as_bytes();
                let eq = s.len() == l.len()
                    && s.iter().zip(l.iter()).all(|(a, b)| {
                        match b.int_value() {
                            Ok(b) => (*a as i32) == b,
                            Err(_) => false
                        }
                    });
                (eq, vec![])
            },

            (Self::String(_), _) | (_, Self::String(_)) => (false, vec![]),
            (Self::List(_), _) | (_, Self::List(_)) => (false, vec![]),

            _ => {
                // preserves today's panic-on-incompatible-type behavior
                let (a, mut warnings) = self.int().unwrap();
                let (b, w2) = other.int().unwrap();
                warnings.extend(w2);
                (a == b, warnings)
            }
        }
    }

    /// `<`, broadcast-aware. Unlike `eq_checked`'s `List` handling (whole-list
    /// identity, left deliberately unchanged by this feature - see this
    /// module's own notes on why `==`/`!=` don't broadcast), `<`/`>`/`<=`/`>=`
    /// had no defined `List`/`Range` behavior before this (`Ord::cmp` below
    /// panics via `unimplemented!()` the moment either side is a `List`), so
    /// broadcasting here is purely additive, not a behavior change. The
    /// scalar fallback's use of `Ord::cmp` is safe specifically because
    /// `broadcast` returning `None` already guarantees neither operand is a
    /// `List`/`Range`, so `cmp`'s `unimplemented!()` arm can never be hit
    /// from this path.
    pub fn lt_checked(&self, other: &Self) -> Result<Self, ExpressionTypeError> {
        if let Some(result) = broadcast(self, other, |a, b| a.lt_checked(&b)) {
            return result;
        }
        Ok(Self::from(self.cmp(other) == std::cmp::Ordering::Less))
    }

    /// `>`, broadcast-aware - see `lt_checked`.
    pub fn gt_checked(&self, other: &Self) -> Result<Self, ExpressionTypeError> {
        if let Some(result) = broadcast(self, other, |a, b| a.gt_checked(&b)) {
            return result;
        }
        Ok(Self::from(self.cmp(other) == std::cmp::Ordering::Greater))
    }

    /// `<=`, broadcast-aware - see `lt_checked`.
    pub fn le_checked(&self, other: &Self) -> Result<Self, ExpressionTypeError> {
        if let Some(result) = broadcast(self, other, |a, b| a.le_checked(&b)) {
            return result;
        }
        Ok(Self::from(self.cmp(other) != std::cmp::Ordering::Greater))
    }

    /// `>=`, broadcast-aware - see `lt_checked`.
    pub fn ge_checked(&self, other: &Self) -> Result<Self, ExpressionTypeError> {
        if let Some(result) = broadcast(self, other, |a, b| a.ge_checked(&b)) {
            return result;
        }
        Ok(Self::from(self.cmp(other) != std::cmp::Ordering::Less))
    }
}

impl std::ops::Shr for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn shr(self, rhs: Self) -> Self::Output {
        self.shr_checked(rhs).map(|(v, _)| v)
    }
}

impl std::ops::Shl for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn shl(self, rhs: Self) -> Self::Output {
        self.shl_checked(rhs).map(|(v, _)| v)
    }
}

impl std::ops::BitAnd for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.bitand_checked(rhs).map(|(v, _)| v)
    }
}

impl std::ops::BitOr for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.bitor_checked(rhs).map(|(v, _)| v)
    }
}

impl std::ops::BitXor for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.bitxor_checked(rhs).map(|(v, _)| v)
    }
}

impl std::cmp::PartialEq for ExprResult {
    #[allow(clippy::non_canonical_partial_ord_impl)]
    fn eq(&self, other: &Self) -> bool {
        self.eq_checked(other).0
    }
}

impl std::cmp::Ord for ExprResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Float(l0), Self::Float(r0)) => l0.cmp(r0),
            (Self::Value(l0), Self::Value(r0)) => l0.cmp(r0),

            (Self::String(l0), Self::String(r0)) => l0.cmp(r0),
            (Self::String(_), _) | (_, Self::String(_)) => unimplemented!(),

            (Self::List(l0), Self::List(r0)) => l0.cmp(r0),
            (Self::List(_), _) | (_, Self::List(_)) => unimplemented!(),

            _ => {
                self.float()
                    .unwrap()
                    .partial_cmp(&other.float().unwrap())
                    .unwrap()
            },
        }
    }
}

impl std::cmp::PartialOrd for ExprResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(f2) => write!(f, "{}", f2.into_inner()),
            ExprResult::Value(v) => write!(f, "{v}"),
            ExprResult::Char(v) => write!(f, "'{}'", *v as char),
            ExprResult::Bool(b) => write!(f, "{b}"),
            ExprResult::String(v) => write!(f, "\"{v}\""),
            ExprResult::List(v) => {
                write!(
                    f,
                    "[{}]",
                    v.iter()
                        .map(|item| format!("{item}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            ExprResult::Matrix { .. } => {
                write!(
                    f,
                    "matrix({})",
                    self.matrix_rows()
                        .iter()
                        .map(|row| format!("{row}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            ExprResult::Range { start, end, inclusive, step } => {
                write!(f, "{start}..{}{end}", if *inclusive { "=" } else { "" })?;
                if *step != 1 {
                    write!(f, " step {step}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::fmt::LowerHex for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(_f2) => write!(f, "????"),
            ExprResult::Value(v) => write!(f, "{v:x}"),
            ExprResult::Char(v) => write!(f, "{v:x}"),
            ExprResult::Bool(v) => write!(f, "{:x}", *v as u8),
            ExprResult::String(_v) => write!(f, "STRING REPRESENTATION ISSUE"),
            ExprResult::List(v) => {
                write!(
                    f,
                    "[{}]",
                    v.iter()
                        .map(|item| format!("{item:x}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            ExprResult::Matrix { .. } => {
                write!(
                    f,
                    "matrix({})",
                    self.matrix_rows()
                        .iter()
                        .map(|row| format!("{row:x}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            ExprResult::Range { .. } => write!(f, "RANGE REPRESENTATION ISSUE")
        }
    }
}

impl std::fmt::UpperHex for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(_f2) => write!(f, "????"),
            ExprResult::Value(v) => write!(f, "{v:X}"),
            ExprResult::Char(v) => write!(f, "{:X}", *v),
            ExprResult::Bool(v) => write!(f, "{:X}", *v as u8),
            ExprResult::String(_v) => write!(f, "STRING REPRESENTATION ISSUE"),
            ExprResult::List(v) => {
                write!(
                    f,
                    "[{}]",
                    v.iter()
                        .map(|item| format!("{item:X}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            ExprResult::Matrix { .. } => {
                write!(
                    f,
                    "matrix({})",
                    self.matrix_rows()
                        .iter()
                        .map(|row| format!("{row:X}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            ExprResult::Range { .. } => write!(f, "RANGE REPRESENTATION ISSUE")
        }
    }
}

impl std::ops::AddAssign for ExprResult {
    fn add_assign(&mut self, rhs: Self) {
        if let Ok(v) = self.clone().add(rhs) {
            *self = v
        }
    }
}

impl std::ops::SubAssign for ExprResult {
    fn sub_assign(&mut self, rhs: Self) {
        if let Ok(v) = self.clone().sub(rhs) {
            *self = v
        }
    }
}

#[cfg(test)]
mod int_warning_tests {
    use super::*;

    #[test]
    fn a_lossless_float_produces_no_warning() {
        let (i, warnings) = ExprResult::Float(3.0.into()).int().unwrap();
        assert_eq!(i, 3);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_lossy_float_produces_exactly_one_warning() {
        let (i, warnings) = ExprResult::Float(3.5.into()).int().unwrap();
        assert_eq!(i, 4);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].kind, ExprWarningKind::PrecisionLoss);
    }

    #[test]
    fn int_div_never_warns_on_two_integer_operands() {
        let (v, warnings) = ExprResult::Value(7).int_div(ExprResult::Value(2)).unwrap();
        assert_eq!(v, ExprResult::Value(3));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn int_div_truncates_toward_zero_for_negative_operands() {
        assert_eq!(
            ExprResult::Value(-7).int_div(ExprResult::Value(2)).unwrap().0,
            ExprResult::Value(-3)
        );
        assert_eq!(
            ExprResult::Value(7).int_div(ExprResult::Value(-2)).unwrap().0,
            ExprResult::Value(-3)
        );
        assert_eq!(
            ExprResult::Value(-7).int_div(ExprResult::Value(-2)).unwrap().0,
            ExprResult::Value(3)
        );
    }

    #[test]
    fn int_div_keeps_both_operands_warnings_distinct() {
        let (_, warnings) = ExprResult::Float(3.5.into())
            .int_div(ExprResult::Float(2.5.into()))
            .unwrap();
        assert_eq!(warnings.len(), 2, "{warnings:?}");
    }

    #[test]
    fn int_div_by_zero_is_an_error_not_a_panic() {
        assert!(ExprResult::Value(1).int_div(ExprResult::Value(0)).is_err());
    }

    #[test]
    fn rem_by_zero_is_an_error_not_a_panic() {
        use std::ops::Rem;
        assert!(ExprResult::Value(10).rem(ExprResult::Value(0)).is_err());
    }

    #[test]
    fn rem_truncates_toward_zero_for_negative_operands() {
        use std::ops::Rem;
        assert_eq!(
            ExprResult::Value(-7).rem(ExprResult::Value(2)).unwrap(),
            ExprResult::Value(-1)
        );
        assert_eq!(
            ExprResult::Value(7).rem(ExprResult::Value(-2)).unwrap(),
            ExprResult::Value(1)
        );
        assert_eq!(
            ExprResult::Value(-7).rem(ExprResult::Value(-2)).unwrap(),
            ExprResult::Value(-1)
        );
    }
}

#[cfg(test)]
mod range_and_broadcast_tests {
    use super::*;

    fn range(start: i32, end: i32, inclusive: bool) -> ExprResult {
        ExprResult::Range { start, end, inclusive, step: 1 }
    }

    fn stepped_range(start: i32, end: i32, inclusive: bool, step: i32) -> ExprResult {
        ExprResult::Range { start, end, inclusive, step }
    }

    fn values(vs: &[i32]) -> Vec<ExprResult> {
        vs.iter().map(|v| ExprResult::Value(*v)).collect()
    }

    #[test]
    fn range_to_expr_evaluates_ascending_exclusive_and_inclusive() {
        let exclusive = Expr::Range(
            Box::new(Expr::Value(0)),
            Box::new(Expr::Value(5)),
            false,
            None
        );
        assert_eq!(
            try_eval_expr_without_context(&exclusive).unwrap(),
            ExprResult::Range { start: 0, end: 5, inclusive: false, step: 1 }
        );

        let inclusive = Expr::Range(
            Box::new(Expr::Value(0)),
            Box::new(Expr::Value(5)),
            true,
            None
        );
        assert_eq!(
            try_eval_expr_without_context(&inclusive).unwrap(),
            ExprResult::Range { start: 0, end: 5, inclusive: true, step: 1 }
        );
    }

    #[test]
    fn range_bound_rejects_non_integer() {
        let bad = Expr::Range(
            Box::new(Expr::Float(1.5.into())),
            Box::new(Expr::Value(5)),
            false,
            None
        );
        assert!(try_eval_expr_without_context(&bad).is_err());
    }

    #[test]
    fn range_len_ascending_exclusive_and_inclusive() {
        assert_eq!(ExprResult::range_len(0, 5, false, 1), 5);
        assert_eq!(ExprResult::range_len(0, 5, true, 1), 6);
    }

    #[test]
    fn range_len_descending_is_empty_both_forms() {
        // Matches Rust exactly - no auto-descending.
        assert_eq!(ExprResult::range_len(5, 1, false, 1), 0);
        assert_eq!(ExprResult::range_len(5, 1, true, 1), 0);
    }

    #[test]
    fn range_len_equal_bounds() {
        assert_eq!(ExprResult::range_len(3, 3, false, 1), 0);
        assert_eq!(ExprResult::range_len(3, 3, true, 1), 1);
    }

    #[test]
    fn range_len_and_nth_value_with_a_step_that_does_not_evenly_divide_the_span() {
        // 0, 3, 6, 9 - 10 itself is excluded (exclusive end), and 12 would
        // overshoot, so exactly 4 values, not 10/3 rounded some other way.
        assert_eq!(ExprResult::range_len(0, 10, false, 3), 4);
        assert_eq!(ExprResult::range_nth_value(0, 3, 0), 0);
        assert_eq!(ExprResult::range_nth_value(0, 3, 3), 9);
    }

    #[test]
    fn materialize_expands_a_range_into_a_list_of_values() {
        assert_eq!(
            range(0, 4, false).materialize(),
            ExprResult::List(values(&[0, 1, 2, 3]).into())
        );
    }

    #[test]
    fn materialize_is_a_no_op_for_non_range_values() {
        let list = ExprResult::List(values(&[1, 2, 3]).into());
        assert_eq!(list.materialize(), list);
        assert_eq!(ExprResult::Value(5).materialize(), ExprResult::Value(5));
    }

    #[test]
    fn broadcast_add_list_scalar_and_scalar_list_both_orders() {
        let list = ExprResult::List(values(&[1, 2, 3]).into());
        assert_eq!(
            (list.clone() + ExprResult::Value(10)).unwrap(),
            ExprResult::List(values(&[11, 12, 13]).into())
        );
        assert_eq!(
            (ExprResult::Value(10) + list).unwrap(),
            ExprResult::List(values(&[11, 12, 13]).into())
        );
    }

    #[test]
    fn broadcast_mul_range_scalar_materializes_first() {
        // (0..3) * 2 -> [0, 2, 4] - no longer a contiguous range once
        // scaled, so this must come back as a List, not a Range.
        let result = (range(0, 3, false) * ExprResult::Value(2)).unwrap();
        assert_eq!(result, ExprResult::List(values(&[0, 2, 4]).into()));
    }

    #[test]
    fn broadcast_nested_list_recurses() {
        let nested = ExprResult::List(
            vec![
                ExprResult::List(values(&[1, 2]).into()),
                ExprResult::List(values(&[3, 4]).into()),
            ]
            .into()
        );
        let result = (nested + ExprResult::Value(1)).unwrap();
        assert_eq!(
            result,
            ExprResult::List(
                vec![
                    ExprResult::List(values(&[2, 3]).into()),
                    ExprResult::List(values(&[4, 5]).into()),
                ]
                .into()
            )
        );
    }

    #[test]
    fn broadcast_mismatched_length_list_list_still_errors() {
        let a = ExprResult::List(values(&[1, 2]).into());
        let b = ExprResult::List(values(&[1, 2, 3]).into());
        assert!((a + b).is_err());
    }

    #[test]
    fn bitand_checked_list_list_actually_ands_not_adds() {
        // Regression test for the discovered bug: the old (List,List) arm
        // called `.add(b)` instead of `.bitand_checked(b)`.
        let a = ExprResult::List(values(&[0b110, 0b101]).into());
        let b = ExprResult::List(values(&[0b011, 0b110]).into());
        let (result, _) = a.bitand_checked(b).unwrap();
        assert_eq!(result, ExprResult::List(values(&[0b010, 0b100]).into()));
    }

    #[test]
    fn bitor_checked_list_list_actually_ors_not_adds() {
        let a = ExprResult::List(values(&[0b100, 0b001]).into());
        let b = ExprResult::List(values(&[0b010, 0b010]).into());
        let (result, _) = a.bitor_checked(b).unwrap();
        assert_eq!(result, ExprResult::List(values(&[0b110, 0b011]).into()));
    }

    #[test]
    fn bitand_checked_broadcasts_list_scalar() {
        let list = ExprResult::List(values(&[0b110, 0b101]).into());
        let (result, _) = list.bitand_checked(ExprResult::Value(0b011)).unwrap();
        assert_eq!(result, ExprResult::List(values(&[0b010, 0b001]).into()));
    }

    #[test]
    fn lt_checked_broadcasts_list_scalar_into_list_of_bool() {
        let list = ExprResult::List(values(&[1, 2, 3]).into());
        let result = list.lt_checked(&ExprResult::Value(2)).unwrap();
        assert_eq!(
            result,
            ExprResult::List(
                vec![ExprResult::Bool(true), ExprResult::Bool(false), ExprResult::Bool(false)]
                    .into()
            )
        );
    }

    #[test]
    fn eq_checked_on_list_list_is_unchanged_whole_list_equality_not_broadcast() {
        // Q1 regression guard: `==`/`!=` deliberately do NOT broadcast -
        // this must stay a single Bool, not a List of per-element Bools.
        let a = ExprResult::List(values(&[1, 2]).into());
        let b = ExprResult::List(values(&[1, 2]).into());
        let (eq, _) = a.eq_checked(&b);
        assert!(eq);

        let c = ExprResult::List(values(&[1, 2]).into());
        let (eq_scalar, _) = c.eq_checked(&ExprResult::Value(1));
        assert!(!eq_scalar, "List == scalar is always false, never broadcast");
    }

    #[test]
    fn bool_on_a_range_or_list_errors_exactly_the_same_way() {
        // No new IF/truthiness coercion introduced by this feature.
        let list_err = ExprResult::List(values(&[1, 2, 3]).into()).bool();
        let range_err = range(0, 3, false).bool();
        assert!(list_err.is_err());
        assert!(range_err.is_err());
    }

    #[test]
    fn stepped_range_materializes_correctly() {
        assert_eq!(
            stepped_range(0, 10, false, 2).materialize(),
            ExprResult::List(values(&[0, 2, 4, 6, 8]).into())
        );
    }

    #[test]
    fn subscript_single_index_on_a_list() {
        let list = ExprResult::List(values(&[10, 20, 30]).into());
        assert_eq!(
            list.subscript(&[ExprResult::Value(1)]).unwrap(),
            ExprResult::Value(20)
        );
    }

    #[test]
    fn subscript_single_index_out_of_range_errors() {
        let list = ExprResult::List(values(&[10, 20, 30]).into());
        assert!(list.subscript(&[ExprResult::Value(3)]).is_err());
    }

    #[test]
    fn subscript_range_slices_a_list() {
        let list = ExprResult::List(values(&[10, 20, 30, 40, 50]).into());
        assert_eq!(
            list.subscript(&[range(1, 3, false)]).unwrap(),
            ExprResult::List(values(&[20, 30]).into())
        );
    }

    #[test]
    fn subscript_single_index_on_a_range_is_o1_no_materialization_needed_to_be_correct() {
        assert_eq!(
            range(0, 1000000, false)
                .subscript(&[ExprResult::Value(500000)])
                .unwrap(),
            ExprResult::Value(500000)
        );
    }

    #[test]
    fn subscript_single_index_on_a_string_returns_a_char() {
        let s = ExprResult::String("hello".into());
        assert_eq!(s.subscript(&[ExprResult::Value(1)]).unwrap(), ExprResult::Char(b'e'));
    }

    #[test]
    fn subscript_range_slices_a_string() {
        let s = ExprResult::String("hello".into());
        assert_eq!(
            s.subscript(&[range(1, 4, false)]).unwrap(),
            ExprResult::String("ell".into())
        );
    }

    #[test]
    fn subscript_two_indices_on_a_matrix_is_x_then_y_user_facing() {
        // A 2-wide, 3-tall matrix - row 0 is [1,2], row 1 is [3,4], row 2 is
        // [5,6]. `content` holds one `List` per row, not flat scalars -
        // confirmed by `matrix_get`'s own `matrix_rows()[y].list_get(x)`.
        let matrix = ExprResult::Matrix {
            width: 2,
            height: 3,
            content: vec![
                ExprResult::List(values(&[1, 2]).into()),
                ExprResult::List(values(&[3, 4]).into()),
                ExprResult::List(values(&[5, 6]).into()),
            ]
            .into()
        };
        // matrix[x=1, y=2] must be row 2, column 1 -> value 6.
        assert_eq!(
            matrix
                .subscript(&[ExprResult::Value(1), ExprResult::Value(2)])
                .unwrap(),
            ExprResult::Value(6)
        );
        // matrix[x=0, y=0] must be row 0, column 0 -> value 1.
        assert_eq!(
            matrix
                .subscript(&[ExprResult::Value(0), ExprResult::Value(0)])
                .unwrap(),
            ExprResult::Value(1)
        );
    }

    #[test]
    fn subscript_two_indices_on_a_non_matrix_errors() {
        let list = ExprResult::List(values(&[1, 2, 3]).into());
        assert!(list.subscript(&[ExprResult::Value(0), ExprResult::Value(0)]).is_err());
    }

    #[test]
    fn subscript_wrong_index_count_errors() {
        let list = ExprResult::List(values(&[1, 2, 3]).into());
        assert!(list.subscript(&[]).is_err());
        assert!(
            list.subscript(&[ExprResult::Value(0), ExprResult::Value(0), ExprResult::Value(0)])
                .is_err()
        );
    }
}

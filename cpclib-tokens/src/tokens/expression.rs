use std::borrow::Borrow;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Sub};

use cpclib_common::itertools::Itertools;
use cpclib_common::smol_str::SmolStr;
use ordered_float::OrderedFloat;

use crate::tokens::Token;
use crate::ListingElement;

/// Expression nodes.
#[derive(Debug, PartialEq, Eq, Clone)]
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

    /// Label with a prefix
    PrefixedLabel(LabelPrefix, SmolStr),

    Paren(Box<Expr>),

    UnaryFunction(UnaryFunction, Box<Expr>),
    UnaryOperation(UnaryOperation, Box<Expr>),
    UnaryTokenOperation(UnaryTokenOperation, Box<Token>),
    BinaryFunction(BinaryFunction, Box<Expr>, Box<Expr>),
    BinaryOperation(BinaryOperation, Box<Expr>, Box<Expr>),

    /// Function supposely coded by the user
    AnyFunction(SmolStr, Vec<Expr>),

    /// Random value
    Rnd
}

/// All methods are unchecked
pub trait ExprElement: Sized {
    type ResultExpr: ExprElement;
    type Token: ListingElement;

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
    fn list(&self) -> &[Self];

    fn is_label(&self) -> bool;
    fn label(&self) -> &str;

    fn is_token_operation(&self) -> bool;
    fn token_operation(&self) -> &UnaryTokenOperation;
    fn token(&self) -> &Self::Token;

    fn is_prefix_label(&self) -> bool;
    fn prefix(&self) -> &LabelPrefix;

    fn is_binary_operation(&self) -> bool;
    fn binary_operation(&self) -> BinaryOperation;

    fn is_unary_operation(&self) -> bool;
    fn unary_operation(&self) -> UnaryOperation;

    fn is_unary_function(&self) -> bool;
    fn unary_function(&self) -> UnaryFunction;

    fn is_binary_function(&self) -> bool;
    fn binary_function(&self) -> BinaryFunction;

    fn is_paren(&self) -> bool;

    fn is_rnd(&self) -> bool;

    fn is_any_function(&self) -> bool;
    fn function_name(&self) -> &str;
    fn function_args(&self) -> &[Self];

    fn arg1(&self) -> &Self;
    fn arg2(&self) -> &Self;

    fn neg(&self) -> Self::ResultExpr;
    fn not(&self) -> Self::ResultExpr;
    fn add<E: Into<Self::ResultExpr>>(&self, v: E) -> Self::ResultExpr;

    fn is_context_independant(&self) -> bool;
    fn fix_relative_value(&mut self);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
        write!(format, "{}", repr)
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
        write!(format, "{}", repr)
    }
}

impl ExprFormat {
    /// Generate the string representation of the given value
    pub fn string_representation(&self, val: i32) -> String {
        match self {
            Self::Hex(None) => format!("0x{:x}", val),
            Self::Bin(None) => format!("0b{:x}", val),

            Self::Int => format!("{}", val),

            Self::Hex(Some(2)) => format!("0x{:2x}", val),
            Self::Hex(Some(4)) => format!("0x{:4x}", val),
            Self::Hex(Some(8)) => format!("0x{:8x}", val),

            Self::Bin(Some(8)) => format!("0b{:8x}", val),
            Self::Bin(Some(16)) => format!("0b{:16x}", val),
            Self::Bin(Some(32)) => format!("0b{:32x}", val),

            _ => unreachable!()
        }
    }
}

/// Expression for a print expression
#[derive(Debug, Clone, PartialEq, Eq)]
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
            Self::Raw(expr) => write!(formatter, "{}", expr),
            Self::Formatted(format, expr) => write!(formatter, "{}{}", format, expr)
        }
    }
}

impl From<Expr> for FormattedExpr {
    fn from(e: Expr) -> Self {
        Self::Raw(e)
    }
}

/// Represent a function with one argument
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryFunction {
    /// High byte of a value
    High,
    /// Low byte of a value
    Low,
    /// Memory already assembled
    Memory,
    Char,
    Floor,
    Ceil,
    Frac,
    Int,
    Sin,
    Cos,
    ASin,
    ACos,
    Abs,
    Ln,
    Log10,
    Exp,
    Sqrt
}

impl Display for UnaryFunction {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            UnaryFunction::High => "HI",
            UnaryFunction::Low => "LO",
            UnaryFunction::Memory => "memory",
            UnaryFunction::Floor => "floor",
            UnaryFunction::Ceil => "ceil",
            UnaryFunction::Frac => "frac",
            UnaryFunction::Int => "int",
            UnaryFunction::Char => "char",
            UnaryFunction::Sin => "sin",
            UnaryFunction::Cos => "cos",
            UnaryFunction::ASin => "asin",
            UnaryFunction::ACos => "acos",
            UnaryFunction::Abs => "abs",
            UnaryFunction::Ln => "ln",
            UnaryFunction::Log10 => "log10",
            UnaryFunction::Exp => "exp",
            UnaryFunction::Sqrt => "sqrt"
        };
        write!(format, "{}", repr)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryOperation {
    Neg,
    Not
}

impl Display for UnaryOperation {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            UnaryOperation::Neg => "-",
            UnaryOperation::Not => "~"
        };
        write!(format, "{}", repr)
    }
}

impl Display for BinaryFunction {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            BinaryFunction::Min => "min",
            BinaryFunction::Max => "max",
            BinaryFunction::Pow => "pow"
        };
        write!(format, "{}", repr)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
        write!(format, "{}", repr)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BinaryOperation {
    RightShift,
    LeftShift,

    Add,
    Sub,
    Mul,
    Div,
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
        write!(format, "{}", repr)
    }
}

/// Function with two arguments
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BinaryFunction {
    Min,
    Max,
    Pow
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
    type ResultExpr = Expr;
    type Token = Token;

    fn is_negated(&self) -> bool {
        match self {
            Expr::UnaryOperation(UnaryOperation::Neg, _) => true,
            _ => false
        }
    }

    fn is_relative(&self) -> bool {
        match self {
            Expr::RelativeDelta(_) => true,
            _ => false
        }
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
        use self::Expr::*;
        match *self {
            Label(_) => false,
            _ => true
        }
    }

    /// When disassembling an instruction with relative expressions, the contained value needs to be transformed as an absolute value
    fn fix_relative_value(&mut self) {
        if let Expr::Value(val) = self {
            let mut new_expr = Expr::RelativeDelta(*val as i8);
            std::mem::swap(self, &mut new_expr);
        }
    }

    fn not(&self) -> Self::ResultExpr {
        todo!()
    }

    fn is_value(&self) -> bool {
        match self {
            Self::Value(_) => true,
            _ => false
        }
    }

    fn value(&self) -> i32 {
        match self {
            Self::Value(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_char(&self) -> bool {
        match self {
            Self::Char(_) => true,
            _ => false
        }
    }

    fn char(&self) -> char {
        match self {
            Self::Char(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_bool(&self) -> bool {
        match self {
            Self::Bool(_) => true,
            _ => false
        }
    }

    fn bool(&self) -> bool {
        match self {
            Self::Bool(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_string(&self) -> bool {
        match self {
            Self::String(_) => true,
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
            Self::Float(_) => true,
            _ => false
        }
    }

    fn float(&self) -> OrderedFloat<f64> {
        match self {
            Self::Float(v) => *v,
            _ => unreachable!()
        }
    }

    fn is_list(&self) -> bool {
        match self {
            Self::List(_) => true,
            _ => false
        }
    }

    fn list(&self) -> &[Self] {
        match self {
            Self::List(v) => v.as_slice(),
            _ => unreachable!()
        }
    }

    fn is_label(&self) -> bool {
        match self {
            Self::Label(_) => true,
            _ => false
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Label(v) => v.as_str(),
            Self::PrefixedLabel(_, v) => v.as_str(),
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
            Self::UnaryTokenOperation(op, _) => op,
            _ => unreachable!()
        }
    }

    fn token(&self) -> &Self::Token {
        match self {
            Self::UnaryTokenOperation(_, box token) => token,
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
            Self::PrefixedLabel(prefix, _) => prefix,
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
            Self::UnaryOperation(op, _) => *op,
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
            Self::UnaryFunction(f, _) => *f,
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
            Self::Rnd => true,
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
            Self::AnyFunction(n, _) => n.as_str(),
            Self::UnaryFunction(_f, _) => todo!(),
            Self::BinaryFunction(_f, ..) => todo!(),
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
            Self::BinaryOperation(_, box arg1, _) => arg1,
            Self::UnaryOperation(_, box arg) => arg,
            Self::UnaryFunction(_, box arg) => arg,
            Self::BinaryFunction(_, box arg1, _) => arg1,
            Self::Paren(box p) => p,

            _ => unreachable!()
        }
    }

    fn arg2(&self) -> &Self {
        match self {
            Self::BinaryOperation(_, _, box arg2) => arg2,
            Self::BinaryFunction(_, _, box arg2) => arg2,

            _ => unreachable!()
        }
    }
}

impl Expr {
    pub fn to_simplified_string(&self) -> String {
        let exp = self.to_string();
        let exp = exp.trim();
        let exp = exp.strip_prefix('(').unwrap_or_else(|| exp);
        let exp = exp.strip_suffix(')').unwrap_or_else(|| exp);
        exp.to_owned()
    }
}
impl Display for Expr {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        use self::Expr::*;
        match self {
            Rnd => write!(format, "RND()"),
            // Should not be displayed often
            RelativeDelta(delta) => write!(format, "$ + {} + 2", delta),

            Value(val) => write!(format, "0x{:x}", val),
            Float(val) => write!(format, "{}", val),
            Char(c) => write!(format, "'{}'", c),
            Bool(b) => write!(format, "{}", if *b { "true" } else { "false" }),
            String(ref string) => write!(format, "\"{}\"", string),
            List(l) => write!(format, "[{}]", l.iter().map(|e| e.to_string()).join(",")),
            Label(ref label) => write!(format, "{}", label),
            PrefixedLabel(prefix, label) => write!(format, "{}{}", prefix, label),

            UnaryFunction(func, arg) => write!(format, "{}({})", func, arg),

            BinaryFunction(func, arg1, arg2) => write!(format, "{}({}, {})", func, arg1, arg2),

            Paren(ref expr) => write!(format, "({})", expr),

            AnyFunction(name, args) => {
                write!(
                    format,
                    "{}({})",
                    name,
                    args.iter().map(|e| e.to_string()).join(",")
                )
            }

            UnaryOperation(op, exp) => write!(format, "{}{}", op, exp),
            UnaryTokenOperation(op, tok) => write!(format, "{}({})", op, tok),
            BinaryOperation(op, exp1, exp2) => write!(format, "{}({},{})", op, exp1, exp2)
        }
    }
}

// impl Debug for Expr {
// fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
// use self::Expr::*;
// match *self {
// Value(val) => write!(format, "{}", val),
// String(ref string) => write!(format, "\"{}\"", string),
// Label(ref label) => write!(format, "{}", label),
// Duration(ref token) => write!(format, "DURATION({:?})", token),
// OpCode(ref token) => write!(format, "OPCODE({:?})", token),
//
// Add(ref left, ref right) => write!(format, "({:?} + {:?})", left, right),
// Sub(ref left, ref right) => write!(format, "({:?} - {:?})", left, right),
// Mul(ref left, ref right) => write!(format, "({:?} * {:?})", left, right),
// Mod(ref left, ref right) => write!(format, "({:?} % {:?})", left, right),
// Div(ref left, ref right) => write!(format, "({:?} / {:?})", left, right),
//
// BinaryAnd(ref left, ref right) => write!(format, "({:?} & {:?})", left, right),
// BinaryOr(ref left, ref right) => write!(format, "({:?} | {:?})", left, right),
// BinaryXor(ref left, ref right) => write!(format, "({:?} ^ {:?})", left, right),
//
// Neg(ref e) => write!(format, "Neg({:?})", e),
//
// Paren(ref expr) => write!(format, "[{:?}]", expr),
//
// Equal(ref left, ref right) => write!(format, "{:?} == {:?}", left, right),
// GreaterOrEqual(ref left, ref right) => write!(format, "{:?} >= {:?}", left, right),
// StrictlyGreater(ref left, ref right) => write!(format, "{:?} > {:?}", left, right),
// StrictlyLower(ref left, ref right) => write!(format, "{:?} < {:?}", left, right),
// LowerOrEqual(ref left, ref right) => write!(format, "{:?} <= {:?}", left, right),
//
// High(ref inner) => write!(format, "HI({:?})", inner),
// Low(ref inner) => write!(format, "LO({:?})", inner),
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

    pub fn do_apply_macro_labels_modification(s: &mut std::string::String, seed: usize) {
        assert!(!s.is_empty());
        if s.starts_with("@") {
            let mut new = format!("__macro__{}__{}", seed, s);
            std::mem::swap(&mut new, s);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionTypeError(String);

impl Display for ExpressionTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

/// The successful result of an evaluation.
/// Embeds  a real,  an integer or a string
#[derive(Eq, Ord, Debug, Clone)]
pub enum ExprResult {
    Float(OrderedFloat<f64>),
    Value(i32),
    Char(u8),
    Bool(bool),
    String(SmolStr),
    List(Vec<ExprResult>),
    Matrix {
        width: usize,
        height: usize,
        content: Vec<ExprResult>
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
        ExprResult::List(slice.iter().cloned().map(|e| e.into()).collect_vec())
    }
}

impl ExprResult {
    pub fn is_float(&self) -> bool {
        match self {
            Self::Float(_) => true,
            _ => false
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Self::Value(_) => true,
            _ => false
        }
    }

    pub fn is_char(&self) -> bool {
        match self {
            Self::Char(_) => true,
            _ => false
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            Self::String(_) => true,
            _ => false
        }
    }

    pub fn string(&self) -> Result<&str, ExpressionTypeError> {
        match self {
            ExprResult::String(s) => Ok(s.borrow()),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {} as an string",
                    self
                )))
            }
        }
    }

    pub fn int(&self) -> Result<i32, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.into_inner() as _),
            ExprResult::Value(i) => Ok(*i),
            ExprResult::Char(i) => Ok(*i as i32),
            ExprResult::Bool(b) => Ok(if *b { 1 } else { 0 }),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {} as an int",
                    self
                )))
            }
        }
    }

    pub fn float(&self) -> Result<f64, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.into_inner()),
            ExprResult::Value(i) => Ok(*i as f64),
            ExprResult::Char(i) => Ok(*i as f64),
            ExprResult::Bool(b) => Ok(if *b { 1 as f64 } else { 0 as f64 }),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {} as a float",
                    self
                )))
            }
        }
    }

    pub fn char(&self) -> Result<char, ExpressionTypeError> {
        match self {
            ExprResult::Char(u) => Ok(*u as char),
            ExprResult::Float(f) => Ok(f.into_inner() as u8 as char),
            ExprResult::Value(v) => Ok(*v as u8 as char),
            ExprResult::Bool(b) => Ok(if *b { 'T'.into() } else { 'F'.into() }),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to convert {} as a char",
                    self
                )))
            }
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
                    "Try to convert {} as a bool",
                    self
                )))
            }
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

    pub fn list_set(&mut self, pos: usize, value: ExprResult) {
        match self {
            ExprResult::List(content, ..) => content[pos] = value,
            _ => panic!("not a list")
        }
    }
}

impl ExprResult {
    pub fn matrix_set(&mut self, y: usize, x: usize, value: ExprResult) {
        match self {
            ExprResult::Matrix { content, .. } => content[y].list_set(x, value),
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
                self.matrix_rows().get(0).map(|r| r.list_len()).unwrap_or(0)
            }
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_rows(&self) -> &[ExprResult] {
        match self {
            ExprResult::Matrix { content, .. } => content,
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_col(&self, x: usize) -> ExprResult {
        let l = (0..self.matrix_height())
            .into_iter()
            .map(|row| self.matrix_rows()[row].list_get(x))
            .cloned()
            .collect_vec();
        ExprResult::List(l)
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
                let cols = cols.into_iter().map(|v| ExprResult::List(v)).collect_vec();
                ExprResult::Matrix {
                    content: cols,
                    width: *height,
                    height: *width
                }
            }
            _ => panic!("not a matrix")
        }
    }

    pub fn matrix_cols(&self) -> Vec<ExprResult> {
        let t = self.matrix_transpose();
        t.matrix_rows().into_iter().cloned().collect_vec()
    }
}

impl ExprResult {
    pub fn floor(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.floor().into()),
            ExprResult::Value(v) => Ok(v.clone().into()),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to apply floor to {}",
                    self
                )))
            }
        }
    }

    pub fn ceil(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.ceil().into()),
            ExprResult::Value(v) => Ok(v.clone().into()),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to apply ceil to {}",
                    self
                )))
            }
        }
    }

    pub fn frac(&self) -> Result<Self, ExpressionTypeError> {
        match self {
            ExprResult::Float(f) => Ok(f.fract().into()),
            ExprResult::Value(_v) => Ok(0.into()),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to apply frac to {}",
                    self
                )))
            }
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
            _ => Err(ExpressionTypeError(format!("Try to apply abs to {}", self)))
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
                return Err(ExpressionTypeError(
                    "Float are not compatible with ~ operator".to_owned()
                ))
            }
            ExprResult::Value(i) => Ok((!*i).into()),
            ExprResult::Bool(b) => Ok((!*b).into()),
            _ => {
                Err(ExpressionTypeError(format!(
                    "Try to apply floor to {}",
                    self
                )))
            }
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
            _ => Err(ExpressionTypeError(format!("Try to substract {}", self)))
        }
    }
}

impl AsRef<ExprResult> for ExprResult {
    fn as_ref(&self) -> &ExprResult {
        self
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Add<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn add(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        match (&self, rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => Ok((f1 + f2).into()),
            (ExprResult::Float(f1), ExprResult::Value(_)) => Ok((f1 + rhs.float()?).into()),
            (ExprResult::Value(_) | ExprResult::Char(_), ExprResult::Float(f2)) => {
                Ok((self.float()? + f2.into_inner()).into())
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => Ok((v1 + v2).into()),
            (ExprResult::Char(v1), ExprResult::Char(v2)) => Ok((v1 + v2).into()),
            (ExprResult::Value(v1), ExprResult::Char(v2)) => Ok((v1 + *v2 as i32).into()),
            (ExprResult::Char(v1), ExprResult::Value(v2)) => Ok((*v1 as i32 + *v2).into()),

            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible addition between {} and {}",
                    self, rhs
                )))
            }
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Sub<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn sub(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        match (&self, rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => Ok((f1 - f2).into()),
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() - rhs.float()?).into())
            }
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((self.float()? - f2.into_inner()).into())
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => Ok((v1 - v2).into()),
            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible substraction between {} and {}",
                    self, rhs
                )))
            }
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Mul<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn mul(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        match (&self, rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => Ok((f1 * f2).into()),
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() * rhs.float()?).into())
            }
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((self.float()? * f2.into_inner()).into())
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => Ok((*v1 * *v2).into()),

            (ExprResult::Value(v1), ExprResult::Char(v2))
            | (ExprResult::Char(v2), ExprResult::Value(v1)) => Ok((*v1 * (*v2 as i32)).into()),

            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible multiplication between {} and {}",
                    self, rhs
                )))
            }
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Div<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn div(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        match (&self, rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => Ok((f1 / f2).into()),
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() / rhs.float()?).into())
            }
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((self.float()? / f2.into_inner()).into())
            }
            (ExprResult::Value(_), ExprResult::Value(_)) => {
                Ok((self.float()? / rhs.float()?).into())
            }
            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible division between {} and {}",
                    self, rhs
                )))
            }
        }
    }
}

impl<T: AsRef<Self> + std::fmt::Display> std::ops::Rem<T> for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn rem(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        match (&self, &rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => Ok((f1 % f2).into()),
            (ExprResult::Float(f1), ExprResult::Value(_)) => {
                Ok((f1.into_inner() % rhs.float()?).into())
            }
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                Ok((self.float()? % f2.into_inner()).into())
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => Ok((v1 % v2).into()),
            (..) => {
                Err(ExpressionTypeError(format!(
                    "Impossible reminder between {} and {}",
                    self, rhs
                )))
            }
        }
    }
}

impl std::ops::Shr for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn shr(self, rhs: Self) -> Self::Output {
        Ok((self.int()? >> rhs.int()?).into())
    }
}

impl std::ops::Shl for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn shl(self, rhs: Self) -> Self::Output {
        Ok((self.int()? << rhs.int()?).into())
    }
}

impl std::ops::BitAnd for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn bitand(self, rhs: Self) -> Self::Output {
        Ok((self.int()? & rhs.int()?).into())
    }
}

impl std::ops::BitOr for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn bitor(self, rhs: Self) -> Self::Output {
        Ok((self.int()? | rhs.int()?).into())
    }
}

impl std::ops::BitXor for ExprResult {
    type Output = Result<Self, ExpressionTypeError>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Ok((self.int()? ^ rhs.int()?).into())
    }
}

impl std::cmp::PartialEq for ExprResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Value(l0), Self::Value(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::List(l0), Self::List(r0)) => l0 == r0,

            (Self::String(s), Self::List(l)) | (Self::List(l), Self::String(s)) => {
                let s = s.as_bytes();
                if s.len() != l.len() {
                    return false;
                }
                s.iter().zip(l.iter()).all(|(a, b)| {
                    match b.int() {
                        Ok(b) => (*a as i32) == b,
                        Err(_) => false
                    }
                })
            }

            (Self::String(_), _) | (_, Self::String(_)) => false,
            (Self::List(_), _) | (_, Self::List(_)) => false,

            _ => self.int().unwrap() == other.int().unwrap()
        }
    }
}

impl std::cmp::PartialOrd for ExprResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Float(l0), Self::Float(r0)) => l0.partial_cmp(r0),
            (Self::Value(l0), Self::Value(r0)) => l0.partial_cmp(r0),

            (Self::String(l0), Self::String(r0)) => l0.partial_cmp(r0),
            (Self::String(_), _) | (_, Self::String(_)) => None,

            (Self::List(l0), Self::List(r0)) => l0.partial_cmp(r0),
            (Self::List(_), _) | (_, Self::List(_)) => None,

            _ => self.float().unwrap().partial_cmp(&other.float().unwrap())
        }
    }
}

impl std::fmt::Display for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(f2) => write!(f, "{}", f2.into_inner()),
            ExprResult::Value(v) => write!(f, "{}", v),
            ExprResult::Char(v) => write!(f, "'{}'", *v as char),
            ExprResult::Bool(b) => write!(f, "{}", b),
            ExprResult::String(v) => write!(f, "\"{}\"", v),
            ExprResult::List(v) => {
                write!(
                    f,
                    "[{}]",
                    v.iter().map(|item| format!("{}", item)).join(",")
                )
            }
            ExprResult::Matrix { .. } => {
                write!(
                    f,
                    "matrix({})",
                    self.matrix_rows()
                        .iter()
                        .map(|row| format!("{}", row))
                        .join(",")
                )
            }
        }
    }
}

impl std::fmt::LowerHex for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(_f2) => write!(f, "????"),
            ExprResult::Value(v) => write!(f, "{:x}", v),
            ExprResult::Char(v) => write!(f, "{:x}", v),
            ExprResult::Bool(v) => write!(f, "{:x}", *v as u8),
            ExprResult::String(_v) => write!(f, "STRING REPRESENTATION ISSUE"),
            ExprResult::List(v) => {
                write!(
                    f,
                    "[{}]",
                    v.iter().map(|item| format!("{:x}", item)).join(",")
                )
            }
            ExprResult::Matrix { .. } => {
                write!(
                    f,
                    "matrix({})",
                    self.matrix_rows()
                        .iter()
                        .map(|row| format!("{:x}", row))
                        .join(",")
                )
            }
        }
    }
}

impl std::fmt::UpperHex for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(_f2) => write!(f, "????"),
            ExprResult::Value(v) => write!(f, "{:X}", v),
            ExprResult::Char(v) => write!(f, "{:X}", *v),
            ExprResult::Bool(v) => write!(f, "{:X}", *v as u8),
            ExprResult::String(_v) => write!(f, "STRING REPRESENTATION ISSUE"),
            ExprResult::List(v) => {
                write!(
                    f,
                    "[{}]",
                    v.iter().map(|item| format!("{:X}", item)).join(",")
                )
            }
            ExprResult::Matrix { .. } => {
                write!(
                    f,
                    "matrix({})",
                    self.matrix_rows()
                        .iter()
                        .map(|row| format!("{:X}", row))
                        .join(",")
                )
            }
        }
    }
}

impl std::ops::AddAssign for ExprResult {
    fn add_assign(&mut self, rhs: Self) {
        match self.clone().add(rhs) {
            Ok(v) => *self = v,
            Err(_) => {}
        }
    }
}

impl std::ops::SubAssign for ExprResult {
    fn sub_assign(&mut self, rhs: Self) {
        match self.clone().sub(rhs) {
            Ok(v) => *self = v,
            Err(_) => {}
        }
    }
}

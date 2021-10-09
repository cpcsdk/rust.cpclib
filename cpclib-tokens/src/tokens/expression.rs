use crate::tokens::Token;
use ordered_float::OrderedFloat;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Sub};

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
    /// String (for db directive)
    String(String),
    /// Label
    Label(String),
    /// Label with a prefix
    PrefixedLabel(LabelPrefix, String),

    /// This expression node represents the duration of an instruction. The duration is compute at assembling and not at parsing in order to benefit of the symbol table
    Duration(Box<Token>), // TODO move in a token function stuff
    OpCode(Box<Token>), // TODO move in a token general function stuff

    // Arithmetic operations
    RightShift(Box<Expr>, Box<Expr>),
    LeftShift(Box<Expr>, Box<Expr>),

    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),

    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),

    // Binary operations
    BinaryAnd(Box<Expr>, Box<Expr>),
    BinaryOr(Box<Expr>, Box<Expr>),
    BinaryXor(Box<Expr>, Box<Expr>),
    BinaryNot(Box<Expr>),

    // Boolean operations
    BooleanAnd(Box<Expr>, Box<Expr>),
    BooleanOr(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),

    Paren(Box<Expr>),

    // Boolean operations
    Equal(Box<Expr>, Box<Expr>),
    Different(Box<Expr>, Box<Expr>),
    LowerOrEqual(Box<Expr>, Box<Expr>),
    GreaterOrEqual(Box<Expr>, Box<Expr>),
    StrictlyGreater(Box<Expr>, Box<Expr>),
    StrictlyLower(Box<Expr>, Box<Expr>),

    // Function with one argument
    UnaryFunction(UnaryFunction, Box<Expr>),
    // Function with two arguments
    BinaryFunction(BinaryFunction, Box<Expr>, Box<Expr>),

    Rnd,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// Represents a prefix that provides information related to banks for a label
pub enum LabelPrefix {
    /// We want the bank of the label
    Bank,
    /// We want the page of the label
    Page,
    /// We want the Gate array configuration for the label
    Pageset,
}

impl Display for LabelPrefix {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr: &'static str = match self {
            Self::Bank => "{bank}",
            Self::Page => "{page}",
            Self::Pageset => "{pageset}",
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
    Int,
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

            _ => unreachable!(),
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

            _ => unreachable!(),
        }
    }
}

/// Expression for a print expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormattedExpr {
    // A raw expression is represented as it is
    Raw(Expr),
    // A formatted expression has a representatio nthat depends on its format
    Formatted(ExprFormat, Expr),
}

impl FormattedExpr {
    /*
    pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
        match self {
            FormattedExpr::Raw(e) | FormattedExpr::Formatted(_, e) => e.fix_local_macro_labels_with_seed(seed),
        }
    }
    */
}

impl Display for FormattedExpr {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw(expr) => write!(formatter, "{}", expr),
            Self::Formatted(format, expr) => write!(formatter, "{}{}", format, expr),
        }
    }
}

impl From<Expr> for FormattedExpr {
    fn from(e: Expr) -> Self {
        Self::Raw(e)
    }
}

/// Represent a function with one argument
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UnaryFunction {
    /// High byte of a value
    High,
    /// Low byte of a value
    Low,
    /// Memory already assembled
    Memory,

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
    Sqrt,
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
            UnaryFunction::Sin => "sin",
            UnaryFunction::Cos => "cos",
            UnaryFunction::ASin => "asin",
            UnaryFunction::ACos => "acos",
            UnaryFunction::Abs => "abs",
            UnaryFunction::Ln => "ln",
            UnaryFunction::Log10 => "log10",
            UnaryFunction::Exp => "exp",
            UnaryFunction::Sqrt => "sqrt",
        };
        write!(format, "{}", repr)
    }
}

/// Function with two arguments
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BinaryFunction {
    Min,
    Max,
}

impl Display for BinaryFunction {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::Min => "MIN",
            Self::Max => "MAX",
        };
        write!(format, "{}", repr)
    }
}

impl From<&str> for Expr {
    fn from(src: &str) -> Self {
        Expr::Label(src.to_string())
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
impl Expr {
    pub fn is_negated(&self) -> bool {
        match self {
            Expr::Neg(_) => true,
            _ => false,
        }
    }

    pub fn is_relative(&self) -> bool {
        match self {
            Expr::RelativeDelta(_) => true,
            _ => false,
        }
    }

    pub fn relative_delta(&self) -> Option<i8> {
        match self {
            Expr::RelativeDelta(val) => Some(*val),
            _ => None,
        }
    }

    pub fn neg(&self) -> Self {
        Expr::Neg(Box::new(self.clone()))
    }

    pub fn add<E: Into<Expr>>(&self, v: E) -> Self {
        Expr::Add(Box::new(self.clone()), v.into().into())
    }

    /// Check if it is necessary to read within a symbol table
    pub fn is_context_independant(&self) -> bool {
        use self::Expr::*;
        match *self {
            Label(_) => false,
            _ => true,
        }
    }

    /// When disassembling an instruction with relative expressions, the contained value needs to be transformed as an absolute value
    pub fn fix_relative_value(&mut self) {
        if let Expr::Value(val) = self {
            let mut new_expr = Expr::RelativeDelta(*val as i8);
            std::mem::swap(self, &mut new_expr);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(missing_docs)]
pub enum Oper {
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
    LowerOrEqual,
    GreaterOrEqual,
    StrictlyGreater,
    StrictlyLower,
    Different,
}

impl Display for Oper {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        use self::Oper::*;

        match self {
            &Add => write!(format, "+"),
            &Sub => write!(format, "-"),
            &Mul => write!(format, "*"),
            &Div => write!(format, "/"),
            &Mod => write!(format, "%"),
            &RightShift => write!(format, ">>"),
            &LeftShift => write!(format, "<<"),

            BinaryAnd => write!(format, "&"),
            BinaryOr => write!(format, "|"),
            BinaryXor => write!(format, "^"),

            BooleanAnd => write!(format, "&&"),
            BooleanOr => write!(format, "||"),

            &Equal => write!(format, "=="),
            &Different => write!(format, "!="),
            &LowerOrEqual => write!(format, "<="),
            &GreaterOrEqual => write!(format, ">="),
            &StrictlyGreater => write!(format, ">"),
            &StrictlyLower => write!(format, "<"),
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
            &RelativeDelta(delta) => write!(format, "$ + {} + 2", delta),

            &Value(val) => write!(format, "0x{:x}", val),
            &Float(val) => write!(format, "{}", val),
            Char(c) => write!(format, "'{}'", c),
            &String(ref string) => write!(format, "\"{}\"", string),
            &Label(ref label) => write!(format, "{}", label),
            PrefixedLabel(prefix, label) => write!(format, "{}{}", prefix, label),

            UnaryFunction(func, arg) => write!(format, "{}({})", func, arg),

            BinaryFunction(func, arg1, arg2) => write!(format, "{}({}, {})", func, arg1, arg2),

            &Duration(ref token) => write!(format, "DURATION({})", token),
            &OpCode(ref token) => write!(format, "OPCODE({})", token),

            &RightShift(ref left, ref right) => write!(format, "({} >> {})", left, right),
            &LeftShift(ref left, ref right) => write!(format, "({} << {})", left, right),
            &Add(ref left, ref right) => write!(format, "({} + {})", left, right),
            &Sub(ref left, ref right) => write!(format, "({} - {})", left, right),
            &Mul(ref left, ref right) => write!(format, "({} * {})", left, right),
            &Mod(ref left, ref right) => write!(format, "({} % {})", left, right),
            &Div(ref left, ref right) => write!(format, "({} / {})", left, right),

            BinaryAnd(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BinaryAnd, right)
            }
            BinaryOr(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BinaryOr, right)
            }
            BinaryXor(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BinaryXor, right)
            }
            BinaryNot(ref e) => write!(format, "~({})", e),

            BooleanAnd(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BooleanAnd, right)
            }
            BooleanOr(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BooleanOr, right)
            }
            &Neg(ref e) => write!(format, "-({})", e),

            &Paren(ref expr) => write!(format, "({})", expr),

            &Different(ref left, ref right) => write!(format, "{} != {}", left, right),
            &Equal(ref left, ref right) => write!(format, "{} == {}", left, right),
            &GreaterOrEqual(ref left, ref right) => write!(format, "{} >= {}", left, right),
            &StrictlyGreater(ref left, ref right) => write!(format, "{} > {}", left, right),
            &StrictlyLower(ref left, ref right) => write!(format, "{} < {}", left, right),
            &LowerOrEqual(ref left, ref right) => write!(format, "{} <= {}", left, right),

            PrefixedLabel(prefix, label) => write!(format, "{}{}", prefix, label),
        }
    }
}

/*
impl Debug for Expr {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        use self::Expr::*;
        match *self {
            Value(val) => write!(format, "{}", val),
            String(ref string) => write!(format, "\"{}\"", string),
            Label(ref label) => write!(format, "{}", label),
            Duration(ref token) => write!(format, "DURATION({:?})", token),
            OpCode(ref token) => write!(format, "OPCODE({:?})", token),

            Add(ref left, ref right) => write!(format, "({:?} + {:?})", left, right),
            Sub(ref left, ref right) => write!(format, "({:?} - {:?})", left, right),
            Mul(ref left, ref right) => write!(format, "({:?} * {:?})", left, right),
            Mod(ref left, ref right) => write!(format, "({:?} % {:?})", left, right),
            Div(ref left, ref right) => write!(format, "({:?} / {:?})", left, right),

            BinaryAnd(ref left, ref right) => write!(format, "({:?} & {:?})", left, right),
            BinaryOr(ref left, ref right) => write!(format, "({:?} | {:?})", left, right),
            BinaryXor(ref left, ref right) => write!(format, "({:?} ^ {:?})", left, right),

            Neg(ref e) => write!(format, "Neg({:?})", e),

            Paren(ref expr) => write!(format, "[{:?}]", expr),

            Equal(ref left, ref right) => write!(format, "{:?} == {:?}", left, right),
            GreaterOrEqual(ref left, ref right) => write!(format, "{:?} >= {:?}", left, right),
            StrictlyGreater(ref left, ref right) => write!(format, "{:?} > {:?}", left, right),
            StrictlyLower(ref left, ref right) => write!(format, "{:?} < {:?}", left, right),
            LowerOrEqual(ref left, ref right) => write!(format, "{:?} <= {:?}", left, right),

            High(ref inner) => write!(format, "HI({:?})", inner),
            Low(ref inner) => write!(format, "LO({:?})", inner),
        }
    }
}
*/
impl Expr {
    /*
    pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
        use Expr::*;
        match self {
            RelativeDelta(_) | Value(_) | String(_) | Char(_)=> {}

            Label(s) | PrefixedLabel(_, s) => {
                Self::do_apply_macro_labels_modification(s, seed);
            }

            Duration(t) | OpCode(t) => {
                t.fix_local_macro_labels_with_seed(seed);
            }

            Neg(b) | Paren(b) | UnaryFunction(_, b) => {
                b.fix_local_macro_labels_with_seed(seed);
            }

            RightShift(b1, b2)
            |LeftShift(b1, b2)
            |Add(b1, b2)
            | Sub(b1, b2)
            | Mul(b1, b2)
            | Div(b1, b2)
            | Mod(b1, b2)
            | BinaryAnd(b1, b2)
            | BinaryOr(b1, b2)
            | BinaryXor(b1, b2)
            | BooleanAnd(b1, b2)
            | BooleanOr(b1, b2)
            | Equal(b1, b2)
            | Different(b1, b2)
            | LowerOrEqual(b1, b2)
            | GreaterOrEqual(b1, b2)
            | StrictlyGreater(b1, b2)
            | StrictlyLower(b1, b2)
            | BinaryFunction(_, b1, b2) => {
                b1.fix_local_macro_labels_with_seed(seed);
                b2.fix_local_macro_labels_with_seed(seed);
            }
        }
    }*/

    pub fn do_apply_macro_labels_modification(s: &mut std::string::String, seed: usize) {
        assert!(!s.is_empty());
        if s.starts_with("@") {
            let mut new = format!("__macro__{}__{}", seed, s);
            std::mem::swap(&mut new, s);
        }
    }
}

/// The successful result of an evaluation.
/// Embeds eiterh a real or an integer
#[derive(Eq, Ord, Debug, Clone, Copy)]
pub enum ExprResult {
    Float(OrderedFloat<f64>),
    Value(i32),
}

impl From<f64> for ExprResult {
    fn from(f: f64) -> Self {
        ExprResult::Float(f.into())
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
        ExprResult::Value(i as _)
    }
}

impl From<bool> for ExprResult {
    fn from(b: bool) -> Self {
        if b {
            1.into()
        } else {
            0.into()
        }
    }
}

impl ExprResult {
    pub fn is_float(&self) -> bool {
        match self {
            Self::Float(_) => true,
            _ => false,
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Self::Value(_) => true,
            _ => false,
        }
    }

    pub fn int(&self) -> i32 {
        match self {
            ExprResult::Float(f) => f.into_inner() as _,
            ExprResult::Value(i) => *i,
        }
    }

    pub fn float(&self) -> f64 {
        match self {
            ExprResult::Float(f) => f.into_inner(),
            ExprResult::Value(i) => *i as f64,
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            ExprResult::Float(f) => *f != 0.,
            ExprResult::Value(i) => *i != 0,
        }
    }
}

impl ExprResult {
    pub fn floor(&self) -> Self {
        match self {
            ExprResult::Float(f) => f.floor().into(),
            ExprResult::Value(v) => v.clone().into(),
        }
    }
    pub fn ceil(&self) -> Self {
        match self {
            ExprResult::Float(f) => f.ceil().into(),
            ExprResult::Value(v) => v.clone().into(),
        }
    }
    pub fn frac(&self) -> Self {
        match self {
            ExprResult::Float(f) => f.fract().into(),
            ExprResult::Value(_v) => 0.into(),
        }
    }
    pub fn sin(&self) -> Self {
        (self.float() * 3.1415926545 / 180.0).sin().into()
    }
    pub fn cos(&self) -> Self {
        (self.float() * 3.1415926545 / 180.0).cos().into()
    }
    pub fn asin(&self) -> Self {
        (self.float() * 180.0 / 3.1415926545).asin().into()
    }
    pub fn acos(&self) -> Self {
        (self.float() * 180.0 / 3.1415926545).acos().into()
    }
    pub fn atan(&self) -> Self {
        (self.float() * 180.0 / 3.1415926545).atan().into()
    }
    pub fn abs(&self) -> Self {
        match self {
            ExprResult::Float(f) => f.abs().into(),
            ExprResult::Value(v) => v.abs().into(),
        }
    }
    pub fn ln(&self) -> Self {
        self.float().ln().into()
    }
    pub fn log10(&self) -> Self {
        self.float().log10().into()
    }
    pub fn exp(&self) -> Self {
        self.float().exp().into()
    }
    pub fn sqrt(&self) -> Self {
        self.float().sqrt().into()
    }

    pub fn binary_not(&self) -> Result<Self, String> {
        match self {
            ExprResult::Float(_) => {
                return Err("Float are not compatible with ~ operator".to_owned())
            }
            ExprResult::Value(i) => Ok((!*i).into()),
        }
    }
}

impl std::ops::Neg for ExprResult {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            ExprResult::Float(f) => f.neg().into(),
            ExprResult::Value(i) => i.neg().into(),
        }
    }
}

impl std::ops::Add for ExprResult {
    type Output = ExprResult;

    fn add(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => (f1 + f2).into(),
            (ExprResult::Float(f1), ExprResult::Value(_)) => (f1 + rhs.float()).into(),
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                (self.float() + f2.into_inner()).into()
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => (v1 + v2).into(),
        }
    }
}

impl std::ops::Sub for ExprResult {
    type Output = ExprResult;

    fn sub(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => (f1 - f2).into(),
            (ExprResult::Float(f1), ExprResult::Value(_)) => (f1.into_inner() - rhs.float()).into(),
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                (self.float() - f2.into_inner()).into()
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => (v1 - v2).into(),
        }
    }
}

impl std::ops::Mul for ExprResult {
    type Output = ExprResult;

    fn mul(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => (f1 * f2).into(),
            (ExprResult::Float(f1), ExprResult::Value(_)) => (f1.into_inner() * rhs.float()).into(),
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                (self.float() * f2.into_inner()).into()
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => (v1 * v2).into(),
        }
    }
}

impl std::ops::Div for ExprResult {
    type Output = ExprResult;

    fn div(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => (f1 / f2).into(),
            (ExprResult::Float(f1), ExprResult::Value(_)) => (f1.into_inner() / rhs.float()).into(),
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                (self.float() / f2.into_inner()).into()
            }
            (ExprResult::Value(_), ExprResult::Value(_)) => (self.float() / rhs.float()).into(),
        }
    }
}

impl std::ops::Rem for ExprResult {
    type Output = ExprResult;

    fn rem(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (ExprResult::Float(f1), ExprResult::Float(f2)) => (f1 % f2).into(),
            (ExprResult::Float(f1), ExprResult::Value(_)) => (f1.into_inner() % rhs.float()).into(),
            (ExprResult::Value(_), ExprResult::Float(f2)) => {
                (self.float() % f2.into_inner()).into()
            }
            (ExprResult::Value(v1), ExprResult::Value(v2)) => (v1 % v2).into(),
        }
    }
}

impl std::ops::Shr for ExprResult {
    type Output = ExprResult;
    fn shr(self, rhs: Self) -> Self::Output {
        (self.int() >> rhs.int()).into()
    }
}

impl std::ops::Shl for ExprResult {
    type Output = ExprResult;
    fn shl(self, rhs: Self) -> Self::Output {
        (self.int() << rhs.int()).into()
    }
}

impl std::ops::BitAnd for ExprResult {
    type Output = ExprResult;
    fn bitand(self, rhs: Self) -> Self::Output {
        (self.int() & rhs.int()).into()
    }
}

impl std::ops::BitOr for ExprResult {
    type Output = ExprResult;
    fn bitor(self, rhs: Self) -> Self::Output {
        (self.int() | rhs.int()).into()
    }
}

impl std::ops::BitXor for ExprResult {
    type Output = ExprResult;
    fn bitxor(self, rhs: Self) -> Self::Output {
        (self.int() ^ rhs.int()).into()
    }
}

impl std::cmp::PartialEq for ExprResult {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Value(l0), Self::Value(r0)) => l0 == r0,
            _ => self.int() == other.int(),
        }
    }
}

impl std::cmp::PartialOrd for ExprResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Float(l0), Self::Float(r0)) => l0.partial_cmp(r0),
            (Self::Value(l0), Self::Value(r0)) => l0.partial_cmp(r0),
            _ => self.float().partial_cmp(&other.float()),
        }
    }
}

impl std::fmt::Display for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(f2) => write!(f, "{}", f2.into_inner()),
            ExprResult::Value(v) => write!(f, "{}", v),
        }
    }
}

impl std::fmt::LowerHex for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(_f2) => write!(f, "????"),
            ExprResult::Value(v) => write!(f, "{:x}", v),
        }
    }
}

impl std::fmt::UpperHex for ExprResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprResult::Float(_f2) => write!(f, "????"),
            ExprResult::Value(v) => write!(f, "{:X}", v),
        }
    }
}

impl std::ops::AddAssign for ExprResult {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone().add(rhs) // todo implement in a faster way
    }
}

impl std::ops::SubAssign for ExprResult {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone().sub(rhs) // todo implement in a faster way
    }
}

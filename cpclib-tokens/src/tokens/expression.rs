use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use crate::tokens::listing::ListingElement;
use crate::tokens::Token;

/// Expression nodes.
#[derive(Debug, PartialEq, Eq, Clone)]
#[allow(missing_docs)]
pub enum Expr {
    /// 32 bits integer value (should be able to include any integer value manipulated by the assember.
    Value(i32),
    /// String (for db directive)
    String(String),
    /// Label
    Label(String),

        /// This expression node represents the duration of an instruction. The duration is compute at assembling and not at parsing in order to benefit of the symbol table
        Duration(Box<Token>), // TODO move in a token function stuff
        OpCode(Box<Token>), // TODO move in a token general function stuff

    // Arithmetic operations
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),

    // Binary operations
    BinaryAnd(Box<Expr>, Box<Expr>),
    BinaryOr(Box<Expr>, Box<Expr>),
    BinaryXor(Box<Expr>, Box<Expr>),

    Neg(Box<Expr>),

    Paren(Box<Expr>),

    // Boolean operations
    Equal(Box<Expr>, Box<Expr>),
    LowerOrEqual(Box<Expr>, Box<Expr>),
    GreaterOrEqual(Box<Expr>, Box<Expr>),
    StrictlyGreater(Box<Expr>, Box<Expr>),
    StrictlyLower(Box<Expr>, Box<Expr>),

    // Function with one argument
    UnaryFunction(UnaryFunction, Box<Expr>),
    // Function with two arguments
    BinaryFunction(BinaryFunction, Box<Expr>, Box<Expr>)
}

/// Represent a function with one argument
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UnaryFunction {
    High,
    Low,
}

impl Display for UnaryFunction {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Self::High => "HI",
            Self::Low => "LO"
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
            Self::Max => "MAX"
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
            _ => false
        }
    }

    pub fn neg(&self) -> Self {
        Expr::Neg(Box::new(self.clone()))
    }

 

    /// Check if it is necessary to read within a symbol table
    pub fn is_context_independant(&self) -> bool {
        use self::Expr::*;
        match *self {
            Label(_) => false,
            _ => true,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(missing_docs)]
pub enum Oper {
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    BinaryAnd,
    BinaryOr,
    BinaryXor,

    Equal,
    LowerOrEqual,
    GreaterOrEqual,
    StrictlyGreater,
    StrictlyLower,
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

            BinaryAnd => write!(format, "&"),
            BinaryOr => write!(format, "|"),
            BinaryXor => write!(format, "^"),

            &Equal => write!(format, "=="),
            &LowerOrEqual => write!(format, "<="),
            &GreaterOrEqual => write!(format, ">="),
            &StrictlyGreater => write!(format, ">"),
            &StrictlyLower => write!(format, "<"),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, format: &mut Formatter<'_>) -> fmt::Result {
        use self::Expr::*;
        match self {
            &Value(val) => write!(format, "0x{:x}", val),
            &String(ref string) => write!(format, "\"{}\"", string),
            &Label(ref label) => write!(format, "{}", label),

            UnaryFunction(func, arg) => {
                write!(format, "{}({})", func, arg)
            },


            BinaryFunction(func, arg1, arg2) => {
                write!(format, "{}({}, {})", func, arg1, arg2)
            },

            &Duration(ref token) => write!(format, "DURATION({})", token),
            &OpCode(ref token) => write!(format, "OPCODE({})", token),

            &Add(ref left, ref right) => write!(format, "{} + {}", left, right),
            &Sub(ref left, ref right) => write!(format, "{} - {}", left, right),
            &Mul(ref left, ref right) => write!(format, "{} * {}", left, right),
            &Mod(ref left, ref right) => write!(format, "{} % {}", left, right),
            &Div(ref left, ref right) => write!(format, "{} / {}", left, right),

            BinaryAnd(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BinaryAnd, right)
            }
            BinaryOr(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BinaryOr, right)
            }
            BinaryXor(ref left, ref right) => {
                write!(format, "{} {} {}", left, Oper::BinaryXor, right)
            }

            &Neg(ref e) => write!(format, "-({})", e),

            &Paren(ref expr) => write!(format, "({})", expr),

            &Equal(ref left, ref right) => write!(format, "{} == {}", left, right),
            &GreaterOrEqual(ref left, ref right) => write!(format, "{} >= {}", left, right),
            &StrictlyGreater(ref left, ref right) => write!(format, "{} > {}", left, right),
            &StrictlyLower(ref left, ref right) => write!(format, "{} < {}", left, right),
            &LowerOrEqual(ref left, ref right) => write!(format, "{} <= {}", left, right),

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

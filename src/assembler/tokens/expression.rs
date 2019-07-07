use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use crate::assembler::assembler::SymbolsTableCaseDependent;
use crate::assembler::tokens::listing::ListingElement;
use crate::assembler::tokens::Token;
use crate::assembler::AssemblerError;

/// Expression nodes.
#[derive(PartialEq, Eq, Clone)]
#[allow(missing_docs)]
pub enum Expr {
    /// 32 bits integer value (should be able to include any integer value manipulated by the assember.
    Value(i32),
    /// String (for db directive)
    String(String),
    /// Label
    Label(String),
    /// This expression node represents the duration of an instruction. The duration is compute at assembling and not at parsing in order to benefit of the symbol table
    Duration(Box<Token>),
    /// This expression node represents an opcode that needs to be assembled in order to produce its binary representation
    OpCode(Box<Token>),

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

    // Functions
    High(Box<Expr>),
    Low(Box<Expr>),
}

impl From<&str> for Expr {
    fn from(src: &str) -> Self {
        Expr::Label(src.to_string())
    }
}
impl From<i32> for Expr {
    fn from(src: i32) -> Self {
        Expr::Value(src)
    }
}
impl From<u8> for Expr {
    fn from(src: u8) -> Self {
        Expr::Value(i32::from(src))
    }
}

/*
impl<N: num::ToPrimitive> From<N> for Expr {
    fn from(src: N) -> Expr {
        Expr::Value(num.to_i32().unwrap())
    }
}

// Currently impossible because of conflict with i32
impl<S:AsRef<str>> From<S> for Expr {
    fn from(src: S) -> Expr {
        Expr::Label(s.as_ref().to_string())
    }
}
*/

#[allow(missing_docs)]
impl Expr {
    pub fn neg(&self) -> Self {
        Expr::Neg(Box::new(self.clone()))
    }

    /// Simple evaluation without context => can only evaluate number based operations.
    pub fn eval(&self) -> Result<i32, AssemblerError> {
        let sym = SymbolsTableCaseDependent::default();
        self.resolve(&sym)
    }

    pub fn resolve(&self, sym: &SymbolsTableCaseDependent) -> Result<i32, AssemblerError> {
        use self::Expr::*;

        let oper = |left: &Self, right: &Self, oper: Oper| -> Result<i32, AssemblerError> {
            let res_left = left.resolve(sym);
            let res_right = right.resolve(sym);

            match (res_left, res_right) {
                (Ok(a), Ok(b)) => match oper {
                    Oper::Add => Ok(a + b),
                    Oper::Sub => Ok(a - b),
                    Oper::Mul => Ok(a * b),
                    Oper::Div => Ok(a / b),
                    Oper::Mod => Ok(a % b),

                    Oper::BinaryAnd => Ok(a & b),
                    Oper::BinaryOr => Ok(a | b),
                    Oper::BinaryXor => Ok(a ^ b),

                    Oper::Equal => Ok((a == b) as i32),

                    Oper::LowerOrEqual => Ok((a <= b) as i32),
                    Oper::StrictlyLower => Ok((a < b) as i32),
                    Oper::GreaterOrEqual => Ok((a >= b) as i32),
                    Oper::StrictlyGreater => Ok((a > b) as i32),
                },
                (Err(a), Ok(_b)) => {
                    Err(format!("Unable to make the operation {:?}: {:?}", oper, a).into())
                }
                (Ok(_a), Err(b)) => {
                    Err(format!("Unable to make the operation {:?}: {:?}", oper, b).into())
                }
                (Err(a), Err(b)) => Err(format!(
                    "Unable to make the operation {:?}: {:?} & {:?}",
                    oper, a, b
                )
                .into()),
            }
        };

        match *self {
            Value(val) => Ok(val),

            String(ref string) => panic!("String values cannot be converted to i32 {}", string),

            Label(ref label) => match sym.value(label) {
                Some(val) => Ok(val),
                None => Err(AssemblerError::UnknownSymbol {
                    symbol: label.to_owned(),
                    closest: sym.closest_symbol(label),
                }),
            },

            Duration(ref token) => {
                let duration = token.estimated_duration()?;
                let duration = duration as i32;
                Ok(duration)
            }

            OpCode(ref token) => {
                let bytes = token.to_bytes()?;
                match bytes.len() {
                    0 => Err(format!("{} is assembled with 0 bytes", token).into()),
                    1 => Ok(i32::from(bytes[0])),
                    2 => Ok(i32::from(bytes[0]) * 256 + i32::from(bytes[1])),
                    val => Err(format!("{} is assembled with {} bytes", token, val).into()),
                }
            }

            Add(ref left, ref right) => oper(left, right, Oper::Add),
            Sub(ref left, ref right) => oper(left, right, Oper::Sub),
            Mul(ref left, ref right) => oper(left, right, Oper::Mul),
            Div(ref left, ref right) => oper(left, right, Oper::Div),
            Mod(ref left, ref right) => oper(left, right, Oper::Mod),

            BinaryAnd(ref left, ref right) => oper(left, right, Oper::BinaryAnd),
            BinaryOr(ref left, ref right) => oper(left, right, Oper::BinaryOr),
            BinaryXor(ref left, ref right) => oper(left, right, Oper::BinaryXor),

            Neg(ref e) => e.resolve(sym).map(|result| -result),

            Equal(ref left, ref right) => oper(left, right, Oper::Equal),
            LowerOrEqual(ref left, ref right) => oper(left, right, Oper::LowerOrEqual),
            GreaterOrEqual(ref left, ref right) => oper(left, right, Oper::GreaterOrEqual),
            StrictlyGreater(ref left, ref right) => oper(left, right, Oper::StrictlyGreater),
            StrictlyLower(ref left, ref right) => oper(left, right, Oper::StrictlyLower),

            Paren(ref e) => e.resolve(sym),

            High(ref inner) => inner.resolve(sym).and_then(|val| Ok((val >> 8) & 0xff)),

            Low(ref inner) => inner.resolve(sym).and_then(|val| Ok(val & 0xff)),
        }
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

#[derive(Copy, Clone, Debug)]
/// Function that can be applied in expressions
pub enum Function {
    /// Computes the high byte of a 16bits number
    Hi,
    /// Computes the low byte of a 16bits number
    Lo,
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

            &High(ref inner) => write!(format, "hi({})", inner),
            &Low(ref inner) => write!(format, "lo({})", inner),
        }
    }
}

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

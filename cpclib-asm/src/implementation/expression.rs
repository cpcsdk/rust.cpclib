use cpclib_tokens::tokens::*;
use cpclib_tokens::symbols::*;
use crate::error::*;

use crate::implementation::tokens::*;

///! Add all important methods to expresison-like structure sthat are not availalbe in the cpclib_tokens crate.

/// Evaluate an aexpression
pub trait ExprEvaluationExt {
    fn eval(&self) -> Result<i32, AssemblerError>;
    fn resolve(&self, sym: &SymbolsTableCaseDependent) -> Result<i32, AssemblerError>;


}

impl ExprEvaluationExt for Expr {
    /// Simple evaluation without context => can only evaluate number based operations.
    fn eval(&self) -> Result<i32, AssemblerError> {
        let sym = SymbolsTableCaseDependent::default();
        self.resolve(&sym)
    }

    fn resolve(&self, sym: &SymbolsTableCaseDependent) -> Result<i32, AssemblerError> {
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
                let bytes = token.as_ref().to_bytes()?;
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
}
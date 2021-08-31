use crate::assembler::Env;
use crate::error::*;
use cpclib_tokens::symbols::*;
use cpclib_tokens::tokens::*;
use cpclib_tokens::ordered_float::OrderedFloat;

use crate::implementation::tokens::*;

///! Add all important methods to expresison-like structure sthat are not availalbe in the cpclib_tokens crate.

/// The result of expression (without taking into account the strings) is either a int (no complex mathematical expression) or a float (division/sinus and so on)

/// Evaluate an aexpression
pub trait ExprEvaluationExt {
    /// Simple evaluation without context => can only evaluate number based operations.
    fn eval(&self) -> Result<ExprResult, AssemblerError> {
        let env = Env::default();
        self.resolve(&env)
    }

    fn resolve(&self, sym: &Env) -> Result<ExprResult, AssemblerError>;
}

impl ExprEvaluationExt for Expr {
    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        let sym = env.symbols();
        use self::Expr::*;

        let oper = |left: &Self, right: &Self, oper: Oper| -> Result<ExprResult, AssemblerError> {
            let res_left = left.resolve(env);
            let res_right = right.resolve(env);

            match (res_left, res_right) {
                (Ok(a), Ok(b)) => match oper {
                    Oper::Add => Ok(a + b),
                    Oper::Sub => Ok(a - b),
                    Oper::Div => Ok(a / b),
                    Oper::Mod => Ok(a % b),
                    Oper::Mul => Ok(a * b),
                    Oper::RightShift => Ok(a >> b),
                    Oper::LeftShift => Ok(a << b),

                    Oper::BinaryAnd => Ok(a & b),
                    Oper::BinaryOr => Ok(a | b),
                    Oper::BinaryXor => Ok(a ^ b),

                    Oper::BooleanAnd => Ok(((a != 0.into()) && (b != 0.into())).into()),
                    Oper::BooleanOr => Ok(((a != 0.into()) || (b != 0.into())).into()),

                    Oper::Equal => Ok((a == b).into()),
                    Oper::Different => Ok((a != b).into()),

                    Oper::LowerOrEqual => Ok((a <= b).into()),
                    Oper::StrictlyLower => Ok((a < b).into()),
                    Oper::GreaterOrEqual => Ok((a >= b).into()),
                    Oper::StrictlyGreater => Ok((a > b).into()),
                },
                (Err(a), Ok(_b)) => {
                    Err(AssemblerError::ExpressionError{msg: format!("Unable to make the operation {:?}: error in left operand {:?}", oper, a)})
                }
                (Ok(_a), Err(b)) => {
                    Err(AssemblerError::ExpressionError{msg: format!("Unable to make the operation {:?}: error in right operand {:?}", oper, b)})
                }
                (Err(a), Err(b)) => Err(AssemblerError::ExpressionError{msg: format!(
                    "Unable to make the operation {:?}: error in both operands {:?} & {:?}",
                    oper, a, b
                )
                }),
            }
        };

        match self {
            RelativeDelta(delta) => Ok((Expr::Label("$".into()).resolve(env)? + delta.clone().into()).into()),

            Value(val) => Ok(val.clone().into()),
            Char(c) => {
                // TODO convert them in another encoding
                Ok(c.clone().into())
            }

            String(ref string) => panic!("String values cannot be converted to i32 {}", string),

            Label(ref label) => match sym.value(label)? {
                Some(cpclib_tokens::symbols::Value::Number(ref val)) => Ok(val.clone().into()),
                Some(cpclib_tokens::symbols::Value::Struct(s)) => Ok(s.len(sym.as_ref()).into()),
                Some(_) => Err(AssemblerError::WrongSymbolType {
                    symbol: label.to_owned(),
                    isnot: "a value".to_owned(),
                }),
                None => Err(AssemblerError::UnknownSymbol {
                    symbol: label.to_owned(),
                    closest: sym.closest_symbol(label, SymbolFor::Integer)?,
                }),
            },

            PrefixedLabel(_prefix, _label) => unimplemented!(
                "Need to add management of the prefix. Not sur the symbol table fits this purpose"
            ),

            Duration(ref token) => {
                let duration = token.estimated_duration()?;
                let duration = duration as i32;
                Ok(duration.into())
            }

            OpCode(ref token) => {
                let bytes = token.as_ref().to_bytes()?;
                match bytes.len() {
                    0 => Err(AssemblerError::ExpressionError{msg:format!("{} is assembled with 0 bytes", token)}),
                    1 => Ok(i32::from(bytes[0]).into()),
                    2 => Ok((i32::from(bytes[0]) * 256 + i32::from(bytes[1])).into()),
                    val => Err(AssemblerError::ExpressionError{msg:format!("{} is assembled with {} bytes", token, val)}),
                }
            }

            RightShift(ref left, ref right) => oper(left, right, Oper::RightShift),
            LeftShift(ref left, ref right) => oper(left, right, Oper::LeftShift),
            Add(ref left, ref right) => oper(left, right, Oper::Add),
            Sub(ref left, ref right) => oper(left, right, Oper::Sub),
            Mul(ref left, ref right) => oper(left, right, Oper::Mul),
            Div(ref left, ref right) => oper(left, right, Oper::Div),
            Mod(ref left, ref right) => oper(left, right, Oper::Mod),

            BinaryAnd(ref left, ref right) => oper(left, right, Oper::BinaryAnd),
            BinaryOr(ref left, ref right) => oper(left, right, Oper::BinaryOr),
            BinaryXor(ref left, ref right) => oper(left, right, Oper::BinaryXor),

            BooleanAnd(ref left, ref right) => oper(left, right, Oper::BooleanAnd),
            BooleanOr(ref left, ref right) => oper(left, right, Oper::BooleanOr),

            Neg(ref e) => e.resolve(env).map(|result| -result),

            Equal(ref left, ref right) => oper(left, right, Oper::Equal),
            Different(ref left, ref right) => oper(left, right, Oper::Different),
            LowerOrEqual(ref left, ref right) => oper(left, right, Oper::LowerOrEqual),
            GreaterOrEqual(ref left, ref right) => oper(left, right, Oper::GreaterOrEqual),
            StrictlyGreater(ref left, ref right) => oper(left, right, Oper::StrictlyGreater),
            StrictlyLower(ref left, ref right) => oper(left, right, Oper::StrictlyLower),

            Paren(ref e) => e.resolve(env),

            UnaryFunction(func, exp) => UnaryFunctionWrapper::new(func, &exp).resolve(env),
            BinaryFunction(func, exp1, exp2) => {
                BinaryFunctionWrapper::new(func, &exp1, &exp2).resolve(env)
            }

            PrefixedLabel(prefix, label) => match sym.prefixed_value(prefix, label)? {
                Some(value) => Ok(value.into()),
                None => Err(AssemblerError::ExpressionError{msg: format!("Unable to obtain {} of {}", prefix, label)}),
            },
            Float(f) => Ok(f.into_inner().into()),
            Rnd =>  unimplemented!("Env need to maintain a counter of call with its value to ensure a consistant generation among the passes")
        }
    }
}

/// utility class for unary function evaluation
struct UnaryFunctionWrapper<'a> {
    func: &'a UnaryFunction,
    arg: &'a Expr,
}

impl<'a> UnaryFunctionWrapper<'a> {
    fn new(func: &'a UnaryFunction, arg: &'a Expr) -> UnaryFunctionWrapper<'a> {
        UnaryFunctionWrapper { func, arg }
    }
}

impl<'a> ExprEvaluationExt for UnaryFunctionWrapper<'a> {
    /// TODO handle float numbers
    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        let arg = self.arg.resolve(env)?;

        match self.func {

            UnaryFunction::Low => Ok((arg >> 8.into()) & 0xff.into()),
            UnaryFunction::High => Ok(arg & 0xff.into()),
            UnaryFunction::Memory => {
                if arg < 0.into() || arg > 0xffff.into() {
                    return Err(AssemblerError::ExpressionError{
                        msg: format!("Impossible to read memory address {}", arg)
                    });
                }
                else {
                    Ok(env.peek(&env.logical_to_physical_address(arg.int() as _)).into())
                }
            },
            UnaryFunction::Floor =>  {
                Ok(arg.floor())
            }
            UnaryFunction::Ceil => {
                Ok(arg.ceil())
            },
            UnaryFunction::Frac => {
                Ok(arg.frac())
            },
            UnaryFunction::Int => {
                Ok(arg.int().into())
            },
            UnaryFunction::Sin => {
                Ok(arg.sin())
            },
            UnaryFunction::Cos => {
                Ok(arg.cos())
            },
            UnaryFunction::ASin => {
                Ok(arg.asin())
            },
            UnaryFunction::ACos => {
                Ok(arg.acos())
            },
            UnaryFunction::Abs => {
                Ok(arg.abs())
            },
            UnaryFunction::Ln => {
                Ok(arg.ln())
            },
            UnaryFunction::Log10 => {
                Ok(arg.log10())
            },
            UnaryFunction::Exp => {
                Ok(arg.exp())
            },
            UnaryFunction::Sqrt => {
                Ok(arg.sqrt())
            },
        }
    }
}

/// utility class for binary function evaluation
struct BinaryFunctionWrapper<'a> {
    func: &'a BinaryFunction,
    arg1: &'a Expr,
    arg2: &'a Expr,
}

impl<'a> BinaryFunctionWrapper<'a> {
    fn new(func: &'a BinaryFunction, arg1: &'a Expr, arg2: &'a Expr) -> BinaryFunctionWrapper<'a> {
        BinaryFunctionWrapper { func, arg1, arg2 }
    }
}

impl<'a> ExprEvaluationExt for BinaryFunctionWrapper<'a> {
    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        let arg1 = self.arg1.resolve(env)?;
        let arg2 = self.arg2.resolve(env)?;

        match self.func {
            BinaryFunction::Min => Ok(arg1.min(arg2)),
            BinaryFunction::Max => Ok(arg1.max(arg2)),
        }
    }

}

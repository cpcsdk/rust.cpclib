use crate::assembler::Env;
use crate::error::*;
use cpclib_tokens::symbols::*;
use cpclib_tokens::tokens::*;

use crate::implementation::tokens::*;

///! Add all important methods to expresison-like structure sthat are not availalbe in the cpclib_tokens crate.

/// Evaluate an aexpression
pub trait ExprEvaluationExt {
    /// Simple evaluation without context => can only evaluate number based operations.
    fn eval(&self) -> Result<i32, AssemblerError> {
        let env = Env::default();
        self.resolve(&env)
    }

    fn resolve(&self, sym: &Env) -> Result<i32, AssemblerError>;
}

impl ExprEvaluationExt for Expr {
    fn resolve(&self, env: &Env) -> Result<i32, AssemblerError> {
        let sym = env.symbols();
        use self::Expr::*;

        let oper = |left: &Self, right: &Self, oper: Oper| -> Result<i32, AssemblerError> {
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

                    Oper::BooleanAnd => Ok(((a != 0) && (b != 0)) as _),
                    Oper::BooleanOr => Ok(((a != 0) || (b != 0)) as _),

                    Oper::Equal => Ok((a == b) as i32),
                    Oper::Different => Ok((a != b) as i32),

                    Oper::LowerOrEqual => Ok((a <= b) as i32),
                    Oper::StrictlyLower => Ok((a < b) as i32),
                    Oper::GreaterOrEqual => Ok((a >= b) as i32),
                    Oper::StrictlyGreater => Ok((a > b) as i32),
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
            RelativeDelta(delta) => Ok(Expr::Label("$".into()).resolve(env)? + *delta as i32),

            Value(val) => Ok(*val),
            Char(c) => {
                // TODO convert them in another encoding
                Ok(*c as i32)
            }

            String(ref string) => panic!("String values cannot be converted to i32 {}", string),

            Label(ref label) => match sym.value(label)? {
                Some(cpclib_tokens::symbols::Value::Integer(ref val)) => Ok(*val),
                Some(cpclib_tokens::symbols::Value::Struct(s)) => Ok(s.len(sym.as_ref())),
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
                Ok(duration)
            }

            OpCode(ref token) => {
                let bytes = token.as_ref().to_bytes()?;
                match bytes.len() {
                    0 => Err(AssemblerError::ExpressionError{msg:format!("{} is assembled with 0 bytes", token)}),
                    1 => Ok(i32::from(bytes[0])),
                    2 => Ok(i32::from(bytes[0]) * 256 + i32::from(bytes[1])),
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
                Some(value) => Ok(value as _),
                None => Err(AssemblerError::ExpressionError{msg: format!("Unable to obtain {} of {}", prefix, label)}),
            },
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
    fn resolve(&self, env: &Env) -> Result<i32, AssemblerError> {
        let arg = self.arg.resolve(env)?;

        match self.func {
            UnaryFunction::Low => Ok((arg >> 8) & 0xff),
            UnaryFunction::High => Ok(arg & 0xff),
            UnaryFunction::Memory => {
                if arg < 0 || arg > 0xffff {
                    return Err(AssemblerError::ExpressionError{
                        msg: format!("Impossible to read memory address {}", arg)
                    });
                }
                else {
                    Ok(env.peek(arg as usize) as i32)
                }
            },
            UnaryFunction::Floor =>  {
                Ok(arg) // TODO really handle floor
            }
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
    fn resolve(&self, env: &Env) -> Result<i32, AssemblerError> {
        let arg1 = self.arg1.resolve(env)?;
        let arg2 = self.arg2.resolve(env)?;

        match self.func {
            BinaryFunction::Min => Ok(arg1.min(arg2)),
            BinaryFunction::Max => Ok(arg2.max(arg2)),
        }
    }
}

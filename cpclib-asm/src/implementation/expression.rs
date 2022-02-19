use std::ops::Neg;

use cpclib_common::itertools::Itertools;
use cpclib_tokens::symbols::*;
use cpclib_tokens::tokens::*;

use crate::assembler::Env;
use crate::error::*;
use crate::implementation::tokens::*;

/// ! Add all important methods to expresison-like structure sthat are not availalbe in the cpclib_tokens crate.

/// The result of expression (without taking into account the strings) is either a int (no complex mathematical expression) or a float (division/sinus and so on)

/// Evaluate an aexpression
pub trait ExprEvaluationExt {
    /// Simple evaluation without context => can only evaluate number based operations.
    fn eval(&self) -> Result<ExprResult, AssemblerError> {
        let env = Env::default();
        self.resolve(&env)
    }

    /// Resolve the expression base on the env context
    fn resolve(&self, sym: &Env) -> Result<ExprResult, AssemblerError>;

    /// Get all the symbols used
    fn symbols_used(&self) -> Vec<&str>;
}

impl ExprEvaluationExt for Expr {
    fn symbols_used(&self) -> Vec<&str> {
        match self {
            Expr::RelativeDelta(_)
            | Expr::Value(_)
            | Expr::Float(_)
            | Expr::Char(_)
            | Expr::Bool(_)
            | Expr::String(_)
            | Expr::Duration(_)
            | Expr::OpCode(_)
            | Expr::Rnd => Vec::new(),

            Expr::Label(label) | Expr::PrefixedLabel(_, label) => vec![label.as_str()],

            Expr::RightShift(a, b)
            | Expr::LeftShift(a, b)
            | Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Mod(a, b)
            | Expr::BinaryAnd(a, b)
            | Expr::BinaryOr(a, b)
            | Expr::BinaryXor(a, b)
            | Expr::BooleanAnd(a, b)
            | Expr::Equal(a, b)
            | Expr::Different(a, b)
            | Expr::LowerOrEqual(a, b)
            | Expr::GreaterOrEqual(a, b)
            | Expr::StrictlyGreater(a, b)
            | Expr::StrictlyLower(a, b)
            | Expr::BinaryFunction(_, a, b)
            | Expr::BooleanOr(a, b) => {
                a.symbols_used()
                    .into_iter()
                    .chain(b.symbols_used().into_iter())
                    .collect_vec()
            }

            Expr::BinaryNot(a) | Expr::Neg(a) | Expr::Paren(a) | Expr::UnaryFunction(_, a) => {
                a.symbols_used()
            }

            Expr::AnyFunction(_, l) | Expr::List(l) => {
                l.iter().map(|e| e.symbols_used()).flatten().collect_vec()
            }
        }
    }

    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        let sym = env.symbols();
        use self::Expr::*;

        let oper = |left: &Self, right: &Self, oper: Oper| -> Result<ExprResult, AssemblerError> {
            let res_left = left.resolve(env);
            let res_right = right.resolve(env);

            match (res_left, res_right) {
                (Ok(a), Ok(b)) => {
                    match oper {
                        Oper::Add => (a + b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        Oper::Sub => (a - b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        Oper::Div => (a / b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        Oper::Mod => (a % b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        Oper::Mul => (a * b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        Oper::RightShift => {
                            (a >> b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }
                        Oper::LeftShift => {
                            (a << b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }

                        Oper::BinaryAnd => {
                            (a & b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }
                        Oper::BinaryOr => {
                            (a | b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }
                        Oper::BinaryXor => {
                            (a ^ b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }

                        Oper::BooleanAnd => Ok(ExprResult::from(a.bool()? && (b.bool()?))),
                        Oper::BooleanOr => Ok(ExprResult::from(a.bool()? || (b.bool()?))),

                        Oper::Equal => Ok((a == b).into()),
                        Oper::Different => Ok((a != b).into()),

                        Oper::LowerOrEqual => Ok((a <= b).into()),
                        Oper::StrictlyLower => Ok((a < b).into()),
                        Oper::GreaterOrEqual => Ok((a >= b).into()),
                        Oper::StrictlyGreater => Ok((a > b).into())
                    }
                }
                (Err(a), Ok(_b)) => {
                    Err(AssemblerError::ExpressionError(ExpressionError::LeftError(
                        oper, box a
                    )))
                }

                (Ok(_a), Err(b)) => {
                    Err(AssemblerError::ExpressionError(
                        ExpressionError::RightError(oper, box b)
                    ))
                }
                (Err(a), Err(b)) => {
                    Err(AssemblerError::ExpressionError(
                        ExpressionError::LeftAndRightError(oper, box a, box b)
                    ))
                }
            }
        };

        match self {
            RelativeDelta(delta) => (Expr::Label("$".into()).resolve(env)? + ExprResult::from(delta.clone())).map_err(|e| AssemblerError::ExpressionTypeError(e)),

            Value(val) => Ok(val.clone().into()),
            Bool(b) => {
                Ok(b.clone().into())
            }
            Char(c) => {
                // TODO convert them in another encoding
                Ok(c.clone().into())
            }

            String(ref string) => Ok(ExprResult::String(string.clone())),
            List(ref l) => Ok(ExprResult::List(l.iter().map(|e| e.resolve(env)).collect::<Result<Vec<_>, _>>()?)),

            Label(ref label) => match sym.value(label)? {
                Some(cpclib_tokens::symbols::Value::Expr(ref val)) => Ok(val.clone().into()),
                Some(cpclib_tokens::symbols::Value::Address(ref val)) => Ok(val.address().into()),
                Some(cpclib_tokens::symbols::Value::Struct(s)) => Ok(s.len(sym).into()),
                Some(cpclib_tokens::symbols::Value::String(ref val)) => Ok(val.into()),
                Some(e) => {dbg!(e); Err(AssemblerError::WrongSymbolType {
                    symbol: label.clone(),
                    isnot: "a value".into(),
                })},
                None => Err(if env.pass().is_first_pass() {
                    // no need to lost time to make the leveinstein search
                    AssemblerError::UnknownSymbol {
                        symbol: label.to_owned(),
                        closest: None,
                    }
                } else {
                    // here it is more problematic
                    AssemblerError::UnknownSymbol {
                        symbol: label.to_owned(),
                        closest: sym.closest_symbol(label, SymbolFor::Number)?,
                    }
                }),
            },

            PrefixedLabel(prefix, label) => {
                let val = env.symbols()
                                        .prefixed_value(prefix, label)?;
                match  val  {
                    Some(val) => Ok(val.into()),
                    None => Err(AssemblerError::AssemblingError {
                        msg: format!("Unable to use prefix {} for {}", prefix, label)
                    })
                }
            },
            Duration(ref token) => {
                let duration = token.estimated_duration()?;
                let duration = duration as i32;
                Ok(duration.into())
            }

            OpCode(ref token) => {
                let bytes = token.clone().to_bytes()?;
                match bytes.len() {
                    0 => Err(
                        AssemblerError::ExpressionError(
                            ExpressionError::OwnError(
                                box AssemblerError::AssemblingError{msg:format!("{} is assembled with 0 bytes", token)}
                            )
                        )
                    ),
                    1 => Ok(i32::from(bytes[0]).into()),
                    2 => Ok((i32::from(bytes[0]) * 256 + i32::from(bytes[1])).into()),
                    val => Err(
                        AssemblerError::ExpressionError(
                            ExpressionError::OwnError(
                                box AssemblerError::AssemblingError{msg:format!("{} is assembled with {} bytes", token, val)}
                            )
                        )
                    )
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
            BinaryNot(ref e) => {
                e.resolve(env)?
                 .binary_not()
                 .map_err(|e| AssemblerError::ExpressionTypeError(e))
            },


            BooleanAnd(ref left, ref right) => oper(left, right, Oper::BooleanAnd),
            BooleanOr(ref left, ref right) => oper(left, right, Oper::BooleanOr),

            Neg(ref e) => (e.resolve(env)?).neg().map_err(|e| AssemblerError::ExpressionTypeError(e)),

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


            Float(f) => Ok(f.into_inner().into()),
            Rnd =>  unimplemented!("Env need to maintain a counter of call with its value to ensure a consistant generation among the passes"),

            AnyFunction(d, expr) => {
                let f = env.any_function(d)?;
                let params = expr.iter()
                            .map(|p| env.resolve_expr_may_fail_in_first_pass(p))
                            .collect::<Result<Vec<ExprResult>, AssemblerError>>()?;
                f.eval(env, &params)
            }
        }
    }
}

/// utility class for unary function evaluation
struct UnaryFunctionWrapper<'a> {
    func: &'a UnaryFunction,
    arg: &'a Expr
}

impl<'a> UnaryFunctionWrapper<'a> {
    fn new(func: &'a UnaryFunction, arg: &'a Expr) -> UnaryFunctionWrapper<'a> {
        UnaryFunctionWrapper { func, arg }
    }
}

impl<'a> ExprEvaluationExt for UnaryFunctionWrapper<'a> {
    fn symbols_used(&self) -> Vec<&str> {
        self.arg.symbols_used()
    }

    /// TODO handle float numbers
    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        let arg = self.arg.resolve(env)?;

        match self.func {
            UnaryFunction::High => {
                ((arg >> 8.into())? & 0xFF.into())
                    .map_err(|e| AssemblerError::ExpressionTypeError(e))
            }
            UnaryFunction::Low => {
                (arg & 0xFF.into()).map_err(|e| AssemblerError::ExpressionTypeError(e))
            }
            UnaryFunction::Memory => {
                if arg < 0.into() || arg > 0xFFFF.into() {
                    return Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                        box AssemblerError::AssemblingError {
                            msg: format!("Impossible to read memory address {}", arg)
                        }
                    )));
                }
                else {
                    Ok(env
                        .peek(&env.logical_to_physical_address(arg.int()? as _))
                        .into())
                }
            }
            UnaryFunction::Floor => {
                (arg.floor()).map_err(|e| AssemblerError::ExpressionTypeError(e))
            }
            UnaryFunction::Ceil => (arg.ceil()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::Frac => (arg.frac()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::Int => {
                (arg.int())
                    .map(|i| i.into())
                    .map_err(|e| AssemblerError::ExpressionTypeError(e))
            }
            UnaryFunction::Sin => (arg.sin()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::Cos => (arg.cos()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::ASin => (arg.asin()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::ACos => (arg.acos()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::Abs => (arg.abs()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::Ln => (arg.ln()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::Log10 => {
                (arg.log10()).map_err(|e| AssemblerError::ExpressionTypeError(e))
            }
            UnaryFunction::Exp => (arg.exp()).map_err(|e| AssemblerError::ExpressionTypeError(e)),
            UnaryFunction::Sqrt => (arg.sqrt()).map_err(|e| AssemblerError::ExpressionTypeError(e))
        }
    }
}

/// utility class for binary function evaluation
struct BinaryFunctionWrapper<'a> {
    func: &'a BinaryFunction,
    arg1: &'a Expr,
    arg2: &'a Expr
}

impl<'a> BinaryFunctionWrapper<'a> {
    fn new(func: &'a BinaryFunction, arg1: &'a Expr, arg2: &'a Expr) -> BinaryFunctionWrapper<'a> {
        BinaryFunctionWrapper { func, arg1, arg2 }
    }
}

impl<'a> ExprEvaluationExt for BinaryFunctionWrapper<'a> {
    fn symbols_used(&self) -> Vec<&str> {
        self.arg1
            .symbols_used()
            .into_iter()
            .chain(self.arg2.symbols_used().into_iter())
            .collect_vec()
    }

    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        let arg1 = self.arg1.resolve(env)?;
        let arg2 = self.arg2.resolve(env)?;

        match self.func {
            BinaryFunction::Min => Ok(arg1.min(arg2)),
            BinaryFunction::Max => Ok(arg1.max(arg2)),
            BinaryFunction::Pow => {
                let power = arg2.int()?;
                match arg1 {
                    ExprResult::Float(f) => Ok(f.into_inner().powf(power as f64).into()),
                    ExprResult::Value(v) => Ok(v.pow(power as _).into()),
                    _ => {
                        Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                            box AssemblerError::AssemblingError {
                                msg: format!("pow cannot be applied to a string")
                            }
                        )))
                    }
                    ExprResult::List(_) => {
                        Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                            box AssemblerError::AssemblingError {
                                msg: format!("pow cannot be applied to a list")
                            }
                        )))
                    }
                }
            }
        }
    }
}

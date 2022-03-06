use cpclib_common::itertools::Itertools;
use cpclib_tokens::symbols::*;
use cpclib_tokens::tokens::*;

use crate::assembler::Env;
use crate::error::{ExpressionError, *};
use crate::implementation::tokens::{TokenExt, *};
use crate::SymbolFor;

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
    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError>;

    /// Get all the symbols used
    fn symbols_used(&self) -> Vec<&str>;
}

#[macro_export]
macro_rules! resolve_impl {
    ($self: ident, $env: ident) => { {
        use std::ops::Neg;
        use cpclib_tokens::symbols::SymbolsTableTrait;


/// utility class for unary function evaluation
struct UnaryFunctionWrapper<'a, E:ExprEvaluationExt> {
    func:  UnaryFunction,
    arg: &'a E
}

impl<'a, E:ExprEvaluationExt> UnaryFunctionWrapper<'a, E> {
    fn new(func:  UnaryFunction, arg: &'a E) -> UnaryFunctionWrapper<'a,E> {
        UnaryFunctionWrapper { func, arg }
    }
}



impl<'a, E:ExprEvaluationExt> ExprEvaluationExt for UnaryFunctionWrapper<'a,E> {


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
struct BinaryFunctionWrapper<'a,  E:ExprEvaluationExt> {
    func: BinaryFunction,
    arg1: &'a E,
    arg2: &'a E
}

impl<'a,  E:ExprEvaluationExt> BinaryFunctionWrapper<'a, E> {
    fn new(func:  BinaryFunction, arg1: &'a E, arg2: &'a E) -> Self {
        BinaryFunctionWrapper { func, arg1, arg2 }
    }
}

impl<'a,  E:ExprEvaluationExt> ExprEvaluationExt for BinaryFunctionWrapper<'a, E> {
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



        let sym = $env.symbols();

        let binary_operation = |left: &Self, right: &Self, oper: cpclib_tokens::BinaryOperation| -> Result<ExprResult, AssemblerError> {
            let res_left = left.resolve($env);
            let res_right = right.resolve($env);

            match (res_left, res_right) {
                (Ok(a), Ok(b)) => {
                    match oper {
                        cpclib_tokens::BinaryOperation::Add => (a + b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::Sub => (a - b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::Div => (a / b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::Mod => (a % b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::Mul => (a * b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::RightShift => {
                            (a >> b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }
                        cpclib_tokens::BinaryOperation::LeftShift => {
                            (a << b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }

                        cpclib_tokens::BinaryOperation::BinaryAnd => {
                            (a & b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }
                        cpclib_tokens::BinaryOperation::BinaryOr => {
                            (a | b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }
                        cpclib_tokens::BinaryOperation::BinaryXor => {
                            (a ^ b).map_err(|e| AssemblerError::ExpressionTypeError(e))
                        }

                        cpclib_tokens::BinaryOperation::BooleanAnd => Ok(ExprResult::from(a.bool()? && (b.bool()?))),
                        cpclib_tokens::BinaryOperation::BooleanOr => Ok(ExprResult::from(a.bool()? || (b.bool()?))),

                        cpclib_tokens::BinaryOperation::Equal => Ok((a == b).into()),
                        cpclib_tokens::BinaryOperation::Different => Ok((a != b).into()),

                        cpclib_tokens::BinaryOperation::LowerOrEqual => Ok((a <= b).into()),
                        cpclib_tokens::BinaryOperation::StrictlyLower => Ok((a < b).into()),
                        cpclib_tokens::BinaryOperation::GreaterOrEqual => Ok((a >= b).into()),
                        cpclib_tokens::BinaryOperation::StrictlyGreater => Ok((a > b).into())
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

        if $self.is_binary_operation() {
            binary_operation($self.arg1(), $self.arg2(), $self.binary_operation())
        }
        else if $self.is_paren() {
            let e = $self.arg1();
            e.resolve($env)
        }
        else if $self.is_relative() {
            (Expr::Label("$".into()).resolve($env)? + ExprResult::from($self.relative_delta()))
                .map_err(|e| AssemblerError::ExpressionTypeError(e))
        }
        else if $self.is_value(){
            Ok($self.value().into())
        }
        else if $self.is_char() {
            Ok($self.char().into())
        }
        else if $self.is_bool() {
            Ok($self.bool().into())
        } else if $self.is_string() {
            Ok(ExprResult::String($self.string().into()))
        }
        else if $self.is_float() {
            Ok($self.float().into_inner().into())
        }
        else if $self.is_list() {
            Ok(ExprResult::List(
                $self.list().iter()
                    .map(|e| e.resolve($env))
                    .collect::<Result<Vec<_>, _>>()?
                )
            )
        }
        else if $self.is_label() {
            let label = $self.label();
            match sym.value(label)? {
                Some(cpclib_tokens::symbols::Value::Expr(ref val)) => Ok(val.clone().into()),
                Some(cpclib_tokens::symbols::Value::Address(ref val)) => Ok(val.address().into()),
                Some(cpclib_tokens::symbols::Value::Struct(s)) => Ok(s.len(sym).into()),
                Some(cpclib_tokens::symbols::Value::String(ref val)) => Ok(val.into()),
                Some(e) => {dbg!(e); Err(AssemblerError::WrongSymbolType {
                    symbol: label.into(),
                    isnot: "a value".into(),
                })},
                None => Err(if $env.pass().is_first_pass() {
                    // no need to lost time to make the leveinstein search
                    AssemblerError::UnknownSymbol {
                        symbol: label.into(),
                        closest: None,
                    }
                } else {
                    // here it is more problematic
                    AssemblerError::UnknownSymbol {
                        symbol: label.into(),
                        closest: sym.closest_symbol(label, SymbolFor::Number)?,
                    }
                })
            }

        }
        else if $self.is_prefix_label() {
            let label = $self.label();
            let prefix = $self.prefix();

            let val = $env.symbols()
                                    .prefixed_value(prefix, label)?;
            match  val  {
                Some(val) => Ok(val.into()),
                None => Err(AssemblerError::AssemblingError {
                    msg: format!("Unable to use prefix {} for {}", prefix, label)
                })
            }
        }

        else if $self.is_token_operation() {
            let token = $self.token();
            match $self.token_operation() {
                cpclib_tokens::UnaryTokenOperation::Duration => {
                    let duration = token.estimated_duration()?;
                    let duration = duration as i32;
                    Ok(duration.into())
                },

                cpclib_tokens::UnaryTokenOperation::Opcode => {
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
            }
        }
        else if $self.is_unary_operation() {
            let e = $self.arg1();

            match $self.unary_operation() {
                cpclib_tokens::UnaryOperation::Not => {
                    e.resolve($env)?
                    .binary_not()
                    .map_err(|e| AssemblerError::ExpressionTypeError(e))
                },
                cpclib_tokens::UnaryOperation::Neg => {
                    (e.resolve($env)?)
                        .neg()
                        .map_err(|e| AssemblerError::ExpressionTypeError(e))
                }
            }
        }
        else if $self.is_unary_function() {
            let func = $self.unary_function();
            let arg = $self.arg1();
            UnaryFunctionWrapper::new(func, arg).resolve($env)
        }
        else if $self.is_binary_function() {
            let func = $self.binary_function();
            let arg1 = $self.arg1();
            let arg2 = $self.arg1();
            BinaryFunctionWrapper::new(func, arg1, arg2).resolve($env)
        }

        else if $self.is_rnd() {
            unimplemented!("Env need to maintain a counter of call with its value to ensure a consistant generation among the passes")
        }
        else if $self.is_any_function(){
            let d = $self.function_name();
            let expr = $self.function_args();

            let f = $env.any_function(d)?;
            let params = expr.iter()
                        .map(|p| $env.resolve_expr_may_fail_in_first_pass(p))
                        .collect::<Result<Vec<ExprResult>, AssemblerError>>()?;
            f.eval($env, &params)


        } else {
            unreachable!()
        }
    }
    };
}

impl ExprEvaluationExt for Expr {
    /// XXX Be sure it is well synchronized with LocatedExpr
    fn symbols_used(&self) -> Vec<&str> {
        match self {
            Expr::RelativeDelta(_)
            | Expr::Value(_)
            | Expr::Float(_)
            | Expr::Char(_)
            | Expr::Bool(_)
            | Expr::String(_)
            | Expr::Rnd => Vec::new(),

            Expr::Label(label) | Expr::PrefixedLabel(_, label) => vec![label.as_str()],

            Expr::BinaryFunction(_, box a, box b) | Expr::BinaryOperation(_, box a, box b) => {
                a.symbols_used()
                    .into_iter()
                    .chain(b.symbols_used().into_iter())
                    .collect_vec()
            }

            Expr::Paren(a) | Expr::UnaryFunction(_, a) | Expr::UnaryOperation(_, a) => {
                a.symbols_used()
            }

            Expr::AnyFunction(_, l) | Expr::List(l) => {
                l.iter().map(|e| e.symbols_used()).flatten().collect_vec()
            }

            Expr::UnaryTokenOperation(_, box t) => {
                unimplemented!("Need to retreive the symbols from the operation")

            }
        }
    }

    fn resolve(&self, env: &Env) -> Result<ExprResult, AssemblerError> {
        resolve_impl!(self, env)
    }
}

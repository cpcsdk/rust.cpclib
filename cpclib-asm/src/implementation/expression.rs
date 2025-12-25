use std::fmt::Display;

use cpclib_common::itertools::Itertools;
use cpclib_tokens::tokens::*;

use crate::SymbolFor;
use crate::assembler::Env;
use crate::error::{ExpressionError, *};
use crate::implementation::tokens::TokenExt;

/// XXX Orgams only handles integer values and strings
/// TODO call it somewhere in the expression evaluation
/// because it seesm not use anymore since various refactoring
pub fn ensure_orgams_type(e: ExprResult, env: &Env) -> Result<ExprResult, Box<AssemblerError>> {
    let e = if env.options().parse_options().is_orgams() {
        match &e {
            ExprResult::Float(_)
            | ExprResult::Value(_)
            | ExprResult::Char(_)
            | ExprResult::Bool(_) => ExprResult::Value(e.int()?),
            ExprResult::String(_s) => e,
            _ => {
                return Err(Box::new(AssemblerError::AlreadyRenderedError(format!(
                    "Incompatible type with orgams {e:?}"
                ))));
            }
        }
    }
    else {
        e
    };

    Ok(e)
}

/// Add all important methods to expression-like structures that are not available in the cpclib_tokens crate.

/// The result of expression (without taking into account the strings) is either a int (no complex mathematical expression) or a float (division/sinus and so on)

/// Evaluate an expression
pub trait ExprEvaluationExt: Display {
    /// Simple evaluation without context => can only evaluate number based operations.
    fn eval(&self) -> Result<ExprResult, Box<AssemblerError>> {
        let mut env = Env::default();
        self.resolve(&mut env)
    }

    /// Resolve the expression base on the env context
    fn resolve(&self, env: &mut Env) -> Result<ExprResult, Box<AssemblerError>>;

    /// Get all the symbols used
    fn symbols_used(&self) -> Vec<&str>;
}

#[macro_export]
macro_rules! resolve_impl {

    ($self: ident, $env: ident) => { {
        use std::ops::Neg;
        use cpclib_tokens::symbols::SymbolsTableTrait;

        let mut binary_operation = |left: &Self, right: &Self, oper: cpclib_tokens::BinaryOperation| -> Result<ExprResult, Box<AssemblerError>> {
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
                        oper, a
                    )))
                }

                (Ok(_a), Err(b)) => {

                    Err(AssemblerError::ExpressionError(
                        ExpressionError::RightError(oper, b)
                    ))
                }
                (Err(a), Err(b)) => {
                    Err(AssemblerError::ExpressionError(
                        ExpressionError::LeftAndRightError(oper, a, b)
                    ))
                }
            }.map_err(|e| Box::new(e))
        };

        if $self.is_binary_operation() {
            binary_operation($self.arg1(), $self.arg2(), $self.binary_operation())
        }
        else if $self.is_paren() {
            let e = $self.arg1();
            e.resolve($env)
        }
        else if $self.is_relative() {
            Ok((Expr::Label("$".into()).resolve($env)? + ExprResult::from($self.relative_delta()))
                .map_err(|e| AssemblerError::ExpressionTypeError(e))?)
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
            match  $env.symbols().any_value(label)?.map(|vl| vl.value()) {
                Some(cpclib_tokens::symbols::Value::Expr( val)) => Ok(val.clone().into()),
                Some(cpclib_tokens::symbols::Value::Address( val)) => Ok(val.address().into()),
                Some(cpclib_tokens::symbols::Value::Struct(s)) => Ok(s.len($env.symbols()).into()),
                Some(cpclib_tokens::symbols::Value::String( val)) => Ok(val.into()),
                Some(_e) => { Err(AssemblerError::WrongSymbolType {
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
                        closest:  $env.symbols().closest_symbol(label, SymbolFor::Number)?.map(|s| s.into()),
                    }
                })
            }.map_err(|e| Box::new(e))

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
                }).map_err(|e| Box::new(e))
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
                    let bytes = token.to_bytes()?;
                    match bytes.len() {
                        0 => Err(
                            AssemblerError::ExpressionError(
                                ExpressionError::OwnError(
                                    Box::new(AssemblerError::AssemblingError{msg:format!("{} is assembled with 0 bytes", token)})
                                )
                            )
                        ),
                        1 => Ok(i32::from(bytes[0]).into()),
                        2 => Ok((i32::from(bytes[0]) * 256 + i32::from(bytes[1])).into()),
                        val => Err(
                            AssemblerError::ExpressionError(
                                ExpressionError::OwnError(
                                    Box::new(AssemblerError::AssemblingError{msg:format!("{} is assembled with {} bytes", token, val)})
                                )
                            )
                        )
                    }
                }
            }.map_err(|e| Box::new(e))
        }
        else if $self.is_unary_operation() {
            let e = $self.arg1();

            match $self.unary_operation() {
                cpclib_tokens::UnaryOperation::BinaryNot => {
                    e.resolve($env)?
                    .binary_not()
                    .map_err(|e| AssemblerError::ExpressionTypeError(e))
                },
                cpclib_tokens::UnaryOperation::Not => {
                    e.resolve($env)?
                    .not()
                    .map_err(|e| AssemblerError::ExpressionTypeError(e))
                },
                cpclib_tokens::UnaryOperation::Neg => {
                    (e.resolve($env)?)
                        .neg()
                        .map_err(|e| AssemblerError::ExpressionTypeError(e))
                }
            }.map_err(|e| Box::new(e))
        }

        else if $self.is_rnd() {
            unimplemented!("Env need to maintain a counter of call with its value to ensure a consistant generation among the passes")
        }
        else if $self.is_any_function(){
            let d = $self.function_name();
            let expr = $self.function_args();


            let mut params = Vec::with_capacity(expr.len());
            for p in expr.iter() {
                let v = $env.resolve_expr_may_fail_in_first_pass(p) ?;
                params.push(v);
            }

            $env.eval_any_function(d, &params)

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

            Expr::Paren(a) | Expr::UnaryOperation(_, a) => a.symbols_used(),

            Expr::AnyFunction(_, l) | Expr::List(l) => {
                l.iter().flat_map(|e| e.symbols_used()).collect_vec()
            },

            Expr::BinaryOperation(_, left, right) => {
                let mut syms = left.symbols_used();
                syms.extend(right.symbols_used());
                syms
            },

            _ => {
                unimplemented!("Need to retreive the symbols from the operation")
            }
        }
    }

    fn resolve(&self, env: &mut Env) -> Result<ExprResult, Box<AssemblerError>> {
        resolve_impl!(self, env)
    }
}

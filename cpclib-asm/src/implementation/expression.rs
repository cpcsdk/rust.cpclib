use std::borrow::Cow;
use std::fmt::Display;

use cpclib_common::itertools::Itertools;
use cpclib_tokens::tokens::*;

use crate::SymbolFor;
use crate::assembler::Env;
use crate::error::{ExpressionError, *};
use crate::implementation::tokens::TokenExt;

/// Orgams only handles integer values and strings. Called unconditionally
/// from `LocatedExpr::resolve` on every resolved expression - a no-op
/// outside Orgams-compatibility mode (`is_orgams()` gates the whole body).
pub fn ensure_orgams_type(e: ExprResult, env: &mut Env) -> Result<ExprResult, Box<AssemblerError>> {
    let e = if env.options().parse_options().is_orgams() {
        match &e {
            ExprResult::Float(_)
            | ExprResult::Value(_)
            | ExprResult::Char(_)
            | ExprResult::Bool(_) => ExprResult::Value(env.int_forward(&e)?),
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
///
/// The result of expression (without taking into account the strings) is either a int (no complex mathematical expression) or a float (division/sinus and so on)
///
/// Evaluate an expression
pub trait ExprEvaluationExt: Display {
    /// Simple evaluation without context => can only evaluate number based operations.
    fn eval(&self) -> Result<ExprResult, Box<AssemblerError>> {
        let mut env = Env::default();
        self.resolve(&mut env)
    }

    /// Resolve the expression base on the env context
    fn resolve(&self, env: &mut Env) -> Result<ExprResult, Box<AssemblerError>>;

    /// Get all the symbols used (returns Cow to use references where possible, owned strings when needed)
    fn symbols_used(&self) -> Vec<Cow<'_, str>>;

    fn r#type(&self) -> &str;
}

impl<T> ExprEvaluationExt for Box<T>
where T: ExprEvaluationExt + ?Sized
{
    fn resolve(&self, env: &mut Env) -> Result<ExprResult, Box<AssemblerError>> {
        (**self).resolve(env)
    }

    fn symbols_used(&self) -> Vec<Cow<'_, str>> {
        (**self).symbols_used()
    }

    fn r#type(&self) -> &str {
        (**self).r#type()
    }
}

#[macro_export]
macro_rules! resolve_impl {

    ($self: ident, $env: ident) => { {
        use std::ops::Neg;
        use cpclib_tokens::symbols::SymbolsTableTrait;

 //       if let Ok(value) = cpclib_tokens::tokens::try_eval_expr_without_context($self.to_expr().as_ref()) {
 //           return Ok(value);
 //       }

        let mut binary_operation = |left: &Self, right: &Self, oper: cpclib_tokens::BinaryOperation| -> Result<ExprResult, Box<AssemblerError>> {
            // `&&`/`||` short-circuit: the right operand is only resolved
            // when its value could actually change the result, matching
            // every other language's boolean operators - avoids needless
            // work (and a spurious error/warning from a right-hand side
            // that a decided left side already made moot) for guards like
            // `defined(X) && uses(X)`.
            if matches!(oper, cpclib_tokens::BinaryOperation::BooleanAnd | cpclib_tokens::BinaryOperation::BooleanOr) {
                let left_bool = left.resolve($env)
                    .map_err(|e| Box::new(AssemblerError::ExpressionError(ExpressionError::LeftError(oper, e))))?
                    .bool()
                    .map_err(AssemblerError::ExpressionTypeError)
                    .map_err(Box::new)?;
                let short_circuits_on = oper == cpclib_tokens::BinaryOperation::BooleanOr;
                if left_bool == short_circuits_on {
                    return Ok(ExprResult::from(left_bool));
                }
                let right_bool = right.resolve($env)
                    .map_err(|e| Box::new(AssemblerError::ExpressionError(ExpressionError::RightError(oper, e))))?
                    .bool()
                    .map_err(AssemblerError::ExpressionTypeError)
                    .map_err(Box::new)?;
                return Ok(ExprResult::from(right_bool));
            }

            let res_left = left.resolve($env);
            let res_right = right.resolve($env);

            match (res_left, res_right) {
                (Ok(a), Ok(b)) => {
                    match oper {
                        cpclib_tokens::BinaryOperation::Add => (a + b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::Sub => (a - b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::Div => (a / b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::IntDiv => {
                            a.int_div(b)
                                .map(|(v, warnings)| { $env.add_expression_warnings(warnings); v })
                                .map_err(AssemblerError::ExpressionTypeError)
                        },
                        cpclib_tokens::BinaryOperation::Mod => (a % b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::Mul => (a * b).map_err(|e| AssemblerError::ExpressionTypeError(e)),
                        cpclib_tokens::BinaryOperation::RightShift => {
                            a.shr_checked(b)
                                .map(|(v, warnings)| { $env.add_expression_warnings(warnings); v })
                                .map_err(AssemblerError::ExpressionTypeError)
                        }
                        cpclib_tokens::BinaryOperation::LeftShift => {
                            a.shl_checked(b)
                                .map(|(v, warnings)| { $env.add_expression_warnings(warnings); v })
                                .map_err(AssemblerError::ExpressionTypeError)
                        }

                        cpclib_tokens::BinaryOperation::BinaryAnd => {
                            a.bitand_checked(b)
                                .map(|(v, warnings)| { $env.add_expression_warnings(warnings); v })
                                .map_err(AssemblerError::ExpressionTypeError)
                        }
                        cpclib_tokens::BinaryOperation::BinaryOr => {
                            a.bitor_checked(b)
                                .map(|(v, warnings)| { $env.add_expression_warnings(warnings); v })
                                .map_err(AssemblerError::ExpressionTypeError)
                        }
                        cpclib_tokens::BinaryOperation::BinaryXor => {
                            a.bitxor_checked(b)
                                .map(|(v, warnings)| { $env.add_expression_warnings(warnings); v })
                                .map_err(AssemblerError::ExpressionTypeError)
                        }

                        cpclib_tokens::BinaryOperation::BooleanAnd | cpclib_tokens::BinaryOperation::BooleanOr => {
                            unreachable!("short-circuited above, before left/right were both eagerly resolved")
                        },

                        cpclib_tokens::BinaryOperation::Equal => {
                            let (eq, warnings) = a.eq_checked(&b);
                            $env.add_expression_warnings(warnings);
                            Ok(eq.into())
                        },
                        cpclib_tokens::BinaryOperation::Different => {
                            let (eq, warnings) = a.eq_checked(&b);
                            $env.add_expression_warnings(warnings);
                            Ok((!eq).into())
                        },

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

        // `is_value`/`is_label` checked first - a plain literal or label
        // reference is the single most common leaf node in real source, so
        // resolving it shouldn't pay for several other, rarer checks first
        // (the checks are mutually exclusive by construction, so reordering
        // changes nothing but which is found fastest).
        if $self.is_value(){
            Ok($self.value().into())
        }
        else if $self.is_label() {
            let label = $self.label();
            let value = $env.symbols().any_value(label)?;
            match value.map(|vl| vl.value()) {
                Some(cpclib_tokens::symbols::Value::Expr( val)) => Ok(val.clone().into()),
                Some(cpclib_tokens::symbols::Value::Address( val)) => Ok(val.address().into()),
                Some(cpclib_tokens::symbols::Value::Struct(s)) => Ok(s.len($env.symbols()).into()),
                Some(cpclib_tokens::symbols::Value::String( val)) => Ok(val.into()),
                // Error cases: expand label for clear error messages (e.g., show "label_0" not "label_{{idx}}")
                error_case => {
                    let expanded_label = $env.symbols().extend_local_and_patterns_for_symbol(label)?;
                    match error_case {
                        Some(_e) => Err(AssemblerError::WrongSymbolType {
                            symbol: expanded_label.into(),
                            isnot: "a value".into(),
                        }),
                        None => Err(if $env.pass().is_first_pass() {
                            // no need to lost time to make the leveinstein search
                            AssemblerError::UnknownSymbol {
                                symbol: expanded_label.into(),
                                closest: None,
                            }
                        } else {
                            // here it is more problematic
                            AssemblerError::UnknownSymbol {
                                symbol: expanded_label.clone().into(),
                                closest:  $env.symbols().closest_symbol(expanded_label.value(), SymbolFor::Number)?.map(|s| s.into()),
                            }
                        })
                    }
                }
            }.map_err(|e| Box::new(e))

        }
        else if $self.is_binary_operation() {
            binary_operation($self.arg1(), $self.arg2(), $self.binary_operation())
        }
        else if $self.is_ternary() {
            let condition = $self.ternary_condition().resolve($env)?;
            if condition.bool()? {
                $self.ternary_true().resolve($env)
            } else {
                $self.ternary_false().resolve($env)
            }
        }
        else if $self.is_paren() {
            let e = $self.arg1();
            e.resolve($env)
        }
        else if $self.is_relative() {
            Ok((Expr::Label("$".into()).resolve($env)? + ExprResult::from($self.relative_delta()))
                .map_err(|e| AssemblerError::ExpressionTypeError(e))?)
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
                    .into()
                )
            )
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
                    if bytes.is_empty() {
                        Err(
                            AssemblerError::ExpressionError(
                                ExpressionError::OwnError(
                                    Box::new(AssemblerError::AssemblingError{msg:format!("{} is assembled with 0 bytes", token)})
                                )
                            )
                        )
                    } else {
                        // Always return a list (convertible to int when length is 1)
                        let byte_list: Vec<ExprResult> = bytes.iter()
                            .map(|&b| ExprResult::Value(i32::from(b)))
                            .collect();
                        Ok(ExprResult::List(byte_list.into()))
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
    fn symbols_used(&self) -> Vec<Cow<'_, str>> {
        match self {
            Expr::RelativeDelta(_)
            | Expr::Value(_)
            | Expr::Float(_)
            | Expr::Char(_)
            | Expr::Bool(_)
            | Expr::String(_)
            | Expr::Rnd => Vec::new(),

            Expr::Label(label) | Expr::PrefixedLabel(_, label) => {
                vec![Cow::Borrowed(label.as_str())]
            },

            Expr::Paren(a) | Expr::UnaryOperation(_, a) => a.symbols_used(),

            Expr::AnyFunction(_, l) | Expr::List(l) => {
                l.iter().flat_map(|e| e.symbols_used()).collect_vec()
            },

            Expr::BinaryOperation(_, left, right) => {
                let mut syms = left.symbols_used();
                syms.extend(right.symbols_used());
                syms
            },

            Expr::UnaryTokenOperation(_op, token) => {
                // Extract symbols from the token's expressions (e.g., duration(ld a, (label)))
                // Token.symbols() returns HashSet<String>, so we need Cow::Owned here
                use cpclib_tokens::ListingElement;
                token.symbols().into_iter().map(Cow::Owned).collect()
            },

            Expr::Ternary(cond, true_expr, false_expr) => {
                let mut syms = cond.symbols_used();
                syms.extend(true_expr.symbols_used());
                syms.extend(false_expr.symbols_used());
                syms
            }
        }
    }

    fn resolve(&self, env: &mut Env) -> Result<ExprResult, Box<AssemblerError>> {
        resolve_impl!(self, env)
    }

    fn r#type(&self) -> &str {
        match self {
            Expr::RelativeDelta(_) => "relative_delta",
            Expr::Value(_) => "value",
            Expr::Char(_) => "char",
            Expr::Bool(_) => "bool",
            Expr::String(_) => "string",
            Expr::Float(_) => "float",
            Expr::Label(_) => "label",
            Expr::PrefixedLabel(..) => "prefixed_label",
            Expr::Paren(_) => "paren",
            Expr::UnaryOperation(..) => "unary_operation",
            Expr::BinaryOperation(..) => "binary_operation",
            Expr::AnyFunction(name, _) => name.as_str(),
            Expr::List(_) => "list",
            Expr::Rnd => "rnd",
            _ => "unknown"
        }
    }
}

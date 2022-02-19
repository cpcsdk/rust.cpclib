use std::ops::Deref;

use cpclib_common::itertools::{EitherOrBoth, Itertools};
use cpclib_tokens::symbols::{Macro, Source, Struct};
use cpclib_tokens::{MacroParam, Token};

use crate::error::AssemblerError;
use crate::preamble::Z80Span;
use crate::Env;

/// To be implemented for each element that can be expended based on some patterns (i.e. macros, structs)
pub trait Expandable {
    /// Returns a string version of the element after expansion
    fn expand(&self, env: &Env) -> Result<String, AssemblerError>;
}

impl Expandable for MacroParam {
    /// Expansion is slightly different thant to_string has it does not print the bracket
    fn expand(&self, env: &Env) -> Result<String, AssemblerError> {
        match self {
            Self::Single(s) => {
                let trimmed = s.trim();
                const EVAL: &str = "{eval}";
                if trimmed.starts_with(EVAL) {
                    let src = &s[EVAL.len()..];
                    let src = Z80Span::from(src);
                    let expr_token = crate::parser::expr(src)
                        .map_err(|e| AssemblerError::AssemblingError { msg: e.to_string() })?
                        .1;
                    let value = env.resolve_expr_must_never_fail(&expr_token)?;
                    return Ok(value.to_string());
                }
                else {
                    Ok(s.clone())
                }
            }
            Self::List(l) => {
                Ok(format!(
                    "{}",
                    l.iter()
                        .map(|p| p.expand(env))
                        .collect::<Result<Vec<_>, AssemblerError>>()?
                        .join(",")
                ))
            }
        }
    }
}

/// Encodes both the arguments and the macro
pub struct MacroWithArgs<'m, 'a> {
    r#macro: &'m Macro,
    args: &'a [MacroParam]
}

impl<'m, 'a> MacroWithArgs<'m, 'a> {
    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#macro: &'m Macro, args: &'a [MacroParam]) -> Result<Self, AssemblerError> {
        if r#macro.nb_args() != args.len() {
            Err(AssemblerError::MacroError {
                name: r#macro.name().into(),
                root: box AssemblerError::AssemblingError {
                    msg: format!(
                        "{} arguments provided, but {} expected.",
                        args.len(),
                        r#macro.nb_args()
                    )
                }
            })
        }
        else {
            Ok(Self { r#macro, args })
        }
    }

    pub fn source(&self) -> Option<&Source> {
        self.r#macro.source()
    }
}

impl<'m, 'a> Expandable for MacroWithArgs<'m, 'a> {
    /// Develop the macro with the given arguments
    fn expand(&self, env: &Env) -> Result<String, AssemblerError> {
        //        assert_eq!(args.len(), self.nb_args());
        let mut listing = self.r#macro.code().to_string();

        // replace the arguments for the listing
        for (argname, argvalue) in self.r#macro.params().iter().zip(self.args.iter()) {
            listing = listing.replace(&format!("{{{}}}", argname), &argvalue.expand(env)?);
        }

        Ok(listing)
    }
}

pub struct StructWithArgs<'s, 'a> {
    r#struct: &'s Struct,
    args: &'a [MacroParam]
}

impl<'s, 'a> StructWithArgs<'s, 'a> {
    pub fn r#struct(&self) -> &Struct {
        self.r#struct
    }

    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#struct: &'s Struct, args: &'a [MacroParam]) -> Result<Self, AssemblerError> {
        if r#struct.nb_args() < args.len() {
            Err(AssemblerError::MacroError {
                name: r#struct.name().into(),
                root: box AssemblerError::AssemblingError {
                    msg: format!(
                        "{} arguments provided, but at most {} expected.",
                        args.len(),
                        r#struct.nb_args()
                    )
                }
            })
        }
        else {
            Ok(Self { r#struct, args })
        }
    }

    pub fn source(&self) -> Option<&Source> {
        self.r#struct.source()
    }
}

impl<'s, 'a> Expandable for StructWithArgs<'s, 'a> {
    /// Generate the token that correspond to the current structure
    /// Current bersion does not handle at all directive with several arguments
    /// BUG does not work when directives have a prefix
    fn expand(&self, env: &Env) -> Result<String, AssemblerError> {
        //        dbg!("{:?} != {:?}", self.args, self.r#struct().content());

        let prefix = ""; // TODO acquire this prefix

        // self.args has priority over self.content information
        let mut developped: String = self
            .r#struct()
            .content()
            .iter()
            .zip_longest(self.args.iter()) // by construction longest is the struct template
            .enumerate()
            .map(
                |(_idx, current_information)| -> Result<String, AssemblerError> {
                    let ((_name, token), provided_param) = {
                        match current_information {
                            EitherOrBoth::Both((name, token), provided_param) => {
                                ((name, token), Some(provided_param))
                            }
                            EitherOrBoth::Left((name, token)) => ((name, token), None),
                            _ => unreachable!()
                        }
                    };

                    match token {
                        Token::Defb(c) | Token::Defw(c) => {
                            assert_eq!(c.len(), 1);

                            let tok = if matches!(token, Token::Defb(_)) {
                                "DB"
                            }
                            else {
                                "DW"
                            };

                            let elem = match provided_param {
                                Some(provided_param) => {
                                    let elem = provided_param.expand(env)?;
                                    if elem.is_empty() {
                                        c[0].to_string()
                                    }
                                    else {
                                        elem
                                    }
                                }
                                None => c[0].to_string()
                            };
                            Ok(format!(" {}{} {}", prefix, tok, elem))
                        }

                        Token::MacroCall(r#macro, current_default_args) => {
                            let mut call = format!(" {}{} ", prefix, r#macro);

                            let args = match provided_param {
                                Some(MacroParam::Single(..)) => {
                                    provided_param
                                        .into_iter()
                                        .zip_longest(current_default_args)
                                        .map(|a| {
                                            match a {
                                                EitherOrBoth::Both(provided, _)
                                                | EitherOrBoth::Left(provided) => provided,
                                                EitherOrBoth::Right(default) => default
                                            }
                                        })
                                        .map(|a| {
                                            a.expand(env).map(|repr| {
                                                if a.is_single() {
                                                    repr
                                                }
                                                else {
                                                    format!("[{}]", repr)
                                                }
                                            })
                                        })
                                        .collect::<Result<Vec<String>, AssemblerError>>()?
                                }
                                Some(MacroParam::List(l)) => {
                                    l.into_iter()
                                        .zip_longest(current_default_args)
                                        .map(|a| {
                                            match a {
                                                EitherOrBoth::Both(provided, _)
                                                | EitherOrBoth::Left(provided) => provided.deref(),
                                                EitherOrBoth::Right(default) => default
                                            }
                                        })
                                        .map(|a| {
                                            a.expand(env).map(|repr| {
                                                if a.is_single() {
                                                    repr
                                                }
                                                else {
                                                    format!("[{}]", repr)
                                                }
                                            })
                                        })
                                        .collect::<Result<Vec<String>, AssemblerError>>()?
                                }
                                None => {
                                    current_default_args
                                        .iter()
                                        .map(|a| a.expand(env))
                                        .collect::<Result<Vec<String>, AssemblerError>>()?
                                }
                            };
                            call.push_str(&args.join(","));
                            Ok(call)
                        }
                        _ => unreachable!("{:?}", token)
                    }
                }
            )
            .collect::<Result<Vec<String>, AssemblerError>>()?
            .join("\n");

        let last = developped.pop().unwrap();
        developped.push(last);
        if last != 'n' {
            developped.push('\n');
        }
        dbg!(Ok(developped))
    }
}

use std::ops::Deref;


use cpclib_common::itertools::{EitherOrBoth, Itertools};
use cpclib_common::winnow::Parser;
use cpclib_tokens::symbols::{Macro, Source, Struct};
use cpclib_tokens::{MacroParamElement, Token};

use crate::error::AssemblerError;
use crate::preamble::{Z80ParserError, Z80Span};
use crate::Env;

/// To be implemented for each element that can be expended based on some patterns (i.e. macros, structs)
pub trait Expandable {
    /// Returns a string version of the element after expansion
    fn expand(&self, env: &Env) -> Result<String, AssemblerError>;
}

fn expand_param<P: MacroParamElement>(m: &P, env: &Env) -> Result<String, AssemblerError> {
    if m.is_single() {
        let s = m.single_argument();
        let trimmed = s.trim();
        const EVAL: &str = "{eval}";
        if trimmed.starts_with(EVAL) {
            let src = &s[EVAL.len()..];
            let ctx_builder = env
                .options()
                .parse_options()
                .clone()
                .context_builder()
                .remove_filename()
                .set_context_name("MACRO parameter expansion");
            let ctx = ctx_builder.build(src);
            let src = Z80Span::new_extra(src, &ctx);
            let expr_token = crate::parser::located_expr.parse(src.0).map_err(|e| {
                let e: &Z80ParserError = e.inner();
                AssemblerError::SyntaxError { error: e.clone() }
            })?;
            let value = env
                .resolve_expr_must_never_fail(&expr_token)
                .map_err(|e| AssemblerError::AssemblingError { msg: e.to_string() })?;
            return Ok(value.to_string());
        }
        else {
            Ok(s.into_owned())
        }
    }
    else {
        let l = m.list_argument();
        Ok(format!(
            "{}",
            l.iter()
                .map(|p| expand_param(p.deref(), env))
                .collect::<Result<Vec<_>, AssemblerError>>()?
                .join(",")
        ))
    }
}

/// Encodes both the arguments and the macro
#[derive(Debug)]
pub struct MacroWithArgs<'m, 'a, P: MacroParamElement> {
    r#macro: &'m Macro,
    args: &'a [P]
}

impl<'m, 'a, P: MacroParamElement> MacroWithArgs<'m, 'a, P> {
    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#macro: &'m Macro, args: &'a [P]) -> Result<Self, AssemblerError> {
        if r#macro.nb_args() != args.len() {
            Err(AssemblerError::MacroError {
                name: r#macro.name().into(),
                root: Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "{} arguments provided, but {} expected.",
                        args.len(),
                        r#macro.nb_args()
                    )
                })
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

impl<'m, 'a, P: MacroParamElement> Expandable for MacroWithArgs<'m, 'a, P> {
    /// Develop the macro with the given arguments
    fn expand(&self, env: &Env) -> Result<String, AssemblerError> {
        //        assert_eq!(args.len(), self.nb_args());
        let mut listing = self.r#macro.code().to_string();

        // replace the arguments for the listing
        for (argname, argvalue) in self.r#macro.params().iter().zip(self.args.iter()) {
            let expanded = expand_param(argvalue, env)?;
            listing =
                if argname.starts_with("r#") & expanded.starts_with("\"") & expanded.ends_with("\"")
                {
                    // remove " " before doing the expansion
                    listing.replace(
                        &format!("{{{}}}", &argname[2..]),
                        &expanded[1..(expanded.len() - 1)]
                    )
                }
                else {
                    listing.replace(&format!("{{{}}}", argname), &expanded)
                }
        }

        Ok(listing)
    }
}

#[derive(Debug)]
pub struct StructWithArgs<'s, 'a, P: MacroParamElement> {
    r#struct: &'s Struct,
    args: &'a [P]
}

impl<'s, 'a, P: MacroParamElement> StructWithArgs<'s, 'a, P> {
    pub fn r#struct(&self) -> &Struct {
        self.r#struct
    }

    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#struct: &'s Struct, args: &'a [P]) -> Result<Self, AssemblerError> {
        if r#struct.nb_args() < args.len() {
            Err(AssemblerError::MacroError {
                name: r#struct.name().into(),
                root: Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "{} arguments provided, but at most {} expected.",
                        args.len(),
                        r#struct.nb_args()
                    )
                })
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

impl<'s, 'a, P: MacroParamElement> Expandable for StructWithArgs<'s, 'a, P> {
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
                    let ((name, token), provided_param) = {
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
                            let tok = if matches!(token, Token::Defb(_)) {
                                "DB"
                            }
                            else {
                                "DW"
                            };

                            let elem = match provided_param {
                                Some(provided_param) => {
                                    let elem = expand_param(provided_param, env)?;
                                    if elem.is_empty() {
                                        c[0].to_string()
                                    }
                                    else {
                                        elem
                                    }
                                }
                                None => {
                                    if c.len() == 0 {
                                        return Err(AssemblerError::AssemblingError {
                                            msg: format!("A value is expected for {} (no default value is provided)", name)
                                        })
                                    } else {
                                        c[0].to_string()
                                    }
                                }
                            };


                            Ok(format!(" {}{} {}", prefix, tok, elem))
                        }

                        Token::MacroCall(r#macro, current_default_args) => {
                            let mut call = format!(" {}{} ", prefix, r#macro);

                            let args = match provided_param {
                                Some(provided_param2) => {
                                    if provided_param2.is_single() {
                                        provided_param
                                            .into_iter()
                                            .zip_longest(current_default_args)
                                            .map(|a| {
                                                match a {
                                                    EitherOrBoth::Both(provided, _)
                                                    | EitherOrBoth::Left(provided) => {
                                                        (
                                                            provided.is_single(),
                                                            expand_param(provided, env)
                                                        )
                                                    }
                                                    EitherOrBoth::Right(default) => {
                                                        (
                                                            default.is_single(),
                                                            expand_param(default, env)
                                                        )
                                                    }
                                                }
                                            })
                                            .map(|(is_single, a)| {
                                                a.map(|repr| {
                                                    if is_single {
                                                        repr
                                                    }
                                                    else {
                                                        format!("[{}]", repr)
                                                    }
                                                })
                                            })
                                            .collect::<Result<Vec<String>, AssemblerError>>()?
                                    }
                                    else {
                                        provided_param2
                                            .list_argument()
                                            .into_iter()
                                            .zip_longest(current_default_args)
                                            .map(|a| {
                                                match a {
                                                    EitherOrBoth::Both(provided, _)
                                                    | EitherOrBoth::Left(provided) => {
                                                        (
                                                            provided.is_single(),
                                                            expand_param(provided.deref(), env)
                                                        )
                                                    }
                                                    EitherOrBoth::Right(default) => {
                                                        (
                                                            default.is_single(),
                                                            expand_param(default, env)
                                                        )
                                                    }
                                                }
                                            })
                                            .map(|(is_single, a)| {
                                                a.map(|repr| {
                                                    if is_single {
                                                        repr
                                                    }
                                                    else {
                                                        format!("[{}]", repr)
                                                    }
                                                })
                                            })
                                            .collect::<Result<Vec<String>, AssemblerError>>()?
                                    }
                                }

                                None => {
                                    current_default_args
                                        .iter()
                                        .map(|a| expand_param(a, env))
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
        Ok(developped)
    }
}

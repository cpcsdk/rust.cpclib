use std::ops::Deref;

use aho_corasick::{AhoCorasick, MatchKind};
use compact_str::CompactString;
use cpclib_common::itertools::{EitherOrBoth, Itertools};
use cpclib_common::winnow::Parser;
use cpclib_tokens::symbols::{Macro, Source, Struct};
use cpclib_tokens::{AssemblerFlavor, MacroParamElement, Token};

use crate::error::AssemblerError;
use crate::preamble::{Z80ParserError, Z80Span};
use crate::Env;

/// To be implemented for each element that can be expended based on some patterns (i.e. macros, structs)
pub trait Expandable {
    /// Returns a string version of the element after expansion
    fn expand(&self, env: &Env) -> Result<String, AssemblerError>;
}

#[inline]
fn expand_param<'p, P: MacroParamElement>(
    m: &'p P,
    env: &Env
) -> Result<beef::lean::Cow<'p, str>, AssemblerError> {
    let extended = if m.is_single() {
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
            beef::lean::Cow::owned(value.to_string())
        }
        else {
            s
        }
    }
    else {
        let l = m.list_argument();
        beef::lean::Cow::owned(
            l.iter()
                .map(|p| expand_param(p.deref(), env))
                .collect::<Result<Vec<_>, AssemblerError>>()?
                .join(",")
                .to_string()
        )
    };

    Ok(extended)
}

/// Encodes both the arguments and the macro
#[derive(Debug)]
pub struct MacroWithArgs<'m, 'a, P: MacroParamElement> {
    r#macro: &'m Macro,
    args: &'a [P]
}

impl<'m, 'a, P: MacroParamElement> MacroWithArgs<'m, 'a, P> {
    /// The construction fails if the number pf arguments is incorrect
    #[inline]
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
                }),
                location: None // TODO set up the location
            })
        }
        else {
            Ok(Self { r#macro, args })
        }
    }

    #[inline]
    pub fn source(&self) -> Option<&Source> {
        self.r#macro.source()
    }

    #[inline]
    pub fn flavor(&self) -> AssemblerFlavor {
        self.r#macro.flavor()
    }

    #[inline]
    fn expand_for_basm(&self, env: &Env) -> Result<String, AssemblerError> {
        //        assert_eq!(args.len(), self.nb_args());
        let listing = self.r#macro.code();
        let all_expanded = self.args.iter().map(|argvalue| expand_param(argvalue, env)); //.collect::<Result<Vec<_>, _ >>()?; // we ensure there is no more resizing
                                                                                         // build the needed datastructures for replacement
                                                                                         // let (patterns, replacements) =
        {
            // let capacity = all_expanded.len();
            // let mut patterns = Vec::with_capacity(capacity);
            // let mut replacement = Vec::with_capacity(capacity);

            let mut listing = beef::lean::Cow::borrowed(listing);

            for (argname, expanded) in self.r#macro.params().iter().zip(/* & */ all_expanded) {
                let expanded = expanded?;
                let (pattern, replacement) = if argname.starts_with("r#")
                    & expanded.starts_with("\"")
                    & expanded.ends_with("\"")
                {
                    let mut search = CompactString::with_capacity(argname.len() - 2 + 2);
                    search += "{";
                    search += &argname[2..];
                    search += "}";
                    // remove " " before doing the expansion
                    (search, &expanded[1..(expanded.len() - 1)])
                }
                else {
                    let mut search = CompactString::with_capacity(argname.len() + 2);
                    search += "{";
                    search += argname;
                    search += "}";
                    (search, &expanded[..])
                };

                listing = listing.replace(pattern.as_str(), replacement).into();
                // sadly this dumb way is faster than the ahocarasick one ...
            }
            Ok(listing.into_owned())

            //(patterns, replacement)
        }
    }

    #[inline]
    fn expand_for_orgams(&self, env: &Env) -> Result<String, AssemblerError> {
        let listing = self.r#macro.code();
        let all_expanded = self
            .args
            .iter()
            .map(|argvalue| expand_param(argvalue, env))
            .collect::<Result<Vec<_>, AssemblerError>>()?;

        let capacity: usize = self.args.len();
        let mut patterns = Vec::with_capacity(capacity);
        let mut replacements = Vec::with_capacity(capacity);

        for (argname, expanded) in self.r#macro.params().iter().zip(&all_expanded) {
            let (pattern, replacement) = if argname.starts_with("r#")
                & expanded.starts_with("\"")
                & expanded.ends_with("\"")
            {
                // remove " " before doing the expansion
                (&argname[2..], &expanded[1..(expanded.len() - 1)])
            }
            else {
                (&argname[..], &expanded[..])
            };

            patterns.push(pattern);
            replacements.push(replacement);
        }

        let ac = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .kind(None)
            .build(&patterns)
            .unwrap();
        let result = ac.replace_all(listing, &replacements);

        Ok(result)
    }
}

impl<P: MacroParamElement> Expandable for MacroWithArgs<'_, '_, P> {
    /// Develop the macro with the given arguments
    #[inline]
    fn expand(&self, env: &Env) -> Result<String, AssemblerError> {
        if self.flavor() == AssemblerFlavor::Basm {
            self.expand_for_basm(env)
        }
        else {
            assert_eq!(self.flavor(), AssemblerFlavor::Orgams);
            self.expand_for_orgams(env)
        }

        // make all replacements in one row :( sadly it is too slow :(
        // let ac = AhoCorasick::builder()
        // .match_kind(MatchKind::Standard)
        // .kind(None)
        // .build(&patterns)
        // .unwrap();
        // let result = ac.replace_all(listing, &replacements);
        //
        // Ok(result)
        //

        // replace the arguments for the listing
        // for (argname, argvalue) in self.r#macro.params().iter().zip(self.args.iter()) {
        // let expanded = expand_param(argvalue, env)?;
        // listing =
        // if argname.starts_with("r#") & expanded.starts_with("\"") & expanded.ends_with("\"")
        // {
        // remove " " before doing the expansion
        // listing.replace(
        // &format!("{{{}}}", &argname[2..]),
        // &expanded[1..(expanded.len() - 1)]
        // ).into()
        // }
        // else {
        // listing.replace(&format!("{{{}}}", argname), &expanded).into()
        // }
        // }
        //
        // Ok(listing)
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
                }),
                location: None // TODO setup the location
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

impl<P: MacroParamElement> Expandable for StructWithArgs<'_, '_, P> {
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
            .zip_longest(self.args.iter())
            .map(
                |current_information| -> Result<String, AssemblerError> {
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

                            let elem  = match provided_param {
                                Some(provided_param) => {
                                    let elem = expand_param(provided_param, env)?;
                                    if elem.is_empty() {
                                        beef::lean::Cow::owned(c[0].to_simplified_string())
                                    }
                                    else {
                                        elem
                                    }
                                }
                                None => {
                                    if c.is_empty() {
                                        return Err(AssemblerError::AssemblingError {
                                            msg: format!("A value is expected for {} (no default value is provided)", name)
                                        })
                                    } else {
                                        beef::lean::Cow::owned(c[0].to_string())
                                    }
                                }
                            };


                            Ok(format!(" {}{} {}", prefix, tok, elem))
                        }

                        Token::MacroCall(r#macro, current_default_args) => {
                            let mut call = format!(" {}{} ", prefix, r#macro);

                            let args: Vec<beef::lean::Cow<str>> = match provided_param {
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
                                                        beef::lean::Cow::owned(format!("[{}]", repr))
                                                    }
                                                })
                                            })
                                            .collect::<Result<Vec<_>, AssemblerError>>()?
                                    }
                                    else {
                                        provided_param2
                                            .list_argument()
                                            .iter()
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
                                                        beef::lean::Cow::owned(format!("[{}]", repr))
                                                    }
                                                })
                                            })
                                            .collect::<Result<Vec<_>, AssemblerError>>()?
                                    }
                                }

                                None => {
                                    current_default_args
                                        .iter()
                                        .map(|a| expand_param(a, env))
                                        .collect::<Result<Vec<_>, AssemblerError>>()?
                                }
                            };
                            call.push_str(&args.join(",")); // TODO push all strings instead of creating a new one and pushing it
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

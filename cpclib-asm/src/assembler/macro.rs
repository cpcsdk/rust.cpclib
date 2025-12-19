use std::ops::Deref;
use std::sync::Arc;

use aho_corasick::{AhoCorasick, MatchKind};
use cpclib_common::itertools::{EitherOrBoth, Itertools};
use cpclib_common::winnow::Parser;
use cpclib_tokens::symbols::{Macro, SourceLocation, Struct};
use cpclib_tokens::{AssemblerFlavor, MacroParamElement, Token};

use crate::Env;
use crate::error::AssemblerError;
use crate::preamble::{Z80ParserError, Z80Span};

/// To be implemented for each element that can be expended based on some patterns (i.e. macros, structs)
pub trait Expandable {
    /// Returns a string version of the element after expansion
    fn expand(&self, env: &mut Env) -> Result<String, AssemblerError>;
}

#[derive(Copy, Clone, Debug)]
enum MacroSegment {
    Lit { start: usize, end: usize },
    Arg { index: usize }
}

/// Strip raw string quotes if the parameter is a raw string literal.
/// Raw strings are marked with `r#` prefix and have literal quotes that need removal.
fn strip_raw_string_quotes<'a>(
    argname: &str,
    expanded: beef::lean::Cow<'a, str>
) -> beef::lean::Cow<'a, str> {
    if argname.starts_with("r#") && expanded.starts_with("\"") && expanded.ends_with("\"") {
        beef::lean::Cow::owned(expanded[1..expanded.len() - 1].to_string())
    }
    else {
        expanded
    }
}

fn tokenize_macro_body(r#macro: &Macro) -> Vec<MacroSegment> {
    let listing = r#macro.code();
    let mut segments = Vec::new();
    let mut cursor = 0;

    // Build param index lookup
    let param_names: Vec<&str> = r#macro
        .params()
        .iter()
        .map(|p| {
            let s = p.as_str();
            if s.starts_with("r#") { &s[2..] } else { s }
        })
        .collect();

    while let Some(rel_open) = listing[cursor..].find('{') {
        let open = cursor + rel_open;
        if open > cursor {
            segments.push(MacroSegment::Lit {
                start: cursor,
                end: open
            });
        }

        let after_open = open + 1;
        if let Some(rel_close) = listing[after_open..].find('}') {
            let close = after_open + rel_close;
            let key = &listing[after_open..close];

            if let Some(idx) = param_names.iter().position(|n| *n == key) {
                segments.push(MacroSegment::Arg { index: idx });
                cursor = close + 1;
                continue;
            }

            // Not a known placeholder: keep verbatim
            segments.push(MacroSegment::Lit {
                start: open,
                end: close + 1
            });
            cursor = close + 1;
        }
        else {
            // No closing brace: rest is literal
            segments.push(MacroSegment::Lit {
                start: open,
                end: listing.len()
            });
            cursor = listing.len();
        }
    }

    if cursor < listing.len() {
        segments.push(MacroSegment::Lit {
            start: cursor,
            end: listing.len()
        });
    }

    segments
}

#[inline]
fn expand_param<'p, P: MacroParamElement>(
    m: &'p P,
    env: &mut Env
) -> Result<beef::lean::Cow<'p, str>, AssemblerError> {
    let extended = if m.is_single() {
        let s = m.single_argument();
        let _trimmed = s.trim();
        if m.must_be_evaluated() {
            let src = &s[..];
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
pub struct MacroWithArgs<P: MacroParamElement> {
    r#macro: Macro,
    args: Vec<P>,
    segments: Arc<Vec<MacroSegment>>
}

impl<P: MacroParamElement> MacroWithArgs<P> {
    /// The construction fails if the number pf arguments is incorrect
    #[inline]
    pub fn build(r#macro: &Macro, args: &[P]) -> Result<Self, AssemblerError> {
        if r#macro.nb_args() != args.len() {
            Err(AssemblerError::MacroError {
                name: r#macro.name().into(),
                root: Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "{} arguments provided, but {} expected. [{}]",
                        args.len(),
                        r#macro.nb_args(),
                        r#macro.params().join(",")
                    )
                }),
                location: r#macro.source().cloned() // TODO set up the location
            })
        }
        else {
            Ok(Self {
                r#macro: r#macro.clone(),
                args: args.to_vec(),
                segments: Arc::new(tokenize_macro_body(r#macro))
            })
        }
    }

    #[inline]
    pub fn source(&self) -> Option<&SourceLocation> {
        self.r#macro.source()
    }

    #[inline]
    pub fn flavor(&self) -> AssemblerFlavor {
        self.r#macro.flavor()
    }

    #[inline]
    fn expand_for_basm(&self, env: &mut Env) -> Result<String, AssemblerError> {
        let listing = self.r#macro.code();
        let mut expanded_args: Vec<Option<beef::lean::Cow<'_, str>>> = vec![None; self.args.len()];

        // First pass: expand all arguments and calculate exact capacity.
        let capacity =
            self.segments
                .iter()
                .try_fold(0, |acc, segment| -> Result<usize, AssemblerError> {
                    match *segment {
                        MacroSegment::Lit { start, end } => Ok(acc + (end - start)),
                        MacroSegment::Arg { index } => {
                            let slot = expanded_args.get_mut(index).expect("Invalid segment index");

                            if slot.is_none() {
                                let argvalue =
                                    self.args.get(index).expect("Argument count mismatch");
                                let mut expanded = expand_param(argvalue, env)?;
                                let argname = self
                                    .r#macro
                                    .params()
                                    .get(index)
                                    .expect("Param count mismatch");
                                expanded = strip_raw_string_quotes(argname, expanded);
                                let arg_len = expanded.len();
                                *slot = Some(expanded);
                                Ok(acc + arg_len)
                            }
                            else {
                                Ok(acc + slot.as_ref().unwrap().len())
                            }
                        }
                    }
                })?;

        // Second pass: assemble output from pre-expanded arguments.
        let mut output = String::with_capacity(capacity);
        for segment in self.segments.iter() {
            match *segment {
                MacroSegment::Lit { start, end } => {
                    output.push_str(&listing[start..end]);
                },
                MacroSegment::Arg { index } => {
                    // All arguments were expanded in first pass, so this is always Some
                    let expanded_value = &expanded_args[index].as_ref().unwrap();
                    output.push_str(expanded_value.as_ref());
                }
            }
        }

        debug_assert_eq!(output.len(), capacity, "Capacity estimation mismatch");
        Ok(output)
    }

    #[inline]
    fn expand_for_orgams(&self, env: &mut Env) -> Result<String, AssemblerError> {
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
            let pattern = if argname.starts_with("r#") {
                &argname[2..]
            } else {
                argname.as_str()
            };
            let replacement = if argname.starts_with("r#")
                && expanded.starts_with("\"")
                && expanded.ends_with("\"")
            {
                &expanded[1..(expanded.len() - 1)]
            } else {
                &expanded[..]
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

impl<P: MacroParamElement> Expandable for MacroWithArgs<P> {
    /// Develop the macro with the given arguments
    #[inline]
    fn expand(&self, env: &mut Env) -> Result<String, AssemblerError> {
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
pub struct StructWithArgs<P: MacroParamElement> {
    r#struct: Struct,
    args: Vec<P>
}

impl<P: MacroParamElement> StructWithArgs<P> {
    pub fn r#struct(&self) -> &Struct {
        &self.r#struct
    }

    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#struct: &Struct, args: &[P]) -> Result<Self, AssemblerError> {
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
                location: r#struct.source().cloned() // TODO setup the location
            })
        }
        else {
            Ok(Self {
                r#struct: r#struct.clone(),
                args: args.to_vec()
            })
        }
    }

    pub fn source(&self) -> Option<&SourceLocation> {
        self.r#struct.source()
    }
}

impl<P: MacroParamElement> Expandable for StructWithArgs<P> {
    /// Generate the token that correspond to the current structure
    /// Current bersion does not handle at all directive with several arguments
    /// BUG does not work when directives have a prefix
    fn expand(&self, env: &mut Env) -> Result<String, AssemblerError> {
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
                                            msg: format!("A value is expected for {name} (no default value is provided)")
                                        })
                                    } else {
                                        beef::lean::Cow::owned(c[0].to_string())
                                    }
                                }
                            };


                            Ok(format!(" {prefix}{tok} {elem}"))
                        }

                        Token::MacroCall(r#macro, current_default_args) => {
                            let mut call = format!(" {prefix}{macro} ");

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
                                                        beef::lean::Cow::owned(format!("[{repr}]"))
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
                                                        beef::lean::Cow::owned(format!("[{repr}]"))
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

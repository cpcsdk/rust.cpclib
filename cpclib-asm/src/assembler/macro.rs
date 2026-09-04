use std::ops::Deref;

use aho_corasick::{AhoCorasick, MatchKind};
use cpclib_common::itertools::{EitherOrBoth, Itertools};
use cpclib_common::winnow::Parser;
use cpclib_tokens::symbols::{SourceLocation, Struct, ValueMacro};
use cpclib_tokens::{AssemblerFlavor, MacroParamElement, Token};

use crate::Env;
use crate::parser::context::ExpansionColumnMap;
use crate::error::AssemblerError;
use crate::preamble::{Z80ParserError, Z80Span};

/// Per-argument expansion of a macro call, resolved lazily - see
/// `MacroWithArgs::resolve_referenced_args`.
type ExpandedMacroArgs<'s> = Vec<Option<beef::lean::Cow<'s, str>>>;

/// To be implemented for each element that can be expended based on some patterns (i.e. macros, structs)
pub trait Expandable {
    /// Returns a string version of the element after expansion
    fn expand(&self, env: &mut Env) -> Result<String, Box<AssemblerError>>;

    /// The same expansion, with a way back to the columns of the text it was
    /// made from - see [`ExpansionColumnMap`].
    ///
    /// `None` where no such way exists: an expansion that rewrites its body
    /// rather than substituting into it has no column in the file to point at,
    /// and saying so is what makes the caller record no columns rather than the
    /// expansion's own.
    fn expand_with_columns(
        &self,
        env: &mut Env
    ) -> Result<(String, Option<ExpansionColumnMap>), Box<AssemblerError>> {
        Ok((self.expand(env)?, None))
    }
}

use cpclib_tokens::MacroSegment;

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

#[inline]
fn expand_param<'p, P: MacroParamElement>(
    m: &'p P,
    env: &mut Env
) -> Result<beef::lean::Cow<'p, str>, Box<AssemblerError>> {
    // Treat non-list params (including Empty) as single arguments
    let extended = if m.is_single() || !m.is_list() {
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
        let (oks, errs): (Vec<_>, Vec<_>) = l
            .iter()
            .map(|p| expand_param(p.deref(), env))
            .partition_map(|res| {
                match res {
                    Ok(val) => either::Either::Left(val),
                    Err(e) => either::Either::Right(e)
                }
            });
        if !errs.is_empty() {
            return Err(Box::new(AssemblerError::MultipleErrors { errors: errs }));
        }
        beef::lean::Cow::owned(oks.join(",").to_string())
    };

    Ok(extended)
}

/// Encodes both the arguments and the macro
#[derive(Debug)]
pub struct MacroWithArgs<'a, P: MacroParamElement> {
    r#macro: ValueMacro, // TODO check if we can use a reference here
    args: &'a [P]
}

impl<'a, P: MacroParamElement> MacroWithArgs<'a, P> {
    /// The construction fails if the number of arguments is incorrect - a
    /// variadic macro (`MACRO foo(a, b, ...)`) accepts `nb_args()` or more
    /// (the extras are indexed positionally in the body via `{2}`, `{3}`,
    /// ...); a non-variadic one still requires an exact match, unchanged.
    #[inline]
    pub fn build(r#macro: &ValueMacro, args: &'a [P]) -> Result<Self, Box<AssemblerError>> {
        let arity_ok = if r#macro.has_variadic() {
            args.len() >= r#macro.nb_args()
        }
        else {
            args.len() == r#macro.nb_args()
        };

        if !arity_ok {
            Err(Box::new(AssemblerError::MacroError {
                name: r#macro.name().into(),
                root: Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "{} arguments provided, but {}{} expected. [{}]",
                        args.len(),
                        if r#macro.has_variadic() {
                            "at least "
                        }
                        else {
                            ""
                        },
                        r#macro.nb_args(),
                        r#macro.params().join(",")
                    )
                }),
                location: r#macro.source().cloned() // TODO set up the location
            }))
        }
        else {
            Ok(Self {
                r#macro: r#macro.clone(), // TODO use reference?
                args
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
    fn expand_for_basm(
        &self,
        env: &mut Env
    ) -> Result<(String, ExpansionColumnMap), Box<AssemblerError>> {
        let (expanded_args, capacity) = self.resolve_referenced_args(env)?;
        self.finish_expand_for_basm(expanded_args, capacity)
    }

    /// First half of what `expand_for_basm` used to do in one pass: lazily
    /// resolve only the call arguments the macro body actually references
    /// (`{index}`/`{index:=default}` segments) and compute the exact output
    /// capacity. Split out so a cache lookup can be built from the resolved
    /// values *before* paying for the second half (splicing the body
    /// together) or a full re-parse - see `ProcessedToken::update_macro_or_struct_state`.
    #[inline]
    pub(crate) fn resolve_referenced_args<'s>(
        &'s self,
        env: &mut Env
    ) -> Result<(ExpandedMacroArgs<'s>, usize), Box<AssemblerError>> {
        let mut expanded_args: ExpandedMacroArgs<'_> = vec![None; self.args.len()];
        let arg_count = self.args.len().to_string();

        // First pass: expand all arguments and calculate exact capacity.
        let capacity = self.r#macro.segments().iter().try_fold(
            0,
            |acc, segment| -> Result<usize, Box<AssemblerError>> {
                match *segment {
                    MacroSegment::Lit { start, end } => Ok(acc + (end - start)),
                    MacroSegment::ArgCount => Ok(acc + arg_count.len()),
                    // `{N:=text}`: the call may legitimately not supply this
                    // argument, and then the default stands in for it. The
                    // default is body text, so its length is known without
                    // expanding anything.
                    MacroSegment::ArgOr { index, start, end } => {
                        if index < self.args.len() {
                            let slot = &mut expanded_args[index];
                            if slot.is_none() {
                                let mut expanded = expand_param(&self.args[index], env)?;
                                if let Some(argname) = self.r#macro.params().get(index) {
                                    expanded = strip_raw_string_quotes(argname, expanded);
                                }
                                let arg_len = expanded.len();
                                *slot = Some(expanded);
                                Ok(acc + arg_len)
                            }
                            else {
                                Ok(acc + slot.as_ref().unwrap().len())
                            }
                        }
                        else {
                            Ok(acc + (end - start))
                        }
                    },
                    MacroSegment::Arg { index } => {
                        // `index` comes from the macro's own body, tokenized
                        // once at declaration time - independent of any
                        // particular call's argument count. A variadic
                        // macro's body may reference `{N}` for an `N` this
                        // specific call doesn't actually supply (e.g. only
                        // one extra argument passed, but the body also uses
                        // `{3}`) - a real, per-call condition, not a bug, so
                        // it must be a clean error rather than a panic.
                        let Some(slot) = expanded_args.get_mut(index)
                        else {
                            return Err(self.arg_index_out_of_range_error(index));
                        };

                        if slot.is_none() {
                            let argvalue = &self.args[index];
                            let mut expanded = expand_param(argvalue, env)?;
                            // Extra (variadic) positional args have no
                            // declared name to check for the `r#`-raw-string
                            // convention - only named params are eligible.
                            if let Some(argname) = self.r#macro.params().get(index) {
                                expanded = strip_raw_string_quotes(argname, expanded);
                            }
                            let arg_len = expanded.len();
                            *slot = Some(expanded);
                            Ok(acc + arg_len)
                        }
                        else {
                            Ok(acc + slot.as_ref().unwrap().len())
                        }
                    }
                }
            }
        )?;

        Ok((expanded_args, capacity))
    }

    /// Second half of what `expand_for_basm` used to do in one pass: splice
    /// already-resolved arguments (from `resolve_referenced_args`) into the
    /// body's literal segments.
    ///
    /// The columns of the result are recorded as it is built, because this
    /// is the only place both texts are in hand at once: after this, the
    /// expansion is a source of its own and nothing in it says how far each
    /// substitution moved the text around it. See [`ExpansionColumnMap`].
    pub(crate) fn finish_expand_for_basm(
        &self,
        expanded_args: ExpandedMacroArgs<'_>,
        capacity: usize
    ) -> Result<(String, ExpansionColumnMap), Box<AssemblerError>> {
        let listing = self.r#macro.code();
        let arg_count = self.args.len().to_string();

        let mut output = String::with_capacity(capacity);
        let mut columns = ExpansionColumnMap::default();
        // Where the body has got to. A substitution's placeholder starts here,
        // and only the next literal says where it ended - which is all the map
        // needs, since it asks for the start of a piece and never its length.
        let mut source = 0usize;
        for segment in self.r#macro.segments().iter() {
            match *segment {
                MacroSegment::Lit { start, end } => {
                    columns.push_piece(output.len(), start, true);
                    output.push_str(&listing[start..end]);
                    source = end;
                },
                MacroSegment::ArgCount => {
                    columns.push_piece(output.len(), source, false);
                    output.push_str(&arg_count);
                },
                MacroSegment::ArgOr { index, start, end } => {
                    columns.push_piece(output.len(), source, false);
                    match expanded_args.get(index).and_then(|slot| slot.as_ref()) {
                        Some(value) => output.push_str(value),
                        // Emitted verbatim, never re-expanded: a default is
                        // written by whoever wrote the macro, in the macro's
                        // own body, so there is nothing caller-specific in it
                        // to substitute.
                        None => output.push_str(&listing[start..end])
                    }
                },
                MacroSegment::Arg { index } => {
                    columns.push_piece(output.len(), source, false);
                    // All in-range arguments were expanded in the first pass
                    // (guaranteed Some) - an out-of-range index already
                    // returned an error there, so this loop never reaches it.
                    output.push_str(expanded_args[index].as_ref().unwrap());
                }
            }
        }
        // A body ending on a substitution has no literal after it to say where
        // the placeholder stopped, so the end of both texts is recorded as a
        // piece of its own.
        columns.push_piece(output.len(), listing.len(), true);

        debug_assert_eq!(output.len(), capacity, "Capacity estimation mismatch");
        Ok((output, columns))
    }

    /// A macro body referenced `{index}` (0-based) but this particular call
    /// only provided `self.args.len()` argument(s) - real for a variadic
    /// macro, since how many total arguments a call passes is caller-
    /// dependent, unlike the fixed set of `{N}` indices the body itself may
    /// reference.
    #[inline]
    fn arg_index_out_of_range_error(&self, index: usize) -> Box<AssemblerError> {
        Box::new(AssemblerError::MacroError {
            name: self.r#macro.name().into(),
            root: Box::new(AssemblerError::AssemblingError {
                msg: format!(
                    "argument {{{index}}} is referenced in the body of macro `{}`, but only {} argument(s) were provided at this call",
                    self.r#macro.name(),
                    self.args.len()
                )
            }),
            location: self.r#macro.source().cloned()
        })
    }

    #[inline]
    fn expand_for_orgams(&self, env: &mut Env) -> Result<String, Box<AssemblerError>> {
        let oks = self.resolve_all_args_for_orgams(env)?;
        self.finish_expand_for_orgams(oks)
    }

    /// First half of what `expand_for_orgams` used to do in one call: eagerly
    /// resolve every call argument (orgams expansion has no per-segment
    /// laziness - see below) - split out so a cache lookup can be built from
    /// the resolved values before paying for the `AhoCorasick` replace or a
    /// full re-parse - see `ProcessedToken::update_macro_or_struct_state`.
    #[inline]
    pub(crate) fn resolve_all_args_for_orgams<'s>(
        &'s self,
        env: &mut Env
    ) -> Result<Vec<beef::lean::Cow<'s, str>>, Box<AssemblerError>> {
        // Orgams-flavor expansion substitutes named params only (a literal
        // pattern->replacement pass over `params()`, no segment/index model
        // at all - see `finish_expand_for_orgams`) - it has no way to
        // place a variadic macro's extra positional args anywhere, so
        // silently dropping them would be a real, confusing bug rather than
        // an unsupported-but-honest error.
        if self.r#macro.has_variadic() && self.args.len() > self.r#macro.nb_args() {
            return Err(Box::new(AssemblerError::MacroError {
                name: self.r#macro.name().into(),
                root: Box::new(AssemblerError::AssemblingError {
                    msg: "variadic macros (extra arguments beyond the named parameters) are not \
                          yet supported for the orgams assembler flavor"
                        .to_owned()
                }),
                location: self.r#macro.source().cloned()
            }));
        }

        let all_expanded = self
            .args
            .iter()
            .map(|argvalue| expand_param(argvalue, env))
            .partition_map(|res| {
                match res {
                    Ok(val) => either::Either::Left(val),
                    Err(e) => either::Either::Right(e)
                }
            });
        let (oks, errs): (Vec<_>, Vec<_>) = all_expanded;
        if !errs.is_empty() {
            return Err(Box::new(AssemblerError::MultipleErrors { errors: errs }));
        }

        Ok(oks)
    }

    /// Second half of what `expand_for_orgams` used to do in one call: turn
    /// already-resolved arguments (from `resolve_all_args_for_orgams`) into
    /// an `AhoCorasick` pattern/replacement pass over the macro body.
    pub(crate) fn finish_expand_for_orgams(
        &self,
        oks: Vec<beef::lean::Cow<'_, str>>
    ) -> Result<String, Box<AssemblerError>> {
        let listing = self.r#macro.code();
        let capacity: usize = self.args.len();
        let mut patterns = Vec::with_capacity(capacity);
        let mut replacements = Vec::with_capacity(capacity);

        for (argname, expanded) in self.r#macro.params().iter().zip(&oks) {
            let pattern = argname.strip_prefix("r#").unwrap_or(argname.as_str());
            let replacement = if argname.starts_with("r#")
                && expanded.starts_with('"')
                && expanded.ends_with('"')
            {
                expanded
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(expanded)
            }
            else {
                expanded
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

impl<'a, P: MacroParamElement> Expandable for MacroWithArgs<'a, P> {
    /// Develop the macro with the given arguments
    #[inline]
    fn expand(&self, env: &mut Env) -> Result<String, Box<AssemblerError>> {
        Ok(self.expand_with_columns(env)?.0)

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

    #[inline]
    fn expand_with_columns(
        &self,
        env: &mut Env
    ) -> Result<(String, Option<ExpansionColumnMap>), Box<AssemblerError>> {
        if self.flavor() == AssemblerFlavor::Basm {
            let (code, columns) = self.expand_for_basm(env)?;
            Ok((code, Some(columns)))
        }
        else {
            // Orgams substitutes bare parameter names anywhere in the body with
            // a whole-text search and replace, with no record of where the
            // matches were - so there is nothing to build a map from.
            assert_eq!(self.flavor(), AssemblerFlavor::Orgams);
            Ok((self.expand_for_orgams(env)?, None))
        }
    }
}

#[derive(Debug)]
pub struct StructWithArgs<'a, P: MacroParamElement> {
    r#struct: Struct,
    args: &'a [P]
}

impl<'a, P: MacroParamElement> StructWithArgs<'a, P> {
    pub fn r#struct(&self) -> &Struct {
        &self.r#struct
    }

    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#struct: &Struct, args: &'a [P]) -> Result<Self, Box<AssemblerError>> {
        if r#struct.nb_args() < args.len() {
            Err(Box::new(AssemblerError::MacroError {
                name: r#struct.name().into(),
                root: Box::new(AssemblerError::AssemblingError {
                    msg: format!(
                        "{} arguments provided, but at most {} expected.",
                        args.len(),
                        r#struct.nb_args()
                    )
                }),
                location: r#struct.source().cloned() // TODO setup the location
            }))
        }
        else {
            Ok(Self {
                r#struct: r#struct.clone(),
                args
            })
        }
    }

    pub fn source(&self) -> Option<&SourceLocation> {
        self.r#struct.source()
    }
}

impl<'a, P: MacroParamElement> Expandable for StructWithArgs<'a, P> {
    /// Generate the token that correspond to the current structure
    /// Current bersion does not handle at all directive with several arguments
    /// BUG does not work when directives have a prefix
    fn expand(&self, env: &mut Env) -> Result<String, Box<AssemblerError>> {
        //        dbg!("{:?} != {:?}", self.args, self.r#struct().content());

        let prefix = ""; // TODO acquire this prefix

        // self.args has priority over self.content information
        let mut developped: String = self
            .r#struct()
            .content()
            .iter()
            .zip_longest(self.args.iter())
            .map(
                |current_information| -> Result<String, Box<AssemblerError>> {
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
                                        return Err(Box::new(AssemblerError::AssemblingError {
                                            msg: format!("A value is expected for {name} (no default value is provided)")
                                        }))
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
                                        // For single provided argument, fall back to default when empty
                                        provided_param
                                            .into_iter()
                                            .zip_longest(current_default_args)
                                            .map(|pair| {
                                                match pair {
                                                    EitherOrBoth::Both(provided, default) => {
                                                        // Use default when provided is an empty single argument
                                                        if provided.is_empty() {
                                                            (
                                                                default.is_single(),
                                                                expand_param(default, env)
                                                            )
                                                        } else {
                                                            (
                                                                provided.is_single(),
                                                                expand_param(provided, env)
                                                            )
                                                        }
                                                    }
                                                    EitherOrBoth::Left(provided) => {
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
                                                    } else {
                                                        beef::lean::Cow::owned(format!("[{repr}]"))
                                                    }
                                                })
                                            })
                                            .collect::<Result<Vec<_>, Box<AssemblerError>>>()?
                                    } else {
                                        // For list provided arguments, apply per-element fallback to defaults when elements are empty
                                        provided_param2
                                            .list_argument()
                                            .iter()
                                            .zip_longest(current_default_args)
                                            .map(|pair| {
                                                match pair {
                                                    EitherOrBoth::Both(provided, default) => {
                                                        if provided.is_empty() {
                                                            (
                                                                default.is_single(),
                                                                expand_param(default, env)
                                                            )
                                                        } else {
                                                            (
                                                                provided.is_single(),
                                                                expand_param(provided.deref(), env)
                                                            )
                                                        }
                                                    }
                                                    EitherOrBoth::Left(provided) => {
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
                                                    } else {
                                                        beef::lean::Cow::owned(format!("[{repr}]"))
                                                    }
                                                })
                                            })
                                            .collect::<Result<Vec<_>, Box<AssemblerError>>>()?
                                    }
                                }

                                None => {
                                    current_default_args
                                        .iter()
                                        .map(|a| expand_param(a, env))
                                        .collect::<Result<Vec<_>, Box<AssemblerError>>>()?
                                }
                            };
                            call.push_str(&args.join(",")); // TODO push all strings instead of creating a new one and pushing it
                            Ok(call)
                        }
                        _ => unreachable!("{:?}", token)
                    }
                }
            )
            .collect::<Result<Vec<String>, Box<AssemblerError>>>()?
            .join("\n");

        let last = developped.pop().unwrap();
        developped.push(last);
        if last != 'n' {
            developped.push('\n');
        }
        Ok(developped)
    }
}

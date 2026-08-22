use std::borrow::Borrow;

use cpclib_common::itertools::Itertools;
use cpclib_common::smol_str::SmolStr;
use cpclib_tokens::{Expr, ExprFormat, ExprResult};
use substring::Substring;

use crate::{Env, error::{AssemblerError, ExpressionError}};

pub fn fix_string<S: Borrow<str>>(s: S) -> SmolStr {
    s.borrow().replace("\\n", "\n").into()
}

/// Create a new list
pub fn list_new(count: usize, value: ExprResult) -> ExprResult {
    ExprResult::List(vec![value; count])
}

/// Create a new string
pub fn string_new(count: usize, value: ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let value = value.char()?;
    let s = (0..count).map(|_| value).collect::<SmolStr>();
    Ok(ExprResult::String(fix_string(s)))
}

/// Modify a list or a string
pub fn list_set(
    mut list: ExprResult,
    index: usize,
    value: ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => {
            if index >= s.len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), index)
                )));
            }
            let c = value.int()? as u8 as char;
            let c = format!("{c}");
            let mut s = s.to_string();
            s.replace_range(index..index + 1, &c);
            Ok(ExprResult::String(fix_string(s)))
        },
        ExprResult::List(_) => {
            if index >= list.list_len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(list.list_len(), index)
                )));
            }
            list.list_set(index, value);
            Ok(list)
        },

        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}



pub fn list_position_value(env: &mut Env, list: &ExprResult, value: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::List(l) => {
            for (i, item) in l.iter().enumerate() {
                if item == value {
                    return Ok(ExprResult::Value(i as _));
                }
            }
            Ok(ExprResult::Value(-1))
        },

        ExprResult::String(s) => {
            let value = value.char()?;
            for (i, c) in s.chars().enumerate() {
                if c == value as u8 as char {
                    return Ok(ExprResult::Value(i as _));
                }       
            }
            Ok(ExprResult::Value(-1))
        },

        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list or a string")
                }))
            )))
        },
    }
}

pub fn list_position_predicate(env: &mut Env, list: &ExprResult, predicate: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let predicate = match predicate {
        ExprResult::String(f) => f,
        _ => {
            return Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{predicate} is not a function name stored in a string")
                }))
            )));
        }
    };

    match list {
        ExprResult::List(l) => {
            for (i, item) in l.iter().enumerate() {
                if item ==  &env.eval_any_function(predicate, &[item])? {
                    return Ok(ExprResult::Value(i as _));
                }
            }
            Ok(ExprResult::Value(-1))
        },

        ExprResult::String(s) => {
            for (i, c) in s.chars().enumerate() {
                if ExprResult::Char(c as _) ==  env.eval_any_function(predicate, &[&ExprResult::Char(c as _)])? {
                    return Ok(ExprResult::Value(i as _));
                }       
            }
            Ok(ExprResult::Value(-1))
        },

        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list or a string")
                }))
            )))
        },
    }
}

/// Get an item in a list of string
pub fn list_get(list: &ExprResult, index: usize) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => {
            if index >= s.len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), index)
                )));
            }
            Ok(ExprResult::Char(s.chars().nth(index).unwrap() as _))
        },
        ExprResult::List(_) => {
            if index >= list.list_len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(list.list_len(), index)
                )));
            }
            Ok(list.list_get(index).clone())
        },

        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}


pub fn list_split_by_value(
    list: &ExprResult,
    value: &ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::List(l) => {
            let mut result = Vec::new();
            let mut current = Vec::new();
            for item in l.iter() {
                if item == value {
                    result.push(ExprResult::List(current));
                    current = Vec::new();
                } else {
                    current.push(item.clone());
                }
            }
            if !current.is_empty() {
                result.push(ExprResult::List(current));
            }
            Ok(ExprResult::List(result))
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}


pub fn string_get(
    list: &ExprResult,
    index: usize
) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => {
            list_get(list, index)
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a string")
                }))
            )))
        },
    }
}

pub fn string_upper_case(list: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => {
            let s = s.to_uppercase();
            Ok(ExprResult::String(s.into()))
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a string")
                }))
            )))
        },
    }
}


pub fn string_len(list: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => list_len(list),
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a string")
                }))
            )))
        },
    }
}

/// Get a sublist  a list of string
pub fn list_sublist(
    list: &ExprResult,
    start: usize,
    end: usize // not included
) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => {
            if start >= s.len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), start)
                )));
            }
            if end > s.len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), end)
                )));
            }
            Ok(ExprResult::String(s.substring(start, end).into()))
        },
        ExprResult::List(l) => {
            if start >= l.len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(l.len(), start)
                )));
            }
            if end > l.len() {
                return Err(Box::new(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(l.len(), end)
                )));
            }
            Ok(ExprResult::List(l[start..end].to_vec()))
        },

        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}

pub fn list_len(list: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::List(l) => Ok(l.len().into()),
        ExprResult::String(s) => Ok(s.len().into()),
        ExprResult::Char(_) => Ok(1.into()),
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}

pub fn list_push(list: ExprResult, elem: ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::List(mut l) => {
            l.push(elem);
            Ok(ExprResult::List(l))
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}

pub fn list_extend(
    list1: ExprResult,
    list2: ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    match list1 {
        ExprResult::List(mut l) => {
            match list2 {
                ExprResult::List(mut l2) => {
                    l.append(&mut l2);
                    Ok(ExprResult::List(l))
                },
                _ => {
                    Err(Box::new(AssemblerError::ExpressionError(
                        ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                            msg: format!("{list2} is not a list")
                        }))
                    )))
                },
            }
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list1} is not a list")
                }))
            )))
        },
    }
}

pub fn list_sort(mut list: ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match &mut list {
        ExprResult::List(l) => {
            l.sort(); // inplace sort
            Ok(list)
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}

pub fn list_reverse(list: ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::List(mut l) => {
            l.reverse(); // inplace reverse
            Ok(ExprResult::List(l))
        },
        ExprResult::String(s) => {
            let s = s.chars().rev().collect::<SmolStr>();
            Ok(s.into())
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}

pub fn list_argsort(list: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::List(l) => {
            // https://stackoverflow.com/questions/69764050/how-to-get-the-indices-that-would-sort-a-vector-in-rust
            fn argsort<T: Ord>(data: &[T]) -> Vec<ExprResult> {
                let mut indices = (0..data.len()).map(ExprResult::from).collect::<Vec<_>>();
                indices.sort_by_key(|i| &data[i.int().unwrap() as usize]);
                indices
            }

            let l = argsort(l);
            Ok(ExprResult::List(l))
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}


pub fn list_filter(
    env: &mut Env,
    list: &ExprResult,
    predicate: &ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    let predicate = match predicate {
        ExprResult::String(f) => f,
        _ => {
            return Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{predicate} is not a function name stored in a string")
                }))
            )));
        }
    };

    match list {
        ExprResult::List(l)=> {
            let mut result = Vec::with_capacity(l.len());
            for item in l.into_iter() {
                let keep = env.eval_any_function(predicate, &[item])?;
                if keep.bool()? {
                    result.push(item.clone());
                }
            }
            Ok(ExprResult::List(result))
        },
        ExprResult::String(s) => {
            let mut result = String::with_capacity(s.len());
            for c in s.chars() {
                let keep = env.eval_any_function(predicate, &[ExprResult::Char(c as _ )])?;
                if keep.bool()? {
                    result.push(c);}
                }
                Ok(ExprResult::String(result.into()))
            },       
             _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list or a string")
                }))
            )))
        },
    }
}


pub fn string_filter(
    env: &mut Env,
    list: &ExprResult,
    predicate: &ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => list_filter(env, list, predicate),
        _ => {
            return Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a string")
                }))
            )));
        }
    }
}


pub fn list_map(
    env: &mut Env,
    list: &ExprResult,
    mapper: &ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    let mapper = match mapper {
        ExprResult::String(f) => f,
        _ => {
            return Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{mapper} is not a function name stored in a string")
                }))
            )));
        }
    };

    match list {
        ExprResult::List(l)=> {
            let mut result = Vec::with_capacity(l.len());
            for item in l.into_iter() {
                let mapped = env.eval_any_function(mapper, &[item])?;
                result.push(mapped);
            }
            Ok(ExprResult::List(result))
        },
        ExprResult::String(s) => {
            let mut result = String::with_capacity(s.len());
            for c in s.chars() {
                let mapped = env.eval_any_function(mapper, &[ExprResult::Char(c as _ )])?;
                let mapped_char = mapped.char()?;
                result.push(mapped_char as char);
            }
            Ok(ExprResult::String(result.into()))
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list or a string")
                }))
            )))
        },
    }
}


pub fn list_fold(
    env: &mut Env,
    list: &ExprResult,
    initial: &ExprResult,
    folder: &ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    let folder = match folder {
        ExprResult::String(f) => f,
        _ => {
            return Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{folder} is not a function name stored in a string")
                }))
            )));
        }
    };

    match list {
        ExprResult::List(l)=> {
            let mut acc = initial.clone();
            for item in l.into_iter() {
                acc = env.eval_any_function(folder, &[&acc, item.as_ref()])?;
            }
            Ok(acc)
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                }))
            )))
        },
    }
}


pub fn string_map(
    env: &mut Env,
    list: &ExprResult,
    mapper: &ExprResult
) -> Result<ExprResult, Box<AssemblerError>> {
    match list {
        ExprResult::String(s) => list_map(env, list, mapper),
        _ => {
            return Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a string")
                }))
            )));
        }
    }
}

/// BUG bytes must be enced in utf8
pub fn string_from_list(s1: ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match s1 {
        ExprResult::List(l1) => {
            use either::Either;
            let (oks, errs): (Vec<_>, Vec<_>) = l1
                .iter()
                .enumerate()
                .map(|(idx, v)| {
                    let v = v.int()?;
                    if !(0..=255).contains(&v) {
                        Err(Box::new(AssemblerError::AssemblingError {
                            msg: format!("{v} at {idx} is not a valid byte value")
                        }))
                    }
                    else {
                        Ok(v as u8)
                    }
                })
                .partition_map(|res| {
                    match res {
                        Ok(val) => Either::Left(val),
                        Err(e) => Either::Right(e)
                    }
                });
            if !errs.is_empty() {
                return Err(Box::new(AssemblerError::MultipleErrors { errors: errs }));
            }
            String::from_utf8(oks)
                .map_err(|e| {
                    Box::new(AssemblerError::AssemblingError {
                        msg: format!("Error when generating a string. {e}")
                    })
                })
                .map(|s| s.into())
        },
        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: "string_from_list must take a list as an argument".to_string()
                }))
            )))
        },
    }
}

/// The text a single `string_format` argument contributes when substituted
/// into a `{N}` placeholder - the raw string content for a string (no
/// surrounding quotes, unlike `ExprResult`'s own `Display`, which is meant
/// for diagnostic/debug output, not interpolation), the plain character for
/// a `Char` (not `Display`'s quoted `'c'` form), and each other variant's
/// natural textual form otherwise.
fn string_format_arg(val: &ExprResult) -> String {
    match val {
        ExprResult::String(s) => s.to_string(),
        ExprResult::Char(c) => (*c as char).to_string(),
        ExprResult::Value(v) => v.to_string(),
        ExprResult::Bool(b) => b.to_string(),
        ExprResult::Float(f) => f.into_inner().to_string(),
        // List/Matrix: no more meaningful a textual form than their own
        // Display already provides.
        other => format!("{other}")
    }
}

fn string_format_error(msg: String) -> Box<AssemblerError> {
    Box::new(AssemblerError::ExpressionError(ExpressionError::OwnError(
        Box::new(AssemblerError::AssemblingError { msg })
    )))
}

/// Parse a `{N:spec}` placeholder's `spec` part into the same `ExprFormat`
/// the `PRINT` statement's `{hex4}`-style interpolation already uses
/// (`cpclib-asm/src/parser/directives.rs`) - kept as a plain string match
/// here rather than reusing that winnow parser directly, since by this
/// point `spec` is already an isolated, resolved string slice, not raw
/// source text to re-parse.
fn parse_format_spec(spec: &str) -> Option<ExprFormat> {
    match spec {
        "hex" => Some(ExprFormat::Hex(None)),
        "hex2" => Some(ExprFormat::Hex(Some(2))),
        "hex4" => Some(ExprFormat::Hex(Some(4))),
        "hex8" => Some(ExprFormat::Hex(Some(8))),
        "bin" => Some(ExprFormat::Bin(None)),
        "bin8" => Some(ExprFormat::Bin(Some(8))),
        "bin16" => Some(ExprFormat::Bin(Some(16))),
        "bin32" => Some(ExprFormat::Bin(Some(32))),
        "int" => Some(ExprFormat::Int),
        _ => None
    }
}

/// `string_format(template, arg0, arg1, ...)` - Rust/Python-`str.format`-
/// style positional placeholders: `{0}`, `{1}`, ... refer to `arg0`,
/// `arg1`, ... (0-based), `{{`/`}}` are literal `{`/`}`. A placeholder
/// index beyond the number of arguments given, or malformed `{...}`
/// content, is a hard error rather than being silently left as-is or
/// swallowed - a typo'd index is far more likely than a deliberate literal
/// `{3}` in real code.
///
/// A placeholder may also carry a width/base format spec after a `:`, e.g.
/// `{0:hex4}`, reusing `ExprFormat`'s own `PRINT`-statement rendering
/// (`hex`/`hex2`/`hex4`/`hex8`/`bin`/`bin8`/`bin16`/`bin32`/`int`) - the
/// argument must then resolve to an integer, or it's a hard error the same
/// way an out-of-range index is.
pub fn string_format<E: AsRef<ExprResult>>(params: &[E]) -> Result<ExprResult, Box<AssemblerError>> {
    let template = match params[0].as_ref() {
        ExprResult::String(s) => s.to_string(),
        other => {
            return Err(string_format_error(format!(
                "string_format's first argument must be a string, got {other}"
            )));
        }
    };
    let args = &params[1..];

    let mut out = String::with_capacity(template.len());
    let mut i = 0usize;
    while i < template.len() {
        let rest = &template[i..];
        let c = rest.chars().next().expect("i < template.len()");

        if c == '{' {
            if rest.starts_with("{{") {
                out.push('{');
                i += 2;
                continue;
            }
            let after_brace = &rest[1..];
            let Some(close_rel) = after_brace.find('}')
            else {
                return Err(string_format_error(format!(
                    "string_format: unclosed '{{' in template {template:?}"
                )));
            };
            let inner = &after_brace[..close_rel];
            let (index_str, spec_str) = match inner.split_once(':') {
                Some((idx, spec)) => (idx, Some(spec)),
                None => (inner, None)
            };
            let Ok(index) = index_str.parse::<usize>()
            else {
                return Err(string_format_error(format!(
                    "string_format: invalid placeholder '{{{inner}}}' in template {template:?} - expected a plain 0-based index, optionally followed by ':spec'"
                )));
            };
            let Some(arg) = args.get(index)
            else {
                return Err(string_format_error(format!(
                    "string_format: placeholder {{{inner}}} has no matching argument ({} argument(s) given) in template {template:?}",
                    args.len()
                )));
            };
            let rendered = match spec_str {
                None => string_format_arg(arg.as_ref()),
                Some(spec) => {
                    let format = parse_format_spec(spec).ok_or_else(|| {
                        string_format_error(format!(
                            "string_format: unknown format spec '{spec}' in placeholder '{{{inner}}}' in template {template:?} - expected one of hex, hex2, hex4, hex8, bin, bin8, bin16, bin32, int"
                        ))
                    })?;
                    let value = arg.as_ref().int().map_err(|_| {
                        let arg = arg.as_ref();
                        string_format_error(format!(
                            "string_format: placeholder {{{inner}}} needs a numeric argument for format '{spec}', got {arg}"
                        ))
                    })?;
                    format.string_representation(value)
                }
            };
            out.push_str(&rendered);
            i += 1 + close_rel + 1;
            continue;
        }

        if c == '}' {
            if rest.starts_with("}}") {
                out.push('}');
                i += 2;
                continue;
            }
            return Err(string_format_error(format!(
                "string_format: unmatched '}}' in template {template:?}"
            )));
        }

        out.push(c);
        i += c.len_utf8();
    }

    Ok(ExprResult::String(fix_string(out)))
}

pub fn string_push(s1: ExprResult, s2: ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    match (&s1, &s2) {
        (ExprResult::Char(s1), ExprResult::Char(s2)) => {
            let s1 = format!("{}{}", *s1 as char, *s2 as char);
            Ok(ExprResult::String(s1.into()))
        },

        (ExprResult::Char(s1), ExprResult::String(s2)) => {
            let s1 = format!("{}{}", *s1 as char, fix_string(s2.clone()));
            Ok(ExprResult::String(s1.into()))
        },

        (ExprResult::String(s1), ExprResult::Char(s2)) => {
            let s1 = format!("{}{}", s1, *s2 as char);
            Ok(ExprResult::String(s1.into()))
        },

        (ExprResult::String(s1), ExprResult::String(s2)) => {
            let mut result = String::with_capacity(s1.len() + s2.len());
            result.push_str(s1);
            result.push_str(&fix_string(s2.clone()));
            Ok(ExprResult::String(result.into()))
        },
        (ExprResult::String(s1), ExprResult::List(l)) => {
            // Pre-estimate capacity
            let capacity = s1.len() + 2 + l.len() * 10; // rough estimate
            let mut result = String::with_capacity(capacity);
            result.push_str(s1);
            result.push('[');

            for (i, e) in l.iter().enumerate() {
                if i != 0 {
                    result.push(',');
                }
                // Directly append without intermediate String allocation
                match string_push(result.into(), e.clone())? {
                    ExprResult::String(s) => result = s.to_string(),
                    _ => unreachable!()
                }
            }

            result.push(']');
            Ok(ExprResult::String(result.into()))
        },

        (ExprResult::String(s1), ExprResult::Float(s2)) => {
            let s2_str = s2.into_inner().to_string();
            let mut result = String::with_capacity(s1.len() + s2_str.len());
            result.push_str(s1);
            result.push_str(&s2_str);
            Ok(ExprResult::String(result.into()))
        },

        (ExprResult::String(s1), ExprResult::Value(s2)) => {
            let s2_str = s2.to_string();
            let mut result = String::with_capacity(s1.len() + s2_str.len());
            result.push_str(s1);
            result.push_str(&s2_str);
            Ok(ExprResult::String(result.into()))
        },

        (ExprResult::String(s1), ExprResult::Bool(s2)) => {
            let s2_str = s2.to_string();
            let mut result = String::with_capacity(s1.len() + s2_str.len());
            result.push_str(s1);
            result.push_str(&s2_str);
            Ok(ExprResult::String(result.into()))
        },

        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: format!("string_push called with wrong types {s1:?} {s2:?}")
                }))
            )))
        },
    }
}

#[cfg(test)]
mod string_format_tests {
    use super::*;

    fn s(text: &str) -> ExprResult {
        ExprResult::String(text.into())
    }

    #[test]
    fn substitutes_positional_placeholders_in_order() {
        let result = string_format(&[
            s("Score: {0}/{1}"),
            ExprResult::Value(10),
            ExprResult::Value(100)
        ])
        .unwrap();
        assert_eq!(result, s("Score: 10/100"));
    }

    #[test]
    fn a_string_argument_is_substituted_without_quotes() {
        // Distinguishes this from `ExprResult`'s own `Display`, which wraps
        // strings in quotes for diagnostic output - interpolation must use
        // the raw content.
        let result = string_format(&[s("Hello, {0}!"), s("world")]).unwrap();
        assert_eq!(result, s("Hello, world!"));
    }

    #[test]
    fn a_placeholder_can_be_reused_several_times() {
        let result = string_format(&[s("{0}-{0}-{0}"), ExprResult::Value(7)]).unwrap();
        assert_eq!(result, s("7-7-7"));
    }

    #[test]
    fn double_braces_are_literal_braces() {
        let result = string_format(&[s("{{literal}} {0}"), ExprResult::Value(1)]).unwrap();
        assert_eq!(result, s("{literal} 1"));
    }

    #[test]
    fn a_template_with_no_placeholders_is_returned_unchanged() {
        let result = string_format(&[s("no placeholders here")]).unwrap();
        assert_eq!(result, s("no placeholders here"));
    }

    #[test]
    fn out_of_range_index_is_a_clear_error() {
        let result = string_format(&[s("{1}"), ExprResult::Value(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn a_non_numeric_placeholder_is_a_clear_error() {
        let result = string_format(&[s("{abc}"), ExprResult::Value(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn an_unclosed_brace_is_a_clear_error() {
        let result = string_format(&[s("{0")]);
        assert!(result.is_err());
    }

    #[test]
    fn a_non_string_template_is_a_clear_error() {
        let result = string_format(&[ExprResult::Value(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn a_format_spec_renders_hex_with_the_requested_width() {
        let result = string_format(&[s("{0:hex4}"), ExprResult::Value(0xAB)]).unwrap();
        assert_eq!(result, s("0x00ab"));
    }

    #[test]
    fn a_format_spec_renders_unpadded_hex_and_bin() {
        let result = string_format(&[s("{0:hex} {0:bin}"), ExprResult::Value(5)]).unwrap();
        assert_eq!(result, s("0x5 0b101"));
    }

    #[test]
    fn a_format_spec_renders_padded_bin() {
        let result = string_format(&[s("{0:bin8}"), ExprResult::Value(5)]).unwrap();
        assert_eq!(result, s("0b00000101"));
    }

    #[test]
    fn a_format_spec_can_be_reused_with_different_specs_on_the_same_argument() {
        let result = string_format(&[s("{0:int} = {0:hex2}"), ExprResult::Value(10)]).unwrap();
        assert_eq!(result, s("10 = 0x0a"));
    }

    #[test]
    fn an_unknown_format_spec_is_a_clear_error() {
        let result = string_format(&[s("{0:nope}"), ExprResult::Value(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn a_format_spec_on_a_non_numeric_argument_is_a_clear_error() {
        let result = string_format(&[s("{0:hex4}"), s("not a number")]);
        assert!(result.is_err());
    }
}

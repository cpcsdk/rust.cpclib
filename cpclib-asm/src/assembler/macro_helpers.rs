use cpclib_common::itertools::EitherOrBoth;

use crate::r#macro::expand_param;

use super::*;

#[allow(clippy::too_many_arguments)]
pub fn expand_macro_call<P: MacroParamElement>(
    prefix: &str,
    macro_name: &str,
    current_default_args: &[P],
    provided_param: Option<&P>,
    env: &mut Env
) -> Result<String, Box<AssemblerError>> {
    let mut call = format!(" {prefix}{macro_name} ");

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
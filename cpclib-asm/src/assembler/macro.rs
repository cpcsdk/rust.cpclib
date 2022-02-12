use cpclib_tokens::MacroParam;
use cpclib_tokens::symbols::Macro;
use cpclib_tokens::symbols::Struct;
use cpclib_tokens::symbols::Source;
use cpclib_tokens::Token;
use crate::error::AssemblerError;
use crate::Env;
use crate::preamble::Z80Span;
use cpclib_common::itertools::Itertools;

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
                const EVAL:&str = "{eval}";
                if trimmed.starts_with(EVAL) {
                    let src = &s[EVAL.len()..];
                    let src = Z80Span::from(src);
                    let expr_token = crate::parser::expr(src).map_err(|e| AssemblerError::AssemblingError{msg: e.to_string()})?.1;
                    let value = env.resolve_expr_must_never_fail(&expr_token)?;
                    return Ok(value.to_string());
                } else {
                    Ok(s.clone())
                }
            },
            Self::List(l) => {
                Ok(
                    format!("{}", 
                    l.iter().map(|p| p.expand(env))
                    .collect::<Result<Vec<_>, AssemblerError>>()?
                    .join(",")))
            }
        }
    }
}


/// Encodes both the arguments and the macro
pub struct MacroWithArgs<'m, 'a> {
    r#macro: &'m Macro,
    args: &'a[MacroParam]
}

impl<'m, 'a> MacroWithArgs<'m, 'a> {
    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#macro: &'m Macro, args: &'a [MacroParam]) -> Result<Self, AssemblerError> {
        if r#macro.nb_args() != args.len() {
            Err(AssemblerError::MacroError{
                name: r#macro.name().into(),
                root: box AssemblerError::AssemblingError{
                    msg: format!("{} arguments provided, but {} expected.", args.len(), r#macro.nb_args())
                }
            })
        } else {
            Ok(Self {
                r#macro,
                args
            })
        }
    }

    
    pub fn source(&self) -> Option<&Source>  {
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
            listing = listing.replace(
                &format!("{{{}}}", argname), 
                &argvalue.expand(env)?
            );
        }

        Ok(listing)
    }
}


pub struct StructWithArgs<'s, 'a> {
    r#struct: &'s Struct,
    args: &'a[MacroParam]
}

impl<'s,'a> StructWithArgs<'s,'a> {

    pub fn r#struct(&self) -> &Struct {
        self.r#struct
    }

    /// The construction fails if the number pf arguments is incorrect
    pub fn build(r#struct: &'s Struct, args: &'a [MacroParam]) -> Result<Self, AssemblerError> {
        if r#struct.nb_args() < args.len() {
            Err(AssemblerError::MacroError{
                name: r#struct.name().into(),
                root: box AssemblerError::AssemblingError{
                    msg: format!("{} arguments provided, but at most {} expected.", args.len(), r#struct.nb_args())
                }
            })
        } else {
            Ok(Self {
                r#struct,
                args
            })
        }
    }

    pub fn source(&self) -> Option<&Source>  {
        self.r#struct.source()
    }

}


impl<'s, 'a> Expandable for StructWithArgs<'s, 'a> {
        /// Generate the token that correspond to the current structure
    /// Current bersion does not handle at all directive with several arguments
    fn expand(&self, env: &Env) -> Result<String, AssemblerError> {
        assert_eq!(self.args.len(), self.r#struct().content().len());

        let mut developped: String = self
            .r#struct()
            .content()
            .iter()
            .zip(self.args.iter())
            .enumerate()
            .map(|(_idx, ((_name, token), current_param))| -> Result<String, AssemblerError> {
                match token {
                    Token::Defb(c) | Token::Defw(c) => {
                        assert_eq!(c.len(), 1);

                        let tok = if matches!(token, Token::Defb(_)) {
                            "db"
                        }
                        else {
                            "dw"
                        };

                        if current_param.is_empty() {
                            Ok(format!(" {} {}", tok, c[0].to_string()))
                        }
                        else {
                            let elem = current_param.expand(env)?;
                            Ok(format!(" {} {}", tok, elem))
                        }
                    }

                    Token::MacroCall(n, current_default_arg) => {
                        let mut call = format!(" {} ", n);

                        // The way to manage default/provided params differ depending on the combination
                        let args = match (current_param, current_default_arg.len()) {
                            // no default
                            (_, 0) => {
                                let elem = current_param.expand(env)?;
                                vec![elem]
                            }

                            // one default
                            (_, 1) => {
                                let val = if current_param.is_empty() {
                                    &current_default_arg[0]
                                }
                                else {
                                    current_param
                                };
                                let elem = val.expand(env)?;
                                vec![elem]
                            }

                            // default is several, provided is single. Use provided only if not empty
                            (MacroParam::Single(_), _nb_default) => {
                                let mut default_iter = current_default_arg.iter();
                                let first_default = default_iter.next().unwrap();
                                let mut collected = Vec::new();
                                collected.push(if current_param.is_empty() {
                                    first_default
                                }
                                else {
                                    current_param
                                });
                                collected.extend(default_iter);

                                collected.iter().map(|p| p.expand(env))
                                    .collect::<Result<Vec<_>,_>>()?
                            }

                            // default and provided are several
                            (MacroParam::List(all_curr), nb_default) => {
                                let max_size = all_curr.len().max(nb_default);

                                let mut collected = Vec::new();
                                for idx2 in 0..max_size {
                                    if idx2 >= all_curr.len() {
                                        collected.push(current_default_arg[idx2].expand(env)?);
                                    }
                                    else if idx2 >= nb_default {
                                        collected.push(all_curr[idx2].expand(env)?);
                                    }
                                    else {
                                        let current = &all_curr[idx2];
                                        let default = &current_default_arg[idx2];

                                        if current.is_empty() {
                                            collected.push(default.expand(env)?);
                                        }
                                        else {
                                            collected.push(current.expand(env)?);
                                        }
                                    }
                                }
                                collected
                            }
                        };

                        call.push_str(&args.join(","));
                        Ok(call)
                    }
                    _ => unreachable!("{:?}", token)
                }
            })
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
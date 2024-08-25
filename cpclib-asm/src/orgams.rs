use std::{fmt::Display, ops::Deref};

use beef::lean::Cow;
use cpclib_common::{camino::Utf8Path, itertools::Itertools};
use cpclib_tokens::{ListingElement, MacroParamElement, Token};

use crate::{r#macro, parse_z80, MayHaveSpan, SourceString, TokenExt};

#[derive(Debug)]
pub struct ToOrgamsError(String);

impl Display for ToOrgamsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl Into<ToOrgamsError> for &str {
    fn into(self) -> ToOrgamsError {
        ToOrgamsError(self.into())
    }
}
impl Into<ToOrgamsError> for &std::io::Error {
    fn into(self) -> ToOrgamsError {
        let content = self.to_string();
        content.as_str().into()
    }
}

/// Convert an item to the orgams format
pub trait ToOrgams {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError>;
}

/* 
impl ToOrgams for Token {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
        todo!()
    }
}
    */

impl<T:TokenExt + MayHaveSpan + ListingElement + Display> ToOrgams for T {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
        // we assume it is already a BASM format and not an ORGAMS format
        let handle_macro_definition = |token: &T| -> Cow<str> {
            let macro_name = token.macro_definition_name();
            let arguments_name = token.macro_definition_arguments();
            let mut macro_content = token.macro_definition_code().to_owned();

            for arg in arguments_name.iter() {
                macro_content = macro_content.replace(&format!("{{{arg}}}"), arg);
            }
            macro_content = macro_content.replace('\n',"\n\t");

            // also transform the content of the macro
            // in case of failure, fallback to the original content
            let macro_content = if let Ok(macro_content_listing) = parse_z80(&macro_content) {
                let macro_content_listing = &macro_content_listing[..];
                macro_content_listing.to_orgams_string()
                    .map(|s| s.to_string())
                    .unwrap_or(macro_content)
            } else {
                macro_content
            };

            let macro_args = arguments_name.into_iter().join(", ");
            
            let output = format!("\tMACRO {macro_name} {macro_args}\n{macro_content}\tENDM");
            Cow::owned(output)
        };

        let handle_macro_call = |token: &T| -> Cow<str> {
            let name = self.macro_call_name();
            let arguments = self.macro_call_arguments()
                .into_iter()
                .map(|s| if s.is_single() {
                    s.single_argument()
                }else {
                    unimplemented!("We consider it does not happens with ORGAMS")
                })
                .join(",");

            let repr = format!("{name}({arguments})");
            repr.into()
        };

        let handle_standard_instruction = |token: &T| -> Cow<str> {
            /*
            if self.has_span() {
                Cow::borrowed(self.span().as_str())
            } else {
                Cow::owned(self.to_string())
            }
            */
            token.to_token().to_string().into()
        };

        let handle_print = |token :&T| -> Cow<str> {
            let s = Token::Comment(format!("; {}", token.to_string()))
                .to_orgams_string()
                .unwrap()
                .to_string();
            s.into()
        };

        // This is the default behavior that changes nothing
        let repr = if self.is_macro_definition() {
            handle_macro_definition(self)
        } else if self.is_print() {
            handle_print(self)
        } else if self.is_call_macro_or_build_struct() {
            handle_macro_call(self)
        }else {
            handle_standard_instruction(self)
        };

        if repr.is_empty() {
            return Ok(repr);
        }

        // ensure the is space first
        let repr = if !self.is_comment() && 
                    !self.is_label() &&
                    !self.is_equ() &&
                    !self.is_assign() {
            let first = repr.chars().next().unwrap();
            if first != ' ' && first != '\t' {
                Cow::owned(format!("\t{}", repr))
            } else {
                repr
            }
        } else {
            repr
        };
        Ok(repr)
    }
}

/*
impl ToOrgams for Listing {
    fn to_orgams_string(&self) -> Result<String, ToOrgamsError> {
        todo!()
    }
}

impl ToOrgams for LocatedListing {
    fn to_orgams_string(&self) -> Result<String, ToOrgamsError> {
        let mut content = String::new();

        for token in self.iter() {

        }

        Ok(content)
    }
}
*/

impl<T: ToOrgams> ToOrgams for &[T] {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
        let mut content = String::with_capacity(self.len() * 10);

        for token in self.iter() {
            content.push_str(token.to_orgams_string()?.deref());
            content.push('\n');
        }

        // TODO do it properly by coding the complete expression display
        let content = content.replace("0x", "&");

        // TODO handle that in macro call
        let content = content.replace("(void)", "()");

        // TODO handle that directly in instruction print out
        let content = content.replace(", ", ",");

        Ok(content.into())
    }
}

///
/// COnvert a basm txt source file as a orgams text source file.
/// There are tons of current limitations. I have only implemented what I need
/// TODO - convert expressions to be orgams compatible. REwrite them ? Write parenthesis ?
/// TODO - rewrite macros
pub fn convert_source<P1: AsRef<Utf8Path>, P2: AsRef<Utf8Path>>(src: P1, tgt: P2) -> Result<(), ToOrgamsError> {
    let src = src.as_ref();
    let tgt = tgt.as_ref();
    let code = std::fs::read_to_string(src)
        .map_err(|e| ToOrgamsError(format!("Error while reading {}. {}", src, e.to_string())))?;
    let lst = parse_z80(code)
        .map_err(|e| ToOrgamsError(format!("Error while parsing {}. {}", src, e.to_string())))?;
    let lst = lst.as_slice();
    let orgams = lst.to_orgams_string()?;
    std::fs::write(tgt, orgams.as_bytes())
        .map_err(|e| format!("Error while saving {}", tgt).as_str().into())
}
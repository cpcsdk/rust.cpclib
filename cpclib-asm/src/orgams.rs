use std::fmt::Display;
use std::ops::Deref;

use beef::lean::Cow;
use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use cpclib_tokens::{BinaryOperation, DataAccess, DataAccessElem, Expr, ExprElement, ListingElement, MacroParamElement, Mnemonic, TestKind, TestKindElement, Token};

use crate::{parse_z80, LocatedDataAccess, LocatedExpr, LocatedTestKind, MayHaveSpan, SourceString, TokenExt};

#[derive(Debug)]
pub struct ToOrgamsError(String);

impl Display for ToOrgamsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl Into<ToOrgamsError> for String {
    fn into(self) -> ToOrgamsError {
        ToOrgamsError(self)
    }
}
impl Into<ToOrgamsError> for &std::io::Error {
    fn into(self) -> ToOrgamsError {
        let content = self.to_string();
        content.into()
    }
}

/// Convert an item to the orgams format
pub trait ToOrgams {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError>;
}


impl ToOrgams for Mnemonic {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
        Ok(self.to_string().to_lowercase().into())
    }
}

impl ToOrgams for BinaryOperation {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
        Ok(self.to_string().to_uppercase().into())
    }
}

macro_rules! expr_to_orgams {
    () => {
        fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
            let repr = match self {

                Self::Value(v,..) => {
                    if self.has_span() {
                        // basm allow _ between numbers
                        let span = dbg!(self.span().as_str().replace("_", ""));
                        if span.starts_with("0x") || span.starts_with("0X") {
                            format!("&{}", &span[2..])
                        } else if span.starts_with("#") {
                            format!("&{}", &span[1..])
                        } else {
                            format!("{}", v)
                        }
                    } else {
                        format!("&{:x}", v)
                    }
                },

                Self::Label(l) => {
                    format!("{}", l)
                }

                Self::BinaryOperation(op, left, right, ..) => {
                    let rleft = left.to_orgams_string()?;
                    let rright = right.to_orgams_string()?;
                    let rleft = rleft.as_ref();
                    let op = op.to_orgams_string()?;

                    let protect =  |expr: &Self, repr: &str| -> String {
                        if expr.is_label() || expr.is_value() {
                            repr.into()
                        } else {
                            format!("[{}]", repr).into()
                        }
                    };

                    let rleft = protect(left, rleft.as_ref());
                    let rright = protect(right, rright.as_ref());

                    format!("{}{}{}", rleft, op, rright)
                }

                _ => unimplemented!("{:?}", self)
            };

            Ok(repr.into())
        }
    }
}

impl ToOrgams for LocatedExpr {
    expr_to_orgams!();
}

impl ToOrgams for Expr {
    expr_to_orgams!();
}

macro_rules! test_kind_to_orgams {
    () => {
        fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
            if self.is_true_test() {
                let expr = self.expr_unchecked();
                Ok(format!("IF {}", expr.to_orgams_string()?).into())
            } else {
                Err(format!("{:?}", self).into())
            }
        }
    };
}

impl ToOrgams for LocatedTestKind  {
    test_kind_to_orgams!();
}

impl ToOrgams for TestKind  {
    test_kind_to_orgams!();
}


macro_rules! data_access_to_orgams {
    () => {
        fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {

            let repr = if self.is_expression() {
                let exp = self.get_expression().unwrap();
                return exp.to_orgams_string();
            }
            else {
                self.to_string().to_lowercase()
            };

            Ok(repr.into())
        }
        
    };
}



impl ToOrgams for DataAccess {
    data_access_to_orgams!();
}

impl ToOrgams for LocatedDataAccess {
    data_access_to_orgams!();
}

impl<T> ToOrgams for T where 
T: TokenExt + MayHaveSpan + ListingElement + ToString + ?Sized,
T::DataAccess: ToOrgams,
T::Expr: ToOrgams,
T::TestKind: ToOrgams
{
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
        // we assume it is already a BASM format and not an ORGAMS format
        let handle_macro_definition = |token: &T| -> Cow<str> {
            let macro_name = token.macro_definition_name();
            let arguments_name = token.macro_definition_arguments();
            let mut macro_content = token.macro_definition_code().to_owned();


            for arg in arguments_name.iter() {
                macro_content = macro_content.replace(&format!("{{{arg}}}"), arg);
            }
            macro_content = macro_content.replace('\n', "\n\t");

            // also transform the content of the macro
            // in case of failure, fallback to the original content
            let macro_content = if let Ok(macro_content_listing) = parse_z80(&macro_content) {
                let macro_content_listing = macro_content_listing.as_slice();
                macro_content_listing
                    .to_orgams_string()
                    .map(|s| s.to_string())
                    .unwrap_or(macro_content)
            }
            else {
                macro_content
            };

            let macro_args = arguments_name.into_iter().join(", ");

            let output = format!("\tMACRO {macro_name} {macro_args}\n{macro_content}\tENDM");
            Cow::owned(output)
        };

        let handle_macro_call = |token: &T| -> Cow<str> {
            let name = token.macro_call_name();
            let arguments = token
                .macro_call_arguments()
                .into_iter()
                .map(|s| {
                    if s.is_single() {
                        s.single_argument()
                    }
                    else {
                        unimplemented!("We consider it does not happens with ORGAMS")
                    }
                })
                .join(",");

            let repr = format!("{name}({arguments})");
            repr.into()
        };

        let handle_standard_directive = |token: &T| -> Cow<str> {
            // if self.has_span() {
            // Cow::borrowed(self.span().as_str())
            // } else {
            // Cow::owned(self.to_string())
            // }
            token.to_token().to_string().into()
        };

        let handle_print = |token: &T| -> Cow<str> {
            let s = Token::Comment(format!("; {}", token.to_string()))
                .to_orgams_string()
                .unwrap()
                .to_string();
            s.into()
        };

        // XXX strong limitation, does not yet handle 3 args
        let handle_opcode = |token: &T| -> String {
            let mut op = token.mnemonic().unwrap().to_orgams_string().unwrap().to_string();

            if let Some(arg) = token.mnemonic_arg1() {
                op.push(' ');
                op.push_str(&arg.to_orgams_string().unwrap())
            }

            if let Some(arg) = token.mnemonic_arg2() {
                if token.mnemonic_arg1().is_some() {
                    op.push(',');
                } else {
                    op.push(' ');
                }
                op.push_str(&arg.to_orgams_string().unwrap())
            }

            op

        };

        let handle_assign = |token: &T| -> String {
            let label = token.assign_symbol();
            let value = token.assign_value();

            format!("{}={}", label, value.to_orgams_string().unwrap())
        };

        let handle_equ = |token: &T| -> String {
            let label = token.equ_symbol();
            let value = token.equ_value();

            format!("{} EQU {}", label, value.to_orgams_string().unwrap())
        };

        let handle_if = |token: &T| -> String {
            assert!(self.if_nb_tests() == 1);
            
            let (test, code) = token.if_test(0);
            let mut content = format!("{}\n{}", test.to_orgams_string().unwrap(), code.to_orgams_string().unwrap());

            if let Some(code) = token.if_else() {
                content.push_str("\n\tELSE\n");
                content.push_str(&code.to_orgams_string().unwrap());
            }

            content.push_str("\n\tENDIF\n");
            content
        };

        // This is the default behavior that changes nothing
        let repr = if self.is_opcode() {
            Cow::owned(handle_opcode(self))
        } else if self.is_macro_definition() {
            handle_macro_definition(self)
        }
        else if self.is_print() {
            handle_print(self)
        }
        else if self.is_call_macro_or_build_struct() {
            handle_macro_call(self)
        }
        else if self.is_assign() {
            handle_assign(self).into()
        }
        else if self.is_equ(){ 
            handle_equ(self).into()
        }
        else if self.is_if() {
            handle_if(self).into()
        }
        else {
            handle_standard_directive(self)
        };

        if repr.is_empty() {
            return Ok(repr);
        }

        // ensure the is space first
        let repr = if !self.is_comment() && !self.is_label() && !self.is_equ() && !self.is_assign()
        {
            let first = repr.chars().next().unwrap();
            if first != ' ' && first != '\t' {
                Cow::owned(format!("\t{}", repr))
            }
            else {
                repr
            }
        }
        else {
            repr
        };
        Ok(repr)
    }
}

// impl ToOrgams for Listing {
// fn to_orgams_string(&self) -> Result<String, ToOrgamsError> {
// todo!()
// }
// }
//
// impl ToOrgams for LocatedListing {
// fn to_orgams_string(&self) -> Result<String, ToOrgamsError> {
// let mut content = String::new();
//
// for token in self.iter() {
//
// }
//
// Ok(content)
// }
// }

impl<T: ToOrgams> ToOrgams for &[T] {
    fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
        let mut content = String::with_capacity(self.len() * 10);

        for token in self.iter() {
            content.push_str(token.to_orgams_string()?.deref());
            content.push('\n');
        }

        // TODO do it properly by coding the complete expression display
//        let content = content.replace("0x", "&");

        Ok(content.into())
    }
}

/// COnvert a basm txt source file as a orgams text source file.
/// There are tons of current limitations. I have only implemented what I need
/// TODO - convert expressions to be orgams compatible. REwrite them ? Write parenthesis ?
/// TODO - rewrite macros
pub fn convert_source<P1: AsRef<Utf8Path>, P2: AsRef<Utf8Path>>(
    src: P1,
    tgt: P2
) -> Result<(), ToOrgamsError> {
    let src = src.as_ref();
    let tgt = tgt.as_ref();
    let code = std::fs::read_to_string(src)
        .map_err(|e| ToOrgamsError(format!("Error while reading {}. {}", src, e.to_string())))?;
    let lst = parse_z80(code)
        .map_err(|e| ToOrgamsError(format!("Error while parsing {}. {}", src, e.to_string())))?;
    let lst = lst.as_slice();
    let orgams = lst.to_orgams_string()?;
    std::fs::write(tgt, orgams.as_bytes())
        .map_err(|e| format!("Error while saving {}. {}", tgt, e.to_string()).into())
}

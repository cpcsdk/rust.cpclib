use std::fmt::Display;
use std::ops::Deref;

use beef::lean::Cow;
use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use cpclib_tokens::{
    BinaryOperation, DataAccess, DataAccessElem, Expr, ExprElement, ListingElement, MacroParam, MacroParamElement, Mnemonic, TestKind, TestKindElement, Token
};

use crate::{
    parse_z80, LocatedDataAccess, LocatedExpr, LocatedMacroParam, LocatedTestKind, MayHaveSpan, ParserContext, ParserContextBuilder, SourceString, TokenExt, Z80Span
};


fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
    let ctx = Box::new(
        ParserContextBuilder::default()
            .set_context_name("TEST")
            .build(code)
    );
    let span = Z80Span::new_extra(code, ctx.deref());
    (ctx, span)
}

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

macro_rules! macro_params_to_orgams {
    () => {
        fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
            let repr: String = if self.is_single() {
                let arg = self.single_argument();
                let (_ctx ,mut code) = ctx_and_span(unsafe{std::mem::transmute(arg.deref())});
                let value = crate::located_expr(&mut code);
                match value {
                    Ok(expr) => expr.to_orgams_string()?.into_owned(),
                    Err(_) => arg.into_owned()
                }
            }
            else {
                unimplemented!("We consider it does not happens with ORGAMS")
            };

            Ok(repr.into())
        }
    };
}


impl ToOrgams for MacroParam {
    macro_params_to_orgams!();
}

impl ToOrgams for LocatedMacroParam {
    macro_params_to_orgams!();
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
                Self::Value(v, ..) => {
                    if self.has_span() {
                        // basm allow _ between numbers
                        let span = self.span().as_str().replace("_", "");
                        if span.starts_with("0x") || span.starts_with("0X") {
                            format!("&{}", &span[2..])
                        }
                        else if span.starts_with("#") {
                            format!("&{}", &span[1..])
                        }
                        else {
                            format!("{}", v)
                        }
                    }
                    else {
                        format!("{}", v)
                    }
                },

                Self::Label(l) => {
                    format!("{}", l)
                },

                Self::String(s) => {
                    format!("\"{}\"", s)
                },

                Self::BinaryOperation(op, left, right, ..) => {
                    let rleft = left.to_orgams_string()?;
                    let rright = right.to_orgams_string()?;
                    let rleft = rleft.as_ref();
                    let op = op.to_orgams_string()?;

                    let protect = |expr: &Self, repr: &str| -> String {
                        if expr.is_label() || expr.is_value() {
                            repr.into()
                        }
                        else {
                            format!("[{}]", repr).into()
                        }
                    };

                    let rleft = protect(left, rleft.as_ref());
                    let rright = protect(right, rright.as_ref());

                    format!("{}{}{}", rleft, op, rright)
                },

                _ => unimplemented!("{:?}", self)
            };

            Ok(repr.into())
        }
    };
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
            }
            else {
                Err(format!("{:?}", self).into())
            }
        }
    };
}

impl ToOrgams for LocatedTestKind {
    test_kind_to_orgams!();
}

impl ToOrgams for TestKind {
    test_kind_to_orgams!();
}

macro_rules! data_access_to_orgams {
    () => {
        fn to_orgams_string(&self) -> Result<Cow<str>, ToOrgamsError> {
            let repr = if self.is_expression() {
                let exp = self.get_expression().unwrap();
                return exp.to_orgams_string();
            }
            else if self.is_memory() || self.is_port_n() {
                let exp = self.get_expression().unwrap();
                let exp = exp.to_orgams_string()?;
                format!("({})", exp)
            }
            else if self.is_register16()
                || self.is_register8()
                || self.is_indexregister16()
                || self.is_indexregister8()
                || self.is_port_c()
                || self.is_address_in_register16()
                || self.is_address_in_indexregister16()
                || self.is_flag_test()
            {
                self.to_string().to_lowercase()
            }
            else {
                unimplemented!("{:?}", self)
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

impl<T> ToOrgams for T
where
    T: TokenExt + MayHaveSpan + ListingElement + ToString + ?Sized,
    T::DataAccess: ToOrgams,
    T::Expr: ToOrgams,
    T::TestKind: ToOrgams,
    T::MacroParam: ToOrgams
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
                  s.to_orgams_string().unwrap()
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

        let comment_token = |token: &T| -> Result<Cow<str>, ToOrgamsError> {
            let repr = token.to_string();
            let repr: String = repr.lines().map(|l| format!(" ; {l}")).join("\n");
            let token = Token::Comment(format!("; {repr}",));
            let res = token.to_orgams_string()?;
            Ok(res.into_owned().into())
        };

        let handle_print = |token: &T| -> Result<Cow<str>, ToOrgamsError> { comment_token(token) };

        let handle_assert = |token: &T| -> Result<Cow<str>, ToOrgamsError> { comment_token(token) };

        let handle_data = |token: &T|  -> Result<Cow<str>, ToOrgamsError> { 
            let exprs = token.data_exprs()
                .iter()
                .map(|e| e.to_orgams_string())
                .collect::<Result<Vec<_>, ToOrgamsError>>()?;
            let exprs = exprs.into_iter().join(",");
            let mne = if token.is_db() {
                "BYTE"
            } else if token.is_dw(){
                "WORD"
            } else {
                unreachable!()
            };

            Ok(format!("{} {}", mne, exprs).into())
        };

        // XXX strong limitation, does not yet handle 3 args
        let handle_opcode = |token: &T| -> String {
            dbg!(token);
            let mut op = token
                .mnemonic()
                .unwrap()
                .to_orgams_string()
                .unwrap()
                .to_string();

            if let Some(arg) = token.mnemonic_arg1() {
                op.push(' ');
                op.push_str(&arg.to_orgams_string().unwrap())
            }

            if let Some(arg) = token.mnemonic_arg2() {
                if token.mnemonic_arg1().is_some() {
                    op.push(',');
                }
                else {
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
            let mut content = format!(
                "{}\n{}",
                test.to_orgams_string().unwrap(),
                code.to_orgams_string().unwrap()
            );

            if let Some(code) = token.if_else() {
                content.push_str("\n\tELSE\n");
                content.push_str(&code.to_orgams_string().unwrap());
            }

            content.push_str("\n\tEND\n");
            content
        };

        // An include is injected in the file
        // FNAME must be encoded within a string. TO improve this aspect, it is necessary to assemble the file and manipulate the `Env` and its symbols table
        let handle_include = |token: &T| -> Result<String, ToOrgamsError> {
            let fname = token.include_fname().string();
            let mut include = format!(" ; START Included from {fname}\n");
            let content = convert_from(fname)?;
            include.push_str(&format!("{content}\n ; STOP Included from {fname}\n"));
            Ok(include)
        };

        // This is the default behavior that changes nothing
        let repr = if self.is_opcode() {
            Cow::owned(handle_opcode(self))
        }
        else if self.is_macro_definition() {
            handle_macro_definition(self)
        }
        else if self.is_print() {
            handle_print(self)?
        }
        else if self.is_call_macro_or_build_struct() {
            handle_macro_call(self)
        }
        else if self.is_assign() {
            handle_assign(self).into()
        }
        else if self.is_equ() {
            handle_equ(self).into()
        }
        else if self.is_if() {
            handle_if(self).into()
        }
        else if self.is_include() {
            handle_include(self)?.into()
        }
        else if self.is_assert() {
            handle_assert(self)?.into()
        }
        else if self.is_db() || self.is_dw() {
            handle_data(self)?.into()
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

pub fn convert_source(code: &str) -> Result<String, ToOrgamsError> {
    let lst = parse_z80(code)
        .map_err(|e| ToOrgamsError(format!("Error while parsing. {}", e.to_string())))?;
    let lst = lst.as_slice();
    lst.to_orgams_string().map(|s| s.into_owned())
}

pub fn convert_from<P: AsRef<Utf8Path>>(p: P) -> Result<String, ToOrgamsError> {
    let p = p.as_ref();
    let code = std::fs::read_to_string(p)
        .map_err(|e| ToOrgamsError(format!("Error while reading {}. {}", p, e.to_string())))?;
    convert_source(&code)
        .map_err(|e| format!("Error while handling {}. {}", p, e.to_string()).into())
}

/// COnvert a basm txt source file as a orgams text source file.
/// There are tons of current limitations. I have only implemented what I need
/// TODO - convert expressions to be orgams compatible. REwrite them ? Write parenthesis ?
/// TODO - rewrite macros
pub fn convert_from_to<P1: AsRef<Utf8Path>, P2: AsRef<Utf8Path>>(
    src: P1,
    tgt: P2
) -> Result<(), ToOrgamsError> {
    let src = src.as_ref();
    let tgt = tgt.as_ref();
    let orgams = convert_from(src)?;
    std::fs::write(tgt, orgams.as_bytes())
        .map_err(|e| format!("Error while saving {}. {}", tgt, e.to_string()).into())
}

#[cfg(test)]
mod test {
    use std::ops::Deref;

    use cpclib_common::winnow::error::ParseError;
    use cpclib_common::winnow::Parser;
    use cpclib_tokens::{DataAccess, Expr};

    use super::{ctx_and_span, ToOrgams};
    use crate::{
        located_expr, AssemblerError, InnerZ80Span, ParserContext, ParserContextBuilder,
        Z80ParserError, Z80Span
    };

    #[derive(Debug)]
    struct TestResult<O: std::fmt::Debug> {
        ctx: Box<ParserContext>,
        span: Z80Span,
        res: Result<O, ParseError<InnerZ80Span, Z80ParserError>>
    }

    impl<O: std::fmt::Debug> Deref for TestResult<O> {
        type Target = Result<O, ParseError<InnerZ80Span, Z80ParserError>>;

        fn deref(&self) -> &Self::Target {
            &self.res
        }
    }



    fn parse_test<O, P: Parser<InnerZ80Span, O, Z80ParserError>>(
        mut parser: P,
        code: &'static str
    ) -> TestResult<O>
    where
        O: std::fmt::Debug
    {
        let (ctx, span) = ctx_and_span(code);
        let res = parser.parse(span.0);
        if let Err(e) = &res {
            let e = e.inner();
            let e = AssemblerError::SyntaxError { error: e.clone() };
            eprintln!("Parse error: {}", e);
        }

        TestResult { ctx, span, res }
    }

    #[test]
    fn test_expression() {
        assert_eq!(
            parse_test(located_expr, "25")
                .as_ref()
                .unwrap()
                .to_orgams_string()
                .unwrap(),
            "25"
        );
        assert_eq!(
            parse_test(located_expr, "0x25")
                .as_ref()
                .unwrap()
                .to_orgams_string()
                .unwrap(),
            "&25"
        );
    }

    #[test]
    fn test_data_access() {
        assert_eq!(
            DataAccess::Expression(Expr::Value(25))
                .to_orgams_string()
                .unwrap(),
            "25"
        );
        assert_eq!(
            DataAccess::Memory(Expr::Value(25))
                .to_orgams_string()
                .unwrap(),
            "(25)"
        );
    }
}

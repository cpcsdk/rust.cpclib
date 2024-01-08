use std::fmt::{Display, Write};

use cpclib_common::smol_str::SmolStr;
use cpclib_tokens::{Expr, ExprFormat, ExprResult, FormattedExpr};

use super::Env;
use crate::error::AssemblerError;

#[derive(Clone, Debug)]
pub enum PreprocessedFormattedExpr {
    String(SmolStr),
    Char(char),
    ExprResult(ExprResult),
    Formatted(ExprFormat, i32)
}

#[derive(Default, Clone, Debug)]
pub struct PreprocessedFormattedString {
    components: Vec<PreprocessedFormattedExpr>
}

impl Display for PreprocessedFormattedExpr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocessedFormattedExpr::String(s) => f.write_str(s),
            PreprocessedFormattedExpr::Char(c) => f.write_char(*c),
            PreprocessedFormattedExpr::ExprResult(e) => f.write_str(&e.to_string()),
            PreprocessedFormattedExpr::Formatted(f2, v) => {
                f.write_str(&f2.string_representation(*v))
            },
        }
    }
}

impl Display for PreprocessedFormattedString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.components.iter().for_each(|c| {
            c.fmt(f);
        });
        Ok(())
    }
}

impl PreprocessedFormattedExpr {
    pub fn try_new(
        fe: &FormattedExpr,
        env: &Env
    ) -> Result<PreprocessedFormattedExpr, AssemblerError> {
        match fe {
            FormattedExpr::Raw(Expr::String(string)) => {
                Ok(PreprocessedFormattedExpr::String(string.clone()))
            },
            FormattedExpr::Raw(Expr::Char(char)) => Ok(PreprocessedFormattedExpr::Char(*char)),
            FormattedExpr::Raw(expr) => {
                let value = env.resolve_expr_may_fail_in_first_pass(expr)?;
                Ok(PreprocessedFormattedExpr::ExprResult(value))
            },
            FormattedExpr::Formatted(format, expr) => {
                let value = env.resolve_expr_may_fail_in_first_pass(expr)?.int()? as i32;
                Ok(PreprocessedFormattedExpr::Formatted(format.clone(), value))
            }
        }
    }
}

impl PreprocessedFormattedString {
    pub fn try_new(info: &[FormattedExpr], env: &Env) -> Result<Self, AssemblerError> {
        let mut components = Vec::with_capacity(info.len());
        for component in info.iter() {
            components.push(PreprocessedFormattedExpr::try_new(component, env)?);
        }

        Ok(Self { components })
    }
}

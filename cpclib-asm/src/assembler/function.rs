use std::any::Any;

use crate::{error::AssemblerError, preamble::LocatedToken, Visited};
use cpclib_common::itertools::Itertools;
use cpclib_tokens::{Expr, ExprResult, ListingElement, Token};

use super::Env;

/// Returns the expression of the RETURN directive
pub trait ReturnExpr {
    fn return_expr(&self) -> Option<&Expr>;
}

impl ReturnExpr for Token {
    fn return_expr(&self) -> Option<&Expr> {
        match self {
            Token::Return(exp) => Some(exp),
            _ => None,
        }
    }
}

impl ReturnExpr for LocatedToken {
    fn return_expr(&self) -> Option<&Expr> {
        match self {
            LocatedToken::Standard { token, .. } => token.return_expr(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnyFunction<T: ListingElement + Visited> {
    name: String,
    args: Vec<String>,
    inner: Vec<T>,
}

impl<T: ListingElement + Visited + Clone> AnyFunction<T> {
    fn new(name: &str, args: &[String], inner: &[T]) -> Self {
        AnyFunction {
            name: name.to_owned(),
            args: args.to_vec(),
            inner: inner.to_vec(),
        }
    }
}

impl<T: ListingElement + Visited + ReturnExpr> AnyFunction<T> {
    pub fn eval(&self, env: &Env, params: Vec<ExprResult>) -> Result<ExprResult, AssemblerError> {
        if self.args.len() != params.len() {
            return Err(AssemblerError::FunctionWithWrongNumberOfArguments(
                self.name.clone(),
                self.args.len(),
                params.len(),
            ));
        }
        // we copy the environement to be sure no bug can modify it
        // and to keep the symbol table fixed.
        // a better alternative would be to backup the symbol table
        let mut env = env.clone();

        // set the parameters
        for param in self.args.iter().zip(params.iter()) {
            // TODO modify the code according to the value
            env.add_function_parameter_to_symbols_table(
                format!("{{{}}}", param.0),
                param.1.clone(),
            )
            .unwrap();
        }

        for token in self.inner.iter() {
            dbg!(token, token.return_expr());
            token
                .visited(&mut env)
                .map_err(|e| AssemblerError::FunctionError(self.name.clone(), box e))?;

            if env.return_value.is_some() {
                return Ok(env.return_value.take().unwrap());
            }
        }

        Err(AssemblerError::FunctionWithoutReturn(self.name.clone()))
    }
}

#[derive(Debug, Clone)]
pub enum Function {
    Located(AnyFunction<LocatedToken>),
    Standard(AnyFunction<Token>),
    HardCoded(HardCodedFunction),
}

#[derive(Debug, Clone)]
pub enum HardCodedFunction {
    ByteToMode0PenAt,
    ByteToMode1PenAt,
    ByteToMode2PenAt,

    PenAtToMode0Byte,
    PenAtToMode1Byte,
    PenAtToMode2Byte,

    PensToMode0Byte,
    PensToMode1Byte,
    PensToMode2Byte,
}

impl HardCodedFunction {
    pub fn nb_expected_params(&self) -> usize {
        match self {
            HardCodedFunction::ByteToMode0PenAt => 2,
            HardCodedFunction::ByteToMode1PenAt => 2,
            HardCodedFunction::ByteToMode2PenAt => 2,

            HardCodedFunction::PenAtToMode0Byte => 2,
            HardCodedFunction::PenAtToMode1Byte => 2,
            HardCodedFunction::PenAtToMode2Byte => 2,

            HardCodedFunction::PensToMode0Byte => 2,
            HardCodedFunction::PensToMode1Byte => 4,
            HardCodedFunction::PensToMode2Byte => 8,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            HardCodedFunction::ByteToMode0PenAt => "byte_to_mode0_pixel_at",
            HardCodedFunction::ByteToMode1PenAt => "byte_to_mode1_pixel_at",
            HardCodedFunction::ByteToMode2PenAt => "byte_to_mode2_pixel_at",

            HardCodedFunction::PenAtToMode0Byte => "pen_at_mode0_byte",
            HardCodedFunction::PenAtToMode1Byte => "pen_at_mode1_byte",
            HardCodedFunction::PenAtToMode2Byte => "pen_at_mode2_byte",

            HardCodedFunction::PensToMode0Byte => "pens_to_mode0_byte",
            HardCodedFunction::PensToMode1Byte => "pens_to_mode1_byte",
            HardCodedFunction::PensToMode2Byte => "pens_to_mode2_byte",
        }
    }

    pub fn eval(&self, env: &Env, params: Vec<ExprResult>) -> Result<ExprResult, AssemblerError> {
        if self.nb_expected_params() != params.len() {
            return Err(AssemblerError::FunctionWithWrongNumberOfArguments(
                self.name().into(),
                self.nb_expected_params(),
                params.len(),
            ));
        }

        match self {
            HardCodedFunction::ByteToMode0PenAt => {
				Ok(cpclib_image::pixels::mode0::byte_to_pens(params[0].int()? as _)[params[1].int()? as usize %2].number().into())
			},
            HardCodedFunction::ByteToMode1PenAt =>  {
				Ok(cpclib_image::pixels::mode1::byte_to_pens(params[0].int()? as _)[params[1].int()? as usize %4].number().into())
			},
            HardCodedFunction::ByteToMode2PenAt =>  {
				Ok(cpclib_image::pixels::mode2::byte_to_pens(params[0].int()? as _)[params[1].int()? as usize %8].number().into())
			},

            HardCodedFunction::PenAtToMode0Byte => {
                Ok(cpclib_image::pixels::mode0::pen_to_pixel_byte(
                    (params[0].int()? as u8 %16).into(),
                    (params[1].int()? as u8 %2).into(),
                )
                .into())
            },

            HardCodedFunction::PenAtToMode1Byte => {
                Ok(cpclib_image::pixels::mode1::pen_to_pixel_byte(
                    (params[0].int()? as u8 %4).into(),
                    (params[1].int()? as u8 %4).into(),
                )
                .into())
            },

            HardCodedFunction::PenAtToMode2Byte => {
                Ok(cpclib_image::pixels::mode2::pen_to_pixel_byte(
                    (params[0].int()? as u8 %2).into(),
                    (params[1].int()? as u8 %8).into(),
                )
                .into())
            }

            HardCodedFunction::PensToMode0Byte => {
				Ok(cpclib_image::pixels::mode0::pens_to_byte(params[0].int()?.into(), params[1].int()?.into()).into())
			},
            HardCodedFunction::PensToMode1Byte =>  {
				Ok(cpclib_image::pixels::mode1::pens_to_byte(params[0].int()?.into(), params[1].int()?.into(), params[2].int()?.into(), params[3].int()?.into()).into())
			},
            HardCodedFunction::PensToMode2Byte => {
				Ok(cpclib_image::pixels::mode2::pens_to_byte(
					params[0].int()?.into(), params[1].int()?.into(), params[2].int()?.into(), params[3].int()?.into(),
					params[4].int()?.into(), params[5].int()?.into(), params[6].int()?.into(), params[7].int()?.into()
				
				).into())
			},
        }
    }
}

impl Function {
    pub fn new_located(
        name: &str,
        args: &[String],
        inner: &[LocatedToken],
    ) -> Result<Self, AssemblerError> {
        if inner.is_empty() {
            return Err(AssemblerError::FunctionWithEmptyBody(name.to_owned()));
        }
        return Ok(Function::Located(AnyFunction::<LocatedToken>::new(
            name, args, inner,
        )));
    }

    pub fn new_standard(
        name: &str,
        args: &[String],
        inner: &[Token],
    ) -> Result<Self, AssemblerError> {
        if inner.is_empty() {
            return Err(AssemblerError::FunctionWithEmptyBody(name.to_owned()));
        }
        return Ok(Function::Standard(AnyFunction::<Token>::new(
            name, args, inner,
        )));
    }

    pub fn eval(&self, env: &Env, params: Vec<ExprResult>) -> Result<ExprResult, AssemblerError> {
        match self {
            Self::Located(f) => f.eval(env, params),
            Self::Standard(f) => f.eval(env, params),
            Self::HardCoded(f) => f.eval(env, params),
        }
    }
}

pub trait FunctionBuilder {
    fn new(name: &str, args: &[String], inner: &[Self]) -> Result<Function, AssemblerError>
    where
        Self: Sized;
}

impl FunctionBuilder for LocatedToken {
    fn new(
        name: &str,
        args: &[String],
        inner: &[LocatedToken],
    ) -> Result<Function, AssemblerError> {
        Function::new_located(name, args, inner)
    }
}

impl FunctionBuilder for Token {
    fn new(name: &str, args: &[String], inner: &[Token]) -> Result<Function, AssemblerError> {
        Function::new_standard(name, args, inner)
    }
}

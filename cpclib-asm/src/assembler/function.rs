use std::any::Any;

use cpclib_common::itertools::Itertools;
use cpclib_tokens::{Expr, ExprResult, ListingElement, Token};
use crate::{Visited, error::AssemblerError, preamble::LocatedToken};

use super::Env;

/// Returns the expression of the RETURN directive
pub trait ReturnExpr {
	fn return_expr(&self) -> Option<&Expr>; 
}

impl ReturnExpr for Token {
    fn return_expr(&self) -> Option<&Expr> {
        match self {
			Token::Return(exp) => Some(exp),
			_ => None
		}
    }
}

impl ReturnExpr for LocatedToken {
    fn return_expr(&self) -> Option<&Expr> {
        match self {
			LocatedToken::Standard{token, ..} => token.return_expr(),
			_ => None
		}
    }
}

#[derive(Debug, Clone)]
pub struct AnyFunction <T: ListingElement + Visited> {
    name: String,
    args: Vec<String>,
    inner: Vec<T>
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
				params.len()
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
				param.1.clone()).unwrap();
		}

		for token in self.inner.iter() {
			dbg!(token, token.return_expr());
			token.visited(&mut env)
							.map_err(|e| {
								AssemblerError::FunctionError(
									self.name.clone(),
								box e)
							})?;
							
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
	Standard(AnyFunction<Token>)
}


impl Function {

	pub fn new_located(name: &str, args: &[String], inner: &[LocatedToken]) -> Result<Self, AssemblerError>  {
		if inner.is_empty() {
			return Err(AssemblerError::FunctionWithEmptyBody(name.to_owned()));
		}
		return Ok(Function::Located(
			AnyFunction::<LocatedToken>::new(
				name,
				args,
				inner)
			)
		);
	}

	pub fn new_standard(name: &str, args: &[String], inner: &[Token]) -> Result<Self, AssemblerError>  {
		if inner.is_empty() {
			return Err(AssemblerError::FunctionWithEmptyBody(name.to_owned()));
		}
		return Ok(Function::Standard(
			AnyFunction::<Token>::new(
				name,
				args,
				inner)
			)
		);
	}

	pub fn eval(&self, env: &Env, params: Vec<ExprResult>) -> Result<ExprResult, AssemblerError> {
		match self {
			Self::Located(f) => f.eval(env, params),
			Self::Standard(f) => f.eval(env, params),
		}
	}
}

pub trait FunctionBuilder{
	fn new(name: &str, args: &[String], inner: &[Self]) -> Result<Function, AssemblerError> where Self: Sized;
}

impl FunctionBuilder for LocatedToken {
	fn new(name: &str, args: &[String], inner: &[LocatedToken]) -> Result<Function, AssemblerError> {
		Function::new_located(name, args, inner)
	}
}

impl FunctionBuilder for Token {
	fn new(name: &str, args: &[String], inner: &[Token]) -> Result<Function, AssemblerError> {
		Function::new_standard(name, args, inner)
	}
}

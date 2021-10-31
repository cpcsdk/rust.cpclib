use cpclib_tokens::{ExprResult, ExpressionTypeError};

use crate::error::{AssemblerError, ExpressionError};


/// Create a new list
pub fn list_new(count: usize, value: ExprResult) -> ExprResult {
	ExprResult::List(vec![value; count])
}

/// Create a new string
pub fn string_new(count: usize, value: ExprResult) -> Result<ExprResult, AssemblerError>  {
	let value = value.char()?;
	let s = (0..count).map(|_| value).collect::<String>();
	Ok(ExprResult::String(s))
}

/// Modify a list or a string
pub fn list_set(mut list: ExprResult, index: usize, value: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
	match list {
		ExprResult::String(mut s) => {
			if index >= s.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(s.len(), index)));
			}
			let c = value.int()? as u8 as char;
			let c = format!("{}", c);
			s.replace_range(index..index+1, &c);
			Ok(ExprResult::String(s))
		},
		ExprResult::List(mut l) => {
			if index >= l.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(l.len(), index)));
			}
			l[index] = value;
			Ok(ExprResult::List(l))
		}

		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a list", list)
				}
			)
		))
	}
}

/// Get an item in a list of string
pub fn list_get(mut list: ExprResult, index: usize) -> Result<ExprResult, crate::AssemblerError> {
	match list {
		ExprResult::String(s) => {
			if index >= s.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(s.len(), index)));
			}
			Ok(ExprResult::Value(s.chars().nth(index).unwrap() as _))
		},
		ExprResult::List(mut l) => {
			if index >= l.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(l.len(), index)));
			}
			Ok(l[index].clone())
		}

		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a list", list)
				}
			)
		))
	}
}



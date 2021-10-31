use cpclib_tokens::{ExprResult, ExpressionTypeError};

use crate::error::{AssemblerError, ExpressionError};
use substring::Substring;

pub fn fix_string(mut s: String) -> String {
	s.replace("\\n", "\n")
}


/// Create a new list
pub fn list_new(count: usize, value: ExprResult) -> ExprResult {
	ExprResult::List(vec![value; count])
}

/// Create a new string
pub fn string_new(count: usize, value: ExprResult) -> Result<ExprResult, AssemblerError>  {
	let value = value.char()?;
	let s = (0..count).map(|_| value).collect::<String>();
	Ok(ExprResult::String(fix_string(s)))
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
			Ok(ExprResult::String(fix_string(s)))
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




/// Get a sublist  a list of string
pub fn list_sublist(mut list: ExprResult, start: usize, end: usize) -> Result<ExprResult, crate::AssemblerError> {

	match list {
		ExprResult::String(s) => {
			if start >= s.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(s.len(), start)));
			}
			if end >= s.len()+1 {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(s.len(), end+1)));
			}
			Ok(ExprResult::String(s.substring(start, end).to_owned()))
		},
		ExprResult::List(mut l) => {
			if start >= l.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(l.len(), start)));
			}
			if end >= l.len()+1 {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(l.len(), end+1)));
			}
			Ok(ExprResult::List(l[start..end].to_vec()))
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

pub fn list_len(list: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
	match list {
		ExprResult::List(l) => Ok(l.len().into()),
		ExprResult::String(s) => Ok(s.len().into()),
		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a list", list)
				}
			)
		))
	}
}

pub fn string_push(mut s1: ExprResult, mut s2: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
	match (s1, s2) {
		(ExprResult::String(mut s1), ExprResult::String(mut s2)) => {
			s1 += &fix_string(s2);
			Ok(ExprResult::String(s1))
		},

		(ExprResult::String(mut s1), ExprResult::Float(s2)) => {
			s1 += &s2.into_inner().to_string();
			Ok(ExprResult::String(s1))
		},

		(ExprResult::String(mut s1), ExprResult::Value(s2)) => {
			s1 += &s2.to_string();
			Ok(ExprResult::String(s1))
		},
		
		(ExprResult::String(mut s1), ExprResult::Bool(s2)) => {
			s1 += &s2.to_string();
			Ok(ExprResult::String(s1))
		},

		_ => {
			Err(AssemblerError::ExpressionError(
				ExpressionError::OwnError(
					box AssemblerError::AssemblingError {
						msg: format!("string_push called with wrong types")
					}
				)
			))
		}
	}
}

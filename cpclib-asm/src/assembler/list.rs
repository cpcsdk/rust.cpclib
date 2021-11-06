use std::borrow::Borrow;

use cpclib_common::itertools::Itertools;
use cpclib_common::smol_str::SmolStr;
use cpclib_tokens::{ExprResult, ExpressionTypeError};

use crate::error::{AssemblerError, ExpressionError};
use substring::Substring;

pub fn fix_string<S: Borrow<str>>(mut s: S) -> SmolStr {
	s.borrow().replace("\\n", "\n").into()
}


/// Create a new list
pub fn list_new(count: usize, value: ExprResult) -> ExprResult {
	ExprResult::List(vec![value; count])
}

/// Create a new string
pub fn string_new(count: usize, value: ExprResult) -> Result<ExprResult, AssemblerError>  {
	let value = value.char()?;
	let s = (0..count).map(|_| value).collect::<SmolStr>();
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
			let mut s = s.to_string();
			s.replace_range(index..index+1, &c);
			Ok(ExprResult::String(fix_string(s)))
		},
		ExprResult::List(_) => {
			if index >= list.list_len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(list.list_len(), index)));
			}
			list.list_set(index,  value);
			Ok(list)
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
pub fn list_get(mut list: &ExprResult, index: usize) -> Result<ExprResult, crate::AssemblerError> {
	match list {
		ExprResult::String(s) => {
			if index >= s.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(s.len(), index)));
			}
			Ok(ExprResult::Value(s.chars().nth(index).unwrap() as _))
		},
		ExprResult::List(_) => {
			if index >= list.list_len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(list.list_len(), index)));
			}
			Ok(list.list_get(index).clone())
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
pub fn list_sublist(mut list: &ExprResult, start: usize, end: usize) -> Result<ExprResult, crate::AssemblerError> {

	match list {
		ExprResult::String(s) => {
			if start >= s.len() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(s.len(), start)));
			}
			if end >= s.len()+1 {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(s.len(), end+1)));
			}
			Ok(ExprResult::String(s.substring(start, end).into()))
		},
		ExprResult::List(l) => {
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

pub fn list_len(list: &ExprResult) -> Result<ExprResult, crate::AssemblerError> {
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

pub fn list_push(mut list: ExprResult, mut elem: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
	match list {
		ExprResult::List(mut l) => {
			l.push(elem);
			Ok(ExprResult::List(l))
		},
		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a list", list)
				}
			)
		))
	}
}

pub fn list_sort(mut list: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
	match list {
		ExprResult::List(mut l) => {
			l.sort();
			Ok(ExprResult::List(l))
		},
		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a list", list)
				}
			)
		))
	}
}




pub fn list_argsort(list: &ExprResult) -> Result<ExprResult, crate::AssemblerError> {
	match list {
		ExprResult::List( l) => {
			//https://stackoverflow.com/questions/69764050/how-to-get-the-indices-that-would-sort-a-vector-in-rust
			fn argsort<T: Ord>(data: &[T]) -> Vec<ExprResult> {
				let mut indices = (0..data.len())
								.map(|i| ExprResult::from(i))
								.collect_vec();
				indices.sort_by_key(|i| &data[i.int().unwrap() as usize]);
				indices
			}

			let l = argsort(l);
			Ok(ExprResult::List(l.into()))
		},
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
			let s1 = s1.to_string() + &fix_string(s2);
			Ok(ExprResult::String(s1.into()))
		},
		(ExprResult::String(mut s1), ExprResult::List(mut l)) => {
			let mut s1 = s1.to_string() + "[";

			for (i, e)in l.into_iter().enumerate() {
				if i!= 0 {
					s1 += ","
				}

				s1 = string_push(s1.into(), e)?
						.string()
						.unwrap()
						.to_string();
			}

			s1 += "]";
			Ok(ExprResult::String(s1.into()))
		},


		(ExprResult::String(mut s1), ExprResult::Float(s2)) => {
			let mut s1 = s1.to_string();
			s1 += &s2.into_inner().to_string();
			Ok(ExprResult::String(s1.into()))
		},

		(ExprResult::String(mut s1), ExprResult::Value(s2)) => {
			let mut s1 = s1.to_string();

			s1 += &s2.to_string();
			Ok(ExprResult::String(s1.into()))
		},
		
		(ExprResult::String(mut s1), ExprResult::Bool(s2)) => {
			let mut s1 = s1.to_string();

			s1 += &s2.to_string();
			Ok(ExprResult::String(s1.into()))
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

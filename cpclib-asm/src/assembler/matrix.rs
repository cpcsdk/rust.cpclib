use cpclib_tokens::ExprResult;

use crate::{assembler::list::list_new, error::{AssemblerError, ExpressionError}};

/// Create a new matrix
pub fn matrix_new(height: usize, width: usize, value: ExprResult) -> ExprResult {
	ExprResult::Matrix{
		content: vec![list_new(width, value); height],
		width,
		height
	}
}

pub fn matrix_col(matrix: &ExprResult, x: usize) -> Result<ExprResult, crate::AssemblerError> {
	match matrix {
		ExprResult::Matrix{..} => {
			if x >= matrix.matrix_width() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(matrix.matrix_width(), x)));
			}

			Ok(matrix.matrix_col(x))
		},


		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a matrix", matrix)
				}
			)
		))
	}
}

pub fn matrix_row(matrix: &ExprResult, y: usize) -> Result<ExprResult, crate::AssemblerError> {
	match matrix {
		ExprResult::Matrix{  ..} => {
			if y >= matrix.matrix_height() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(matrix.matrix_height(), y)));
			}

			Ok(matrix.matrix_row(y).clone())
		},


		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a matrix", matrix)
				}
			)
		))
	}
}

pub fn matrix_set(mut matrix: ExprResult, y: usize, x: usize, value: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
	match matrix {
		ExprResult::Matrix{  ..} => {
			if y >= matrix.matrix_height() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(matrix.matrix_height(), y)));
			}
			if x >= matrix.matrix_width() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(matrix.matrix_width(), x)));
			}
			matrix.matrix_set(y, x, value);
			Ok(matrix)
		}

		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a matrix", matrix)
				}
			)
		))
	}
}

pub fn matrix_get(matrix: &ExprResult, y: usize, x: usize) -> Result<ExprResult, crate::AssemblerError> {
	match matrix {
		ExprResult::Matrix{ ..} => {
			if y >= matrix.matrix_height() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(matrix.matrix_height(), y)));
			}
			if x >= matrix.matrix_width() {
				return Err(AssemblerError::ExpressionError(ExpressionError::InvalidSize(matrix.matrix_width(), x)));
			}
			Ok(matrix.matrix_get(y, x).clone())
		}

		_ => Err(AssemblerError::ExpressionError(
			ExpressionError::OwnError(
				box AssemblerError::AssemblingError {
					msg: format!("{} is not a matrix", matrix)
				}
			)
		))
	}
}
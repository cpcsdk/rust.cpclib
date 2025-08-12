use cpclib_common::itertools::Itertools;
use cpclib_tokens::ExprResult;

use crate::assembler::list::list_new;
use crate::error::{AssemblerError, ExpressionError};
use crate::list::list_get;

/// Create a new matrix
pub fn matrix_new(height: usize, width: usize, value: ExprResult) -> ExprResult {
    ExprResult::Matrix {
        content: vec![list_new(width, value); height],
        width,
        height
    }
}

pub fn matrix_from_list(expr: &ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match expr {
        ExprResult::List(l) => {
            let height = l.len();

            assert!(l.iter().all(|row| row.is_list())); // TODO generate an error message
            let lengths = l.iter().map(|row| row.list_len()).collect_vec();
            let width = lengths[0]; // TODO handle case of empty matrix
            // TODO handle error properly
            assert!(lengths.iter().all(|&len| len == width));

            Ok(ExprResult::Matrix {
                width,
                height,
                content: l.clone()
            })
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{expr} is not a a list")
                })
            )))
        },
    }
}

pub fn matrix_col(matrix: &ExprResult, x: usize) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        ExprResult::Matrix { .. } => {
            if x >= matrix.matrix_width() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(matrix.matrix_width(), x)
                ));
            }

            Ok(matrix.matrix_col(x))
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

pub fn matrix_set_col(
    mut matrix: ExprResult,
    x: usize,
    col: &ExprResult
) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        ExprResult::Matrix { width, .. } => {
            if x >= width {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(width, x)
                ));
            }

            match col {
                ExprResult::List(_) => {
                    if col.list_len() != width {
                        return Err(AssemblerError::ExpressionError(
                            ExpressionError::InvalidSize(width, col.list_len())
                        ));
                    }

                    matrix.matrix_set_col(x, col.list_content());
                    Ok(matrix)
                },
                _ => {
                    Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                        Box::new(AssemblerError::AssemblingError {
                            msg: format!("{matrix} is not a list")
                        })
                    )))
                },
            }
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

pub fn matrix_row(matrix: &ExprResult, y: usize) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        ExprResult::Matrix { .. } => {
            if y >= matrix.matrix_height() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(matrix.matrix_height(), y)
                ));
            }

            Ok(matrix.matrix_row(y).clone())
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

pub fn matrix_set_row(
    mut matrix: ExprResult,
    y: usize,
    row: &ExprResult
) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        ExprResult::Matrix {
            ref mut content,
            height,
            ..
        } => {
            if y >= height {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(height, y)
                ));
            }

            match row {
                ExprResult::List(_) => {
                    if row.list_len() != height {
                        return Err(AssemblerError::ExpressionError(
                            ExpressionError::InvalidSize(height, row.list_len())
                        ));
                    }

                    content[y] = row.clone();
                    Ok(matrix)
                },
                _ => {
                    Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                        Box::new(AssemblerError::AssemblingError {
                            msg: format!("{matrix} is not a list")
                        })
                    )))
                },
            }
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

pub fn matrix_set(
    mut matrix: ExprResult,
    y: usize,
    x: usize,
    value: ExprResult
) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        ExprResult::Matrix { .. } => {
            if y >= matrix.matrix_height() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(matrix.matrix_height(), y)
                ));
            }
            if x >= matrix.matrix_width() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(matrix.matrix_width(), x)
                ));
            }
            matrix.matrix_set(y, x, value);
            Ok(matrix)
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

pub fn matrix_get(
    matrix: &ExprResult,
    y: usize,
    x: usize
) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        // A matrix is explicitely coded, so lets use it directly
        ExprResult::Matrix { .. } => {
            if y >= matrix.matrix_height() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(matrix.matrix_height(), y)
                ));
            }
            if x >= matrix.matrix_width() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(matrix.matrix_width(), x)
                ));
            }
            Ok(matrix.matrix_get(y, x).clone())
        },

        // Maybe there is a matrix coded as a list of list
        // it is up to the responsability of the caller to respect that
        ExprResult::List(l) => {
            if y >= l.len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(l.len(), y)
                ));
            }
            let row = &l[y];
            list_get(row, x)
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

pub fn matrix_width(matrix: &ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        ExprResult::Matrix { width, .. } => Ok((*width as i32).into()),
        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

pub fn matrix_height(matrix: &ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match matrix {
        ExprResult::Matrix { height, .. } => Ok((*height as i32).into()),
        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{matrix} is not a matrix")
                })
            )))
        },
    }
}

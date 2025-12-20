use std::borrow::Borrow;

use cpclib_common::smol_str::SmolStr;
use cpclib_tokens::ExprResult;
use substring::Substring;

use crate::error::{AssemblerError, ExpressionError};

pub fn fix_string<S: Borrow<str>>(s: S) -> SmolStr {
    s.borrow().replace("\\n", "\n").into()
}

/// Create a new list
pub fn list_new(count: usize, value: ExprResult) -> ExprResult {
    ExprResult::List(vec![value; count])
}

/// Create a new string
pub fn string_new(count: usize, value: ExprResult) -> Result<ExprResult, AssemblerError> {
    let value = value.char()?;
    let s = (0..count).map(|_| value).collect::<SmolStr>();
    Ok(ExprResult::String(fix_string(s)))
}

/// Modify a list or a string
pub fn list_set(
    mut list: ExprResult,
    index: usize,
    value: ExprResult
) -> Result<ExprResult, crate::AssemblerError> {
    match list {
        ExprResult::String(s) => {
            if index >= s.len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), index)
                ));
            }
            let c = value.int()? as u8 as char;
            let c = format!("{c}");
            let mut s = s.to_string();
            s.replace_range(index..index + 1, &c);
            Ok(ExprResult::String(fix_string(s)))
        },
        ExprResult::List(_) => {
            if index >= list.list_len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(list.list_len(), index)
                ));
            }
            list.list_set(index, value);
            Ok(list)
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                })
            )))
        },
    }
}

/// Get an item in a list of string
pub fn list_get(list: &ExprResult, index: usize) -> Result<ExprResult, crate::AssemblerError> {
    match list {
        ExprResult::String(s) => {
            if index >= s.len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), index)
                ));
            }
            Ok(ExprResult::Value(s.chars().nth(index).unwrap() as _))
        },
        ExprResult::List(_) => {
            if index >= list.list_len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(list.list_len(), index)
                ));
            }
            Ok(list.list_get(index).clone())
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                })
            )))
        },
    }
}

/// Get a sublist  a list of string
pub fn list_sublist(
    list: &ExprResult,
    start: usize,
    end: usize // not included
) -> Result<ExprResult, crate::AssemblerError> {
    match list {
        ExprResult::String(s) => {
            if start >= s.len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), start)
                ));
            }
            if end > s.len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(s.len(), end)
                ));
            }
            Ok(ExprResult::String(s.substring(start, end).into()))
        },
        ExprResult::List(l) => {
            if start >= l.len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(l.len(), start)
                ));
            }
            if end > l.len() {
                return Err(AssemblerError::ExpressionError(
                    ExpressionError::InvalidSize(l.len(), end)
                ));
            }
            Ok(ExprResult::List(l[start..end].to_vec()))
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                })
            )))
        },
    }
}

pub fn list_len(list: &ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match list {
        ExprResult::List(l) => Ok(l.len().into()),
        ExprResult::String(s) => Ok(s.len().into()),
        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                })
            )))
        },
    }
}

pub fn list_push(list: ExprResult, elem: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match list {
        ExprResult::List(mut l) => {
            l.push(elem);
            Ok(ExprResult::List(l))
        },
        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                })
            )))
        },
    }
}

pub fn list_extend(
    list1: ExprResult,
    list2: ExprResult
) -> Result<ExprResult, crate::AssemblerError> {
    match list1 {
        ExprResult::List(mut l) => {
            match list2 {
                ExprResult::List(l2) => {
                    for item in l2 {
                        l.push(item);
                    }
                    Ok(ExprResult::List(l))
                },
                _ => {
                    Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                        Box::new(AssemblerError::AssemblingError {
                            msg: format!("{list2} is not a list")
                        })
                    )))
                },
            }
        },
        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list1} is not a list")
                })
            )))
        },
    }
}

pub fn list_sort(list: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match list {
        ExprResult::List(mut l) => {
            l.sort();
            Ok(ExprResult::List(l))
        },
        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                })
            )))
        },
    }
}

pub fn list_argsort(list: &ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match list {
        ExprResult::List(l) => {
            // https://stackoverflow.com/questions/69764050/how-to-get-the-indices-that-would-sort-a-vector-in-rust
            fn argsort<T: Ord>(data: &[T]) -> Vec<ExprResult> {
                let mut indices = (0..data.len()).map(ExprResult::from).collect::<Vec<_>>();
                indices.sort_by_key(|i| &data[i.int().unwrap() as usize]);
                indices
            }

            let l = argsort(l);
            Ok(ExprResult::List(l))
        },
        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("{list} is not a list")
                })
            )))
        },
    }
}

/// BUG bytes must be enced in utf8
pub fn string_from_list(s1: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match s1 {
        ExprResult::List(l1) => {
            let bytes = l1
                .iter()
                .enumerate()
                .map(|(idx, v)| {
                    let v = v.int()?;
                    if !(0..=255).contains(&v) {
                        Err(AssemblerError::AssemblingError {
                            msg: format!("{v} at {idx} is not a valid byte value")
                        })
                    }
                    else {
                        Ok(v as u8)
                    }
                })
                .collect::<Result<Vec<u8>, AssemblerError>>()?;

            String::from_utf8(bytes)
                .map_err(|e| {
                    AssemblerError::AssemblingError {
                        msg: format!("Error when generating a string. {e}")
                    }
                })
                .map(|s| s.into())
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: "string_from_list must take a list as an argument".to_string()
                })
            )))
        },
    }
}

pub fn string_push(s1: ExprResult, s2: ExprResult) -> Result<ExprResult, crate::AssemblerError> {
    match (&s1, &s2) {
        (ExprResult::Char(s1), ExprResult::Char(s2)) => {
            let s1 = format!("{}{}", *s1 as char, *s2 as char);
            Ok(ExprResult::String(s1.into()))
        },

        (ExprResult::String(s1), ExprResult::Char(s2)) => {
            let s1 = format!("{}{}", s1, *s2 as char);
            Ok(ExprResult::String(s1.into()))
        },

        (ExprResult::String(s1), ExprResult::String(s2)) => {
            let mut result = String::with_capacity(s1.len() + s2.len());
            result.push_str(&s1);
            result.push_str(&fix_string(s2.clone()));
            Ok(ExprResult::String(result.into()))
        },
        (ExprResult::String(s1), ExprResult::List(l)) => {
            // Pre-estimate capacity
            let capacity = s1.len() + 2 + l.len() * 10; // rough estimate
            let mut result = String::with_capacity(capacity);
            result.push_str(&s1);
            result.push('[');

            for (i, e) in l.iter().enumerate() {
                if i != 0 {
                    result.push(',');
                }
                // Directly append without intermediate String allocation
                match string_push(result.into(), e.clone())? {
                    ExprResult::String(s) => result = s.to_string(),
                    _ => unreachable!()
                }
            }

            result.push(']');
            Ok(ExprResult::String(result.into()))
        },

        (ExprResult::String(s1), ExprResult::Float(s2)) => {
            let s2_str = s2.into_inner().to_string();
            let mut result = String::with_capacity(s1.len() + s2_str.len());
            result.push_str(&s1);
            result.push_str(&s2_str);
            Ok(ExprResult::String(result.into()))
        },

        (ExprResult::String(s1), ExprResult::Value(s2)) => {
            let s2_str = s2.to_string();
            let mut result = String::with_capacity(s1.len() + s2_str.len());
            result.push_str(&s1);
            result.push_str(&s2_str);
            Ok(ExprResult::String(result.into()))
        },

        (ExprResult::String(s1), ExprResult::Bool(s2)) => {
            let s2_str = s2.to_string();
            let mut result = String::with_capacity(s1.len() + s2_str.len());
            result.push_str(&s1);
            result.push_str(&s2_str);
            Ok(ExprResult::String(result.into()))
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("string_push called with wrong types {s1:?} {s2:?}")
                })
            )))
        },
    }
}

use std::path::Path;

use cpclib_tokens::{ExprResult};

pub fn load_binary<P: AsRef<std::path::Path>>(fname: P) -> std::io::Result<ExprResult> {
	let mut read = std::fs::read(fname)?;
    Ok(ExprResult::from(read.as_slice()))
}
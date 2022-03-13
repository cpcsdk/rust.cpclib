use std::fmt::Debug;

use cpclib_tokens::ExprResult;

use crate::error::AssemblerError;
use crate::preamble::ParserContext;

pub fn load_binary<P: AsRef<std::path::Path>>(
    fname: P,
    ctx: &ParserContext
) -> Result<Vec<u8>, AssemblerError> {
    let fname = fname.as_ref();
    let fname_repr = fname.to_string_lossy();
    match ctx.get_path_for(fname) {
        Err(_e) => {
            return Err(AssemblerError::IOError {
                msg: format!("{} not found", fname_repr)
            });
        }
        Ok(ref fname) => {
            let read = std::fs::read(fname).map_err(|e| {
                AssemblerError::IOError {
                    msg: format!("Unable to read {}.", fname_repr)
                }
            })?;
            Ok(read)
        }
    }
}

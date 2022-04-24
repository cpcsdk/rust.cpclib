use std::path::{Path, PathBuf};

use super::Env;
use crate::error::AssemblerError;
use crate::preamble::ParserContext;

pub fn get_filename(
    fname: &str,
    ctx: &ParserContext,
    env: Option<&Env>
) -> Result<PathBuf, AssemblerError> {
    ctx.get_path_for(fname, env).map_err(|e| {
        match e {
            either::Either::Left(asm) => asm,
            either::Either::Right(tested) => {
                AssemblerError::AssemblingError {
                    msg: format!("{} not found. TEsted {:?}", fname, tested)
                }
            }
        }
    })
}

/// Load a file
/// - if path is provided, this is the file name used
/// - if a string is provided, there is a search of appropriate filename
pub fn load_binary(
    fname: either::Either<&Path, &str>,
    ctx: &ParserContext,
    env: &Env
) -> Result<Vec<u8>, AssemblerError> {
    // Retreive fname
    let fname = match &fname {
        either::Either::Right(p) => get_filename(p, ctx, Some(env))?,
        either::Either::Left(p) => p.into()
    };

    // Load data
    std::fs::read(fname.clone()).map_err(|_e| {
        AssemblerError::IOError {
            msg: format!("Unable to read {}.", fname.to_string_lossy().to_string())
        }
    })
}

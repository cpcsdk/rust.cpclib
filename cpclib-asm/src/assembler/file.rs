use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use either::Either;

use super::embedded::EmbeddedFiles;
use super::Env;
use crate::error::AssemblerError;
use crate::preamble::ParserContext;
use crate::progress::Progress;

type Fname<'a, 'b> = either::Either<&'a Path, (&'a str, &'b Env)>;

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
pub fn load_binary(fname: Fname, ctx: &ParserContext) -> Result<Vec<u8>, AssemblerError> {
    // Retreive fname
    let fname = match &fname {
        either::Either::Right((p, env)) => get_filename(p, ctx, Some(env))?,
        either::Either::Left(p) => p.into()
    };

    let fname_repr = fname.to_str().unwrap();

    let progress = if ctx.show_progress {
        Progress::progress().add_load(fname_repr);
        Some(fname_repr)
    }
    else {
        None
    };

    let content = if fname_repr.starts_with("inner://") {
        // handle inner file
        EmbeddedFiles::get(fname_repr)
            .ok_or(AssemblerError::IOError {
                msg: format!("Unable to open {:?}; it is not embedded.", fname_repr)
            })?
            .data
            .to_vec()
    }
    else {
        // handle real file
        let mut f = File::open(&fname).map_err(|e| {
            AssemblerError::IOError {
                msg: format!("Unable to open {:?}. {}", fname, e)
            }
        })?;

        let mut content = Vec::new();
        f.read_to_end(&mut content).map_err(|e| {
            AssemblerError::IOError {
                msg: format!("Unable to read {:?}. {}", fname, e.to_string())
            }
        })?;
        content
    };

    if let Some(progress) = progress {
        Progress::progress().remove_load(progress);
    }
    Ok(content)
}

/// Read the content of the source file.
/// Uses the context to obtain the appropriate file other the included directories
pub fn read_source<P: AsRef<Path>>(
    fname: P,
    ctx: &ParserContext
) -> Result<String, AssemblerError> {
    let fname = fname.as_ref();

    let content = load_binary(Either::Left(fname), ctx)?;
    handle_source_encoding(fname.to_str().unwrap(), &content)
}

// Never fail
pub fn handle_source_encoding(fname: &str, content: &[u8]) -> Result<String, AssemblerError> {
    let mut decoder = chardetng::EncodingDetector::new();
    decoder.feed(content, true);
    let encoding = decoder.guess(None, true);
    let content = encoding.decode(content).0;

    let content = content.into_owned();

    Ok(content)
}

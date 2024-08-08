use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_disc::amsdos::AmsdosHeader;
use either::Either;

use super::embedded::EmbeddedFiles;
use super::Env;
use crate::error::AssemblerError;
use crate::preamble::ParserOptions;
use crate::progress::Progress;

type Fname<'a, 'b> = either::Either<&'a Utf8Path, (&'a str, &'b Env)>;

pub fn get_filename(
    fname: &str,
    options: &ParserOptions,
    env: Option<&Env>
) -> Result<Utf8PathBuf, AssemblerError> {
    options.get_path_for(fname, env).map_err(|e| {
        match e {
            either::Either::Left(asm) => asm,
            either::Either::Right(tested) => {
                AssemblerError::AssemblingError {
                    msg: format!("{} not found. TEsted {:?}", fname, tested)
                }
            },
        }
    })
}

/// Load a file and remove header if any
/// - if path is provided, this is the file name used
/// - if a string is provided, there is a search of appropriate filename
pub fn load_binary(
    fname: Fname,
    options: &ParserOptions
) -> Result<(VecDeque<u8>, Option<AmsdosHeader>), AssemblerError> {
    // Get the file content
    let data = load_binary_raw(fname, options)?;
    let mut data = VecDeque::from(data);

    // get a slice on the data to ease its cut
    let header = if data.len() >= 128 {
        // by construction there is only one slice
        let header = AmsdosHeader::from_buffer(data.as_slices().0);

        if header.represent_a_valid_file() {
            data.drain(..128);
            Some(header)
        }
        else {
            None
        }
    }
    else {
        None
    };

    Ok((data, header))
}

/// Load a file and keep the header if any
pub fn load_binary_raw(fname: Fname, options: &ParserOptions) -> Result<Vec<u8>, AssemblerError> {
    // Retreive fname
    let fname = match &fname {
        either::Either::Right((p, env)) => get_filename(p, options, Some(env))?,
        either::Either::Left(p) => p.into()
    };

    let fname_repr = fname.as_str();

    let progress = if options.show_progress {
        Progress::progress().add_load(fname_repr);
        Some(fname_repr)
    }
    else {
        None
    };

    // Get the content from the inner files or the disc
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
                msg: format!("Unable to read {:?}. {}", fname, e)
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
pub fn read_source<P: AsRef<Utf8Path>>(
    fname: P,
    options: &ParserOptions
) -> Result<String, AssemblerError> {
    let fname = fname.as_ref();

    let (mut content, header_removed) = load_binary(Either::Left(fname), options)?;
    assert!(header_removed.is_none());

    let content = content.make_contiguous();
    // handle_source_encoding(fname.to_str().unwrap(), &content)

    Ok(String::from_utf8_lossy(content).into_owned())
}

// Never fail
#[cfg(all(feature = "chardetng", not(target_arch = "wasm32")))]
pub fn handle_source_encoding(_fname: &str, content: &[u8]) -> Result<String, AssemblerError> {
    let mut decoder = chardetng::EncodingDetector::new();
    decoder.feed(content, true);
    let encoding = decoder.guess(None, true);
    let content = encoding.decode(content).0;

    let content = content.into_owned();

    Ok(content)
}

#[cfg(any(not(feature = "chardetng"), target_arch = "wasm32"))]
pub fn handle_source_encoding(_fname: &str, _content: &[u8]) -> Result<String, AssemblerError> {
    unimplemented!(
        "i have deactivated this stuff to speed up everything. Let's consider each source is UTF8!"
    )
}

use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use cpclib_disc::amsdos::{AmsdosFile, AmsdosFileName, AmsdosHeader, AmsdosManagerNonMut};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::{ExtendedDsk, Head};
use either::Either;

use super::embedded::EmbeddedFiles;
use super::Env;
use crate::error::AssemblerError;
use crate::preamble::ParserOptions;
use crate::progress::Progress;

type Fname<'a, 'b> = either::Either<&'a Utf8Path, (&'a str, &'b Env)>;

const DSK_SEPARATOR: char = '#';

pub fn get_filename<S: AsRef<str>>(
    fname: S,
    options: &ParserOptions,
    env: Option<&Env>
) -> Result<Utf8PathBuf, AssemblerError> {
    let fname = fname.as_ref();

    let components = fname.split(DSK_SEPARATOR).collect_vec();
    let real_fname = components.first().unwrap();

    let res = options.get_path_for(real_fname, env).map_err(|e| {
        match e {
            either::Either::Left(asm) => asm,
            either::Either::Right(tested) => {
                AssemblerError::AssemblingError {
                    msg: format!("{} not found. Tested {:?}", fname, tested)
                }
            },
        }
    })?;

    let res = if components.len() == 2 {
        let mut s = res.to_string();
        s.push(DSK_SEPARATOR);
        s.push_str(components.last().unwrap());
        Utf8PathBuf::from(&s)
    }
    else {
        res
    };

    Ok(res)
}

/// Load a file and remove header if any
/// - if path is provided, this is the file name used
/// - if a string is provided, there is a search of appropriate filename
pub fn load_binary(
    fname: Fname,
    options: &ParserOptions
) -> Result<(VecDeque<u8>, Option<AmsdosHeader>), AssemblerError> {
    let true_fname = match &fname {
        either::Either::Right((p, env)) => get_filename(p, options, Some(env))?,
        either::Either::Left(p) => p.into()
    };

    let mut parts = true_fname.as_str().split(DSK_SEPARATOR).rev().collect_vec();
    let (data, header) = if parts.len() == 1 {
        // here we handle a standard file

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

        (data, header)
    }
    else {
        // here we read from a dsk
        let pc_fname = parts.pop().unwrap();
        let amsdos_fname = parts.pop().unwrap();

        let disc: Box<ExtendedDsk> /* we cannot use Disc ATM */ = if pc_fname.to_ascii_uppercase().ends_with(".DSK") {
            Box::new(ExtendedDsk::open(pc_fname).map_err(|e| AssemblerError::AssemblingError { msg: e })?)

        } else {
            unimplemented!("Need to code loading of {pc_fname}. Disc trait needs to be simplifed by removing all generic parameters :(");
        };

        let manager = AmsdosManagerNonMut::new_from_disc(&disc, Head::A);
        let file = manager
            .get_file(AmsdosFileName::try_from(amsdos_fname)?)
            .ok_or_else(|| {
                AssemblerError::AssemblingError {
                    msg: format!("Unable to get {amsdos_fname}")
                }
            })?;

        let header = file.header();
        let data = VecDeque::from_iter(file.content().iter().cloned());

        (data, header)
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

use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use cpclib_disc::amsdos::{AmsdosFileName, AmsdosHeader, AmsdosManagerNonMut};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::{ExtendedDsk, Head};
use either::Either;

use super::embedded::EmbeddedFiles;
use super::Env;
use crate::error::AssemblerError;
use crate::preamble::ParserOptions;
use crate::progress::Progress;

pub struct Fname<'a, 'b>(Either<&'a Utf8Path, (&'a str, &'b Env)>);

impl<'a, 'b> Deref for Fname<'a, 'b> {
    type Target = Either<&'a Utf8Path, (&'a str, &'b Env)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a Utf8Path> for Fname<'a, '_> {
    fn from(value: &'a Utf8Path) -> Self {
        Self(Either::Left(value))
    }
}


impl<'a> From<&'a str> for Fname<'a, '_> {
    fn from(value: &'a str) -> Self {
        let p: &Utf8Path = value.into();
        p.into()
    }
}

impl<'a, 'b> From<(&'a str, &'b Env)> for Fname<'a, 'b> {
    fn from(value: (&'a str, &'b Env)) -> Self {
        Self(Either::Right(value))
    }
}

pub enum AnyFileNameOwned {
    InImage { image: String, content: String },
    Standard(String)
}

impl From<&AnyFileName<'_>> for AnyFileNameOwned {
    fn from(value: &AnyFileName) -> Self {
        match value {
            AnyFileName::InImage { image, content } => Self::new_in_image(*image, *content),
            AnyFileName::Standard(content) => Self::new_standard(*content)
        }
    }
}

impl<'fname> From<&'fname AnyFileNameOwned> for AnyFileName<'fname> {
    fn from(value: &'fname AnyFileNameOwned) -> Self {
        match value {
            AnyFileNameOwned::InImage {
                image,
                content: amsdos
            } => {
                AnyFileName::InImage {
                    image: image.as_str(),
                    content: amsdos.as_str()
                }
            },
            AnyFileNameOwned::Standard(fname) => AnyFileName::Standard(fname.as_str())
        }
    }
}

impl From<&str> for AnyFileNameOwned {
    fn from(value: &str) -> Self {
        let any = AnyFileName::from(value);
        AnyFileNameOwned::from(&any)
    }
}

impl AnyFileNameOwned {
    pub fn new_standard<S: Into<String>>(fname: S) -> Self {
        Self::Standard(fname.into())
    }

    pub fn new_in_image<S1: Into<String>, S2: Into<String>>(image: S1, amsdos: S2) -> Self {
        Self::InImage {
            image: image.into(),
            content: amsdos.into()
        }
    }

    pub fn as_any_filename(&self) -> AnyFileName {
        AnyFileName::from(self)
    }
}

/// Helper to handler filenames that contains both a dsk name and a file
pub enum AnyFileName<'fname> {
    InImage {
        image: &'fname str,
        content: &'fname str
    },
    Standard(&'fname str)
}

impl<'fname> AnyFileName<'fname> {
    const DSK_SEPARATOR: char = '#';

    pub fn new_standard(fname: &'fname str) -> Self {
        Self::Standard(fname)
    }

    pub fn new_in_image(image: &'fname str, amsdos: &'fname str) -> Self {
        Self::InImage {
            image,
            content: amsdos
        }
    }

    pub fn use_image(&self) -> bool {
        match self {
            AnyFileName::InImage { .. } => true,
            _ => false
        }
    }

    pub fn image_filename(&self) -> Option<&str> {
        match self {
            AnyFileName::InImage { image, .. } => Some(image),
            AnyFileName::Standard(_) => None
        }
    }

    pub fn content_filename(&self) -> &str {
        match self {
            AnyFileName::InImage {
                image,
                content: amsdos
            } => amsdos,
            AnyFileName::Standard(content) => content
        }
    }

    pub fn basm_fname(&self) -> Cow<str> {
        match self {
            AnyFileName::InImage { image, content } => {
                Cow::Owned(format!("{}{}{}", image, Self::DSK_SEPARATOR, content))
            },
            AnyFileName::Standard(content) => Cow::Borrowed(content)
        }
    }

    fn base_filename(&self) -> &str {
        match self {
            AnyFileName::InImage {
                image,
                content: amsdos
            } => image,
            AnyFileName::Standard(f) => f
        }
    }

    pub fn path_for_base_filename(
        &self,
        options: &ParserOptions,
        env: Option<&Env>
    ) -> Result<Utf8PathBuf, AssemblerError> {
        let real_fname = self.base_filename();

        let res = options.get_path_for(real_fname, env).map_err(|e| {
            match e {
                either::Either::Left(asm) => asm,
                either::Either::Right(tested) => {
                    AssemblerError::AssemblingError {
                        msg: format!("{} not found. Tested {:?}", self.base_filename(), tested)
                    }
                },
            }
        })?;

        let res = if self.image_filename().is_some() {
            let mut s = res.to_string();
            s.push(Self::DSK_SEPARATOR);
            s.push_str(self.content_filename());
            Utf8PathBuf::from(&s)
        }
        else {
            res
        };

        Ok(res)
    }
}

impl<'fname> From<&'fname str> for AnyFileName<'fname> {
    fn from(fname: &'fname str) -> Self {
        const IMAGES_EXT: &[&str] = &[".dsk", ".edsk", ".hfe"];

        let components = fname.split(Self::DSK_SEPARATOR).collect_vec();
        match components[..] {
            [fname] => AnyFileName::Standard(fname),
            [first, second] => {
                let is_image = IMAGES_EXT
                    .iter()
                    .any(|ext| first.to_ascii_lowercase().ends_with(ext));
                if is_image {
                    AnyFileName::InImage {
                        image: first,
                        content: second
                    }
                }
                else {
                    AnyFileName::Standard(fname)
                }
            },
            _ => {
                todo!(
                    "Need to handle case where fname as several {}",
                    Self::DSK_SEPARATOR
                )
            }
        }
    }
}

pub fn get_filename_to_read<S: AsRef<str>>(
    fname: S,
    options: &ParserOptions,
    env: Option<&Env>
) -> Result<Utf8PathBuf, AssemblerError> {
    let fname = fname.as_ref();

    AnyFileName::from(fname).path_for_base_filename(options, env)
}

/// TODO refactor and move that from asm stuff. Should be done only in the disc crate
/// Load a file and remove header if any
/// - if path is provided, this is the file name used
/// - if a string is provided, there is a search of appropriate filename
pub fn load_file<'a, 'b, F: Into<Fname<'a, 'b>>>(
    fname: F,
    options: &ParserOptions
) -> Result<(VecDeque<u8>, Option<AmsdosHeader>), AssemblerError> {
    let fname = fname.into();
    let true_fname = match &fname.deref() {
        either::Either::Right((p, env)) => get_filename_to_read(p, options, Some(env))?,
        either::Either::Left(p) => p.into()
    };

    let true_name = true_fname.as_str();
    let any_filename: AnyFileName<'_> = true_name.into();
    let (data, header) = if !any_filename.use_image() {
        // here we handle a standard file

        // Get the file content
        let data = load_file_raw(any_filename.content_filename(), options)?;
        let mut data = VecDeque::from(data);

        // get a slice on the data to ease its cut
        let header = if data.len() >= 128 {
            // by construction there is only one slice
            let header = AmsdosHeader::from_buffer(data.as_slices().0);

            // XXX previously, I was checking the file name validity, but it is a
            //     bad heursitic as orgams does not respect that
            if (header.file_length() + 128) as usize == data.len() {
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
        let image_fname = any_filename.image_filename().unwrap();
        let amsdos_fname = any_filename.content_filename();

        let disc: Box<ExtendedDsk> /* we cannot use Disc ATM */ = if image_fname.to_ascii_uppercase().ends_with(".DSK") {
            Box::new(ExtendedDsk::open(image_fname).map_err(|e| AssemblerError::AssemblingError { msg: e })?)

        } else {
            unimplemented!("Need to code loading of {image_fname}. Disc trait needs to be simplifed by removing all generic parameters :(");
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
pub fn load_file_raw<'a, 'b, F: Into<Fname<'a, 'b>>>(
    fname: F,
    options: &ParserOptions
) -> Result<Vec<u8>, AssemblerError> {
    let fname = fname.into();

    // Retreive fname
    let fname = match &fname.deref() {
        either::Either::Right((p, env)) => get_filename_to_read(p, options, Some(env))?,
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

    let (mut content, header_removed) = load_file(fname, options)?;
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

// TODO add file saving functions and factorize code from other places

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use cpclib_disc::amsdos::{AmsdosAddBehavior, AmsdosError, AmsdosFile, AmsdosFileName};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;
use cpclib_disc::open_disc;
use either::Either;

pub type AmsdosOrRaw<'d> = Either<AmsdosFile, &'d [u8]>;

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub enum FileType {
    AmsdosBin,
    AmsdosBas,
    Ascii,
    NoHeader,
    Auto
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageSupport {
    Disc(Utf8PathBuf),
    Tape(Utf8PathBuf),
    Host
}

impl StorageSupport {
    pub fn in_disc(&self) -> bool {
        matches!(self, Self::Disc(_))
    }

    pub fn in_tape(&self) -> bool {
        matches!(self, Self::Tape(_))
    }

    pub fn in_host(&self) -> bool {
        matches!(self, Self::Host)
    }

    pub fn container_filename(&self) -> Option<&Utf8Path> {
        match self {
            StorageSupport::Disc(d) => Some(d.as_path()),
            StorageSupport::Tape(t) => Some(t.as_path()),
            StorageSupport::Host => None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileAndSupport {
    support: StorageSupport,
    file: (FileType, Utf8PathBuf)
}

impl FileAndSupport {
    delegate::delegate! {
        to self.support {
            pub fn in_disc(&self) -> bool;
            pub fn in_tape(&self) -> bool;
            pub fn in_host(&self) -> bool;
            pub fn container_filename(&self) -> Option<&Utf8Path>;
        }
    }

    pub fn new(support: StorageSupport, file: (FileType, Utf8PathBuf)) -> Self {
        Self { support, file }
    }

    pub fn new_amsdos<P: Into<Utf8PathBuf>>(p: P) -> Self {
        Self {
            support: StorageSupport::Host,
            file: (FileType::AmsdosBin, p.into())
        }
    }

    pub fn new_amsdos_in_disc<P: Into<Utf8PathBuf>, F: Into<Utf8PathBuf>>(p: P, f: F) -> Self {
        Self {
            support: StorageSupport::Disc(p.into()),
            file: (FileType::AmsdosBin, f.into())
        }
    }

    pub fn new_basic<P: Into<Utf8PathBuf>>(p: P) -> Self {
        Self {
            support: StorageSupport::Host,
            file: (FileType::AmsdosBas, p.into())
        }
    }

    pub fn new_basic_in_disc<P: Into<Utf8PathBuf>, F: Into<Utf8PathBuf>>(p: P, f: F) -> Self {
        Self {
            support: StorageSupport::Disc(p.into()),
            file: (FileType::AmsdosBas, f.into())
        }
    }

    pub fn new_ascii<P: Into<Utf8PathBuf>>(p: P) -> Self {
        Self {
            support: StorageSupport::Host,
            file: (FileType::Ascii, p.into())
        }
    }

    pub fn new_ascii_in_disc<P: Into<Utf8PathBuf>, F: Into<Utf8PathBuf>>(p: P, f: F) -> Self {
        Self {
            support: StorageSupport::Disc(p.into()),
            file: (FileType::Ascii, f.into())
        }
    }

    pub fn new_no_header<P: Into<Utf8PathBuf>>(p: P) -> Self {
        Self {
            support: StorageSupport::Host,
            file: (FileType::NoHeader, p.into())
        }
    }

    pub fn new_auto<P: Into<Utf8PathBuf>>(p: P, header: bool) -> Self {
        let fname = p.into();

        const IMAGES_EXT: &[&str] = &[".dsk", ".edsk", ".hfe"];

        let components = fname.as_str().split('#').collect_vec();
        match components[..] {
            [fname] => {
                if header {
                    Self::new_amsdos(fname)
                }
                else {
                    Self::new_no_header(fname)
                }
            },
            [first, second] => {
                let is_image = IMAGES_EXT
                    .iter()
                    .any(|ext| first.to_ascii_lowercase().ends_with(ext));
                if is_image {
                    Self {
                        support: StorageSupport::Disc(first.into()),
                        file: (FileType::Auto, second.into())
                    }
                }
                else if header {
                    Self::new_amsdos(fname)
                }
                else {
                    Self::new_no_header(fname)
                }
            },
            _ => {
                todo!("Need to handle case where fname as several #",)
            }
        }
    }

    pub fn filename(&self) -> Utf8PathBuf {
        match &self.support {
            StorageSupport::Disc(p) => Utf8PathBuf::from(format!("{}#{}", p, self.file.1)),
            StorageSupport::Tape(utf8_path_buf) => todo!(),
            StorageSupport::Host => Utf8PathBuf::from(format!("{}", &self.file.1))
        }
    }

    pub fn amsdos_filename(&self) -> &Utf8Path {
        &self.file.1
    }

    fn build_amsdos_bin_file(
        &self,
        data: &[u8],
        loading_address: Option<u16>,
        exec_address: Option<u16>
    ) -> Result<AmsdosFile, AmsdosError> {
        let size = data.len();
        if size > 0x10000 {
            return Err(AmsdosError::FileLargerThan64Kb);
        }
        let size = size as u16;

        let loading_address = loading_address.unwrap_or(0);
        let execution_address = exec_address
            .map(|e| {
                if e < loading_address + size {
                    e
                }
                else {
                    loading_address
                }
            })
            .unwrap_or(loading_address);

        AmsdosFile::binary_file_from_buffer(
            &AmsdosFileName::try_from(self.amsdos_filename().as_str())?,
            loading_address,
            execution_address,
            data
        )
    }

    fn build_amsdos_bas_file(&self, data: &[u8]) -> Result<AmsdosFile, AmsdosError> {
        AmsdosFile::basic_file_from_buffer(
            &AmsdosFileName::try_from(self.amsdos_filename().as_str())?,
            data
        )
    }

    fn build_ascii_file(&self, data: &[u8]) -> Result<AmsdosFile, AmsdosError> {
        match AmsdosFileName::try_from(self.amsdos_filename().as_str()) {
            Ok(amsfname) => {
                Ok(AmsdosFile::ascii_file_from_buffer_with_name(
                    &amsfname, data
                ))
            },
            Err(e) => {
                if self.in_disc() {
                    Err(e)?;
                }
                Ok(AmsdosFile::from_buffer(data))
            }
        }
    }

    pub fn build_file<'d>(
        &self,
        data: &'d [u8],
        loading_address: Option<u16>,
        exec_address: Option<u16>
    ) -> Result<AmsdosOrRaw<'d>, AmsdosError> {
        match self.resolve_file_type() {
            FileType::AmsdosBin => {
                self.build_amsdos_bin_file(data, loading_address, exec_address)
                    .map(Either::Left)
            },
            FileType::AmsdosBas => self.build_amsdos_bas_file(data).map(Either::Left),
            FileType::Ascii => self.build_ascii_file(data).map(Either::Left),
            FileType::NoHeader => Ok(Either::Right(data)),
            FileType::Auto => unreachable!()
        }
    }

    pub fn save<D: AsRef<[u8]>>(
        &self,
        data: D,
        loading_address: Option<u16>,
        exec_address: Option<u16>,
        add_behavior: Option<AmsdosAddBehavior>
    ) -> Result<(), String> {
        let data = data.as_ref();

        let built_file = self
            .build_file(data, loading_address, exec_address)
            .map_err(|e| e.to_string())?;

        match &self.support {
            StorageSupport::Disc(disc_filename) => {
                let mut disc =
                    open_disc(disc_filename, false).map_err(|msg| format!("Disc error: {msg}"))?;

                let head = Head::A;
                let system = false;
                let read_only = false;

                let amsdos_file = built_file.unwrap_left();
                disc.add_amsdos_file(
                    &amsdos_file,
                    head,
                    read_only,
                    system,
                    add_behavior.unwrap_or(AmsdosAddBehavior::FailIfPresent)
                )
                .map_err(|e| e.to_string())?;

                disc.save(disc_filename)
                    .map_err(|e| format!("Error while saving {e}"))?;
            },
            StorageSupport::Tape(utf8_path_buf) => unimplemented!(),
            StorageSupport::Host => {
                // handle case with and without header
                let (fname, content) = match &built_file {
                    Either::Left(amsdos_file) => {
                        if self.resolve_file_type() == FileType::Ascii {
                            (self.filename().into(), amsdos_file.header_and_content())
                        }
                        else {
                            let fname = amsdos_file
                                .amsdos_filename()
                                .unwrap()
                                .unwrap()
                                .ibm_filename();
                            (fname, amsdos_file.header_and_content())
                        }
                    },
                    Either::Right(buffer) => (self.filename().into(), *buffer)
                };

                std::fs::write(&fname, content)
                    .map_err(|e| format!("Error while saving \"{fname}\". {e}"))?;
            }
        }

        Ok(())
    }

    /// Ensure the file is not auto
    pub fn resolve_file_type(&self) -> FileType {
        match &self.file.0 {
            FileType::Auto => {
                let lower = self.amsdos_filename().as_str().to_lowercase();
                if lower.ends_with(".bas") {
                    FileType::AmsdosBas
                }
                else if lower.ends_with(".asc") {
                    FileType::Ascii
                }
                else {
                    FileType::AmsdosBin
                }
            },
            other => *other
        }
    }
}

use std::convert::TryFrom;
use std::fs::File;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_disc::amsdos::{AmsdosFile, AmsdosFileName};
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;
#[cfg(feature = "hfe")]
use cpclib_disc::hfe::Hfe;
use cpclib_disc::open_disc;
use cpclib_tokens::SaveType;

use super::report::SavedFile;
use super::Env;
use crate::error::AssemblerError;
use crate::progress::{self, Progress};


#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub enum FileType {
    AmsdosBin,
    AmsdosBas,
    Ascii,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SaveFile {
    pub(crate) support: StorageSupport,
    pub(crate) file: (FileType, Utf8PathBuf)
}


impl SaveFile {
    pub fn filename(&self) -> Utf8PathBuf {
        match &self.support {
            StorageSupport::Disc(p) => Utf8PathBuf::from(format!("{}#{}", p, self.file.1)),
            StorageSupport::Tape(utf8_path_buf) => todo!(),
            StorageSupport::Host => Utf8PathBuf::from(format!("{}", &self.file.1)),
        }
    }
    delegate::delegate! {
        to self.support {
            pub fn in_disc(&self) -> bool;
            pub fn in_tape(&self) -> bool;
            pub fn in_host(&self) -> bool;
        }
    }
}

/// Save command information
/// RMR is already properly set up when executing the instruction
#[derive(Debug, Clone)]
pub struct SaveCommand {
    from: Option<i32>,
    size: Option<i32>,
    file: SaveFile,
    ga_mmr: u8
}

impl SaveCommand {
    pub fn new(
        from: Option<i32>,
        size: Option<i32>,
        file:SaveFile,
        ga_mmr: u8
    ) -> Self {
        SaveCommand {
            from,
            size,
            file,
            ga_mmr
        }
    }

    pub fn ga_mmr(&self) -> u8 {
        self.ga_mmr
    }

    pub fn can_be_saved_in_parallel(&self) -> bool {
        self.file.in_host()
    }

    pub fn amsdos_filename(&self) -> &Utf8Path {
        &self.file.file.1
    }

    pub fn file_type(&self) -> &FileType {
        &self.file.file.0
    }

    pub fn container_filename(&self) -> Option<&Utf8Path> {
        match &self.file.support {
            StorageSupport::Disc(d) => Some(d.as_path()),
            StorageSupport::Tape(t) => Some(t.as_path()),
            StorageSupport::Host => None,
        }
    }

    /// Really make the save - Prerequisit : the page is properly selected
    /// Do not yet handle the ascii format
    pub fn execute_on(&self, env: &Env) -> Result<SavedFile, AssemblerError> {
        assert_eq!(env.ga_mmr, self.ga_mmr);
        if env.options().show_progress() {
            Progress::progress().add_save(progress::normalize(&self.file.filename()));
        }

        let from = match self.from {
            Some(from) => from,
            None => env.start_address().unwrap() as _
        };

        let size = match self.size {
            Some(size) => size,
            None => {
                let stop = env.maximum_address();
                (stop - from as u16) as _
            }
        };

        let data = env.get_memory(from as _, size as _);

        // ensure we have no more AUTO file type
        let file_type = match self.file_type() {
            FileType::Auto => {
                let lower = self.amsdos_filename().as_str().to_lowercase();
                if lower.ends_with(".bas") {
                    FileType::AmsdosBas
                } else if lower.ends_with(".asc") {
                    FileType::Ascii
                } else {
                    FileType::AmsdosBin
                }
            },
            other => other.clone()
        };

        // Add the header if any
        enum AmsdosOrRaw {
            Raw(Vec<u8>),
            Amsdos(AmsdosFile)
        }

        // Generate the file
        let file_content : AmsdosOrRaw = match &file_type {
            FileType::AmsdosBin => {
                let loading_address = from as u16;
                let execution_address = match env.run_options {
                    Some((exec_address, _)) if exec_address < loading_address + size as u16 => {
                        exec_address
                    },
                    _ => loading_address
                };
                let f = AmsdosFile::binary_file_from_buffer(
                    &AmsdosFileName::try_from(self.amsdos_filename().as_str())?,
                    loading_address,
                    execution_address,
                    &data
                )?;
                AmsdosOrRaw::Amsdos(f)
            },
            FileType::AmsdosBas => {
                let f = AmsdosFile::basic_file_from_buffer(
                    &AmsdosFileName::try_from(self.amsdos_filename().as_str())?,
                    &data
                )?;
                AmsdosOrRaw::Amsdos(f)
            },
            FileType::Ascii => AmsdosOrRaw::Raw(data.clone()),
            FileType::Auto => unreachable!(),
        };

        // save the file
        match &self.file.support {
            StorageSupport::Disc(disc_filename) => {
                let mut disc = open_disc(disc_filename, false).map_err(|msg| {
                    AssemblerError::AlreadyRenderedError(format!("Disc error: {}", msg))
                })?;

                let head = Head::A;
                let system = false;
                let read_only = false;

                match file_content {
                    AmsdosOrRaw::Raw(vec) => unimplemented!(),
                    AmsdosOrRaw::Amsdos(amsdos_file) => {
                        disc.add_amsdos_file(
                            &amsdos_file,
                            head,
                            read_only,
                            system,
                            env.options().assemble_options().save_behavior()
                        )?;
                    },
                };
                
                disc.save(disc_filename).map_err(|e| {
                    AssemblerError::AssemblingError {
                        msg: format!("Error while saving {e}")
                    }
                })?;
            },
            StorageSupport::Tape(utf8_path_buf) => unimplemented!(),
            StorageSupport::Host => {
                let content = match &file_content {
                    AmsdosOrRaw::Raw(content) => &content,
                    AmsdosOrRaw::Amsdos(amsdos_file) => amsdos_file.header_and_content(),
                };
                std::fs::write(self.amsdos_filename(), &data).map_err(|e| {
                    AssemblerError::AssemblingError {
                        msg: format!("Error while saving \"{}\". {}", &self.amsdos_filename(), e)
                    }
                })?;
            },
        }

 

        if env.options().show_progress() {
            Progress::progress().remove_save(progress::normalize(&self.amsdos_filename()));
        }

        Ok(SavedFile {
            name: self.amsdos_filename().to_owned(),
            size: size as _
        })
    }
}

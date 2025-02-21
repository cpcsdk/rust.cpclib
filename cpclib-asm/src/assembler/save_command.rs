use cpclib_disc::disc::Disc;
#[cfg(feature = "hfe")]
use cpclib_disc::hfe::Hfe;
use cpclib_files::*;

use super::report::SavedFile;
use super::Env;
use crate::error::AssemblerError;
use crate::progress::{self, Progress};

pub type SaveFile = FileAndSupport;

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
    pub fn new(from: Option<i32>, size: Option<i32>, file: SaveFile, ga_mmr: u8) -> Self {
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
                if env.start_address().is_some() {
                    let stop = env.maximum_address();
                    (stop - from as u16) as i32 + 1
                }
                else {
                    0
                }
            },
        };

        // get the data from the CPC memory
        let data = env.get_memory(from as _, size as _);

        // Generate and save the file

        dbg!(&self.file)
            .save(
                data,
                Some(from as u16),
                match env.run_options {
                    Some((exec_address, _)) if exec_address < from as u16 + size as u16 => {
                        Some(exec_address)
                    },
                    _ => None
                },
                Some(env.options().assemble_options().save_behavior())
            )
            .map_err(|e| {
                AssemblerError::AssemblingError {
                    msg: format!("Error while saving. {e}")
                }
            })?;

        if env.options().show_progress() {
            Progress::progress().remove_save(progress::normalize(&self.file.filename()));
        }

        Ok(SavedFile {
            name: self.file.filename().to_owned(),
            size: size as _
        })
    }
}

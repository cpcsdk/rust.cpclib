use std::convert::TryFrom;

use cpclib_disc::{
    amsdos::{AmsdosFile, AmsdosFileName},
    edsk::ExtendedDsk,
};
use cpclib_tokens::SaveType;

use crate::error::AssemblerError;

use super::{report::SavedFile, Env};

/// Save command information
#[derive(Debug, Clone)]
pub struct SaveCommand {
    from: Option<i32>,
    size: Option<i32>,
    filename: String,
    save_type: Option<SaveType>,
    dsk_filename: Option<String>,
}

impl SaveCommand {
    pub fn new(
        from: Option<i32>,
        size: Option<i32>,
        filename: String,
        save_type: Option<SaveType>,
        dsk_filename: Option<String>,
    ) -> Self {
        SaveCommand {
            from,
            size,
            filename,
            save_type,
            dsk_filename,
        }
    }

    /// Really make the save - Prerequisit : the page is properly selected
    pub fn execute_on(&self, env: &Env) -> Result<SavedFile, AssemblerError> {
        let from = match self.from {
            Some(from) => from,
            None => env.start_address().unwrap() as _,
        };

        let size = match self.size {
            Some(size) => size,
            None => {
                let stop = env.maximum_address();
                (stop - from as u16) as _
            }
        };

        let data = env.memory(from as _, size as _);

        // Add the header if any
        let object: either::Either<Vec<u8>, AmsdosFile> = match self.save_type {
            Some(r#type) => {
                let loading_address = from as u16;
                let execution_address = match env.run_options {
                    Some((exec_address, _)) => exec_address,
                    None => loading_address,
                };

                let amsdos_file = if r#type == SaveType::AmsdosBas {
                    AmsdosFile::basic_file_from_buffer(
                        &AmsdosFileName::try_from(self.filename.as_str())?,
                        &data,
                    )?
                } else {
                    AmsdosFile::binary_file_from_buffer(
                        &AmsdosFileName::try_from(self.filename.as_str())?,
                        loading_address,
                        execution_address,
                        &data,
                    )?
                };

                match r#type {
                    SaveType::AmsdosBin | SaveType::AmsdosBas => {
                        either::Left(amsdos_file.full_content().copied().collect::<Vec<u8>>())
                    }
                    SaveType::Dsk | SaveType::Tape => either::Right(amsdos_file),
                }
            }
            None => either::Left(data),
        };

        // Save at the right place
        match object {
            either::Right(amsdos_file) => {
                if let Some(dsk_filename) = &self.dsk_filename {
                    let mut dsk = if std::path::Path::new(dsk_filename.as_str()).exists() {
                        ExtendedDsk::open(dsk_filename)?
                    } else {
                        ExtendedDsk::default()
                    };

                    dsk.add_amsdos_file(&amsdos_file)?;

                    dsk.save(dsk_filename)?;
                } else {
                    return Err(AssemblerError::InvalidArgument {
                        msg: "DSK parameter not provided".to_owned(),
                    });
                }
            }
            either::Left(data) => {
                std::fs::write(&self.filename, &data).map_err(|e| {
                    AssemblerError::AssemblingError {
                        msg: format!(
                            "Error while saving \"{}\". {}",
                            &self.filename,
                            e.to_string()
                        ),
                    }
                })?;
            }
        }

        Ok(SavedFile {
            name: self.filename.clone(),
            size: size as _,
        })
    }
}

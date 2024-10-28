use cpclib_common::camino::Utf8Path;

use super::{ExternRunner, Runner};
use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const RASM_CMD: &str = "rasm";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExternAssembler {
    Rasm(RasmVersion)
}

impl ExternAssembler {
    pub fn get_command(&self) -> &str {
        match self {
            ExternAssembler::Rasm(_) => RASM_CMD
        }
    }

    pub fn configuration<E: EventObserver + 'static>(&self) -> DelegateApplicationDescription<E> {
        match self {
            ExternAssembler::Rasm(r) => r.configuration()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RasmVersion {
    Consolidation2024 // V2_2_5
}

impl Default for RasmVersion {
    fn default() -> Self {
        Self::Consolidation2024
    }
}

// Here we need to regularly look at rasm release file. because files often disapppear
cfg_match! {
    cfg(target_os = "linux") =>
    {
        impl RasmVersion {
            pub fn configuration<E:EventObserver +'static>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    RasmVersion::Consolidation2024  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/EdouardBERGE/rasm/archive/refs/tags/v2.2.9.zip", // we assume a modern CPU
                            folder : "rasm_consolidation",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "rasm",
                            compile: Some(Box::new(|path: &Utf8Path, o: &E| -> Result<(), String>{
                                let command = vec!["make"];
                                ExternRunner::default().inner_run(&command, o)?;

                                let command = vec!["mv", "rasm.exe", "rasm"];
                                ExternRunner::default().inner_run(&command, o)?;

                                Ok(())
                            }))
                        }
                    }
            }
        }

    }
    cfg(target_os = "windows") =>
    {
        impl RasmVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    RasmVersion::Consolidation2024  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/EdouardBERGE/rasm/releases/download/v2.2.9/rasm_w64.exe", // we assume a modern CPU
                            folder : "rasm_consolidation",
                            archive_format: ArchiveFormat::Raw,
                            exec_fname: "rasm_w64.exe",
                            compile: None
                        }
                    }
            }
        }

    }
    cfg(target_os = "macos") =>
    {

    }
    _ => {
    }
}

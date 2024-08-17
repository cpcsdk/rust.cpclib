use cpclib_common::camino::Utf8Path;

use super::{ExternRunner, Runner};
use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};

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

    pub fn configuration(&self) -> DelegateApplicationDescription {
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

cfg_match! {
    cfg(target_os = "linux") =>
    {
        impl RasmVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    RasmVersion::Consolidation2024  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/EdouardBERGE/rasm/archive/refs/tags/v2.2.5.zip", // we assume a modern CPU
                            folder : "rasm_consolidation",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "rasm",
                            compile: Some(Box::new(|path: &Utf8Path| -> Result<(), String>{
                                let command = vec!["make"];
                                ExternRunner::default().inner_run(&command)?;

                                let command = vec!["mv", "rasm.exe", "rasm"];
                                ExternRunner::default().inner_run(&command)?;

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
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    RasmVersion::Consolidation2024  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/EdouardBERGE/rasm/releases/download/v2.2.5/rasm_win64.exe", // we assume a modern CPU
                            folder : "rasm_consolidation",
                            archive_format: ArchiveFormat::Raw,
                            exec_fname: "rasm.exe",
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

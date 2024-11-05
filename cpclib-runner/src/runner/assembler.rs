
use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

#[cfg(target_os = "linux")]
use crate::runner::ExternRunner;
#[cfg(target_os = "linux")]
use crate::runner::runner::Runner;
#[cfg(target_os = "linux")]
use cpclib_common::camino::Utf8Path;
#[cfg(target_os = "linux")]
use crate::delegated::Compiler;

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
                    RasmVersion::Consolidation2024  => {
                        let install : Box<dyn Fn(&Utf8Path, &E) -> Result<(), String>> = Box::new(|_path: &Utf8Path, o: &E| -> Result<(), String>{
                            let command = vec!["make"];
                            ExternRunner::default().inner_run(&command, o)?;

                            let command = vec!["mv", "rasm.exe", "rasm"];
                            ExternRunner::default().inner_run(&command, o)?;

                            Ok(())
                        });
                        let install = Compiler::from(install);
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/EdouardBERGE/rasm/archive/refs/tags/v2.2.9.zip") // we assume a modern CPU
                            .folder("rasm_consolidation")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("rasm")
                            .compile(install)
                            .build()
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
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/EdouardBERGE/rasm/releases/download/v2.2.9/rasm_w64.exe") // we assume a modern CPU
                            .folder("rasm_consolidation")
                            .archive_format(ArchiveFormat::Raw)
                            .exec_fname("rasm_w64.exe")
                            .build()
                        
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

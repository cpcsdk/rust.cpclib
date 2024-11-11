use std::fmt::Display;

use crate::delegated::{ArchiveFormat, CompilableInformation, DownloadableInformation, ExecutableInformation, GithubCompilableApplication, GithubInformation};

pub const RASM_CMD: &str = "rasm";



#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RasmVersion {
    Consolidation2024 // V2_2_5
}

impl GithubCompilableApplication for RasmVersion {

}

impl Default for RasmVersion {
    fn default() -> Self {
        Self::Consolidation2024
    }
}


impl Display for RasmVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rasm {}", self.version_name())
    }
}


impl GithubInformation for RasmVersion {
    fn version_name(&self) -> &'static str {
        match self {
            Self::Consolidation2024 => "Consolidation"
        }
    }
    
    fn project(&self) -> &'static str {
        "rasm"
    }
    
    fn owner(&self) -> &'static str {
        "EdouardBERGE"
    }
    
    fn linux_key(&self) -> Option<&'static str> {
        Some("Source code (zip)")
    }
    
    fn windows_key(&self) -> Option<&'static str> {
        Some("rasm_x64.exe")
    }
    
}

impl ExecutableInformation for RasmVersion {
 
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::Consolidation2024 => "rasm_consolidation"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os="windows")]
        return "rasm_w64.exe";
        #[cfg(target_os="macos")]
        unimplemented!();
        #[cfg(target_os="linux")]
        return "rasm";

    }
}

impl CompilableInformation for RasmVersion {
    fn target_os_commands(&self) -> Option<&'static[&'static[&'static str]]> {
        if cfg!(target_os = "linux") {
            Some(&[
                &["make"],
                &["mv", "rasm.exe", "rasm"]
            ])
            } else {
                None
            }
    }
}

impl DownloadableInformation for RasmVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }
}


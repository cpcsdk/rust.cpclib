use std::fmt::Display;

use crate::delegated::{
    ArchiveFormat, CompilableInformation, DownloadableInformation, ExecutableInformation,
    GithubCompilableApplication, GithubInformation
};

pub const HSPC_CMD: &str = "hspcompiler";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum HspCompilerVersion {
    #[default]
    V_2_0 // V2_2_5
}

impl HspCompilerVersion {
    pub fn get_command(&self) -> &str {
        HSPC_CMD
    }
}
impl GithubCompilableApplication for HspCompilerVersion {}

impl Display for HspCompilerVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rasm {}", self.version_name())
    }
}

impl GithubInformation for HspCompilerVersion {
    fn version_name(&self) -> &'static str {
        match self {
            Self::V_2_0 => "v2.0"
        }
    }

    fn project(&self) -> &'static str {
        "hspcompiler"
    }

    fn owner(&self) -> &'static str {
        "EdouardBERGE"
    }

    fn linux_key(&self) -> Option<&'static str> {
        Some("Source code (zip)")
    }

    fn windows_key(&self) -> Option<&'static str> {
        Some("compiler.exe")
    }
}

impl ExecutableInformation for HspCompilerVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V_2_0 => "hspcompiler_2_0"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "compiler.exe";
        #[cfg(target_os = "macos")]
        unimplemented!();
        #[cfg(target_os = "linux")]
        return "hspcompiler";
    }
}

impl CompilableInformation for HspCompilerVersion {
    fn target_os_commands(&self) -> Option<&'static [&'static [&'static str]]> {
        if cfg!(target_os = "linux") {
            Some(&[&["gcc", "compiler.c", "-O2", "-o", "hspcompiler"]])
        }
        else {
            None
        }
    }
}

impl DownloadableInformation for HspCompilerVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "linux")]
        return ArchiveFormat::Zip;
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Raw;
    }
}

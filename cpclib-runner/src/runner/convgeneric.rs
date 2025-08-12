use std::fmt::Display;

use crate::delegated::{
    ArchiveFormat, CompilableInformation, DownloadableInformation, ExecutableInformation,
    GithubCompilableApplication, GithubInformation
};

pub const CONVGENERIC_CMD: &str = "convgeneric";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum ConvGenericVersion {
    #[default]
    GroSSeWindowBug // V2_2
}

impl ConvGenericVersion {
    pub fn get_command(&self) -> &str {
        CONVGENERIC_CMD
    }
}

impl GithubCompilableApplication for ConvGenericVersion {}

impl Display for ConvGenericVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rasm {}", self.version_name())
    }
}

impl GithubInformation for ConvGenericVersion {
    fn version_name(&self) -> &'static str {
        match self {
            Self::GroSSeWindowBug => "GroSSe Windows Bug!"
        }
    }

    fn project(&self) -> &'static str {
        "convgeneric"
    }

    fn owner(&self) -> &'static str {
        "EdouardBERGE"
    }

    fn linux_key(&self) -> Option<&'static str> {
        Some("Source code (zip)")
    }

    fn windows_key(&self) -> Option<&'static str> {
        Some("convgeneric.exe")
    }
}

impl ExecutableInformation for ConvGenericVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::GroSSeWindowBug => "convgeneric_grosswindowbug"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "convgeneric.exe";
        #[cfg(not(target_os = "windows"))]
        return "convgeneric";
    }
}

impl CompilableInformation for ConvGenericVersion {
    fn target_os_commands(&self) -> Option<&'static [&'static [&'static str]]> {
        if cfg!(target_os = "linux") {
            Some(&[&[
                "gcc",
                "convgeneric.c",
                "-o",
                "convgeneric",
                "-lm",
                "-lpng16"
            ]])
        }
        else {
            None
        }
    }
}

impl DownloadableInformation for ConvGenericVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Raw;

        #[cfg(not(target_os = "windows"))]
        return ArchiveFormat::Zip;
    }
}

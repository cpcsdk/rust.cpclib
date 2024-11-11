use std::fmt::Display;

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation, GithubCompiledApplication,
    GithubInformation
};
use crate::runner::runner::RunInDir;

pub const SUGARBOX_V2_CMD: &str = "sugarbox";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum SugarBoxV2Version {
    #[default]
    V2_0_2
}

impl Display for SugarBoxV2Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sugarbox {}", self.version_name())
    }
}

impl DownloadableInformation for SugarBoxV2Version {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "windows")]
        return ArchiveFormat::SevenZ;
        #[cfg(target_os = "macos")]
        return ArchiveFormat::TarGz;
        #[cfg(target_os = "linux")]
        return ArchiveFormat::TarGz;
    }
}

impl ExecutableInformation for SugarBoxV2Version {
    fn target_os_folder(&self) -> &'static str {
        match self {
            SugarBoxV2Version::V2_0_2 => {
                #[cfg(target_os = "windows")]
                return "Sugarbox-2.0.2-win64/Sugarbox-2.0.2-win64";
                #[cfg(target_os = "macos")]
                return "Sugarbox-2.0.2-Darwin";
                #[cfg(target_os = "linux")]
                return "Sugarbox-2.0.2-Linux";
            }
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "Sugarbox.exe";
        #[cfg(target_os = "macos")]
        return "Sugarbox";
        #[cfg(target_os = "linux")]
        return "Sugarbox-2.0.2-Linux/Sugarbox";
    }

    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl GithubInformation for SugarBoxV2Version {
    fn project(&self) -> &'static str {
        "SugarboxV2"
    }

    fn owner(&self) -> &'static str {
        "Tom1975"
    }

    fn version_name(&self) -> &'static str {
        match self {
            SugarBoxV2Version::V2_0_2 => "v2.0.2"
        }
    }

    fn linux_key(&self) -> Option<&'static str> {
        Some("Sugarbox-2.0.2-Linux.tar.gz")
    }

    fn windows_key(&self) -> Option<&'static str> {
        Some("Sugarbox-2.0.2-win64.7z")
    }

    fn macos_key(&self) -> Option<&'static str> {
        Some("Sugarbox-2.0.2-Darwin.tar.gz")
    }
}

impl GithubCompiledApplication for SugarBoxV2Version {}

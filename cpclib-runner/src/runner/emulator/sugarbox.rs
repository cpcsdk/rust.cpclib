use std::fmt::Display;

// To compile:
// git clone https://github.com/Tom1975/SugarboxV2.git
// cd SugarboxV2
// cmake .
// make -j20
use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation, GithubCompiledApplication,
    GithubInformation
};
use crate::runner::runner::RunInDir;

pub const SUGARBOX_V2_CMD: &str = "sugarbox";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum SugarBoxV2Version {
    #[default]
    V2_0_3,
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
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        return ArchiveFormat::TarGz;
    }
}

impl ExecutableInformation for SugarBoxV2Version {
    fn target_os_folder(&self) -> &'static str {
        match self {
            SugarBoxV2Version::V2_0_3 => {
                #[cfg(target_os = "windows")]
                return "Sugarbox-2.0.3-win64/Sugarbox-2.0.3-win64";
                #[cfg(target_os = "macos")]
                return "Sugarbox-2.0.3-Darwin";
                #[cfg(target_os = "linux")]
                return "Sugarbox-2.0.3-Linux";
            },
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
        return match self {
            Self::V2_0_3 => "Sugarbox-2.0.3-Linux/Sugarbox",
            Self::V2_0_2 => "Sugarbox-2.0.2-Linux/Sugarbox"
        };
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
            SugarBoxV2Version::V2_0_3 => "v2.0.3",
            SugarBoxV2Version::V2_0_2 => "v2.0.2"
        }
    }

    fn linux_key(&self) -> Option<&'static str> {
        match self {
            Self::V2_0_3 => Some("Sugarbox-2.0.3-Linux.tar.gz"),
            Self::V2_0_2 => Some("Sugarbox-2.0.2-Linux.tar.gz")
        }
    }

    fn windows_key(&self) -> Option<&'static str> {
        match self {
            Self::V2_0_3 => Some("Sugarbox-2.0.3-win64.7z"),
            Self::V2_0_2 => Some("Sugarbox-2.0.2-win64.7z")
        }
    }

    fn macos_key(&self) -> Option<&'static str> {
        match self {
            Self::V2_0_3 => Some("Sugarbox-2.0.3-Darwin.tar.gz"),
            Self::V2_0_2 => Some("Sugarbox-2.0.2-Darwin.tar.gz")
        }
    }
}

impl GithubCompiledApplication for SugarBoxV2Version {}

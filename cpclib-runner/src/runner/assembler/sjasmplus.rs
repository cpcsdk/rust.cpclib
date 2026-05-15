use std::fmt::Display;

use crate::delegated::{
    ArchiveFormat, CompilableInformation, DownloadableInformation, ExecutableInformation,
    GithubCompilableApplication, GithubInformation
};

pub const SJASMPLUS_CMD: &str = "sjasmplus";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum SjasmplusVersion {
    #[default]
    V1_20_3
}

impl GithubCompilableApplication for SjasmplusVersion {}

impl Display for SjasmplusVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sjasmplus {}", self.version_name())
    }
}

impl GithubInformation for SjasmplusVersion {
    fn version_name(&self) -> &'static str {
        match self {
            SjasmplusVersion::V1_20_3 => "v1.20.3"
        }
    }

    fn project(&self) -> &'static str {
        "sjasmplus"
    }

    fn owner(&self) -> &'static str {
        "z00m128"
    }

    fn linux_key(&self) -> Option<&'static str> {
        Some("sjasmplus-1.20.3-src.tar.xz")
    }

    fn windows_key(&self) -> Option<&'static str> {
        Some("sjasmplus-1.20.3.win.zip")
    }

    fn macos_key(&self) -> Option<&'static str> {
        Some("sjasmplus-1.20.3-src.tar.xz")
    }
}

impl ExecutableInformation for SjasmplusVersion {
    fn target_os_folder(&self) -> &'static str {
        "sjasmplus-1.20.3"
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(any(target_os = "linux", target_os = "haiku"))]
        return "sjasmplus";
        #[cfg(target_os = "windows")]
        return "sjasmplus.exe";
        #[cfg(target_os = "macos")]
        return "sjasmplus";
    }
}

impl CompilableInformation for SjasmplusVersion {
    fn target_os_commands(&self) -> Option<&'static [&'static [&'static str]]> {
        if cfg!(target_os = "linux") {
            Some(&[&["cmake", "sjasmplus-1.20.3"], &["make"]])
        }
        else if cfg!(target_os = "macos") {
            Some(&[
                &["make", "-C", "sjasmplus-1.20.3"],
                &["cp", "sjasmplus-1.20.3/sjasmplus", "."]
            ])
        }
        else {
            None
        }
    }
}

impl DownloadableInformation for SjasmplusVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(any(target_os = "linux", target_os = "haiku"))]
        return ArchiveFormat::TarXz;
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Zip;
        #[cfg(target_os = "macos")]
        return ArchiveFormat::TarXz;
    }
}

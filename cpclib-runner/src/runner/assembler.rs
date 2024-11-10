use std::default;
use std::fmt::Display;

#[cfg(target_os = "linux")]
use cpclib_common::camino::Utf8Path;

#[cfg(target_os = "linux")]
use crate::delegated::Compiler;
use crate::delegated::{github_download_urls, ArchiveFormat, CompilableInformation, DelegateApplicationDescription, DownloadableInformation, ExecutableInformation, GithubInformation, HasConfiguration, MutiplatformUrls};
use crate::event::EventObserver;
#[cfg(target_os = "linux")]
use crate::runner::runner::Runner;
#[cfg(target_os = "linux")]
use crate::runner::ExternRunner;

pub const RASM_CMD: &str = "rasm";
pub const SJASMPLUS_CMD: &str = "sjasmplus";

fn rasm_download_urls(version: RasmVersion) -> Result<MutiplatformUrls, String> {
    github_download_urls(&version)
}

fn sjasm_download_urls(version: SjasmplusVersion) -> Result<MutiplatformUrls, String> {
    github_download_urls(&version)
}


#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExternAssembler {
    Rasm(RasmVersion),
    Sjasmplus(SjasmplusVersion),
}

impl ExternAssembler {
    pub fn get_command(&self) -> &str {
        match self {
            ExternAssembler::Rasm(_) => RASM_CMD,
            ExternAssembler::Sjasmplus(_) => SJASMPLUS_CMD,
        }
    }

    pub fn configuration<E: EventObserver +'static>(&self) -> DelegateApplicationDescription<E> {
        match self {
            ExternAssembler::Rasm(r) => r.configuration(),
            ExternAssembler::Sjasmplus(r) => r.configuration(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum SjasmplusVersion {
    #[default]
    V1_20_3
}


impl Display for SjasmplusVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sjasmplus {}", self.version_name())
    }
}



impl GithubInformation for SjasmplusVersion {
    fn version_name(&self) -> &'static str {
        match self {
            SjasmplusVersion::V1_20_3 => "v1.20.3",
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
}

impl ExecutableInformation for SjasmplusVersion {
    fn folder(&self) -> &'static str {
        "sjasmplus-1.20.3"

    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "linux")]
        return "sjasmplus";
        #[cfg(target_os = "windows")]
        return "sjasmplus.exe"
    }
}


impl CompilableInformation for SjasmplusVersion {
    fn target_os_commands(&self) -> Option<&'static[&'static[&'static str]]> {
        if cfg!(target_os = "linux") {
            Some(&[
                &["cmake", "sjasmplus-1.20.3"],
                &["make"]
            ])
            } else {
                None
            }
    }
}


impl DownloadableInformation for SjasmplusVersion {
    fn archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "linux")]
        return ArchiveFormat::TarXz;
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Zip;
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
 
    fn folder(&self) -> &'static str {
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
    fn archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }
}

#[cfg(test)]
mod test {
    use super::{rasm_download_urls, RasmVersion};
    use crate::delegated::cpclib_download;


    #[test]
    fn test_download_rasm() {
        let urls= rasm_download_urls(RasmVersion::Consolidation2024).unwrap();

        assert!(cpclib_download(dbg!(&urls.linux.unwrap())).is_ok());
        assert!(cpclib_download(dbg!(&urls.windows.unwrap())).is_ok());
    }
}

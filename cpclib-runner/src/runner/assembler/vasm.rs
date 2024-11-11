
use std::sync::OnceLock;

use crate::delegated::{ArchiveFormat, DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation};

pub const VASM_CMD: &str = "vasm";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum VasmVersion {
	#[default]
    V2021_03_19 
}


impl InternetStaticCompiledApplication for VasmVersion {
	
}

impl DownloadableInformation for VasmVersion {
	fn target_os_archive_format(&self) -> ArchiveFormat {
		match self {
			VasmVersion::V2021_03_19 => {
				#[cfg(target_os = "windows")]
				return ArchiveFormat::Zip;
				#[cfg(target_os = "macos")]
				unimplemented!();
				#[cfg(target_os = "linux")]
				return ArchiveFormat::Tar; // XXX yep extension is wrong
			}
		}
	}
}

impl StaticInformation for VasmVersion {
	fn static_download_urls(&self) -> &'static MutiplatformUrls {
		match self {
			VasmVersion::V2021_03_19 => {
				static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
				URL.get_or_init(|| MutiplatformUrls{
					linux: Some("http://www.ibaug.de/vbcc/vbcc_linux_x64.tar.gz".to_owned()),
					windows: Some("http://www.ibaug.de/vbcc/vbcc_win_x64.zip".to_owned()),
					macos: None
				})
			},
		}
		
	}
}


impl ExecutableInformation for VasmVersion {
 
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V2021_03_19 => "vasm20210319"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os="windows")]
        return "bin/vasmz80_oldstyle.exe";
        #[cfg(target_os="macos")]
        unimplemented!();
        #[cfg(target_os="linux")]
        return "vbcc/bin/vasmz80_oldstyle";

    }
}
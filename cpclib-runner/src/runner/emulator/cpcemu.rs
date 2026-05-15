use std::sync::OnceLock;

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation,
    InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation
};
use crate::runner::runner::RunInDir;

pub const CPCEMU_CMD: &str = "cpcemu";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CpcEmuVersion {
    #[default]
    V3_0_2
}

impl InternetStaticCompiledApplication for CpcEmuVersion {}

impl ExecutableInformation for CpcEmuVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            CpcEmuVersion::V3_0_2 => "cpcemu_3_0_2"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "cpcemu-3.0.2/cpcemu_portable.exe";

        #[cfg(target_os = "macos")]
        return "Contents/MacOS/CPCemuMacOS";

        #[cfg(target_os = "linux")]
        return "cpcemu";
    }

    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl StaticInformation for CpcEmuVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        static URLS: OnceLock<MutiplatformUrls> = OnceLock::new();

        URLS.get_or_init(|| {
            MutiplatformUrls::builder()
                .windows(
                    "https://cpc-emu.org/Release/2025-04-24/cpcemu_portable-win32-x86-3.0.2.zip"
                )
                .macos("https://cpc-emu.org/Release/2025-04-24/CPCemuMacOS-3.0.2.zip")
                .build()
        })
    }
}

impl DownloadableInformation for CpcEmuVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }
}

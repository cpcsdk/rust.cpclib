use std::sync::OnceLock;

use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{ArchiveFormat, DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation};

pub const WINAPE_CMD: &str = "winape";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum WinapeVersion {
    #[default]
    V2_0b2
}

impl WinapeVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();
        conf.cache_folder().join("ROM")
    }
}

impl InternetStaticCompiledApplication for WinapeVersion {

}

impl ExecutableInformation for WinapeVersion {
    fn target_os_folder(&self) -> &'static str {
        "winape_2_0b2"
    }

    fn target_os_exec_fname(&self) -> &'static str {
        "WinApe.exe"
    }
}

impl DownloadableInformation for WinapeVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }
}
impl StaticInformation for WinapeVersion {
    fn static_download_urls(&self) -> &'static crate::delegated::MutiplatformUrls {
        static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
        URL.get_or_init(||  MutiplatformUrls::unique_url("http://www.winape.net/download/WinAPE20B2.zip"))
    }
}


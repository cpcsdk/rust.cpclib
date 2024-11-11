use std::sync::OnceLock;

use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{ArchiveFormat, DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation};

pub const CPCEC_CMD: &str = "cpcec";



#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CpcecVersion {
    #[default]
    V20240505
}


impl InternetStaticCompiledApplication for CpcecVersion {

}

impl ExecutableInformation for CpcecVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            CpcecVersion::V20240505 => "cpcec20240505",
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        "CPCEC.EXE"
    }
}

impl StaticInformation for CpcecVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        static URL: OnceLock<MutiplatformUrls> = OnceLock::new();

        URL.get_or_init(|| MutiplatformUrls::unique_url(
            match self {
            CpcecVersion::V20240505 => "http://cngsoft.no-ip.org/cpcec-20240505.zip"
            })
        )
    }
}

impl DownloadableInformation for CpcecVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }
}

impl CpcecVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();
        conf.cache_folder()
    }
}
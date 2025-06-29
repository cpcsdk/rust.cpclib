use std::fmt::Display;
use std::sync::OnceLock;

use crate::delegated::{
    DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication,
    MutiplatformUrls, StaticInformation
};

pub const CHIPNSFX_CMD: &str = "chipnsfx";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum ChipnsfxVersion {
    #[default]
    V20241231
}

impl Display for ChipnsfxVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            ChipnsfxVersion::V20241231 => "20241231"
        };

        write!(f, "{v}")
    }
}

impl StaticInformation for ChipnsfxVersion {
    fn static_download_urls(&self) -> &'static crate::delegated::MutiplatformUrls {
        static URL: OnceLock<MutiplatformUrls> = OnceLock::new();

        URL.get_or_init(|| {
            MutiplatformUrls::builder()
                .linux("http://cngsoft.no-ip.org/chipnsfx-20241231.zip")
                .windows("http://cngsoft.no-ip.org/chipnsfx-20241231.zip")
                .build()
        })
    }
}

impl DownloadableInformation for ChipnsfxVersion {
    fn target_os_archive_format(&self) -> crate::delegated::ArchiveFormat {
        crate::delegated::ArchiveFormat::Zip
    }
}

impl ExecutableInformation for ChipnsfxVersion {
    fn target_os_folder(&self) -> &'static str {
        static FOLDER: OnceLock<String> = OnceLock::new();
        FOLDER.get_or_init(|| format!("chipnsfx_{self}")).as_str()
    }

    fn target_os_exec_fname(&self) -> &'static str {
        "CHIPNSFX.EXE"
    }
}

impl InternetStaticCompiledApplication for ChipnsfxVersion {}

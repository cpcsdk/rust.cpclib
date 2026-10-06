use std::fmt::Display;
use std::sync::OnceLock;

use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::event::EventObserver;

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

impl ChipnsfxVersion {
    /// The Z80 player source (`CHIPNSFX.I80`) shipped in the CHIPNSFX archive,
    /// next to the converter - only present once the tool has been downloaded
    /// (i.e. after a first `chipnsfx` run).
    pub fn player_path<E: EventObserver>(&self) -> Utf8PathBuf {
        self.configuration::<E>()
            .cache_folder()
            .join("CHIPNSFX.I80")
    }
}

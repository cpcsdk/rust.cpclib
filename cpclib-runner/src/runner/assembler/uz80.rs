use std::fmt::Display;
use std::sync::OnceLock;

use crate::delegated::{
    DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication,
    MutiplatformUrls, StaticInformation
};

pub const UZ80_CMD: &str = "uz80";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Uz80Version {
    #[default]
    V20240224
}

impl Display for Uz80Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            Uz80Version::V20240224 => "20240224"
        };

        write!(f, "{v}")
    }
}

impl StaticInformation for Uz80Version {
    fn static_download_urls(&self) -> &'static crate::delegated::MutiplatformUrls {
        static URL: OnceLock<MutiplatformUrls> = OnceLock::new();

        URL.get_or_init(|| {
		MutiplatformUrls::builder()
			.linux("http://cngsoft.no-ip.org/uz80-20240224.zip")
			.windows("http://cngsoft.no-ip.org/uz80-20240224.zip")
			.build()
		})
    }
}

impl DownloadableInformation for Uz80Version {
    fn target_os_archive_format(&self) -> crate::delegated::ArchiveFormat {
        crate::delegated::ArchiveFormat::Zip
    }
}

impl ExecutableInformation for Uz80Version {
    fn target_os_folder(&self) -> &'static str {
        static FOLDER: OnceLock<String> = OnceLock::new();
        FOLDER.get_or_init(|| format!("at3_{}", self)).as_str()
    }

    fn target_os_exec_fname(&self) -> &'static str {
        return "UZ80.EXE";
    }
}

impl InternetStaticCompiledApplication for Uz80Version {}

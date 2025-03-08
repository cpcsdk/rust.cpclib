use std::collections::BTreeMap;

use cpclib_common::camino::Utf8PathBuf;
use directories::BaseDirs;
use scraper::{Html, Selector};

use crate::delegated::{
    cpclib_download, ArchiveFormat, DownloadableInformation, DynamicUrlInformation,
    ExecutableInformation, InternetDynamicCompiledApplication, MutiplatformUrls
};

pub const ACE_CMD: &str = "ace";

const ACE_URL: &str = "http://www.roudoudou.com/ACE-DL";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum AceVersion {
    #[default]
    UnknownLastVersion, // directly parse the webpage
    Bnd4,      // 2024/10/26
    ZenSummer, // 2024/08/18
    WakePoint  // 2024/06/21
}

impl DownloadableInformation for AceVersion {
    fn target_os_archive_format(&self) -> crate::delegated::ArchiveFormat {
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Zip;
        #[cfg(target_os = "linux")]
        return ArchiveFormat::TarGz;
        #[cfg(target_os = "macos")]
        return ArchiveFormat::Zip;
    }
}
impl DynamicUrlInformation for AceVersion {
    fn dynamic_download_urls(&self) -> Result<MutiplatformUrls, String> {
        match self {
            Self::UnknownLastVersion => {
                let mut content = cpclib_download(ACE_URL)?;
                let mut html = String::new();
                content
                    .read_to_string(&mut html)
                    .map_err(|e| e.to_string())?;

                let document = Html::parse_document(&html);
                let selector = Selector::parse("#dl td a")
                    .map_err(|e| e.to_string())
                    .map_err(|e| e.to_string())?;

                let mut map = BTreeMap::new();
                for element in document.select(&selector) {
                    map.insert(element.inner_html(), element.attr("href").unwrap());
                }

                let macos = map
                    .get("All versions")
                    .map(|url| format!("{}/{}", ACE_URL, url));
                let windows = map
                    .get("x64 (64 bits)")
                    .map(|url| format!("{}/{}", ACE_URL, url));
                let linux = map
                    .get("Ubuntu 24.04 LTS (AVX2)")
                    .map(|url| format!("{}/{}", ACE_URL, url));

                Ok(MutiplatformUrls {
                    linux,
                    windows,
                    macos
                })
            },
            AceVersion::Bnd4 => {
                Ok(MutiplatformUrls {
                    linux: Some("http://www.roudoudou.com/ACE-DL/BZen.tar.gz".to_string()),
                    windows: Some("http://www.roudoudou.com/ACE-DL/W64bnd4.zip".to_string()),
                    macos: None
                })
            },
            AceVersion::ZenSummer => {
                Ok(MutiplatformUrls {
                    linux: Some(
                        "http://www.roudoudou.com/ACE-DL/LinuxZenSummer.tar.gz".to_string()
                    ),
                    windows: Some("http://www.roudoudou.com/ACE-DL/Win64Summer.zip".to_string()),
                    macos: None
                })
            },
            AceVersion::WakePoint => {
                Ok(MutiplatformUrls {
                    linux: Some("http://www.roudoudou.com/ACE-DL/BZen.tar.gz".to_string()),
                    windows: Some("http://www.roudoudou.com/ACE-DL/BWIN64.zip".to_string()),
                    macos: Some("http://www.roudoudou.com/ACE-DL/BMAC.zip".to_string())
                })
            },
        }
    }
}

impl ExecutableInformation for AceVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            AceVersion::UnknownLastVersion => "UnknownLastAceVersion",
            AceVersion::Bnd4 => "AceWakePoint",
            AceVersion::ZenSummer => "AceBnd4",
            AceVersion::WakePoint => "AceZenSummer"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "AceDL.exe";
        #[cfg(target_os = "linux")]
        return "AceDL";
    }
}

impl InternetDynamicCompiledApplication for AceVersion {}

impl AceVersion {
    pub fn config_file(&self) -> Utf8PathBuf {
        let p = match self {
            Self::ZenSummer | Self::Bnd4 | Self::UnknownLastVersion => {
                BaseDirs::new()
                    .unwrap()
                    .config_local_dir()
                    .join("ACE-DL_futuristics/config.cfg")
            },
            _ => unimplemented!()
        };

        Utf8PathBuf::from_path_buf(p).unwrap()
    }
}

impl AceVersion {
    pub fn screenshots_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();

        conf.cache_folder().join("export").join("screenshot")
    }

    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();

        conf.cache_folder().join("media").join("rom")
    }

    pub fn albireo_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();

        conf.cache_folder().join("media").join("albireo1")
    }
}

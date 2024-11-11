use std::collections::BTreeMap;

use cpclib_common::{camino::Utf8PathBuf, event::EventObserver};
use directories::BaseDirs;
use scraper::{Html, Selector};

use crate::delegated::{cpclib_download, DelegateApplicationDescription, UrlGenerator};

pub const ACE_CMD: &str = "ace";

const ACE_URL: &str = "http://www.roudoudou.com/ACE-DL";

pub(crate) fn ace_download_fn_urls_lin_win() -> Result<(String, String), String> {
    let mut content = cpclib_download(ACE_URL)?;
	let mut html = String::new();
	content.read_to_string(&mut html).map_err(|e| e.to_string())?;

    let document = Html::parse_document(&html);
    let selector = Selector::parse("#dl td a")
        .map_err(|e| e.to_string())
        .map_err(|e| e.to_string())?;

    let mut map = BTreeMap::new();
    for element in document.select(&selector) {
        map.insert(element.inner_html(), element.attr("href").unwrap());
    }

    let windows_url = format!("{}/{}", ACE_URL, map.get("x64 (64 bits)").unwrap());
    let linux_url = format!(
        "{}/{}",
        ACE_URL,
        map.get("Ubuntu 22.04 LTS (AVX2)").unwrap()
    );

    Ok((linux_url, windows_url))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum AceVersion {
    #[default]
    UnknownLastVersion, // directly parse the webpage
    Bnd4,      // 2024/10/26
    ZenSummer, // 2024/08/18
    WakePoint  // 2024/06/21
}

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


#[cfg(target_os = "linux")]
fn linux_ace_desc<E: EventObserver, F: Into<UrlGenerator>>(
    download_fn_url: F,
    folder: &'static str
) -> DelegateApplicationDescription<E> {
    use cpclib_common::event::EventObserver;

    use crate::delegated::{ArchiveFormat, DelegateApplicationDescription, UrlGenerator};

    DelegateApplicationDescription::builder()
        .download_fn_url(download_fn_url)
        .folder(folder)
        .archive_format(ArchiveFormat::TarGz)
        .exec_fname("AceDL")
        .build()
}

#[cfg(windows)]
fn windows_ace_desc<E: EventObserver, F: Into<UrlGenerator>>(
    download_fn_url: F,
    folder: &'static str
) -> DelegateApplicationDescription<E> {
    use crate::delegated::ArchiveFormat;

    DelegateApplicationDescription::builder()
        .download_fn_url(download_fn_url.into())
        .folder(folder)
        .archive_format(ArchiveFormat::Zip)
        .exec_fname("AceDL.exe")
        .build()
}


cfg_match! {
    cfg(target_os = "linux") =>
    {

        impl AceVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    AceVersion::UnknownLastVersion => linux_ace_desc(ace_download_fn_urls_lin_win().unwrap().0, "UnknwownLastAceVersion"),
                    AceVersion::WakePoint => linux_ace_desc("http://www.roudoudou.com/ACE-DL/BZen.tar.gz", "AceWakePoint"),
                    AceVersion::Bnd4 => linux_ace_desc("http://www.roudoudou.com/ACE-DL/LinuxZENbnd4.tar.gz", "AceBnd4"),
                    AceVersion::ZenSummer => linux_ace_desc("http://www.roudoudou.com/ACE-DL/LinuxZenSummer.tar.gz", "AceZenSummer")
                }
            }
        }
	}


    cfg(target_os = "windows") =>
    {
        impl AceVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    AceVersion::UnknownLastVersion => windows_ace_desc(ace_download_fn_urls_lin_win().unwrap().1.clone(), "UnknwownLastAceVersion"),
                    AceVersion::Bnd4 => windows_ace_desc(
                        "http://www.roudoudou.com/ACE-DL/W64bnd4.zip",
                        "AceBnd4"
                    ),
                    AceVersion::WakePoint => windows_ace_desc(
                        "http://www.roudoudou.com/ACE-DL/BWIN64.zip", // we assume a 64bits machine
                        "AceWakePoint"),
                    AceVersion::ZenSummer => windows_ace_desc(
                        "http://www.roudoudou.com/ACE-DL/Win64Summer.zip",
                        "AceZenSummer"
                    )
                }
            }
        }
    }
	
    cfg(target_os = "macos") =>
    {
        impl AceVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    AceVersion::WakePoint => DelegateApplicationDescription::builder()
                    .download_fn_url("http://www.roudoudou.com/ACE-DL/BMAC.zip")
                    .folder("TODO")
                    .archive_format(ArchiveFormat::Zip)
                    .exec_fname("TODO")
                    .build()
                }
            }
        }
    }
    _ => {
    }
}

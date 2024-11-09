use std::collections::BTreeMap;
use std::path::absolute;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::winnow::Parser;
use directories::BaseDirs;
use scraper::{Html, Selector};

use crate::delegated::{
    cpclib_download, ArchiveFormat, DelegateApplicationDescription, UrlGenerator
};
use crate::event::EventObserver;

pub const ACE_CMD: &str = "ace";
pub const WINAPE_CMD: &str = "winape";
pub const CPCEC_CMD: &str = "cpcec";
pub const AMSPIRIT_CMD: &str = "amspirit";

const ACE_URL: &str = "http://www.roudoudou.com/ACE-DL";

fn ace_download_fn_urls_lin_win() -> Result<(String, String), String> {
    let html = cpclib_download(ACE_URL)?;
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Emulator {
    Ace(AceVersion),
    Amspirit(AmspiritVersion),
    Cpcec(CpcecVersion),
    Winape(WinapeVersion)
}

impl Default for Emulator {
    fn default() -> Self {
        Emulator::Ace(AceVersion::default())
    }
}

impl Emulator {
    pub fn ace_version(&self) -> Option<&AceVersion> {
        match self {
            Emulator::Ace(v) => Some(v),
            _ => None
        }
    }

    pub fn is_ace(&self) -> bool {
        matches!(self, Emulator::Ace(_))
    }

    pub fn is_amspirit(&self) -> bool {
        matches!(self, Emulator::Amspirit(_))
    }

    pub fn is_cpcec(&self) -> bool {
        matches!(self, Emulator::Cpcec(_))
    }

    pub fn is_winape(&self) -> bool {
        matches!(self, Emulator::Winape(_))
    }

    pub fn get_command(&self) -> &str {
        match self {
            Emulator::Ace(_) => ACE_CMD,
            Emulator::Amspirit(_) => AMSPIRIT_CMD,
            Emulator::Cpcec(_) => CPCEC_CMD,
            Emulator::Winape(_) => WINAPE_CMD
        }
    }

    pub fn window_name_corresponds(&self, window_name: &str) -> bool {
        let window_name = window_name.trim();
        match self {
            Emulator::Ace(_) => window_name.starts_with("ACE-DL -"),
            Emulator::Cpcec(_) => window_name.starts_with("CPCEC "),
            Emulator::Winape(_) => window_name.starts_with("Windows Amstrad Plus"),
            Emulator::Amspirit(_) => window_name.starts_with("AMSpiriT")
        }
    }

    pub fn screenshots_folder(&self) -> Utf8PathBuf {
        match self {
            Emulator::Ace(v) => v.screenshots_folder(),
            _ => unimplemented!()
        }
    }

    pub fn roms_folder(&self) -> Utf8PathBuf {
        match self {
            Emulator::Ace(v) => v.roms_folder(),
            Emulator::Cpcec(v) => v.roms_folder(),
            Emulator::Winape(v) => v.roms_folder(),
            _ => unimplemented!()
        }
    }

    pub fn albireo_folder(&self) -> Utf8PathBuf {
        match self {
            Emulator::Ace(v) => v.albireo_folder(),
            _ => unimplemented!()
        }
    }

    /// Handle filename to make them work properly using wine
    pub fn wine_compatible_fname(&self, p: &Utf8Path) -> Result<Utf8PathBuf, String> {
        let abspath = absolute(p).map_err(|e| e.to_string())?;
        let abspath = Utf8PathBuf::from_path_buf(abspath).map_err(|e| "File error".to_owned())?;
        if cfg!(target_os = "windows") {
            Ok(abspath)
        }
        else {
            Ok(("Z:".to_owned() + abspath.as_str()).into())
        }
    }
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CpcecVersion {
    #[default]
    V20240505
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum WinapeVersion {
    #[default]
    V2_0b2
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum AmspiritVersion {
    #[default]
    Rc1_01
}

impl Emulator {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            Emulator::Ace(v) => v.configuration(),
            Emulator::Cpcec(v) => v.configuration(),
            Emulator::Winape(v) => v.configuration(),
            Emulator::Amspirit(v) => v.configuration()
        }
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

impl CpcecVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();
        conf.cache_folder()
    }
}

impl WinapeVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();
        conf.cache_folder().join("ROM")
    }
}

#[cfg(target_os = "linux")]
fn linux_ace_desc<E: EventObserver, F: Into<UrlGenerator>>(
    download_fn_url: F,
    folder: &'static str
) -> DelegateApplicationDescription<E> {
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
    DelegateApplicationDescription::builder()
        .download_fn_url(download_fn_url.into())
        .folder(folder)
        .archive_format(ArchiveFormat::Zip)
        .exec_fname("AceDL.exe")
        .build()
}

impl WinapeVersion {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            WinapeVersion::V2_0b2 => {
                DelegateApplicationDescription::builder()
                    .download_fn_url("http://www.winape.net/download/WinAPE20B2.zip")
                    .folder("winape_2_0b2")
                    .archive_format(ArchiveFormat::Zip)
                    .exec_fname("WinApe.exe")
                    .build()
            },
        }
    }
}

impl AmspiritVersion {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            Self::Rc1_01 => {
                let original_fname = "CPC_AMSpiriT_RC_v1.01_Win_x64/Amspirit v1.01_RC_x64.exe";
                static MODIFIED_FNAME: &'static str =
                    "CPC_AMSpiriT_RC_v1.01_Win_x64/Amspirit_v1.01_RC_x64.exe";
                assert!(!MODIFIED_FNAME.contains(" "));

                let owned_original = original_fname.to_owned();
                let post_install: Box<
                    dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
                > = Box::new(move |d: &DelegateApplicationDescription<E>| {
                    std::fs::rename(
                        d.cache_folder().join(&owned_original),
                        d.cache_folder().join(MODIFIED_FNAME.to_owned())
                    )
                    .map_err(|e| e.to_string())
                });

                DelegateApplicationDescription::builder()
                    .download_fn_url("https://www.amspirit.fr/content/files/2024/04/CPC_AMSpiriT_RC_v1.01_Win_x64.7z")
                    .folder("CPC_AMSpiriT_RC_v1.01_Win_x64")
                    .archive_format(ArchiveFormat::SevenZ)
                    .exec_fname(MODIFIED_FNAME)
                    .in_dir(super::runner::RunInDir::AppDir)
                    .post_install(post_install)
                    .build()
            }
        }
    }
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

        impl CpcecVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    CpcecVersion::V20240505 => {
                        DelegateApplicationDescription::builder()
                            .download_fn_url("http://cngsoft.no-ip.org/cpcec-20240505.zip")
                            .folder("cpcec20240505")
                            .archive_format( ArchiveFormat::Zip)
                            .exec_fname("CPCEC.EXE") // XXX there is a case issue I do not want to solve. so wine is used ...
                            .build()
                    },
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

        impl CpcecVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    CpcecVersion::V20240505 => {
                        DelegateApplicationDescription::builder()
                            .download_fn_url("http://cngsoft.no-ip.org/cpcec-20240505.zip")
                            .folder("cpcec20240505")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("CPCEC.EXE")
                            .build()
                    },
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

#[cfg(test)]
mod test {
    use super::ace_download_fn_urls_lin_win;
    use crate::delegated::cpclib_download;

    #[test]
    fn retreive_ace_urls() {
        ace_download_fn_urls_lin_win().unwrap();
    }

    #[test]
    fn test_download_ace() {
        let (lin, win) = ace_download_fn_urls_lin_win().unwrap();

        assert!(cpclib_download(dbg!(&lin)).is_ok());
        assert!(cpclib_download(dbg!(&win)).is_ok());
    }
}

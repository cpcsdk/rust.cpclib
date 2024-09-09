use cpclib_common::camino::Utf8PathBuf;
use directories::BaseDirs;

use crate::{delegated::{ArchiveFormat, DelegateApplicationDescription}, event::EventObserver};

pub const ACE_CMD: &str = "ace";
pub const WINAPE_CMD: &str = "winape";
pub const CPCEC_CMD: &str = "cpcec";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Emulator {
    Ace(AceVersion),
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
        match self {
            Emulator::Ace(_) => true,
            _ => false
        }
    }

    pub fn get_command(&self) -> &str {
        match self {
            Emulator::Ace(_) => ACE_CMD,
            Emulator::Cpcec(_) => CPCEC_CMD,
            Emulator::Winape(_) => WINAPE_CMD
        }
    }

    pub fn window_name_corresponds(&self, window_name: &str) -> bool {
        let window_name = window_name.trim();
        match self {
            Emulator::Ace(_) => window_name.starts_with("ACE-DL -"),
            Emulator::Cpcec(_) => window_name.starts_with("CPCEC "),
            Emulator::Winape(_) => window_name.starts_with("Windows Amstrad Plus")
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum AceVersion {
    #[default]
    ZenSummer, // 2024/08/18
    WakePoint // 2024/06/21
}

impl AceVersion {
    pub fn config_file(&self) -> Utf8PathBuf {
        let p = match self {
            Self::ZenSummer => {
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

impl Emulator {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            Emulator::Ace(version) => version.configuration(),
            Emulator::Cpcec(version) => version.configuration(),
            Emulator::Winape(version) => version.configuration()
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
const fn linux_ace_desc<E: EventObserver>(
    download_url: &'static str,
    folder: &'static str
) -> DelegateApplicationDescription<E> {
    DelegateApplicationDescription {
        download_url,
        folder,
        archive_format: ArchiveFormat::TarGz,
        exec_fname: "AceDL",
        compile: None
    }
}

#[cfg(windows)]
const fn windows_ace_desc(
    download_url: &'static str,
    folder: &'static str
) -> DelegateApplicationDescription {
    DelegateApplicationDescription {
        download_url,
        folder,
        archive_format: ArchiveFormat::Zip,
        exec_fname: "AceDL.exe",
        compile: None
    }
}

cfg_match! {
    cfg(target_os = "linux") =>
    {

        impl AceVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    AceVersion::WakePoint => linux_ace_desc("http://www.roudoudou.com/ACE-DL/BZen.tar.gz", "AceWakePoint"),
                    AceVersion::ZenSummer => linux_ace_desc("http://www.roudoudou.com/ACE-DL/LinuxZenSummer.tar.gz", "AceZenSummer")
                }
            }
        }

        impl CpcecVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    CpcecVersion::V20240505 => {
                        DelegateApplicationDescription {
                            download_url: "http://cngsoft.no-ip.org/cpcec-20240505.zip",
                            folder: "cpcec20240505",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "CPCEC.EXE", // XXX there is a case issue I do not want to solve. so wine is used ...
                            compile: None
                        }
                    },
                }
            }
        }


        impl WinapeVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    WinapeVersion::V2_0b2 => {
                        DelegateApplicationDescription {
                            download_url: "http://www.winape.net/download/WinAPE20B2.zip",
                            folder: "winape_2_0b2",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "WinApe.exe",
                            compile: None
                        }
                    },
                }
            }
        }

    }
    cfg(target_os = "windows") =>
    {
        impl AceVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
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
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    CpcecVersion::V20240505 => {
                        DelegateApplicationDescription {
                            download_url: "http://cngsoft.no-ip.org/cpcec-20240505.zip",
                            folder: "cpcec20240505",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "CPCEC.EXE",
                            compile: None
                        }
                    },
                }
            }
        }

        impl WinapeVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    WinapeVersion::V2_0b2 => {
                        DelegateApplicationDescription {
                            download_url: "http://www.winape.net/download/WinAPE20B2.zip",
                            folder: "winape_2_0b2",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "WinApe.exe",
                            compile: None
                        }
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
                    AceVersion::WakePoint => DelegateApplicationDescription{
                    download_url: "http://www.roudoudou.com/ACE-DL/BMAC.zip",
                    folder : "TODO",
                    archive_format: ArchiveFormat::Zip,
                    exec_fname: "TODO"
                }}
            }
        }
    }
    _ => {
    }
}

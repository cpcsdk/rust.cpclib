use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::task::{ACE_CMDS, CPCEC_CMDS, WINAPE_CMDS};

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
    pub fn get_command(&self) -> &str {
        match self {
            Emulator::Ace(_) => ACE_CMDS[0],
            Emulator::Cpcec(_) => CPCEC_CMDS[0],
            Emulator::Winape(_) => WINAPE_CMDS[0]
        }
    }

    pub fn window_name_corresponds(&self, window_name: &str) -> bool{
        let window_name = window_name.trim();
        dbg!(window_name);
        match self {
            Emulator::Ace(_) => {
                window_name.starts_with("ACE-DL -")
            },
            Emulator::Cpcec(_) =>  window_name.starts_with("CPCEC ")
            ,
            Emulator::Winape(_) => window_name.starts_with("Windows Amstrad Plus"),
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
    WakePoint // 2024/06/21
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CpcecVersion {
    #[default]
    v20240505
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum WinapeVersion {
    #[default]
    v2_0b2
}

impl Emulator {
    pub fn configuration(&self) -> DelegateApplicationDescription {
        match self {
            Emulator::Ace(version) => version.configuration(),
            Emulator::Cpcec(version) => version.configuration(),
            Emulator::Winape(version) => version.configuration()
        }
    }
}

impl AceVersion {
    pub fn screenshots_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration();
        let path = conf.cache_folder().join("export").join("screenshot");
        path

    }

    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration();
        let path = conf.cache_folder().join("media").join("rom");
        path

    }

    pub fn albireo_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration();
        let path = conf.cache_folder().join("media").join("albireo1");
        path

    }
}


impl CpcecVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration();
        conf.cache_folder()
    }
}

impl WinapeVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration();
        conf.cache_folder().join("ROM")
    }
}


cfg_match! {
    cfg(target_os = "linux") =>
    {

        impl AceVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    AceVersion::WakePoint =>
                        DelegateApplicationDescription {
                            download_url: "http://www.roudoudou.com/ACE-DL/BZen.tar.gz", // we assume a modern CPU
                            folder : "AceWakePoint",
                            archive_format: ArchiveFormat::TarGz,
                            exec_fname: "AceDL",
                            compile: None
                        }
                    }
            }
        }

        impl CpcecVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    CpcecVersion::v20240505 => {
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
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    WinapeVersion::v2_0b2 => {
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
                    AceVersion::WakePoint => DelegateApplicationDescription{
                    download_url: "http://www.roudoudou.com/ACE-DL/BWIN64.zip", // we assume a 64bits machine
                    folder : "AceWakePoint",
                    archive_format: ArchiveFormat::Zip,
                    exec_fname: "AceDL.exe",
                    compile: None
                }}
            }
        }

        impl CpcecVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    CpcecVersion::v20240505 => {
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
                    WinapeVersion::v2_0b2 => {
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

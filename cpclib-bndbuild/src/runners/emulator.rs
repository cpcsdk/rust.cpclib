

use std::default;

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
                            exec_fname: "CPCEC.EXE", // TODO see how to handle the fact it is windows file. Do we need to compile the linux version ?
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
                    exec_fname: "AceDL.exe"
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
                            exec_fname: "CPCEC.EXE"
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
                            exec_fname: "WinApe.exe"
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


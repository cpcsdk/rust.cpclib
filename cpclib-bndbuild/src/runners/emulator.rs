use std::io::{Cursor, Read};

use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::itertools::Itertools;
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use tar::Archive;
use ureq::Response;

use super::Runner;
use crate::runners::r#extern::ExternRunner;
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

pub enum ArchiveFormat {
    TarGz,
    Zip
}

pub struct EmulatorConfiguration {
    download_url: &'static str,
    folder: &'static str,
    exec_fname: &'static str,
    archive_format: ArchiveFormat
}

impl Emulator {
    pub fn configuration(&self) -> EmulatorConfiguration {
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
            pub fn configuration(&self) -> EmulatorConfiguration {
                match self {
                    AceVersion::WakePoint =>
                        EmulatorConfiguration {
                            download_url: "http://www.roudoudou.com/ACE-DL/BZen.tar.gz", // we assume a modern CPU
                            folder : "AceWakePoint",
                            archive_format: ArchiveFormat::TarGz,
                            exec_fname: "AceDL"
                        }
                    }
            }
        }

        impl CpcecVersion {
            pub fn configuration(&self) -> EmulatorConfiguration {
                match self {
                    CpcecVersion::v20240505 => {
                        EmulatorConfiguration {
                            download_url: "http://cngsoft.no-ip.org/cpcec-20240505.zip",
                            folder: "cpcec20240505",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "CPCEC.EXE" // TODO see how to handle the fact it is windows file. Do we need to compile the linux version ?
                        }
                    },
                }
            }
        }


        impl WinapeVersion {
            pub fn configuration(&self) -> EmulatorConfiguration {
                match self {
                    WinapeVersion::v2_0b2 => {
                        EmulatorConfiguration {
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
    cfg(target_os = "windows") =>
    {
        impl AceVersion {
            pub fn configuration(&self) -> EmulatorConfiguration {
                match self {
                    AceVersion::WakePoint => EmulatorConfiguration{
                    download_url: "http://www.roudoudou.com/ACE-DL/BWIN64.zip", // we assume a 64bits machine
                    folder : "AceWakePoint",
                    archive_format: ArchiveFormat::Zip,
                    exec_fname: "AceDL.exe"
                }}
            }
        }

        impl CpcecVersion {
            pub fn configuration(&self) -> EmulatorConfiguration {
                match self {
                    CpcecVersion::v20240505 => {
                        EmulatorConfiguration {
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
            pub fn configuration(&self) -> EmulatorConfiguration {
                match self {
                    WinapeVersion::v2_0b2 => {
                        EmulatorConfiguration {
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
            pub fn configuration(&self) -> EmulatorConfiguration {
                match self {
                    AceVersion::WakePoint => EmulatorConfiguration{
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

impl EmulatorConfiguration {
    pub fn is_cached(&self) -> bool {
        self.cache_folder().exists()
    }

    pub fn cache_folder(&self) -> Utf8PathBuf {
        let proj_dirs = ProjectDirs::from("net.cpcscene", "benediction", "bnd build").unwrap();
        let base_cache = proj_dirs.cache_dir();

        if !base_cache.exists() {
            std::fs::create_dir_all(base_cache);
        }

        base_cache.join(self.folder).try_into().unwrap()
    }

    pub fn exec_fname(&self) -> Utf8PathBuf {
        self.cache_folder().join(self.exec_fname)
    }

    pub fn install(&self) {
        // get the file
        let dest = self.cache_folder();

        let resp = self.download().unwrap();
        let mut input = resp.into_reader();

        // uncompress it
        match self.archive_format {
            ArchiveFormat::TarGz => {
                let gz = GzDecoder::new(input);
                let mut archive = Archive::new(gz);
                archive.unpack(dest).unwrap();
            },
            ArchiveFormat::Zip => {
                let mut buffer = Vec::new();
                input.read_to_end(&mut buffer).unwrap();
                dbg!(&dest);
                zip_extract::extract(Cursor::new(buffer), dest.as_std_path(), true).unwrap();
            }
        }
    }

    fn download(&self) -> Result<Response, ureq::Error> {
        ureq::get(self.download_url).call()
    }
}

pub struct EmulatorRunner {
    pub(crate) emu: Emulator
}

impl Runner for EmulatorRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let cfg = self.emu.configuration();

        // ensure the emulator exists
        if !cfg.is_cached() {
            println!("> Install emulator");
            cfg.install();
        }
        assert!(cfg.is_cached());

        // Build the command
        let mut command = Vec::with_capacity(1 + itr.len());
        let fname = cfg.exec_fname();

        #[cfg(target_os = "linux")]
        {
            if fname.as_str().to_lowercase().ends_with(".exe") {
                command.push("wine");
            }
        }

        command.push(fname.as_str());
        for arg in itr.iter() {
            command.push(arg.as_ref());
        }

        // Delegate it to the appropriate luncher
        ExternRunner::default().inner_run(&command)
    }

    fn get_command(&self) -> &str {
        match self.emu {
            Emulator::Ace(_) => ACE_CMDS[0],
            Emulator::Cpcec(_) => CPCEC_CMDS[0],
            Emulator::Winape(_) => WINAPE_CMDS[0]
        }
    }
}

use crate::{delegated::{ArchiveFormat, DelegateApplicationDescription}, task::IMPDISC_CMDS};

pub enum ImpDskVersion {
	V0_24
}

impl Default for ImpDskVersion {
	fn default() -> Self {
		ImpDskVersion::V0_24
	}
}

impl ImpDskVersion {
	pub fn get_command(&self) -> &str {
		IMPDISC_CMDS[0]
	}
}

cfg_match! {
    cfg(target_os = "linux") =>
    {
		impl ImpDskVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    ImpDskVersion::V0_24  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/jeromelesaux/dsk/releases/download/v0.24/dsk-0.24-linux-amd64.zip", // we assume a modern CPU
                            folder : "ImpDsk_0_24",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "binaries/dsk-linux-amd64",
                            compile: None
                        }
                    }
            }
		}
    }
    cfg(target_os = "windows") =>
    {
        impl ImpDskVersion {
            pub fn configuration(&self) -> DelegateApplicationDescription {
                match self {
                    ImpDskVersion::V0_24  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/jeromelesaux/dsk/releases/download/v0.24/dsk-0.24-windows-amd64.zip", 
                            folder : "ImpDsk_0_24",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "binaries/dsk-windows-amd64.exe",
                            compile: None
                        }
                    }
            }
        }

    }
    cfg(target_os = "macos") =>
    {

    }
    _ => {
    }
}
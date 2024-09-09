use crate::{delegated::{ArchiveFormat, DelegateApplicationDescription}, event::EventObserver};

pub const MARTINE_CMD: &str = "martine";

#[derive(Default)]
pub enum MartineVersion {
    #[default]
    V0_39
}

impl MartineVersion {
    pub fn get_command(&self) -> &str {
        MARTINE_CMD
    }
}

cfg_match! {
    cfg(target_os = "linux") =>
    {
        impl MartineVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    MartineVersion::V0_39  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/jeromelesaux/martine/releases/download/v0.39/martine-0.39-linux-amd64.zip", // we assume a modern CPU
                            folder : "martine_0_39",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "martine.linux",
                            compile: None
                        }
                    }
            }
        }
    }
    cfg(target_os = "windows") =>
    {
        impl MartineVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    MartineVersion::V0_39  =>
                        DelegateApplicationDescription {
                            download_url: "https://github.com/jeromelesaux/martine/releases/download/v0.39/martine-0.39-windows-amd64.zip",
                            folder : "martine_0_39",
                            archive_format: ArchiveFormat::Zip,
                            exec_fname: "martine.exe",
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

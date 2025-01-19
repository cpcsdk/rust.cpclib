use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const IMPDISC_CMD: &str = "impdsk";

#[derive(Default)]
pub enum ImpDskVersion {
    #[default]
    V0_24
}

impl ImpDskVersion {
    pub fn get_command(&self) -> &str {
        IMPDISC_CMD
    }
}

cfg_match! {
    target_os = "linux" =>
    {
        impl ImpDskVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    ImpDskVersion::V0_24  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.24/dsk-0.24-linux-amd64.zip") // we assume a modern CPU
                            .folder("ImpDsk_0_24")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("binaries/dsk-linux-amd64")
                            .build()
                    }
            }
        }
    }
    target_os = "windows" =>
    {
        impl ImpDskVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    ImpDskVersion::V0_24  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.24/dsk-0.24-windows-amd64.zip")
                            .folder("ImpDsk_0_24")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("binaries/dsk-windows-amd64.exe")
                            .build()
                    }
            }
        }

    }
    target_os = "macos" =>
    {

    }
    _ => {
    }
}

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const IMPDISC_CMD: &str = "impdsk";

#[derive(Default)]
pub enum ImpDskVersion {
    #[default]
    V0_34,
    V0_31,
    V0_24
}

impl ImpDskVersion {
    pub fn get_command(&self) -> &str {
        IMPDISC_CMD
    }
}

cfg_select! {
    target_os = "linux" =>
    {
        impl ImpDskVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    ImpDskVersion::V0_34=>DelegateApplicationDescription::builder().download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.34/dsk-0.34-linux-amd64.zip").folder("ImpDsk_0_34").archive_format(ArchiveFormat::Zip).exec_fname("binaries/dsk-linux-amd64").build(),
                    ImpDskVersion::V0_24=>DelegateApplicationDescription::builder().download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.24/dsk-0.24-linux-amd64.zip").folder("ImpDsk_0_24").archive_format(ArchiveFormat::Zip).exec_fname("binaries/dsk-linux-amd64").build(),
                    ImpDskVersion::V0_31 => DelegateApplicationDescription::builder().download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.31/dsk-0.31-linux-amd64.zip").folder("ImpDsk_0_31").archive_format(ArchiveFormat::Zip).exec_fname("binaries/dsk-linux-amd64").build(),
                                    }
            }
        }
    }
    target_os = "windows" =>
    {
        impl ImpDskVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {

                    ImpDskVersion::V0_34  => {
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.34/dsk-0.34-windows-amd64.zip")
                            .folder("ImpDsk_0_34")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("binaries/dsk-windows-amd64.exe")
                            .build()
                    }

                    ImpDskVersion::V0_31  => {
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.31/dsk-0.31-windows-amd64.zip")
                            .folder("ImpDsk_0_31")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("binaries/dsk-windows-amd64.exe")
                            .build()
                    }

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

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const IMPDISC_CMD: &str = "impdsk";

#[derive(Default)]
pub enum ImpDskVersion {
    #[default]
    V0_35,
    V0_34,
    V0_31,
    V0_24
}

impl ImpDskVersion {
    pub fn get_command(&self) -> &str {
        IMPDISC_CMD
    }
}

#[cfg(any(target_os = "linux", target_os = "haiku"))]
        impl ImpDskVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    ImpDskVersion::V0_35=>DelegateApplicationDescription::builder().download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.35/dsk-0.35-linux-amd64.zip").folder("ImpDsk_0_35").archive_format(ArchiveFormat::Zip).exec_fname("binaries/dsk-linux-amd64").build(),
                    ImpDskVersion::V0_34=>DelegateApplicationDescription::builder().download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.34/dsk-0.34-linux-amd64.zip").folder("ImpDsk_0_34").archive_format(ArchiveFormat::Zip).exec_fname("binaries/dsk-linux-amd64").build(),
                    ImpDskVersion::V0_24=>DelegateApplicationDescription::builder().download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.24/dsk-0.24-linux-amd64.zip").folder("ImpDsk_0_24").archive_format(ArchiveFormat::Zip).exec_fname("binaries/dsk-linux-amd64").build(),
                    ImpDskVersion::V0_31 => DelegateApplicationDescription::builder().download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.31/dsk-0.31-linux-amd64.zip").folder("ImpDsk_0_31").archive_format(ArchiveFormat::Zip).exec_fname("binaries/dsk-linux-amd64").build(),
                                    }
            }
        }

#[cfg(target_os = "windows")]
        impl ImpDskVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {

                    ImpDskVersion::V0_35  => {
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.35/dsk-0.35-windows-amd64.zip")
                            .folder("ImpDsk_0_35")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("binaries/dsk-windows-amd64.exe")
                            .build()
                    }

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

#[cfg(target_os = "macos")]
        impl ImpDskVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    ImpDskVersion::V0_35 => {
                        let is_arm = std::env::consts::ARCH == "aarch64";
                        let (url, exec) = if is_arm {
                            (
                                "https://github.com/jeromelesaux/dsk/releases/download/v0.35/dsk-0.35-darwin-arm64.zip",
                                "binaries/dsk-darwin-arm64"
                            )
                        }
                        else {
                            (
                                "https://github.com/jeromelesaux/dsk/releases/download/v0.35/dsk-0.35-darwin-amd64.zip",
                                "binaries/dsk-darwin-amd64"
                            )
                        };

                        DelegateApplicationDescription::builder()
                            .download_fn_url(url)
                            .folder("ImpDsk_0_35")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname(exec)
                            .build()
                    },
                    ImpDskVersion::V0_34 => DelegateApplicationDescription::builder()
                        .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.34/dsk-0.34-linux-amd64.zip")
                        .folder("ImpDsk_0_34")
                        .archive_format(ArchiveFormat::Zip)
                        .exec_fname("binaries/dsk-linux-amd64")
                        .build(),
                    ImpDskVersion::V0_24 => DelegateApplicationDescription::builder()
                        .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.24/dsk-0.24-linux-amd64.zip")
                        .folder("ImpDsk_0_24")
                        .archive_format(ArchiveFormat::Zip)
                        .exec_fname("binaries/dsk-linux-amd64")
                        .build(),
                    ImpDskVersion::V0_31 => DelegateApplicationDescription::builder()
                        .download_fn_url("https://github.com/jeromelesaux/dsk/releases/download/v0.31/dsk-0.31-linux-amd64.zip")
                        .folder("ImpDsk_0_31")
                        .archive_format(ArchiveFormat::Zip)
                        .exec_fname("binaries/dsk-linux-amd64")
                        .build(),
                }
            }
        }

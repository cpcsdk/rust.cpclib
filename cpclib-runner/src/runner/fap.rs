use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const FAP_CMD: &str = "fap";
pub const DOWNLOAD_URL_V1_1: &str = "https://raw.githubusercontent.com/RenaudLottiaux/FastAyPlayer/refs/heads/main/Release/Fap-1.0.0.zip";

#[derive(Default)]
pub enum FAPVersion {
    #[default]
    V1_0_0
}

impl FAPVersion {
    pub fn get_command(&self) -> &str {
        FAP_CMD
    }
}

cfg_match! {
    target_os = "linux" =>
    {
        impl FAPVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    FAPVersion::V1_0_0  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url(DOWNLOAD_URL_V1_1) // we assume a modern CPU
                            .folder("Build")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("FapCrunchLin")
                            .build()
                    }
            }
        }
    }
    target_os = "windows" =>
    {
        impl FAPVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    FAPVersion::V1_0_0  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url(DOWNLOAD_URL_V1_1)
                            .folder("Build")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("FapCrunchWin.exe")
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

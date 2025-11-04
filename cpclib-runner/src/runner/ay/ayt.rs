use cpclib_common::event::EventObserver;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};

#[derive(Default)]
pub enum AytVersion {
    #[default]
    V1_01
}

pub const AYT_CMD: &str = "ayt";
impl AytVersion {
    pub fn get_command(&self) -> &str {
        AYT_CMD
    }
}

cfg_select! {
    target_os = "windows" =>
    {
        impl AytVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    AytVersion::V1_01  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/Logon-System/AYT-Format/releases/download/1.01/ayt-tools-win64.zip") // we assume a modern CPU
                            .folder("ayt_1_01")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("ym2ayt\\ym2ayt.exe")
                            .build()
                }
            }
        }
    }

    target_os = "linux" =>
    {
        impl AytVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                unimplemented!()
            }
        }
    }

     target_os = "macos" =>
    {
        impl AytVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                unimplemented!()
            }
        }
    }
}

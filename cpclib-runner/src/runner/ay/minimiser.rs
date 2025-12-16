use cpclib_common::event::EventObserver;

#[allow(unused_imports)]
use crate::delegated::ArchiveFormat;
use crate::delegated::DelegateApplicationDescription;

#[derive(Default)]
pub enum MinimiserVersion {
    #[default]
    V0_4
}

pub const MINIMISER_CMD: &str = "miny";
impl MinimiserVersion {
    pub fn get_command(&self) -> &str {
        MINIMISER_CMD
    }
}

cfg_select! {
    target_os = "windows" =>
    {
        impl MinimiserVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    MinimiserVersion::V0_4  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/tattlemuss/minymiser/releases/download/release-v0.4/minymiser-release-v0.4.zip") // we assume a modern CPU
                            .folder("minimiser_0_4")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("packer\\bin\\miny-amd64-win.exe")
                            .build()
                }
            }
        }
    }

    target_os = "linux" =>
    {
        impl MinimiserVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    MinimiserVersion::V0_4  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/tattlemuss/minymiser/releases/download/release-v0.4/minymiser-release-v0.4.zip") // we assume a modern CPU
                            .folder("minimiser_0_4")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("packer\\bin\\miny-amd64-win.exe")
                            .build()
                }
            }
        }
    }

     target_os = "macos" =>
    {
        impl MinimiserVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                unimplemented!()
            }
        }
    }
}

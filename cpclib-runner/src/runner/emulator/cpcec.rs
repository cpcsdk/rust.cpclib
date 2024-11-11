use cpclib_common::{camino::Utf8PathBuf, event::EventObserver};

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};

pub const CPCEC_CMD: &str = "cpcec";



#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CpcecVersion {
    #[default]
    V20240505
}

impl CpcecVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();
        conf.cache_folder()
    }
}

cfg_match! {
    cfg(target_os = "linux") =>
    {
        impl CpcecVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    CpcecVersion::V20240505 => {
                        DelegateApplicationDescription::builder()
                            .download_fn_url("http://cngsoft.no-ip.org/cpcec-20240505.zip")
                            .folder("cpcec20240505")
                            .archive_format( ArchiveFormat::Zip)
                            .exec_fname("CPCEC.EXE") // XXX there is a case issue I do not want to solve. so wine is used ...
                            .build()
                    },
                }
            }
        }
	}
	cfg(target_os = "windows") =>
    {
		impl CpcecVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    CpcecVersion::V20240505 => {
                        DelegateApplicationDescription::builder()
                            .download_fn_url("http://cngsoft.no-ip.org/cpcec-20240505.zip")
                            .folder("cpcec20240505")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("CPCEC.EXE")
                            .build()
                    },
                }
            }
        }
	}

    _ => {
    }
}
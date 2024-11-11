use cpclib_common::{camino::Utf8PathBuf, event::EventObserver};

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};

pub const WINAPE_CMD: &str = "winape";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum WinapeVersion {
    #[default]
    V2_0b2
}

impl WinapeVersion {
    pub fn roms_folder(&self) -> Utf8PathBuf {
        let conf = self.configuration::<()>();
        conf.cache_folder().join("ROM")
    }
}

impl WinapeVersion {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            WinapeVersion::V2_0b2 => {
                DelegateApplicationDescription::builder()
                    .download_fn_url("http://www.winape.net/download/WinAPE20B2.zip")
                    .folder("winape_2_0b2")
                    .archive_format(ArchiveFormat::Zip)
                    .exec_fname("WinApe.exe")
                    .build()
            },
        }
    }
}
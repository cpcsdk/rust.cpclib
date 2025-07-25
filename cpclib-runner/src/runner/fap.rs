use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const FAP_CMD: &str = "fap";
pub const DOWNLOAD_URL_V1_0: &str = "https://raw.githubusercontent.com/RenaudLottiaux/FastAyPlayer/refs/heads/main/Release/Fap-1.0.0.zip";
pub const DOWNLOAD_URL_V1_0_2: &str =
    "https://github.com/grim1z/FastAyPlayer/raw/refs/heads/dev/Release/Fap-1.0.2.zip";

#[derive(Default)]
pub enum FAPVersion {
    #[default]
    V1_0_2,
    V1_0_0
}

impl FAPVersion {
    pub fn get_command(&self) -> &str {
        FAP_CMD
    }
}

impl FAPVersion {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let (url, folder) = match self {
            FAPVersion::V1_0_2 => (DOWNLOAD_URL_V1_0_2, "fap1.0.2"),
            FAPVersion::V1_0_0 => (DOWNLOAD_URL_V1_0, "fap1.0.0")
        };

        #[cfg(target_os = "linux")]
        let exec = "FapCrunchLin";
        #[cfg(target_os = "windows")]
        let exec = "FapCrunchWin.exe";

        DelegateApplicationDescription::builder()
            .download_fn_url(url) // we assume a modern CPU
            .folder(folder)
            .archive_format(ArchiveFormat::Zip)
            .exec_fname(exec)
            .build()
    }

    pub fn fap_play_path<E: EventObserver>(&self) -> Utf8PathBuf {
        self.configuration::<E>()
            .cache_folder()
            .join("fap-play.bin")
    }

    pub fn fap_init_path<E: EventObserver>(&self) -> Utf8PathBuf {
        self.configuration::<E>()
            .cache_folder()
            .join("fap-init.bin")
    }
}

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const GRAFX2_CMD: &str = "grafx2";
pub const DOWNLOAD_URL_V2_9_WINDOWS: &str = "https://gitlab.com/GrafX2/grafX2/-/jobs/10877001445/artifacts/raw/grafx2-sdl2-2.9.3245-win32.zip";

#[derive(Default)]
pub enum Grafx2Version {
    #[default]
    V2_9
}

impl Grafx2Version {
    pub fn get_command(&self) -> &str {
        GRAFX2_CMD
    }
}

impl Grafx2Version {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let url = match self {
            Grafx2Version::V2_9 => DOWNLOAD_URL_V2_9_WINDOWS
        };

        let folder = match self {
            Grafx2Version::V2_9 => "grafx2_2.9"
        };

        let exec = "bin/grafx2-sdl2.exe";

        DelegateApplicationDescription::builder()
            .download_fn_url(url) // we assume a modern CPU
            .folder(folder)
            .archive_format(ArchiveFormat::Zip)
            .exec_fname(exec)
            .build()
    }
}

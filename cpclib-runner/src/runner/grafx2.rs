use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const GRAFX2_CMD: &str = "grafx2";
pub const DOWNLOAD_URL_V2_9_WINDOWS: &str = "https://gitlab.com/GrafX2/grafX2/-/jobs/10877001445/artifacts/raw/grafx2-sdl2-2.9.3245-win32.zip";
pub const DOWNLOAD_URL_V2_P_APPIMAGE: &str = "https://gitlab.com/GrafX2/grafX2/-/jobs/10877001436/artifacts/raw/GrafX2-2.9.3245-x86_64.AppImage";
pub const DOWNLOAD_URL_V2_9_MACOS: &str = "https://pulkomandy.tk/projects/GrafX2/downloads/71";
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
            #[cfg(target_os = "windows")]
            Grafx2Version::V2_9 => DOWNLOAD_URL_V2_9_WINDOWS,
            #[cfg(target_os = "linux")]
            Self::V2_9 => DOWNLOAD_URL_V2_P_APPIMAGE,
            #[cfg(target_os = "macos")]
            Self::V2_9 => DOWNLOAD_URL_V2_9_MACOS,
            #[allow(unreachable_patterns)]
            _ => unreachable!()
        };

        let folder = match self {
            Grafx2Version::V2_9 => "grafx2_2.9"
        };

        #[cfg(target_os = "windows")]
        let exec = "bin/grafx2-sdl2.exe";
        #[cfg(target_os = "linux")]
        let exec = "GrafX2-2.9.3245-x86_64.AppImage";
        #[cfg(target_os = "macos")]
        let exec = "GrafX2";

        #[cfg(target_os = "windows")]
        let archive_format = ArchiveFormat::Zip;
        #[cfg(target_os = "linux")]
        let archive_format = ArchiveFormat::Raw;
        #[cfg(target_os = "macos")]
        let archive_format = ArchiveFormat::Zip;

        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(url) // we assume a modern CPU
            .folder(folder)
            .archive_format(archive_format)
            .exec_fname(exec);

        // On linux it is needed to add execution right to the downloaded appimage
        #[cfg(target_os = "linux")]
        let builder = {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    use std::os::unix::fs::PermissionsExt;

                    let app_image = desc.exec_fname();
                    let mut perms = fs_err::metadata(&app_image).unwrap().permissions();
                    let mode = perms.mode() | 0o100; // Add execution mode
                    perms.set_mode(mode);
                    let _ = fs_err::set_permissions(&app_image, perms);
                    Ok(())
                }
            );
            builder.post_install(post_install)
        };

        builder.build()
    }
}

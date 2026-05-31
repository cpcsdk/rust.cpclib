use std::thread::available_parallelism;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const CADENCE_CMD: &str = "cadence";
pub const DOWNLOAD_URL_V0_3A_LINUX: &str =
    "https://github.com/abalore/Cadence/releases/download/v0.3a/Cadence-0.3a-x86_64.AppImage";
pub const DOWNLOAD_URL_V0_3A_MACOS: &str =
    "https://github.com/abalore/Cadence/archive/refs/tags/v0.3a.zip";

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CadenceVersion {
    #[default]
    V0_3a
}

impl CadenceVersion {
    pub fn get_command(&self) -> &str {
        CADENCE_CMD
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let folder = match self {
            CadenceVersion::V0_3a => "cadence_0.3a"
        };

        #[cfg(target_os = "windows")]
        let url: crate::delegated::UrlGenerator = {
            let deferred: Box<dyn Fn() -> Result<String, String>> =
                Box::new(|| Err("Cadence is not available on Windows".to_owned()));
            deferred.into()
        };

        #[cfg(target_os = "linux")]
        let url = DOWNLOAD_URL_V0_3A_LINUX;

        #[cfg(target_os = "macos")]
        let url = DOWNLOAD_URL_V0_3A_MACOS;

        #[cfg(target_os = "windows")]
        let exec = "cadence.exe";
        #[cfg(target_os = "linux")]
        let exec = "Cadence-0.3a-x86_64.AppImage";
        #[cfg(target_os = "macos")]
        let exec = "cadence.app/Contents/MacOS/cadence";

        #[cfg(target_os = "windows")]
        let archive_format = ArchiveFormat::Raw;
        #[cfg(target_os = "linux")]
        let archive_format = ArchiveFormat::Raw;
        #[cfg(target_os = "macos")]
        let archive_format = ArchiveFormat::Zip;

        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(url)
            .folder(folder)
            .archive_format(archive_format)
            .exec_fname(exec);

        #[cfg(target_os = "linux")]
        let builder = {
            let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
                Box::new(|desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    use std::os::unix::fs::PermissionsExt;

                    let app_image = desc.exec_fname();
                    let mut perms = fs_err::metadata(&app_image)
                        .map_err(|e| format!("Unable to inspect {app_image}: {e}"))?
                        .permissions();
                    perms.set_mode(perms.mode() | 0o100);
                    fs_err::set_permissions(&app_image, perms)
                        .map_err(|e| format!("Unable to set execute bit on {app_image}: {e}"))
                });
            builder.post_install(post_install)
        };

        #[cfg(target_os = "macos")]
        let builder = {
            let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
                Box::new(|desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    use std::process::Command;

                    let cache_folder = desc.cache_folder();
                    let src_dir = if cache_folder.join("Cadence.pro").exists() {
                        cache_folder.clone()
                    }
                    else {
                        let extracted = cache_folder.join("Cadence-0.3a");
                        if extracted.join("Cadence.pro").exists() {
                            extracted
                        }
                        else {
                            return Err(format!(
                                "Cadence source folder not found in {}",
                                cache_folder
                            ));
                        }
                    }
                    ;

                    let qmake = Command::new("qmake")
                        .arg("Cadence.pro")
                        .current_dir(&src_dir)
                        .output()
                        .map_err(|e| format!("Failed to run qmake in {src_dir}: {e}"))?;
                    if !qmake.status.success() {
                        return Err(format!(
                            "qmake failed: {}",
                            String::from_utf8_lossy(&qmake.stderr)
                        ));
                    }

                    let jobs = available_parallelism().map(|n| n.get()).unwrap_or(1);
                    let make = Command::new("make")
                        .args(["-j", &jobs.to_string()])
                        .current_dir(&src_dir)
                        .output()
                        .map_err(|e| format!("Failed to run make in {src_dir}: {e}"))?;
                    if !make.status.success() {
                        return Err(format!(
                            "make failed: {}",
                            String::from_utf8_lossy(&make.stderr)
                        ));
                    }

                    let built_app = src_dir.join("cadence.app");
                    let target_app = cache_folder.join("cadence.app");
                    if built_app.exists() && built_app != target_app {
                        if target_app.exists() {
                            fs_err::remove_dir_all(&target_app).map_err(|e| {
                                format!("Failed to remove existing app bundle {target_app}: {e}")
                            })?;
                        }

                        fs_err::rename(&built_app, &target_app).map_err(|e| {
                            format!("Failed to move app bundle from {built_app} to {target_app}: {e}")
                        })?;
                    }

                    Ok(())
                });
            builder.post_install(post_install)
        };

        builder.build()
    }
}

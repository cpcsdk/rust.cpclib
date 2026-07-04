
use cpclib_common::camino::Utf8Path;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription, UrlGenerator};
use crate::event::EventObserver;

pub const CADENCE_CMD: &str = "cadence";
pub const DOWNLOAD_URL_V0_3A_LINUX: &str =
    "https://github.com/abalore/Cadence/releases/download/v0.3a/Cadence-0.3a-x86_64.AppImage";
pub const DOWNLOAD_URL_V0_3A_MACOS: &str =
    "https://github.com/abalore/Cadence/archive/refs/tags/v0.3a.zip";
pub const DOWNLOAD_URL_V1_1_LINUX: &str =
    "https://github.com/abalore/Cadence-releases/releases/download/v1.1/Cadence-1.1-x86_64.AppImage";
pub const DOWNLOAD_URL_V1_1_MACOS: &str =
    "https://github.com/abalore/Cadence-releases/releases/download/v1.1/Cadence-1.1-macOS-arm64.dmg";
pub const DOWNLOAD_URL_V1_1_WINDOWS: &str =
    "https://github.com/abalore/Cadence-releases/releases/download/v1.1/cadence-windows-x64.zip";
pub const DOWNLOAD_URL_V1_4_LINUX: &str =
    "https://github.com/abalore/Cadence-releases/releases/download/v1.4/Cadence-1.4-x86_64.AppImage";
pub const DOWNLOAD_URL_V1_4_MACOS: &str =
    "https://github.com/abalore/Cadence-releases/releases/download/v1.4/Cadence-1.4-macOS-arm64.dmg";
pub const DOWNLOAD_URL_V1_4_WINDOWS: &str =
    "https://github.com/abalore/Cadence-releases/releases/download/v1.4/Cadence-1.4-windows-x64.zip";

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CadenceVersion {
    V0_3a,
    V1_1,
    #[default]
    V1_4
}

impl CadenceVersion {
    pub fn get_command(&self) -> &str {
        CADENCE_CMD
    }

    fn target_folder(&self) -> &'static str {
        match self {
            CadenceVersion::V0_3a => "cadence_0.3a",
            CadenceVersion::V1_1 => "cadence_1.1",
            CadenceVersion::V1_4 => "cadence_1.4"
        }
    }

    #[cfg(target_os = "windows")]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            CadenceVersion::V0_3a => {
                let deferred: Box<dyn Fn() -> Result<String, String>> =
                    Box::new(|| Err("Cadence v0.3a is not available on Windows".to_owned()));
                deferred.into()
            },
            CadenceVersion::V1_1 => DOWNLOAD_URL_V1_1_WINDOWS.into(),
            CadenceVersion::V1_4 => DOWNLOAD_URL_V1_4_WINDOWS.into()
        }
    }

    #[cfg(target_os = "linux")]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            CadenceVersion::V0_3a => DOWNLOAD_URL_V0_3A_LINUX,
            CadenceVersion::V1_1 => DOWNLOAD_URL_V1_1_LINUX,
            CadenceVersion::V1_4 => DOWNLOAD_URL_V1_4_LINUX
        }
        .into()
    }

    #[cfg(target_os = "macos")]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            CadenceVersion::V0_3a => DOWNLOAD_URL_V0_3A_MACOS,
            CadenceVersion::V1_1 => DOWNLOAD_URL_V1_1_MACOS,
            CadenceVersion::V1_4 => DOWNLOAD_URL_V1_4_MACOS
        }
        .into()
    }

    fn target_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            return "cadence.exe";
        }

        #[cfg(target_os = "linux")]
        {
            match self {
                CadenceVersion::V0_3a => "Cadence-0.3a-x86_64.AppImage",
                CadenceVersion::V1_1 => "Cadence-1.1-x86_64.AppImage",
                CadenceVersion::V1_4 => "Cadence-1.4-x86_64.AppImage"
            }
        }

        #[cfg(target_os = "macos")]
        {
            return match self {
                CadenceVersion::V0_3a => "cadence.app/Contents/MacOS/cadence",
                CadenceVersion::V1_1 => "cadence",
                CadenceVersion::V1_4 => "cadence"
            };
        }
    }

    fn target_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "windows")]
        {
            return match self {
                CadenceVersion::V0_3a => ArchiveFormat::Raw,
                CadenceVersion::V1_1 => ArchiveFormat::Zip,
                CadenceVersion::V1_4 => ArchiveFormat::Zip
            };
        }

        #[cfg(target_os = "linux")]
        {
            ArchiveFormat::Raw
        }

        #[cfg(target_os = "macos")]
        {
            return match self {
                CadenceVersion::V0_3a => ArchiveFormat::Zip,
                CadenceVersion::V1_1 => ArchiveFormat::Raw,
                CadenceVersion::V1_4 => ArchiveFormat::Raw
            };
        }
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let _version = self.clone();

        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(self.target_url_generator())
            .folder(self.target_folder())
            .archive_format(self.target_archive_format())
            .exec_fname(self.target_exec_fname());

        #[cfg(target_os = "linux")]
        let builder = {
            let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
                Box::new(|desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    ensure_executable(&desc.exec_fname())
                });
            builder.post_install(post_install)
        };

        #[cfg(target_os = "macos")]
        let builder = {
            let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
                Box::new(move |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    let cache_folder = desc.cache_folder();

                    match &version {
                        CadenceVersion::V0_3a => install_macos_source_release(&cache_folder)?,
                        CadenceVersion::V1_1 | CadenceVersion::V1_4 => install_macos_dmg_release(&cache_folder, &desc.exec_fname())?
                    }

                    Ok(())
                });
            builder.post_install(post_install)
        };

        builder.build()
    }
}

#[cfg(unix)]
fn ensure_executable(path: &Utf8Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs_err::metadata(path)
        .map_err(|e| format!("Unable to inspect {path}: {e}"))?
        .permissions();
    perms.set_mode(perms.mode() | 0o100);
    fs_err::set_permissions(path, perms)
        .map_err(|e| format!("Unable to set execute bit on {path}: {e}"))
}

#[cfg(target_os = "macos")]
fn install_macos_source_release(cache_folder: &Utf8Path) -> Result<(), String> {
    use std::process::Command;

    let src_dir = if cache_folder.join("Cadence.pro").exists() {
        cache_folder.to_owned()
    }
    else {
        let extracted = cache_folder.join("Cadence-0.3a");
        if extracted.join("Cadence.pro").exists() {
            extracted
        }
        else {
            return Err(format!("Cadence source folder not found in {}", cache_folder));
        }
    };

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
            fs_err::remove_dir_all(&target_app)
                .map_err(|e| format!("Failed to remove existing app bundle {target_app}: {e}"))?;
        }

        fs_err::rename(&built_app, &target_app)
            .map_err(|e| format!("Failed to move app bundle from {built_app} to {target_app}: {e}"))?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn install_macos_dmg_release(
    cache_folder: &Utf8Path,
    dmg_path: &Utf8Path
) -> Result<(), String> {
    use std::process::Command;

    let target_app = cache_folder.join("cadence.app");
    let mount_point = cache_folder.join("cadence_mount");
    let launcher = cache_folder.join("cadence");

    if mount_point.exists() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point.as_str(), "-quiet", "-force"])
            .output();
        let _ = fs_err::remove_dir_all(&mount_point);
    }
    fs_err::create_dir_all(&mount_point)
        .map_err(|e| format!("Failed to create Cadence mount directory {mount_point}: {e}"))?;

    let attach = Command::new("hdiutil")
        .args([
            "attach",
            dmg_path.as_str(),
            "-mountpoint",
            mount_point.as_str(),
            "-nobrowse",
            "-quiet"
        ])
        .output()
        .map_err(|e| format!("Failed to mount Cadence DMG {}: {e}", dmg_path))?;
    if !attach.status.success() {
        return Err(format!(
            "Unable to mount Cadence DMG {}: {}",
            dmg_path,
            String::from_utf8_lossy(&attach.stderr)
        ));
    }

    let mounted_app = mount_point.join("cadence.app");
    if !mounted_app.exists() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point.as_str(), "-quiet", "-force"])
            .output();
        return Err(format!("Cadence app bundle not found in mounted DMG at {mounted_app}"));
    }

    if target_app.exists() {
        fs_err::remove_dir_all(&target_app)
            .map_err(|e| format!("Failed to remove previous Cadence app bundle {target_app}: {e}"))?;
    }

    let copy = Command::new("cp")
        .args(["-R", mounted_app.as_str(), target_app.as_str()])
        .output()
        .map_err(|e| format!("Failed to copy Cadence app bundle from DMG to cache: {e}"))?;
    if !copy.status.success() {
        return Err(format!(
            "Unable to copy Cadence app bundle from DMG: {}",
            String::from_utf8_lossy(&copy.stderr)
        ));
    }

    let detach = Command::new("hdiutil")
        .args(["detach", mount_point.as_str(), "-quiet"])
        .output()
        .map_err(|e| format!("Failed to unmount Cadence DMG: {e}"))?;
    if !detach.status.success() {
        return Err(format!(
            "Unable to unmount Cadence DMG: {}",
            String::from_utf8_lossy(&detach.stderr)
        ));
    }
    let _ = fs_err::remove_dir_all(&mount_point);

    let launcher_content = "#!/bin/sh\nSCRIPT_DIR=\"$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\"\nexec \"$SCRIPT_DIR/cadence.app/Contents/MacOS/cadence\" \"$@\"\n";
    fs_err::write(&launcher, launcher_content)
        .map_err(|e| format!("Failed to create Cadence launcher at {launcher}: {e}"))?;
    ensure_executable(&launcher)?;

    Ok(())
}

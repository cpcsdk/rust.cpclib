use std::sync::OnceLock;

use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation,
    InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation
};

pub const RETROVM_CMD: &str = "retrovm";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum RetroVmVersion {
    #[default]
    V2_0Beta1R7
}

impl InternetStaticCompiledApplication for RetroVmVersion {}

impl ExecutableInformation for RetroVmVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            RetroVmVersion::V2_0Beta1R7 => "retrovm_2_0_beta1_r7"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "RetroVirtualMachine.exe";

        #[cfg(target_os = "linux")]
        return "RetroVirtualMachine";

        #[cfg(target_os = "macos")]
        return "RetroVirtualMachine";
    }
}

impl StaticInformation for RetroVmVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        static URLS: OnceLock<MutiplatformUrls> = OnceLock::new();

        URLS.get_or_init(|| {
            MutiplatformUrls::builder()
                .linux(
                    "https://static.retrovm.org/release/beta1/linux/x64/RetroVirtualMachine.2.0.beta-1.r7.linux.x64.zip"
                )
                .windows(
                    "https://static.retrovm.org/release/beta1/windows/x64/RetroVirtualMachine.2.0.beta-1.r7.windows.x64.zip"
                )
                .macos(
                    "https://static.retrovm.org/release/beta1/macos/RetroVirtualMachine.2.0.beta-1.r7.macos.dmg"
                )
                .build()
        })
    }
}

impl DownloadableInformation for RetroVmVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Zip;

        #[cfg(target_os = "linux")]
        return ArchiveFormat::Zip;

        #[cfg(target_os = "macos")]
        return ArchiveFormat::Raw;
    }

    #[cfg(target_os = "linux")]
    fn target_os_postinstall<E: cpclib_common::event::EventObserver>(
        &self
    ) -> Option<crate::delegated::PostInstall<E>> {
        let post_install: Box<dyn Fn(&crate::delegated::DelegateApplicationDescription<E>) -> Result<(), String>> =
            Box::new(|d| {
                use std::os::unix::fs::PermissionsExt;

                let fname = d.exec_fname();
                let mut perms = fs_err::metadata(&fname)
                    .map_err(|e| format!("Failed to inspect {}: {}", fname, e))?
                    .permissions();
                perms.set_mode(perms.mode() | 0o111);
                fs_err::set_permissions(&fname, perms).map_err(|e| e.to_string())
            });

        Some(post_install.into())
    }

    #[cfg(target_os = "macos")]
    fn target_os_postinstall<E: cpclib_common::event::EventObserver>(
        &self
    ) -> Option<crate::delegated::PostInstall<E>> {
        let post_install: Box<dyn Fn(&crate::delegated::DelegateApplicationDescription<E>) -> Result<(), String>> =
            Box::new(|d| {
                use std::process::Command;

                let exec_path = d.exec_fname();
                let dmg_path = d.cache_folder().join("RetroVirtualMachine.2.0.beta-1.r7.macos.dmg");
                let app_path = d.cache_folder().join("Retro Virtual Machine 2.app");

                fs_err::rename(&exec_path, &dmg_path).map_err(|e| {
                    format!(
                        "Failed to rename downloaded DMG from {} to {}: {}",
                        exec_path, dmg_path, e
                    )
                })?;

                let attach = Command::new("hdiutil")
                    .args(["attach", "-nobrowse", "-readonly", dmg_path.as_str()])
                    .output()
                    .map_err(|e| format!("Failed to mount DMG: {}", e))?;

                if !attach.status.success() {
                    return Err(format!(
                        "hdiutil attach failed: {}",
                        String::from_utf8_lossy(&attach.stderr)
                    ));
                }

                let attach_out = String::from_utf8_lossy(&attach.stdout);
                let mount_line = attach_out
                    .lines()
                    .find(|line| line.contains("/Volumes/"))
                    .ok_or_else(|| "Unable to detect DMG mount point".to_owned())?;
                let mount_idx = mount_line
                    .find("/Volumes/")
                    .ok_or_else(|| "Unable to parse DMG mount point".to_owned())?;
                let mount_point = &mount_line[mount_idx..];

                let source_app = Utf8PathBuf::from(mount_point).join("Retro Virtual Machine 2.app");

                if app_path.exists() {
                    fs_err::remove_dir_all(&app_path).map_err(|e| {
                        format!("Failed to remove existing app bundle {}: {}", app_path, e)
                    })?;
                }

                let copy = Command::new("cp")
                    .args(["-R", source_app.as_str(), app_path.as_str()])
                    .output()
                    .map_err(|e| format!("Failed to copy app from DMG: {}", e))?;

                let _ = Command::new("hdiutil")
                    .args(["detach", mount_point])
                    .output();

                if !copy.status.success() {
                    return Err(format!(
                        "Failed to copy RetroVM app bundle: {}",
                        String::from_utf8_lossy(&copy.stderr)
                    ));
                }

                let launcher = "#!/bin/sh\nexec \"$(dirname \"$0\")/Retro Virtual Machine 2.app/Contents/MacOS/Retro Virtual Machine 2\" \"$@\"\n".to_string();

                fs_err::write(&exec_path, launcher)
                    .map_err(|e| format!("Failed to write launcher {}: {}", exec_path, e))?;

                let chmod = Command::new("chmod")
                    .args(["+x", exec_path.as_str()])
                    .output()
                    .map_err(|e| format!("Failed to chmod launcher: {}", e))?;

                if !chmod.status.success() {
                    return Err(format!(
                        "chmod failed for launcher: {}",
                        String::from_utf8_lossy(&chmod.stderr)
                    ));
                }

                Ok(())
            });

        Some(post_install.into())
    }
}

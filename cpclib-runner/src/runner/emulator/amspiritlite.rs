//! AMSpiriT Lite - a portable build of the AMSpiriT emulator.
//!
//! A separate emulator from [`super::amspirit`], not a variant of it: different
//! releases, different command line, and on Linux a different renderer
//! entirely. Upstream ships three unrelated packages per release - a Qt build
//! for Windows and macOS, and an SDL/ImGui AppImage for Linux - so the URL, the
//! archive format and the executable name all differ by platform rather than
//! only the URL.
//!
//! It also carries an embedded HTTP debug server, which is why it is worth
//! having: it can answer far more than the wasm emulator can.

use cpclib_common::camino::Utf8Path;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription, UrlGenerator};
use crate::event::EventObserver;

pub const AMSPIRIT_LITE_CMD: &str = "amspiritlite";

pub const DOWNLOAD_URL_V1_14_WINDOWS: &str = "https://github.com/AMSpiriT-Emulator/amspirit-releases/releases/download/Lite-1.14.3/Amspirit-lite-Qt-1.14.3-win64.zip";
pub const DOWNLOAD_URL_V1_14_MACOS: &str = "https://github.com/AMSpiriT-Emulator/amspirit-releases/releases/download/Lite-1.14.3/Amspirit-lite-Qt-1.14.3-mac-arm64.dmg";
/// SDL rather than Qt on Linux: the Qt build of this release does not work
/// there, which is why the three platforms do not share a package.
pub const DOWNLOAD_URL_V1_14_LINUX: &str = "https://github.com/AMSpiriT-Emulator/amspirit-releases/releases/download/Lite-1.14.3/Amspirit-Lite-SDL-ImGui-1.14.3-x86_64.AppImage";

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AmspiritLiteVersion {
    #[default]
    V1_14_3
}

impl AmspiritLiteVersion {
    pub fn get_command(&self) -> &str {
        AMSPIRIT_LITE_CMD
    }

    fn target_folder(&self) -> &'static str {
        match self {
            Self::V1_14_3 => "amspirit_lite_1.14.3"
        }
    }

    #[cfg(target_os = "windows")]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            Self::V1_14_3 => DOWNLOAD_URL_V1_14_WINDOWS
        }
        .into()
    }

    #[cfg(target_os = "linux")]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            Self::V1_14_3 => DOWNLOAD_URL_V1_14_LINUX
        }
        .into()
    }

    #[cfg(target_os = "macos")]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            Self::V1_14_3 => DOWNLOAD_URL_V1_14_MACOS
        }
        .into()
    }

    fn target_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            return "Amspirit-Lite-Qt.exe";
        }

        #[cfg(target_os = "linux")]
        {
            return "Amspirit-Lite-SDL-ImGui-1.14.3-x86_64.AppImage";
        }

        #[cfg(target_os = "macos")]
        {
            return "Amspirit-Lite.app/Contents/MacOS/Amspirit-Lite";
        }
    }

    fn target_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "windows")]
        {
            return ArchiveFormat::Zip;
        }

        // An AppImage is the executable; there is nothing to unpack.
        #[cfg(target_os = "linux")]
        {
            return ArchiveFormat::Raw;
        }

        // A `.dmg` is mounted rather than extracted, which the post-install
        // step below does.
        #[cfg(target_os = "macos")]
        {
            return ArchiveFormat::Raw;
        }
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(self.target_url_generator())
            .folder(self.target_folder())
            .archive_format(self.target_archive_format())
            .exec_fname(self.target_exec_fname());

        // A downloaded AppImage arrives without its executable bit.
        #[cfg(target_os = "linux")]
        let builder = {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    ensure_executable(&desc.exec_fname())
                }
            );
            builder.post_install(post_install)
        };

        #[cfg(target_os = "macos")]
        let builder = {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    super::cadence::install_macos_dmg_release(
                        &desc.cache_folder(),
                        &desc.exec_fname()
                    )
                }
            );
            builder.post_install(post_install)
        };

        builder.build()
    }
}

#[cfg(target_os = "linux")]
fn ensure_executable(path: &Utf8Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs_err::metadata(path)
        .map_err(|e| format!("Unable to inspect {path}: {e}"))?
        .permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs_err::set_permissions(path, permissions)
        .map_err(|e| format!("Unable to make {path} executable: {e}"))
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
fn ensure_executable(_path: &Utf8Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three platforms, three unrelated packages.
    ///
    /// Upstream ships a Qt build for Windows and macOS and an SDL/ImGui
    /// AppImage for Linux, so unlike most emulators here the URL, the archive
    /// format *and* the executable name all differ per platform.
    #[test]
    fn every_platform_has_its_own_package() {
        for url in [
            DOWNLOAD_URL_V1_14_WINDOWS,
            DOWNLOAD_URL_V1_14_MACOS,
            DOWNLOAD_URL_V1_14_LINUX
        ] {
            assert!(
                url.starts_with("https://github.com/AMSpiriT-Emulator/"),
                "{url}"
            );
            assert!(url.contains("Lite-1.14"), "{url}");
        }
        assert!(DOWNLOAD_URL_V1_14_WINDOWS.ends_with(".zip"));
        assert!(DOWNLOAD_URL_V1_14_MACOS.ends_with(".dmg"));
        assert!(
            DOWNLOAD_URL_V1_14_LINUX.ends_with(".AppImage"),
            "the Qt build does not work on Linux for this release"
        );
        // Three different packages, not one URL reused.
        assert_ne!(DOWNLOAD_URL_V1_14_WINDOWS, DOWNLOAD_URL_V1_14_LINUX);
        assert_ne!(DOWNLOAD_URL_V1_14_MACOS, DOWNLOAD_URL_V1_14_LINUX);
    }

    /// The executable this platform will actually run matches the package it
    /// downloads - getting these out of step installs fine and launches
    /// nothing.
    #[test]
    fn the_executable_matches_the_package() {
        let version = AmspiritLiteVersion::default();
        let executable = version.target_exec_fname();

        #[cfg(target_os = "linux")]
        assert_eq!(executable, "Amspirit-Lite-SDL-ImGui-1.14.3-x86_64.AppImage");
        #[cfg(target_os = "windows")]
        assert!(executable.ends_with(".exe"), "{executable}");
        #[cfg(target_os = "macos")]
        assert!(executable.contains("Contents/MacOS"), "{executable}");
    }

    /// It is its own emulator, cached separately from AMSpiriT proper.
    #[test]
    fn it_does_not_share_a_cache_with_amspirit() {
        let folder = AmspiritLiteVersion::default().target_folder();
        assert!(folder.contains("lite"), "{folder}");
        assert!(
            folder.contains("1.14.3"),
            "and per release, so two versions never share one: {folder}"
        );
    }

    #[test]
    fn it_answers_to_its_own_command_name() {
        assert_eq!(AmspiritLiteVersion::default().get_command(), "amspiritlite");
    }
}

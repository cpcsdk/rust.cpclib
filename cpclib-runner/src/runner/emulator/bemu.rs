//! bemu (`amsrust`): a pin-level CPC emulator written by the author of this workspace.
//!
//! It serves an AMSpiriT-Lite compatible HTTP API (`--debug-server`, see
//! [`super::amspiritlite_api`]) that the `Robot` automation reuses: memory, screenshot, keyboard,
//! media, BASIC...
//!
//! It is downloaded from the release page of its repository (<https://github.com/rgiot/bemu>):
//! its CI publishes there a build per platform (Linux x86_64, macOS arm64, Windows x86_64).
//! The ROMs it needs (AMSDOS, UniDOS, Orgams...) are embedded in it (`inner://NAME`).

use std::sync::OnceLock;

use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{
    ArchiveFormat, DelegateApplicationDescription, DownloadableInformation, ExecutableInformation,
    InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation
};
use crate::event::EventObserver;

pub const BEMU_CMD: &str = "bemu";

/// Where the builds of bemu are published
const BEMU_RELEASES: &str = "https://github.com/rgiot/bemu/releases/download";

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BemuVersion {
    /// The development build of bemu's default branch (the `latest` release)
    #[default]
    Latest
}

impl BemuVersion {
    pub fn get_command(&self) -> &str {
        BEMU_CMD
    }

    /// The tag of the GitHub release
    fn tag(&self) -> &'static str {
        match self {
            Self::Latest => "latest"
        }
    }

    /// Where the ROMs given to `--rom` are written
    pub fn roms_folder(&self) -> Utf8PathBuf {
        self.configuration::<()>().cache_folder().join("roms")
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        InternetStaticCompiledApplication::configuration(self)
    }
}

impl StaticInformation for BemuVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        static LATEST: OnceLock<MutiplatformUrls> = OnceLock::new();
        let tag = self.tag();
        let url = |asset: &str| format!("{BEMU_RELEASES}/{tag}/{asset}");
        match self {
            Self::Latest => {
                LATEST.get_or_init(|| {
                    MutiplatformUrls::builder()
                        .linux(url("bemu-linux-x86_64.tar.gz"))
                        .windows(url("bemu-windows-x86_64.zip"))
                        .macos(url("bemu-macos-arm64.tar.gz"))
                        .build()
                })
            },
        }
    }
}

impl ExecutableInformation for BemuVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::Latest => "bemu_latest"
        }
    }

    // The archives hold the executable at their root
    fn target_os_exec_fname(&self) -> &'static str {
        if cfg!(target_os = "windows") {
            "amsrust.exe"
        }
        else {
            "amsrust"
        }
    }
}

impl DownloadableInformation for BemuVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        if cfg!(target_os = "windows") {
            ArchiveFormat::Zip
        }
        else {
            ArchiveFormat::TarGz
        }
    }

    // An archive does not always keep the executable bit.
    #[cfg(unix)]
    fn target_os_postinstall<E: EventObserver>(&self) -> Option<crate::delegated::PostInstall<E>> {
        let post_install: Box<crate::delegated::PostInstallFn<E>> = Box::new(|d, _o| {
            use std::os::unix::fs::PermissionsExt;

            let fname = d.exec_fname();
            let mut perms = fs_err::metadata(&fname)
                .map_err(|e| format!("Failed to inspect {fname}: {e}"))?
                .permissions();
            perms.set_mode(perms.mode() | 0o111);
            fs_err::set_permissions(&fname, perms)
                .map_err(|e| format!("Failed to make {fname} executable: {e}"))
        });
        Some(post_install.into())
    }
}

impl InternetStaticCompiledApplication for BemuVersion {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_is_downloaded_from_the_github_releases() {
        let version = BemuVersion::default();
        let conf = version.configuration::<()>();
        assert!(conf.custom_install.is_none());
        assert_eq!(conf.exec_fname.trim_end_matches(".exe"), "amsrust");

        let urls = version.static_download_urls();
        for url in [&urls.linux, &urls.windows, &urls.macos] {
            let url = url.as_deref().unwrap();
            assert!(
                url.starts_with("https://github.com/rgiot/bemu/releases/download/latest/bemu-"),
                "{url}"
            );
        }
    }
}

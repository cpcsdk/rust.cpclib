use std::fmt::Display;

use crate::delegated::{
    ArchiveFormat, DelegateApplicationDescription, DownloadableInformation, GithubInformation
};
use crate::event::EventObserver;
use crate::runner::exec::RunInDir;

/// konCePCja, by ikari: https://github.com/ikari-pl/konCePCja
///
/// `koncepcja [options] <slotfile(s)>`: the media are positional, a `.dsk`
/// goes in drive A and a second one in drive B, and `-a/--autocmd=<text>` is
/// typed once the CPC has booted (each one ends with Enter).
pub const KONCEPCJA_CMD: &str = "koncepcja";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum KoncepcjaVersion {
    #[default]
    V6_4_2
}

impl Display for KoncepcjaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "konCePCja {}", self.version_name())
    }
}

impl GithubInformation for KoncepcjaVersion {
    fn project(&self) -> &'static str {
        "konCePCja"
    }

    fn owner(&self) -> &'static str {
        "ikari-pl"
    }

    fn version_name(&self) -> &'static str {
        match self {
            Self::V6_4_2 => "v6.4.2"
        }
    }

    // There is no Linux build in the releases (only the sources), so Linux
    // runs the Windows one through wine.
    fn linux_key(&self) -> Option<&'static str> {
        self.windows_key()
    }

    fn windows_key(&self) -> Option<&'static str> {
        match self {
            Self::V6_4_2 => Some("koncepcja-windows-x64.zip")
        }
    }

    // koncepcja-macos.zip holds a bare executable that expects SDL3 to be
    // installed; the dmg holds the self-contained app bundle.
    fn macos_key(&self) -> Option<&'static str> {
        match self {
            Self::V6_4_2 => Some("konCePCja.dmg")
        }
    }
}

impl DownloadableInformation for KoncepcjaVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        // The dmg is mounted by the post-install step, not extracted.
        #[cfg(target_os = "macos")]
        return ArchiveFormat::Raw;
        #[cfg(not(target_os = "macos"))]
        return ArchiveFormat::Zip;
    }
}

impl KoncepcjaVersion {
    fn target_folder(&self) -> &'static str {
        match self {
            Self::V6_4_2 => "konCePCja_6.4.2"
        }
    }

    fn target_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        return "konCePCja.app/Contents/MacOS/koncepcja";
        #[cfg(not(target_os = "macos"))]
        return "koncepcja.exe";
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(self)
            .folder(self.target_folder())
            .archive_format(self.target_os_archive_format())
            .exec_fname(self.target_exec_fname())
            // koncepcja.cfg points at rom/ and resources/ beside the executable
            .in_dir(RunInDir::AppDir);

        #[cfg(target_os = "macos")]
        let builder = {
            let post_install: Box<crate::delegated::PostInstallFn<E>> = Box::new(
                |desc: &DelegateApplicationDescription<E>, _o: &E| -> Result<(), String> {
                    let cache_folder = desc.cache_folder();
                    let dmg = cache_folder.join("konCePCja.dmg");
                    super::cadence::install_macos_dmg_release(
                        &cache_folder,
                        &dmg,
                        "konCePCja.app",
                        None
                    )?;
                    fs_err::remove_file(&dmg).map_err(|e| format!("Unable to remove {dmg}: {e}"))
                }
            );
            builder
                .download_fname("konCePCja.dmg")
                .post_install(post_install)
        };

        builder.build()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn linux_runs_the_windows_build() {
        let version = KoncepcjaVersion::default();
        assert_eq!(version.linux_key(), version.windows_key());
        #[cfg(not(target_os = "macos"))]
        assert!(version.target_exec_fname().ends_with(".exe"));
    }
}

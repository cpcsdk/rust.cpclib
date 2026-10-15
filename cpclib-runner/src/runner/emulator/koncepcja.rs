use crate::delegated::{ArchiveFormat, DelegateApplicationDescription, UrlGenerator};
use crate::event::EventObserver;
use crate::runner::exec::RunInDir;

/// konCePCja, by ikari: https://github.com/ikari-pl/konCePCja
///
/// `koncepcja [options] <slotfile(s)>`: the media are positional, a `.dsk`
/// goes in drive A and a second one in drive B, and `-a/--autocmd=<text>` is
/// typed once the CPC has booted (each one ends with Enter).
pub const KONCEPCJA_CMD: &str = "koncepcja";

#[cfg(not(target_os = "macos"))]
const DOWNLOAD_URL_V6_4_2_WINDOWS: &str =
    "https://github.com/ikari-pl/konCePCja/releases/download/v6.4.2/koncepcja-windows-x64.zip";
/// The macOS zip holds a bare executable that expects SDL3 to be installed;
/// the dmg holds the self-contained app bundle.
#[cfg(target_os = "macos")]
const DOWNLOAD_URL_V6_4_2_MACOS: &str =
    "https://github.com/ikari-pl/konCePCja/releases/download/v6.4.2/konCePCja.dmg";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum KoncepcjaVersion {
    #[default]
    V6_4_2
}

impl KoncepcjaVersion {
    fn target_folder(&self) -> &'static str {
        match self {
            Self::V6_4_2 => "konCePCja_6.4.2"
        }
    }

    // There is no Linux build in the releases (only the sources), so Linux
    // runs the Windows one through wine.
    #[cfg(not(target_os = "macos"))]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            Self::V6_4_2 => DOWNLOAD_URL_V6_4_2_WINDOWS
        }
        .into()
    }

    #[cfg(target_os = "macos")]
    fn target_url_generator(&self) -> UrlGenerator {
        match self {
            Self::V6_4_2 => DOWNLOAD_URL_V6_4_2_MACOS
        }
        .into()
    }

    fn target_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        return "konCePCja.app/Contents/MacOS/koncepcja";
        #[cfg(not(target_os = "macos"))]
        return "koncepcja.exe";
    }

    fn target_archive_format(&self) -> ArchiveFormat {
        // The dmg is mounted by the post-install step, not extracted.
        #[cfg(target_os = "macos")]
        return ArchiveFormat::Raw;
        #[cfg(not(target_os = "macos"))]
        return ArchiveFormat::Zip;
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(self.target_url_generator())
            .folder(self.target_folder())
            .archive_format(self.target_archive_format())
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
    #[cfg(not(target_os = "macos"))]
    fn the_windows_build_is_the_one_run_outside_macos() {
        assert!(DOWNLOAD_URL_V6_4_2_WINDOWS.ends_with("koncepcja-windows-x64.zip"));
        assert!(
            KoncepcjaVersion::default()
                .target_exec_fname()
                .ends_with(".exe")
        );
    }
}

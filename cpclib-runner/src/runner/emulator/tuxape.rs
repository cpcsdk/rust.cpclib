//! TuxAPE, by Thomas Bressel: <https://tuxape.amstariga.fr> (sources:
//! <https://github.com/thomas-bressel/tux-ape>).
//!
//! A C++20 / Qt 6 re-implementation of WinAPE 2.0 Beta 2 (same functions, same file formats, same
//! interface layout), for Linux on a 64-bit PC. Its command line is `tuxape [--sym <file>]...
//! [--tape <file>] [--insert <drive>:<file>]... [<disc>]`: the optional positional argument is the
//! disc put in drive A:, `--insert B:file` the one for drive B:. There is no option to load a
//! snapshot, to run a program or to type keys: those are menu entries of the window.
//!
//! The author publishes the builds on his site, not as GitHub releases: the page
//! <https://tuxape.amstariga.fr/telechargements.html> links
//! `telechargements/tuxape-<version>-linux-x86_64.tar.gz` (34 MB) and a `.deb` package. The archive
//! unpacks to a `tuxape-<version>-linux-x86_64` folder holding the program (executable), Qt, the ROMs
//! (`roms/`), the profiles and the folders it works in; it keeps everything there, so it is run in
//! its own folder. Its `LICENCE.txt` allows to use, copy and give it away whole and free of charge.
//!
//! The site keeps the current version only: an older one disappears (1.1.9 does not answer
//! any more), so the version that is known is the current one when this is written.

use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{
    ArchiveFormat, DelegateApplicationDescription, DownloadableInformation, ExecutableInformation,
    InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation
};
use crate::event::EventObserver;
use crate::runner::exec::RunInDir;

pub const TUXAPE_CMD: &str = "tuxape";

/// Where the builds of TuxAPE are published
const TUXAPE_DOWNLOADS: &str = "https://tuxape.amstariga.fr/telechargements";

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TuxApeVersion {
    #[default]
    V1_1_10
}

impl TuxApeVersion {
    pub fn get_command(&self) -> &str {
        TUXAPE_CMD
    }

    fn version_name(&self) -> &'static str {
        match self {
            Self::V1_1_10 => "1.1.10"
        }
    }

    /// Where the ROMs of the emulator are (it ships all of Amstrad's)
    pub fn roms_folder(&self) -> Utf8PathBuf {
        self.configuration::<()>()
            .cache_folder()
            .join(format!("tuxape-{}-linux-x86_64/roms", self.version_name()))
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        InternetStaticCompiledApplication::configuration(self)
    }
}

impl StaticInformation for TuxApeVersion {
    // Linux only: no other build is published.
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        static V1_1_10: std::sync::OnceLock<MutiplatformUrls> = std::sync::OnceLock::new();
        match self {
            Self::V1_1_10 => {
                V1_1_10.get_or_init(|| {
                    MutiplatformUrls::builder()
                        .linux(format!(
                            "{TUXAPE_DOWNLOADS}/tuxape-{}-linux-x86_64.tar.gz",
                            self.version_name()
                        ))
                        .build()
                })
            },
        }
    }
}

impl ExecutableInformation for TuxApeVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V1_1_10 => "TuxAPE_1.1.10"
        }
    }

    // The archive keeps its `tuxape-<version>-linux-x86_64` folder.
    fn target_os_exec_fname(&self) -> &'static str {
        match self {
            Self::V1_1_10 => "tuxape-1.1.10-linux-x86_64/tuxape"
        }
    }

    // It finds roms/, profile/, plugins/... beside the executable (and its Qt in qt/).
    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl DownloadableInformation for TuxApeVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::TarGz
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

impl InternetStaticCompiledApplication for TuxApeVersion {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_is_downloaded_from_its_site_and_run_in_its_folder() {
        let version = TuxApeVersion::default();
        let conf = version.configuration::<()>();
        assert!(conf.custom_install.is_none());
        assert_eq!(conf.exec_fname, "tuxape-1.1.10-linux-x86_64/tuxape");

        let urls = version.static_download_urls();
        assert_eq!(
            urls.linux.as_deref(),
            Some("https://tuxape.amstariga.fr/telechargements/tuxape-1.1.10-linux-x86_64.tar.gz")
        );
        // only a Linux build exists
        assert!(urls.windows.is_none() && urls.macos.is_none());
        assert!(version
            .roms_folder()
            .ends_with("tuxape-1.1.10-linux-x86_64/roms"));
    }
}

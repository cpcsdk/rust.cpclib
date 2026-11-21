use std::fmt::Display;

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation, GithubCompiledApplication,
    GithubInformation
};
use crate::event::EventObserver;
use crate::runner::exec::RunInDir;

/// TuxAPE, by Thomas Bressel: <https://github.com/thomas-bressel/tux-ape>
///
/// A C++20 / Qt 6 re-implementation of WinAPE 2.0 Beta 2 (same functions, same file formats, same
/// interface layout). Its command line is `tuxape [--sym <file>]... [--tape <file>]
/// [--insert <drive>:<file>]... [<disc>]`: the optional positional argument is the disc put in drive
/// A:, `--insert B:file` the one for drive B:. There is no option to load a snapshot, to run a
/// program or to type keys: those are menu entries of the window.
///
/// It contains no ROM: it reads WinAPE's (the folder `WinAPE20B2` of the original distribution
/// beside the program, or the folder named by `TUXAPE_ROM_DIR`), see its README.
///
/// The builds are published by the author as GitHub assets: a folder (`TuxAPE/`) holding the
/// program, Qt and the files it needs. At the time of writing only the CI of the repository
/// builds it (Linux x86_64 and arm64, as workflow artifacts) and no release has been published:
/// [`GithubInformation`] below names the assets such a release is expected to have, and has to be
/// checked against the first one.
pub const TUXAPE_CMD: &str = "tuxape";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum TuxApeVersion {
    #[default]
    V1_1_9
}

impl Display for TuxApeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TuxAPE {}", self.version_name().trim_start_matches('v'))
    }
}

impl GithubCompiledApplication for TuxApeVersion {}

impl ExecutableInformation for TuxApeVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V1_1_9 => "TuxAPE_1.1.9"
        }
    }

    // The archive keeps the `TuxAPE/` folder of the build.
    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "TuxAPE/tuxape.exe";
        #[cfg(not(target_os = "windows"))]
        return "TuxAPE/tuxape";
    }

    // It finds roms/, plugins/, profile/... beside the executable (and its Qt in qt/).
    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl GithubInformation for TuxApeVersion {
    fn project(&self) -> &'static str {
        "tux-ape"
    }

    fn owner(&self) -> &'static str {
        "thomas-bressel"
    }

    fn version_name(&self) -> &'static str {
        match self {
            Self::V1_1_9 => "v1.1.9"
        }
    }

    // Only Linux is built (x86_64; the arm64 build is for the Raspberry Pi 4 and 5).
    fn linux_key(&self) -> Option<&'static str> {
        match self {
            Self::V1_1_9 => Some("TuxAPE-linux-x86_64.tar.gz")
        }
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::delegated::GithubCompiledApplication;

    #[test]
    fn it_is_a_github_executable_run_in_its_folder() {
        let version = TuxApeVersion::default();
        assert_eq!(version.to_string(), "TuxAPE 1.1.9");
        assert_eq!(version.owner(), "thomas-bressel");
        assert_eq!(version.project(), "tux-ape");
        assert!(version.linux_key().unwrap().ends_with(".tar.gz"));
        // only Linux builds exist
        assert!(version.windows_key().is_none());
        assert!(version.macos_key().is_none());

        let conf = GithubCompiledApplication::configuration::<()>(&version);
        assert!(conf.custom_install.is_none());
        assert!(
            conf.exec_fname.ends_with("TuxAPE/tuxape")
                || conf.exec_fname.ends_with("TuxAPE/tuxape.exe")
        );
    }
}

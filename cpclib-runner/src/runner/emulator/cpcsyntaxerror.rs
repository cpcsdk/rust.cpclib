use std::sync::OnceLock;

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation,
    InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation
};
use crate::event::EventObserver;
use crate::runner::exec::RunInDir;

/// CPCSyntaxError, by WacKEDmaN: https://github.com/WacKEDmaN/CPCSyntaxError
///
/// The window build takes its media with `--load <file>` (a `.dsk`, `.sna`,
/// `.cpr` or a tape, as the Media menu loads them) and types `--command
/// <text>` afterwards (`\n` is Enter). `--csl <script>` is a separate,
/// windowless CSL player, so it is not used to launch it.
pub const CPCSYNTAXERROR_CMD: &str = "cpcse";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CpcSyntaxErrorVersion {
    #[default]
    V0_6_2
}

impl InternetStaticCompiledApplication for CpcSyntaxErrorVersion {}

impl ExecutableInformation for CpcSyntaxErrorVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V0_6_2 => "CPCSyntaxError_0.6.2"
        }
    }

    // The zips are unpacked without their top folder; the tar.gz keeps it.
    fn target_os_exec_fname(&self) -> &'static str {
        match self {
            Self::V0_6_2 => {
                #[cfg(target_os = "windows")]
                return "cpcse.exe";
                #[cfg(target_os = "macos")]
                return "CPCSyntaxError.app/Contents/MacOS/cpcse";
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                return "CPCSyntaxError-linux-x86_64-v0.6.2/cpcse";
            }
        }
    }

    // It finds roms/ and sounds/ beside the executable.
    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl StaticInformation for CpcSyntaxErrorVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        match self {
            Self::V0_6_2 => {
                static URLS: OnceLock<MutiplatformUrls> = OnceLock::new();
                URLS.get_or_init(|| {
                    MutiplatformUrls::builder()
                        .windows("https://github.com/WacKEDmaN/CPCSyntaxError/releases/download/v0.6.2/CPCSyntaxError-win64-v0.6.2.zip")
                        .linux("https://github.com/WacKEDmaN/CPCSyntaxError/releases/download/v0.6.2/CPCSyntaxError-linux-x86_64-v0.6.2.tar.gz")
                        .macos("https://github.com/WacKEDmaN/CPCSyntaxError/releases/download/v0.6.2/CPCSyntaxError-macos-universal-v0.6.2.zip")
                        .build()
                })
            }
        }
    }
}

impl DownloadableInformation for CpcSyntaxErrorVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        return ArchiveFormat::Zip;
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        return ArchiveFormat::TarGz;
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

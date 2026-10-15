use std::fmt::Display;
use std::sync::OnceLock;

use crate::delegated::{
    ArchiveFormat, CompilableInformation, DelegateApplicationDescription, DownloadableInformation,
    ExecutableInformation, MutiplatformUrls, StaticInformation
};
use crate::event::EventObserver;

/// fantams, by sikorama: https://github.com/sikorama/fantams
///
/// `fantams file.asm [-o out] [--sym[=file]] ...`: the extension of `-o`
/// selects the output (`.sna` a snapshot, `.cpr`/`.cro` a ROM container,
/// anything else a raw binary).
///
/// It publishes no release and no binary: a commit's sources are downloaded
/// and built with `make` (a C++17 compiler is needed).
pub const FANTAMS_CMD: &str = "fantams";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum FantamsVersion {
    /// The commit that `fantams --version` reports as release 2026-09-08.
    #[default]
    V2026_09_08
}

impl FantamsVersion {
    fn commit(&self) -> &'static str {
        match self {
            Self::V2026_09_08 => "575aa8c79c0a6467098756f7e46790ec2cbb682c"
        }
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        DelegateApplicationDescription::builder()
            .download_fn_url(self.target_os_url_generator())
            .folder(self.target_os_folder())
            .archive_format(self.target_os_archive_format())
            .exec_fname(self.target_os_exec_fname())
            .maybe_compile(self.target_os_compiler())
            .in_dir(self.target_os_run_in_dir())
            .build()
    }
}

impl Display for FantamsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V2026_09_08 => write!(f, "fantams 2026-09-08")
        }
    }
}

impl StaticInformation for FantamsVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        match self {
            Self::V2026_09_08 => {
                static URLS: OnceLock<MutiplatformUrls> = OnceLock::new();
                URLS.get_or_init(|| {
                    MutiplatformUrls::unique_url(&format!(
                        "https://github.com/sikorama/fantams/archive/{}.zip",
                        self.commit()
                    ))
                })
            }
        }
    }
}

impl DownloadableInformation for FantamsVersion {
    // The top folder of the archive (fantams-<commit>) is dropped when it is
    // unpacked, so the sources land directly in the cache folder.
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }
}

impl ExecutableInformation for FantamsVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V2026_09_08 => "fantams_2026_09_08"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "fantams.exe";
        #[cfg(not(target_os = "windows"))]
        return "fantams";
    }
}

impl CompilableInformation for FantamsVersion {
    // Only the assembler: the default target also builds the test suites.
    fn target_os_commands(&self) -> Option<&'static [&'static [&'static str]]> {
        Some(&[&["make", "fantams"]])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_sources_of_the_pinned_commit_are_downloaded() {
        let url = FantamsVersion::default()
            .static_download_urls()
            .target_os_url()
            .cloned();
        assert_eq!(
            url.as_deref(),
            Some(
                "https://github.com/sikorama/fantams/archive/575aa8c79c0a6467098756f7e46790ec2cbb682c.zip"
            )
        );
    }
}

use std::fmt::Display;

use cpclib_common::event::EventObserver;

use crate::{delegated::{ArchiveFormat, CompilableInformation, DelegateApplicationDescription, DownloadableInformation, ExecutableInformation, GithubCompilableApplication, GithubInformation, MutiplatformUrls}, runner::runner::RunInDir};


pub const SUGARBOX_V2_CMD: &str = "sugarbox";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum SugarBoxV2Version {
    #[default]
    V2_0_2
}


fn sugarbox_download_urls(version: SugarBoxV2Version) -> Result<MutiplatformUrls, String> {
    version.github_download_urls()
}


impl GithubCompilableApplication for SugarBoxV2Version {

}

impl Display for SugarBoxV2Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sugarbox {}", self.version_name())
    }
}

impl DownloadableInformation for SugarBoxV2Version {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "windows")]
        return ArchiveFormat::SevenZ;
        #[cfg(target_os = "macos")]
        return ArchiveFormat::TarGz;
        #[cfg(target_os = "linux")]
        return ArchiveFormat::TarGz;
    }
}

// currently we dumbly use the compiled version of linux instead of compiling it ourselves.
// TODO really compile for Linux
impl CompilableInformation for SugarBoxV2Version {
    fn target_os_commands(&self) -> Option<&'static[&'static[&'static str]]> {
        None
    }
}

impl ExecutableInformation for SugarBoxV2Version {
    fn target_os_folder(&self) -> &'static str {
        match self {
            SugarBoxV2Version::V2_0_2 => {
                #[cfg(target_os = "windows")]
                return "Sugarbox-2.0.2-win64/Sugarbox-2.0.2-win64";
                #[cfg(target_os = "macos")]
                return "Sugarbox-2.0.2-Darwin";
                #[cfg(target_os = "linux")]
                return "Sugarbox-2.0.2-Linux/Sugarbox-2.0.2-Linux";
            },
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "Sugarbox.exe";
        #[cfg(target_os = "macos")]
        return "Sugarbox";
        #[cfg(target_os = "linux")]
        return "Sugarbox";
    }
}

impl GithubInformation for SugarBoxV2Version {
    fn project(&self) -> &'static str {
        "SugarboxV2"
    }

    fn owner(&self) -> &'static str {
        "Tom1975"
    }

    fn version_name(&self) -> &'static str {
        match self {
            SugarBoxV2Version::V2_0_2 => "v2.0.2",
        }
    }
}


impl SugarBoxV2Version {



    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let version_cloned = self.clone();
        let get_url = move || -> Result<String, String> {
            sugarbox_download_urls(version_cloned.clone())
                .map(|urls| urls.target_os_url().unwrap().clone())
        };
        let get_url: Box<dyn Fn() -> Result<String,String>>  = Box::new(get_url);

        DelegateApplicationDescription::builder()
            .download_fn_url(get_url) // we assume a modern CPU
            .folder(self.target_os_folder())
            .archive_format(self.target_os_archive_format())
            .exec_fname(self.target_os_exec_fname())
            .in_dir(RunInDir::AppDir)
            .build()
    }
}
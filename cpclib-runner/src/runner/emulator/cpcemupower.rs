const URL: &str =
    "https://www.cpc-power.com/cpcarchives/download/Emulateurs/20210531_CPCEPower_SDL_Release.zip";

use std::sync::OnceLock;

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation,
    InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation
};
use crate::runner::runner::RunInDir;

pub const CPCEMUPOWER_CMD: &str = "cpcemupower";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CpcEmuPowerVersion {
    #[default]
    R20210531
}

impl InternetStaticCompiledApplication for CpcEmuPowerVersion {}

impl ExecutableInformation for CpcEmuPowerVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::R20210531 => "20210531_CPCEPower_SDL_Release"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        match self {
            Self::R20210531 => {
                #[cfg(target_os = "windows")]
                return "CPCEPower_SDL_x64.exe";
                #[cfg(target_os = "linux")]
                return "CPCEPower_SDL_Linux_x64";
                #[cfg(target_os = "macosx")]
                return "CPCEPower_SDL_MacOS";
            }
        }
    }

    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl StaticInformation for CpcEmuPowerVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        match self {
            CpcEmuPowerVersion::R20210531 => {
                static URLS: OnceLock<MutiplatformUrls> = OnceLock::new();
                URLS.get_or_init(|| MutiplatformUrls::unique_url(URL))
            }
        }
    }
}

impl DownloadableInformation for CpcEmuPowerVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }

    #[cfg(target_os = "linux")]
    fn target_os_postinstall<E: cpclib_common::event::EventObserver>(
        &self
    ) -> Option<crate::delegated::PostInstall<E>> {
        use cpclib_common::camino::Utf8PathBuf;
        use ureq::post;

        use crate::delegated::DelegateApplicationDescription;

        let fname = self.target_os_exec_fname().to_owned();
        let fname = Utf8PathBuf::from(fname);

        let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
            Box::new(move |d: &DelegateApplicationDescription<E>| {
                use std::os::unix::fs::PermissionsExt;

                let fname = d.cache_folder().join(&fname);

                let mut perms = std::fs::metadata(&fname)
                    .map_err(|e| {
                        format!(
                            "Error when setting execution rights to {}. {}",
                            &fname,
                            e.to_string()
                        )
                    })?
                    .permissions();

                let mode = perms.mode() | 0o100; // Add execution mode
                perms.set_mode(mode);
                std::fs::set_permissions(&fname, perms).map_err(|e| e.to_string())?;

                Ok(())
            });

        Some(post_install.into())
    }

    // fn target_os_postinstall<E: EventObserver>(&self) -> Option<crate::delegated::PostInstall<E>> {
    // let owned_original = match self {
    // CpcEmuPowerVersion::R20210531 => {
    // "CPC_AMSpiriT_RC_v1.01_Win_x64/Amspirit v1.01_RC_x64.exe".to_owned()
    // },
    // };
    // let owned_result = self.target_os_exec_fname().to_owned();
    //
    // let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
    // Box::new(move |d: &DelegateApplicationDescription<E>| {
    // std::fs::rename(
    // d.cache_folder().join(&owned_original),
    // d.cache_folder().join(&owned_result)
    // )
    // .map_err(|e| e.to_string())
    // });
    //
    // Some(post_install.into())
    // }
}

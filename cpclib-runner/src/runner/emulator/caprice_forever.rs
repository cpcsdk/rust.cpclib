use std::sync::OnceLock;

use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation,
    InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation
};
use crate::runner::runner::RunInDir;

pub const CAPRICEFOREVER_CMD: &str = "caprice";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum CapriceForeverVersion {
    #[default]
    V25_3
}

impl InternetStaticCompiledApplication for CapriceForeverVersion {}

impl ExecutableInformation for CapriceForeverVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V25_3 => "CapriceForever_25.3"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        match self {
            Self::V25_3 => {
                #[cfg(target_os = "windows")]
                return "Caprice64.exe";
                #[cfg(target_os = "linux")]
                return "Caprice64.exe";
                #[cfg(target_os = "macosx")]
                unreachable!();
            }
        }
    }

    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl StaticInformation for CapriceForeverVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        match self {
            CapriceForeverVersion::V25_3 => {
                let url = "https://www.cpc-power.com/cpcarchives/download/Emulateurs/%5BWin64%5D%20Caprice_Forever_v25.3.7z";
                static URLS: OnceLock<MutiplatformUrls> = OnceLock::new();
                URLS.get_or_init(|| MutiplatformUrls::builder().windows(url).linux(url).build())
            }
        }
    }
}

impl DownloadableInformation for CapriceForeverVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::SevenZ
    }

    // fn target_os_postinstall<E: EventObserver>(&self) -> Option<crate::delegated::PostInstall<E>> {
    // let owned_original = match self {
    // CapriceForeverVersion::R20210531 => {
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

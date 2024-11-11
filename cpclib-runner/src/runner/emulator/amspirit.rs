use std::sync::OnceLock;

use cpclib_common::event::EventObserver;

use crate::{delegated::{ArchiveFormat, DelegateApplicationDescription, DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication, MutiplatformUrls, StaticInformation}, runner::runner::RunInDir};

pub const AMSPIRIT_CMD: &str = "amspirit";


#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum AmspiritVersion {
    #[default]
    Rc1_01
}

impl InternetStaticCompiledApplication for AmspiritVersion {
    
}

impl ExecutableInformation for AmspiritVersion {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::Rc1_01 => "CPC_AMSpiriT_RC_v1.01_Win_x64"

        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        match self {
            Self::Rc1_01 => "CPC_AMSpiriT_RC_v1.01_Win_x64/Amspirit_v1.01_RC_x64.exe"
        }
    }

    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl StaticInformation for AmspiritVersion {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        match self {
            AmspiritVersion::Rc1_01 => {
                static URLS: OnceLock<MutiplatformUrls> = OnceLock::new();
                URLS.get_or_init(|| MutiplatformUrls::unique_url("https://www.amspirit.fr/content/files/2024/04/CPC_AMSpiriT_RC_v1.01_Win_x64.7z"))
            },
        }
    }
}

impl DownloadableInformation for AmspiritVersion {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        ArchiveFormat::SevenZ
    }

    fn target_os_postinstall<E:EventObserver + 'static>(&self) -> Option<crate::delegated::PostInstall<E>> {
        let owned_original = match self {
            AmspiritVersion::Rc1_01 => "CPC_AMSpiriT_RC_v1.01_Win_x64/Amspirit v1.01_RC_x64.exe".to_owned(),
        };
        let owned_result = self.target_os_exec_fname().to_owned();

        let post_install: Box<
            dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
        > = Box::new(move |d: &DelegateApplicationDescription<E>| {
            std::fs::rename(
                d.cache_folder().join(&owned_original),
                d.cache_folder().join(&owned_result)
            )
            .map_err(|e| e.to_string())
        });

        Some(post_install.into())
    }
}

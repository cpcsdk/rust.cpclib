use cpclib_common::event::EventObserver;

use crate::{delegated::{ArchiveFormat, DelegateApplicationDescription}, runner::runner::RunInDir};

pub const AMSPIRIT_CMD: &str = "amspirit";


#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum AmspiritVersion {
    #[default]
    Rc1_01
}



impl AmspiritVersion {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        match self {
            Self::Rc1_01 => {
                let original_fname = "CPC_AMSpiriT_RC_v1.01_Win_x64/Amspirit v1.01_RC_x64.exe";
                static MODIFIED_FNAME: &'static str =
                    "CPC_AMSpiriT_RC_v1.01_Win_x64/Amspirit_v1.01_RC_x64.exe";
                assert!(!MODIFIED_FNAME.contains(" "));

                let owned_original = original_fname.to_owned();
                let post_install: Box<
                    dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
                > = Box::new(move |d: &DelegateApplicationDescription<E>| {
                    std::fs::rename(
                        d.cache_folder().join(&owned_original),
                        d.cache_folder().join(MODIFIED_FNAME.to_owned())
                    )
                    .map_err(|e| e.to_string())
                });

                DelegateApplicationDescription::builder()
                    .download_fn_url("https://www.amspirit.fr/content/files/2024/04/CPC_AMSpiriT_RC_v1.01_Win_x64.7z")
                    .folder("CPC_AMSpiriT_RC_v1.01_Win_x64")
                    .archive_format(ArchiveFormat::SevenZ)
                    .exec_fname(MODIFIED_FNAME)
                    .in_dir(RunInDir::AppDir)
                    .post_install(post_install)
                    .build()
            }
        }
    }
}

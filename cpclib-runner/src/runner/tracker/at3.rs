use std::fmt::Display;
use std::sync::OnceLock;

use crate::delegated::{
    DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication,
    MutiplatformUrls, StaticInformation
};

pub const AT_CMD: &str = "at3";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum At3Version {
    #[default]
    V3_2_3
}

impl Display for At3Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            At3Version::V3_2_3 => "v3.2.3"
        };

        write!(f, "{v}")
    }
}

impl StaticInformation for At3Version {
    fn static_download_urls(&self) -> &'static crate::delegated::MutiplatformUrls {
        static URL: OnceLock<MutiplatformUrls> = OnceLock::new();

        URL.get_or_init(|| {
		MutiplatformUrls::builder()
			.linux("https://bitbucket.org/JulienNevo/arkostracker3/downloads/ArkosTracker-linux64-3.2.3.zip")
			.windows("https://bitbucket.org/JulienNevo/arkostracker3/downloads/ArkosTracker-windows-3.2.3.zip")
			.macos("https://bitbucket.org/JulienNevo/arkostracker3/downloads/ArkosTracker-macos-3.2.3.zip")
			.build()
		})
    }
}

impl DownloadableInformation for At3Version {
    fn target_os_archive_format(&self) -> crate::delegated::ArchiveFormat {
        crate::delegated::ArchiveFormat::Zip
    }
}

impl ExecutableInformation for At3Version {
    fn target_os_folder(&self) -> &'static str {
        static FOLDER: OnceLock<String> = OnceLock::new();
        FOLDER.get_or_init(|| format!("at3_{}", self)).as_str()
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(not(target_os = "windows"))]
        return "ArkosTracker3";
        #[cfg(target_os = "windows")]
        return "ArkosTracker3.exe";
    }
}

impl InternetStaticCompiledApplication for At3Version {}


pub mod extra {
    use std::ops::Deref;

    use crate::runner::extra::ExtraTool;

    use super::At3Version;

    macro_rules!  generate_song_handler {
        ($($name: ident)*) => {
        $(

            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            pub struct $name(ExtraTool<At3Version>);
            impl Deref for $name {
                type Target = ExtraTool<At3Version>;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl Default for $name {
                fn default() -> Self {
                    let extra = ExtraTool::builder()
                        .tool(Default::default())
                        .target_os_exec_fname(concat!("tools/", stringify!($name)))
                        .build();

                    Self(extra)
                }
            }

            impl $name {
                pub const CMD: &'static str = stringify!($name);
            }


        )*
        };
    }

    generate_song_handler!{SongToAkg  SongToAkm  SongToAky  SongToEvents  SongToRaw  SongToSoundEffects  SongToVgm  SongToWav  SongToYm}

}
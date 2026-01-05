use std::fmt::Display;
use std::sync::OnceLock;

use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::event::EventObserver;

use crate::delegated::{
    DownloadableInformation, ExecutableInformation, InternetStaticCompiledApplication,
    MutiplatformUrls, StaticInformation
};

pub const AT_CMD: &str = "at3";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum At3Version {
    #[default]
    V3_5_1,
    V3_5_1A,
    V3_5,
    V3_4,
    V3_3,
    V3_2_7,
    V3_2_3
}

impl Display for At3Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            At3Version::V3_5_1 => "v3.5.1",
            At3Version::V3_5_1A => "v3.5.1a",
            At3Version::V3_5 => "v3.5",
            At3Version::V3_4 => "v3.4",
            At3Version::V3_3 => "v3.3",
            At3Version::V3_2_7 => "v3.2.7",
            At3Version::V3_2_3 => "v3.2.3"
        };

        write!(f, "{v}")
    }
}

impl StaticInformation for At3Version {
    fn static_download_urls(&self) -> &'static crate::delegated::MutiplatformUrls {
        match self {
            At3Version::V3_5_1 => {
                static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
                URL.get_or_init(|| {
                MutiplatformUrls::builder()
                    .linux("https://www.julien-nevo.com/arkostracker/release/3.5.1/linux64/ArkosTracker-linux64-3.5.1.zip")
                    .windows("https://www.julien-nevo.com/arkostracker/release/3.5.1/windows/ArkosTracker-windows-3.5.1.zip")
                    .macos("https://www.julien-nevo.com/arkostracker/release/3.5.1/macosx/ArkosTracker-macosx-3.5.1.zip")
                    .build()
                })
            },
            At3Version::V3_5_1A => {
                static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
                URL.get_or_init(|| {
                MutiplatformUrls::builder()
                    .linux("https://www.julien-nevo.com/arkostracker/release/3.5.1a/linux64/ArkosTracker-linux64-3.5.1a.zip")
                    .windows("https://www.julien-nevo.com/arkostracker/release/3.5.1a/windows/ArkosTracker-windows-3.5.1a.zip")
                    .macos("https://www.julien-nevo.com/arkostracker/release/3.5.1a/windows/ArkosTracker-windows-3.5.1a.zip")
                    .build()
                })
            },
            At3Version::V3_5 => {
                static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
                URL.get_or_init(|| {
                MutiplatformUrls::builder()
                    .linux("https://www.julien-nevo.com/arkostracker/release/3.5/linux64/ArkosTracker-linux64-3.5.zip")
                    .windows("https://www.julien-nevo.com/arkostracker/release/3.5/windows/ArkosTracker-windows-3.5.zip")
                    .macos("https://www.julien-nevo.com/arkostracker/release/3.5/macosx/ArkosTracker-macosx-3.5.zip")
                    .build()
                })
            },

            At3Version::V3_4 => {
                static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
                URL.get_or_init(|| {
                MutiplatformUrls::builder()
                    .linux("https://www.julien-nevo.com/arkostracker/release/3.4/linux64/ArkosTracker-linux64-3.4.zip")
                    .windows("https://www.julien-nevo.com/arkostracker/release/3.4/windows/ArkosTracker-windows-3.4.zip")
                    .macos("https://www.julien-nevo.com/arkostracker/release/3.4/macosx/ArkosTracker-macosx-3.4.zip")
                    .build()
                })
            },

            At3Version::V3_3 => {
                static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
                URL.get_or_init(|| {
                MutiplatformUrls::builder()
                    .linux("https://www.julien-nevo.com/arkostracker/release/3.3/linux64/ArkosTracker-linux64-3.3.zip")
                    .windows("https://www.julien-nevo.com/arkostracker/release/3.3/windows/ArkosTracker-windows-3.3.zip")
                    .macos("https://www.julien-nevo.com/arkostracker/release/3.3/macosx/ArkosTracker-macosx-3.3.zip")
                    .build()
                })
            },

            At3Version::V3_2_7 => {
                static URL: OnceLock<MutiplatformUrls> = OnceLock::new();
                URL.get_or_init(|| {
                MutiplatformUrls::builder()
                    .linux("https://www.julien-nevo.com/arkostracker/release/3.2.7/linux64/ArkosTracker-linux64-3.2.7.zip")
                    .windows("https://www.julien-nevo.com/arkostracker/release/3.2.7/windows/ArkosTracker-windows-3.2.7.zip")
                    .macos("https://www.julien-nevo.com/arkostracker/release/3.2.7/macosx/ArkosTracker-macosx-3.2.7.zip")
                    .build()
                })
            },
            At3Version::V3_2_3 => {
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
        FOLDER.get_or_init(|| format!("at3_{self}")).as_str()
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(not(target_os = "windows"))]
        return "ArkosTracker3";
        #[cfg(target_os = "windows")]
        return "ArkosTracker3.exe";
    }
}

impl InternetStaticCompiledApplication for At3Version {}

impl At3Version {
    pub fn akg_path<E: EventObserver>(&self) -> Utf8PathBuf {
        self.configuration::<E>()
            .cache_folder()
            .join("players")
            .join("playerAkg")
            .join("sources")
            .join("z80")
            .join("PlayerAkg.asm")
    }

    pub fn akm_path<E: EventObserver>(&self) -> Utf8PathBuf {
        self.configuration::<E>()
            .cache_folder()
            .join("players")
            .join("playerAkm")
            .join("sources")
            .join("z80")
            .join("PlayerAkm.asm")
    }

    pub fn aky_path<E: EventObserver>(&self) -> Utf8PathBuf {
        self.configuration::<E>()
            .cache_folder()
            .join("players")
            .join("playerAky")
            .join("sources")
            .join("z80")
            .join("PlayerAky.asm")
    }

    pub fn aky_stable_path<E: EventObserver>(&self) -> Utf8PathBuf {
        self.configuration::<E>()
            .cache_folder()
            .join("players")
            .join("playerAky")
            .join("sources")
            .join("z80")
            .join("PlayerAkyStabilized_CPC.asm")
    }
}

pub mod extra {
    use std::ops::Deref;

    use super::At3Version;
    use crate::runner::extra::ExtraTool;

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

    generate_song_handler! {SongToAkg  SongToAkm  SongToAky  SongToEvents  SongToRaw  SongToSoundEffects  SongToVgm  SongToWav  SongToYm  Z80Profiler}
}

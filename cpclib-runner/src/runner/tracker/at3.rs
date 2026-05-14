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

    #[cfg(target_os = "macos")]
    fn target_os_postinstall<E: cpclib_common::event::EventObserver>(
        &self
    ) -> Option<crate::delegated::PostInstall<E>> {
        use crate::delegated::DelegateApplicationDescription;
        use std::os::unix::fs::PermissionsExt;

        let post_install: Box<
            dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
        > = Box::new(|desc: &DelegateApplicationDescription<E>| {
            let folder = desc.cache_folder();

            // Remove quarantine attributes recursively (no-op if not set, safe to run)
            let _ = std::process::Command::new("xattr")
                .args(["-dr", "com.apple.quarantine", folder.as_str()])
                .output();

            // Restore execute bit and ad-hoc sign every regular file under the folder.
            // zip_extract does not always preserve Unix permissions, and on macOS
            // Gatekeeper sends SIGKILL to unsigned binaries before they can run.
            fn fix_dir(dir: &cpclib_common::camino::Utf8Path) -> Result<(), String> {
                for entry in fs_err::read_dir(dir).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    let path = entry.path();
                    let meta = entry.metadata().map_err(|e| e.to_string())?;
                    if meta.is_dir() {
                        if let Some(p) = cpclib_common::camino::Utf8Path::from_path(&path) {
                            fix_dir(p)?;
                        }
                    } else if meta.is_file() {
                        // Ensure executable bit
                        let mut perms = meta.permissions();
                        perms.set_mode(perms.mode() | 0o111);
                        fs_err::set_permissions(&path, perms).map_err(|e| e.to_string())?;

                        // Ad-hoc sign (suppresses Gatekeeper SIGKILL on unsigned binaries)
                        let _ = std::process::Command::new("codesign")
                            .args(["--sign", "-", "--force", "--preserve-metadata=entitlements"])
                            .arg(&path)
                            .output();
                    }
                }
                Ok(())
            }

            fix_dir(&folder)
        });

        Some(post_install.into())
    }
}

impl ExecutableInformation for At3Version {
    fn target_os_folder(&self) -> &'static str {
        static FOLDER: OnceLock<String> = OnceLock::new();
        FOLDER.get_or_init(|| format!("at3_{self}")).as_str()
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        return "ArkosTracker3.app/Contents/MacOS/ArkosTracker3";
        #[cfg(target_os = "windows")]
        return "ArkosTracker3.exe";
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        return "ArkosTracker3";
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

use std::fmt::Display;

// To compile:
// git clone https://github.com/Tom1975/SugarboxV2.git
// cd SugarboxV2
// cmake .
// make -j20
use crate::delegated::{
    ArchiveFormat, DownloadableInformation, ExecutableInformation, GithubCompiledApplication,
    GithubInformation, PostInstall, PostInstallFn
};
use crate::event::EventObserver;
use crate::runner::exec::RunInDir;

pub const SUGARBOX_V2_CMD: &str = "sugarbox";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum SugarBoxV2Version {
    /// v2.1.1 - the first release with a bridgeable DAP debug server
    /// (`Sugarbox/debugers/`, JSON-over-TCP) and, on Linux, a single
    /// `.AppImage` instead of a `tar.gz` (see `target_os_postinstall`'s own
    /// doc comment for why that matters beyond just packaging).
    #[default]
    V2_1_1,
    V2_0_3,
    V2_0_2
}

impl Display for SugarBoxV2Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sugarbox {}", self.version_name())
    }
}

impl DownloadableInformation for SugarBoxV2Version {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "linux")]
        if matches!(self, Self::V2_1_1) {
            // v2.1.1 ships Linux as a single `.AppImage`, not an archive -
            // see `target_os_exec_fname`/`target_os_postinstall`.
            return ArchiveFormat::Raw;
        }
        #[cfg(all(target_os = "windows", feature = "archive-7z"))]
        return ArchiveFormat::SevenZ;
        #[cfg(all(target_os = "windows", not(feature = "archive-7z")))]
        return ArchiveFormat::Zip;
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "haiku"))]
        return ArchiveFormat::TarGz;
    }

    fn target_os_postinstall<E: EventObserver>(&self) -> Option<PostInstall<E>> {
        // v2.1.1's Linux AppImage's own `AppRun` script does
        // `export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"`
        // - `$HERE/usr/lib` doesn't ship its own `libpthread.so.0` (the
        // AppImage assumes that one comes from the system), and prepending
        // it changes the dynamic linker's resolution order enough that it
        // picks up a broken snap-provided `libpthread.so.0` instead of the
        // real one on at least this environment - the exact
        // "symbol lookup error: .../snap/core20/current/lib/x86_64-linux-gnu/
        // libpthread.so.0: undefined symbol: __libc_pthread_init, version
        // GLIBC_PRIVATE" crash reported when launching from VS Code.
        // Reproduces identically running the AppImage from a plain shell too
        // (confirmed directly) - it is the AppImage's own `AppRun` wrapper at
        // fault, not anything VS-Code-specific. Running the very same binary
        // straight out of the AppImage's own embedded squashfs (bypassing
        // `AppRun` entirely) resolves every one of its ~70 shared library
        // dependencies cleanly, including whatever ends up providing pthread
        // symbols - confirmed directly (`ldd`/actually running it, both
        // clean). So: extract once at install time
        // (`--appimage-extract`, the no-FUSE-needed mode already built into
        // every AppImage) and overwrite the downloaded `.AppImage` in place
        // with a thin wrapper script that execs the extracted binary
        // directly - `exec_fname()` keeps pointing at the same path either
        // way, only what that path actually *runs* changes.
        #[cfg(target_os = "linux")]
        if matches!(self, Self::V2_1_1) {
            let post_install: Box<PostInstallFn<E>> = Box::new(|desc, _o| {
                use std::os::unix::fs::PermissionsExt;

                fn make_executable(path: &cpclib_common::camino::Utf8Path) -> Result<(), String> {
                    let mut perms =
                        fs_err::metadata(path).map_err(|e| e.to_string())?.permissions();
                    perms.set_mode(perms.mode() | 0o100);
                    fs_err::set_permissions(path, perms).map_err(|e| e.to_string())
                }

                let app_image = desc.exec_fname();
                let cache_folder = desc.cache_folder();

                make_executable(&app_image)?;

                let status = std::process::Command::new(&app_image)
                    .arg("--appimage-extract")
                    .current_dir(&cache_folder)
                    .status()
                    .map_err(|e| format!("Failed to extract the AppImage: {e}"))?;
                if !status.success() {
                    return Err(format!("AppImage extraction failed with status {status}"));
                }

                // A second, unrelated crash class: Qt prints
                // "QXcbIntegration: Cannot create platform OpenGL context,
                // neither GLX nor EGL are enabled" and then reliably
                // segfaults - even though the system's own GL stack is
                // perfectly healthy (confirmed directly: `glxinfo`/
                // `xdpyinfo` on the very same display report direct
                // rendering through a real NVIDIA GPU) and even though
                // `libEGL.so.1`/`libGLX.so.0` both `dlopen` cleanly
                // (confirmed via `strace`). Root-caused directly: Anaconda's
                // `qt-main` conda package's own activation script
                // (`etc/conda/activate.d/qt-main_activate.sh`) unconditionally
                // exports `QT_XCB_GL_INTEGRATION=none` - a real, documented
                // value meaning "disable GL integration entirely" - into
                // *every* shell with a conda environment active, which is
                // why this reproduced identically from a plain terminal and
                // from VS Code alike (nothing snap/VS-Code-specific this
                // time). Confirmed fix: force `QT_XCB_GL_INTEGRATION=xcb_glx`
                // - no more error output, no crash, a real `SugarboxV2`
                // window appears (checked with `wmctrl -l`). This must
                // unconditionally override any inherited value (not just
                // fill it in when unset, e.g. via `${VAR:-xcb_glx}`) since
                // conda's script sets a real, non-empty value.
                // The real `AppRun` also exports `QT_PLUGIN_PATH`/
                // `QT_QPA_PLATFORM_PLUGIN_PATH` (pointing at `usr/plugins`/
                // `usr/plugins/platforms`) - kept here too since it's exactly
                // what `AppRun` does and is a purely additive plugin *search*
                // path with no downside, even though on its own it wasn't
                // sufficient to fix the GLX/EGL crash above. Deliberately
                // still NOT exporting `LD_LIBRARY_PATH` - that one really did
                // break dynamic linking (the pthread/glibc crash explained
                // above), which is the entire reason this wrapper bypasses
                // `AppRun` in the first place.
                //
                // `cd "$HERE"` (also part of the real `AppRun`, per its own
                // "Fix CWD so Sugarbox finds ROM/, CONF/, etc. via
                // std::filesystem::current_path()" comment) was skipped
                // here at first, on the theory that `ExternRunner`'s own
                // `RunInDir::AppDir` might already point the child at a
                // usable directory - it does not: confirmed by a real
                // "*** ERROR LOADING Keyboard ..." failure (Sugarbox
                // resolving `Keyboards/101_keyboard_linux` against whatever
                // CWD it inherited, which is `RunInDir::AppDir`'s value -
                // the cache folder itself, one level above `squashfs-root/
                // usr/` where `Keyboards/`/`ROM/`/`CONF/` actually live).
                fs_err::write(
                    &app_image,
                    "#!/bin/sh\n\
                     HERE=\"$(dirname \"$0\")/squashfs-root/usr\"\n\
                     export QT_PLUGIN_PATH=\"$HERE/plugins\"\n\
                     export QT_QPA_PLATFORM_PLUGIN_PATH=\"$HERE/plugins/platforms\"\n\
                     export QT_XCB_GL_INTEGRATION=xcb_glx\n\
                     cd \"$HERE\" || exit 1\n\
                     exec \"$HERE/Sugarbox\" \"$@\"\n"
                )
                .map_err(|e| e.to_string())?;
                make_executable(&app_image)
            });
            return Some(post_install.into());
        }
        None
    }
}

impl ExecutableInformation for SugarBoxV2Version {
    fn target_os_folder(&self) -> &'static str {
        match self {
            SugarBoxV2Version::V2_1_1 => {
                #[cfg(target_os = "windows")]
                return "Sugarbox-2.1.1-win64/Sugarbox-2.1.1-win64";
                #[cfg(target_os = "macos")]
                return "Sugarbox-2.1.1-Darwin";
                // The AppImage is a single file, not an archive with its own
                // top-level directory - it lives directly in the cache
                // folder we name for this version.
                #[cfg(target_os = "linux")]
                return "Sugarbox-2.1.1-Linux";
                #[cfg(target_os = "haiku")]
                return "Sugarbox-2.1.1-Haiku";
            },
            SugarBoxV2Version::V2_0_3 => {
                #[cfg(target_os = "windows")]
                return "Sugarbox-2.0.3-win64/Sugarbox-2.0.3-win64";
                #[cfg(target_os = "macos")]
                return "Sugarbox-2.0.3-Darwin";
                #[cfg(target_os = "linux")]
                return "Sugarbox-2.0.3-Linux";
                #[cfg(target_os = "haiku")]
                return "Sugarbox-2.0.3-Haiku";
            },
            SugarBoxV2Version::V2_0_2 => {
                #[cfg(target_os = "windows")]
                return "Sugarbox-2.0.2-win64/Sugarbox-2.0.2-win64";
                #[cfg(target_os = "macos")]
                return "Sugarbox-2.0.2-Darwin";
                #[cfg(target_os = "linux")]
                return "Sugarbox-2.0.2-Linux";
                #[cfg(target_os = "haiku")]
                return "Sugarbox-2.0.2-Haiku";
            }
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "linux")]
        if matches!(self, Self::V2_1_1) {
            return "Sugarbox-Linux-x86_64.AppImage";
        }
        #[cfg(target_os = "windows")]
        return "Sugarbox.exe";
        #[cfg(target_os = "macos")]
        return "Sugarbox";
        #[cfg(target_os = "haiku")]
        return "sugarbox";
        #[cfg(target_os = "linux")]
        return match self {
            Self::V2_1_1 => unreachable!("handled above"),
            Self::V2_0_3 => "Sugarbox-2.0.3-Linux/Sugarbox",
            Self::V2_0_2 => "Sugarbox-2.0.2-Linux/Sugarbox"
        };
    }

    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
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
            SugarBoxV2Version::V2_1_1 => "v2.1.1",
            SugarBoxV2Version::V2_0_3 => "v2.0.3",
            SugarBoxV2Version::V2_0_2 => "v2.0.2"
        }
    }

    fn linux_key(&self) -> Option<&'static str> {
        match self {
            // No version number in this one's own filename - the release
            // itself (v2.1.1) is what pins the version, same as every other
            // key here pins to one release via `version_name`.
            Self::V2_1_1 => Some("Sugarbox-Linux-x86_64.AppImage"),
            Self::V2_0_3 => Some("Sugarbox-2.0.3-Linux.tar.gz"),
            Self::V2_0_2 => Some("Sugarbox-2.0.2-Linux.tar.gz")
        }
    }

    fn windows_key(&self) -> Option<&'static str> {
        match self {
            Self::V2_1_1 => Some("Sugarbox-2.1.1-win64.7z"),
            Self::V2_0_3 => Some("Sugarbox-2.0.3-win64.7z"),
            Self::V2_0_2 => Some("Sugarbox-2.0.2-win64.7z")
        }
    }

    fn macos_key(&self) -> Option<&'static str> {
        match self {
            Self::V2_1_1 => Some("Sugarbox-2.1.1-Darwin.tar.gz"),
            Self::V2_0_3 => Some("Sugarbox-2.0.3-Darwin.tar.gz"),
            Self::V2_0_2 => Some("Sugarbox-2.0.2-Darwin.tar.gz")
        }
    }
}

impl GithubCompiledApplication for SugarBoxV2Version {}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end regression guard for the AppImage `libpthread` crash,
    /// through the *real* launch path (`DelegatedRunner` - install-if-
    /// missing, then `ExternRunner`, the exact same route
    /// `spawn_emulator_with_args`/every real "Run in emulator" click takes)
    /// rather than a raw `Command`, so it also exercises
    /// `SNAP_LEAKED_ENV_VARS` being stripped in `runner::exec` - not just
    /// the AppImage extraction/wrapper fix in this file. Reproduces the
    /// exact command that used to crash with `symbol lookup error: .../
    /// snap/core20/current/lib/x86_64-linux-gnu/libpthread.so.0: undefined
    /// symbol: __libc_pthread_init, version GLIBC_PRIVATE` when this
    /// process's own environment carries a snap-packaged parent's
    /// `GTK_PATH` (true of this very test suite, run from a snap-packaged
    /// VS Code's integrated terminal - the same environment the original
    /// bug report came from). `#[ignore]`d like this crate's other
    /// real-download tests: downloads ~40MB and spawns a real process, too
    /// heavy for the default `cargo test` run.
    #[test]
    #[cfg(target_os = "linux")]
    #[ignore]
    fn v2_1_1_installs_and_runs_without_the_appimage_libpthread_crash() {
        use cpclib_common::event::CapturingObserver;

        use crate::runner::Runner as _;

        let version = SugarBoxV2Version::V2_1_1;
        let conf = version.configuration::<CapturingObserver>();
        let runner = crate::delegated::DelegatedRunner::new(conf, "sugarbox".to_string());
        let observer = CapturingObserver::new();

        runner
            .inner_run(&["--version"], &observer)
            .expect("launching the installed emulator failed");

        let stdout = observer.stdout_joined();
        let stderr = observer.stderr_joined();
        assert!(
            !stdout.contains("symbol lookup error") && !stderr.contains("symbol lookup error"),
            "the libpthread crash is back - stdout: {stdout}, stderr: {stderr}"
        );
    }
}

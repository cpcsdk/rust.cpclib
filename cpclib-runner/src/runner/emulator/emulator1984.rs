use std::thread::available_parallelism;
use std::process::Command;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription, MutiplatformUrls};
use crate::event::EventObserver;
use crate::runner::runner::RunInDir;

pub const EMULATOR_1984_CMD: &str = "1984";
pub const DOWNLOAD_URL_V0_4_3_LINUX: &str =
    "https://github.com/salvogendut/1984/releases/download/v0.4.3/1984-v0.4.3-linux-x86_64";
pub const DOWNLOAD_URL_V0_4_3_WINDOWS: &str =
    "https://github.com/salvogendut/1984/releases/download/v0.4.3/1984-v0.4.3-windows-x86_64.zip";
pub const DOWNLOAD_URL_V0_4_3_SOURCE: &str =
    "https://github.com/salvogendut/1984/archive/refs/tags/v0.4.3.zip";

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Emulator1984Version {
    #[default]
    V0_4_3
}

impl Emulator1984Version {
    pub fn get_command(&self) -> &str {
        EMULATOR_1984_CMD
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let folder = match self {
            Self::V0_4_3 => "1984_0.4.3"
        };

        #[cfg(target_os = "windows")]
        let url = DOWNLOAD_URL_V0_4_3_WINDOWS;

        #[cfg(target_os = "linux")]
        let url = DOWNLOAD_URL_V0_4_3_LINUX;

        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        let url = DOWNLOAD_URL_V0_4_3_SOURCE;

        #[cfg(target_os = "windows")]
        let exec = "1984.exe";
        #[cfg(target_os = "linux")]
        let exec = "1984";
        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        let exec = "1984";

        #[cfg(target_os = "windows")]
        let archive_format = ArchiveFormat::Zip;
        #[cfg(target_os = "linux")]
        let archive_format = ArchiveFormat::Raw;
        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        let archive_format = ArchiveFormat::Zip;

        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(url)
            .folder(folder)
            .archive_format(archive_format)
            .exec_fname(exec)
            .in_dir(RunInDir::AppDir);

        #[cfg(target_os = "linux")]
        let builder = {
            let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
                Box::new(|desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    use std::os::unix::fs::PermissionsExt;

                    let app_image = desc.exec_fname();
                    let mut perms = fs_err::metadata(&app_image)
                        .map_err(|e| format!("Unable to inspect {app_image}: {e}"))?
                        .permissions();
                    perms.set_mode(perms.mode() | 0o111);
                    fs_err::set_permissions(&app_image, perms)
                        .map_err(|e| format!("Unable to set execute bit on {app_image}: {e}"))
                });
            builder.post_install(post_install)
        };

        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        let builder = {
            let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
                Box::new(|desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    let source_root = locate_source_root(&desc.cache_folder())?;

                    ensure_autotools_available()?;
                    ensure_sdl3_available()?;

                    run_command("autoreconf", ["-iv"], source_root.as_std_path(), "autoreconf -iv")?;
                    run_command("./configure", [], source_root.as_std_path(), "./configure")?;

                    let jobs = available_parallelism().map(|n| n.get()).unwrap_or(1);
                    let jobs_arg = jobs.to_string();
                    run_command("make", ["-j", jobs_arg.as_str()], source_root.as_std_path(), "make -j")?;

                    let built_binary = source_root.join("1984");
                    let target_binary = desc.exec_fname();
                    if built_binary.exists() && built_binary != target_binary {
                        if target_binary.exists() {
                            fs_err::remove_file(&target_binary)
                                .or_else(|_| fs_err::remove_dir_all(&target_binary))
                                .map_err(|e| {
                                    format!(
                                        "Failed to remove existing executable {}: {}",
                                        target_binary, e
                                    )
                                })?;
                        }

                        fs_err::rename(&built_binary, &target_binary).map_err(|e| {
                            format!(
                                "Failed to move built executable from {} to {}: {}",
                                built_binary, target_binary, e
                            )
                        })?;
                    }

                    Ok(())
                });
            builder.post_install(post_install)
        };

        builder.build()
    }
}

impl crate::delegated::InternetStaticCompiledApplication for Emulator1984Version {}

impl crate::delegated::ExecutableInformation for Emulator1984Version {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V0_4_3 => "1984_0.4.3"
        }
    }

    fn target_os_exec_fname(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        return "1984.exe";

        #[cfg(target_os = "linux")]
        return "1984";

        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        return "1984";
    }

    fn target_os_run_in_dir(&self) -> RunInDir {
        RunInDir::AppDir
    }
}

impl crate::delegated::StaticInformation for Emulator1984Version {
    fn static_download_urls(&self) -> &'static MutiplatformUrls {
        static URLS: std::sync::OnceLock<MutiplatformUrls> = std::sync::OnceLock::new();

        URLS.get_or_init(|| {
            MutiplatformUrls::builder()
                .linux(DOWNLOAD_URL_V0_4_3_LINUX)
                .windows(DOWNLOAD_URL_V0_4_3_WINDOWS)
                .macos(DOWNLOAD_URL_V0_4_3_SOURCE)
                .build()
        })
    }
}

impl crate::delegated::DownloadableInformation for Emulator1984Version {
    fn target_os_archive_format(&self) -> ArchiveFormat {
        #[cfg(target_os = "windows")]
        return ArchiveFormat::Zip;

        #[cfg(target_os = "linux")]
        return ArchiveFormat::Raw;

        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        return ArchiveFormat::Zip;
    }

    #[cfg(any(target_os = "macos", target_os = "openbsd"))]
    fn target_os_postinstall<E: EventObserver>(
        &self
    ) -> Option<crate::delegated::PostInstall<E>> {
        let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
            Box::new(|desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                let source_root = locate_source_root(&desc.cache_folder())?;

                ensure_autotools_available()?;
                ensure_sdl3_available()?;

                run_command("autoreconf", ["-iv"], source_root.as_std_path(), "autoreconf -iv")?;
                run_command("./configure", [], source_root.as_std_path(), "./configure")?;

                let jobs = available_parallelism().map(|n| n.get()).unwrap_or(1);
                let jobs_arg = jobs.to_string();
                run_command("make", ["-j", jobs_arg.as_str()], source_root.as_std_path(), "make -j")?;

                let built_binary = source_root.join("1984");
                let target_binary = desc.exec_fname();
                if built_binary.exists() && built_binary != target_binary {
                    if target_binary.exists() {
                        fs_err::remove_file(&target_binary)
                            .or_else(|_| fs_err::remove_dir_all(&target_binary))
                            .map_err(|e| {
                                format!(
                                    "Failed to remove existing executable {}: {}",
                                    target_binary, e
                                )
                            })?;
                    }

                    fs_err::rename(&built_binary, &target_binary).map_err(|e| {
                        format!(
                            "Failed to move built executable from {} to {}: {}",
                            built_binary, target_binary, e
                        )
                    })?;
                }

                Ok(())
            });

        Some(post_install.into())
    }
}

#[cfg(any(target_os = "macos", target_os = "openbsd"))]
fn ensure_autotools_available() -> Result<(), String> {
    for tool in ["autoreconf", "autoconf", "automake"] {
        let status = Command::new(tool)
            .arg("--version")
            .output()
            .map_err(|_e| {
                if cfg!(target_os = "macos") {
                        "Autotools are required. Install them with: brew install automake"
                        .to_owned()
                }
                else {
                        "Autotools are required. Install them with: pkg_add automake"
                        .to_owned()
                }
            })?;

        if !status.status.success() {
            return Err(if cfg!(target_os = "macos") {
                "Autotools are required. Install them with: brew install automake"
                    .to_owned()
            }
            else {
                "Autotools are required. Install them with: pkg_add automake"
                    .to_owned()
            });
        }
    }

    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "openbsd"))]
fn ensure_sdl3_available() -> Result<(), String> {
    let status = Command::new("pkg-config")
        .args(["--exists", "sdl3"])
        .status()
        .map_err(|_e| {
            if cfg!(target_os = "macos") {
                "SDL3 is required. Install it with: brew install sdl3".to_owned()
            }
            else {
                "SDL3 is required. Install it with: pkg_add sdl3".to_owned()
            }
        })?;

    if !status.success() {
        return Err(if cfg!(target_os = "macos") {
            "SDL3 is required. Install it with: brew install sdl3".to_owned()
        }
        else {
            "SDL3 is required. Install it with: pkg_add sdl3".to_owned()
        });
    }

    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "openbsd"))]
fn locate_source_root(base: &Utf8Path) -> Result<Utf8PathBuf, String> {
    let preferred = base.join("1984-0.4.3");
    if preferred.exists() {
        return Ok(preferred);
    }

    if base.join("configure").exists() && base.join("Makefile.am").exists() {
        return Ok(base.to_owned());
    }

    let entries = fs_err::read_dir(base)
        .map_err(|e| format!("Unable to inspect extracted 1984 files in {base}: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Unable to inspect extracted 1984 files in {base}: {e}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let path = Utf8PathBuf::from_path_buf(path)
            .map_err(|p| format!("Non-UTF8 extracted path for 1984 source: {:?}", p))?;

        if path.join("configure").exists() && path.join("Makefile.am").exists() {
            return Ok(path);
        }
    }

    Err(format!("1984 source folder not found in {base}"))
}

#[cfg(any(target_os = "macos", target_os = "openbsd"))]
fn run_command<const N: usize>(
    program: &str,
    args: [&str; N],
    current_dir: &std::path::Path,
    label: &str
) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .output()
        .map_err(|e| format!("Failed to run {label} in {}: {e}", current_dir.display()))?;

    if output.status.success() {
        Ok(())
    }
    else {
        Err(format!("{label} failed: {}", String::from_utf8_lossy(&output.stderr)))
    }
}
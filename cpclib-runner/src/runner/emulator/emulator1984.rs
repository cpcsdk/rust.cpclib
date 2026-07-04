use std::process::Command;

use cpclib_common::camino::Utf8Path;

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
pub const DOWNLOAD_URL_V0_4_5_LINUX: &str =
    "https://github.com/salvogendut/1984/releases/download/v0.4.5/1984-v0.4.5-linux-x86_64";
pub const DOWNLOAD_URL_V0_4_5_WINDOWS: &str =
    "https://github.com/salvogendut/1984/releases/download/v0.4.5/1984-v0.4.5-windows-x86_64.zip";
pub const DOWNLOAD_URL_V0_4_5_SOURCE: &str =
    "https://github.com/salvogendut/1984/archive/refs/tags/v0.4.5.zip";

// ROM files required by 1984 emulator (from INSTALL.md)
// - OS_464.ROM: CPC 464 OS ROM (16 KB)
// - BASIC_1.0.ROM: CPC 464 Locomotive BASIC 1.0 (16 KB)
// - OS_6128.ROM: CPC 6128 OS ROM (16 KB)
// - BASIC_1.1.ROM: CPC 6128 Locomotive BASIC 1.1 (16 KB)
// - AMSDOS.ROM: AMSDOS disk filing system (16 KB)
pub const ROM_BASE_URL: &str = "https://raw.githubusercontent.com/salvogendut/1984/main/roms";
pub const ROM_FILES: &[&str] = &[
    "OS_464.ROM",
    "BASIC_1.0.ROM",
    "OS_6128.ROM",
    "BASIC_1.1.ROM",
    "AMSDOS.ROM"
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Emulator1984Version {
    V0_4_3,
    #[default]
    V0_4_5
}

impl Emulator1984Version {
    pub fn get_command(&self) -> &str {
        EMULATOR_1984_CMD
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let folder = match self {
            Self::V0_4_3 => "1984_0.4.3",
            Self::V0_4_5 => "1984_0.4.5"
        };

        #[cfg(target_os = "windows")]
        let url = match self {
            Self::V0_4_3 => DOWNLOAD_URL_V0_4_3_WINDOWS,
            Self::V0_4_5 => DOWNLOAD_URL_V0_4_5_WINDOWS
        };

        #[cfg(target_os = "linux")]
        let url = match self {
            Self::V0_4_3 => DOWNLOAD_URL_V0_4_3_LINUX,
            Self::V0_4_5 => DOWNLOAD_URL_V0_4_5_LINUX
        };

        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        let url = match self {
            Self::V0_4_3 => DOWNLOAD_URL_V0_4_3_SOURCE,
            Self::V0_4_5 => DOWNLOAD_URL_V0_4_5_SOURCE
        };

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
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    use std::os::unix::fs::PermissionsExt;

                    let app_image = desc.exec_fname();
                    let mut perms = fs_err::metadata(&app_image)
                        .map_err(|e| format!("Unable to inspect {app_image}: {e}"))?
                        .permissions();
                    perms.set_mode(perms.mode() | 0o111);
                    fs_err::set_permissions(&app_image, perms)
                        .map_err(|e| format!("Unable to set execute bit on {app_image}: {e}"))?;

                    // Check if SDL3 is available system-wide
                    if !is_sdl3_available_system() {
                        eprintln!("⚠️  WARNING: SDL3 is not installed on your system.");
                        eprintln!("The 1984 emulator requires SDL3 to run.");
                        eprintln!();
                        eprintln!("To install SDL3 on Ubuntu/Debian:");
                        eprintln!(
                            "  wget https://github.com/libsdl-org/SDL/releases/download/release-3.4.10/SDL3-3.4.10.tar.gz"
                        );
                        eprintln!("  tar xzf SDL3-3.4.10.tar.gz");
                        eprintln!("  cd SDL3-3.4.10");
                        eprintln!("  cmake -B build -DCMAKE_BUILD_TYPE=Release");
                        eprintln!("  cmake --build build");
                        eprintln!("  sudo cmake --install build");
                        eprintln!("  sudo ldconfig");
                        eprintln!();
                        eprintln!("Emulator downloaded but will not run until SDL3 is installed.");
                    }

                    // Download required ROM files
                    let rom_dir = desc.cache_folder();
                    download_roms(&rom_dir)?;

                    Ok(())
                }
            );
            builder.post_install(post_install)
        };

        #[cfg(any(target_os = "macos", target_os = "openbsd"))]
        let builder = {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    let source_root = locate_source_root(&desc.cache_folder())?;

                    ensure_autotools_available()?;
                    ensure_sdl3_available()?;

                    run_command(
                        "autoreconf",
                        ["-iv"],
                        source_root.as_std_path(),
                        "autoreconf -iv"
                    )?;
                    run_command("./configure", [], source_root.as_std_path(), "./configure")?;

                    let jobs = available_parallelism().map(|n| n.get()).unwrap_or(1);
                    let jobs_arg = jobs.to_string();
                    run_command(
                        "make",
                        ["-j", jobs_arg.as_str()],
                        source_root.as_std_path(),
                        "make -j"
                    )?;

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

                    // Download required ROM files
                    let rom_dir = desc.cache_folder();
                    download_roms(&rom_dir)?;

                    Ok(())
                }
            );
            builder.post_install(post_install)
        };

        #[cfg(target_os = "windows")]
        let builder = {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    // Download required ROM files
                    let rom_dir = desc.cache_folder();
                    download_roms(&rom_dir)?;

                    Ok(())
                }
            );
            builder.post_install(post_install)
        };

        builder.build()
    }
}

impl crate::delegated::InternetStaticCompiledApplication for Emulator1984Version {}

impl crate::delegated::ExecutableInformation for Emulator1984Version {
    fn target_os_folder(&self) -> &'static str {
        match self {
            Self::V0_4_3 => "1984_0.4.3",
            Self::V0_4_5 => "1984_0.4.5"
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
                .linux(DOWNLOAD_URL_V0_4_5_LINUX)
                .windows(DOWNLOAD_URL_V0_4_5_WINDOWS)
                .macos(DOWNLOAD_URL_V0_4_5_SOURCE)
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
    fn target_os_postinstall<E: EventObserver>(&self) -> Option<crate::delegated::PostInstall<E>> {
        let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
            Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    let source_root = locate_source_root(&desc.cache_folder())?;

                    ensure_autotools_available()?;
                    ensure_sdl3_available()?;

                    run_command(
                        "autoreconf",
                        ["-iv"],
                        source_root.as_std_path(),
                        "autoreconf -iv"
                    )?;
                    run_command("./configure", [], source_root.as_std_path(), "./configure")?;

                    let jobs = available_parallelism().map(|n| n.get()).unwrap_or(1);
                    let jobs_arg = jobs.to_string();
                    run_command(
                        "make",
                        ["-j", jobs_arg.as_str()],
                        source_root.as_std_path(),
                        "make -j"
                    )?;

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
                }
            );

        Some(post_install.into())
    }
}

#[cfg(any(target_os = "macos", target_os = "openbsd"))]
fn ensure_autotools_available() -> Result<(), String> {
    for tool in ["autoreconf", "autoconf", "automake"] {
        let status = Command::new(tool).arg("--version").output().map_err(|_e| {
            if cfg!(target_os = "macos") {
                "Autotools are required. Install them with: brew install automake".to_owned()
            }
            else {
                "Autotools are required. Install them with: pkg_add automake".to_owned()
            }
        })?;

        if !status.status.success() {
            return Err(if cfg!(target_os = "macos") {
                "Autotools are required. Install them with: brew install automake".to_owned()
            }
            else {
                "Autotools are required. Install them with: pkg_add automake".to_owned()
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
    let preferred = base.join("1984-0.4.5");
    if preferred.exists() {
        return Ok(preferred);
    }

    if base.join("configure").exists() && base.join("Makefile.am").exists() {
        return Ok(base.to_owned());
    }

    let entries = fs_err::read_dir(base)
        .map_err(|e| format!("Unable to inspect extracted 1984 files in {base}: {e}"))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("Unable to inspect extracted 1984 files in {base}: {e}"))?;
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
        Err(format!(
            "{label} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(target_os = "linux")]
fn is_sdl3_available_system() -> bool {
    Command::new("pkg-config")
        .args(["--exists", "sdl3"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Download required ROM files for the 1984 emulator
fn download_roms(target_dir: &Utf8Path) -> Result<(), String> {
    eprintln!("Downloading required ROM files for 1984 emulator...");

    for rom_file in ROM_FILES {
        let rom_path = target_dir.join(rom_file);

        // Skip if ROM already exists
        if rom_path.exists() {
            eprintln!("  ✓ {} already exists", rom_file);
            continue;
        }

        let rom_url = format!("{}/{}", ROM_BASE_URL, rom_file);
        eprintln!("  Downloading {}...", rom_file);

        // Try curl first, then wget
        let result = Command::new("curl")
            .args([
                "-L",
                "-o",
                rom_path.as_str(),
                &rom_url,
                "--silent",
                "--show-error"
            ])
            .status()
            .and_then(|s| {
                if s.success() {
                    Ok(())
                }
                else {
                    Err(std::io::Error::other("curl failed"))
                }
            })
            .or_else(|_| {
                Command::new("wget")
                    .args(["-q", "-O", rom_path.as_str(), &rom_url])
                    .status()
                    .and_then(|s| {
                        if s.success() {
                            Ok(())
                        }
                        else {
                            Err(std::io::Error::other("wget failed"))
                        }
                    })
            });

        match result {
            Ok(_) => eprintln!("  ✓ {} downloaded successfully", rom_file),
            Err(e) => {
                return Err(format!(
                    "Failed to download {} from {}: {}. Please install curl or wget.",
                    rom_file, rom_url, e
                ));
            }
        }
    }

    eprintln!("✓ All ROM files downloaded successfully");
    Ok(())
}

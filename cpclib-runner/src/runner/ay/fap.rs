use std::process::Command;
use std::io::Cursor;

use cpclib_common::camino::Utf8PathBuf;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const FAP_CMD: &str = "fap";
pub const DOWNLOAD_URL_V1_0: &str = "https://raw.githubusercontent.com/RenaudLottiaux/FastAyPlayer/refs/heads/main/Release/Fap-1.0.0.zip";
pub const DOWNLOAD_URL_V1_0_2: &str =
    "https://github.com/grim1z/FastAyPlayer/raw/refs/heads/dev/Release/Fap-1.0.2.zip";
pub const SOURCE_URL_V1_0_2: &str = "https://github.com/grim1z/FastAyPlayer.git";

#[derive(Default)]
pub enum FAPVersion {
    #[default]
    V1_0_2,
    V1_0_0
}

impl FAPVersion {
    pub fn get_command(&self) -> &str {
        FAP_CMD
    }
}

impl FAPVersion {
    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        #[cfg(target_os = "macos")]
        let (url, folder, exec) = match self {
            FAPVersion::V1_0_2 => (DOWNLOAD_URL_V1_0_2, "fap1.0.2", "FapCrunchMac"),
            FAPVersion::V1_0_0 => (DOWNLOAD_URL_V1_0, "fap1.0.0", "FapCrunchMac")
        };

        #[cfg(target_os = "linux")]
        let (url, folder, exec) = match self {
            FAPVersion::V1_0_2 => (DOWNLOAD_URL_V1_0_2, "fap1.0.2", "Build/FapCrunchLin"),
            FAPVersion::V1_0_0 => (DOWNLOAD_URL_V1_0, "fap1.0.0", "Build/FapCrunchLin")
        };

        #[cfg(target_os = "windows")]
        let (url, folder, exec) = match self {
            FAPVersion::V1_0_2 => {
                (DOWNLOAD_URL_V1_0_2, "fap1.0.2", "Build/FapCrunchWin.exe")
            },
            FAPVersion::V1_0_0 => (DOWNLOAD_URL_V1_0, "fap1.0.0", "Build/FapCrunchWin.exe")
        };

        let builder = DelegateApplicationDescription::builder()
            .download_fn_url(url) // we assume a modern CPU
            .folder(folder)
            .archive_format(ArchiveFormat::Zip)
            .exec_fname(exec);

        #[cfg(target_os = "macos")]
        let builder = {
            use std::os::unix::fs::PermissionsExt;

            // On macOS, build the cruncher from git tag v1.0.2.
            let compile: Box<dyn Fn(&cpclib_common::camino::Utf8Path, &E) -> Result<(), String>> =
                Box::new(|path, o| {
                    let src_dir = path.join("fastayplayer-v1.0.2-src");
                    if src_dir.exists() {
                        fs_err::remove_dir_all(&src_dir).map_err(|e| e.to_string())?;
                    }

                    o.emit_stdout(">> Clone FastAyPlayer v1.0.2 source tag\n");
                    let status = Command::new("git")
                        .arg("clone")
                        .arg("--depth")
                        .arg("1")
                        .arg("--branch")
                        .arg("v1.0.2")
                        .arg(SOURCE_URL_V1_0_2)
                        .arg(src_dir.as_str())
                        .status()
                        .map_err(|e| format!("Unable to run git clone for FastAyPlayer: {e}"))?;

                    if !status.success() {
                        return Err("Unable to clone FastAyPlayer v1.0.2 tag".to_owned());
                    }

                    let make_dir = src_dir.join("src");

                    // Upstream v1.0.2 uses <byteswap.h> which is not available on macOS.
                    // Patch the source locally before compilation.
                    let ymload = make_dir.join("FapCrunch").join("YmLoad.cpp");
                    let original = fs_err::read(&ymload).map_err(|e| e.to_string())?;
                    let old = b"#ifdef _MSC_VER\n#define bswap_16(x) _byteswap_ushort(x)\n#define bswap_32(x) _byteswap_ulong(x)\n#else\n#include <byteswap.h>\n#endif";
                    let new = b"#ifdef _MSC_VER\n#define bswap_16(x) _byteswap_ushort(x)\n#define bswap_32(x) _byteswap_ulong(x)\n#elif defined(__APPLE__)\n#include <libkern/OSByteOrder.h>\n#define bswap_16(x) OSSwapInt16(x)\n#define bswap_32(x) OSSwapInt32(x)\n#else\n#include <byteswap.h>\n#endif";
                    let pos = original
                        .windows(old.len())
                        .position(|window| window == old)
                        .ok_or_else(|| "Unable to patch FastAyPlayer byteswap block".to_owned())?;
                    let mut patched = Vec::with_capacity(original.len() - old.len() + new.len());
                    patched.extend_from_slice(&original[..pos]);
                    patched.extend_from_slice(new);
                    patched.extend_from_slice(&original[pos + old.len()..]);
                    fs_err::write(&ymload, patched).map_err(|e| e.to_string())?;

                    o.emit_stdout(">> Compile FAP cruncher from source\n");
                    let status = Command::new("make")
                        .arg("cruncher")
                        .current_dir(make_dir.as_std_path())
                        .status()
                        .map_err(|e| format!("Unable to run make for FastAyPlayer: {e}"))?;

                    if !status.success() {
                        return Err("Unable to compile FAP cruncher on macOS".to_owned());
                    }

                    let built = make_dir.join("Build").join("FapCrunchLin");
                    let target = path.join("FapCrunchMac");
                    fs_err::copy(&built, &target).map_err(|e| e.to_string())?;
                    let mut perms = fs_err::metadata(&target)
                        .map_err(|e| e.to_string())?
                        .permissions();
                    perms.set_mode(0o755);
                    fs_err::set_permissions(&target, perms).map_err(|e| e.to_string())?;

                    Ok(())
                });

            let post_install: Box<dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>> =
                Box::new(|desc: &DelegateApplicationDescription<E>| {
                    // Ensure player binaries are available from the official v1.0.2 release archive.
                    let release_zip = desc
                        .cache_folder()
                        .join("fastayplayer-v1.0.2-src")
                        .join("Release")
                        .join("Fap-1.0.2.zip");
                    let release_dir = desc.cache_folder().join("fap-release-assets");
                    if release_dir.exists() {
                        fs_err::remove_dir_all(&release_dir).map_err(|e| e.to_string())?;
                    }
                    fs_err::create_dir_all(&release_dir).map_err(|e| e.to_string())?;

                    let zip_content = fs_err::read(&release_zip).map_err(|e| e.to_string())?;
                    zip_extract::extract(Cursor::new(zip_content), release_dir.as_std_path(), true)
                        .map_err(|e| e.to_string())?;

                    let from_play = {
                        let in_build = release_dir.join("Build").join("fap-play.bin");
                        if in_build.exists() {
                            in_build
                        }
                        else {
                            release_dir.join("fap-play.bin")
                        }
                    };
                    let from_init = {
                        let in_build = release_dir.join("Build").join("fap-init.bin");
                        if in_build.exists() {
                            in_build
                        }
                        else {
                            release_dir.join("fap-init.bin")
                        }
                    };
                    fs_err::copy(from_play, desc.cache_folder().join("fap-play.bin"))
                        .map_err(|e| e.to_string())?;
                    fs_err::copy(from_init, desc.cache_folder().join("fap-init.bin"))
                        .map_err(|e| e.to_string())?;
                    Ok(())
                });

            builder.compile(compile).post_install(post_install)
        };

        builder.build()
    }

    pub fn fap_play_path<E: EventObserver>(&self) -> Utf8PathBuf {
        let cache = self.configuration::<E>().cache_folder();
        let root = cache.join("fap-play.bin");
        if root.exists() {
            root
        }
        else {
            cache.join("Build").join("fap-play.bin")
        }
    }

    pub fn fap_init_path<E: EventObserver>(&self) -> Utf8PathBuf {
        let cache = self.configuration::<E>().cache_folder();
        let root = cache.join("fap-init.bin");
        if root.exists() {
            root
        }
        else {
            cache.join("Build").join("fap-init.bin")
        }
    }
}

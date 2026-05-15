use std::fmt::Display;

#[cfg(target_os = "linux")]
use cpclib_common::camino::Utf8Path;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;
#[cfg(target_os = "linux")]
use crate::runner::ExternRunner;
#[cfg(target_os = "linux")]
use crate::runner::Runner;

pub const TWO_CDT_CMD: &str = "2cdt";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum TwoCdtVersion {
    #[default]
    V1
}

impl TwoCdtVersion {
    pub fn get_command(&self) -> &str {
        TWO_CDT_CMD
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        #[cfg(target_os = "macos")]
        {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(
                |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                    use std::process::Command;
                    use std::os::unix::fs::PermissionsExt;
                    
                    let cache_folder = desc.cache_folder();
                    let src_folder = cache_folder.join("2cdt/src");
                    
                    // Compile 2cdt using cc (standard C compiler on macOS)
                    let mut cmd = Command::new("cc");
                    cmd.current_dir(&src_folder);
                    cmd.arg("2cdt.c");
                    cmd.arg("tzxfile.c");
                    cmd.arg("opth.c");
                    cmd.arg("-o");
                    cmd.arg(cache_folder.join("2cdt.compiled"));
                    
                    let output = cmd.output()
                        .map_err(|e| format!("Failed to compile with cc: {}", e))?;
                    
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        return Err(format!("Compilation failed: {}", stderr));
                    }
                    
                    // Move the compiled binary to 2cdt/2cdt (replacing the Linux binary)
                    let compiled_binary = cache_folder.join("2cdt.compiled");
                    let target_path = cache_folder.join("2cdt/2cdt");
                    fs_err::remove_file(&target_path)
                        .ok(); // Ignore error if file doesn't exist
                    fs_err::rename(&compiled_binary, &target_path)
                        .map_err(|e| format!("Failed to move compiled binary: {}", e))?;
                    
                    // Make it executable
                    let mut perms = fs_err::metadata(&target_path)
                        .map_err(|e| e.to_string())?
                        .permissions();
                    perms.set_mode(0o755);
                    fs_err::set_permissions(&target_path, perms)
                        .map_err(|e| e.to_string())?;
                    
                    Ok(())
                }
            );

            DelegateApplicationDescription::builder()
                .download_fn_url("https://cpctech.cpcwiki.de/download/2cdt.zip")
                .folder("2cdt")
                .archive_format(ArchiveFormat::Zip)
                .exec_fname("2cdt/2cdt")
                .post_install(post_install)
                .build()
        }

        #[cfg(target_os = "windows")]
        {
            DelegateApplicationDescription::builder()
                .download_fn_url("https://cpctech.cpcwiki.de/download/2cdt.zip")
                .folder("2cdt")
                .archive_format(ArchiveFormat::Zip)
                .exec_fname("2cdt.exe")
                .build()
        }

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            DelegateApplicationDescription::builder()
                .download_fn_url("https://cpctech.cpcwiki.de/download/2cdt.zip")
                .folder("2cdt")
                .archive_format(ArchiveFormat::Zip)
                .exec_fname("2cdt")
                .build()
        }
    }
}

impl Display for TwoCdtVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "2cdt")
    }
}

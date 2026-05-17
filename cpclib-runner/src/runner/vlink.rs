use std::fmt::Display;
use std::process::Command;

use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const VLINK_CMD: &str = "vlink";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Vlink;

impl Vlink {
    pub fn get_command(&self) -> &str {
        VLINK_CMD
    }

    fn build_vlink<E: EventObserver>(desc: &DelegateApplicationDescription<E>, _archive_format: ArchiveFormat, exec_fname: &str) -> Result<(), String> {
        let cache_folder = desc.cache_folder();
        
        // The tarball extracts with a top-level 'vlink' directory
        // Inside cache_folder which is already "vlink", we get vlink/vlink
        // But we need to build in the inner vlink directory
        let src_folder = cache_folder.join("vlink");
        
        // Fallback: if vlink/vlink doesn't exist, try cache_folder directly
        let src_folder = if src_folder.join("Makefile").exists() {
            src_folder
        } else if cache_folder.join("Makefile").exists() {
            cache_folder.clone()
        } else {
            return Err("Makefile not found".to_string());
        };

        let make_status = Command::new("make")
            .current_dir(&src_folder)
            .status()
            .map_err(|e| format!("Failed to execute make: {}", e))?;

        if !make_status.success() {
            return Err("Make command failed".to_string());
        }

        let vlink_binary = src_folder.join(exec_fname);
        
        // Check if the binary exists at the build location
        if !vlink_binary.exists() {
            return Err(format!("Compiled binary not found at {:?}", vlink_binary));
        }

        // Copy the binary to cache_folder/vlink (where exec_fname expects it)
        let target_path = cache_folder.join(exec_fname);
        
        // If target is a directory (from extraction), we need to remove it or use a different approach
        // Let's just overwrite by removing then copying
        if target_path.is_dir() {
            // Source and target are the same directory, skip
            if vlink_binary.parent() == Some(&target_path) {
                // Binary is already in the right place
                return Ok(());
            }
            // Otherwise remove the directory
            fs_err::remove_dir_all(&target_path)
                .map_err(|e| format!("Failed to remove directory: {}", e))?;
        } else if target_path.exists() {
            fs_err::remove_file(&target_path)
                .map_err(|e| format!("Failed to remove file: {}", e))?;
        }

        fs_err::copy(&vlink_binary, &target_path)
            .map_err(|e| format!("Failed to copy vlink binary: {}", e))?;

        // Make the binary executable using chmod command
        #[cfg(unix)]
        {
            Command::new("chmod")
                .arg("+x")
                .arg(&target_path)
                .status()
                .map_err(|e| format!("Failed to chmod binary: {}", e))?;
        }

        Ok(())
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        #[cfg(target_os = "macos")]
        {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(|desc| Self::build_vlink(desc, ArchiveFormat::TarGz, "vlink"));

            DelegateApplicationDescription::builder()
                .download_fn_url("http://sun.hasenbraten.de/vlink/release/vlink.tar.gz")
                    .folder("vlink-build")
                .archive_format(ArchiveFormat::TarGz)
                .exec_fname("vlink/vlink")
                .post_install(post_install)
                .build()
        }

        #[cfg(target_os = "windows")]
        {
            DelegateApplicationDescription::builder()
                .download_fn_url("http://sun.hasenbraten.de/vlink/bin/rel/vlink_Win64.zip")
                    .folder("vlink-build")
                .archive_format(ArchiveFormat::Zip)
                .exec_fname("vlink.exe")
                .build()
        }

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        {
            let post_install: Box<
                dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
            > = Box::new(|desc| Self::build_vlink(desc, ArchiveFormat::TarGz, "vlink"));

            DelegateApplicationDescription::builder()
                .download_fn_url("http://sun.hasenbraten.de/vlink/release/vlink.tar.gz")
                    .folder("vlink-build")
                .archive_format(ArchiveFormat::TarGz)
                .exec_fname("vlink")
                .post_install(post_install)
                .build()
        }
    }
}

impl Display for Vlink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vlink")
    }
}
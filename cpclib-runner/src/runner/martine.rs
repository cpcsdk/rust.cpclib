use crate::delegated::{ArchiveFormat, DelegateApplicationDescription};
use crate::event::EventObserver;

pub const MARTINE_CMD: &str = "martine";

#[derive(Default)]
pub enum MartineVersion {
    #[default]
    V0_41_4,
    V0_39
}

impl MartineVersion {
    pub fn get_command(&self) -> &str {
        MARTINE_CMD
    }
}

cfg_select! {
    target_os = "linux" =>
    {
        impl MartineVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    MartineVersion::V0_41_4 =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/martine/releases/download/v0.41.4/martine-0.41.4-linux-amd64.zip")
                            .folder("martine_0_41_4")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("build/martine")
                            .build(),
                    MartineVersion::V0_39  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/martine/releases/download/v0.39/martine-0.39-linux-amd64.zip") // we assume a modern CPU
                            .folder("martine_0_39")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("martine.linux")
                            .build()
                }
            }
        }
    }
    target_os = "windows" =>
    {
        impl MartineVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    MartineVersion::V0_41_4 =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/martine/releases/download/v0.41.4/martine-0.41.4-windows-amd64.zip")
                            .folder("martine_0_41_4")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("build/martine.exe")
                            .build(),
                    MartineVersion::V0_39  =>
                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/martine/releases/download/v0.39/martine-0.39-windows-amd64.zip")
                            .folder("martine_0_39")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("martine.exe")
                            .build()
                    }
            }
        }

    }
    target_os = "macos" =>
    {
        impl MartineVersion {
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
                match self {
                    MartineVersion::V0_41_4 => {
                        let post_install: Box<
                            dyn Fn(&DelegateApplicationDescription<E>) -> Result<(), String>
                        > = Box::new(
                            |desc: &DelegateApplicationDescription<E>| -> Result<(), String> {
                                use std::os::unix::fs::PermissionsExt;

                                let real_exec = desc.cache_folder().join(
                                    "binaries/martine-darwin-amd64/martine.app/Contents/MacOS/martine"
                                );
                                let wrapper = desc.exec_fname();

                                // Ensure the downloaded binary remains executable.
                                let mut real_perms = fs_err::metadata(&real_exec)
                                    .map_err(|e| e.to_string())?
                                    .permissions();
                                real_perms.set_mode(real_perms.mode() | 0o100);
                                fs_err::set_permissions(&real_exec, real_perms)
                                    .map_err(|e| e.to_string())?;

                                                                let script = r#"#!/bin/sh
MARTINE_EXEC="$(cd "$(dirname "$0")" && pwd)/binaries/martine-darwin-amd64/martine.app/Contents/MacOS/martine"

if [ "$(uname -m)" = "arm64" ]; then
    if /usr/bin/arch -x86_64 /usr/bin/true >/dev/null 2>&1; then
        exec /usr/bin/arch -x86_64 "$MARTINE_EXEC" "$@"
    else
        echo "Martine 0.41.4 for macOS is x86_64-only. Install Rosetta 2: softwareupdate --install-rosetta --agree-to-license" >&2
        exit 1
    fi
fi

exec "$MARTINE_EXEC" "$@"
"#;

                                fs_err::write(&wrapper, script).map_err(|e| e.to_string())?;
                                let mut wrapper_perms = fs_err::metadata(&wrapper)
                                    .map_err(|e| e.to_string())?
                                    .permissions();
                                wrapper_perms.set_mode(0o755);
                                fs_err::set_permissions(&wrapper, wrapper_perms)
                                    .map_err(|e| e.to_string())?;

                                Ok(())
                            }
                        );

                        DelegateApplicationDescription::builder()
                            .download_fn_url("https://github.com/jeromelesaux/martine/releases/download/v0.41.4/martine-0.41.4-darwin-amd64.zip")
                            .folder("martine_0_41_4")
                            .archive_format(ArchiveFormat::Zip)
                            .exec_fname("build/martine")
                            .post_install(post_install)
                            .build()
                    },
                    MartineVersion::V0_39 => DelegateApplicationDescription::builder()
                        .download_fn_url("https://github.com/jeromelesaux/martine/releases/download/v0.39/martine-0.39-linux-amd64.zip")
                        .folder("martine_0_39")
                        .archive_format(ArchiveFormat::Zip)
                        .exec_fname("martine.linux")
                        .build(),
                }
            }
        }
    }
    _ => {
    }
}

//! bemu (`amsrust`): a pin-level CPC emulator written by the author of this workspace.
//!
//! It serves an AMSpiriT-Lite compatible HTTP API (`--debug-server`, see
//! [`super::amspiritlite_api`]) that the `Robot` automation reuses: memory, screenshot, keyboard,
//! media, BASIC...
//!
//! It is not downloaded: it is built from its own (private) repository. The sources are, in order:
//! - `$CPCLIB_BEMU`: an executable already built (nothing to build);
//! - `$CPCLIB_BEMU_SRC`: a checkout, where `cargo build --release` is run;
//! - a clone in the cache folder (`git@github.com:rgiot/bemu.git`), with a sibling clone of this
//!   workspace as bemu's manifest has path dependencies on `../rust.cpcdemotools`.
//!
//! Installing copies the built `amsrust` in the cache folder, like any other delegated tool.

use std::process::Command;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

use crate::delegated::{
    ArchiveFormat, Compiler, CompilerFn, DelegateApplicationDescription, UrlGenerator,
    base_cache_folder
};
use crate::event::EventObserver;

pub const BEMU_CMD: &str = "bemu";

/// The environment variable that gives an already built executable
pub const BEMU_EXE_ENV: &str = "CPCLIB_BEMU";
/// The environment variable that gives a checkout of bemu to build
pub const BEMU_SRC_ENV: &str = "CPCLIB_BEMU_SRC";

const BEMU_REPOSITORY: &str = "git@github.com:rgiot/bemu.git";
const CPCLIB_REPOSITORY: &str = "https://github.com/cpcsdk/rust.cpclib.git";

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BemuVersion {
    /// The head of the default branch of the checkout
    #[default]
    Master
}

fn exec_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "amsrust.exe"
    }
    else {
        "amsrust"
    }
}

fn run<E: EventObserver>(
    mut command: Command,
    what: &str,
    o: &E
) -> Result<(), String> {
    o.emit_stdout(&format!(">> {what}\n"));
    let output = command
        .output()
        .map_err(|e| format!("cannot run `{what}`: {e}"))?;
    o.emit_stdout(&String::from_utf8_lossy(&output.stdout));
    o.emit_stderr(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(())
    }
    else {
        Err(format!("`{what}` failed ({})", output.status))
    }
}

/// The checkout to build: `$CPCLIB_BEMU_SRC`, else a clone in the cache folder
/// (made, with the workspace it depends on, if it does not exist yet).
fn source_folder<E: EventObserver>(o: &E) -> Result<Utf8PathBuf, String> {
    if let Some(src) = std::env::var_os(BEMU_SRC_ENV) {
        let src = Utf8PathBuf::from_path_buf(src.into())
            .map_err(|p| format!("${BEMU_SRC_ENV} is not utf8: {p:?}"))?;
        if !src.join("Cargo.toml").exists() {
            return Err(format!("${BEMU_SRC_ENV} = {src} is not a bemu checkout (no Cargo.toml)"));
        }
        return Ok(src);
    }

    // bemu's Cargo.toml says `../rust.cpcdemotools/...`: both are cloned side by side
    let root = base_cache_folder().join("bemu_sources");
    let bemu = root.join("bemu");
    let cpclib = root.join("rust.cpcdemotools");
    fs_err::create_dir_all(&root).map_err(|e| e.to_string())?;
    for (url, folder) in [(BEMU_REPOSITORY, &bemu), (CPCLIB_REPOSITORY, &cpclib)] {
        let mut command = Command::new("git");
        if folder.exists() {
            command.args(["-C", folder.as_str(), "pull", "--ff-only"]);
            run(command, &format!("git pull {folder}"), o)?;
        }
        else {
            command.args(["clone", "--depth", "1", url, folder.as_str()]);
            run(command, &format!("git clone {url}"), o).map_err(|e| {
                format!("{e}. Set ${BEMU_SRC_ENV} to a checkout of bemu if you cannot clone {url}")
            })?;
        }
    }
    Ok(bemu)
}

/// Builds bemu and puts its executable in `dest`
fn install<E: EventObserver>(dest: &Utf8Path, o: &E) -> Result<(), String> {
    let target = dest.join(exec_name());

    if let Some(exe) = std::env::var_os(BEMU_EXE_ENV) {
        fs_err::copy(&exe, &target)
            .map_err(|e| format!("cannot copy ${BEMU_EXE_ENV} ({exe:?}): {e}"))?;
        return Ok(());
    }

    let src = source_folder(o)?;
    let mut command = Command::new("cargo");
    command.args(["build", "--release", "--bin", "amsrust"]).current_dir(&src);
    run(command, &format!("cargo build --release in {src}"), o)?;

    let built = src.join("target").join("release").join(exec_name());
    fs_err::copy(&built, &target).map_err(|e| format!("cannot copy {built}: {e}"))?;
    Ok(())
}

impl BemuVersion {
    pub fn get_command(&self) -> &str {
        BEMU_CMD
    }

    /// Where the ROMs given to `--rom` are written
    pub fn roms_folder(&self) -> Utf8PathBuf {
        self.configuration::<()>().cache_folder().join("roms")
    }

    pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E> {
        let install: Box<CompilerFn<E>> = Box::new(|dest, o| install(dest, o));
        let no_download: Box<dyn Fn() -> Result<String, String>> =
            Box::new(|| Err("bemu is built from its sources, not downloaded".to_owned()));
        DelegateApplicationDescription::builder()
            .download_fn_url(UrlGenerator::from(no_download))
            .folder("bemu")
            .exec_fname(exec_name())
            .archive_format(ArchiveFormat::Raw)
            .custom_install(Compiler::from(install))
            .build()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_is_installed_by_building_not_by_downloading() {
        let conf = BemuVersion::default().configuration::<()>();
        assert!(conf.custom_install.is_some());
        assert_eq!(conf.exec_fname.trim_end_matches(".exe"), "amsrust");
    }
}

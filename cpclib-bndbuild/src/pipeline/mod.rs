//! Task-machinery-agnostic building blocks for composing multi-step
//! "do a real thing end to end" pipelines (build a disc, launch an
//! emulator, ...) on top of `cpclib-bndbuild`'s existing `Task`/`execute`
//! infrastructure.
//!
//! Submodules here (e.g. [`basic_run`]) express what they need in terms of
//! the functions below and never import `crate::task::{InnerTask,
//! StandardTaskArguments, Task}` directly - so a future change to how tasks
//! are represented/dispatched only has to update this one file.

pub mod basic_run;

use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use camino_tempfile::Builder as TempBuilder;
use cpclib_disc::amsdos::AmsdosFile;

use crate::event::BndBuilderObserver;
use crate::runners::emulator::Emulator;
use crate::task::{InnerTask, StandardTaskArguments, Task};

/// Formats a fresh DSK at `dsk_path` - creates it, does not require the
/// target file to already exist (unlike `add_file_to_disc`/most other disc
/// operations).
pub fn format_disc<E: BndBuilderObserver + 'static>(
    dsk_path: &Utf8Path,
    observer: &Arc<E>
) -> Result<(), String> {
    let task: Task = InnerTask::Disc(StandardTaskArguments::new(format!(
        "{dsk_path} format -f data"
    )))
    .into();
    task.execute(observer)
}

/// Adds `file_path` to the DSK at `dsk_path`. `file_path` must already
/// carry a valid AMSDOS header if it's meant to be a BINARY/BASIC file
/// (plain ASCII files need none) - this function does not build one.
pub fn add_file_to_disc<E: BndBuilderObserver + 'static>(
    dsk_path: &Utf8Path,
    file_path: &Utf8Path,
    observer: &Arc<E>
) -> Result<(), String> {
    let task: Task = InnerTask::Disc(StandardTaskArguments::new(format!(
        "{dsk_path} add {file_path}"
    )))
    .into();
    task.execute(observer)
}

/// Launches `emulator` (by its `emucontrol` CLI name, e.g. `"ace"`) with
/// `drive_a` inserted, auto-RUNning `auto_run_file`, without blocking the
/// calling thread until the emulator window closes (fire-and-forget - see
/// this function's own implementation note on why that matters for a
/// caller running inside an async handler's blocking task).
pub fn launch_emulator_with_auto_run<E: BndBuilderObserver + 'static>(
    drive_a: &Utf8Path,
    emulator: &str,
    auto_run_file: &str,
    observer: &Arc<E>
) -> Result<(), String> {
    // --background (-B): without it, the emulator task blocks the calling
    // thread until the emulator window closes (see `emucontrol.rs`'s "For
    // non-background tasks: block until the emulator window is closed") -
    // callers of this function are expected to run it from inside a
    // `spawn_blocking` on an async handler, where blocking until the user
    // closes the emulator would hang that worker thread for the whole
    // session; fire-and-forget is the correct default here.
    let task: Task = InnerTask::Emulator(
        Emulator::EmulatorFacade,
        StandardTaskArguments::new(format!(
            "--drivea {drive_a} --emulator {emulator} --auto-run-file {auto_run_file} --background run"
        ))
    )
    .into();
    task.execute(observer)
}

/// Sanitizes `hint` into an AMSDOS-safe filename stem: uppercase, only
/// alphanumeric characters, at most 8 of them, falling back to `"PROG"` if
/// nothing survives the filter.
pub fn sanitize_amsdos_stem(hint: &str) -> String {
    let stem: String = hint
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    if stem.is_empty() {
        "PROG".to_string()
    }
    else {
        stem
    }
}

/// Writes `file`'s AMSDOS header + content to a fresh temp file, returning
/// its path (the temp file is `keep()`-ed, so it outlives this call - the
/// caller is responsible for it, same as every other temp path this module
/// hands out).
fn write_amsdos_file_to_temp(file: &AmsdosFile) -> Result<Utf8PathBuf, String> {
    let tmp = TempBuilder::new()
        .suffix(".bin")
        .tempfile()
        .map_err(|e| format!("Could not create a temp file: {e}"))?;
    let path = tmp
        .into_temp_path()
        .keep()
        .map_err(|e| format!("Could not persist temp file: {e}"))?;
    fs_err::write(&path, file.header_and_content())
        .map_err(|e| format!("Could not write the AMSDOS file: {e}"))?;
    Ok(path)
}

/// Builds a fresh DSK containing exactly `file` (writes it to a temp
/// AMSDOS-headered file, formats a fresh DSK, adds the file to it), and
/// returns the DSK's path. The one-file-per-DSK shape matches every current
/// caller (e.g. [`basic_run`]) - a multi-file variant can be added if a
/// future pipeline needs it.
pub fn build_dsk_with_single_amsdos_file<E: BndBuilderObserver + 'static>(
    file: &AmsdosFile,
    observer: &Arc<E>
) -> Result<Utf8PathBuf, String> {
    let file_path = write_amsdos_file_to_temp(file)?;

    let dsk_tmp = TempBuilder::new()
        .suffix(".dsk")
        .tempfile()
        .map_err(|e| format!("Could not create a temp DSK file: {e}"))?;
    let dsk_path = dsk_tmp
        .into_temp_path()
        .keep()
        .map_err(|e| format!("Could not persist temp DSK: {e}"))?;

    format_disc(&dsk_path, observer).map_err(|e| format!("Could not format DSK: {e}"))?;
    add_file_to_disc(&dsk_path, &file_path, observer)
        .map_err(|e| format!("Could not add file to DSK: {e}"))?;

    Ok(dsk_path)
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::*;

    /// Regression guard for a real bug: the emulator CLI's flag is
    /// `--auto-run-file` (`cpclib_runner::emucontrol::EmuCli`'s
    /// `auto_run_file` field - `--auto-run` isn't even one of its aliases,
    /// `["auto", "run", "autoRunFile"]`), but this module's first version
    /// used `--auto-run`, which `clap` rejected outright - caught only by
    /// the user actually clicking "Run in emulator" in the real editor,
    /// since every prior automated test only exercised emulators rejected
    /// *before* this string is ever built. Parses the exact argument string
    /// `launch_emulator_with_auto_run` constructs against the real `EmuCli`
    /// parser (with `cpc` prepended, matching `EmulatorFacadeRunner::
    /// inner_run`'s own convention) without ever calling `handle_arguments`
    /// (which would try to launch/install a real emulator).
    #[test]
    fn the_constructed_emulator_args_string_parses_against_the_real_cli() {
        let args_str = format!(
            "--drivea {} --emulator {} --auto-run-file {} --background run",
            "/tmp/test.dsk", "ace", "PROG.BAS"
        );
        let mut args: Vec<&str> = vec!["cpc"];
        args.extend(args_str.split_whitespace());
        let parsed = cpclib_runner::emucontrol::EmuCli::try_parse_from(args);
        assert!(parsed.is_ok(), "{parsed:?}");
    }

    #[test]
    fn sanitize_amsdos_stem_uppercases_and_strips_non_alphanumerics() {
        assert_eq!(sanitize_amsdos_stem("my-cool prog!!"), "MYCOOLPR");
        assert_eq!(sanitize_amsdos_stem("hello"), "HELLO");
    }

    #[test]
    fn sanitize_amsdos_stem_caps_at_eight_chars() {
        assert_eq!(sanitize_amsdos_stem("abcdefghijkl"), "ABCDEFGH");
    }

    #[test]
    fn sanitize_amsdos_stem_falls_back_to_prog_when_nothing_survives() {
        assert_eq!(sanitize_amsdos_stem("..."), "PROG");
        assert_eq!(sanitize_amsdos_stem(""), "PROG");
    }
}

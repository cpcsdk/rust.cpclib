//! Writes a CSL (CPC Script Language) source to a temp file and launches an
//! emulator with it - built entirely on the task-agnostic operations in the
//! parent [`super`] module, so this file has no knowledge of how
//! disc/emulator operations are actually dispatched. Unlike
//! [`super::basic_run`], no disc/wrapping step is needed: `emucontrol`'s own
//! `--csl` handling accepts the file directly, whether the target emulator
//! supports CSL natively or needs the CSL-interpreter fallback.

use std::sync::Arc;

use camino::Utf8Path;
use camino_tempfile::Builder as TempBuilder;

use crate::event::BndBuilderObserver;

pub struct CslRunOutcome {
    pub message: String,
    pub success: bool
}

fn failure(message: impl Into<String>) -> CslRunOutcome {
    CslRunOutcome {
        message: message.into(),
        success: false
    }
}

/// Writes `csl_source` to a temp `.csl` file (in the OS temp directory,
/// same as before - never inside the user's own project, since this file
/// is deliberately kept/never auto-deleted for as long as the emulator
/// might still be reading it) and launches `emulator` with it, without
/// blocking on the emulator window closing (see
/// [`super::launch_emulator_with_csl`]'s own doc comment on why -
/// `--background`/fire-and-forget, same as [`super::basic_run::
/// run_basic_in_emulator`]).
///
/// `source_dir`, when given, is the *original* `.csl` file's own parent
/// directory (its editor buffer might hold unsaved changes, so the file
/// itself can't just be handed over as-is - that's what the temp copy is
/// for) - passed through to `emucontrol::run_csl_file` as an explicit
/// base directory to resolve the script's own relative
/// `disk_insert`/`snapshot_load`/etc. paths against, *decoupled* from
/// wherever the temp copy physically lives. A CSL script that says
/// `disk_insert 'game.dsk'` means "the file next to me" - real
/// Shaker-authored scripts routinely rely on exactly this - and every
/// native-CSL-capable emulator launches with its own install directory as
/// its working directory (see `cpclib_runner`'s `RunInDir::AppDir`), so
/// without this the emulator looks for `game.dsk` next to its own binary
/// instead. `None` (an unsaved buffer with no on-disk location) leaves the
/// script's own relative paths unresolved against anything in particular,
/// an honest limit rather than a regression, since an unsaved buffer has
/// no "next to me" location to begin with.
pub fn run_csl_in_emulator<E: BndBuilderObserver + 'static>(
    csl_source: &str,
    source_dir: Option<&Utf8Path>,
    emulator: &str,
    observer: &Arc<E>
) -> CslRunOutcome {
    let tmp = match TempBuilder::new().suffix(".csl").tempfile() {
        Ok(f) => f,
        Err(e) => return failure(format!("Could not create a temp file: {e}"))
    };
    let csl_path = match tmp.into_temp_path().keep() {
        Ok(p) => p,
        Err(e) => return failure(format!("Could not persist temp CSL file: {e}"))
    };
    if let Err(e) = fs_err::write(&csl_path, csl_source) {
        return failure(format!("Could not write the CSL file: {e}"));
    }

    match super::launch_emulator_with_csl(&csl_path, source_dir, emulator, observer) {
        Ok(()) => {
            CslRunOutcome {
                message: format!("Launched {emulator} with the CSL script"),
                success: true
            }
        },
        Err(e) => failure(format!("Failed to launch emulator: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    #[test]
    fn the_constructed_emulator_args_string_parses_against_the_real_cli() {
        let args_str =
            format!("--emulator {} --csl {} --background run", "winape", "/tmp/test.csl");
        let mut args: Vec<&str> = vec!["cpc"];
        args.extend(args_str.split_whitespace());
        let parsed = cpclib_runner::emucontrol::EmuCli::try_parse_from(args);
        assert!(parsed.is_ok(), "{parsed:?}");
    }
}

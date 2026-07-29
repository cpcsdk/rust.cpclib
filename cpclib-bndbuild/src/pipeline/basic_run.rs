//! Tokenizes a Locomotive BASIC source, wraps it into a bootable DSK, and
//! launches a CPC emulator with it auto-RUN - built entirely on the
//! task-agnostic operations in the parent [`super`] module, so this file
//! has no knowledge of how disc/emulator operations are actually dispatched.

use std::sync::Arc;

use camino::Utf8PathBuf;
use cpclib_basic::BasicProgram;
use cpclib_disc::amsdos::{AmsdosFile, AmsdosFileName};

use crate::event::BndBuilderObserver;

/// The emulator backends `cpclib_runner::emucontrol` genuinely honors
/// `--auto-run` for (verified directly against
/// `EmulatorConf::args_for_emu`'s per-variant match arms in
/// `cpclib-runner/src/emucontrol.rs`) - everything else either silently
/// ignores the request (CpcEmu, Cpcec, RetroVm, Cadence) or, for
/// SugarBoxV2, panics via `unimplemented!()`. Checked before building any
/// disc/emulator operation, so a bad value fails with a clear message
/// instead of a silent no-op or a crash.
pub const SUPPORTED_AUTO_RUN_EMULATORS: &[&str] = &[
    "ace",
    "winape",
    "cpcemupower",
    "caprice",
    "emulator1984",
    "amspirit"
];

pub struct BasicRunOutcome {
    pub message: String,
    pub success: bool
}

fn failure(message: impl Into<String>) -> BasicRunOutcome {
    BasicRunOutcome {
        message: message.into(),
        success: false
    }
}

/// Tokenizes `basic_source`, wraps it into an AMSDOS-headered BASIC file
/// named from `name_hint` (sanitized by [`super::sanitize_amsdos_stem`]),
/// and builds a fresh DSK containing it via [`super::build_dsk_with_single_amsdos_file`].
/// Returns the DSK path and the AMSDOS filename actually used inside it.
/// Only the BASIC tokenization and the AMSDOS BASIC-file-type header are
/// specific to this function - everything else about "wrap one file into a
/// bootable DSK" is generic, shared machinery from the parent module.
fn build_basic_dsk<E: BndBuilderObserver + 'static>(
    basic_source: &str,
    name_hint: &str,
    observer: &Arc<E>
) -> Result<(Utf8PathBuf, String), String> {
    let prog = BasicProgram::parse(basic_source).map_err(|e| format!("BASIC parse error: {e}"))?;
    let bytes = prog.as_bytes();

    let basic_name = format!("{}.BAS", super::sanitize_amsdos_stem(name_hint));
    let fname = AmsdosFileName::try_from(basic_name.as_str())
        .map_err(|e| format!("Could not build an AMSDOS filename: {e:?}"))?;
    let file = AmsdosFile::basic_file_from_buffer(&fname, &bytes)
        .map_err(|e| format!("Could not build the AMSDOS BASIC file: {e:?}"))?;

    let dsk_path = super::build_dsk_with_single_amsdos_file(&file, observer)?;

    Ok((dsk_path, basic_name))
}

/// Tokenizes `basic_source`, builds a bootable DSK, and launches `emulator`
/// with it auto-RUN. `emulator` must be one of [`SUPPORTED_AUTO_RUN_EMULATORS`].
pub fn run_basic_in_emulator<E: BndBuilderObserver + 'static>(
    basic_source: &str,
    name_hint: &str,
    emulator: &str,
    observer: &Arc<E>
) -> BasicRunOutcome {
    if !SUPPORTED_AUTO_RUN_EMULATORS.contains(&emulator) {
        return failure(format!(
            "\"{emulator}\" does not support auto-run. Supported: {}",
            SUPPORTED_AUTO_RUN_EMULATORS.join(", ")
        ));
    }

    let (dsk_path, basic_name) = match build_basic_dsk(basic_source, name_hint, observer) {
        Ok(v) => v,
        Err(e) => return failure(e)
    };

    // See `super::launch_emulator_with_auto_run`'s own doc comment for why
    // this always runs fire-and-forget (`--background`) rather than
    // blocking until the emulator window closes - it also does a mandatory
    // install-if-missing check and, for Ace, ROM/config setup on first use,
    // which is exactly why streaming progress through `observer` matters
    // here.
    match super::launch_emulator_with_auto_run(&dsk_path, emulator, &basic_name, observer) {
        Ok(()) => {
            BasicRunOutcome {
                message: format!("Launched {emulator} with {basic_name}"),
                success: true
            }
        },
        Err(e) => failure(format!("Failed to launch emulator: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use cpclib_disc::disc::Disc;

    use super::*;

    #[derive(Debug)]
    struct TestObserver;
    impl cpclib_runner::event::EventObserver for TestObserver {
        fn emit_stdout(&self, _s: &str) {}

        fn emit_stderr(&self, _s: &str) {}
    }
    impl BndBuilderObserver for TestObserver {
        fn update(&self, _event: crate::event::BndBuilderEvent) {}
    }

    #[test]
    fn unsupported_emulator_is_rejected_before_building_anything() {
        let observer = Arc::new(TestObserver);
        let outcome = run_basic_in_emulator("10 PRINT \"HI\"", "PROG", "sugarbox", &observer);
        assert!(!outcome.success);
        assert!(outcome.message.contains("sugarbox"));
        assert!(outcome.message.contains("ace"));
    }

    #[test]
    fn cpcec_is_rejected_since_it_silently_ignores_auto_run() {
        let observer = Arc::new(TestObserver);
        let outcome = run_basic_in_emulator("10 PRINT \"HI\"", "PROG", "cpcec", &observer);
        assert!(!outcome.success);
    }

    #[test]
    fn malformed_basic_source_fails_before_touching_disc() {
        let observer = Arc::new(TestObserver);
        let err = build_basic_dsk("this is not valid basic {{{", "PROG", &observer).unwrap_err();
        assert!(err.contains("parse error"));
    }

    #[test]
    fn valid_source_produces_a_dsk_with_the_expected_amsdos_entry() {
        let observer = Arc::new(TestObserver);
        let (dsk_path, basic_name) =
            build_basic_dsk("10 PRINT \"HELLO\"", "PROG", &observer).unwrap();
        assert_eq!(basic_name, "PROG.BAS");
        let disc = cpclib_disc::open_disc(&dsk_path, true).unwrap();
        let fname = AmsdosFileName::try_from(basic_name.as_str()).unwrap();
        let file = disc
            .get_amsdos_file(cpclib_disc::edsk::Head::A, fname)
            .unwrap();
        assert!(file.is_some(), "the DSK should contain {basic_name}");
    }

    #[test]
    fn name_hint_is_sanitized_and_falls_back_to_prog() {
        let observer = Arc::new(TestObserver);
        let (_, basic_name) =
            build_basic_dsk("10 PRINT \"HI\"", "my-cool prog!!", &observer).unwrap();
        assert_eq!(basic_name, "MYCOOLPR.BAS");

        let (_, basic_name) = build_basic_dsk("10 PRINT \"HI\"", "...", &observer).unwrap();
        assert_eq!(basic_name, "PROG.BAS");
    }
}

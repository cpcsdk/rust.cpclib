//! Pure translation logic for running a parsed CSL script against an
//! emulator that has no native CSL support (see
//! [`crate::runner::emulator::Emulator::accept_csl`]) - the actual
//! spawn/keystroke-replay orchestration lives in `emucontrol.rs`, next to
//! the `Robot`/`Enigo` machinery it needs; this module only decides *what*
//! to do with each instruction, so that decision can be unit-tested without
//! a real emulator process or OS keyboard.
//!
//! # Capability ceiling
//!
//! A native `--csl` launch (see `EmulatorConf::args_for_emu`) is always
//! preferred when available - this module exists only for emulators that
//! have none. Two existing, narrower mechanisms are all that's available to
//! reconstruct CSL behavior from: `EmulatorConf`'s own per-field
//! launch-argument translation, and the `Robot`/enigo keystroke-injection
//! layer already used for autotype. Neither has any live/mid-session
//! endpoint for media, reset, or machine-config changes - those can only be
//! honored if they appear at the very start of a script, folded into launch
//! arguments. A script's instructions are split at the first
//! `wait*`/`key_output`/`key_from_file`/`keyboard_write` instruction:
//! everything before that point is "leading" (folded into launch args),
//! everything from there on is "live" (replayed after launch, one step at a
//! time). A media/config instruction that shows up in the *live* portion
//! (i.e. after the script has already started typing/waiting) has no
//! backend to execute it, and is reported as unsupported rather than
//! silently dropped or failing the whole run.

use std::time::Duration;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_csl::{CslInstruction, CslScript, CslScriptBuilder, Drive, KeyElement, SpecialKey};

use crate::emucontrol::EmulatorConf;

/// Rewrites every path-bearing instruction in `script` so a relative path
/// is resolved against `base_dir` - meant to be the `.csl` file's own
/// parent directory - rather than left for the emulator to resolve
/// against its own working directory.
///
/// That matters because every emulator with native CSL support today
/// (AMSpiriT, SugarboxV2) is launched with its **install directory** as
/// its working directory (`RunInDir::AppDir` in `emucontrol.rs`'s
/// `ExternRunner` - needed for the emulator's own resource loading, not
/// something this crate controls), not the script's location or the
/// caller's own cwd. A script that says `disk_insert 'game.dsk'` means
/// "the file next to me" - a real Shaker-authored CSL script routinely
/// does exactly this - and without this rewrite the emulator looks for
/// `game.dsk` next to its own binary instead, which is the "unable to
/// find the dsk" failure this fixes. Already-absolute paths are left
/// untouched. Called once, right after parsing, before the leading/live
/// split - so both the native (re-serialize-and-hand-off) and the
/// non-native (`fold_leading_instructions`) paths see already-resolved
/// paths.
pub fn resolve_relative_paths(script: &CslScript, base_dir: &Utf8Path) -> CslScript {
    fn resolve(base_dir: &Utf8Path, path: Utf8PathBuf) -> Utf8PathBuf {
        if path.is_absolute() {
            path
        }
        else {
            base_dir.join(path)
        }
    }

    fn rewrite(base_dir: &Utf8Path, instr: CslInstruction) -> CslInstruction {
        match instr {
            CslInstruction::DiskInsert { drive, filename } => {
                CslInstruction::DiskInsert {
                    drive,
                    filename: resolve(base_dir, filename)
                }
            },
            CslInstruction::DiskDir(p) => CslInstruction::DiskDir(resolve(base_dir, p)),
            CslInstruction::TapeInsert(p) => CslInstruction::TapeInsert(resolve(base_dir, p)),
            CslInstruction::TapeDir(p) => CslInstruction::TapeDir(resolve(base_dir, p)),
            CslInstruction::SnapshotLoad(p) => CslInstruction::SnapshotLoad(resolve(base_dir, p)),
            CslInstruction::SnapshotDir(p) => CslInstruction::SnapshotDir(resolve(base_dir, p)),
            CslInstruction::SnapshotName(p) => CslInstruction::SnapshotName(resolve(base_dir, p)),
            CslInstruction::ScreenshotName(p) => {
                CslInstruction::ScreenshotName(resolve(base_dir, p))
            },
            CslInstruction::ScreenshotDir(p) => CslInstruction::ScreenshotDir(resolve(base_dir, p)),
            CslInstruction::KeyFromFile(p) => CslInstruction::KeyFromFile(resolve(base_dir, p)),
            CslInstruction::RomDir(p) => CslInstruction::RomDir(resolve(base_dir, p)),
            CslInstruction::RomConfig(cpclib_csl::RomConfig {
                rom_type,
                num,
                filename
            }) => {
                CslInstruction::RomConfig(cpclib_csl::RomConfig {
                    rom_type,
                    num,
                    filename: resolve(base_dir, filename)
                })
            },
            CslInstruction::CslLoad(p) => CslInstruction::CslLoad(resolve(base_dir, p)),
            CslInstruction::InstructionWithComment(inner, comment) => {
                CslInstruction::InstructionWithComment(Box::new(rewrite(base_dir, *inner)), comment)
            },
            // Every other variant carries no path - passed through as-is.
            other => other
        }
    }

    let mut builder = CslScriptBuilder::new();
    for instr in script.instructions() {
        builder = builder
            .with_instruction(rewrite(base_dir, instr.clone()))
            // `script` was already a successfully-parsed/validated script -
            // rewriting a path (never a version/feature-gating concern)
            // cannot turn a valid instruction sequence into an invalid one.
            .expect("rewriting a path never invalidates an already-valid instruction");
    }
    builder
        .build()
        .expect("rewriting a path never invalidates an already-valid script")
}

/// Whether `instr` is where a script's "live" (post-launch) portion begins.
fn is_live_trigger(instr: &CslInstruction) -> bool {
    matches!(
        unwrap_comment(instr),
        CslInstruction::Wait(_)
            | CslInstruction::WaitDriveOnOff(_)
            | CslInstruction::WaitVsyncOffOn
            | CslInstruction::WaitSsm0000
            | CslInstruction::KeyDelay { .. }
            | CslInstruction::KeyOutput(_)
            | CslInstruction::KeyFromFile(_)
            | CslInstruction::KeyboardWrite(_)
    )
}

/// `key_output`/`csl_load`/etc. lines can be wrapped in
/// `InstructionWithComment` (a trailing `;comment`) - every match in this
/// module cares about the real instruction underneath, never the wrapper.
fn unwrap_comment(instr: &CslInstruction) -> &CslInstruction {
    match instr {
        CslInstruction::InstructionWithComment(inner, _) => inner,
        other => other
    }
}

/// Splits `script`'s instructions into a leading run (foldable into launch
/// arguments) and a live run (replayed after the emulator starts) - see
/// this module's own doc comment for the split rule and its rationale.
pub fn split_leading_and_live(script: &CslScript) -> (Vec<&CslInstruction>, Vec<&CslInstruction>) {
    let instructions = script.instructions();
    let split_at = instructions
        .iter()
        .position(is_live_trigger)
        .unwrap_or(instructions.len());
    (
        instructions[..split_at].iter().collect(),
        instructions[split_at..].iter().collect()
    )
}

/// Folds a script's leading instructions into `conf`'s own fields.
///
/// Only `disk_insert`/`snapshot_load` are translated: they map directly and
/// losslessly onto `EmulatorConf::drive_a`/`drive_b`/`snapshot`, the same
/// fields `args_for_emu`'s per-field match arms already build CLI args
/// from. `reset`/`crtc_select`/`memory_exp`/`cpc_model`/`gate_array`/
/// `rom_config` are deliberately **not** translated: CSL's memory/CRTC/model
/// vocabulary doesn't map cleanly onto `EmulatorConf::memory`/`crtc`
/// (different emulators interpret those CLI values differently, and
/// guessing a mapping would silently misconfigure the emulator rather than
/// honestly doing nothing) - a script that depends on those succeeding
/// against a non-native emulator isn't supported yet.
pub fn fold_leading_instructions(
    leading: &[&CslInstruction],
    mut conf: EmulatorConf
) -> EmulatorConf {
    for instr in leading {
        match unwrap_comment(instr) {
            CslInstruction::DiskInsert {
                drive: Drive::A,
                filename
            } => conf.drive_a = Some(filename.clone()),
            CslInstruction::DiskInsert {
                drive: Drive::B,
                filename
            } => conf.drive_b = Some(filename.clone()),
            CslInstruction::SnapshotLoad(path) => conf.snapshot = Some(path.clone()),
            _ => {}
        }
    }
    conf
}

/// What to do with one "live" instruction, decided without touching an
/// emulator/OS - `emucontrol.rs` executes these in order.
#[derive(Debug, Clone, PartialEq)]
pub enum CslLiveStep {
    /// Type this literal text (already flattened from `KeyOutput`'s
    /// elements - see [`key_output_to_text`]) into the running emulator.
    TypeText(String),
    /// Sleep this long, approximating an emulated-time wait as wall-clock
    /// time - no non-native backend exposes a pollable emulated clock.
    Sleep(Duration),
    /// A media/config/reset instruction appeared in the live portion of the
    /// script (i.e. after typing/waiting had already started) - no backend
    /// can act on it mid-session. Carries a human-readable description for
    /// the caller to log.
    Unsupported(String)
}

/// `key_delay`'s microsecond fields are per the CSL spec's own units,
/// matching `wait`'s.
fn micros(n: u64) -> Duration {
    Duration::from_micros(n)
}

/// Flattens a `KeyOutput`'s elements into literal text for
/// `Robot::handle_raw_text` - which only understands plain characters and
/// the literal two-char sequence `\n` for Return, nothing about CSL's
/// special-key escapes. `SpecialKey::Return`/`Enter` become a real newline;
/// every other special key and every `Simultaneous` chord (e.g. Ctrl+C) has
/// no plain-text equivalent and is dropped - the returned `bool` says
/// whether anything was dropped, so the caller can report reduced fidelity
/// instead of silently mistyping.
pub fn key_output_to_text(elements: &[KeyElement]) -> (String, bool) {
    let mut text = String::new();
    let mut dropped_anything = false;
    for element in elements {
        match element {
            KeyElement::Character(c) => text.push(*c),
            KeyElement::Special(SpecialKey::Return | SpecialKey::Enter) => text.push('\n'),
            KeyElement::Special(_) | KeyElement::Simultaneous(_) => dropped_anything = true
        }
    }
    (text, dropped_anything)
}

/// Describes one CSL instruction for [`CslLiveStep::Unsupported`]'s
/// message: just enough to locate it in the script (its keyword), no full
/// re-serialization needed.
fn describe(instr: &CslInstruction) -> String {
    instr.instruction_name().to_string()
}

/// Translates a script's live instructions into an ordered plan of steps -
/// pure decision-making, no I/O; `emucontrol.rs` walks the result and does
/// the actual typing/sleeping/logging.
pub fn plan_live_steps(live: &[&CslInstruction]) -> Vec<CslLiveStep> {
    live.iter()
        .flat_map(|instr| -> Vec<CslLiveStep> {
            match unwrap_comment(instr) {
                CslInstruction::KeyOutput(output) => {
                    let (text, dropped) = key_output_to_text(output.elements());
                    if dropped {
                        // Still type what could be typed - partial fidelity
                        // beats nothing - but flag that some special
                        // keys/chords on this line had no plain-text
                        // equivalent and were dropped.
                        vec![
                            CslLiveStep::TypeText(text),
                            CslLiveStep::Unsupported(
                                "key_output: one or more special keys/chords in this line have \
                                 no plain-text equivalent and were dropped"
                                    .to_string()
                            ),
                        ]
                    }
                    else {
                        vec![CslLiveStep::TypeText(text)]
                    }
                },
                CslInstruction::KeyFromFile(_) => {
                    // The file's content isn't read here (this module has
                    // no filesystem access by design) - `emucontrol.rs`
                    // reads it and substitutes a `TypeText` step itself.
                    vec![CslLiveStep::Unsupported(
                        "key_from_file (resolve the file's content before replay)".to_string()
                    )]
                },
                CslInstruction::Wait(n) => vec![CslLiveStep::Sleep(micros(*n))],
                CslInstruction::KeyDelay { press_delay, .. } => {
                    vec![CslLiveStep::Sleep(micros(*press_delay))]
                },
                CslInstruction::WaitDriveOnOff(_)
                | CslInstruction::WaitVsyncOffOn
                | CslInstruction::WaitSsm0000 => {
                    // No non-native backend exposes drive/vsync/SSM state to
                    // poll - approximate with a fixed, conservative pause
                    // rather than not waiting at all.
                    vec![CslLiveStep::Sleep(Duration::from_millis(500))]
                },
                other => vec![CslLiveStep::Unsupported(describe(other))]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use cpclib_common::camino::Utf8PathBuf;
    use cpclib_csl::{CslInstruction, KeyOutput};

    use super::*;

    fn conf() -> EmulatorConf {
        EmulatorConf::builder()
            .break_on_bad_vbl(false)
            .break_on_bad_hbl(false)
            .transparent(false)
            .build()
    }

    #[test]
    fn resolve_relative_paths_joins_a_relative_disk_insert_to_the_base_dir() {
        let script = cpclib_csl::CslScriptBuilder::new()
            .with_instruction(CslInstruction::disk_insert(Drive::A, "shaker26.dsk".into()))
            .unwrap()
            .build()
            .unwrap();
        let resolved = resolve_relative_paths(&script, Utf8Path::new("/scripts/MODULE A"));
        match &resolved.instructions()[0] {
            CslInstruction::DiskInsert { filename, .. } => {
                assert_eq!(filename, &Utf8PathBuf::from("/scripts/MODULE A/shaker26.dsk"));
            },
            other => panic!("expected DiskInsert, got {other:?}")
        }
    }

    #[test]
    fn resolve_relative_paths_leaves_an_already_absolute_path_untouched() {
        let script = cpclib_csl::CslScriptBuilder::new()
            .with_instruction(CslInstruction::SnapshotLoad("/elsewhere/game.sna".into()))
            .unwrap()
            .build()
            .unwrap();
        let resolved = resolve_relative_paths(&script, Utf8Path::new("/scripts"));
        match &resolved.instructions()[0] {
            CslInstruction::SnapshotLoad(p) => {
                assert_eq!(p, &Utf8PathBuf::from("/elsewhere/game.sna"));
            },
            other => panic!("expected SnapshotLoad, got {other:?}")
        }
    }

    #[test]
    fn resolve_relative_paths_recurses_through_a_trailing_comment_wrapper() {
        let script = cpclib_csl::CslScriptBuilder::new()
            .with_instruction(CslInstruction::InstructionWithComment(
                Box::new(CslInstruction::disk_insert(Drive::A, "game.dsk".into())),
                " a comment".to_string()
            ))
            .unwrap()
            .build()
            .unwrap();
        let resolved = resolve_relative_paths(&script, Utf8Path::new("/scripts"));
        match &resolved.instructions()[0] {
            CslInstruction::InstructionWithComment(inner, comment) => {
                assert_eq!(comment, " a comment");
                match inner.as_ref() {
                    CslInstruction::DiskInsert { filename, .. } => {
                        assert_eq!(filename, &Utf8PathBuf::from("/scripts/game.dsk"));
                    },
                    other => panic!("expected DiskInsert, got {other:?}")
                }
            },
            other => panic!("expected InstructionWithComment, got {other:?}")
        }
    }

    #[test]
    fn resolve_relative_paths_leaves_path_free_instructions_alone() {
        let script = cpclib_csl::CslScriptBuilder::new()
            .with_instruction(CslInstruction::Wait(1000))
            .unwrap()
            .build()
            .unwrap();
        let resolved = resolve_relative_paths(&script, Utf8Path::new("/scripts"));
        assert!(matches!(resolved.instructions()[0], CslInstruction::Wait(1000)));
    }

    #[test]
    fn split_puts_leading_config_before_the_first_wait() {
        let script = cpclib_csl::CslScriptBuilder::new()
            .with_instruction(CslInstruction::disk_insert(Drive::A, "TEST.DSK".into()))
            .unwrap()
            .with_instruction(CslInstruction::Reset(cpclib_csl::ResetType::Soft))
            .unwrap()
            .with_instruction(CslInstruction::Wait(1000))
            .unwrap()
            .with_instruction(CslInstruction::key_output(KeyOutput::try_from("RUN").unwrap()))
            .unwrap()
            .build()
            .unwrap();

        let (leading, live) = split_leading_and_live(&script);
        assert_eq!(leading.len(), 2);
        assert_eq!(live.len(), 2);
        assert!(matches!(live[0], CslInstruction::Wait(1000)));
    }

    #[test]
    fn a_script_with_no_live_trigger_is_entirely_leading() {
        let script = cpclib_csl::CslScriptBuilder::new()
            .with_instruction(CslInstruction::disk_insert(Drive::A, "TEST.DSK".into()))
            .unwrap()
            .build()
            .unwrap();
        let (leading, live) = split_leading_and_live(&script);
        assert_eq!(leading.len(), 1);
        assert!(live.is_empty());
    }

    #[test]
    fn fold_leading_sets_disk_and_snapshot_fields() {
        let leading = vec![
            CslInstruction::disk_insert(Drive::A, "A.DSK".into()),
            CslInstruction::disk_insert(Drive::B, "B.DSK".into()),
            CslInstruction::SnapshotLoad(Utf8PathBuf::from("game.sna")),
        ];
        let refs: Vec<&CslInstruction> = leading.iter().collect();
        let folded = fold_leading_instructions(&refs, conf());
        assert_eq!(folded.drive_a, Some(Utf8PathBuf::from("A.DSK")));
        assert_eq!(folded.drive_b, Some(Utf8PathBuf::from("B.DSK")));
        assert_eq!(folded.snapshot, Some(Utf8PathBuf::from("game.sna")));
    }

    #[test]
    fn fold_leading_ignores_untranslatable_config_instructions() {
        let leading = vec![CslInstruction::Reset(cpclib_csl::ResetType::Hard)];
        let refs: Vec<&CslInstruction> = leading.iter().collect();
        let folded = fold_leading_instructions(&refs, conf());
        assert_eq!(folded.drive_a, None);
    }

    #[test]
    fn key_output_return_becomes_a_newline() {
        let elements = vec![
            KeyElement::Character('R'),
            KeyElement::Character('U'),
            KeyElement::Character('N'),
            KeyElement::Special(SpecialKey::Return),
        ];
        let (text, dropped) = key_output_to_text(&elements);
        assert_eq!(text, "RUN\n");
        assert!(!dropped);
    }

    #[test]
    fn key_output_untranslatable_special_key_is_reported_as_dropped() {
        let elements = vec![
            KeyElement::Character('A'),
            KeyElement::Special(SpecialKey::Esc),
        ];
        let (text, dropped) = key_output_to_text(&elements);
        assert_eq!(text, "A");
        assert!(dropped);
    }

    #[test]
    fn plan_live_steps_translates_wait_and_key_output() {
        let wait = CslInstruction::Wait(2000);
        let key = CslInstruction::key_output(KeyOutput::try_from("RUN").unwrap());
        let live = vec![&wait, &key];
        let plan = plan_live_steps(&live);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0], CslLiveStep::Sleep(Duration::from_micros(2000)));
        assert_eq!(plan[1], CslLiveStep::TypeText("RUN".to_string()));
    }

    #[test]
    fn plan_live_steps_reports_mid_script_media_changes_as_unsupported() {
        let key = CslInstruction::key_output(KeyOutput::try_from("A").unwrap());
        let disk = CslInstruction::disk_insert(Drive::A, "SWAP.DSK".into());
        let live = vec![&key, &disk];
        let plan = plan_live_steps(&live);
        assert_eq!(plan[0], CslLiveStep::TypeText("A".to_string()));
        assert!(matches!(&plan[1], CslLiveStep::Unsupported(msg) if msg.contains("disk_insert")));
    }
}

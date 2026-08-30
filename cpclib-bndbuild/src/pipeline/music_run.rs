//! Converts an Arkos Tracker source song into a standalone AKG-player
//! program, then either launches it in an emulator or packs it onto a DSK -
//! built entirely on the task-agnostic operations in the parent [`super`]
//! module (plus [`super::convert_song_to_akg`]/[`super::assemble_source`]),
//! so this file has no knowledge of how the conversion/assembler tasks are
//! actually dispatched.
//!
//! The design (harness shape, `SongToAkg` flags) is a direct Rust port of
//! the working Python pipeline in
//! <https://github.com/cpcsdk/amstrad_cpc_players_comparison>
//! (`music_play.py`/`players.py`/`players/akg/akg.asm`), not a fresh design -
//! with one deliberate difference: the reference harness resolves its paths
//! through basm `-D`-defined symbols used bare (e.g. `incbin
//! MUSIC_DATA_FNAME`); real testing against this codebase's basm showed that
//! is read as the literal text `MUSIC_DATA_FNAME` as a filename, not
//! dereferenced - so this port instead substitutes real, already-escaped
//! paths into `{{PLACEHOLDER}}` markers in the harness text itself before
//! it is ever handed to basm (see `music_akg_harness.asm`).

use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use cpclib_disc::amsdos::{AmsdosFile, AmsdosFileName};
use cpclib_runner::runner::tracker::at3::At3Version;

use crate::event::BndBuilderObserver;

/// Arkos-Tracker-compatible source-file extensions (lower-case, no leading
/// dot). The canonical list - kept here rather than in `cpclib-project`'s
/// `MusicConfig` (which depends on this crate, not the other way around) so
/// there is exactly one place that defines it. Verified against the real
/// formats Arkos Tracker 3 can import (its own "Import from..." feature
/// list): AKS (Arkos Tracker 1/2/3), SKS (STarKos), 128 (BSC's Soundtrakker),
/// VT2 (Vortex Tracker 2), WYZ (Wyz Tracker).
pub const DEFAULT_SONG_EXTENSIONS: &[&str] = &["aks", "sks", "128", "vt2", "wyz"];

/// The AKG player-harness source, embedded at compile time - see
/// `music_akg_harness.asm` next to this file for the full commented source
/// and the `{{PLACEHOLDER}}`s it expects substituted before assembling.
const AKG_HARNESS_SOURCE: &str = include_str!("music_akg_harness.asm");

pub struct MusicRunOutcome {
    pub message: String,
    pub success: bool
}

fn failure(message: impl Into<String>) -> MusicRunOutcome {
    MusicRunOutcome {
        message: message.into(),
        success: false
    }
}

/// The path text to drop into the harness's `"{{PLACEHOLDER}}"` markers -
/// already inside a quoted string literal in the template, so this only
/// escapes what a basm string literal needs escaped, it does not add quotes.
#[cfg(not(target_os = "windows"))]
fn basm_escaped_path(path: &Utf8Path) -> String {
    path.to_string()
}

#[cfg(target_os = "windows")]
fn basm_escaped_path(path: &Utf8Path) -> String {
    // basm's string literals treat `\` as an escape character, same reason
    // `cpclib_bndbuild::env::create_template_env`'s `basm_escape_path` jinja
    // filter exists for `.bnd` templates - this is the same fix for a path
    // built directly in Rust instead of through a template.
    path.as_str().replace('\\', "\\\\")
}

/// One assembled run's working files, all inside a fresh temp directory so
/// concurrent runs (and repeated runs of the same song) never collide.
///
/// `bin_path` is a **headerless** raw memory dump (`0x500..$`, i.e. `load
/// address..end of everything assembled`) - real testing found that basm's
/// own `SAVE ..., AMSDOS` directive silently writes nothing when its target
/// is a bare host path rather than a path inside a `.dsk` (a real,
/// reproducible bug in the version this was built against, isolated with a
/// standalone repro against `cpclib-basm/tests/asm/good_save.asm`'s own
/// `hello.bin` case), so the AMSDOS header is instead built in Rust, from
/// `bin_name`, with `binary_file_from_buffer`. Both addresses are `0x500`:
/// the harness's `jp Start` at its very first byte (see
/// `music_akg_harness.asm`) means the load address doubles as the entry
/// point, without needing this code to know `Start`'s real address.
struct Build {
    _dir: camino_tempfile::Utf8TempDir,
    bin_path: Utf8PathBuf,
    bin_name: String
}

/// The AMSDOS binary's load/execution address - see `Build`'s doc comment.
const HARNESS_LOAD_ADDRESS: u16 = 0x500;

/// Converts `song_path` and assembles the AKG harness around it, naming the
/// resulting AMSDOS binary from `name_hint` (sanitized the same way
/// `basic_run` names its `.BAS` file). `extra_asm_args` lets the two public
/// entry points below differ only in whether they also ask for a snapshot -
/// everything else (conversion, path substitution, harness source) is
/// identical, matching how the Python reference's
/// `__build_replay_program__` builds both in one assemble call.
fn convert_and_assemble<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    extra_asm_args: &[String],
    observer: &Arc<E>
) -> Result<Build, String> {
    let dir = camino_tempfile::tempdir()
        .map_err(|e| format!("Could not create a temp working directory: {e}"))?;

    let akg_path = dir.path().join("song.akg");
    // AT3's own naming convention for `--exportPlayerConfig`'s companion
    // file: `output_path` with its extension stripped, `_playerconfig.asm`
    // appended - see `super::convert_song_to_akg`'s doc comment.
    let player_config_path =
        Utf8PathBuf::from(format!("{}_playerconfig.asm", akg_path.with_extension("")));
    let bin_name = format!("{}.BIN", super::sanitize_amsdos_stem(name_hint));
    let bin_path = dir.path().join(&bin_name);
    let player_source_path = At3Version::default().akg_path::<()>();

    let harness_source = AKG_HARNESS_SOURCE
        .replace(
            "{{MUSIC_DATA_FNAME}}",
            &basm_escaped_path(&akg_path)
        )
        .replace(
            "{{PLAYER_CONFIG_FNAME}}",
            &basm_escaped_path(&player_config_path)
        )
        .replace(
            "{{PLAYER_SOURCE_FNAME}}",
            &basm_escaped_path(&player_source_path)
        )
        .replace("{{MUSIC_EXEC_FNAME}}", &basm_escaped_path(&bin_path));
    let harness_path = dir.path().join("harness.asm");
    fs_err::write(&harness_path, harness_source)
        .map_err(|e| format!("Could not write the player harness: {e}"))?;

    super::convert_song_to_akg(song_path, &akg_path, observer)
        .map_err(|e| format!("Could not convert {song_path} to AKG: {e}"))?;

    super::assemble_source(&harness_path, extra_asm_args, observer)
        .map_err(|e| format!("Could not assemble the player harness: {e}"))?;

    Ok(Build {
        _dir: dir,
        bin_path,
        bin_name
    })
}

/// Converts `song_path`, assembles it into a standalone AKG player, and
/// launches `emulator` on the resulting snapshot - built in the same
/// assemble pass as the AMSDOS binary (`--snapshot -o <path>`), so this is
/// the snapshot-boot path (like `basm::run::run_document_in_emulator`), not
/// the DSK-auto-run path `basic_run` uses. Any emulator name
/// `cpclib_runner::emucontrol` accepts is valid here - unlike `basic_run`,
/// there is no auto-RUN-only restriction to honor.
pub fn run_music_in_emulator<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    emulator: &str,
    observer: &Arc<E>
) -> MusicRunOutcome {
    if !song_path.is_file() {
        return failure(format!("{song_path} does not exist"));
    }

    let dir = match camino_tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return failure(format!("Could not create a temp working directory: {e}"))
    };
    let sna_path = dir.path().join("song.sna");

    if let Err(e) = convert_and_assemble(
        song_path,
        name_hint,
        &["--snapshot".to_string(), "-o".to_string(), sna_path.to_string()],
        observer
    ) {
        return failure(e);
    }

    match super::launch_emulator_with_snapshot(&sna_path, emulator, observer) {
        Ok(()) => {
            MusicRunOutcome {
                message: format!("Launched {emulator} with {song_path}"),
                success: true
            }
        },
        Err(e) => failure(format!("Failed to launch emulator: {e}"))
    }
}

/// Converts `song_path` and assembles it into a standalone AKG player,
/// wraps it as an AMSDOS binary named from `name_hint`, and builds a fresh
/// DSK containing just that file - no emulator launch. Mirrors
/// `basic_run::run_basic_in_emulator`'s DSK-building half.
pub fn build_music_dsk<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    observer: &Arc<E>
) -> Result<Utf8PathBuf, String> {
    if !song_path.is_file() {
        return Err(format!("{song_path} does not exist"));
    }

    let built = convert_and_assemble(song_path, name_hint, &[], observer)?;

    let bytes = fs_err::read(&built.bin_path)
        .map_err(|e| format!("Could not read the assembled binary: {e}"))?;
    let fname = AmsdosFileName::try_from(built.bin_name.as_str())
        .map_err(|e| format!("Could not build an AMSDOS filename: {e:?}"))?;
    // Headerless raw bytes - see `Build`'s doc comment for why the header is
    // built here rather than by basm's own `SAVE ..., AMSDOS`.
    let file = AmsdosFile::binary_file_from_buffer(
        &fname,
        HARNESS_LOAD_ADDRESS,
        HARNESS_LOAD_ADDRESS,
        &bytes
    )
    .map_err(|e| format!("Could not build the AMSDOS binary file: {e:?}"))?;

    super::build_dsk_with_single_amsdos_file(&file, observer)
}

#[cfg(test)]
mod tests {
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
    fn missing_song_file_is_rejected_before_touching_disc() {
        let observer = Arc::new(TestObserver);
        let outcome =
            run_music_in_emulator(Utf8Path::new("/no/such/song.aks"), "PROG", "ace", &observer);
        assert!(!outcome.success);
        assert!(outcome.message.contains("does not exist"));
    }

    #[test]
    fn missing_song_file_is_rejected_before_touching_disc_dsk_path() {
        let observer = Arc::new(TestObserver);
        let err =
            build_music_dsk(Utf8Path::new("/no/such/song.aks"), "PROG", &observer).unwrap_err();
        assert!(err.contains("does not exist"));
    }

    /// Real end-to-end pass against a real fixture: downloads AT3 (if not
    /// already cached), really invokes `SongToAkg`, really assembles the
    /// harness against the real AT3-bundled `PlayerAkg.asm`, and really
    /// builds a DSK. `#[ignore]`d - needs network access and isn't something
    /// CI should pay for on every run - run manually with `--ignored`.
    #[test]
    #[ignore]
    fn real_fixture_builds_a_real_dsk() {
        let observer = Arc::new(TestObserver);
        let song = Utf8Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/at3/Targhan - Crtc - End part.aks"
        ));
        let dsk_path = build_music_dsk(song, "TARGHAN", &observer)
            .expect("the real conversion+assemble+DSK pipeline should succeed");
        use cpclib_disc::disc::Disc;
        let disc = cpclib_disc::open_disc(&dsk_path, true).unwrap();
        let fname = cpclib_disc::amsdos::AmsdosFileName::try_from("TARGHAN.BIN").unwrap();
        let file = disc
            .get_amsdos_file(cpclib_disc::edsk::Head::A, fname)
            .unwrap();
        assert!(file.is_some(), "the DSK should contain TARGHAN.BIN");
    }
}

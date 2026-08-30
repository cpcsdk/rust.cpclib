//! Converts an Arkos Tracker source song into a standalone player program,
//! then either launches it in an emulator or packs it onto a DSK - built
//! entirely on the task-agnostic operations in the parent [`super`] module,
//! so this file has no knowledge of how the conversion/assembler tasks are
//! actually dispatched.
//!
//! Two completely different players, chosen automatically per song
//! ([`super::song_uses_sid`]):
//! - **AKG** (`music_akg_harness.asm`, assembled with basm) for everything
//!   AT3 can export normally - the design (harness shape, `SongToAkg` flags)
//!   is a direct Rust port of the working Python pipeline in
//!   <https://github.com/cpcsdk/amstrad_cpc_players_comparison>
//!   (`music_play.py`/`players.py`/`players/akg/akg.asm`), not a fresh
//!   design - with one deliberate difference: the reference harness resolves
//!   its paths through basm `-D`-defined symbols used bare (e.g. `incbin
//!   MUSIC_DATA_FNAME`); real testing against this codebase's basm showed
//!   that is read as the literal text `MUSIC_DATA_FNAME` as a filename, not
//!   dereferenced - so this port instead substitutes real, already-escaped
//!   paths into `{{PLACEHOLDER}}` markers in the harness text itself before
//!   it is ever handed to basm.
//! - **SID** (`music_sid_harness.asm`, assembled with **rasm**, not basm -
//!   the player needs rasm-only directives basm doesn't implement) for a
//!   song using Arkos Tracker's experimental single-PSG-channel CPC "SID"
//!   feature, which the AKG/AKM players cannot play at all. A close,
//!   deliberately-minimal-diff port of AT3's own official example,
//!   `PlayerAkySidTester_CPC.asm` - this engine is cycle-exact (a hard
//!   64-NOP-per-scanline grid, zero tolerance for interrupts or
//!   variable-timing code), so it's a port, not a fresh design; see
//!   `music_sid_harness.asm`'s own header comment for exactly what was
//!   kept/changed and why.

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

/// The SID player-harness source, embedded at compile time - see
/// `music_sid_harness.asm` next to this file for the full commented source.
const SID_HARNESS_SOURCE: &str = include_str!("music_sid_harness.asm");

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

/// The AKG harness's AMSDOS binary load/execution address - see `Build`'s
/// doc comment.
const AKG_LOAD_ADDRESS: u16 = 0x500;

/// The SID harness's AMSDOS binary load/execution address. Unlike AKG, this
/// needs no `jp Start`-at-byte-0 trick: `Start` already lands exactly at
/// `org`'s address (`music_sid_harness.asm`'s `org #100` / `Start equ $`
/// immediately after), with no fixed-address gap to reserve first.
const SID_LOAD_ADDRESS: u16 = 0x100;

/// Which player a song needs - see [`super::song_uses_sid`].
enum PlayerKind {
    Akg,
    Sid
}

fn player_kind(song_path: &Utf8Path) -> Result<PlayerKind, String> {
    if super::song_uses_sid(song_path)? {
        Ok(PlayerKind::Sid)
    }
    else {
        Ok(PlayerKind::Akg)
    }
}

/// Converts `song_path` and assembles the AKG harness around it, naming the
/// resulting AMSDOS binary from `name_hint` (sanitized the same way
/// `basic_run` names its `.BAS` file). `extra_asm_args` lets the two public
/// entry points below differ only in whether they also ask for a snapshot -
/// everything else (conversion, path substitution, harness source) is
/// identical, matching how the Python reference's
/// `__build_replay_program__` builds both in one assemble call.
fn convert_and_assemble_akg<E: BndBuilderObserver + 'static>(
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

/// Converts `song_path` (source-mode, [`super::convert_song_to_aky_source`])
/// and assembles the SID harness around it with rasm, `extra_asm_args`
/// (rasm's own `-oi <snapshot>`/`-ob <binary>` output flags - unlike the AKG
/// harness, output is driven purely by these, no in-source `SAVE`) inserted
/// on the command line. Doesn't return a `Build`: the SID harness's `Start`
/// already lands at a fixed, known address (`SID_LOAD_ADDRESS`, see its own
/// doc comment) with no computed binary/name to hand back - the caller
/// already knows where it told rasm to write the output.
///
/// `wants_snapshot` controls the harness's `buildsna`/`bankset 0` - real
/// testing found rasm's `buildsna` and `-ob` (plain binary output) are
/// mutually exclusive (once `buildsna` is present, `-ob` is silently
/// ignored and only a default-named snapshot comes out), so the caller must
/// say up front which output it's asking `extra_asm_args` for.
fn convert_and_assemble_sid<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    sid_wait_line_count: u16,
    wants_snapshot: bool,
    extra_asm_args: &[String],
    observer: &Arc<E>
) -> Result<(), String> {
    let dir = camino_tempfile::tempdir()
        .map_err(|e| format!("Could not create a temp working directory: {e}"))?;

    let music_path = dir.path().join("music.asm");
    super::convert_song_to_aky_source(song_path, &music_path, observer)
        .map_err(|e| format!("Could not convert {song_path} to AKY: {e}"))?;

    let harness_source = SID_HARNESS_SOURCE
        .replace(
            "{{BUILDSNA_DIRECTIVES}}",
            if wants_snapshot {
                "        buildsna\n        bankset 0"
            }
            else {
                ""
            }
        )
        .replace("{{MUSIC_DATA_FNAME}}", &basm_escaped_path(&music_path))
        .replace(
            "{{PLAYER_SOURCE_FNAME}}",
            &basm_escaped_path(&At3Version::default().aky_sid_path::<()>())
        )
        .replace(
            "{{PLAYER_MACROS_FNAME}}",
            &basm_escaped_path(&At3Version::default().aky_sid_macros_path::<()>())
        )
        .replace("{{WAIT_LINE_COUNT}}", &sid_wait_line_count.to_string());
    let harness_path = dir.path().join("harness.asm");
    fs_err::write(&harness_path, harness_source)
        .map_err(|e| format!("Could not write the SID player harness: {e}"))?;

    super::assemble_source_with_rasm(&harness_path, extra_asm_args, observer)
        .map_err(|e| format!("Could not assemble the SID player harness: {e}"))
}

/// Converts `song_path`, assembles it into a standalone player, and launches
/// `emulator` on the resulting snapshot - built in the same assemble pass as
/// the AMSDOS binary (AKG: `--snapshot -o <path>`; SID: `-oi <path>`), so
/// this is the snapshot-boot path (like `basm::run::run_document_in_emulator`),
/// not the DSK-auto-run path `basic_run` uses. Any emulator name
/// `cpclib_runner::emucontrol` accepts is valid here - unlike `basic_run`,
/// there is no auto-RUN-only restriction to honor.
///
/// `sid_wait_line_count` only matters if `song_path` turns out to use SID
/// (`MusicConfig::sid_wait_line_count`, ignored otherwise) - see
/// `music_sid_harness.asm`'s own doc comment for what it controls.
pub fn run_music_in_emulator<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    emulator: &str,
    sid_wait_line_count: u16,
    observer: &Arc<E>
) -> MusicRunOutcome {
    if !song_path.is_file() {
        return failure(format!("{song_path} does not exist"));
    }

    let kind = match player_kind(song_path) {
        Ok(k) => k,
        Err(e) => return failure(e)
    };

    let dir = match camino_tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return failure(format!("Could not create a temp working directory: {e}"))
    };
    let sna_path = dir.path().join("song.sna");

    let result = match kind {
        PlayerKind::Akg => {
            convert_and_assemble_akg(
                song_path,
                name_hint,
                &["--snapshot".to_string(), "-o".to_string(), sna_path.to_string()],
                observer
            )
            .map(|_| ())
        },
        PlayerKind::Sid => {
            convert_and_assemble_sid(
                song_path,
                sid_wait_line_count,
                true,
                &["-oi".to_string(), sna_path.to_string()],
                observer
            )
        }
    };
    if let Err(e) = result {
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

/// Wraps `bin_path`'s headerless raw bytes into an AMSDOS binary named
/// `bin_name`, loaded/executed at `load_address`, and builds a fresh DSK
/// containing just that file. Shared by both player kinds' DSK-building
/// half - see `Build`'s doc comment for why the header is built in Rust
/// rather than by the assembler's own save/output mechanism.
fn wrap_and_build_dsk<E: BndBuilderObserver + 'static>(
    bin_path: &Utf8Path,
    bin_name: &str,
    load_address: u16,
    observer: &Arc<E>
) -> Result<Utf8PathBuf, String> {
    let bytes = fs_err::read(bin_path)
        .map_err(|e| format!("Could not read the assembled binary: {e}"))?;
    let fname = AmsdosFileName::try_from(bin_name)
        .map_err(|e| format!("Could not build an AMSDOS filename: {e:?}"))?;
    let file = AmsdosFile::binary_file_from_buffer(&fname, load_address, load_address, &bytes)
        .map_err(|e| format!("Could not build the AMSDOS binary file: {e:?}"))?;

    super::build_dsk_with_single_amsdos_file(&file, observer)
}

/// Converts `song_path` and assembles it into a standalone player, wraps it
/// as an AMSDOS binary named from `name_hint`, and builds a fresh DSK
/// containing just that file - no emulator launch. Mirrors
/// `basic_run::run_basic_in_emulator`'s DSK-building half.
///
/// `sid_wait_line_count` only matters if `song_path` turns out to use SID -
/// see [`run_music_in_emulator`]'s doc comment.
pub fn build_music_dsk<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    sid_wait_line_count: u16,
    observer: &Arc<E>
) -> Result<Utf8PathBuf, String> {
    if !song_path.is_file() {
        return Err(format!("{song_path} does not exist"));
    }

    match player_kind(song_path)? {
        PlayerKind::Akg => {
            let built = convert_and_assemble_akg(song_path, name_hint, &[], observer)?;
            wrap_and_build_dsk(&built.bin_path, &built.bin_name, AKG_LOAD_ADDRESS, observer)
        },
        PlayerKind::Sid => {
            let dir = camino_tempfile::tempdir()
                .map_err(|e| format!("Could not create a temp working directory: {e}"))?;
            let bin_name = format!("{}.BIN", super::sanitize_amsdos_stem(name_hint));
            let bin_path = dir.path().join(&bin_name);

            convert_and_assemble_sid(
                song_path,
                sid_wait_line_count,
                false,
                &["-ob".to_string(), bin_path.to_string()],
                observer
            )?;
            wrap_and_build_dsk(&bin_path, &bin_name, SID_LOAD_ADDRESS, observer)
        }
    }
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
        let outcome = run_music_in_emulator(
            Utf8Path::new("/no/such/song.aks"),
            "PROG",
            "ace",
            72,
            &observer
        );
        assert!(!outcome.success);
        assert!(outcome.message.contains("does not exist"));
    }

    #[test]
    fn missing_song_file_is_rejected_before_touching_disc_dsk_path() {
        let observer = Arc::new(TestObserver);
        let err = build_music_dsk(Utf8Path::new("/no/such/song.aks"), "PROG", 72, &observer)
            .unwrap_err();
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
        let dsk_path = build_music_dsk(song, "TARGHAN", 72, &observer)
            .expect("the real conversion+assemble+DSK pipeline should succeed");
        use cpclib_disc::disc::Disc;
        let disc = cpclib_disc::open_disc(&dsk_path, true).unwrap();
        let fname = cpclib_disc::amsdos::AmsdosFileName::try_from("TARGHAN.BIN").unwrap();
        let file = disc
            .get_amsdos_file(cpclib_disc::edsk::Head::A, fname)
            .unwrap();
        assert!(file.is_some(), "the DSK should contain TARGHAN.BIN");
    }

    /// Real end-to-end pass for the SID path, against a real fixture bundled
    /// with the AT3 install itself (not vendored into this repo): downloads
    /// AT3 (if not already cached), really invokes `SongToAky` in source
    /// mode, really assembles the SID harness with rasm against the real
    /// AT3-bundled `PlayerAkySid_CPC.asm`/`PlayerAkySidMacros_CPC.asm`, and
    /// really builds a DSK. `#[ignore]`d for the same reasons as
    /// `real_fixture_builds_a_real_dsk`.
    #[test]
    #[ignore]
    fn real_sid_fixture_builds_a_real_dsk() {
        use cpclib_disc::disc::Disc;
        use cpclib_runner::delegated::InternetStaticCompiledApplication as _;

        let observer = Arc::new(TestObserver);
        let song = At3Version::default()
            .configuration::<()>()
            .cache_folder()
            .join("songs")
            .join("ArkosTracker3")
            .join("sid")
            .join("SidExamples.aks");
        assert!(super::super::song_uses_sid(&song).unwrap(), "fixture should be SID-tagged");

        let dsk_path = build_music_dsk(&song, "SIDTEST", 72, &observer)
            .expect("the real SID conversion+assemble+DSK pipeline should succeed");
        let disc = cpclib_disc::open_disc(&dsk_path, true).unwrap();
        let fname = cpclib_disc::amsdos::AmsdosFileName::try_from("SIDTEST.BIN").unwrap();
        let file = disc
            .get_amsdos_file(cpclib_disc::edsk::Head::A, fname)
            .unwrap();
        assert!(file.is_some(), "the DSK should contain SIDTEST.BIN");
    }

    /// Same real fixture as `real_sid_fixture_builds_a_real_dsk`, but the
    /// snapshot half (`wants_snapshot: true`, `-oi`) - the `buildsna`/`-ob`
    /// mutual-exclusion bug this file's own doc comment describes was found
    /// and fixed via manual CLI testing, not through an automated test; this
    /// confirms the fix through the real Rust pipeline, not just by hand.
    #[test]
    #[ignore]
    fn real_sid_fixture_builds_a_real_snapshot() {
        use cpclib_runner::delegated::InternetStaticCompiledApplication as _;

        let observer = Arc::new(TestObserver);
        let song = At3Version::default()
            .configuration::<()>()
            .cache_folder()
            .join("songs")
            .join("ArkosTracker3")
            .join("sid")
            .join("SidExamples.aks");

        let dir = camino_tempfile::tempdir().unwrap();
        let sna_path = dir.path().join("song.sna");
        convert_and_assemble_sid(
            &song,
            72,
            true,
            &["-oi".to_string(), sna_path.to_string()],
            &observer
        )
        .expect("the real SID conversion+assemble+snapshot pipeline should succeed");
        assert!(sna_path.is_file(), "the snapshot should have been written");
        let size = std::fs::metadata(&sna_path).unwrap().len();
        assert!(size > 20_000, "a real 64K .sna should be well over 20KB, got {size}");
    }
}

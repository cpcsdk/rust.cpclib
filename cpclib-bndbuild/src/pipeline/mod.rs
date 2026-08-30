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
pub mod debug;
pub mod music_run;

use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use camino_tempfile::Builder as TempBuilder;
use cpclib_disc::amsdos::AmsdosFile;

use cpclib_runner::runner::assembler::{ExternAssembler, RasmVersion};

use crate::event::BndBuilderObserver;
use crate::runners::assembler::Assembler;
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

/// Launch an emulator on a snapshot, without waiting for it to close.
///
/// The "run this program" half of what `debug` does: the same build, handed to
/// whichever emulator the project names rather than to the one that speaks the
/// Debug Adapter Protocol. `--background` for the same reason as
/// [`launch_emulator_with_auto_run`] - the caller is an editor request handler,
/// and blocking it until the user closes the emulator would hang it for the
/// rest of the session.
pub fn launch_emulator_with_snapshot<E: BndBuilderObserver + 'static>(
    snapshot: &Utf8Path,
    emulator: &str,
    observer: &Arc<E>
) -> Result<(), String> {
    let task: Task = InnerTask::Emulator(
        Emulator::EmulatorFacade,
        StandardTaskArguments::new(format!(
            "--emulator {emulator} --snapshot {snapshot} --background run"
        ))
    )
    .into();
    task.execute(observer)
}

/// Converts `song_path` (any format Arkos Tracker 3 can import: AKS/SKS/128/
/// VT2/WYZ) to the AKG player format at `output_path`. AT3 also writes a
/// companion `<output_path without extension>_playerconfig.asm` next to it -
/// that naming is AT3's own convention, derived purely from `output_path`, not
/// something this function controls or needs to report back.
///
/// `-bin -adr 0x506` is baked in rather than parameterized: it matches the AKG
/// harness's own `org 0x500` / `assert $ == 0x506` in
/// [`music_run`](self::music_run), and nothing else in this codebase calls
/// `SongToAkg` with a different address.
///
/// `song_path`/`output_path` are shell-quoted before being joined into the
/// single args string `StandardTaskArguments` expects (unlike this module's
/// other helpers, whose paths are always spaceless temp files) - a real song
/// file is user-supplied and routinely has spaces in its name (e.g. `Targhan -
/// Crtc - End part.aks`, a real fixture in this repo's own `tests/at3`).
pub fn convert_song_to_akg<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    output_path: &Utf8Path,
    observer: &Arc<E>
) -> Result<(), String> {
    let args = shlex::try_join(
        [
            "-bin",
            "-adr",
            "0x506",
            "--exportPlayerConfig",
            song_path.as_str(),
            output_path.as_str()
        ]
        .into_iter()
    )
    .map_err(|e| format!("Could not build SongToAkg arguments: {e}"))?;

    let task: Task = InnerTask::with_songconverter(
        crate::runners::tracker::SongConverter::new_song_to_akg_default(),
        StandardTaskArguments::new(args)
    )
    .into();
    task.execute(observer)
}

/// Converts `song_path` to Arkos Tracker's AKY player format at `output_path`,
/// in **source** mode (no `-bin`/`-adr`/`--exportPlayerConfig`) - unlike
/// [`convert_song_to_akg`], this is meant to be `include`d as assembleable
/// `.asm` source, not `incbin`'d as a fixed-address binary blob. Used by
/// [`music_run`](self::music_run)'s SID player path: source mode is what
/// Arkos Tracker's own official SID player example
/// (`PlayerAkySidTester_CPC.asm`) uses and ships a checked-in, unmodified
/// export of as a resource - whether binary mode's baked-in absolute
/// addressing is even correct for SID-tagged content is unverified, so this
/// sticks to the proven-working shape.
pub fn convert_song_to_aky_source<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    output_path: &Utf8Path,
    observer: &Arc<E>
) -> Result<(), String> {
    let args = shlex::try_join([song_path.as_str(), output_path.as_str()].into_iter())
        .map_err(|e| format!("Could not build SongToAky arguments: {e}"))?;

    let task: Task = InnerTask::with_songconverter(
        crate::runners::tracker::SongConverter::new_song_to_aky_default(),
        StandardTaskArguments::new(args)
    )
    .into();
    task.execute(observer)
}

/// Detects whether `song_path` (an Arkos Tracker `.aks` project - a ZIP
/// archive with a single inner XML entry) uses AT3's experimental
/// single-channel CPC "SID" feature, which the AKG/AKM players cannot play at
/// all (a completely different, cycle-exact player is needed - see
/// [`music_run`](self::music_run)).
///
/// Looks for a real `<sidIsActivated>true</sidIsActivated>` XML *element*
/// (event-scanned with `quick_xml`, not a raw substring search over the whole
/// blob) - a substring search would false-positive on a project that merely
/// has an instrument named, or a comment containing, that text without the
/// feature actually being on. Verified against real AT3-bundled SID and
/// non-SID `.aks` files: the tag is emitted per-instrument-cell only when SID
/// is used, and never emitted as `false` - it's simply absent otherwise.
pub fn song_uses_sid(song_path: &Utf8Path) -> Result<bool, String> {
    use std::io::Read;

    let file = fs_err::File::open(song_path)
        .map_err(|e| format!("Could not open {song_path}: {e}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("{song_path} is not a valid Arkos Tracker project: {e}"))?;
    if archive.is_empty() {
        return Err(format!("{song_path} is an empty archive"));
    }

    let mut xml = String::new();
    archive
        .by_index(0)
        .map_err(|e| format!("Could not read {song_path}'s song data: {e}"))?
        .read_to_string(&mut xml)
        .map_err(|e| format!("Could not read {song_path}'s song data: {e}"))?;

    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut in_sid_is_activated = false;
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("Could not parse {song_path}'s song data: {e}"))?
        {
            quick_xml::events::Event::Start(tag) if tag.name().as_ref() == b"sidIsActivated" => {
                in_sid_is_activated = true;
            },
            quick_xml::events::Event::Text(text) if in_sid_is_activated => {
                // A boolean's text content never contains XML entities, so a
                // plain UTF-8 decode (no unescape) is enough here.
                if String::from_utf8_lossy(&text).trim() == "true" {
                    return Ok(true);
                }
                in_sid_is_activated = false;
            },
            quick_xml::events::Event::End(tag) if tag.name().as_ref() == b"sidIsActivated" => {
                in_sid_is_activated = false;
            },
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    Ok(false)
}

/// Assembles `source_path`, with `extra_args` (`-D` definitions, `--snapshot
/// -o <path>`, ...) inserted before it on the command line - runs in-process
/// (`InnerTask::Assembler(Assembler::Basm, _)` is `TaskKind::Embedded`,
/// calling `cpclib_basm::process` directly), no subprocess involved.
///
/// Each entry of `extra_args` is shell-quoted independently before joining -
/// see [`convert_song_to_akg`]'s doc comment on why that matters here.
pub fn assemble_source<E: BndBuilderObserver + 'static>(
    source_path: &Utf8Path,
    extra_args: &[String],
    observer: &Arc<E>
) -> Result<(), String> {
    let args = shlex::try_join(
        extra_args
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(source_path.as_str()))
    )
    .map_err(|e| format!("Could not build basm arguments: {e}"))?;

    let task: Task = InnerTask::new_basm(&args).into();
    task.execute(observer)
}

/// Same as [`assemble_source`], but with `rasm` (auto-downloaded/cached the
/// same way every other delegated tool in this codebase is) instead of basm -
/// a real subprocess (`TaskKind::Delegated`), not in-process. Needed for the
/// SID player harness in [`music_run`](self::music_run): it uses rasm-only
/// directives (`COUNTNOPS`, `ASSERT`, local-label macro substitution) that
/// basm doesn't implement.
pub fn assemble_source_with_rasm<E: BndBuilderObserver + 'static>(
    source_path: &Utf8Path,
    extra_args: &[String],
    observer: &Arc<E>
) -> Result<(), String> {
    let args = shlex::try_join(
        extra_args
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(source_path.as_str()))
    )
    .map_err(|e| format!("Could not build rasm arguments: {e}"))?;

    let task: Task = InnerTask::with_assembler(
        Assembler::Extern(ExternAssembler::Rasm(RasmVersion::default())),
        StandardTaskArguments::new(args)
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
    /// Real AT3-bundled fixtures, both SID and non-SID - needs a real AT3
    /// install (downloaded on demand by other tests/real usage), so this is
    /// `#[ignore]`d rather than assumed present in CI.
    #[test]
    #[ignore]
    fn song_uses_sid_detects_real_sid_and_non_sid_fixtures() {
        use cpclib_runner::delegated::InternetStaticCompiledApplication as _;

        let songs_dir = cpclib_runner::runner::tracker::at3::At3Version::default()
            .configuration::<()>()
            .cache_folder()
            .join("songs")
            .join("ArkosTracker3");

        let sid = songs_dir.join("sid").join("SidExamples.aks");
        assert!(
            song_uses_sid(&sid).expect("should parse"),
            "{sid} should be detected as using SID"
        );

        let non_sid = songs_dir.join("Ok3anos - Cpc Dream.aks");
        assert!(
            !song_uses_sid(&non_sid).expect("should parse"),
            "{non_sid} should NOT be detected as using SID"
        );
    }

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

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
pub mod csl_run;
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
    let task: Task = InnerTask::Disc(StandardTaskArguments::new(
        shlex::try_join([dsk_path.as_str(), "format", "-f", "data"])
            .map_err(|e| format!("Could not build disc arguments: {e}"))?
    ))
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
    let task: Task = InnerTask::Disc(StandardTaskArguments::new(
        shlex::try_join([dsk_path.as_str(), "add", file_path.as_str()])
            .map_err(|e| format!("Could not build disc arguments: {e}"))?
    ))
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
        StandardTaskArguments::new(
            shlex::try_join([
                "--drivea",
                drive_a.as_str(),
                "--emulator",
                emulator,
                "--auto-run-file",
                auto_run_file,
                "--background",
                "run"
            ])
            .map_err(|e| format!("Could not build emulator arguments: {e}"))?
        )
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
        StandardTaskArguments::new(
            shlex::try_join([
                "--emulator",
                emulator,
                "--snapshot",
                snapshot.as_str(),
                "--background",
                "run"
            ])
            .map_err(|e| format!("Could not build emulator arguments: {e}"))?
        )
    )
    .into();
    task.execute(observer)
}

/// Launch an emulator on a `.csl` (CPC Script Language) file, without
/// waiting for it to close - the `.csl`-file analogue of
/// [`launch_emulator_with_snapshot`], built the same way (an
/// `EmulatorFacade` task, `--background` for the same reason: the caller is
/// an editor request handler and must not block on the user closing the
/// emulator).
///
/// `emulator` need not natively support CSL
/// (`cpclib_runner::runner::emulator::Emulator::accept_csl`) - `emucontrol`'s
/// own `--csl` handling (`emucontrol::run_csl_file`) picks the native
/// `--csl=` launch or its CSL-interpreter fallback on its own, so this
/// function never needs to know which case applies.
///
/// `base_dir`, when given, is passed through as `--csl-base-dir` -
/// `emucontrol::run_csl_file` resolves the script's own relative
/// `disk_insert`/`snapshot_load`/etc. paths against it instead of
/// `csl_file`'s own parent directory, which matters whenever `csl_file` is
/// a temp copy of a script that lives (and whose relative paths are
/// meant to resolve) somewhere else entirely - see
/// `csl_run::run_csl_in_emulator`'s own doc comment. Both this and
/// `csl_file` are shell-quoted (`shlex`), since a real project directory
/// routinely has spaces in it (e.g. a Shaker `CSL/MODULE A/` layout).
pub fn launch_emulator_with_csl<E: BndBuilderObserver + 'static>(
    csl_file: &Utf8Path,
    base_dir: Option<&Utf8Path>,
    emulator: &str,
    observer: &Arc<E>
) -> Result<(), String> {
    let args = csl_launch_args(csl_file, base_dir, emulator)?;
    let task: Task =
        InnerTask::Emulator(Emulator::EmulatorFacade, StandardTaskArguments::new(args)).into();
    task.execute(observer)
}

/// The pure argument-string-building half of [`launch_emulator_with_csl`],
/// pulled out so it's unit-testable without going through a real
/// `Task::execute` (which would try to actually locate/spawn an
/// emulator).
fn csl_launch_args(
    csl_file: &Utf8Path,
    base_dir: Option<&Utf8Path>,
    emulator: &str
) -> Result<String, String> {
    let mut parts = vec!["--emulator".to_string(), emulator.to_string()];
    if let Some(dir) = base_dir {
        parts.push("--csl-base-dir".to_string());
        parts.push(dir.to_string());
    }
    parts.push("--csl".to_string());
    parts.push(csl_file.to_string());
    parts.push("--background".to_string());
    parts.push("run".to_string());
    shlex::try_join(parts.iter().map(String::as_str)).map_err(|e| e.to_string())
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
    let args = shlex::try_join([
        "-bin",
        "-adr",
        "0x506",
        "--exportPlayerConfig",
        song_path.as_str(),
        output_path.as_str()
    ])
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
    let args = shlex::try_join([song_path.as_str(), output_path.as_str()])
        .map_err(|e| format!("Could not build SongToAky arguments: {e}"))?;

    let task: Task = InnerTask::with_songconverter(
        crate::runners::tracker::SongConverter::new_song_to_aky_default(),
        StandardTaskArguments::new(args)
    )
    .into();
    task.execute(observer)
}

/// Whether `song_path` is a CHIPNSFX song (`.chp`) - played by a completely
/// different player than Arkos Tracker's (see [`music_run`](self::music_run)).
pub fn song_is_chp(song_path: &Utf8Path) -> bool {
    song_path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("chp"))
}

/// Converts the CHIPNSFX song `song_path` (`.chp`) to the Z80 source
/// (`song_a`/`song_b`/`song_c` data, to be `include`d next to `CHIPNSFX.I80`)
/// at `output_path`. Runs the real `chipnsfx` tool, downloaded on demand.
pub fn convert_chp_to_z80<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    output_path: &Utf8Path,
    observer: &Arc<E>
) -> Result<(), String> {
    let args = shlex::try_join([song_path.as_str(), output_path.as_str()])
        .map_err(|e| format!("Could not build chipnsfx arguments: {e}"))?;

    let task: Task = InnerTask::with_tracker(
        crate::runners::tracker::Tracker::new_chipnsfx_default(),
        StandardTaskArguments::new(args)
    )
    .into();
    task.execute(observer)
}

/// Whether `song_path` is an AY/YM song (`.ym`) - playable as is by the
/// YM-based players ([`music_run`](self::music_run)'s FAP/AYT/MinYMiser), which
/// every other kind of song can also be converted to.
pub fn song_is_ym(song_path: &Utf8Path) -> bool {
    song_path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ym"))
}

/// What `song_converter` + `chipnsfx` need to produce YM files, the common
/// ground of the YM-based players - see [`convert_to_ym`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YmVersion {
    /// Raw, uncompressed register dump, no metadata: what MinYMiser reads.
    Ym3,
    /// YM5/YM6, with title/author/comment: what FAP and AYT read.
    Ym6
}

/// The version of the YM file `path`, from its magic number; `None` when it
/// is not one (e.g. an old LHA-compressed one).
pub fn ym_version(path: &Utf8Path) -> Option<YmVersion> {
    use std::io::Read;

    let mut magic = [0u8; 4];
    fs_err::File::open(path).ok()?.read_exact(&mut magic).ok()?;
    match &magic {
        b"YM3!" | b"YM3b" => Some(YmVersion::Ym3),
        b"YM5!" | b"YM6!" => Some(YmVersion::Ym6),
        _ => None
    }
}

/// Converts any song `song_path` (Arkos Tracker's AKS/SKS/128/VT2/WYZ, CHIPNSFX's CHP, or
/// a YM of the other version) to a YM of `version` at `output_path`.
/// A YM already of that version is not touched: it is up to the caller to
/// use `song_path` directly then (see [`ym_for_player`]).
pub fn convert_to_ym<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    output_path: &Utf8Path,
    version: YmVersion,
    observer: &Arc<E>
) -> Result<(), String> {
    let ym_from = |input: &Utf8Path, output: &Utf8Path| -> Result<(), String> {
        let mut args = Vec::new();
        if version == YmVersion::Ym3 {
            args.push("--ym3");
        }
        args.push(input.as_str());
        args.push(output.as_str());
        let args = shlex::try_join(args)
            .map_err(|e| format!("Could not build SongToYm arguments: {e}"))?;
        let task: Task = InnerTask::with_songconverter(
            crate::runners::tracker::SongConverter::new_song_to_ym_default(),
            StandardTaskArguments::new(args)
        )
        .into();
        task.execute(observer)
    };

    if song_is_chp(song_path) {
        // CHIPNSFX only knows how to write YM3
        let ym3 = if version == YmVersion::Ym3 {
            output_path.to_owned()
        }
        else {
            output_path.with_extension("ym3")
        };
        let args = shlex::try_join([song_path.as_str(), "-y", ym3.as_str()])
            .map_err(|e| format!("Could not build chipnsfx arguments: {e}"))?;
        let task: Task = InnerTask::with_tracker(
            crate::runners::tracker::Tracker::new_chipnsfx_default(),
            StandardTaskArguments::new(args)
        )
        .into();
        task.execute(observer)?;
        if version == YmVersion::Ym6 {
            ym_from(&ym3, output_path)?;
        }
        Ok(())
    }
    else {
        ym_from(song_path, output_path)
    }
}

/// The YM, of the `version` a player needs, to feed it for `song_path`: the
/// song itself when it already is one, else the result of [`convert_to_ym`],
/// written at `converted_path`.
pub fn ym_for_player<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    converted_path: &Utf8Path,
    version: YmVersion,
    observer: &Arc<E>
) -> Result<Utf8PathBuf, String> {
    if song_is_ym(song_path) {
        match (ym_version(song_path), version) {
            (Some(found), wanted) if found == wanted => return Ok(song_path.to_owned()),
            // `SongToYm` hands a YM back as a YM6 whatever it is asked, so
            // the way down to YM3 is done here
            (Some(YmVersion::Ym6), YmVersion::Ym3) => {
                let ym3 = ym6_to_ym3(
                    &fs_err::read(song_path)
                        .map_err(|e| format!("Could not read {song_path}: {e}"))?
                )
                .ok_or_else(|| format!("{song_path} is not a YM file this can convert to YM3"))?;
                fs_err::write(converted_path, ym3)
                    .map_err(|e| format!("Could not write {converted_path}: {e}"))?;
                return Ok(converted_path.to_owned());
            },
            _ => {}
        }
    }
    convert_to_ym(song_path, converted_path, version, observer)
        .map_err(|e| format!("Could not convert {song_path} to YM: {e}"))?;
    Ok(converted_path.to_owned())
}

/// Packs the YM `ym_path` with `cruncher` (FAP, AYT or MinYMiser) into
/// `output_path`, and returns what the tool printed - where the players'
/// buffer sizes are to be found.
pub fn pack_ym<E: BndBuilderObserver + 'static>(
    cruncher: crate::runners::ay::YmCruncher,
    ym_path: &Utf8Path,
    output_path: &Utf8Path,
    observer: &Arc<E>
) -> Result<String, String> {
    use crate::runners::ay::YmCruncher;

    let args = match cruncher {
        #[cfg(feature = "fap")]
        YmCruncher::Fap => shlex::try_join([ym_path.as_str(), output_path.as_str()]),
        YmCruncher::Ayt => {
            shlex::try_join([
                "--verbose",
                "--target",
                "CPC",
                ym_path.as_str(),
                "-o",
                output_path.as_str()
            ])
        },
        YmCruncher::Miny => shlex::try_join(["quick", ym_path.as_str(), output_path.as_str()])
    }
    .map_err(|e| format!("Could not build the packer arguments: {e}"))?;

    let captured = Arc::new(cpclib_common::event::CapturingObserver::new());
    let task: Task = InnerTask::with_ym_cruncher(cruncher, StandardTaskArguments::new(args)).into();
    let result = task.execute(&captured);

    // the tools are chatty: forward what they said, whatever happened
    let stdout = captured.stdout_joined();
    observer.emit_stdout(&stdout);
    observer.emit_stderr(&captured.stderr_joined());
    result?;
    Ok(stdout)
}

/// The number following `label` on the line of `output` that has it, e.g.
/// 3144 for `Decrunch buffer size: 3144 (#C48)`.
pub fn tool_reported_size(output: &str, label: &str) -> Option<usize> {
    output
        .lines()
        .find(|l| l.contains(label))
        .and_then(|l| l.split_once(label))
        .map(|(_, rest)| rest.trim_start_matches([':', ' ', '\t']))
        .and_then(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
}

/// Where things are in a YM5/YM6 file.
struct Ym6Layout {
    /// The title, author and comment: three NUL-terminated strings, after the
    /// header and the digidrums.
    strings_start: usize,
    /// The frames, right after the strings.
    data_start: usize,
    frames: usize,
    /// All the frames of a register, then the next register (YM6's usual
    /// layout) - or one frame after the other.
    interleaved: bool
}

impl Ym6Layout {
    /// `None` if `bytes` is not a (complete) YM5/YM6 file's header.
    fn parse(bytes: &[u8]) -> Option<Self> {
        let be16 = |at: usize| {
            bytes
                .get(at..at + 2)
                .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
        };
        let be32 = |at: usize| {
            bytes
                .get(at..at + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
        };
        if !(bytes.starts_with(b"YM5!") || bytes.starts_with(b"YM6!")) {
            return None;
        }
        let frames = be32(12)?;
        let interleaved = be32(16)? & 1 != 0;
        let mut at = 34 + be16(32)?;
        for _ in 0..be16(20)? {
            at += 4 + be32(at)?;
        }
        let strings_start = at;
        for _ in 0..3 {
            at += bytes.get(at..)?.iter().position(|&b| b == 0)? + 1;
        }
        Some(Self {
            strings_start,
            data_start: at,
            frames,
            interleaved
        })
    }
}

/// A YM5/YM6 file as a YM3: its 14 first registers per frame, interleaved -
/// what MinYMiser reads. The effects and digidrums YM5/6 may add are dropped.
/// `None` if `bytes` is not a (complete) YM5/YM6 file.
fn ym6_to_ym3(bytes: &[u8]) -> Option<Vec<u8>> {
    const REGISTERS: usize = 14;
    let layout = Ym6Layout::parse(bytes)?;
    let frames = layout.frames;
    // YM5/6 store 16 registers per frame
    let data =
        bytes.get(layout.data_start..layout.data_start.checked_add(frames.checked_mul(16)?)?)?;

    let mut ym3 = Vec::with_capacity(4 + frames * REGISTERS);
    ym3.extend_from_slice(b"YM3!");
    for register in 0..REGISTERS {
        ym3.extend((0..frames).map(|frame| {
            if layout.interleaved {
                data[register * frames + frame]
            }
            else {
                data[frame * 16 + register]
            }
        }));
    }
    Some(ym3)
}

/// The title, author and comment of a YM5/YM6 file. No metadata for YM3
/// (a bare register dump) or for anything unrecognised.
fn ym_metadata(bytes: &[u8]) -> SongMetadata {
    let Some(layout) = Ym6Layout::parse(bytes)
    else {
        return SongMetadata::default();
    };
    let mut strings = bytes[layout.strings_start..layout.data_start]
        .split(|&b| b == 0)
        .map(|s| s.iter().map(|&b| char::from(b)).collect::<String>());
    SongMetadata {
        title: strings.next().unwrap_or_default(),
        author: strings.next().unwrap_or_default(),
        comment: strings.next().unwrap_or_default(),
        ..SongMetadata::default()
    }
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

    // CHIPNSFX and YM songs are not Arkos Tracker projects, and have no such feature.
    if song_is_chp(song_path) || song_is_ym(song_path) {
        return Ok(false);
    }

    let file =
        fs_err::File::open(song_path).map_err(|e| format!("Could not open {song_path}: {e}"))?;
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
            quick_xml::events::Event::Start(tag) if tag.name().as_ref() == "sidIsActivated" => {
                in_sid_is_activated = true;
            },
            quick_xml::events::Event::Text(text) if in_sid_is_activated => {
                // A boolean's text content never contains XML entities, so a
                // plain UTF-8 decode (no unescape) is enough here.
                if text.trim() == "true" {
                    return Ok(true);
                }
                in_sid_is_activated = false;
            },
            quick_xml::events::Event::End(tag) if tag.name().as_ref() == "sidIsActivated" => {
                in_sid_is_activated = false;
            },
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    Ok(false)
}

/// The descriptive fields of an Arkos Tracker song, as shown in AT3's
/// "song properties" - all empty when the song has none (or can't be read,
/// see [`song_metadata`]).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SongMetadata {
    pub title: String,
    pub author: String,
    pub composer: String,
    pub comment: String
}

/// The free text a CHIPNSFX song starts with: after the `CHIPNSFX format`
/// line, a title (usually quoted) then any number of description lines (author,
/// year, ...), up to a `===` line. Yields no metadata for anything else.
fn chp_metadata(bytes: &[u8]) -> SongMetadata {
    // Plain 8-bit text: Latin-1 is as good a guess as any, and never fails.
    let text: String = bytes.iter().map(|&b| char::from(b)).collect();
    let mut lines = text.lines();
    if !lines
        .next()
        .is_some_and(|l| l.trim().to_ascii_uppercase().starts_with("CHIPNSFX"))
    {
        return SongMetadata::default();
    }
    let mut header = lines
        .map(str::trim)
        .take_while(|l| !l.starts_with("==="))
        // a malformed song with no `===` must not swallow its whole body
        .take(8);
    SongMetadata {
        title: header
            .next()
            .map(|t| t.replace('"', "").trim().to_string())
            .unwrap_or_default(),
        comment: header.collect::<Vec<_>>().join("\n"),
        ..SongMetadata::default()
    }
}

/// Best-effort read of `song_path`'s title/author/composer/comment - for a
/// CHIPNSFX `.chp`, its text header (see `chp_metadata`); for an `.aks` (only the
/// song's own direct children: instruments carry `title`/`name` tags too, which
/// must not be mistaken for the song's). Any failure (not an AKS zip - e.g. a
/// `.vt2`, which AT3 converts itself -, bad XML...) yields empty metadata, as
/// this only decorates a player and must never stop one from being built.
/// Handles both the AT3 format (`<title>`) and the older AT1/2 one
/// (`<aks:title>`).
pub fn song_metadata(song_path: &Utf8Path) -> SongMetadata {
    use std::io::Read;

    let mut meta = SongMetadata::default();
    if song_is_chp(song_path) {
        return fs_err::read(song_path)
            .map(|bytes| chp_metadata(&bytes))
            .unwrap_or(meta);
    }
    if song_is_ym(song_path) {
        return fs_err::read(song_path)
            .map(|bytes| ym_metadata(&bytes))
            .unwrap_or(meta);
    }
    let Ok(file) = fs_err::File::open(song_path)
    else {
        return meta;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file)
    else {
        return meta;
    };
    if archive.is_empty() {
        return meta;
    }
    let mut xml = String::new();
    match archive.by_index(0) {
        Ok(mut entry) => {
            if entry.read_to_string(&mut xml).is_err() {
                return meta;
            }
        },
        Err(_) => return meta
    }

    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut depth = 0usize;
    let mut current: Option<&'static str> = None;
    while let Ok(event) = reader.read_event() {
        match event {
            quick_xml::events::Event::Start(tag) => {
                depth += 1;
                if depth == 2 {
                    let name = tag.name();
                    let name = name.as_ref().to_string();
                    current = match name.rsplit(':').next().unwrap_or("") {
                        "title" => Some("title"),
                        "author" => Some("author"),
                        "composer" => Some("composer"),
                        "comment" => Some("comment"),
                        _ => None
                    };
                }
            },
            quick_xml::events::Event::Text(text) if depth == 2 => {
                if let Some(field) = current {
                    let raw: &str = &text;
                    let text = quick_xml::escape::unescape(raw)
                        .map_or_else(|_| raw.to_string(), |t| t.into_owned());
                    match field {
                        "title" => meta.title.push_str(&text),
                        "author" => meta.author.push_str(&text),
                        "composer" => meta.composer.push_str(&text),
                        _ => meta.comment.push_str(&text)
                    }
                }
            },
            quick_xml::events::Event::End(_) => {
                depth = depth.saturating_sub(1);
                if depth < 2 {
                    current = None;
                }
            },
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    meta
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

    /// A real Shaker-style layout (`CSL/MODULE A/script.CSL`) has a space
    /// in its own directory name - the constructed `--csl-base-dir` value
    /// must survive being split back into argv by the same `shlex`-based
    /// splitter the real CLI uses (`cpclib_runner::runner::arguments::
    /// get_all_args`), not just look plausible as a raw string. `EmuCli`'s
    /// fields are private, so parsing successfully (rather than splitting
    /// the space-containing directory into two arguments, which would
    /// make `run` an unexpected extra positional and fail to parse) is
    /// what this actually checks.
    #[test]
    fn csl_launch_args_survives_a_space_in_the_base_dir() {
        let args = csl_launch_args(
            Utf8Path::new("/tmp/script.csl"),
            Some(Utf8Path::new("/project/CSL/MODULE A")),
            "sugarbox"
        )
        .unwrap();

        let split = shlex::split(&args).unwrap();
        let mut argv: Vec<&str> = vec!["cpc"];
        argv.extend(split.iter().map(String::as_str));
        let parsed = cpclib_runner::emucontrol::EmuCli::try_parse_from(argv);
        assert!(parsed.is_ok(), "{parsed:?}");
    }

    #[test]
    fn csl_launch_args_omits_base_dir_when_not_given() {
        let args = csl_launch_args(Utf8Path::new("/tmp/script.csl"), None, "amspirit").unwrap();
        assert!(!args.contains("--csl-base-dir"), "{args}");
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

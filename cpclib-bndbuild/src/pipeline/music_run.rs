//! Converts an Arkos Tracker source song into a standalone player program,
//! then either launches it in an emulator or packs it onto a DSK - built
//! entirely on the task-agnostic operations in the parent [`super`] module,
//! so this file has no knowledge of how the conversion/assembler tasks are
//! actually dispatched.
//!
//! Two completely different players, chosen automatically per song
//! ([`super::song_uses_sid`]):
//! - **Arkos Tracker players** - AKG by default, AKM/AKYS/AKYU on request (`music_arkos_harness.asm`, assembled with basm) for everything
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
use cpclib_runner::runner::tracker::chipnsfx::ChipnsfxVersion;

use crate::event::BndBuilderObserver;

/// Arkos-Tracker-compatible source-file extensions (lower-case, no leading
/// dot). The canonical list - kept here rather than in `cpclib-project`'s
/// `MusicConfig` (which depends on this crate, not the other way around) so
/// there is exactly one place that defines it. Verified against the real
/// formats Arkos Tracker 3 can import (its own "Import from..." feature
/// list): AKS (Arkos Tracker 1/2/3), SKS (STarKos), 128 (BSC's Soundtrakker),
/// VT2 (Vortex Tracker 2), WYZ (Wyz Tracker) - plus CHP (CHIPNSFX), which is
/// not an Arkos Tracker format at all and gets its own player (see
/// [`PlayerKind::Chp`]) - and YM (AY register dumps), played by the YM-based
/// players ([`MusicPlayer::Fap`] & co).
pub const DEFAULT_SONG_EXTENSIONS: &[&str] = &["aks", "sks", "128", "vt2", "wyz", "chp", "ym"];

/// The Arkos Tracker player-harness source (AKG, AKM, AKY), embedded at compile
/// time - see `music_arkos_harness.asm` next to this file for the full commented source
/// and the `{{PLACEHOLDER}}`s it expects substituted before assembling.
const ARKOS_HARNESS_SOURCE: &str = include_str!("music_arkos_harness.asm");

/// The CHIPNSFX player-harness source, embedded at compile time - see
/// `music_chp_harness.asm` next to this file.
const CHP_HARNESS_SOURCE: &str = include_str!("music_chp_harness.asm");

/// The FAP, AYT and MinYMiser player harnesses - the YM-based players, see
/// [`PlayerKind`] and `music_fap_harness.asm` & co.
const FAP_HARNESS_SOURCE: &str = include_str!("music_fap_harness.asm");
const AYT_HARNESS_SOURCE: &str = include_str!("music_ayt_harness.asm");
const MINY_HARNESS_SOURCE: &str = include_str!("music_miny_harness.asm");

/// The two YM players that are not downloaded by the tool that packs for them
/// - AYT's player builder (Logon System's AYT-Format) and MinYMiser's Z80 port
/// (Megachur's `ymp_z80.z80`) - are embedded, and written next to the harness
/// that includes them.
const AYT_BUILDER_SOURCE: &str = include_str!("players/AytPlayerBuilder-CPC.asm");
const YMP_SOURCE: &str = include_str!("players/ymp_z80.z80");

/// The SID player-harness source, embedded at compile time - see
/// `music_sid_harness.asm` next to this file for the full commented source.
const SID_HARNESS_SOURCE: &str = include_str!("music_sid_harness.asm");

/// The song-info printer pasted into both harnesses - see
/// `music_info_print.asm`.
const INFO_PRINT_SOURCE: &str = include_str!("music_info_print.asm");

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

/// Mode 2 text width, in characters - the harnesses print one `db` line per
/// text row, with no wrapping of their own.
const INFO_TEXT_COLUMNS: usize = 78;
/// Rows kept for the comment, so the text always fits the 25-row screen.
const INFO_TEXT_MAX_ROWS: usize = 20;

/// Printable-ASCII-only, safe inside a basm/rasm `db "..."` literal (no quote,
/// backslash or `{}` formatting braces; the ROM font only has 32..=127).
fn info_ascii(text: &str) -> String {
    text.chars()
        .map(|c| {
            match c {
                '"' => '\'',
                '\\' => '/',
                '{' => '(',
                '}' => ')',
                '\t' => ' ',
                c if (' '..='~').contains(&c) => c,
                'à' | 'â' | 'ä' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'î' | 'ï' => 'i',
                'ô' | 'ö' => 'o',
                'ù' | 'û' | 'ü' => 'u',
                'ç' => 'c',
                _ => '?'
            }
        })
        .collect()
}

/// Greedy word wrap of one paragraph to `INFO_TEXT_COLUMNS` (long words are cut).
fn info_wrap(paragraph: &str, out: &mut Vec<String>) {
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        let mut word = word;
        while word.len() > INFO_TEXT_COLUMNS {
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            out.push(word[..INFO_TEXT_COLUMNS].to_string());
            word = &word[INFO_TEXT_COLUMNS..];
        }
        if !line.is_empty() && line.len() + 1 + word.len() > INFO_TEXT_COLUMNS {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    out.push(line);
}

/// The text rows a player harness prints: title, author/composer, then the
/// comment. `fallback_title` (the song's file name) stands in for an empty title.
fn info_lines(meta: &super::SongMetadata, fallback_title: &str) -> Vec<String> {
    let meta_field = |s: &str| info_ascii(s).trim().to_string();
    // AT3's default for a freshly created song - not worth printing.
    let useful = |s: String| (!s.is_empty() && !s.eq_ignore_ascii_case("unknown")).then_some(s);

    let title = useful(meta_field(&meta.title)).unwrap_or_else(|| info_ascii(fallback_title));
    let author = useful(meta_field(&meta.author));
    let composer = useful(meta_field(&meta.composer));

    let mut lines = vec![title];
    match (author, composer) {
        (Some(a), Some(c)) if a != c => {
            lines.push(format!("by {a}"));
            lines.push(format!("composer: {c}"));
        },
        (Some(a), _) | (None, Some(a)) => lines.push(format!("by {a}")),
        (None, None) => {}
    }

    let mut comment_rows = Vec::new();
    for paragraph in meta.comment.lines() {
        info_wrap(&info_ascii(paragraph), &mut comment_rows);
    }
    while comment_rows.last().is_some_and(|l| l.is_empty()) {
        comment_rows.pop();
    }
    if !comment_rows.is_empty() {
        lines.push(String::new());
        lines.extend(comment_rows.into_iter().take(INFO_TEXT_MAX_ROWS));
    }
    lines
}

/// The `{{INFO_TEXT}}` replacement: one zero-terminated `db` string per row,
/// ended by a `255` marker (so an empty row, a lone `0`, stays distinguishable).
///
/// `player` names the player the harness embeds, printed as the last row,
/// after the tracker the song was written with when that is known.
fn info_text_db(song_path: &Utf8Path, name_hint: &str, player: &str) -> String {
    let mut lines = info_lines(&super::song_metadata(song_path), name_hint);
    lines.push(String::new());
    if let Some(tracker) = super::song_tracker(song_path) {
        lines.push(format!("Tracker: {tracker}"));
    }
    lines.push(format!("Player: {player}"));
    let mut out = String::new();
    for line in lines {
        if line.is_empty() {
            out.push_str("    db 0\n");
        }
        else {
            out.push_str(&format!("    db \"{line}\",0\n"));
        }
    }
    out.push_str("    db 255\n");
    out
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
/// `music_arkos_harness.asm`) means the load address doubles as the entry
/// point, without needing this code to know `Start`'s real address.
struct Build {
    _dir: camino_tempfile::Utf8TempDir,
    bin_path: Utf8PathBuf,
    bin_name: String,
    /// The size of the song, as the player reads it; `None` when that is not
    /// a binary file (CHIPNSFX's is Z80 source).
    song_bytes: Option<u64>
}

/// The AKG and CHP harnesses' AMSDOS binary load/execution address (both start
/// with `jp Start` at `org 0x500`) - see `Build`'s doc comment.
const BASM_PLAYER_LOAD_ADDRESS: u16 = 0x500;

/// The SID harness's AMSDOS binary load/execution address. Unlike AKG, this
/// needs no `jp Start`-at-byte-0 trick: `Start` already lands exactly at
/// `org`'s address (`music_sid_harness.asm`'s `org #100` / `Start equ $`
/// immediately after), with no fixed-address gap to reserve first.
const SID_LOAD_ADDRESS: u16 = 0x100;

/// The player the user asks for - [`MusicPlayer::Auto`] lets the song decide.
///
/// The YM-based ones (FAP, AYT, MinYMiser) play any song: anything that is
/// not a YM already is converted to one first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicPlayer {
    /// AKG for an Arkos Tracker song (or its SID player, if it uses that
    /// feature), CHIPNSFX's player for a `.chp`, FAP for a `.ym`.
    #[default]
    Auto,
    /// Arkos Tracker 3's AKG player - Arkos Tracker songs only.
    Akg,
    /// Arkos Tracker 3's AKM player (smaller songs, slower) - Arkos Tracker
    /// songs only.
    Akm,
    /// Arkos Tracker 3's AKY player, stabilized (constant CPU time) - Arkos
    /// Tracker songs only.
    Akys,
    /// Arkos Tracker 3's AKY player, unstabilized (the fastest) - Arkos Tracker
    /// songs only.
    Akyu,
    /// CHIPNSFX's player - `.chp` songs only.
    Chip,
    /// FAP, the Fast AY Player.
    Fap,
    /// AYT, Logon System's player builder.
    Ayt,
    /// MinYMiser's Z80 port.
    Miny
}

impl MusicPlayer {
    /// Every choice, in the order a menu should list them.
    pub const ALL: [MusicPlayer; 9] = [
        Self::Auto,
        Self::Akg,
        Self::Akm,
        Self::Akys,
        Self::Akyu,
        Self::Chip,
        Self::Fap,
        Self::Ayt,
        Self::Miny
    ];

    /// The name this player is given in `cpclib-lsp.toml` and in commands.
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Akg => "akg",
            Self::Akm => "akm",
            Self::Akys => "akys",
            Self::Akyu => "akyu",
            Self::Chip => "chipnsfx",
            Self::Fap => "fap",
            Self::Ayt => "ayt",
            Self::Miny => "miny"
        }
    }
}

impl std::str::FromStr for MusicPlayer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|p| p.name().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| {
                format!(
                    "unknown music player `{s}` (expected one of: {})",
                    Self::ALL.map(Self::name).join(", ")
                )
            })
    }
}

/// How to turn a song into a player program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicOptions {
    /// Only matters if the player ends up being the SID one
    /// (`MusicConfig::sid_wait_line_count`) - see `music_sid_harness.asm`'s
    /// own doc comment for what it controls.
    pub sid_wait_line_count: u16,
    pub player: MusicPlayer
}

impl Default for MusicOptions {
    fn default() -> Self {
        Self {
            sid_wait_line_count: 72,
            player: MusicPlayer::Auto
        }
    }
}

/// Which player a song is played with, once [`MusicPlayer::Auto`] is resolved
/// - see [`super::song_uses_sid`] for the SID one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerKind {
    /// One of Arkos Tracker's binary-format players.
    Arkos(ArkosPlayer),
    Sid,
    /// CHIPNSFX's own player, for `.chp` songs.
    Chp,
    Fap,
    Ayt,
    Miny
}

/// Arkos Tracker 3's binary-format players, which share one harness
/// (`music_arkos_harness.asm`) and one pipeline: only these few facts differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArkosPlayer {
    Akg,
    Akm,
    /// AKY, stabilized.
    Akys,
    /// AKY, unstabilized.
    Akyu
}

impl ArkosPlayer {
    /// Printed on the `Player:` row of the song-info screen.
    fn display_name(self) -> &'static str {
        match self {
            Self::Akg => "Arkos Tracker 3 AKG",
            Self::Akm => "Arkos Tracker 3 AKM",
            Self::Akys => "Arkos Tracker 3 AKY (stabilized)",
            Self::Akyu => "Arkos Tracker 3 AKY (unstabilized)"
        }
    }

    /// The AT3 tool that writes the song in this player's format.
    fn converter(self) -> crate::runners::tracker::SongConverter {
        use crate::runners::tracker::SongConverter;
        match self {
            Self::Akg => SongConverter::new_song_to_akg_default(),
            Self::Akm => SongConverter::new_song_to_akm_default(),
            Self::Akys | Self::Akyu => SongConverter::new_song_to_aky_default()
        }
    }

    /// The player routine, in the AT3 install.
    fn source_path(self) -> Utf8PathBuf {
        let at3 = At3Version::default();
        match self {
            Self::Akg => at3.akg_path::<()>(),
            Self::Akm => at3.akm_path::<()>(),
            Self::Akys => at3.aky_stable_path::<()>(),
            Self::Akyu => at3.aky_path::<()>()
        }
    }

    /// `{{PLAYER_DEFINES}}`: the switches the player reads.
    fn defines(self) -> &'static str {
        match self {
            Self::Akg => "PLY_AKG_REMOVE_HOOKS\nPLY_AKG_HARDWARE_CPC = 1",
            Self::Akm => "PLY_AKM_REMOVE_HOOKS\nPLY_AKM_HARDWARE_CPC = 1",
            Self::Akys => "",
            Self::Akyu => "PLY_AKY_REMOVE_HOOKS = 1\nPLY_AKY_HARDWARE_CPC = 1"
        }
    }

    /// `{{PLAYER_INIT_PRE}}`: the init routine's inputs besides HL (the subsong).
    fn init_pre(self) -> &'static str {
        match self {
            Self::Akys => "",
            _ => "xor a"
        }
    }

    fn init(self) -> &'static str {
        match self {
            Self::Akg => "PLY_AKG_Init",
            Self::Akm => "PLY_AKM_Init",
            Self::Akys => "PLY_AKYst_Init",
            Self::Akyu => "PLY_AKY_Init"
        }
    }

    fn play(self) -> &'static str {
        match self {
            Self::Akg => "PLY_AKG_Play",
            Self::Akm => "PLY_AKM_Play",
            Self::Akys => "PLY_AKYst_Play",
            Self::Akyu => "PLY_AKY_Play"
        }
    }
}

fn player_kind(song_path: &Utf8Path, requested: MusicPlayer) -> Result<PlayerKind, String> {
    let is_arkos = !super::song_is_chp(song_path) && !super::song_is_ym(song_path);
    match requested {
        MusicPlayer::Auto => {
            if super::song_is_chp(song_path) {
                Ok(PlayerKind::Chp)
            }
            else if super::song_is_ym(song_path) {
                Ok(PlayerKind::Fap)
            }
            else if super::song_uses_sid(song_path)? {
                Ok(PlayerKind::Sid)
            }
            else {
                Ok(PlayerKind::Arkos(ArkosPlayer::Akg))
            }
        },
        MusicPlayer::Akg | MusicPlayer::Akm | MusicPlayer::Akys | MusicPlayer::Akyu
            if !is_arkos =>
        {
            Err(format!(
                "The {} player needs an Arkos Tracker song, not {song_path}",
                requested.name()
            ))
        },
        MusicPlayer::Akg => Ok(PlayerKind::Arkos(ArkosPlayer::Akg)),
        MusicPlayer::Akm => Ok(PlayerKind::Arkos(ArkosPlayer::Akm)),
        MusicPlayer::Akys => Ok(PlayerKind::Arkos(ArkosPlayer::Akys)),
        MusicPlayer::Akyu => Ok(PlayerKind::Arkos(ArkosPlayer::Akyu)),
        MusicPlayer::Chip if super::song_is_chp(song_path) => Ok(PlayerKind::Chp),
        MusicPlayer::Chip => {
            Err(format!(
                "The CHIPNSFX player needs a .chp song, not {song_path}"
            ))
        },
        MusicPlayer::Fap => Ok(PlayerKind::Fap),
        MusicPlayer::Ayt => Ok(PlayerKind::Ayt),
        MusicPlayer::Miny => Ok(PlayerKind::Miny)
    }
}

/// Everything that differs between the basm-assembled players (AKG, CHP, FAP,
/// AYT, MinYMiser) -
/// see [`assemble_basm_player`].
struct BasmPlayer<'a> {
    /// The harness source, `{{PLACEHOLDER}}`s still in.
    template: &'a str,
    /// Printed on the `Player:` row of the song-info screen.
    player_name: &'a str,
    /// The converted song, `{{MUSIC_DATA_FNAME}}`.
    music_path: &'a Utf8Path,
    /// Whether `music_path` is a binary the player reads (not source, as for
    /// CHIPNSFX) - its size is then the song's.
    song_is_binary: bool,
    /// Harness-specific placeholders, already escaped.
    extra_substitutions: &'a [(&'a str, String)]
}

/// Substitutes `{{PLACEHOLDER}}`s into a player harness: the song-info ones
/// every harness shares (`INFO_PRINT_CODE`, `INFO_TEXT`) plus the caller's
/// own `substitutions`, whose values must already be escaped for the target
/// assembler.
fn fill_harness(
    template: &str,
    song_path: &Utf8Path,
    name_hint: &str,
    player_name: &str,
    substitutions: &[(&str, String)]
) -> String {
    let mut source = template
        .replace("{{INFO_PRINT_CODE}}", INFO_PRINT_SOURCE)
        .replace(
            "{{INFO_TEXT}}",
            &info_text_db(song_path, name_hint, player_name)
        );
    for (placeholder, value) in substitutions {
        source = source.replace(placeholder, value);
    }
    source
}

/// Assembles an already-converted song into a standalone player with basm,
/// inside `dir` (kept alive by the returned [`Build`]): the harness is written
/// there, and assembled into a headerless `<name_hint>.BIN` (the harness's
/// own `save`). `extra_asm_args` is where the callers ask for a snapshot too.
fn assemble_basm_player<E: BndBuilderObserver + 'static>(
    dir: camino_tempfile::Utf8TempDir,
    player: &BasmPlayer,
    song_path: &Utf8Path,
    name_hint: &str,
    extra_asm_args: &[String],
    observer: &Arc<E>
) -> Result<Build, String> {
    let bin_name = format!("{}.BIN", super::sanitize_amsdos_stem(name_hint));
    let bin_path = dir.path().join(&bin_name);

    let mut substitutions = vec![
        ("{{MUSIC_DATA_FNAME}}", basm_escaped_path(player.music_path)),
        ("{{MUSIC_EXEC_FNAME}}", basm_escaped_path(&bin_path)),
    ];
    substitutions.extend(player.extra_substitutions.iter().cloned());

    let harness_source = fill_harness(
        player.template,
        song_path,
        name_hint,
        player.player_name,
        &substitutions
    );
    let harness_path = dir.path().join("harness.asm");
    fs_err::write(&harness_path, harness_source)
        .map_err(|e| format!("Could not write the player harness: {e}"))?;

    super::assemble_source(&harness_path, extra_asm_args, observer)
        .map_err(|e| format!("Could not assemble the player harness: {e}"))?;

    let song_bytes = player
        .song_is_binary
        .then(|| fs_err::metadata(player.music_path).map(|m| m.len()))
        .transpose()
        .map_err(|e| format!("Could not measure the converted song: {e}"))?;

    Ok(Build {
        _dir: dir,
        bin_path,
        bin_name,
        song_bytes
    })
}

/// Converts `song_path` to `player`'s format and assembles the Arkos harness
/// around it, naming the resulting AMSDOS binary from `name_hint` (sanitized
/// the same way `basic_run` names its `.BAS` file). `extra_asm_args` lets the
/// two public entry points below differ only in whether they also ask for a
/// snapshot - everything else (conversion, path substitution, harness source)
/// is identical, matching how the Python reference's `__build_replay_program__`
/// builds both in one assemble call.
fn convert_and_assemble_arkos<E: BndBuilderObserver + 'static>(
    player: ArkosPlayer,
    song_path: &Utf8Path,
    name_hint: &str,
    extra_asm_args: &[String],
    observer: &Arc<E>
) -> Result<Build, String> {
    let dir = camino_tempfile::tempdir()
        .map_err(|e| format!("Could not create a temp working directory: {e}"))?;

    let music_path = dir.path().join("song.bin");
    super::convert_song_to_arkos_binary(player.converter(), song_path, &music_path, observer)
        .map_err(|e| {
            format!(
                "Could not convert {song_path} for {}: {e}",
                player.display_name()
            )
        })?;

    // AT3's own naming convention for `--exportPlayerConfig`'s companion
    // file: `output_path` with its extension stripped, `_playerconfig.asm`
    // appended - see `super::convert_song_to_akg`'s doc comment.
    let player_config_path = Utf8PathBuf::from(format!(
        "{}_playerconfig.asm",
        music_path.with_extension("")
    ));

    assemble_basm_player(
        dir,
        &BasmPlayer {
            template: ARKOS_HARNESS_SOURCE,
            player_name: player.display_name(),
            music_path: &music_path,
            song_is_binary: true,
            extra_substitutions: &[
                (
                    "{{PLAYER_CONFIG_FNAME}}",
                    basm_escaped_path(&player_config_path)
                ),
                (
                    "{{PLAYER_SOURCE_FNAME}}",
                    basm_escaped_path(&player.source_path())
                ),
                ("{{PLAYER_DEFINES}}", player.defines().to_string()),
                ("{{PLAYER_INIT_PRE}}", player.init_pre().to_string()),
                ("{{PLAYER_INIT}}", player.init().to_string()),
                ("{{PLAYER_PLAY}}", player.play().to_string())
            ]
        },
        song_path,
        name_hint,
        extra_asm_args,
        observer
    )
}

/// Converts the CHIPNSFX song `song_path` (`.chp`) and assembles the CHP
/// harness around it - same shape as [`convert_and_assemble_arkos`], whose
/// `Build` it returns (headerless binary, wrapped in an AMSDOS header by the
/// caller).
fn convert_and_assemble_chp<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    extra_asm_args: &[String],
    observer: &Arc<E>
) -> Result<Build, String> {
    let dir = camino_tempfile::tempdir()
        .map_err(|e| format!("Could not create a temp working directory: {e}"))?;

    let music_path = dir.path().join("song.chpz80");
    // Converting first: it is what downloads CHIPNSFX, whose `CHIPNSFX.I80`
    // player the harness includes.
    super::convert_chp_to_z80(song_path, &music_path, observer)
        .map_err(|e| format!("Could not convert {song_path} with chipnsfx: {e}"))?;

    assemble_basm_player(
        dir,
        &BasmPlayer {
            template: CHP_HARNESS_SOURCE,
            player_name: "CHIPNSFX",
            music_path: &music_path,
            song_is_binary: false,
            extra_substitutions: &[(
                "{{PLAYER_SOURCE_FNAME}}",
                basm_escaped_path(&ChipnsfxVersion::default().player_path::<()>())
            )]
        },
        song_path,
        name_hint,
        extra_asm_args,
        observer
    )
}

#[cfg(feature = "fap")]
fn fap_cruncher() -> Result<crate::runners::ay::YmCruncher, String> {
    Ok(crate::runners::ay::YmCruncher::Fap)
}

#[cfg(not(feature = "fap"))]
fn fap_cruncher() -> Result<crate::runners::ay::YmCruncher, String> {
    Err("This build of bndbuild has no FAP support".to_string())
}

/// The FAP player's two binaries, only known once `fap` has been downloaded.
#[cfg(feature = "fap")]
fn fap_player_paths() -> Result<(Utf8PathBuf, Utf8PathBuf), String> {
    let fap = cpclib_runner::runner::ay::fap::FAPVersion::default();
    Ok((fap.fap_init_path::<()>(), fap.fap_play_path::<()>()))
}

#[cfg(not(feature = "fap"))]
fn fap_player_paths() -> Result<(Utf8PathBuf, Utf8PathBuf), String> {
    Err("This build of bndbuild has no FAP support".to_string())
}

/// Converts `song_path` to a YM, packs it for `kind` (FAP, AYT or MinYMiser)
/// and assembles that player's harness around it - same shape as
/// [`convert_and_assemble_arkos`], whose `Build` it returns.
fn convert_and_assemble_ym_player<E: BndBuilderObserver + 'static>(
    kind: PlayerKind,
    song_path: &Utf8Path,
    name_hint: &str,
    extra_asm_args: &[String],
    observer: &Arc<E>
) -> Result<Build, String> {
    use super::YmVersion;
    use crate::runners::ay::YmCruncher;

    let (version, extension, cruncher, template, player_name) = match kind {
        PlayerKind::Fap => {
            (
                YmVersion::Ym6,
                "fap",
                fap_cruncher()?,
                FAP_HARNESS_SOURCE,
                "FAP (Fast AY Player)"
            )
        },
        PlayerKind::Ayt => {
            (
                YmVersion::Ym6,
                "ayt",
                YmCruncher::Ayt,
                AYT_HARNESS_SOURCE,
                "AYT"
            )
        },
        PlayerKind::Miny => {
            (
                YmVersion::Ym3,
                "miny",
                YmCruncher::Miny,
                MINY_HARNESS_SOURCE,
                "MinYMiser"
            )
        },
        PlayerKind::Arkos(_) | PlayerKind::Sid | PlayerKind::Chp => {
            return Err("not a YM-based player".to_string());
        }
    };

    let dir = camino_tempfile::tempdir()
        .map_err(|e| format!("Could not create a temp working directory: {e}"))?;

    let ym_path = super::ym_for_player(song_path, &dir.path().join("song.ym"), version, observer)?;
    let packed_path = dir.path().join(format!("song.{extension}"));
    let report = super::pack_ym(cruncher, &ym_path, &packed_path, observer)
        .map_err(|e| format!("Could not pack {ym_path} for {player_name}: {e}"))?;

    let mut asm_args = extra_asm_args.to_vec();
    let mut substitutions = Vec::new();
    let buffer_size = |label: &str| {
        super::tool_reported_size(&report, label)
            .map(|n| n.to_string())
            .ok_or_else(|| format!("{player_name}'s packer did not report a `{label}`"))
    };
    match kind {
        PlayerKind::Fap => {
            // only now: the packing is what downloads FAP
            let (init, play) = fap_player_paths()?;
            substitutions.push(("{{FAP_INIT_PATH}}", basm_escaped_path(&init)));
            substitutions.push(("{{FAP_PLAY_PATH}}", basm_escaped_path(&play)));
            substitutions.push(("{{MUSIC_BUFF_SIZE}}", buffer_size("Decrunch buffer size")?));
        },
        PlayerKind::Ayt => {
            let builder = dir.path().join("AytPlayerBuilder-CPC.asm");
            fs_err::write(&builder, AYT_BUILDER_SOURCE)
                .map_err(|e| format!("Could not write the AYT player builder: {e}"))?;
            substitutions.push(("{{AYT_BUILDER_FNAME}}", basm_escaped_path(&builder)));
            // the builder is written for a case-insensitive assembler
            asm_args.insert(0, "--case-insensitive".to_string());
        },
        PlayerKind::Miny => {
            let ymp = dir.path().join("ymp_z80.z80");
            fs_err::write(&ymp, YMP_SOURCE)
                .map_err(|e| format!("Could not write the MinYMiser player: {e}"))?;
            substitutions.push(("{{YMP_FNAME}}", basm_escaped_path(&ymp)));
            substitutions.push(("{{MUSIC_BUFF_SIZE}}", buffer_size("Total cache size")?));
        },
        PlayerKind::Arkos(_) | PlayerKind::Sid | PlayerKind::Chp => unreachable!()
    }

    assemble_basm_player(
        dir,
        &BasmPlayer {
            template,
            player_name,
            music_path: &packed_path,
            song_is_binary: true,
            extra_substitutions: &substitutions
        },
        song_path,
        name_hint,
        &asm_args,
        observer
    )
}

/// The `convert_and_assemble_*` function `kind` needs, among the basm-assembled
/// players - which differ only in their conversion.
fn convert_and_assemble_basm_kind<E: BndBuilderObserver + 'static>(
    kind: PlayerKind,
    song_path: &Utf8Path,
    name_hint: &str,
    extra_asm_args: &[String],
    observer: &Arc<E>
) -> Result<Build, String> {
    match kind {
        PlayerKind::Chp => convert_and_assemble_chp(song_path, name_hint, extra_asm_args, observer),
        PlayerKind::Arkos(player) => {
            convert_and_assemble_arkos(player, song_path, name_hint, extra_asm_args, observer)
        },
        PlayerKind::Sid => Err("The SID player is not assembled with basm".to_string()),
        PlayerKind::Fap | PlayerKind::Ayt | PlayerKind::Miny => {
            convert_and_assemble_ym_player(kind, song_path, name_hint, extra_asm_args, observer)
        },
    }
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
    name_hint: &str,
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

    let aky_sid_macros_path = At3Version::default().aky_sid_macros_path::<()>();
    let harness_source = fill_harness(
        SID_HARNESS_SOURCE,
        song_path,
        name_hint,
        "Arkos Tracker 3 AKY (SID)",
        &[
            ("{{MUSIC_DATA_FNAME}}", basm_escaped_path(&music_path)),
            (
                "{{PLAYER_SOURCE_FNAME}}",
                basm_escaped_path(&At3Version::default().aky_sid_path::<()>())
            ),
            (
                "{{PLAYER_MACROS_FNAME}}",
                basm_escaped_path(&aky_sid_macros_path)
            ),
            ("{{WAIT_LINE_COUNT}}", sid_wait_line_count.to_string()),
            (
                "{{BUILDSNA_DIRECTIVES}}",
                if wants_snapshot {
                    "        buildsna\n        bankset 0".to_string()
                }
                else {
                    String::new()
                }
            )
        ]
    );
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
/// `options` says which player to use (by default, the one the song needs),
/// and the SID player's safety margin - see [`MusicOptions`].
pub fn run_music_in_emulator<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    emulator: &str,
    options: &MusicOptions,
    observer: &Arc<E>
) -> MusicRunOutcome {
    if !song_path.is_file() {
        return failure(format!("{song_path} does not exist"));
    }

    let kind = match player_kind(song_path, options.player) {
        Ok(k) => k,
        Err(e) => return failure(e)
    };

    let dir = match camino_tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return failure(format!("Could not create a temp working directory: {e}"))
    };
    let sna_path = dir.path().join("song.sna");

    let result = match kind {
        PlayerKind::Arkos(_)
        | PlayerKind::Chp
        | PlayerKind::Fap
        | PlayerKind::Ayt
        | PlayerKind::Miny => {
            convert_and_assemble_basm_kind(
                kind,
                song_path,
                name_hint,
                &[
                    "--snapshot".to_string(),
                    "-o".to_string(),
                    sna_path.to_string()
                ],
                observer
            )
            .map(|_| ())
        },
        PlayerKind::Sid => {
            convert_and_assemble_sid(
                song_path,
                name_hint,
                options.sid_wait_line_count,
                true,
                &["-oi".to_string(), sna_path.to_string()],
                observer
            )
        },
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

/// What one player makes of a song - see [`compare_music_players`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerComparison {
    pub player: MusicPlayer,
    pub outcome: Result<PlayerSizes, String>
}

/// How big a player program is, for [`PlayerComparison`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSizes {
    /// The song, converted to the player's own format. `None` for CHIPNSFX,
    /// whose song is Z80 source: the program size is the figure to look at.
    pub song_bytes: Option<u64>,
    /// The player's code alone (from the harness's `PlayerStart`/`PlayerEnd`
    /// labels, in the assembler's symbol table). For AYT, that is the builder:
    /// the player it writes at run time, 250 to 340 bytes, comes on top.
    pub player_bytes: Option<u64>,
    /// The whole player program: song, player, and the song-info screen.
    pub program_bytes: u64
}

/// Builds `song_path` with every player that can play it (the Arkos Tracker
/// ones for an Arkos Tracker song, CHIPNSFX's for a `.chp`, the YM-based ones
/// for anything), and measures each - to compare how heavy they are on the
/// same song. A player that fails (a tool missing, a song it cannot play) is
/// reported as such: it does not stop the others.
///
/// The SID player is not part of it: it is not a choice, but what a SID song
/// needs.
pub fn compare_music_players<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    observer: &Arc<E>
) -> Result<Vec<PlayerComparison>, String> {
    if !song_path.is_file() {
        return Err(format!("{song_path} does not exist"));
    }

    let symbols_dir = camino_tempfile::tempdir()
        .map_err(|e| format!("Could not create a temp working directory: {e}"))?;

    Ok(MusicPlayer::ALL
        .into_iter()
        .filter(|&p| p != MusicPlayer::Auto)
        .filter_map(|player| {
            let kind = player_kind(song_path, player).ok()?;
            let symbols_path = symbols_dir.path().join(format!("{}.sym", player.name()));
            let asm_args = ["--sym".to_string(), symbols_path.to_string()];
            let outcome =
                convert_and_assemble_basm_kind(kind, song_path, name_hint, &asm_args, observer)
                    .and_then(|build| {
                        let program_bytes = fs_err::metadata(&build.bin_path)
                            .map_err(|e| format!("Could not measure the program: {e}"))?
                            .len();
                        let player_bytes = fs_err::read_to_string(&symbols_path)
                            .ok()
                            .and_then(|symbols| player_code_size(&symbols));
                        Ok(PlayerSizes {
                            song_bytes: build.song_bytes,
                            player_bytes,
                            program_bytes
                        })
                    });
            Some(PlayerComparison { player, outcome })
        })
        .collect())
}

/// The value of the symbol `name` in basm's symbol table dump (`Name equ
/// #1A2B` lines).
fn symbol_value(symbols: &str, name: &str) -> Option<u64> {
    symbols.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        // (a case-insensitive assembly - AYT's - writes them in lower case)
        if !words.next()?.eq_ignore_ascii_case(name) {
            return None;
        }
        let value = words
            .skip_while(|w| !w.eq_ignore_ascii_case("equ"))
            .nth(1)?;
        match value
            .strip_prefix('#')
            .or_else(|| value.strip_prefix('$'))
            .or_else(|| value.strip_prefix("0x"))
        {
            Some(hex) => u64::from_str_radix(hex, 16).ok(),
            None => value.parse().ok()
        }
    })
}

/// How many bytes the harness's `PlayerStart`..`PlayerEnd` span, from the
/// assembler's symbol table.
fn player_code_size(symbols: &str) -> Option<u64> {
    symbol_value(symbols, "PlayerEnd")?.checked_sub(symbol_value(symbols, "PlayerStart")?)
}

/// `comparisons` as a plain-text table, smallest program first (failures
/// last).
pub fn format_player_comparison(comparisons: &[PlayerComparison]) -> String {
    let mut rows: Vec<&PlayerComparison> = comparisons.iter().collect();
    rows.sort_by_key(|c| c.outcome.as_ref().map_or(u64::MAX, |s| s.program_bytes));

    let bytes = |b: Option<u64>| b.map_or_else(|| "-".to_string(), |b| b.to_string());
    let mut table = format!(
        "{:<10} {:>7} {:>12} {:>9}\n",
        "player", "song", "player code", "program"
    );
    for row in rows {
        match &row.outcome {
            Ok(sizes) => {
                table.push_str(&format!(
                    "{:<10} {:>7} {:>12} {:>9}\n",
                    row.player.name(),
                    bytes(sizes.song_bytes),
                    bytes(sizes.player_bytes),
                    sizes.program_bytes
                ));
            },
            Err(e) => {
                let first_line = e.lines().next().unwrap_or("failed");
                table.push_str(&format!("{:<10} failed: {first_line}\n", row.player.name()));
            }
        }
    }
    table.push_str(
        "\nIn bytes. \"program\" is song + player + the song-info screen. AYT's player code is\nits builder: the player it writes at run time (250 to 340 bytes) comes on top.\n"
    );
    table
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
    let bytes =
        fs_err::read(bin_path).map_err(|e| format!("Could not read the assembled binary: {e}"))?;
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
/// `options`: see [`run_music_in_emulator`].
pub fn build_music_dsk<E: BndBuilderObserver + 'static>(
    song_path: &Utf8Path,
    name_hint: &str,
    options: &MusicOptions,
    observer: &Arc<E>
) -> Result<Utf8PathBuf, String> {
    if !song_path.is_file() {
        return Err(format!("{song_path} does not exist"));
    }

    match player_kind(song_path, options.player)? {
        kind @ (PlayerKind::Arkos(_)
        | PlayerKind::Chp
        | PlayerKind::Fap
        | PlayerKind::Ayt
        | PlayerKind::Miny) => {
            // Both players load, and are entered, at the same address.
            let built = convert_and_assemble_basm_kind(kind, song_path, name_hint, &[], observer)?;
            wrap_and_build_dsk(
                &built.bin_path,
                &built.bin_name,
                BASM_PLAYER_LOAD_ADDRESS,
                observer
            )
        },
        PlayerKind::Sid => {
            let dir = camino_tempfile::tempdir()
                .map_err(|e| format!("Could not create a temp working directory: {e}"))?;
            let bin_name = format!("{}.BIN", super::sanitize_amsdos_stem(name_hint));
            let bin_path = dir.path().join(&bin_name);

            convert_and_assemble_sid(
                song_path,
                name_hint,
                options.sid_wait_line_count,
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
    fn info_lines_lists_title_author_and_wrapped_comment() {
        let meta = super::super::SongMetadata {
            title: "My \"song\" {x}".into(),
            author: "Zoë\u{2603}".into(),
            composer: "Unknown".into(),
            comment: format!("{}\n\nsecond", "word ".repeat(30))
        };
        let lines = info_lines(&meta, "FALLBACK");
        assert_eq!(lines[0], "My 'song' (x)");
        assert_eq!(lines[1], "by Zoe?");
        assert_eq!(lines[2], "");
        assert!(lines[3].len() <= INFO_TEXT_COLUMNS && lines[4].len() <= INFO_TEXT_COLUMNS);
        assert_eq!(lines.last().unwrap(), "second");
    }

    #[test]
    fn info_lines_falls_back_to_the_name_when_there_is_no_metadata() {
        let lines = info_lines(&super::super::SongMetadata::default(), "tune");
        assert_eq!(lines, vec!["tune".to_string()]);
    }

    #[test]
    fn chp_songs_are_detected_by_extension_and_get_their_own_player() {
        assert!(matches!(
            player_kind(Utf8Path::new("/nowhere/Song.CHP"), MusicPlayer::Auto),
            Ok(PlayerKind::Chp)
        ));
        assert!(DEFAULT_SONG_EXTENSIONS.contains(&"chp"));
    }

    #[test]
    fn info_text_names_the_tracker_when_it_is_known() {
        let aks = info_text_db(Utf8Path::new("/nowhere/x.AKS"), "tune", "FAP");
        assert!(aks.contains("db \"Tracker: Arkos Tracker\",0"), "{aks}");
        assert!(aks.find("Tracker:") < aks.find("Player:"));
        let ym = info_text_db(Utf8Path::new("/nowhere/x.ym"), "tune", "FAP");
        assert!(!ym.contains("Tracker:"), "{ym}");
    }

    #[test]
    fn info_text_names_the_player() {
        let db = info_text_db(Utf8Path::new("/nowhere/x.chp"), "tune", "CHIPNSFX");
        assert!(db.contains("db \"Player: CHIPNSFX\",0"), "{db}");
        assert!(db.trim_end().ends_with("db 255"));
    }

    #[test]
    fn chp_metadata_is_the_header_before_the_separator() {
        let meta = crate::pipeline::song_metadata(Utf8Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/chipnsfx/WINGSOD5.CHP"
        )));
        assert_eq!(meta.title, "Wings of Death #5 1990 Thalion");
        assert_eq!(meta.comment, "by CNGSOFT after Jochen Hippel (Madmax)");
    }

    /// Real CHIPNSFX song through the real `chipnsfx` tool (needs it downloaded,
    /// and wine on Linux) - hence `#[ignore]`d.
    #[test]
    #[ignore]
    fn real_chp_fixture_builds_a_real_dsk() {
        let observer = Arc::new(TestObserver);
        let song = Utf8PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/chipnsfx/WINGSOD5.CHP"
        ));
        let dsk = build_music_dsk(&song, "WINGS", &MusicOptions::default(), &observer)
            .expect("the real CHP conversion+assemble+DSK pipeline should succeed");
        assert!(dsk.is_file());
    }

    #[test]
    fn players_are_chosen_by_name_and_checked_against_the_song() {
        assert_eq!("FAP".parse::<MusicPlayer>(), Ok(MusicPlayer::Fap));
        assert_eq!(" chipnsfx ".parse::<MusicPlayer>(), Ok(MusicPlayer::Chip));
        assert!(
            "nope"
                .parse::<MusicPlayer>()
                .unwrap_err()
                .contains("expected one of")
        );

        let ym = Utf8Path::new("/nowhere/tune.ym");
        assert_eq!(player_kind(ym, MusicPlayer::Auto), Ok(PlayerKind::Fap));
        assert_eq!(player_kind(ym, MusicPlayer::Miny), Ok(PlayerKind::Miny));
        assert!(player_kind(ym, MusicPlayer::Akg).is_err());
        assert!(player_kind(ym, MusicPlayer::Chip).is_err());
        // any song can go through a YM-based player
        let chp = Utf8Path::new("/nowhere/tune.chp");
        assert_eq!(player_kind(chp, MusicPlayer::Ayt), Ok(PlayerKind::Ayt));
    }

    #[test]
    fn ym_metadata_reads_the_strings_after_the_header_and_digidrums() {
        let ym = fs_err::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/ay_players/ym/Targhan - Hocus Pocus.ym"
        ))
        .unwrap();
        let meta = crate::pipeline::ym_metadata(&ym);
        assert_eq!(meta.title, "Hocus Pocus - Main");
        assert_eq!(meta.author, "Targhan");
        assert_eq!(meta.comment, "For Tom's Opus Pocus");
        assert_eq!(
            crate::pipeline::ym_metadata(b"YM3!whatever"),
            crate::pipeline::SongMetadata::default()
        );
    }

    #[test]
    fn a_ym6_goes_down_to_a_ym3_register_by_register() {
        let ym = fs_err::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/ay_players/ym/Targhan - Hocus Pocus.ym"
        ))
        .unwrap();
        let ym3 = crate::pipeline::ym6_to_ym3(&ym).expect("a real YM6");
        let layout = crate::pipeline::Ym6Layout::parse(&ym).unwrap();
        let (start, frames) = (layout.data_start, layout.frames);
        assert!(layout.interleaved);
        assert_eq!(frames, 7060);
        assert_eq!(&ym3[..4], b"YM3!");
        assert_eq!(ym3.len(), 4 + 14 * frames);
        // register 1 of frame 5, in both layouts
        assert_eq!(ym3[4 + frames + 5], ym[start + frames + 5]);
        assert!(crate::pipeline::ym6_to_ym3(b"YM3!short").is_none());
    }

    #[test]
    fn tool_reported_sizes_are_found_in_the_packers_output() {
        let out = "Summary:\n  - Decrunch buffer size: 3144 (#C48)\nTotal cache size:   1248\n";
        assert_eq!(
            crate::pipeline::tool_reported_size(out, "Decrunch buffer size"),
            Some(3144)
        );
        assert_eq!(
            crate::pipeline::tool_reported_size(out, "Total cache size"),
            Some(1248)
        );
        assert_eq!(crate::pipeline::tool_reported_size(out, "Nothing"), None);
    }

    /// Every YM-based player on every kind of song, through the real tools
    /// (downloaded on demand; wine on Linux) - hence `#[ignore]`d.
    #[test]
    #[ignore]
    fn real_ym_players_build_a_real_dsk_from_every_kind_of_song() {
        use cpclib_runner::delegated::InternetStaticCompiledApplication as _;

        let observer = Arc::new(TestObserver);
        let tests = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
        let songs = [
            tests.join("ay_players/ym/Targhan - Hocus Pocus.ym"),
            tests.join("chipnsfx/WINGSOD5.CHP"),
            At3Version::default()
                .configuration::<()>()
                .cache_folder()
                .join("songs")
                .join("ArkosTracker3")
                .join("Ok3anos - Cpc Dream.aks")
        ];
        for song in &songs {
            for player in [MusicPlayer::Fap, MusicPlayer::Ayt, MusicPlayer::Miny] {
                let options = MusicOptions {
                    player,
                    ..MusicOptions::default()
                };
                let dsk = build_music_dsk(song, "YMTEST", &options, &observer)
                    .unwrap_or_else(|e| panic!("{player:?} on {song}: {e}"));
                assert!(dsk.is_file(), "{player:?} on {song}");
            }
        }
    }

    #[test]
    fn arkos_players_only_play_arkos_songs() {
        let aks = Utf8Path::new("/nowhere/tune.aks");
        assert_eq!(
            player_kind(aks, MusicPlayer::Akys),
            Ok(PlayerKind::Arkos(ArkosPlayer::Akys))
        );
        for player in [
            MusicPlayer::Akg,
            MusicPlayer::Akm,
            MusicPlayer::Akys,
            MusicPlayer::Akyu
        ] {
            let err = player_kind(Utf8Path::new("/nowhere/tune.ym"), player).unwrap_err();
            assert!(err.contains(player.name()), "{err}");
        }
    }

    /// Every Arkos Tracker player on a real song, through the real tools
    /// (downloaded on demand) - hence `#[ignore]`d.
    #[test]
    #[ignore]
    fn real_arkos_players_build_a_real_dsk() {
        use cpclib_runner::delegated::InternetStaticCompiledApplication as _;

        let observer = Arc::new(TestObserver);
        let song = At3Version::default()
            .configuration::<()>()
            .cache_folder()
            .join("songs")
            .join("ArkosTracker3")
            .join("Ok3anos - Cpc Dream.aks");
        for player in [
            MusicPlayer::Akg,
            MusicPlayer::Akm,
            MusicPlayer::Akys,
            MusicPlayer::Akyu
        ] {
            let options = MusicOptions {
                player,
                ..MusicOptions::default()
            };
            let dsk = build_music_dsk(&song, "ARKOS", &options, &observer)
                .unwrap_or_else(|e| panic!("{player:?}: {e}"));
            assert!(dsk.is_file(), "{player:?}");
        }
    }

    #[test]
    fn the_player_code_size_comes_from_the_symbol_table() {
        let symbols =
            "FontBuf equ #4194\nPlayerEnd equ #3A00\nStart equ #35C1\nPlayerStart equ #3000\n";
        assert_eq!(symbol_value(symbols, "Start"), Some(0x35C1));
        assert_eq!(symbol_value(symbols, "Nope"), None);
        assert_eq!(player_code_size(symbols), Some(0xA00));
        assert_eq!(player_code_size("PlayerStart equ 10\n"), None);
    }

    #[test]
    fn the_comparison_table_lists_the_smallest_first_and_failures_last() {
        let ok = |player, song, program| {
            PlayerComparison {
                player,
                outcome: Ok(PlayerSizes {
                    song_bytes: song,
                    player_bytes: Some(program / 2),
                    program_bytes: program
                })
            }
        };
        let table = format_player_comparison(&[
            PlayerComparison {
                player: MusicPlayer::Miny,
                outcome: Err("wine is missing\nsecond line".to_string())
            },
            ok(MusicPlayer::Akg, Some(900), 4000),
            ok(MusicPlayer::Chip, None, 2000)
        ]);
        let names: Vec<&str> = table
            .lines()
            .skip(1)
            .take(3)
            .map(|l| l.split_whitespace().next().unwrap())
            .collect();
        assert_eq!(names, ["chipnsfx", "akg", "miny"]);
        assert!(table.contains("failed: wine is missing"));
        assert!(!table.contains("second line"));
    }

    /// Every player that can play a real song, through the real tools -
    /// hence `#[ignore]`d.
    #[test]
    #[ignore]
    fn real_comparison_measures_every_compatible_player() {
        use cpclib_runner::delegated::InternetStaticCompiledApplication as _;

        let observer = Arc::new(TestObserver);
        let song = At3Version::default()
            .configuration::<()>()
            .cache_folder()
            .join("songs")
            .join("ArkosTracker3")
            .join("Ok3anos - Cpc Dream.aks");
        let comparisons = compare_music_players(&song, "CMP", &observer).unwrap();
        // 4 Arkos Tracker players + 3 YM-based ones; not CHIPNSFX's
        assert_eq!(comparisons.len(), 7, "{comparisons:?}");
        for c in &comparisons {
            let sizes = c
                .outcome
                .as_ref()
                .unwrap_or_else(|e| panic!("{:?}: {e}", c.player));
            assert!(sizes.program_bytes > sizes.song_bytes.unwrap());
            assert!(
                sizes
                    .player_bytes
                    .is_some_and(|p| p > 100 && p < sizes.program_bytes),
                "{:?}: {sizes:?}",
                c.player
            );
        }
        println!("{}", format_player_comparison(&comparisons));
    }

    #[test]
    fn missing_song_file_is_rejected_before_touching_disc() {
        let observer = Arc::new(TestObserver);
        let outcome = run_music_in_emulator(
            Utf8Path::new("/no/such/song.aks"),
            "PROG",
            "ace",
            &MusicOptions::default(),
            &observer
        );
        assert!(!outcome.success);
        assert!(outcome.message.contains("does not exist"));
    }

    #[test]
    fn missing_song_file_is_rejected_before_touching_disc_dsk_path() {
        let observer = Arc::new(TestObserver);
        let err = build_music_dsk(
            Utf8Path::new("/no/such/song.aks"),
            "PROG",
            &MusicOptions::default(),
            &observer
        )
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
        let dsk_path = build_music_dsk(song, "TARGHAN", &MusicOptions::default(), &observer)
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
        assert!(
            super::super::song_uses_sid(&song).unwrap(),
            "fixture should be SID-tagged"
        );

        let dsk_path = build_music_dsk(&song, "SIDTEST", &MusicOptions::default(), &observer)
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
            "TEST",
            72,
            true,
            &["-oi".to_string(), sna_path.to_string()],
            &observer
        )
        .expect("the real SID conversion+assemble+snapshot pipeline should succeed");
        assert!(sna_path.is_file(), "the snapshot should have been written");
        let size = std::fs::metadata(&sna_path).unwrap().len();
        assert!(
            size > 20_000,
            "a real 64K .sna should be well over 20KB, got {size}"
        );
    }
}

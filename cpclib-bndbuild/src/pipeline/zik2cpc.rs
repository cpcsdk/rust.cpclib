//! `zik2cpc`: every music conversion the bndbuild pipeline knows, as a
//! command line tool (it used to be reachable only from the vscode extension).
//!
//! The same [`Zik2CpcArgs`] drive the standalone `zik2cpc` binary and the
//! embedded `zik2cpc` bndbuild command.

use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Parser, Subcommand, ValueEnum};
use cpclib_common::event::{CapturingObserver, EventObserver};

use super::music_run::{
    self, MusicBuild, MusicOptions, MusicPlayer, compare_music_players, compatible_players,
    format_player_comparison
};
use super::{
    YmVersion, convert_chp_to_z80, convert_song_to_aky_source, convert_song_to_arkos_binary,
    convert_to_ym, pack_ym, sanitize_amsdos_stem, song_metadata, song_uses_sid
};
use crate::runners::ay::YmCruncher;
use crate::runners::tracker::SongConverter;
use crate::task::{InnerTask, StandardTaskArguments, Task};

/// Convert, pack and play CPC music (Arkos Tracker, STarKos, Soundtrakker, Vortex Tracker, Wyz Tracker, CHIPNSFX, YM)
#[derive(Parser, Debug)]
#[command(name = "zik2cpc")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Krusty/Benediction")]
#[command(about = "Convert, pack and play CPC music")]
// the subcommand is optional for clap so that the embedded runner's own `--help` flag is reachable
#[command(arg_required_else_help = true)]
pub struct Zik2CpcArgs {
    #[command(subcommand)]
    pub command: Option<Zik2CpcCommand>
}

#[derive(Subcommand, Debug)]
pub enum Zik2CpcCommand {
    /// Convert a song to a music format (the tools are downloaded on demand)
    Convert(ConvertArgs),
    /// Build a standalone player program (binary, snapshot or DSK) for a song
    Build(BuildArgs),
    /// Build a player for a song and run it in an emulator
    Play(PlayArgs),
    /// Build the song with every player that can play it, and compare their sizes
    Compare(SongArg),
    /// List the players that can play a song
    Players(SongArg),
    /// Show the title, author, composer and comment of a song
    Info(SongArg)
}

#[derive(Args, Debug)]
pub struct SongArg {
    /// The song (.aks, .sks, .128, .vt2, .wyz, .chp, .ym)
    pub input: Utf8PathBuf
}

/// What a song can be converted to
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ConvertFormat {
    /// Arkos Tracker 3 AKG player binary
    Akg,
    /// Arkos Tracker 3 AKM player binary
    Akm,
    /// Arkos Tracker 3 AKY player binary
    Aky,
    /// Arkos Tracker 3 AKY player assembler source
    AkySource,
    /// YM (version 6)
    Ym,
    /// YM (version 3)
    Ym3,
    /// VGM
    Vgm,
    /// WAV
    Wav,
    /// Raw registers dump
    Raw,
    /// Events
    Events,
    /// Sound effects
    SoundEffects,
    /// CHIPNSFX Z80 source (input must be a .chp)
    ChpZ80,
    /// YM packed with FAP
    #[cfg(feature = "fap")]
    Fap,
    /// YM packed with AYT
    Ayt,
    /// YM packed with MinYMiser
    Miny
}

#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// The song to convert
    pub input: Utf8PathBuf,
    /// The file to write
    pub output: Utf8PathBuf,
    /// The format of the file to write
    #[arg(short, long, value_enum)]
    pub to: ConvertFormat
}

/// What a player program is saved as
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum BuildFormat {
    /// DSK containing an AMSDOS binary
    #[default]
    Dsk,
    /// Snapshot of the running player
    Sna,
    /// Headerless binary (loads and starts at the address that is printed)
    Bin
}

#[derive(Args, Debug)]
pub struct PlayerArgs {
    /// The player ("auto" lets the song decide)
    #[arg(short, long, default_value = "auto", value_parser = parse_player)]
    pub player: MusicPlayer,
    /// Safety margin, in lines, of the SID player (only used for songs using the SID feature)
    #[arg(long, default_value_t = MusicOptions::default().sid_wait_line_count)]
    pub sid_wait_line_count: u16
}

impl PlayerArgs {
    fn options(&self) -> MusicOptions {
        MusicOptions {
            sid_wait_line_count: self.sid_wait_line_count,
            player: self.player
        }
    }
}

fn parse_player(s: &str) -> Result<MusicPlayer, String> {
    s.parse()
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// The song to build a player for
    pub input: Utf8PathBuf,
    /// The file to write
    pub output: Utf8PathBuf,
    /// What to write
    #[arg(short, long, value_enum, default_value_t = BuildFormat::Dsk)]
    pub format: BuildFormat,
    #[command(flatten)]
    pub player: PlayerArgs
}

#[derive(Args, Debug)]
pub struct PlayArgs {
    /// The song to play
    pub input: Utf8PathBuf,
    /// The emulator to run
    #[arg(short, long, default_value = "ace")]
    pub emulator: String,
    #[command(flatten)]
    pub player: PlayerArgs
}

/// Runs `args`; the pipeline functions need an `Arc<impl BndBuilderObserver>`
/// that lives for `'static`, so what they say is captured, then forwarded to `o`.
pub fn run<E: EventObserver>(args: &Zik2CpcArgs, o: &E) -> Result<(), String> {
    let captured = Arc::new(CapturingObserver::new());
    let result = match &args.command {
        Some(command) => run_command(command, &captured, o),
        None => Err("a command is required (see --help)".to_owned())
    };
    for (out, text) in [(false, captured.stdout_joined()), (true, captured.stderr_joined())] {
        if !text.is_empty() {
            if out {
                o.emit_stderr(&text);
            }
            else {
                o.emit_stdout(&text);
            }
        }
    }
    result
}

fn run_command<E: EventObserver>(
    command: &Zik2CpcCommand,
    observer: &Arc<CapturingObserver>,
    o: &E
) -> Result<(), String> {
    match command {
        Zik2CpcCommand::Convert(args) => convert(args, observer),
        Zik2CpcCommand::Build(args) => build(args, observer, o),
        Zik2CpcCommand::Play(args) => {
            let outcome = music_run::run_music_in_emulator(
                &args.input,
                &name_hint(&args.input),
                &args.emulator,
                &args.player.options(),
                observer
            );
            o.emit_stdout(&format!("{}\n", outcome.message));
            outcome.success.then_some(()).ok_or(outcome.message)
        },
        Zik2CpcCommand::Compare(args) => {
            let comparisons = compare_music_players(&args.input, &name_hint(&args.input), observer)?;
            o.emit_stdout(&format_player_comparison(&comparisons));
            Ok(())
        },
        Zik2CpcCommand::Players(args) => {
            let players = compatible_players(&args.input);
            o.emit_stdout(&format!(
                "{}\n",
                players
                    .iter()
                    .map(|p| p.name())
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
            Ok(())
        },
        Zik2CpcCommand::Info(args) => {
            let meta = song_metadata(&args.input);
            let mut text = String::new();
            for (label, value) in [
                ("title", &meta.title),
                ("author", &meta.author),
                ("composer", &meta.composer),
                ("comment", &meta.comment)
            ] {
                text.push_str(&format!("{label}: {value}\n"));
            }
            if let Ok(true) = song_uses_sid(&args.input) {
                text.push_str("sid: yes\n");
            }
            o.emit_stdout(&text);
            Ok(())
        }
    }
}

/// The stem of `song`, as the name of the AMSDOS file of a player.
fn name_hint(song: &Utf8Path) -> String {
    sanitize_amsdos_stem(song.file_stem().unwrap_or("MUSIC"))
}

/// Runs the song converter `converter` as `input output`.
fn run_song_converter(
    converter: SongConverter,
    input: &Utf8Path,
    output: &Utf8Path,
    observer: &Arc<CapturingObserver>
) -> Result<(), String> {
    let args = shlex::try_join([input.as_str(), output.as_str()])
        .map_err(|e| format!("Could not build the converter's arguments: {e}"))?;
    let task: Task =
        InnerTask::with_songconverter(converter, StandardTaskArguments::new(args)).into();
    task.execute(observer)
}

fn convert(args: &ConvertArgs, observer: &Arc<CapturingObserver>) -> Result<(), String> {
    let (input, output) = (args.input.as_path(), args.output.as_path());
    if !input.is_file() {
        return Err(format!("{input} does not exist"));
    }

    match args.to {
        ConvertFormat::Akg => {
            convert_song_to_arkos_binary(
                SongConverter::new_song_to_akg_default(),
                input,
                output,
                observer
            )
        },
        ConvertFormat::Akm => {
            convert_song_to_arkos_binary(
                SongConverter::new_song_to_akm_default(),
                input,
                output,
                observer
            )
        },
        ConvertFormat::Aky => {
            convert_song_to_arkos_binary(
                SongConverter::new_song_to_aky_default(),
                input,
                output,
                observer
            )
        },
        ConvertFormat::AkySource => convert_song_to_aky_source(input, output, observer),
        ConvertFormat::Ym => convert_to_ym(input, output, YmVersion::Ym6, observer),
        ConvertFormat::Ym3 => convert_to_ym(input, output, YmVersion::Ym3, observer),
        ConvertFormat::Vgm => {
            run_song_converter(SongConverter::new_song_to_vgm_default(), input, output, observer)
        },
        ConvertFormat::Wav => {
            run_song_converter(SongConverter::new_song_to_wav_default(), input, output, observer)
        },
        ConvertFormat::Raw => {
            run_song_converter(SongConverter::new_song_to_raw_default(), input, output, observer)
        },
        ConvertFormat::Events => {
            run_song_converter(
                SongConverter::new_song_to_events_default(),
                input,
                output,
                observer
            )
        },
        ConvertFormat::SoundEffects => {
            run_song_converter(
                SongConverter::new_song_to_sound_effects_default(),
                input,
                output,
                observer
            )
        },
        ConvertFormat::ChpZ80 => {
            if !super::song_is_chp(input) {
                return Err(format!("{input} is not a CHIPNSFX (.chp) song"));
            }
            convert_chp_to_z80(input, output, observer)
        },
        #[cfg(feature = "fap")]
        ConvertFormat::Fap => pack(YmCruncher::Fap, input, output, observer),
        ConvertFormat::Ayt => pack(YmCruncher::Ayt, input, output, observer),
        ConvertFormat::Miny => pack(YmCruncher::Miny, input, output, observer)
    }
}

/// Packs `input` (converted to the YM version `cruncher` needs, if it is another kind of song).
fn pack(
    cruncher: YmCruncher,
    input: &Utf8Path,
    output: &Utf8Path,
    observer: &Arc<CapturingObserver>
) -> Result<(), String> {
    let dir = camino_tempfile::tempdir()
        .map_err(|e| format!("Could not create a temp working directory: {e}"))?;
    let converted = dir.path().join("song.ym");
    // FAP and MinYMiser read YM6, AYT YM3
    let version = match cruncher {
        YmCruncher::Ayt => YmVersion::Ym3,
        _ => YmVersion::Ym6
    };
    let ym = super::ym_for_player(input, &converted, version, observer)?;
    pack_ym(cruncher, &ym, output, observer).map(|_| ())
}

fn build<E: EventObserver>(
    args: &BuildArgs,
    observer: &Arc<CapturingObserver>,
    o: &E
) -> Result<(), String> {
    let options = args.player.options();
    let hint = name_hint(&args.input);
    match args.format {
        BuildFormat::Dsk => {
            let dsk = music_run::build_music_dsk(&args.input, &hint, &options, observer)?;
            fs_err::copy(&dsk, &args.output)
                .map_err(|e| format!("Could not write {}: {e}", args.output))?;
        },
        BuildFormat::Sna => {
            music_run::build_music(&args.input, &hint, &options, Some(&args.output), observer)?;
        },
        BuildFormat::Bin => {
            let built: MusicBuild =
                music_run::build_music(&args.input, &hint, &options, None, observer)?;
            fs_err::write(&args.output, &built.program)
                .map_err(|e| format!("Could not write {}: {e}", args.output))?;
            o.emit_stdout(&format!(
                "{} bytes, to load and start at #{:04X}\n",
                built.program.len(),
                built.load_address
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn clap_definition_is_consistent() {
        Zik2CpcArgs::command().debug_assert();
    }

    #[test]
    fn parses_convert() {
        let args = Zik2CpcArgs::try_parse_from(["zik2cpc", "convert", "a.aks", "a.akg", "--to", "akg"])
            .unwrap();
        match args.command.unwrap() {
            Zik2CpcCommand::Convert(c) => {
                assert_eq!(c.to, ConvertFormat::Akg);
                assert_eq!(c.input, "a.aks");
            },
            other => panic!("unexpected {other:?}")
        }
    }

    #[test]
    fn parses_build_with_a_player() {
        let args = Zik2CpcArgs::try_parse_from([
            "zik2cpc", "build", "a.aks", "a.sna", "--format", "sna", "--player", "akm"
        ])
        .unwrap();
        match args.command.unwrap() {
            Zik2CpcCommand::Build(b) => {
                assert_eq!(b.format, BuildFormat::Sna);
                assert_eq!(b.player.player, MusicPlayer::Akm);
            },
            other => panic!("unexpected {other:?}")
        }
    }

    #[test]
    fn unknown_player_is_rejected() {
        assert!(
            Zik2CpcArgs::try_parse_from(["zik2cpc", "play", "a.aks", "--player", "nope"]).is_err()
        );
    }

    #[test]
    fn missing_song_is_reported() {
        let observer = Arc::new(CapturingObserver::new());
        let args = ConvertArgs {
            input: "/nonexistent/song.aks".into(),
            output: "/tmp/out.akg".into(),
            to: ConvertFormat::Akg
        };
        assert!(convert(&args, &observer).unwrap_err().contains("does not exist"));
    }
}

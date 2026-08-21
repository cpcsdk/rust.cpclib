use std::io::Write;

use anyhow::{self, Error};
use camino_tempfile as tempfile;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use cpclib::asm::preamble::defb_elements;
use cpclib::asm::{ListingExt, assemble, assemble_to_amsdos_file};
use cpclib::common::camino::{Utf8Path, Utf8PathBuf};
use cpclib::common::event::EventObserver;
use cpclib::common::itertools::Itertools;
use cpclib::common::winnow::{BStr, Parser};
use cpclib::common::{clap, clap_parse_any_positive_number};
use cpclib::disc::amsdos::*;
use cpclib::disc::disc::Disc;
use cpclib::disc::edsk::Head;
use cpclib::image::convert::*;
use cpclib::image::image::Mode;

// Most of this tool is Gate Array work: 27 inks, written through the GA ports.
// The Plus path is a *choice made at the command line* and travels as an
// `AnyLockablePalette`, so it is spelled out where it matters rather than
// making every signature here generic.
type Palette = cpclib::image::ga::Palette<Ink>;
type LockablePalette = cpclib::image::ga::LockablePalette<Ink>;
type ColorMatrix = cpclib::image::image::ColorMatrix<Ink>;
type AsicPalette = cpclib::image::ga::Palette<AsicColor>;
type AsicLockablePalette = cpclib::image::ga::LockablePalette<AsicColor>;
use cpclib::image::asic::AsicColor;
use cpclib::image::color::AmstradColor;
use cpclib::image::ga::{AnyLockablePalette, AnyPalette};
use cpclib::image::kit::Kit;
use cpclib::image::ocp::{self, OcpPalette};
use cpclib::sna::*;
#[cfg(feature = "xferlib")]
use cpclib::xfer::CpcXfer;
use cpclib::{ExtendedDsk, Ink, Pen, sna};
use fs_err::File;
#[cfg(feature = "watch")]
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use owo_colors::{DynColors, OwoColorize};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn clap_parse_ink(arg: &str) -> Result<Ink, String> {
    let nb = clap_parse_any_positive_number(arg)?;
    if nb > 27 {
        Err(format!("{nb} is not a valid ink value"))
    }
    else {
        Ok(nb.into())
    }
}

/// The pens a palette argument can name: 0-15, plus 16 for the border.
pub fn pen_indices() -> std::ops::RangeInclusive<u8> {
    0..=16
}

/// clap wants `&'static str` for names and flags, so the generated ones are
/// built once into a static table rather than leaked afresh on every call -
/// `specify_palette!` runs for each subcommand that takes a palette.
static PALETTE_ARG_NAMES: std::sync::LazyLock<Vec<[String; 4]>> =
    std::sync::LazyLock::new(|| {
        pen_indices()
            .map(|i| {
                [
                    format!("PEN{i}"),
                    format!("pen{i}"),
                    format!("COLB{i}"),
                    format!("colb{i}")
                ]
            })
            .collect()
    });

fn arg_names(index: u8) -> &'static [String; 4] {
    &PALETTE_ARG_NAMES[index as usize]
}

fn pen_description(index: u8) -> String {
    if index == 16 {
        "Ink number of the pen 16 (border)".to_owned()
    }
    else {
        format!("Ink number of the pen {index}")
    }
}

/// `--penN <ink>` - one Gate Array ink for pen `N`.
///
/// Generated rather than written out: there are 17 of these, and with the
/// Amstrad Plus's `--colbN` beside them every pair also needs a conflict
/// declaration. Spelled by hand that is ~600 lines in which every line looks
/// like its neighbour and none of them is making a decision.
pub fn pen_argument(index: u8) -> Arg {
    let names = arg_names(index);
    let mut arg = Arg::new(names[0].as_str())
        .long(names[1].as_str())
        .required(false)
        .help(pen_description(index))
        .conflicts_with("PENS")
        .conflicts_with("OCP_PAL")
        .conflicts_with("GA_PAL")
        .conflicts_with("KIT_PAL")
        .conflicts_with("PLUS")
        .value_parser(value_parser!(u8));
    // A palette is homogeneous: all Gate Array inks, or all ASIC colours.
    // There is no hardware that shows a mixture, so asking for one is a
    // mistake worth naming rather than resolving arbitrarily.
    for other in pen_indices() {
        arg = arg.conflicts_with(arg_names(other)[2].as_str());
    }
    arg
}

/// `--colbN <colour>` - one ASIC colour for pen `N`, for the Amstrad Plus.
///
/// Accepts either spelling `AsicColor`'s own parser does: packed hex (`4A5`,
/// `0x4A5`) or `R,G,B` components (`4,10,5`).
pub fn colb_argument(index: u8) -> Arg {
    let names = arg_names(index);
    let mut arg = Arg::new(names[2].as_str())
        .long(names[3].as_str())
        .required(false)
        .help(format!(
            "Asic color for pen {index} (Amstrad Plus). Either packed hex \
             (4A5, 0x4A5) or R,G,B components 0-15 (4,10,5)"
        ))
        .conflicts_with("PENS")
        .conflicts_with("OCP_PAL")
        .conflicts_with("GA_PAL")
        .value_parser(|raw: &str| {
            <cpclib::image::asic::AsicColor as std::str::FromStr>::from_str(raw)
        });
    for other in pen_indices() {
        arg = arg.conflicts_with(arg_names(other)[0].as_str());
    }
    arg
}

#[macro_export]
macro_rules! specify_palette {

    ($e:expr) => {
        specify_palette!($e, true)
    };

    ($e:expr, $unlock:expr) => {{
        let mut cmd = $e.arg(
            Arg::new("OCP_PAL")
            .long("pal")
            .alias("ocp-pal"    )
            .required(false)
            .help("OCP PAL file. The first palette among 12 is used") // TODO specify a way to select any palette
            .value_parser(|p: &str| cpclib::common::utf8pathbuf_value_parser(true)(p))
        )
        .arg(
            Arg::new("GA_PAL")
                .long("ga-pal")
                .required(false)
                .help("GA PAL file. A binary file that contains the palette in Gate Array format")
                .value_parser(|p: &str| cpclib::common::utf8pathbuf_value_parser(true)(p)
            )
            .conflicts_with("OCP_PAL")

        )
        .arg(
            Arg::new("KIT_PAL")
                .long("kit")
                .required(false)
                .help("Kit file for Amstrad Plus. A binary file that contains the 32 bytes")
                .value_parser(|p: &str| cpclib::common::utf8pathbuf_value_parser(true)(p))
                .conflicts_with("OCP_PAL")
                .conflicts_with("GA_PAL")
                .conflicts_with("PENS")
                .conflicts_with("UNLOCK_PENS")
        )
        .arg(
            Arg::new("PLUS")
                .long("plus")
                .required(false)
                .help("Target the Amstrad Plus: build the palette out of 12-bit ASIC colours taken from the image, instead of the 27 Gate Array inks")
                .action(ArgAction::SetTrue)
                .conflicts_with("OCP_PAL")
                .conflicts_with("GA_PAL")
                .conflicts_with("PENS")
                .conflicts_with("KIT_PAL")
        )
        .arg(
            Arg::new("PENS")
                .long("pens")
                .required(false)
                .help("Separated list of ink number. Use ',' as a separater")
                .conflicts_with("OCP_PAL")
                .conflicts_with("GA_PAL")
        );

        for index in $crate::pen_indices() {
            cmd = cmd
                .arg($crate::pen_argument(index))
                .arg($crate::colb_argument(index));
        }

        if $unlock {

            cmd.arg(
                Arg::new("UNLOCK_PENS")
                    .long("unlock-pens")
                    .required(false)
                    .conflicts_with("OCP_PAL")
                    .conflicts_with("PENS")
                    .conflicts_with("GA_PAL")
                    .help("When some pens are manually provided, allows to also use the other ones by automatically assign them missing inks. By default, this is forbidden.")
                    .action(ArgAction::SetTrue)
            )
        }else {
            cmd
        }
    }};
}

/// Whether a flag is both present in this subcommand and set.
///
/// `--unlock-pens` and `--plus` are only declared for the subcommands that can
/// use them, so asking for one that was never declared has to be a plain
/// "no" rather than a panic.
fn pens_are_unlocked(matches: &ArgMatches, id: &str) -> bool {
    matches.try_get_one::<bool>(id).ok().flatten() == Some(&true)
}

/// The palette the user asked for, on whichever machine they asked for.
///
/// `--kit`/`--colbN` mean the Amstrad Plus and give the `Asic` variant; every
/// other spelling means the Gate Array and behaves exactly as it always has.
pub fn get_requested_palette(matches: &ArgMatches) -> Result<AnyLockablePalette, AmsdosError> {
    if let Some(fname) = matches.get_one::<Utf8PathBuf>("KIT_PAL") {
        let kit = Kit::from_file(fname)
            .map_err(|e| AmsdosError::IO(format!("Unable to read the kit palette file: {e}")))?;
        return Ok(AsicLockablePalette::unlocked(AsicPalette::from(kit)).into());
    }

    // An ASIC palette, either named colour by colour or left to the converter.
    // `--colbN` conflicts with every `--penN`, so seeing one here means the
    // whole palette is ASIC.
    let named_colors = pen_indices().any(|i| matches.contains_id(arg_names(i)[2].as_str()));
    if named_colors || pens_are_unlocked(matches, "PLUS") {
        let mut palette = AsicPalette::empty();
        for i in pen_indices() {
            let key = arg_names(i)[2].as_str();
            if let Some(color) = matches.get_one::<AsicColor>(key) {
                palette.set(i as i32, *color);
            }
        }
        // Same rule as the Gate Array path: naming some colours pins the
        // palette unless the user explicitly allows the rest to be chosen.
        return Ok(
            if named_colors && !pens_are_unlocked(matches, "UNLOCK_PENS") {
                AsicLockablePalette::locked(palette)
            }
            else {
                AsicLockablePalette::unlocked(palette)
            }
            .into()
        );
    }

    if matches.contains_id("PENS") {
        let numbers = matches
            .get_one::<String>("PENS")
            .unwrap()
            .split(",")
            .map(|ink| {
                cpclib::common::parse_value::<_, ()>
                    .parse(BStr::new(ink))
                    .unwrap_or_else(|_| Ink::from(ink.replace("GA_", "")).gate_array_value() as _)
            })
            .map(|n: u32| Ink::from(n))
            .collect::<Vec<_>>();
        Ok(LockablePalette::unlocked(numbers.into()).into())
    }
    else if let Some(fname) = matches.get_one::<Utf8PathBuf>("OCP_PAL") {
        let (mut data, _header) = cpclib::disc::read(fname)?; // get the file content but skip the header
        let data = data.make_contiguous();
        let pal = OcpPalette::from_buffer(data);
        Ok(LockablePalette::unlocked(pal.palette(0).clone()).into())
    }
    else if let Some(fname) = matches.get_one::<Utf8PathBuf>("GA_PAL") {
        use std::io::Read;
        let mut file = File::open(fname).expect("Unable to open the gate array palette file");
        let mut buffer: Vec<u8> = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Unable to read the gate array palette file");
        let pal = Palette::from_iter(buffer);
        Ok(LockablePalette::unlocked(pal).into())
    }
    else {
        let mut one_pen_set = false;
        let mut palette = Palette::empty();
        for i in 0..16 {
            let key = format!("PEN{i}");
            if matches.contains_id(&key) {
                one_pen_set = true;
                palette.set(i, *matches.get_one::<u8>(&key).unwrap())
            }
        }
        if matches.get_flag("UNLOCK_PENS") || !one_pen_set {
            Ok(LockablePalette::unlocked(palette).into())
        }
        else {
            Ok(LockablePalette::locked(palette).into())
        }
    }
}

macro_rules! export_palette {
    ($e: expr) => {
        $e.arg(
            Arg::new("EXPORT_PALETTE") // TODO really save a OCP palette and create an additional Gate array export in order to be constant with loading
                .long("palette")
                .short('p')
                .required(false)
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(Utf8PathBuf))
                .help("Name of the binary file that contains the palette (Gate Array format)"),
        )
        .arg(
            Arg::new("EXPORT_KIT")
                .long("kit")
                .required(false)
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(Utf8PathBuf))
                .help("Name of the binary file that contains the palette in Amstrad Plus kit format (32 bytes)")
        )
        .arg(
            Arg::new("EXPORT_INKS")
            .long("inks")
            .short('i')
            .required(false)
            .action(ArgAction::Set)
            .value_parser(clap::value_parser!(Utf8PathBuf))
            .help("Name of the binary file that will contain the ink numbers (usefull for system based color change)")
        )
        .arg(
            Arg::new("EXPORT_PALETTE_FADEOUT")
                .long("palette_fadeout")
                .required(false)
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(Utf8PathBuf))
                .help("Name of the file that will contain all the steps for a fade out transition (Gate Array format)")
        )
        .arg(
            Arg::new("EXPORT_INK_FADEOUT")
                .long("ink_fadeout")
                .required(false)
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(Utf8PathBuf))
                .help("Name of the file that will contain all the steps for a fade out transition")
        )
    };
}

/// Write out whatever palette exports the user asked for.
///
/// Each flag names a format, not a machine: `--palette` writes the 17 Gate
/// Array bytes and `--kit` the Plus's 32. A Gate Array palette can be written
/// either way - the 27 inks are a subset of the ASIC's colours - but an
/// Amstrad Plus palette has no Gate Array spelling, so `--palette` says so and
/// points at `--kit`.
///
/// The remaining exports (`--inks`, the two fade-outs) are Gate Array notions -
/// an ink *number*, and a walk down the 27-ink ladder - and say the same.
macro_rules! do_export_palette {
    ($arg:expr, $palette:ident) => {
        let any = $palette.clone().into_any();

        if let Some(palette_fname) = $arg.get_one::<Utf8PathBuf>("EXPORT_PALETTE") {
            let palette = any.gate_array().ok_or_else(|| {
                anyhow::anyhow!(
                    "--palette exports the Gate Array format, which cannot hold the 12-bit \
                     colours of an Amstrad Plus palette. Use --kit to export its 32 bytes."
                )
            })?;
            let bytes: Vec<u8> = palette.into();
            let mut file = File::create(palette_fname).expect("Unable to create the palette file");
            file.write_all(&bytes).unwrap();
        }

        if let Some(kit_fname) = $arg.get_one::<Utf8PathBuf>("EXPORT_KIT") {
            // A Gate Array palette is exportable as a kit: every ink has an
            // exact ASIC colour. The reverse is what is impossible.
            let bytes = match &any {
                AnyPalette::Asic(_) => any.asic_bytes().unwrap(),
                AnyPalette::GateArray(p) => {
                    cpclib::image::ga::AnyPalette::Asic(
                        cpclib::image::ga::Palette::<AsicColor>::from(p.clone())
                    )
                    .asic_bytes()
                    .unwrap()
                }
            };
            let mut file = File::create(kit_fname).expect("Unable to create the kit file");
            file.write_all(&bytes).unwrap();
        }

        let gate_array_only = ["EXPORT_PALETTE_FADEOUT", "EXPORT_INKS", "EXPORT_INK_FADEOUT"]
            .iter()
            .find(|id| $arg.get_one::<Utf8PathBuf>(*id).is_some());
        if let Some(id) = gate_array_only {
            let palette = any.gate_array().ok_or_else(|| {
                anyhow::anyhow!(
                    "{} exports ink numbers, which only exist on the Gate Array - this is an \
                     Amstrad Plus palette. Use --kit to export its 32 bytes instead.",
                    match *id {
                        "EXPORT_PALETTE_FADEOUT" => "--palette_fadeout",
                        "EXPORT_INKS" => "--inks",
                        _ => "--ink_fadeout"
                    }
                )
            })?;

            if let Some(fade_fname) = $arg.get_one::<Utf8PathBuf>("EXPORT_PALETTE_FADEOUT") {
                let palettes = palette.rgb_fadout();
                let bytes = palettes.iter().fold(Vec::<u8>::default(), |mut acc, x| {
                    acc.extend(&x.to_gate_array_with_default(0.into()));
                    acc
                });

                assert_eq!(palettes.len() * 17, bytes.len());

                let mut file = File::create(fade_fname).expect("Unable to create the fade out file");
                file.write_all(&bytes).unwrap();
            }

            if let Some(palette_fname) = $arg.get_one::<Utf8PathBuf>("EXPORT_INKS") {
                let mut file = File::create(palette_fname).expect("Unable to create the inks file");
                let inks = palette
                    .inks()
                    .iter()
                    .map(|i| i.number())
                    .collect::<Vec<_>>();
                file.write_all(&inks).unwrap();
            }

            if let Some(fade_fname) = $arg.get_one::<Utf8PathBuf>("EXPORT_INK_FADEOUT") {
                let palettes = palette.rgb_fadout();
                let bytes = palettes
                    .iter()
                    .map(|p| p.inks().iter().map(|i| i.number()).collect::<Vec<_>>())
                    .fold(Vec::default(), |mut acc, x| {
                        acc.extend(&x);
                        acc
                    });
                let mut file = File::create(fade_fname).expect("Unable to create the fade out file");
                file.write_all(&bytes).unwrap();
            }
        }
    };
}

/// Compress data using lz4 algorithm.
/// Should be decompressed on client side.
/// TODO test: implementation has been modified without any testing...
fn lz4_compress(bytes: &[u8]) -> Vec<u8> {
    cpclib::crunchers::lz4::compress(bytes)
}

/// The palette installation as a single self-contained block.
///
/// `palette_installation_code` keeps its data separate so a display routine can
/// place it past its own final `jp`. A loader has no such tail, so the bytes go
/// inline and the code jumps over them.
fn palette_installation_inline(pal: &AnyPalette) -> String {
    let (code, data) = palette_installation_code(pal);
    if data.is_empty() {
        code
    }
    else {
        format!("{code}\n\tjr palette_installed\n{data}\npalette_installed\n")
    }
}

fn standard_linked_code(mode: u8, pal: &AnyPalette, screen: &[u8]) -> String {
    let base_code = standard_display_code(mode, pal);
    format!(
        "   org 0x1000
        di
        ld sp, $

        ; Copy image on screen
        ld hl, image
        ld de, 0xc000
        ld bc, image_end - image
        call lz4_uncrunch

        ; Copy visualization code
        ld hl, code
        ld de, 0x4000
        ld bc, code_end - code
        ldir

        ei
        jp 0x4000
lz4_uncrunch
    {decompressor}
code
    {code}
code_end
        assert $ < 0x4000
image
    {screen}
image_end

        assert $<0xc000
    ",
        decompressor = include_str!("lz4_docent.asm"),
        code = defb_elements(&assemble(&base_code).unwrap()),
        screen = defb_elements(screen)
    )
}

// Produce the code that display a standard screen
//
// It installs the palette itself rather than leaving it to the caller: a
// snapshot's `GA_PAL` registers can carry 27 Gate Array inks and nothing else,
// so an Amstrad Plus palette has no other way in.
fn standard_display_code(mode: u8, palette: &AnyPalette) -> String {
    let code_mode = match mode {
        0 => 0x8C,
        1 => 0x8D,
        2 => 0x8E,
        _ => unreachable!()
    };
    let palette_code = palette_installation_inline(palette);

    format!(
        "
        org 0x4000
        di
    

        {palette_code}

        ld bc, 0x7f00 + 0x{code_mode:x}
        out (c), c

        jp $
    "
    )
}

/// The palette installation, and the data it needs after the routine.
///
/// The Gate Array writes 16 inks through port `0x7f00`, one `out` per pen. The
/// Plus cannot: its colours are 12-bit, so the ASIC is unlocked, the 32 bytes
/// are `ldir`ed into the colour registers at `0x6400`, and the ASIC is locked
/// again. The bytes are data - they are emitted *after* the routine's final
/// `jp`, never in its path.
fn palette_installation_code(palette: &AnyPalette) -> (String, String) {
    match palette {
        AnyPalette::GateArray(_) => {
            let inks = palette
                .gate_array_bytes()
                .expect("a Gate Array palette must produce its ink values");
            let mut code = String::from("\tld bc, 0x7f00\n");
            for ink in inks.iter().take(16) {
                code += &format!(
                    "\tld a, {ink}\n\t out (c), c\n\tout (c), a\n\t inc c\n"
                );
            }
            (code, String::new())
        },
        AnyPalette::Asic(_) => {
            let bytes = palette
                .asic_bytes()
                .expect("an ASIC palette must produce its 32 bytes");
            (
                "
            LD         HL,tasic
            LD         D,17
delock      LD         BC,#BC00
            LD         A,(HL)
            OUT        (C),A
            INC        HL
            DEC        D
            JP         NZ,delock 
            jp continue 

tasic       DB         255,0,255,119,179,81,168,212,98,57,156,70,43,21,138,205,238
        
        continue

        ld bc, 0x7fb8 ; connect the ASIC
        out (c), c
        ld hl, palette_tab
        ld de, 0x6400
        ld bc, 32
        ldir
        ld bc, 0x7fa0 ; lock it again
        out (c), c
"
                .to_owned(),
                format!("\npalette_tab\n\t{}\n", defb_elements(&bytes))
            )
        }
    }
}

fn fullscreen_display_code(mode: u8, crtc_width: usize, palette: &AnyPalette) -> String {
    let code_mode = match mode {
        0 => 0x8C,
        1 => 0x8D,
        2 => 0x8E,
        _ => unreachable!()
    };

    let r12 = 0x20 + 0b0000_1100;

    let (palette_code, palette_data) = palette_installation_code(palette);

    format!(
        "
        org 0x4000

        di
        ld hl, 0xc9fb
        ld (0x38), hl
        ld sp, $
        ei

        ld bc, 0x7f00 + 0x{code_mode:x}
        out (c), c

        ld bc, 0xbc00 + 1
        out (c), c
        ld bc, 0xbd00 + {crtc_width}
        out (c), c

        ld bc, 0xbc00 + 2
        out (c), c
        ld bc, 0xbd00 + 50
        out (c), c

        ld bc, 0xbc00 + 12
        out (c), c
        ld bc, 0xbd00 + {r12}
        out (c), c

        ld bc, 0xbc00 + 13
        out (c), c
        ld bc, 0xbd00 + 0x00
        out (c), c

        ld bc, 0xbc00 + 7
        out (c), c
        ld bc, 0xbd00 + 35
        out (c), c

        ld bc, 0xbc00 + 6
        out (c), c
        ld bc, 0xbd00 + 38
        out (c), c

        {palette_code}

frame_loop
        ld b, 0xf5
vsync_loop
        in a, (c)
        rra
        jr nc, vsync_loop




        jp frame_loop
{palette_data}
    "
    )
}

fn overscan_display_code(mode: u8, crtc_width: usize, pal: &AnyPalette) -> String {
    fullscreen_display_code(mode, crtc_width, pal)
}

fn parse_int(repr: &str) -> usize {
    repr.parse::<usize>()
        .unwrap_or_else(|_| panic!("Error when converting {repr} as integer"))
}

#[allow(clippy::if_same_then_else)] // false positive
fn get_output_format<C: AmstradColor>(matches: &ArgMatches) -> OutputFormat<C> {
    if let Some(sprite_matches) = matches.subcommand_matches("sprite") {
        // Get the format for the sprite encoding
        let sprite_format = match sprite_matches.get_one::<String>("FORMAT").unwrap().as_ref() {
            "linear" => SpriteEncoding::Linear,
            "graycoded" => SpriteEncoding::GrayCoded,
            "zigazag" => SpriteEncoding::LeftToRightToLeft,
            "zigzag+graycoded" => SpriteEncoding::ZigZagGrayCoded,
            _ => unimplemented!()
        };

        // eventually handle sprite masking
        if sprite_matches.contains_id("MASK_FNAME") {
            OutputFormat::MaskedSprite {
                sprite_format,
                mask_ink: C::from(
                    sprite_matches.get_one::<Ink>("MASK_INK").cloned().unwrap()
                ),
                replacement_ink: C::from(
                    sprite_matches
                        .get_one::<Ink>("REPLACEMENT_INK")
                        .cloned()
                        .unwrap()
                )
            }
        }
        else {
            OutputFormat::Sprite(sprite_format)
        }
    }
    else if let Some(tile_matches) = matches.subcommand_matches("tile") {
        OutputFormat::TileEncoded {
            tile_width: TileWidthCapture::NbBytes(parse_int(
                tile_matches
                    .get_one::<String>("WIDTH")
                    .expect("--width argument missing")
            )),

            tile_height: TileHeightCapture::NbLines(parse_int(
                tile_matches
                    .get_one::<String>("HEIGHT")
                    .expect("--height argument missing")
            )),

            horizontal_movement: TileHorizontalCapture::AlwaysFromLeftToRight,
            vertical_movement: TileVerticalCapture::AlwaysFromTopToBottom,

            grid_width: tile_matches
                .get_one::<String>("HORIZ_COUNT")
                .map(|v| parse_int(v))
                .map(GridWidthCapture::TilesInRow)
                .unwrap_or(GridWidthCapture::FullWidth),

            grid_height: tile_matches
                .get_one::<String>("VERT_COUNT")
                .map(|v| parse_int(v))
                .map(GridHeightCapture::TilesInColumn)
                .unwrap_or(GridHeightCapture::FullHeight)
        }
    }
    else {
        // Standard case
        if matches.get_flag("OVERSCAN") {
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::overscan(),
                display_address: DisplayCRTCAddress::new_overscan_from_page(2)
            }
        }
        else if matches.get_flag("FULLSCREEN") {
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::overscan(),
                display_address: DisplayCRTCAddress::new_overscan_from_page(2)
            }
        }
        else {
            // assume it is a standard screen
            let mut format = CPCScreenDimension::standard();
            if let Some(scr) = matches.subcommand_matches("scr") {
                if let Some(&r1) = scr.get_one("R1") {
                    format.horizontal_displayed = r1;
                }
                if let Some(&r6) = scr.get_one("R6") {
                    format.vertical_displayed = r6;
                }
            }
            OutputFormat::CPCMemory {
                output_dimension: format,
                display_address: DisplayCRTCAddress::new_standard_from_page(3)
            }
        }
    }
}

// TODO - Add the ability to import a target palette
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
/// The whole conversion is monomorphised on the colour type: the machine is
/// decided once, here, from the palette the user asked for, and nothing
/// downstream has to keep asking.
fn convert(matches: &ArgMatches, o: &dyn EventObserver) -> anyhow::Result<()> {
    match get_requested_palette(matches)? {
        AnyLockablePalette::GateArray(palette) => convert_with_palette(matches, palette, o),
        AnyLockablePalette::Asic(palette) => convert_with_palette(matches, palette, o)
    }
}

fn convert_with_palette<C: AmstradColor>(
    matches: &ArgMatches,
    palette: cpclib::image::ga::LockablePalette<C>,
    o: &dyn EventObserver
) -> anyhow::Result<()> {
    let input_file = matches.get_one::<Utf8PathBuf>("SOURCE").unwrap();
    let output_mode = matches
        .get_one::<String>("MODE")
        .unwrap()
        .parse::<u8>()
        .unwrap();
    let mut transformations = TransformationsList::default();

    if matches.get_flag("SKIP_ODD_PIXELS") {
        transformations = transformations.skip_odd_pixels();
    }
    if matches.contains_id("PIXEL_COLUMN_START") {
        transformations = transformations.column_start(
            matches
                .get_one::<String>("PIXEL_COLUMN_START")
                .unwrap()
                .parse::<u16>()
                .unwrap()
        )
    }
    if matches.contains_id("PIXEL_LINE_START") {
        transformations = transformations.line_start(
            matches
                .get_one::<String>("PIXEL_LINE_START")
                .unwrap()
                .parse::<u16>()
                .unwrap()
        )
    }
    if matches.contains_id("PIXEL_COLUMNS_KEPT") {
        transformations = transformations.columns_kept(
            matches
                .get_one::<String>("PIXEL_COLUMNS_KEPT")
                .unwrap()
                .parse::<u16>()
                .unwrap()
        )
    }
    if matches.contains_id("PIXEL_LINES_KEPT") {
        transformations = transformations.lines_kept(
            matches
                .get_one::<String>("PIXEL_LINES_KEPT")
                .unwrap()
                .parse::<u16>()
                .unwrap()
        )
    }

    let sub_sna = matches.subcommand_matches("sna");
    #[cfg(feature = "xferlib")]
    let sub_m4 = matches.subcommand_matches("m4");
    #[cfg(not(feature = "xferlib"))]
    let sub_m4: Option<&ArgMatches> = None;
    let sub_dsk = matches.subcommand_matches("dsk");
    let sub_sprite = matches.subcommand_matches("sprite");
    let sub_tile = matches.subcommand_matches("tile");
    let sub_exec = matches.subcommand_matches("exec");
    let sub_scr = matches.subcommand_matches("scr");

    let missing_pen = matches.get_one::<u8>("MISSING_PEN").map(|v| Pen::from(*v));

    let crop_if_too_large = matches.get_flag("CROP_IF_TOO_LARGE");
    let output_format = get_output_format(matches);
    let conversion = ImageConverter::convert(
        input_file,
        palette,
        output_mode.into(),
        transformations,
        output_format,
        crop_if_too_large,
        missing_pen,
        o
    )?;

    if sub_sprite.is_some() {
        // TODO share code with the tile branch

        let sub_sprite = sub_sprite.unwrap();

        // handle the sprite stuff
        match &conversion {
            Output::Sprite(sprite) | Output::SpriteAndMask { sprite, .. } => {
                let palette = sprite.palette();
                // Save the binary data of the palette if any
                do_export_palette!(sub_sprite, palette);

                // Save the binary data of the sprite
                if let Some(sprite_fname) = sub_sprite.get_one::<String>("SPRITE_FNAME") {
                    sprite
                        .save_sprite(sprite_fname)
                        .expect("Unable to create the sprite file");
                }

                sub_sprite
                    .get_one::<String>("CONFIGURATION")
                    .map(|conf_fname: &String| {
                        let mut file = File::create(conf_fname)
                            .expect("Unable to create the configuration file");
                        let fname = Utf8Path::new(conf_fname)
                            .file_stem()
                            .unwrap()
                            .replace(".", "_");
                        writeln!(&mut file, "{}_WIDTH equ {}", fname, sprite.bytes_width())
                            .unwrap();
                        writeln!(&mut file, "{}_HEIGHT equ {}", fname, sprite.height()).unwrap();
                    });
            },
            _ => unreachable!("{:?} not handled", conversion)
        }

        // handle the additional mask stuff
        if let Output::SpriteAndMask { mask, sprite } = &conversion {
            if let Some(mask_fname) = sub_sprite.get_one::<String>("MASK_FNAME") {
                mask.save_sprite(mask_fname)
                    .expect("Unable to create the mask file");
            }

            if let Some(code_fname) = sub_sprite.get_one::<String>("SPRITE_ASM") {
                assert_eq!(
                    mask.encoding(),
                    SpriteEncoding::Linear,
                    "Need to implement the other cases when needed"
                );

                let r1 = sub_sprite.get_one::<u8>("R1").cloned().unwrap_or_else(|| {
                    if matches.get_flag("OVERSCAN") || matches.get_flag("FULLSCREEN") {
                        96 / 2
                    }
                    else {
                        80 / 2
                    }
                });
                let label = sub_sprite
                    .get_one::<String>("SPRITE_ASM_LABEL")
                    .cloned()
                    .unwrap_or_else(|| code_fname.replace('.', "_"));

                // generate the code
                let code = match sub_sprite.get_one::<String>("SPRITE_ASM_KIND").unwrap().as_str() {
                    "masked" => cpclib::sprite_compiler::standard_sprite_compiler(
                        &label, sprite, mask, r1),
                    "backup+masked" => cpclib::sprite_compiler::standard_sprite_with_background_backup_and_restore_compiler(
                        &label, sprite, mask, r1),
                    rest => unreachable!("{rest} unhandled")
                };

                code.save(code_fname)
                    .expect("Unable to save generated code");
            }
        }
    }
    else if let Some(sub_tile) = sub_tile {
        // TODO share code with the sprite branch
        match &conversion {
            Output::TilesList {
                palette,
                list: tile_set,
                ..
            } => {
                // Save the palette
                do_export_palette!(sub_tile, palette);

                // Save the binary data of the tiles
                let tile_fname = Utf8Path::new(
                    sub_tile
                        .get_one::<String>("SPRITE_FNAME")
                        .expect("Missing tileset name")
                );
                let base = tile_fname.with_extension("").to_string();
                let extension = tile_fname.extension().unwrap_or("");
                for (i, data) in tile_set.iter().enumerate() {
                    let current_filename = format!("{base}_{i:03}.{extension}");
                    let mut file = File::create(current_filename.clone())
                        .unwrap_or_else(|_| panic!("Unable to build {current_filename}"));
                    file.write_all(data).unwrap();
                }
            },
            _ => unreachable! {}
        }
    }
    else if let Some(sub_scr) = sub_scr {
        let fname = sub_scr.get_one::<String>("SCR").unwrap();

        match &conversion {
            Output::CPCMemoryStandard(scr, palette) => {
                let scr = if sub_scr.contains_id("COMPRESSED") {
                    ocp::compress(scr, o)
                }
                else {
                    scr.to_vec()
                };

                fs_err::write(fname, &scr)?;

                do_export_palette!(sub_scr, palette);
            },

            Output::CPCMemoryOverscan(scr1, scr2, palette) => {
                if sub_scr.contains_id("COMPRESSED") {
                    unimplemented!();
                }

                let mut buffer = File::create(fname)?;
                buffer.write_all(scr1)?;
                if let Some(scr2) = scr2 {
                    buffer.write_all(scr2)?;
                }
                do_export_palette!(sub_scr, palette);
            },

            _ => unreachable!()
        };
    }
    else {
        // Make the conversion before feeding sna or dsk

        /// TODO manage the presence/absence of file in the dsk, the choice of filename and so on
        if sub_dsk.is_some() || sub_exec.is_some() {
            let code = match &conversion {
                Output::CPCMemoryStandard(memory, pal) => {
                    standard_linked_code(output_mode, &pal.clone().into_any(), memory)
                },

                Output::CPCMemoryOverscan(_memory1, _memory2, _pal) => unimplemented!(),

                _ => unreachable!()
            };

            let filename = {
                if sub_dsk.is_some() {
                    "test.bin"
                }
                else {
                    sub_exec
                        .as_ref()
                        .unwrap()
                        .get_one::<String>("FILENAME")
                        .unwrap()
                }
            };

            let file = assemble_to_amsdos_file(&code, filename, Default::default()).unwrap();

            if sub_exec.is_some() {
                let filename = Utf8Path::new(filename);
                let folder = filename.parent().unwrap();
                let folder = if folder == Utf8Path::new("") {
                    std::env::current_dir().unwrap()
                }
                else {
                    folder.canonicalize().unwrap()
                };
                let folder = Utf8PathBuf::from_path_buf(folder).unwrap();
                file.save_in_folder(folder)?;
            }
            else {
                let fname = sub_dsk.unwrap().get_one::<String>("DSK").unwrap();
                let p = Utf8Path::new(fname);

                let mut dsk = {
                    if p.exists() {
                        ExtendedDsk::open(p).unwrap()
                    }
                    else {
                        ExtendedDsk::default()
                    }
                };

                let head = Head::A;
                let _system = false;
                let _read_only = false;

                dsk.add_amsdos_file(
                    &file,
                    head,
                    false,
                    false,
                    AmsdosAddBehavior::ReplaceAndEraseIfPresent
                )
                .unwrap();
                dsk.save(fname).unwrap();
            }
        }
        if sub_sna.is_some() || sub_m4.is_some() {
            let (palette, code) = match &conversion {
                Output::CPCMemoryStandard(_memory, pal) => {
                    let pal = pal.clone().into_any();
                    let code = assemble(&standard_display_code(output_mode, &pal)).unwrap();
                    (pal, code)
                },

                Output::CPCMemoryOverscan(_memory1, _memory2, pal) => {
                    let pal = pal.clone().into_any();
                    let code =
                        assemble(&fullscreen_display_code(output_mode, 96 / 2, &pal)).unwrap();
                    (pal, code)
                },

                _ => unreachable!()
            };

            // Create a snapshot with a standard screen
            let mut sna = Snapshot::default();

            match &conversion {
                Output::CPCMemoryStandard(memory, _) => {
                    sna.add_data(memory.as_ref(), 0xC000)
                        .expect("Unable to add the image in the snapshot");
                },
                Output::CPCMemoryOverscan(memory1, memory2, _) => {
                    sna.add_data(memory1.as_ref(), 0x8000)
                        .expect("Unable to add the image in the snapshot");

                    if let Some(memory2) = memory2 {
                        sna.add_data(memory2.as_ref(), 0xC000)
                            .expect("Unable to add the image in the snapshot");
                    }
                },
                _ => unreachable!()
            };

            sna.add_data(&code, 0x4000).unwrap();
            sna.set_value(SnapshotFlag::Z80_PC, 0x4000).unwrap();

            // The ASIC has its own 6845, and an emulator has to be told to use
            // it - the display code we just generated talks to hardware a plain
            // CPC does not have. Both fields only exist from version 3 of the
            // snapshot format, so a Plus snapshot is saved as V3.
            let is_plus = matches!(palette, AnyPalette::Asic(_));
            let sna_version = if is_plus {
                sna.set_value(SnapshotFlag::CPC_TYPE, 4).unwrap(); // 6128 Plus
                sna.set_value(SnapshotFlag::CRTC_TYPE, 3).unwrap();
                sna::SnapshotVersion::V3
            }
            else {
                sna.set_value(SnapshotFlag::CPC_TYPE, 0).unwrap(); // CPC464
                sna.set_value(SnapshotFlag::CRTC_TYPE, 0).unwrap();
                sna::SnapshotVersion::V2
            };

            // A Plus palette does not fit these registers; its display code
            // installs it through the ASIC instead.
            if let Some(inks) = palette.gate_array_bytes() {
                sna.set_value(SnapshotFlag::GA_PAL(Some(16)), 0x54).unwrap();
                for (i, ink) in inks.iter().enumerate().take(16) {
                    sna.set_value(SnapshotFlag::GA_PAL(Some(i)), u16::from(*ink))
                        .unwrap();
                }
            }

            if let Some(sub_sna) = sub_sna {
                let sna_fname = sub_sna.get_one::<String>("SNA").unwrap();
                sna.save(sna_fname, sna_version)
                    .expect("Unable to save the snapshot");
            }
            else if let Some(sub_m4) = sub_m4 {
                #[cfg(feature = "xferlib")]
                {
                    let mut f = tempfile::Builder::new()
                        .suffix(".sna")
                        .tempfile()
                        .expect("Unable to create the temporary file");

                    sna.write_all(f.as_file_mut(), sna_version)
                        .expect("Unable to write the sna in the temporary file");

                    let xfer = CpcXfer::new(sub_m4.get_one::<String>("CPCM4").unwrap());

                    let tmp_file_name = f.path();
                    xfer.upload_and_run(tmp_file_name, None)
                        .expect("An error occurred while transferring the snapshot");
                }
            }
        }
    }

    Ok(())
}

pub fn build_cpc2img_args_parser() -> clap::Command {
    specify_palette!(
        clap::Command::new("cpc2png")
            .about("Generate PNG from CPC files")
            //           .subcommand_required(true) # too write seems seems to forbid the use of --help
            .arg(
                Arg::new("MODE")
                    .short('m')
                    .long("mode")
                    .help("Screen mode of the image to convert.")
                    .value_name("MODE")
                    .value_parser(0..=2)
                    .action(clap::ArgAction::Set)
                    .default_value("0")
            )
            .arg(
                Arg::new("MODE0RATIO")
                    .long("mode0ratio")
                    .help("Horizontally double the pixels")
                    .action(ArgAction::SetTrue)
            )
            .subcommand(
                Command::new("OCPPALETTECMD")
                    .about("Load an ocp palette file")
                    .name("palette")
            )
            .subcommand(
                Command::new("SPRITECMD")
                    .about("Load from a linear sprite data")
                    .name("sprite")
                    .arg(
                        Arg::new("WIDTH")
                            .long("width")
                            .required(true)
                            .help("Width of the sprite in pixels")
                    )
            )
            .subcommand(
                Command::new("SCREENCMD")
                    .about("Load from a 16kb screen data")
                    .name("screen")
                    .arg(
                        Arg::new("WIDTH")
                            .long("width")
                            .default_value("80")
                            .help("Width of the screen in bytes")
                    )
            )
            .arg(
                Arg::new("INPUT")
                    .help("File to Read. Can be a .scr, a .pal")
                    .required(true)
            )
            .arg(Arg::new("OUTPUT").required(true))
    )
}

pub fn build_img2cpc_args_parser() -> clap::Command {
    let args = specify_palette!(Command::new("CPC image conversion tool")
                    .version(built_info::PKG_VERSION)
                    .author("Krusty/Benediction")
                    .about("Simple CPC image conversion tool")
                    .arg(
                        Arg::new("SOURCE")
                            .help("Filename to convert")
//                            .last(true)
                            .required(true)
                            .value_parser(|source: &str| {
                              let p = Utf8PathBuf::from(source);
                              if p.exists() {
                                  Ok(p)
                              }
                              else {
                                  Err(format!("{source} does not exists!"))
                              }
                            })
                   )

                .arg(
                    Arg::new("MODE")
                        .short('m')
                        .long("mode")
                        .help("Screen mode of the image to convert.")
                        .value_name("MODE")
                        .default_value("0")
                        .value_parser(["0", "1", "2"])
                )
                .arg(
                    Arg::new("MISSING_PEN")
                        .long("missing-pen")
                        .help("Pen to use when the byte is too small")
                        .value_parser(value_parser!(u8))
                )
                .arg(
                    Arg::new("CROP_IF_TOO_LARGE")
                        .long("crop")
                        .help("Crop the picture if it is too large according  to the destination")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("FULLSCREEN")
                        .long("fullscreen")
                        .action(ArgAction::SetTrue)
                        .help("Specify a full screen displayed using 2 non consecutive banks.")
                        .conflicts_with("OVERSCAN")
                )
                .arg(
                    Arg::new("OVERSCAN")
                        .long("overscan")
                        .action(ArgAction::SetTrue)
                        .help("Specify an overscan screen (crtc meaning).")
                )
                .arg(
                    Arg::new("STANDARD")
                        .long("standard")
                        .action(ArgAction::SetTrue)
                        .help("Specify a standard screen manipulation.")
                        .conflicts_with("OVERSCAN")
                        .conflicts_with("FULLSCREEN")
                )
                .arg(
                    Arg::new("SKIP_ODD_PIXELS")
                        .long("skipoddpixels")
                        .short('s')
                        .help("Skip odd pixels when reading the image (usefull when the picture is mode 0 with duplicated pixels")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("PIXEL_COLUMN_START")
                    .long("columnstart")
                    .required(false)
                    .help("Number of pixel columns to skip on the left.")
                )
                .arg(
                    Arg::new("PIXEL_COLUMNS_KEPT")
                    .long("columnskept")
                    .required(false)
                    .help("Number of pixel columns to keep.")
                )
                .arg(
                    Arg::new("PIXEL_LINE_START")
                    .long("linestart")
                    .required(false)
                    .help("Number of pixel lines to skip.")
                )
                .arg(
                    Arg::new("PIXEL_LINES_KEPT")
                    .long("lineskept")
                    .required(false)
                    .help("Number of pixel lines to keep.")
                )
                    .subcommand(
                        Command::new("sna")
                            .about("Generate a snapshot with the converted image.")
                            .arg(
                                Arg::new("SNA")
                                    .help("snapshot filename to generate")
                                    .required(true)
                                    .value_parser(|sna: &str| {
                                        if sna.to_lowercase().ends_with("sna") {
                                            Ok(sna.to_owned())
                                        }
                                        else {
                                            Err(format!("{sna} has not a snapshot extension."))
                                        }
                                    })
                            )
                    )

                    .subcommand(
                        Command::new("dsk")
                        .about("Generate a DSK with an executable of the converted image.")
                        .arg(
                            Arg::new("DSK")
                            .help("dsk filename to generate")
                            .required(true)
                            .value_parser(|dsk: &str|{
                                if dsk.to_lowercase().ends_with("dsk") {
                                    Ok(dsk.to_owned())
                                }
                                else {
                                    Err(format!("{dsk} has not a dsk extention."))
                                }
                            })
                        )
                    )

                    .subcommand(
                        export_palette!(Command::new("scr")
                        .about("Generate an OCP SCR file")
                        .arg(
                            Arg::new("COMPRESSED")
                                .help("Request a compressed screen")
                                .long("compress")
                                .short('c')
                                .required(false)
                        )
                        .arg(
                            Arg::new("R1")
                                .help("Screen width in number of chars")
                                .long("r1")
                                .alias("horizontal-displayed-character-number")
                                .alias("width")
                                .alias("R1")
                                .value_parser(clap::value_parser!(u8))
                        )
                        .arg(
                            Arg::new("R6")
                                .help("Screen height in number of chars")
                                .long("r6")
                                .alias("vertical-displayed-character-number")
                                .alias("height")
                                .value_parser(clap::value_parser!(u8))
                        )
                        .arg(
                            Arg::new("SCR")
                            .long("output")
                            .short('o')
                            .help("Filename to generate")
                            .required(true)
                        )
                    ))

                    .subcommand(
                        Command::new("exec")
                        .about("Generate a binary file to manually copy in a DSK or M4 folder.")
                        .arg(
                            Arg::new("FILENAME")
                            .help("executable to generate")
                            .required(true)
                            .value_parser(|fname: &str|{
                                let fname = Utf8PathBuf::from(fname);
                                if let Some(ext) = fname.extension()
                                    && ext.len() > 3 {
                                        return Err(format!("{ext} is not a valid amsdos extension."));
                                    }

                                if let Some(stem) = fname.file_stem()
                                    && stem.len() > 8 {
                                        return Err(format!("{stem} is not a valid amsdos file stem."))
                                    }

                                Ok(fname)
                            })
                        )
                    )

                    .subcommand(
                        export_palette!(Command::new("sprite")
                        .about("Generate a sprite file to be included inside an application")
                        .arg(
                            Arg::new("CONFIGURATION")
                            .long("configuration")
                            .short('c')
                            .required(false)
                            .help("Name of the assembly file that contains the size of the sprite")
                        )
                        .arg(
                            Arg::new("FORMAT")
                            .long("format")
                            .short('f')
                            .default_value("linear")
                            .value_parser(["linear", "graycoded", "zigzag+graycoded"])
                        )

                        .arg(
                            Arg::new("SPRITE_FNAME")
                            .long("output")
                            .short('o')
                            .help("Filename where the sprite is stored")
                            .required_unless_present("SPRITE_ASM")
                        )

                        .arg(Arg::new("R1")
                                .help("Screen width in number of chars")
                                .long("r1")
                                .alias("horizontal-displayed-character-number")
                                .alias("width")
                                .alias("R1")
                                .value_parser(clap::value_parser!(u8))
                                .requires("SPRITE_ASM")
                        )

                        .arg(
                            Arg::new("SPRITE_ASM")
                            .long("code")
                            .help("Filename where to store the Z80 display code")
                            .required_unless_present("SPRITE_FNAME")
                            .requires("MASK_INK")
                            .requires("REPLACEMENT_INK")
                        )

                        .arg(
                            Arg::new("SPRITE_ASM_KIND")
                            .long("kind")
                            .help("The kind of code to generate")
                            .requires("SPRITE_ASM")
                            .value_parser(["masked", "backup+masked"])
                            .default_value("masked")
                        )


                        .arg(
                            Arg::new("SPRITE_ASM_LABEL")
                            .long("label")
                            .short('l')
                            .help("Label for the generated asm code")
                        )

                        .arg(
                            Arg::new("MASK_FNAME")
                            .long("mask")
                            .short('m')
                            .help("Filename where the mask is stored")
                            .requires("MASK_INK")
                            .requires("REPLACEMENT_INK")
                        )

                        .arg(
                            Arg::new("MASK_INK")
                            .long("mask-ink")
                            .help("Ink that represents the mask in the input image")
                            .value_parser(clap_parse_ink)
                        )
                        .arg(
                            Arg::new("REPLACEMENT_INK")
                            .long("replacement-ink")
                            .help("Ink that relace the mask ink in the sprite data")
                            .value_parser(clap_parse_ink)
                        )
                    ))

                    .subcommand(
                        export_palette!(Command::new("tile")
                            .about("Generate a list of sprites")
                            .arg(
                                Arg::new("WIDTH")
                                .long("width")
                                .short('W')
                                .required(true)
                                .help("Width (in bytes) of a tile")
                            )
                            .arg(
                                Arg::new("HEIGHT")
                                .long("height")
                                .short('H')
                                .required(true)
                                .help("Height (in lines) of a tile")
                            )
                            .arg(
                                Arg::new("HORIZ_COUNT")
                                .long("horiz_count")
                                .required(false)
                                .help("Horizontal number of tiles to extract. Extra tiles are ignored")
                            )
                            .arg(
                                Arg::new("VERT_COUNT")
                                .long("vert_count")
                                .required(false)
                                .help("Vertical number of tiles to extract. Extra tiles are ignored")
                            )
                            .arg(
                                Arg::new("CONFIGURATION")
                                .long("configuration")
                                .short('c')
                                .required(false)
                                .help("Name of the assembly file that contains the size of the sprite")
                            )
                            .arg(
                                Arg::new("FORMAT")
                                .long("format")
                                .short('f')
                                .value_parser(["linear", "graycoded", "zigzag+graycoded"])
                                .default_value("linear")
                            )
                            .arg(
                                Arg::new("SPRITE_FNAME")
                                .short('o')
                                .long("output")
                                .help("Filename to generate. Will be postfixed by the number")
                                .required(true)
                            )

                    ))


                );

    if cfg!(feature = "xferlib") {
        let subcommand = Command::new("m4")
            .about("Directly send the code on the M4 through a snapshot")
            .arg(Arg::new("CPCM4").help("Address of the M4").required(true));

        let subcommand = if cfg!(feature = "watch") {
            subcommand.arg(
                Arg::new("WATCH")
                .help("Monitor the source file modification and restart the conversion and transfer automatically. Picture must ALWAYS be valid.")
                .long("watch")
            )
        }
        else {
            subcommand
        };
        args.subcommand(subcommand)
    }
    else {
        args
    }
}

/// Decode CPC bytes back into an image, using the palette they were encoded
/// with - whichever machine that palette belongs to.
fn cpc2img_decode<C: AmstradColor>(
    matches: &ArgMatches,
    data: &[u8],
    mode: Mode,
    palette: &cpclib::image::ga::Palette<C>
) -> cpclib::image::image::ColorMatrix<C> {
    if let Some(sprite) = matches.subcommand_matches("sprite") {
        let width: usize = sprite.get_one::<String>("WIDTH").unwrap().parse().unwrap();
        cpclib::image::image::ColorMatrix::from_sprite(data, width as _, mode, palette)
    }
    else if let Some(screen) = matches.subcommand_matches("screen") {
        let width: usize = screen.get_one::<String>("WIDTH").unwrap().parse().unwrap();
        cpclib::image::image::ColorMatrix::from_screen(data, width as _, mode, palette)
    }
    else {
        unreachable!()
    }
}

pub fn process_cpc2img(matches: &ArgMatches, _args: Command) -> anyhow::Result<()> {
    let input_fname = matches.get_one::<String>("INPUT").unwrap();
    let output_fname = matches.get_one::<String>("OUTPUT").unwrap();
    let mode = *matches.get_one::<i64>("MODE").unwrap() as u8;
    let mode = Mode::from(mode);

    let mode0ratio = matches.get_flag("MODE0RATIO");
    // read the data file
    let data = fs_err::read(input_fname).expect("Unable to read input file");

    // remove header if any
    let data = if data.len() >= 128
        && cpclib::disc::amsdos::AmsdosHeader::from_buffer(&data).is_checksum_valid()
        && data[..128].iter().map(|&b| b as usize).sum::<usize>() != 0
    {
        &data[128..]
    }
    else {
        &data
    };

    // Rendering a palette file as an image is a Gate Array affair: both formats
    // it reads (my 17-byte one, and OCP's) store ink numbers. It also ignores
    // the palette the user asked for - the file *is* the palette.
    if matches.subcommand_matches("palette").is_some() {
        let palettes = if data.len() % 17 == 0 {
            // this is my gate array format
            data.chunks(17)
                .map(|p| {
                    let inks = p.iter().map(|b| Ink::from(*b));
                    Palette::from_iter(inks)
                })
                .collect_vec()
        }
        else {
            // this is the real OCP format
            OcpPalette::from_buffer(data)
                .palettes()
                .iter()
                .cloned()
                .collect_vec()
        };

        let rows = palettes
            .into_iter()
            .map(|p| ColorMatrix::from_palette(&p, 32))
            .collect_vec();

        let mut matrix = ColorMatrix::vstack(&rows);
        if mode0ratio {
            matrix.double_horizontally();
        }
        matrix
            .as_image()
            .save(output_fname)
            .expect("Error while saving the file");
        return Ok(());
    }

    match get_requested_palette(matches)? {
        AnyLockablePalette::GateArray(palette) => {
            let mut matrix = cpc2img_decode(matches, data, mode, palette.as_palette());
            if mode0ratio {
                matrix.double_horizontally();
            }
            matrix
                .as_image()
                .save(output_fname)
                .expect("Error while saving the file");
        },
        AnyLockablePalette::Asic(palette) => {
            let mut matrix = cpc2img_decode(matches, data, mode, palette.as_palette());
            if mode0ratio {
                matrix.double_horizontally();
            }
            matrix
                .as_image()
                .save(output_fname)
                .expect("Error while saving the file");
        }
    }

    Ok(())
}

pub fn process_img2cpc(
    matches: &ArgMatches,
    _args: Command,
    o: &dyn EventObserver
) -> anyhow::Result<()> {
    // Note: clap automatically handles --help, no need to check manually
    // Removed: if matches.get_flag("help") { ... }

    #[cfg(feature = "xferlib")]
    let has_m4 = matches.subcommand_matches("m4").is_some();
    #[cfg(not(feature = "xferlib"))]
    let has_m4 = false;

    if !has_m4
        && matches.subcommand_matches("dsk").is_none()
        && matches.subcommand_matches("sna").is_none()
        && matches.subcommand_matches("sprite").is_none()
        && matches.subcommand_matches("tile").is_none()
        && matches.subcommand_matches("exec").is_none()
        && matches.subcommand_matches("scr").is_none()
    {
        o.emit_stderr("[ERROR] you have not specified any action to do.");
        std::process::exit(exitcode::USAGE);
    }

    // A rejected combination of flags is a user error, not a bug: hand it back
    // to `main` to be printed, rather than unwinding with a backtrace over it.
    // (The observer this runs with discards what it is given, so reporting it
    // here would report it to nobody.)
    convert(matches, o)?;

    #[cfg(feature = "xferlib")]
    if let Some(sub_m4) = matches.subcommand_matches("m4") {
        o.emit_stderr("hmmm seems to not be coded yet");
        #[cfg(feature = "watch")]
        if sub_m4.contains_id("WATCH") {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
                move |res| tx.send(res).unwrap(),
                notify::Config::default()
            )?;
            watcher.watch(
                matches
                    .get_one::<Utf8PathBuf>("SOURCE")
                    .unwrap()
                    .as_std_path(),
                RecursiveMode::NonRecursive
            )?;

            for res in rx {
                match res {
                    Ok(notify::event::Event {
                        kind: notify::event::EventKind::Modify(_),
                        ..
                    })
                    | Ok(notify::event::Event {
                        kind: notify::event::EventKind::Create(_),
                        ..
                    }) => {
                        if let Err(e) = convert(matches, o) {
                            return Err(Error::msg(format!(
                                "[ERROR] Unable to convert the image {e}"
                            )));
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// Palette specification for the fade command (no unlock-pens option).
#[derive(clap::Args, Clone, Debug)]
pub struct FadePaletteArgs {
    /// OCP PAL file. The first palette among 12 is used
    #[arg(long, value_parser = cpclib::common::existing_utf8pathbuf_value_parser)]
    pub pal: Option<Utf8PathBuf>,

    /// Separated list of ink numbers. Use ',' as separator
    #[arg(long, conflicts_with = "pal")]
    pub pens: Option<String>,

    /// Ink number of pen 0
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen0: Option<u8>,
    /// Ink number of pen 1
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen1: Option<u8>,
    /// Ink number of pen 2
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen2: Option<u8>,
    /// Ink number of pen 3
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen3: Option<u8>,
    /// Ink number of pen 4
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen4: Option<u8>,
    /// Ink number of pen 5
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen5: Option<u8>,
    /// Ink number of pen 6
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen6: Option<u8>,
    /// Ink number of pen 7
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen7: Option<u8>,
    /// Ink number of pen 8
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen8: Option<u8>,
    /// Ink number of pen 9
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen9: Option<u8>,
    /// Ink number of pen 10
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen10: Option<u8>,
    /// Ink number of pen 11
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen11: Option<u8>,
    /// Ink number of pen 12
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen12: Option<u8>,
    /// Ink number of pen 13
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen13: Option<u8>,
    /// Ink number of pen 14
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen14: Option<u8>,
    /// Ink number of pen 15
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen15: Option<u8>,
    /// Ink number of pen 16 (border)
    #[arg(long, conflicts_with = "pens", conflicts_with = "pal")]
    pub pen16: Option<u8>
}

impl FadePaletteArgs {
    pub fn to_lockable_palette(&self) -> Result<LockablePalette, AmsdosError> {
        if let Some(pens) = &self.pens {
            let numbers = pens
                .split(',')
                .map(|ink| {
                    cpclib::common::parse_value::<_, ()>
                        .parse(BStr::new(ink))
                        .unwrap_or_else(|_| {
                            Ink::from(ink.replace("GA_", "")).gate_array_value() as _
                        })
                })
                .map(|n: u32| Ink::from(n))
                .collect::<Vec<_>>();
            Ok(LockablePalette::unlocked(numbers.into()))
        }
        else if let Some(fname) = &self.pal {
            let (mut data, _header) = cpclib::disc::read(fname)?;
            let data = data.make_contiguous();
            let pal = OcpPalette::from_buffer(data);
            Ok(LockablePalette::unlocked(pal.palette(0).clone()))
        }
        else {
            let mut palette = Palette::empty();
            let mut one_pen_set = false;
            let pen_values = [
                self.pen0, self.pen1, self.pen2, self.pen3, self.pen4, self.pen5, self.pen6,
                self.pen7, self.pen8, self.pen9, self.pen10, self.pen11, self.pen12, self.pen13,
                self.pen14, self.pen15
            ];
            for (i, pen) in pen_values.iter().enumerate() {
                if let Some(ink) = pen {
                    one_pen_set = true;
                    palette.set(i as i32, *ink);
                }
            }
            if one_pen_set {
                Ok(LockablePalette::locked(palette))
            }
            else {
                Ok(LockablePalette::unlocked(palette))
            }
        }
    }
}

/// Algorithm to use for fade generation.
#[derive(clap::Subcommand, Clone, Debug)]
pub enum FadeAlgorithm {
    /// Use the algorithm described in http://cpc.sylvestre.org/technique/technique_coul5.html
    #[command(alias = "superlsy")]
    Rgb
}

/// Arguments for the fade tool.
#[derive(clap::Parser, Clone, Debug)]
#[command(name = "fade")]
pub struct FadeArgs {
    /// Use symbols in assembly generated code
    #[arg(long)]
    pub symbols: bool,

    /// Preview the generated palette in the terminal
    #[arg(long)]
    pub preview: bool,

    /// Filename to store the result. Console otherwise
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    #[command(flatten)]
    pub palette: FadePaletteArgs,

    #[command(subcommand)]
    pub algorithm: FadeAlgorithm
}

/// Process the fade command with the given parsed arguments.
pub fn fade_process(args: &FadeArgs, o: &dyn EventObserver) -> Result<(), String> {
    let palette = args
        .palette
        .to_lockable_palette()
        .map_err(|e| e.to_string())?;

    let fades = match &args.algorithm {
        FadeAlgorithm::Rgb => palette.rgb_fadout()
    };

    let content = if args.symbols {
        fade_output_symbols_assembly(&fades)
    }
    else {
        fade_output_ga_assembly(&fades)
    };

    if let Some(fname) = &args.output {
        fs_err::write(fname, &content).expect("Error while saving file");
    }
    else {
        o.emit_stdout(&content);
    }

    if args.preview {
        fade_display_preview(&fades);
    }

    Ok(())
}

pub fn fade_build_args() -> Command {
    use clap::CommandFactory;
    FadeArgs::command()
        // subcommand_required(true) would forbid the use of --help
        .subcommand_required(false)
}

fn fade_output_ga_assembly(palettes: &[Palette]) -> String {
    palettes
        .iter()
        .map(|palette| {
            let repr = palette
                .inks()
                .into_iter()
                .map(|ink: Ink| ink.gate_array_value())
                .map(|ga| format!("0x{ga:x}"))
                .join(",");
            format!("\tdb {repr}")
        })
        .join("\n")
        + "\n"
}

fn fade_output_symbols_assembly(palettes: &[Palette]) -> String {
    palettes
        .iter()
        .map(|palette| {
            let repr = palette
                .inks()
                .into_iter()
                .map(|ink: Ink| format!("GA_{ink}"))
                .join(",");
            format!("\tdb {repr}")
        })
        .join("\n")
        + "\n"
}

fn fade_display_preview(palettes: &[Palette]) {
    for palette in palettes {
        for ink in palette.inks() {
            let dyncolor = DynColors::Rgb(ink.color()[0], ink.color()[1], ink.color()[2]);
            print!("{}", "   ".on_color(dyncolor));
        }
        println!()
    }
}

pub fn fade_handle_matches(matches: &ArgMatches, o: &dyn EventObserver) -> Result<(), String> {
    use clap::FromArgMatches;
    let args = FadeArgs::from_arg_matches(matches).map_err(|e| e.to_string())?;
    fade_process(&args, o)
}

#[cfg(test)]
mod plus_display_code_tests {
    //! The Z80 that installs a palette, for each machine.

    use cpclib::image::asic::{AsicColor, AsicColorComponent};
    use cpclib::image::ga::{AnyPalette, Palette};
    use cpclib::{Ink, Pen};

    use super::*;

    fn asic_palette() -> AnyPalette {
        let mut palette = Palette::<AsicColor>::empty();
        for pen in 0..16u8 {
            palette.set(
                pen as i32,
                AsicColor::new(
                    AsicColorComponent::from(pen),
                    AsicColorComponent::from(0u8),
                    AsicColorComponent::from(0xFu8)
                )
            );
        }
        AnyPalette::Asic(palette)
    }

    fn gate_array_palette() -> AnyPalette {
        let mut palette = Palette::<Ink>::empty();
        for pen in 0..16u8 {
            palette.set(pen as i32, Ink::from(pen));
        }
        AnyPalette::GateArray(palette)
    }

    /// The Gate Array path must be untouched by the Plus work: 16 `out`s
    /// through `0x7f00`, and no data hanging off the end of the routine.
    #[test]
    fn a_gate_array_palette_is_still_written_pen_by_pen() {
        let code = fullscreen_display_code(0, 48, &gate_array_palette());
        assert_eq!(code.matches("out (c), a").count(), 16, "{code}");
        assert!(!code.contains("palette_tab"), "{code}");
        assemble(&code).expect("the Gate Array display code must assemble");
    }

    /// The Plus path unlocks the ASIC, copies 32 bytes to `0x6400`, and locks
    /// it again.
    #[test]
    fn an_asic_palette_is_copied_through_the_asic() {
        let code = fullscreen_display_code(0, 48, &asic_palette());
        for expected in [
            "ld bc, 0x7fb8",
            "ld hl, palette_tab",
            "ld de, 0x6400",
            "ld bc, 32",
            "ldir",
            "ld bc, 0x7fa0"
        ] {
            assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
        }
        assemble(&code).expect("the ASIC display code must assemble");
    }

    /// The 32 bytes are data. They must sit *after* the `jp` that closes the
    /// display loop, never in its path.
    #[test]
    fn the_asic_palette_bytes_come_after_the_final_jump() {
        let code = fullscreen_display_code(0, 48, &asic_palette());
        let jump = code.rfind("jp frame_loop").expect("the display loop must end with its jump");
        let table = code.find("\npalette_tab\n").expect("the bytes must be labelled");
        assert!(
            table > jump,
            "palette_tab is emitted before the routine's final jump:\n{code}"
        );
    }

    /// A snapshot's `GA_PAL` registers cannot carry 12-bit colours, so the
    /// standard routine has to install a Plus palette itself - whereas for the
    /// Gate Array the snapshot still does it.
    #[test]
    fn the_standard_routine_installs_only_what_a_snapshot_cannot() {
        let asic = standard_display_code(0, &asic_palette());
        assert!(asic.contains("ldir"), "{asic}");
        assemble(&asic).expect("the standard ASIC display code must assemble");

        let ga = standard_display_code(0, &gate_array_palette());
        assemble(&ga).expect("the standard Gate Array display code must assemble");
    }

    /// The exported bytes are the ones the routine copies, in `.kit` order.
    #[test]
    fn the_emitted_bytes_are_the_palette_itself() {
        let palette = asic_palette();
        let bytes = palette.asic_bytes().unwrap();
        assert_eq!(bytes.len(), 32);
        // pen 3 was built as red=3, green=0, blue=15
        assert_eq!(bytes[6], 0x3F, "byte 0 packs red then blue");
        assert_eq!(bytes[7], 0x00, "byte 1 holds green alone");
        let _ = Pen::from(3);
    }
}

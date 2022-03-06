#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    nonstandard_style,
    rust_2018_idioms,
    unused,
    warnings
)]
#![deny(clippy::pedantic)]
#![allow(unused)]

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow;
use clap::{Arg, ArgMatches, Command};
use cpclib::assembler::preamble::*;
use cpclib::common::clap;
use cpclib::disc::amsdos::*;
use cpclib::disc::edsk::ExtendedDsk;
use cpclib::image::ga::Palette;
use cpclib::image::imageconverter::*;
use cpclib::image::ocp;
use cpclib::sna;
use cpclib::sna::*;
#[cfg(feature = "xferlib")]
use cpclib::xfer::CpcXfer;
use crossbeam_channel::unbounded;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tempfile::Builder;

macro_rules! export_palette {
    ($e: expr) => {
        $e.arg(
            Arg::new("EXPORT_PALETTE")
                .long("palette")
                .short('p')
                .takes_value(true)
                .required(false)
                .help("Name of the binary file that contains the palette (Gate Array format)"),
        )
        .arg(
            Arg::new("EXPORT_INKS")
            .long("inks")
            .short('i')
            .takes_value(true)
            .required(false)
            .help("Name of the binary file that contains the ink numbers (usefull for system based color change)")
        )
        .arg(
            Arg::new("EXPORT_PALETTE_FADEOUT")
                .long("palette_fadeout")
                .takes_value(true)
                .required(false)
                .help("Name of the file that will contain all the steps for a fade out transition (Gate Array format)")
        )
        .arg(
            Arg::new("EXPORT_INK_FADEOUT")
                .long("ink_fadeout")
                .takes_value(true)
                .required(false)
                .help("Name of the file that will contain all the steps for a fade out transition")
        )
    };
}

macro_rules! do_export_palette {
    ($arg: expr, $palette: ident) => {
        if let Some(palette_fname) = $arg.value_of("EXPORT_PALETTE") {
            let mut file = File::create(palette_fname).expect("Unable to create the palette file");
            let p: Vec<u8> = $palette.into();
            file.write_all(&p).unwrap();
        }

        if let Some(fade_fname) = $arg.value_of("EXPORT_PALETTE_FADEOUT") {
            let palettes = $palette.rgb_fadout();
            let bytes = palettes.iter().fold(Vec::<u8>::default(), |mut acc, x| {
                acc.extend(&x.to_gate_array_with_default(0.into()));
                acc
            });

            assert_eq!(palettes.len() * 17, bytes.len());

            let mut file = File::create(fade_fname).expect("Unable to create the fade out file");
            file.write_all(&bytes).unwrap();
        }

        if let Some(palette_fname) = $arg.value_of("EXPORT_INKS") {
            let mut file = File::create(palette_fname).expect("Unable to create the inks file");
            let inks = $palette
                .inks()
                .iter()
                .map(|i| i.number())
                .collect::<Vec<_>>();
            file.write_all(&inks).unwrap();
        }

        if let Some(fade_fname) = $arg.value_of("EXPORT_INK_FADEOUT") {
            let palettes = $palette.rgb_fadout();
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
    };
}

/// Compress data using lz4 algorithm.
/// Should be decompressed on client side.
fn lz4_compress(bytes: &[u8]) -> Vec<u8> {
    let mut res = Vec::new();
    let mut encoder = lz4::EncoderBuilder::new().build(&mut res).unwrap();
    let mut bytes = bytes.clone();

    std::io::copy(&mut bytes, &mut encoder).unwrap();
    encoder.finish();
    res
}

fn palette_code(pal: &Palette) -> String {
    let mut asm = " ld bc, 0x7f00\n".to_string();
    // TODO create the linker

    for idx in 0..(16 / 2) {
        asm += &format!("\tld hl, 256*{} + {} : out (c), c : out (c), h : inc c : out (c), c: out (c), l : inc c\n", 
            pal[2*idx + 0].gate_array(), 
            pal[2*idx + 1].gate_array()
        )
    }

    return asm;
}

fn standard_linked_code(mode: u8, pal: &Palette, screen: &[u8]) -> String {
    let parser_context = ParserContext::default();
    let base_code = standard_display_code(mode);
    format!(
        "   org 0x1000
        di
        ld sp, $

        ; Select palette
        {palette}

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
        palette = palette_code(pal),
        decompressor = include_str!("lz4_docent.asm"),
        code = defb_elements(&assemble(&base_code, &parser_context).unwrap()),
        screen = defb_elements(screen)
    )
}

// Produce the code that display a standard screen
fn standard_display_code(mode: u8) -> String {
    format!(
        "
        org 0x4000
        di
        ld bc, 0x7f00 + 0x{:x}
        out (c), c
        jp $
    ",
        match mode {
            0 => 0x8C,
            1 => 0x8D,
            2 => 0x8E,
            _ => unreachable!()
        }
    )
}

fn fullscreen_display_code(mode: u8, crtc_width: usize, palette: &Palette) -> String {
    let code_mode = match mode {
        0 => 0x8C,
        1 => 0x8D,
        2 => 0x8E,
        _ => unreachable!()
    };

    let r12 = 0x20 + 0b0000_1100;

    let mut palette_code = String::new();
    palette_code += "\tld bc, 0x7f00\n";
    for i in 0..16 {
        palette_code += &format!(
            "\tld a, {}\n\t out (c), c\n\tout (c), a\n\t inc c\n",
            palette.get(i.into()).gate_array()
        );
    }

    format!(
        "
        org 0x4000

        di
        ld hl, 0xc9fb
        ld (0x38), hl
        ld sp, $
        ei

        ld bc, 0x7f00 + 0x{:x}
        out (c), c

        ld bc, 0xbc00 + 1
        out (c), c
        ld bc, 0xbd00 + {}
        out (c), c

        ld bc, 0xbc00 + 2
        out (c), c
        ld bc, 0xbd00 + 50
        out (c), c

        ld bc, 0xbc00 + 12
        out (c), c
        ld bc, 0xbd00 + {}
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

        {}

frame_loop
        ld b, 0xf5
vsync_loop
        in a, (c)
        rra
        jr nc, vsync_loop




        jp frame_loop
    ",
        code_mode, crtc_width, r12, palette_code
    )
}

fn overscan_display_code(mode: u8, crtc_width: usize, pal: &Palette) -> String {
    fullscreen_display_code(mode, crtc_width, pal)
}

fn parse_int(repr: &str) -> usize {
    repr.parse::<usize>()
        .expect(&format!("Error when converting {} as integer", repr))
}

#[allow(clippy::if_same_then_else)] // false positive
fn get_output_format(matches: &ArgMatches) -> OutputFormat {
    if let Some(sprite_matches) = matches.subcommand_matches("sprite") {
        match sprite_matches.value_of("FORMAT").unwrap() {
            "linear" => OutputFormat::LinearEncodedSprite,
            "graycoded" => OutputFormat::GrayCodedSprite,
            "zigzag+graycoded" => OutputFormat::ZigZagGrayCodedSprite,
            _ => unimplemented!()
        }
    }
    else if let Some(tile_matches) = matches.subcommand_matches("tile") {
        OutputFormat::TileEncoded {
            tile_width: TileWidthCapture::NbBytes(parse_int(
                tile_matches
                    .value_of("WIDTH")
                    .expect("--width argument missing")
            )),

            tile_height: TileHeightCapture::NbLines(parse_int(
                tile_matches
                    .value_of("HEIGHT")
                    .expect("--height argument missing")
            )),

            horizontal_movement: TileHorizontalCapture::AlwaysFromLeftToRight,
            vertical_movement: TileVerticalCapture::AlwaysFromTopToBottom,

            grid_width: tile_matches
                .value_of("HORIZ_COUNT")
                .map(|v| parse_int(v))
                .map(|v| GridWidthCapture::TilesInRow(v))
                .unwrap_or(GridWidthCapture::FullWidth),

            grid_height: tile_matches
                .value_of("VERT_COUNT")
                .map(|v| parse_int(v))
                .map(|v| GridHeightCapture::TilesInColumn(v))
                .unwrap_or(GridHeightCapture::FullHeight)
        }
    }
    else {
        // Standard case
        if matches.is_present("OVERSCAN") {
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::overscan(),
                display_address: DisplayCRTCAddress::new_overscan_from_page(2)
            }
        }
        else if matches.is_present("FULLSCREEN") {
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::overscan(),
                display_address: DisplayCRTCAddress::new_overscan_from_page(2)
            }
        }
        else {
            // assume it is a standard screen
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::standard(),
                display_address: DisplayCRTCAddress::new_standard_from_page(3)
            }
        }
    }
}

fn get_requested_palette(matches: &ArgMatches) -> Option<Palette> {
    if matches.is_present("PENS") {
        let numbers = matches
            .value_of("PENS")
            .unwrap()
            .split(",")
            .map(|ink| ink.parse::<u8>().unwrap())
            .collect::<Vec<_>>();
        return Some(numbers.into());
    }
    else {
        let mut one_pen_set = false;
        let mut palette = Palette::new();
        for i in 0..16 {
            let key = format!("PEN{}", i);
            if matches.is_present(&key) {
                one_pen_set = true;
                palette.set(i, matches.value_of(&key).unwrap().parse::<u8>().unwrap())
            }
        }

        if one_pen_set {
            return Some(palette);
        }
        else {
            return None;
        }
    }
}

// TODO - Add the ability to import a target palette
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
fn convert(matches: &ArgMatches) -> anyhow::Result<()> {
    let input_file = matches.value_of("SOURCE").unwrap();
    let output_mode = matches.value_of("MODE").unwrap().parse::<u8>().unwrap();
    let mut transformations = TransformationsList::default();

    let palette = get_requested_palette(matches);

    if matches.is_present("SKIP_ODD_PIXELS") {
        transformations = transformations.skip_odd_pixels();
    }

    let sub_sna = matches.subcommand_matches("sna");
    let sub_m4 = matches.subcommand_matches("m4");
    let sub_dsk = matches.subcommand_matches("dsk");
    let sub_sprite = matches.subcommand_matches("sprite");
    let sub_tile = matches.subcommand_matches("tile");
    let sub_exec = matches.subcommand_matches("exec");
    let sub_scr = matches.subcommand_matches("scr");

    let output_format = get_output_format(&matches);
    let conversion = ImageConverter::convert(
        input_file,
        palette,
        output_mode.into(),
        transformations,
        &output_format
    )?;

    let parser_context = ParserContext::default();

    if sub_sprite.is_some() {
        // TODO share code with the tile branch

        let sub_sprite = sub_sprite.unwrap();
        match &conversion {
            Output::LinearEncodedSprite {
                data,
                bytes_width,
                height,
                palette
            }
            | Output::GrayCodedSprite {
                data,
                bytes_width,
                height,
                palette
            }
            | Output::ZigZagGrayCodedSprite {
                data,
                bytes_width,
                height,
                palette
            } => {
                // Save the palette
                do_export_palette!(sub_sprite, palette);

                // Save the binary data of the sprite
                let sprite_fname = sub_sprite.value_of("SPRITE_FNAME").unwrap();
                let mut file =
                    File::create(sprite_fname).expect("Unable to create the sprite file");
                file.write_all(&data).unwrap();

                // Save the binary data of the palette if any
                sub_sprite
                    .value_of("CONFIGURATION")
                    .and_then(|conf_fname: &str| {
                        let mut file = File::create(conf_fname)
                            .expect("Unable to create the configuration file");
                        let fname = std::path::Path::new(conf_fname)
                            .file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .replace(".", "_");
                        writeln!(&mut file, "{}_WIDTH equ {}", fname, bytes_width).unwrap();
                        writeln!(&mut file, "{}_HEIGHT equ {}", fname, height).unwrap();
                        Some(())
                    });
            }
            _ => unreachable!()
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
                let tile_fname = Path::new(
                    sub_tile
                        .value_of("SPRITE_FNAME")
                        .expect("Missing tileset name")
                );
                let base = tile_fname
                    .with_extension("")
                    .as_os_str()
                    .to_str()
                    .unwrap()
                    .to_owned();
                let extension = tile_fname.extension().unwrap().to_str().unwrap();
                for (i, data) in tile_set.iter().enumerate() {
                    let current_filename = format!("{}_{:03}.{}", base, i, extension);
                    let mut file = File::create(current_filename.clone())
                        .expect(&format!("Unable to build {}", current_filename));
                    file.write_all(data).unwrap();
                }
            }
            _ => unreachable! {}
        }
    }
    else if let Some(sub_scr) = sub_scr {
        let fname = dbg!(sub_scr.value_of("SCR").unwrap());

        match &conversion {
            Output::CPCMemoryStandard(scr, palette) => {
                let scr = if sub_scr.is_present("COMPRESSED") {
                    ocp::compress(&scr)
                }
                else {
                    scr.to_vec()
                };

                std::fs::write(fname, &scr)?;

                do_export_palette!(sub_scr, palette);
            }

            Output::CPCMemoryOverscan(scr1, scr2, palette) => {
                if sub_scr.is_present("COMPRESSED") {
                    unimplemented!();
                }

                let mut buffer = File::create(fname)?;
                buffer.write_all(scr1)?;
                buffer.write_all(scr2)?;
                do_export_palette!(sub_scr, palette);
            }

            _ => unreachable!()
        };
    }
    else {
        // Make the conversion before feeding sna or dsk

        /// TODO manage the presence/absence of file in the dsk, the choice of filename and so on
        if sub_dsk.is_some() || sub_exec.is_some() {
            let code = match &conversion {
                Output::CPCMemoryStandard(memory, pal) => {
                    standard_linked_code(output_mode, pal, memory)
                }

                Output::CPCMemoryOverscan(_memory1, _memory2, pal) => unimplemented!(),

                _ => unreachable!()
            };

            let filename = {
                if sub_dsk.is_some() {
                    "test.bin"
                }
                else {
                    sub_exec.as_ref().unwrap().value_of("FILENAME").unwrap()
                }
            };

            let file = assemble_to_amsdos_file(&code, filename, &parser_context).unwrap();

            if sub_exec.is_some() {
                let filename = Path::new(filename);
                let folder = filename.parent().unwrap();
                let folder = if folder == Path::new("") {
                    std::env::current_dir().unwrap()
                }
                else {
                    folder.canonicalize().unwrap()
                };
                file.save_in_folder(folder)?;
            }
            else {
                use cpclib::disc::cfg::DiscConfig;
                let cfg = cpclib::disc::cfg::DiscConfig::single_head_data_format();
                let dsk = cpclib::disc::builder::build_disc_from_cfg(&cfg);
                let mut manager = AmsdosManager::new_from_disc(dsk, 0);
                manager.add_file(&file, false, false).unwrap();
                manager
                    .dsk()
                    .save(sub_dsk.unwrap().value_of("DSK").unwrap())
                    .unwrap();
            }
        }
        if sub_sna.is_some() || sub_m4.is_some() {
            let (palette, code) = match &conversion {
                Output::CPCMemoryStandard(_memory, pal) => {
                    (pal, assemble(&standard_display_code(output_mode), &parser_context).unwrap())
                }

                Output::CPCMemoryOverscan(_memory1, _memory2, pal) => {
                    let code =
                        assemble(&fullscreen_display_code(output_mode, 96 / 2, &pal), &parser_context).unwrap();
                    (pal, code)
                }

                _ => unreachable!()
            };

            // Create a snapshot with a standard screen
            let mut sna = Snapshot::default();

            match &conversion {
                Output::CPCMemoryStandard(memory, _) => {
                    sna.add_data(&memory.to_vec(), 0xC000)
                        .expect("Unable to add the image in the snapshot");
                }
                Output::CPCMemoryOverscan(memory1, memory2, _) => {
                    sna.add_data(&memory1.to_vec(), 0x8000)
                        .expect("Unable to add the image in the snapshot");
                    sna.add_data(&memory2.to_vec(), 0xC000)
                        .expect("Unable to add the image in the snapshot");
                }
                _ => unreachable!()
            };

            sna.add_data(&code, 0x4000).unwrap();
            sna.set_value(SnapshotFlag::Z80_PC, 0x4000).unwrap();
            sna.set_value(SnapshotFlag::GA_PAL(Some(16)), 0x54).unwrap();
            for i in 0..16 {
                sna.set_value(
                    SnapshotFlag::GA_PAL(Some(i)),
                    u16::from(palette.get((i as i32).into()).gate_array())
                )
                .unwrap();
            }

            if let Some(sub_sna) = sub_sna {
                let sna_fname = sub_sna.value_of("SNA").unwrap();
                sna.save(sna_fname, sna::SnapshotVersion::V2)
                    .expect("Unable to save the snapshot");
            }
            else if let Some(sub_m4) = sub_m4 {
                #[cfg(feature = "xferlib")]
                {
                    let mut f = Builder::new()
                        .suffix(".sna")
                        .tempfile()
                        .expect("Unable to create the temporary file");

                    sna.write(f.as_file_mut(), cpclib::sna::SnapshotVersion::V2)
                        .expect("Unable to write the sna in the temporary file");

                    let xfer = CpcXfer::new(sub_m4.value_of("CPCM4").unwrap());

                    let tmp_file_name = f.path().to_str().unwrap();
                    xfer.upload_and_run(tmp_file_name, None)
                        .expect("An error occured while transfering the snapshot");
                }
            }
        }
    }

    Ok(())
}

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() -> anyhow::Result<()> {
    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );

    let args = Command::new("CPC image conversion tool")
                    .version("0.1.2")
                    .author("Krusty/Benediction")
                    .about("Simple CPC image conversion tool")
                    .before_help(&desc_before[..])

                    .subcommand(
                        Command::new("sna")
                            .about("Generate a snapshot with the converted image.")
                            .arg(
                                Arg::new("SNA")
                                    .takes_value(true)
                                    .help("snapshot filename to generate")
                                    .required(true)
                                    .validator(|sna| {
                                        if sna.to_lowercase().ends_with("sna") {
                                            Ok(())
                                        }
                                        else {
                                            Err(format!("{} has not a snapshot extension.", sna))
                                        }
                                    })
                            )
                    )

                    .subcommand(
                        Command::new("dsk")
                        .about("Generate a DSK with an executable of the converted image.")
                        .arg(
                            Arg::new("DSK")
                            .takes_value(true)
                            .help("dsk filename to generate")
                            .required(true)
                            .validator(|dsk|{
                                if dsk.to_lowercase().ends_with("dsk") {
                                    Ok(())
                                }
                                else {
                                    Err(format!("{} has not a dsk extention.", dsk))
                                }
                            })
                        )
                    )

                    .subcommand(
                        export_palette!(Command::new("scr")
                        .about("Generate an OCP SCR file")
                        .arg(
                            Arg::new("SCR")
                                .takes_value(true)
                                .help("SCR file to generate")
                                .required(true)
                        )
                        .arg(
                            Arg::new("COMPRESSED")
                                .help("Request a compressed screen")
                                .long("compress")
                                .short('c')
                                .required(false)
                        )
                    ))

                    .subcommand(
                        Command::new("exec")
                        .about("Generate a binary file to manually copy in a DSK or M4 folder.")
                        .arg(
                            Arg::new("FILENAME")
                            .takes_value(true)
                            .help("executable to generate")
                            .required(true)
                            .validator(|fname|{
                                let fname = std::path::Path::new(&fname);
                                if let Some(ext) = fname.extension() {
                                    let ext = ext.to_os_string().into_string().unwrap();
                                    if ext.len() > 3 {
                                        return Err(format!("{} is not a valid amsdos extension.", ext));
                                    }
                                }

                                if let Some(stem) = fname.file_stem() {
                                    let stem = stem.to_os_string().into_string().unwrap();
                                    if stem.len() > 8 {
                                        return Err(format!("{} is not a valid amsdos file stem.", stem))
                                    }
                                }

                                Ok(())
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
                            .takes_value(true)
                            .required(false)
                            .help("Name of the assembly file that contains the size of the sprite")
                        )
                        .arg(
                            Arg::new("FORMAT")
                            .long("format")
                            .short('f')
                            .required(true)
                            .default_value("linear")
                            .possible_values(&["linear", "graycoded", "zigzag+graycoded"])
                        )
                        .arg(
                            Arg::new("SPRITE_FNAME")
                            .takes_value(true)
                            .help("Filename to generate")
                            .required(true)
                        )
                    ))

                    .subcommand(
                        Command::new("tile")
                            .about("Generate a list of sprites")
                            .arg(
                                Arg::new("EXPORT_PALETTE")
                                .long("palette")
                                .short('p')
                                .takes_value(true)
                                .required(false)
                                .help("Name of the binary file that contains the palette")
                            )
                            .arg(
                                Arg::new("WIDTH")
                                .long("width")
                                .short('w')
                                .takes_value(true)
                                .required(true)
                                .help("Width (in bytes) of a tile")
                            )
                            .arg(
                                Arg::new("HEIGHT")
                                .long("height")
                                .short('h')
                                .takes_value(true)
                                .required(true)
                                .help("Height (in lines) of a tile")
                            )
                            .arg(
                                Arg::new("HORIZ_COUNT")
                                .long("horiz_count")
                                .takes_value(true)
                                .required(false)
                                .help("Horizontal number of tiles to extract. Extra tiles are ignored")
                            )
                            .arg(
                                Arg::new("VERT_COUNT")
                                .long("vert_count")
                                .takes_value(true)
                                .required(false)
                                .help("Vertical number of tiles to extract. Extra tiles are ignored")
                            )
                            .arg(
                                Arg::new("CONFIGURATION")
                                .long("configuration")
                                .short('c')
                                .takes_value(true)
                                .required(false)
                                .help("Name of the assembly file that contains the size of the sprite")
                            )
                            .arg(
                                Arg::new("FORMAT")
                                .long("format")
                                .short('f')
                                .required(true)
                                .default_value("linear")
                            )
                            .arg(
                                Arg::new("SPRITE_FNAME")
                                .takes_value(true)
                                .help("Filename to generate. Will be postfixed by the number")
                                .required(true)
                            )

                    )

                    .arg(
                        Arg::new("MODE")
                            .short('m')
                            .long("mode")
                            .help("Screen mode of the image to convert.")
                            .value_name("MODE")
                            .default_value("0")
                            .possible_values(&["0", "1", "2"])
                    )
                    .arg(
                        Arg::new("FULLSCREEN")
                            .long("fullscreen")
                            .help("Specify a full screen displayed using 2 non consecutive banks.")
                            .conflicts_with("OVERSCAN")
                    )
                    .arg(
                        Arg::new("OVERSCAN")
                            .long("overscan")
                            .help("Specify an overscan screen (crtc meaning).")
                    )
                    .arg(
                        Arg::new("STANDARD")
                            .long("standard")
                            .help("Specify a standard screen manipulation.")
                            .conflicts_with("OVERSCAN")
                            .conflicts_with("FULLSCREEN")
                    )
                    .arg(
                        Arg::new("SKIP_ODD_PIXELS")
                            .long("skipoddpixels")
                            .short('s')
                            .help("Skip odd pixels when reading the image (usefull when the picture is mode 0 with duplicated pixels")
                    )
                    .arg(Arg::new("PENS")
                            .long("pens")
                            .takes_value(true)
                            .required(false)
                            .help("Separated list of ink number. Use ',' as a separater")
                        )
                        .arg(
                            Arg::new("PEN0")
                            .long("pen0")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 0")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN1")
                            .long("pen1")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 1")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN2")
                            .long("pen2")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 2")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN3")
                            .long("pen3")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 3")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN4")
                            .long("pen4")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 4")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN5")
                            .long("pen5")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 5")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN6")
                            .long("pen6")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 6")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN7")
                            .long("pen7")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 7")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN8")
                            .long("pen8")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 8")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN9")
                            .long("pen9")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 9")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN10")
                            .long("pen10")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 10")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN11")
                            .long("pen11")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 11")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN12")
                            .long("pen12")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 12")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN13")
                            .long("pen13")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 13")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN14")
                            .long("pen14")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 14")
                            .conflicts_with("PENS")
                        )
                        .arg(
                            Arg::new("PEN15")
                            .long("pen15")
                            .takes_value(true)
                            .required(false)
                            .help("Ink number of the pen 15")
                            .conflicts_with("PENS")
                        )
                    .arg(
                        Arg::new("SOURCE")
                            .takes_value(true)
                            .help("Filename to convert")
//                            .last(true)
                            .required(true)
                            .forbid_empty_values(true)
                            .validator(|source| {
                              if Path::new(&source).exists() {
                                  Ok(())
                              }
                              else {
                                  Err(format!("{} does not exists!", source))
                              }
                            })
                   );

    let matches = if cfg!(feature = "xferlib") {
        args.subcommand(
                        Command::new("m4")
                        .about("Directly send the code on the M4 through a snapshot")
                        .arg(
                            Arg::new("CPCM4")
                                .takes_value(true)
                            .help("Address of the M4")
                            .required(true)
                        )
                        .arg(
                            Arg::new("WATCH")
                            .takes_value(false)
                            .help("Monitor the source file modification and restart the conversion and transfer automatically. Picture must ALWAYS be valid.")
                            .long("watch")

                        )
                    )
    }
    else {
        args
    }
    .get_matches();

    if matches.subcommand_matches("m4").is_none()
        && matches.subcommand_matches("dsk").is_none()
        && matches.subcommand_matches("sna").is_none()
        && matches.subcommand_matches("sprite").is_none()
        && matches.subcommand_matches("tile").is_none()
        && matches.subcommand_matches("exec").is_none()
        && matches.subcommand_matches("scr").is_none()
    {
        eprintln!("[ERROR] you have not specified any action to do.");
        std::process::exit(exitcode::USAGE);
    }

    convert(&matches).expect("Unable to make the conversion");

    if let Some(sub_m4) = matches.subcommand_matches("m4") {
        if cfg!(feature = "xferlib") && sub_m4.is_present("WATCH") {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher: RecommendedWatcher =
                RecommendedWatcher::new(move |res| tx.send(res).unwrap())?;
            watcher.watch(
                &std::path::Path::new(matches.value_of("SOURCE").unwrap()),
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
                        if let Err(e) = convert(&matches) {
                            eprintln!("[ERROR] Unable to convert the image {}", e);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

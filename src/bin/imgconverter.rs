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


use clap;
use notify;

use clap::{App, Arg, ArgMatches, SubCommand};
use std::path::Path;
use tempfile::Builder;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

use cpclib::assembler::assembler::visit_tokens_all_passes;
use cpclib::assembler::parser::parse_z80_str;
use cpclib::ga::Palette;

use cpclib::imageconverter::*;
use cpclib::sna::*;
use cpclib::sna;

use std::fs::File;
use std::io::Write;

#[cfg(feature = "xferlib")]
use cpclib::xfer::CpcXfer;

fn standard_linker_code() -> &'static str {
    "   org 0x1000
        di
        ld sp, $
        ld hl, image
        ld de, 0xc000
        call lz4_uncrunch
        ld hl, code
        ld de, 0x4000
        ld bc, code_end - code
        ldir

code
    ; TODO add the code
code_end
image
    ; todo add crunched image
    "
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
            0 => 0x8c,
            1 => 0x8d,
            2 => 0x8e,
            _ => unreachable!(),
        }
    )
}

fn fullscreen_display_code(mode: u8, crtc_width: usize, palette: &Palette) -> String {
    let code_mode = match mode {
        0 => 0x8c,
        1 => 0x8d,
        2 => 0x8e,
        _ => unreachable!(),
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

fn assemble(z80: &str) -> Vec<u8> {
    let tokens = parse_z80_str(&z80).expect("Unable to tokenize the code").1;
    let env = visit_tokens_all_passes(&tokens).unwrap();
    let start_code = 0x4000;
    let end_code = env.output_address();
    let code_size = end_code - start_code;

    env.memory(start_code, code_size)
}

#[allow(clippy::if_same_then_else)] // false positive
fn get_output_format(matches: &ArgMatches<'_>) -> OutputFormat {
    if let Some(_sprite_matches) = matches.subcommand_matches("sprite") {
        // Sprite case. Only Linear encoding is currently managed
        OutputFormat::LinearEncodedSprite
    } else {
        // Standard case
        if matches.is_present("OVERSCAN") {
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::overscan(),
                display_address: DisplayCRTCAddress::new_overscan_from_page(2),
            }
        } else if matches.is_present("FULLSCREEN") {
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::overscan(),
                display_address: DisplayCRTCAddress::new_overscan_from_page(2),
            }
        } else {
            // assume it is a standard screen
            OutputFormat::CPCMemory {
                output_dimension: CPCScreenDimension::standard(),
                display_address: DisplayCRTCAddress::new_standard_from_page(3),
            }
        }
    }
}

// TODO - Add the ability to import a target palette
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
fn convert(matches: &ArgMatches<'_>) -> Result<(), String> {
    let input_file = matches.value_of("SOURCE").unwrap();
    let output_mode = matches.value_of("MODE").unwrap().parse::<u8>().unwrap();
    let mut transformations = TransformationsList::new();

    if matches.is_present("SKIP_ODD_PIXELS") {
        transformations = transformations.skip_odd_pixels();
    }

    let output_format = get_output_format(&matches);
    let conversion = ImageConverter::convert(
        input_file,
        None,
        output_mode.into(),
        transformations,
        &output_format,
    )?;

    println!("Expected {:?}", &output_format);
    println!("Conversion  {:?}", &conversion);

    let sub_sna = matches.subcommand_matches("sna");
    let sub_m4 = matches.subcommand_matches("m4");
    let sub_dsk = matches.subcommand_matches("dsk");
    let sub_sprite = matches.subcommand_matches("sprite");

    if sub_sprite.is_some() {
        let sub_sprite = sub_sprite.unwrap();
        match &conversion {
            Output::LinearEncodedSprite {
                data,
                byte_width,
                height,
                ..
            } => {
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
                        writeln!(&mut file, "{}_WIDTH equ {}", fname, byte_width).unwrap();
                        writeln!(&mut file, "{}_HEIGHT equ {}", fname, height).unwrap();
                        Some(())
                    });
            }
            _ => unreachable!(),
        }
    } else {
        // Make the conversion before feeding sna or dsk
        let (palette, code) = match &conversion {
            Output::CPCMemoryStandard(_memory, pal) => {
                (pal, assemble(&standard_display_code(output_mode)))
            }

            Output::CPCMemoryOverscan(_memory1, _memory2, pal) => {
                let code = assemble(&fullscreen_display_code(output_mode, 96 / 2, &pal));
                (pal, code)
            }

            _ => unreachable!(),
        };

        if sub_dsk.is_some() {
            // TODO create the linker

        }
        if sub_sna.is_some() || sub_m4.is_some() {
            // Create a snapshot with a standard screen
            let mut sna = Snapshot::default();

            match &conversion {
                Output::CPCMemoryStandard(memory, _) => {
                    sna.add_data(&memory.to_vec(), 0xc000)
                        .expect("Unable to add the image in the snapshot");
                }
                Output::CPCMemoryOverscan(memory1, memory2, _) => {
                    sna.add_data(&memory1.to_vec(), 0x8000)
                        .expect("Unable to add the image in the snapshot");
                    sna.add_data(&memory2.to_vec(), 0xc000)
                        .expect("Unable to add the image in the snapshot");
                }
                _ => unreachable!(),
            };

            sna.add_data(&code, 0x4000).unwrap();
            sna.set_value(SnapshotFlag::Z80_PC, 0x4000).unwrap();
            sna.set_value(SnapshotFlag::GA_PAL(Some(16)), 0x54).unwrap();
            for i in 0..16 {
                sna.set_value(
                    SnapshotFlag::GA_PAL(Some(i)),
                    u16::from(palette.get((i as i32).into()).gate_array()),
                )
                .unwrap();
            }

            if let Some(sub_sna) = sub_sna {
                let sna_fname = sub_sna.value_of("SNA").unwrap();
                sna.save(sna_fname, sna::SnapshotVersion::V2)
                    .expect("Unable to save the snapshot");
            } else if let Some(sub_m4) = sub_m4 {
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

fn main() {
    let args = App::new("CPC image conversion tool")
                    .version("0.1")
                    .author("Krusty/Benediction")
                    .about("Simple CPC image conversion tool")

                    .subcommand(
                        SubCommand::with_name("sna")
                            .about("Generate a snapshot with the converted image.")
                            .arg(
                                Arg::with_name("SNA")
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
                        SubCommand::with_name("dsk")
                        .about("Generate a DSK with an executable of the converted image.")
                        .arg(
                            Arg::with_name("DSK")
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
                        SubCommand::with_name("sprite")
                        .about("Generate a sprite file to be included inside an application")
                        .arg(
                            Arg::with_name("EXPORT_PALETTE")
                            .long("palette")
                            .short("p")
                            .takes_value(true)
                            .required(false)
                            .help("Name of the binary file that contains the palette")
                        )
                        .arg(
                            Arg::with_name("CONFIGURATION")
                            .long("configuration")
                            .short("c")
                            .takes_value(true)
                            .required(false)
                            .help("Name of the assembly file that contains the size of the sprite")
                        )
                        .arg(
                            Arg::with_name("FORMAT")
                            .long("format")
                            .short("f")
                            .required(true)
                            .default_value("linear")
                        )
                        .arg(
                            Arg::with_name("SPRITE_FNAME")
                            .takes_value(true)
                            .help("Filename to generate")
                            .required(true)
                        )
                    )

                    .arg(
                        Arg::with_name("MODE")
                            .short("m")
                            .long("mode")
                            .help("Screen mode of the image to convert.")
                            .value_name("MODE")
                            .default_value("0")
                            .possible_values(&["0", "1", "2"])
                    )
                    .arg(
                        Arg::with_name("FULLSCREEN")
                            .long("fullscreen")
                            .help("Specify a full screen displayed using 2 non consecutive banks.")
                            .conflicts_with("OVERSCAN")
                    )
                    .arg(
                        Arg::with_name("OVERSCAN")
                            .long("overscan")
                            .help("Specify an overscan screen (crtc meaning).")
                    )
                    .arg(
                        Arg::with_name("STANDARD")
                            .long("standard")
                            .help("Specify a standard screen manipulation.")
                            .conflicts_with("OVERSCAN")
                            .conflicts_with("FULLSCREEN")
                    )
                    .arg(
                        Arg::with_name("SKIP_ODD_PIXELS")
                            .long("skipoddpixels")
                            .short("s")
                            .help("Skip odd pixels when reading the image (usefull when the picture is mode 0 with duplicated pixels")
                    )
                    .arg(
                        Arg::with_name("SOURCE")
                            .takes_value(true)
                            .help("Filename to convert")
//                            .last(true)
                            .required(true)
                            .empty_values(false)
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
                        SubCommand::with_name("m4")
                        .about("Directly send the code on the M4 through a snapshot")
                        .arg(
                            Arg::with_name("CPCM4")
                            .takes_value(true)
                            .help("Address of the M4")
                            .required(true)
                        )
                        .arg(
                            Arg::with_name("WATCH")
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
    {
        eprintln!("[ERROR] you have not specified any action to do.");
        std::process::exit(exitcode::USAGE);
    }

    convert(&matches).expect("Unable to make the conversion");

    if let Some(sub_m4) = matches.subcommand_matches("m4") {
        if cfg!(feature = "xferlib") && sub_m4.is_present("WATCH") {
            println!("Watching for file modification...");
            // Create a channel to receive the events.
            let (tx, rx) = channel();

            // Automatically select the best implementation for your platform.
            // You can also access each implementation directly e.g. INotifyWatcher.
            let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2)).unwrap();

            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            watcher
                .watch(
                    matches.value_of("SOURCE").unwrap(),
                    RecursiveMode::NonRecursive,
                )
                .expect("Unable to watch the file");

            // This is a simple loop, but you may want to use more complex logic here,
            // for example to handle I/O.
            loop {
                match rx.recv() {
                    Ok(event) =>  {
                        if let DebouncedEvent::Write(_) = event {
                            println!("Image modified. Launch new conversion");

                            if let Err(e) = convert(&matches) {
                                    eprintln!("[ERROR] Unable to convert the image {}", e);
                            }
                        }
                    },
                    Err(e) => println!("watch error: {:?}", e),
                }
            }
        }
    }
}

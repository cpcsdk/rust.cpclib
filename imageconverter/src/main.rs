extern crate clap;
extern crate cpc;

use clap::{App, Arg, SubCommand, ArgMatches};
use std::path::Path;

use cpc::imageconverter::*;
use cpc::image::*;
use cpc::sna::*;
use cpc::ga::{Pen, Palette};
use cpc::assembler::parser::parse_z80_str;
use cpc::assembler::assembler::visit_tokens;



// Produce the code that display a standard screen
fn standard_display_code(mode: u8) -> String {
    let code = match mode {
        0 => 0x8c,
        1 => 0x8d,
        2 => 0x8e,
        _ => unreachable!()
    };
    format!("
        org 0x4000
        di
        ld bc, 0x7f00 + 0x{:x}
        out (c), c
        jp $
    ", code)
}


fn fullscreen_display_code(mode: u8, crtc_width: usize) -> String {
    let code = match mode {
        0 => 0x8c,
        1 => 0x8d,
        2 => 0x8e,
        _ => unreachable!()
    };

    format!("
        org 0x4000

        di
        ld hl, 0xc9fb
        ld (0x38), hl
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
        ld bc, 0xbd00 + 0x20
        out (c), c


frame_loop
        ld b, 0xf5
vsync_loop
        in a, (c)
        rra
        jr nc, vsync_loop




        jp frame_loop
    ", code, crtc_width)
}

fn overscan_display_code(mode: u8, crtc_width: usize) -> String {
    fullscreen_display_code(mode, crtc_width)
}

fn assemble(z80: String) -> Vec<u8> {
    let tokens = parse_z80_str(&z80).expect("Unable to tokenize the code").1;
    let env = visit_tokens(&tokens).unwrap();
    let start_code = 0x4000;
    let end_code = env.output_address();
    let code_size = end_code - start_code;
    let mem = env.memory(start_code, code_size);

    mem

}


fn get_output_format(matches: &ArgMatches) -> OutputFormat {
    if matches.is_present("OVERSCAN") {
        OutputFormat::CPCMemory{
            outputDimension: CPCScreenDimension::overscan(),
            displayAddress: DisplayCRTCAddress::new_overscan_from_page(2)
        }
    }
    else if matches.is_present("FULLSCREEN") {
        OutputFormat::CPCMemory{
            outputDimension: CPCScreenDimension::overscan(),
            displayAddress: DisplayCRTCAddress::new_overscan_from_page(2)
        }
    }
    else {
        // assume it is a standard screen
        OutputFormat::CPCMemory{
            outputDimension: CPCScreenDimension::standard(),
            displayAddress: DisplayCRTCAddress::new_standard_from_page(3)
        }
    }
}

fn main() {

    let matches = App::new("CPC image conversion tool")
                    .version("0.1")
                    .author("Krusty/Benediction")
                    .about("Simple CPC image conversion tool")
                    .subcommand(
                        SubCommand::with_name("sna")
                            .about("Generate a snapshot with the converted image.")
                            .arg(
                                Arg::with_name("SNA")
                                    .takes_value(true)
                                    .help("Filename to generate")
                                    .required(true)
                                    .validator(|sna| {
                                        match sna.to_lowercase().ends_with("sna") {
                                            true => Ok(()),
                                            false => Err(format!("{} has not a snapshot extension.", sna))
                                        }
                                    })  
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
                              match  Path::new(&source).exists() {
                                  true => Ok(()),
                                  false => Err(format!("{} does not exists!", source))
                              }
                            })
                   ).get_matches();

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
        &output_format);

    println!("Expected {:?}", & output_format);
    println!("Conversion  {:?}", &conversion);

    if let Some(sub_sna) = matches.subcommand_matches("sna") {
        // Create a snapshot with a standard screen
        let sna_fname = sub_sna.value_of("SNA").unwrap();
        let mut sna = Snapshot::default();
        let mut palette: Option<Palette> = None;
        let mut code = None;

        match conversion {
            Output::CPCMemoryStandard(memory, pal) => {
                palette = Some(pal);
                code = Some(assemble(standard_display_code(output_mode)));
                sna.add_data(memory.to_vec(), 0xc000)
                    .expect("Unable to add the image in the snapshot");
            },

            Output::CPCMemoryOverscan(memory1, memory2, pal) => {
                palette = Some(pal);
                code = Some(assemble(fullscreen_display_code(output_mode, 96/2)));
                sna.add_data(memory1.to_vec(), 0x8000)
                    .expect("Unable to add the image in the snapshot");
                sna.add_data(memory2.to_vec(), 0xc000)
                    .expect("Unable to add the image in the snapshot");
            }

            _ => unreachable!()
        };


        sna.add_data(code.unwrap(), 0x4000);
        sna.set_value(SnapshotFlag::Z80_PC, 0x4000).unwrap();
        sna.set_value(SnapshotFlag::GA_PAL(Some(16)), 0x54).unwrap();
        for i in 0..16 {
            sna.set_value(
                SnapshotFlag::GA_PAL(Some(i)),
                palette.as_ref().unwrap().get((i as i32).into()).gate_array() as u16
            ).unwrap();
        }

        sna.save_sna(sna_fname).expect("Unable to save the snapshot");

    }
}
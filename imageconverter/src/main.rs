extern crate clap;
extern crate cpc;

use clap::{App, Arg, SubCommand};
use std::path::Path;

use cpc::imageconverter::*;
use cpc::image::*;
use cpc::sna::*;
use cpc::ga::Pen;
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

fn assemble(z80: String) -> Vec<u8> {
    let tokens = parse_z80_str(&z80).expect("Unable to tokenize the code").1;
    eprintln!("{:?}", &tokens);
    let env = visit_tokens(&tokens).unwrap();
    let start_code = 0x4000;
    let end_code = env.output_address();
    let code_size = end_code - start_code;
    let mem = env.memory(start_code, code_size);

    mem

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
                        Arg::with_name("OVERSCAN")
                            .long("overscan")
                            .help("Specify an overscan screen manipulation.")
                    )
                    .arg(
                        Arg::with_name("STANDARD")
                            .long("standard")
                            .help("Specify a standard screen manipulation.")
                            .conflicts_with("OVERSCAN")
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

    if let Some(sub_sna) = matches.subcommand_matches("sna") {
        assert!(!matches.is_present("OVERSCAN")); // Need to implement

        // Create a snapshot with a standard screen
        let sna_fname = sub_sna.value_of("SNA").unwrap();
        let output_format = OutputFormat::CPCMemory{
            outputDimension: CPCScreenDimension::standard(),
            displayAddress: DisplayCRTCAddress::new(0x3000)
        };
        let conversion = ImageConverter::convert(
            input_file, 
            None, 
            output_mode.into(), 
            transformations,
            &output_format);
        match conversion {
            Output::CPCMemoryStandard(memory, palette) => {
                let mut sna = Snapshot::default();
                sna.add_data(memory.to_vec(), 0xc000).expect("Unable to add the image in the snapshot");
                let code = assemble(standard_display_code(output_mode));
                sna.add_data(code, 0x4000).expect("Unable to add the code in the snapshot");

                sna.set_value(SnapshotFlag::Z80_PC, 0x4000);
                sna.set_value(SnapshotFlag::GA_PAL(Some(16)), 0x54).unwrap();
                for i in 0..16 {
                   sna.set_value(
                       SnapshotFlag::GA_PAL(Some(i)),
                       palette.get((i as i32).into()).gate_array() as u16
                   ).unwrap();
                }

                sna.save_sna(sna_fname).expect("Unable to save the snapshot");

            }
            _ => unreachable!()
        };

    }
}

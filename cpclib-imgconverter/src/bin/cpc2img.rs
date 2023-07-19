use cpclib::common::clap::{self, value_parser, Arg, ArgAction, Command};
use cpclib::common::itertools::Itertools;
use cpclib::image::image::ColorMatrix;
use cpclib::image::pixels;
use cpclib_imgconverter::{self, get_requested_palette};

fn main() {
    let cmd = cpclib_imgconverter::specify_palette!(clap::Command::new("cpc2png")
        .about("Generate PNG from CPC files")
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
            Arg::new("MODE0RATIO")
                .long("mode0ratio")
                .help("Horizontally double the pixels")
                .action(ArgAction::SetTrue)
        )
        .subcommand(
            Command::new("SPRITE")
                .about("Load from a linear sprite data")
                .name("sprite")
                .arg(
                    Arg::new("WIDTH")
                        .long("width")
                        .required(true)
                        .help("Width of the sprite in pixels")
                )
        )
        .arg(Arg::new("INPUT").required(true))
        .arg(Arg::new("OUTPUT").required(true)));

    let matches = cmd.get_matches();
    let palette = dbg!(get_requested_palette(&matches).unwrap_or_default());
    let input_fname = matches.get_one::<String>("INPUT").unwrap();
    let output_fname = matches.get_one::<String>("OUTPUT").unwrap();
    let mode = matches.get_one::<String>("MODE").unwrap().parse().unwrap();

    let mode0ratio = matches.contains_id("MODE0RATIO");
    // read the data file
    let data = std::fs::read(input_fname).expect("Unable to read input file");

    // remove header if any
    let data = if cpclib::disc::amsdos::AmsdosHeader::from_buffer(&data).is_checksum_valid() {
        &data[128..]
    }
    else {
        &data
    };

    let mut matrix: ColorMatrix = if let Some(sprite) = matches.subcommand_matches("SPRITE") {
        let width: usize = sprite.get_one::<String>("WIDTH").unwrap().parse().unwrap();
        let width = match mode {
            0 => width / 2,
            1 => width / 4,
            2 => width / 8,
            _ => unreachable!()
        };

        // convert it
        data.chunks_exact(width)
            .map(|line| {
                // build lines of pen
                let line = line.iter();
                match mode {
                    0 => {
                        line.flat_map(|b| pixels::mode0::byte_to_pens(*b).into_iter())
                            .collect_vec()
                    }
                    1 => {
                        line.flat_map(|b| pixels::mode1::byte_to_pens(*b).into_iter())
                            .collect_vec()
                    }
                    2 => {
                        line.flat_map(|b| pixels::mode2::byte_to_pens(*b))
                            .collect_vec()
                    }
                    _ => unreachable!()
                }
            })
            .map(move |pens| {
                // build lines of inks
                pens.iter()
                    .map(|pen| palette.get(pen))
                    .cloned()
                    .collect_vec()
            })
            .collect_vec()
            .into()
    }
    else {
        unimplemented!()
    };

    if mode0ratio {
        matrix.double_horizontally();
    }
    // save the generated file
    let img = matrix.as_image();
    img.save(output_fname).expect("Error while saving the file");
}

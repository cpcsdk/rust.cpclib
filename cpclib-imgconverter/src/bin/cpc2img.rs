use cpclib::common::clap::{self, value_parser, Arg, ArgAction, Command};
use cpclib::image::image::{ColorMatrix, Mode};
use cpclib_imgconverter::{self, get_requested_palette};

fn main() {
    let cmd = cpclib_imgconverter::specify_palette!(clap::Command::new("cpc2png")
        .about("Generate PNG from CPC files")
        .subcommand_required(true)
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
        .arg(Arg::new("INPUT").required(true))
        .arg(Arg::new("OUTPUT").required(true)));

    let matches = cmd.get_matches();
    let palette = get_requested_palette(&matches).unwrap_or_default();
    let input_fname = matches.get_one::<String>("INPUT").unwrap();
    let output_fname = matches.get_one::<String>("OUTPUT").unwrap();
    let mode = *matches.get_one::<i64>("MODE").unwrap() as u8;
    let mode = Mode::from(mode);

    let mode0ratio = matches.get_flag("MODE0RATIO");
    // read the data file
    let data = std::fs::read(input_fname).expect("Unable to read input file");

    // remove header if any
    let data = if cpclib::disc::amsdos::AmsdosHeader::from_buffer(&data).is_checksum_valid() {
        &data[128..]
    }
    else {
        &data
    };

    let mut matrix: ColorMatrix = if let Some(sprite) = matches.subcommand_matches("sprite") {
        let width: usize = sprite.get_one::<String>("WIDTH").unwrap().parse().unwrap();
        ColorMatrix::from_sprite(data, width as _, mode, &palette)
    }
    else if let Some(screen) = matches.subcommand_matches("screen") {
        let width: usize = screen.get_one::<String>("WIDTH").unwrap().parse().unwrap();
        ColorMatrix::from_screen(data, width as _, mode, &palette)
    }
    else {
        unreachable!()
    };

    if mode0ratio {
        matrix.double_horizontally();
    }
    // save the generated file
    let img = matrix.as_image();
    img.save(output_fname).expect("Error while saving the file");
}

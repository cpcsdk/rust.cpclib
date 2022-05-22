use cpclib::common::clap::ArgMatches;
use cpclib::Palette;

#[macro_export]
macro_rules! specify_palette {
    ($e: expr) => {
        $e.arg(
            Arg::new("PENS")
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
    };
}

pub fn get_requested_palette(matches: &ArgMatches) -> Option<Palette> {
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
        let mut palette = Palette::empty();
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

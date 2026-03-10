use cpclib_bdasm::{build_args_parser, process};

fn main() {
    let matches = build_args_parser().get_matches();
    if let Err(e) = process(&matches) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

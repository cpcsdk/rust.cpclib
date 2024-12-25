use cpclib_bdasm::{build_args_parser, process};

fn main() {
    let matches = build_args_parser()
					.get_matches();
    process(&matches);
}

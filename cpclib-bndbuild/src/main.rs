use cpclib_bndbuild::runners::RunnerWithClap;
use cpclib_bndbuild::{build_args_parser, process_matches, BndBuilderError};

fn main() {
    match inner_main() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failure\n{}", e);
            std::process::exit(-1);
        }
    }
}

fn inner_main() -> Result<(), BndBuilderError> {
    let cmd = build_args_parser();
    let matches = cmd.clone().get_matches();
    process_matches(cmd, &matches)
}

use std::process::exit;

use cpclib_bndbuild::{app::BndBuilderApp, build_args_parser, process_matches, BndBuilderError};

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
    let command = match BndBuilderApp::new()
        .map_err(|e| {
            BndBuilderError::AnyError(e.to_string())
        })?
        {
        Some(builder) => {
            builder.command()?
        },
        None => {
            exit(0)
        },
    };

    command.execute_one_step()
}

use std::process::exit;

use cpclib_bndbuild::BndBuilderError;
use cpclib_bndbuild::app::BndBuilderApp;
use cpclib_bndbuild::event::BndBuilderObserverRc;

fn main() {
    match inner_main() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failure\n{e}");
            std::process::exit(-1);
        }
    }
}

fn inner_main() -> Result<(), BndBuilderError> {
    let observer = BndBuilderObserverRc::new_default();

    let command =
        match BndBuilderApp::new().map_err(|e| BndBuilderError::AnyError(e.to_string()))? {
            Some(mut app) => {
                app.add_observer(observer);
                app.command()?
            },
            None => exit(0)
        };

    command.execute()
}

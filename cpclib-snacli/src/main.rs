use cpclib_common::clap::error::ErrorKind;
use cpclib_common::event::EventObserver;

fn main() {
    let o = &();
    o.emit_stderr(
        "[WARNING] This is still a draft version that implement still few functionnalities"
    );

    let parser = cpclib_sna::build_arg_parser();
    let matches = match parser.try_get_matches() {
        Ok(m) => m,
        Err(e) => {
            match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    o.emit_stdout(&e.to_string());
                    return;
                },
                _ => {
                    o.emit_stderr(&e.to_string());
                    std::process::exit(1);
                }
            }
        },
    };
    cpclib_sna::process(&matches, o).unwrap();
}

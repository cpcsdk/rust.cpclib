use cpclib_basmdoc::cmdline;

fn main() {
    let parser = cmdline::build_args_parser();
    let matches = parser.clone().get_matches();
    if let Err(e) = cmdline::handle_matches(&matches, &parser) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

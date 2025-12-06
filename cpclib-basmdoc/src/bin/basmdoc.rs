use cpclib_basmdoc::cmdline;

fn main() {
    let parser = cmdline::build_args_parser();
    let matches = parser.get_matches();
    cmdline::handle_matches(&matches);

}

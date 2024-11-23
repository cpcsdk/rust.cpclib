fn main() {
    eprintln!("[WARNING] This is still a draft version that implement still few functionnalities");

    let parser = cpclib_sna::build_arg_parser();
    let matches = parser.get_matches();
    cpclib_sna::process(&matches, &()).unwrap();
}

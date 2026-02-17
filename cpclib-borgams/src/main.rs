fn main() {
    let cli = cpclib_borgams::cli::build_cli();
    let args = cli.get_matches();

    dbg!("Args: {:?}", args);
}

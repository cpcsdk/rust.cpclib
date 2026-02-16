use cpclib_locomotive::{Cli, Parser, handle_locomotive_arguments};

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    handle_locomotive_arguments(cli)
}

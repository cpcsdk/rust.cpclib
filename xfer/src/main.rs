extern crate cpc;
extern crate clap;

use std::path::Path;

fn main() {
    let matches = clap::App::new("CPC xfer to M4")
        .author("Krusty/Benediction")
        .about("RUST version of the communication tool between a PC and a CPC through the CPC Wifi card")
        .arg(
            clap::Arg::with_name("CPCADDR")
            .help("Specify the address of the M4.")
            .required(true) // Make it optional later
        )
        .subcommand(
            clap::SubCommand::with_name("-r")
            .about("Reboot M4.")
        )
        .subcommand(
            clap::SubCommand::with_name("-s")
            .about("Reboot CPC.")
        )
        .subcommand(
            clap::SubCommand::with_name("-y")
            .about("Upload a file on the M4 in the /tmp folder and launch it.")
            .arg(
                clap::Arg::with_name("fname")
                .help("Filename to send and execute. Can be an executable (Amsdos header expected) or a snapshot V2")
                .validator(|fname| {
                    if Path::new(&fname).exists() {
                        Ok(())
                    }
                    else {
                        Err(format!("{} does not exists", fname))
                    }
                })
                .required(true)
            )
        )
        .subcommand(
            clap::SubCommand::with_name("--ls")
            .about("Display contents of the M4")
        )
        .subcommand(
            clap::SubCommand::with_name("--pwd")
            .about("Display the current working directory selected on the M4")
        )
        .get_matches();


    // TODO manage the retreival of env var
    let hostname = matches.value_of("CPCADDR").unwrap();
    let mut xfer = cpc::xfer::CpcXfer::new(hostname);

    if matches.is_present("-r") {
        xfer.reset_m4();
    }
    else if matches.is_present("-s") {
        xfer.reset_cpc();
    }
    else if let Some(y_opt) =  matches.subcommand_matches("-y")  {
        let fname = y_opt.value_of("fname").unwrap();
        xfer.upload_and_run(fname, None);
    }
    else if let Some(ls_opt) = matches.subcommand_matches("--ls") {
        let content = xfer.current_folder_content();
        for file in content.files() {
            println!("{:?}", file);
        }
    }
    else if let Some(pwd_opt) = matches.subcommand_matches("--pwd") {
        let cwd = xfer.current_working_directory();
        println!("{}", cwd);
    }
}

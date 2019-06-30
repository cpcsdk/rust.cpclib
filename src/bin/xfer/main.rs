extern crate clap;
#[macro_use]
extern crate nom;

use std::path::Path;

mod interact;
mod parser;

use cpclib as cpc;

fn main() -> Result<(), cpc::xfer::XferError> {
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
            clap::SubCommand::with_name("-x")
            .about("Execute a file on the cpc (executable or snapshot)")
            .arg(
                clap::Arg::with_name("fname")
                .help("Filename to execute on the CPC")
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
        .subcommand(
            clap::SubCommand::with_name("--cd")
            .about("Change of current directory in the M4.")
            .arg(
                clap::Arg::with_name("directory")
                .help("Directory to move on. Must exists")
                .required(true)
            )
        )
        .subcommand(
            clap::SubCommand::with_name("--interactive")
            .about("Start an interactive session")
        )
        .get_matches();

    // TODO manage the retreival of env var
    let hostname = matches.value_of("CPCADDR").unwrap();
    let xfer = cpc::xfer::CpcXfer::new(hostname);

    if matches.is_present("-r") {
        xfer.reset_m4();
    } else if matches.is_present("-s") {
        xfer.reset_cpc();
    } else if let Some(y_opt) = matches.subcommand_matches("-y") {
        let fname = y_opt.value_of("fname").unwrap();
        let mut done = false;
        // Snapshot needs to be converted in V2 format and handled differently
        if let Some(extension) = std::path::Path::new(fname).extension() {
            let extension = extension.to_str().unwrap().to_ascii_lowercase();
            if extension == "sna" {
                xfer.upload_and_run_sna(&crate::cpc::sna::Snapshot::load(fname))
                    .expect("Unable to launch SNA");
                done = true;
            }
        }
        if !done {
            xfer.upload_and_run(fname, None)
                .expect("Unable to launch file")
        };
    } else if let Some(x_opt) = matches.subcommand_matches("-x") {
        let fname = x_opt.value_of("fname").unwrap();
        xfer.run(fname); /*.expect("Unable to launch file on CPC.");*/
    } else if let Some(_ls_opt) = matches.subcommand_matches("--ls") {
        let content = xfer.current_folder_content()?;
        for file in content.files() {
            println!("{:?}", file);
        }
    } else if let Some(_pwd_opt) = matches.subcommand_matches("--pwd") {
        let cwd = xfer.current_working_directory()?;
        println!("{}", cwd);
    } else if let Some(cd_opt) = matches.subcommand_matches("--cd") {
        let _cwd = xfer
            .cd(cd_opt.value_of("directory").unwrap())
            .expect("Unable to move in the requested folder.");
    } else if let Some(_interactive_opt) = matches.subcommand_matches("--interactive") {
        interact::start(xfer);
    }

    Ok(())
}

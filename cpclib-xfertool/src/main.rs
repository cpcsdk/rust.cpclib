#![deny(
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    nonstandard_style,
    rust_2018_idioms,
    unused,
    warnings
)]
#![deny(clippy::pedantic)]
#![allow(unused)]


pub mod interact;
pub mod parser;


use std::env;
use clap;
use std::path::Path;
use anyhow;



use hotwatch::{Event, Hotwatch};

use crossbeam_channel::unbounded;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::time::Duration;

use cpclib_sna as sna;
use cpclib_disc as disc;
use cpclib_xfer as xfer;


/// Send and run the file on the CPC.
/// Snapshot V3 are downgraded to the V2 version
fn send_and_run_file(xfer: &xfer::CpcXfer, fname: &str) {
    let mut done = false;
    // Snapshot needs to be converted in V2 format and handled differently
    if let Some(extension) = std::path::Path::new(fname).extension() {
        let extension = extension.to_str().unwrap().to_ascii_lowercase();
        if extension == "sna" {
            let sna = sna::Snapshot::load(fname);
            if sna.version_header() == 3 {
                eprintln!("Need to downgrade SNA version. TODO check if it is sill necessary (I thinl not)");
                let sna_fname = std::path::Path::new(fname)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                sna.save(sna_fname, sna::SnapshotVersion::V2)
                    .unwrap();
                xfer.upload_and_run(sna_fname, None)
                    .expect("Unable to launch SNA");
                done = true;
            }
        }
    }
    if !done {
        xfer.upload_and_run(fname, None)
            .expect("Unable to launch file")
    };
}

fn main() -> anyhow::Result<()> {
    let matches = clap::App::new("CPC xfer to M4")
        .author("Krusty/Benediction")
        .about("RUST version of the communication tool between a PC and a CPC through the CPC Wifi card")
        .arg(
            clap::Arg::with_name("CPCADDR")
            .help("Specify the address of the M4. This argument is optional. If not set up, the content of the environment variable CPCIP is used.")
            .required(false) 
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
            .about("Upload a file on the M4 in the /tmp folder and launch it. V3 snapshots are automatically downgraded to V2 version")
            .arg(
                clap::Arg::with_name("WATCH")
                    .help("Watch the file and resend it on the M4 if modified (so xfer does not end when started with this option).")
                    .short("w")
                    .long("watch")
                    .multiple(false)
                    .takes_value(false)
            )
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
                .last(true)
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

    // Retreivethe hostname from the args or from the environment
    let hostname: String = 
    match matches.value_of("CPCADDR") {
        Some(cpcaddr) => cpcaddr.to_string(),
        None => { 
            match env::var("CPCIP") {
                Ok(cpcaddr) => cpcaddr.to_string(),
                Err(_) => panic!("You should provide the CPCADDR argument or set the CPCIP environmeent variable")
            }

        }
    };
    
    let xfer = xfer::CpcXfer::new(hostname);

    if matches.is_present("-r") {
        xfer.reset_m4()?;
    } else if matches.is_present("-s") {
        xfer.reset_cpc()?;
    } else if let Some(y_opt) = matches.subcommand_matches("-y") {
        let fname: String = y_opt.value_of("fname").unwrap().to_string();

        // Simple file sending
        send_and_run_file(&xfer, &fname);

        if y_opt.is_present("WATCH") {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| tx.send(res).unwrap())?;
            watcher.watch(&fname, RecursiveMode::NonRecursive)?;

            for res in rx {
                match res {
                   Ok(notify::event::Event{kind: notify::event::EventKind::Modify(_), ..}) |
                   Ok(notify::event::Event{kind: notify::event::EventKind::Create(_), ..})
                    => {
                       send_and_run_file(&xfer, &fname);
                   },
                   _ => {}
                }
            }

        }

    } else if let Some(x_opt) = matches.subcommand_matches("-x") {
        let fname = x_opt.value_of("fname").unwrap();
        xfer.run(fname)?; /*.expect("Unable to launch file on CPC.");*/
    } else if let Some(_ls_opt) = matches.subcommand_matches("--ls") {
        let content = xfer.current_folder_content()?;
        for file in content.files() {
            println!("{:?}", file);
        }
    } else if let Some(_pwd_opt) = matches.subcommand_matches("--pwd") {
        let cwd = xfer.current_working_directory()?;
        println!("{}", cwd);
    } else if let Some(cd_opt) = matches.subcommand_matches("--cd") {
        xfer.cd(cd_opt.value_of("directory").unwrap())
            .expect("Unable to move in the requested folder.");
    } else if let Some(_interactive_opt) = matches.subcommand_matches("--interactive") {
        println!("Benediction welcomes you to the interactive mode for M4.");
        interact::XferInteractor::start(&xfer);
    }

    Ok(())
}
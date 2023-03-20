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
use std::path::Path;
use std::time::Duration;

use cpclib_common::clap::{self, Command,ArgAction};
use crossbeam_channel::unbounded;
use hotwatch::{Event, Hotwatch};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use {anyhow, cpclib_disc as disc, cpclib_sna as sna, cpclib_xfer as xfer};

/// Send and run the file on the CPC.
/// Snapshot V3 are downgraded to the V2 version
fn send_and_run_file(xfer: &xfer::CpcXfer, fname: &str, run: bool) {
    let mut done = false;
    // Snapshot needs to be converted in V2 format and handled differently
    if let Some(extension) = std::path::Path::new(fname).extension() {
        let extension = extension.to_str().unwrap().to_ascii_lowercase();
        if extension == "sna" {
            let sna = sna::Snapshot::load(fname).expect("Error while loading snapshot");
            if sna.version_header() == 3 {
                eprintln!("Need to downgrade SNA version. TODO check if it is sill necessary (I thinl not)");
                let sna_fname = std::path::Path::new(fname)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                sna.save(sna_fname, sna::SnapshotVersion::V2).unwrap();
                xfer.upload_and_run(sna_fname, None)
                    .expect("Unable to launch SNA");
                done = true;
            }
        }
    }
    if !done {
        if run {
            xfer.upload_and_run(fname, None)
                .expect("Unable to launch file");
        }
        else {
            xfer.upload(fname, "/", None)
                .expect("Unable to put the file");
        }
    };
}

fn main() -> anyhow::Result<()> {
    let matches = clap::Command::new("CPC xfer to M4")
        .author("Krusty/Benediction")
        .about("RUST version of the communication tool between a PC and a CPC through the CPC Wifi card")
        .arg(
            clap::Arg::new("CPCADDR")
            .help("Specify the address of the M4. This argument is optional. If not set up, the content of the environment variable CPCIP is used.")
            .required(false) 
        )
        .subcommand(
            Command::new("-r")
            .about("Reboot M4.")
        )
        .subcommand(
            Command::new("-s")
            .about("Reboot CPC.")
        )
        .subcommand(
            Command::new("-p")
            .about("Upload the given file in the current folder or the provided one")
            .arg(
                clap::Arg::new("fname")
                .help("Filename to send to the CPC")
                .value_parser(|fname: &str| {
                    if Path::new(fname).exists() {
                        Ok(())
                    }
                    else {
                        Err(format!("{fname} does not exists"))
                    }
                })
                .required(true)
            )/* To implement when needed
            .arg(
                clap::with_name("destination")
                .help("Destination folder.")
                .required(false)
            )*/
        )
        .subcommand(
            Command::new("-y")
            .about("Upload a file on the M4 in the /tmp folder and launch it. V3 snapshots are automatically downgraded to V2 version")
            .arg(
                clap::Arg::new("WATCH")
                    .help("Watch the file and resend it on the M4 if modified (so xfer does not end when started with this option).")
                    .short('w')
                    .long("watch")
                    .action(ArgAction::SetTrue)
            )
            .arg(
                clap::Arg::new("fname")
                .help("Filename to send and execute. Can be an executable (Amsdos header expected) or a snapshot V2")
                .value_parser(|fname: &str| {
                    if Path::new(fname).exists() {
                        Ok(())
                    }
                    else {
                        Err(format!("{fname} does not exists"))
                    }
                })
                .required(true)
                .last(true)
            )
        )
        .subcommand(
            Command::new("-x")
            .about("Execute a file on the cpc (executable or snapshot)")
            .arg(
                clap::Arg::new("fname")
                .help("Filename to execute on the CPC")
            )
        )
        .subcommand(
            Command::new("--ls")
            .about("Display contents of the M4")
        )
        .subcommand(
            Command::new("--pwd")
            .about("Display the current working directory selected on the M4")
        )
        .subcommand(
            Command::new("--cd")
            .about("Change of current directory in the M4.")
            .arg(
                clap::Arg::new("directory")
                .help("Directory to move on. Must exists")
                .required(true)
            )
        )
        .subcommand(
            Command::new("--interactive")
            .about("Start an interactive session")
        )
        .get_matches();

    // Retreivethe hostname from the args or from the environment
    let hostname: String = match matches.get_one::<String>("CPCADDR") {
        Some(cpcaddr) => cpcaddr.to_string(),
        None => {
            match env::var("CPCIP") {
                Ok(cpcaddr) => cpcaddr,
                Err(_) => {
                    panic!(
                "You should provide the CPCADDR argument or set the CPCIP environmeent variable"
            )
                }
            }
        }
    };

    let xfer = xfer::CpcXfer::new(hostname);

    if matches.contains_id("-r") {
        xfer.reset_m4()?;
    }
    else if matches.contains_id("-s") {
        xfer.reset_cpc()?;
    }
    else if let Some(p_opt) = matches.subcommand_matches("-p") {
        let fname: String = p_opt.get_one::<String>("fname").unwrap().to_string();
        send_and_run_file(&xfer, &fname, false);
    }
    else if let Some(y_opt) = matches.subcommand_matches("-y") {
        let fname: String = y_opt.get_one::<String>("fname").unwrap().to_string();

        // Simple file sending
        send_and_run_file(&xfer, &fname, true);

        if y_opt.contains_id("WATCH") {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = RecommendedWatcher::new(
                move |res| tx.send(res).unwrap(),
                notify::Config::default()
            )?;

            watcher.watch(std::path::Path::new(&fname), RecursiveMode::NonRecursive)?;

            for res in rx {
                match res {
                    Ok(notify::event::Event {
kind: notify::event::EventKind::Modify(_) |
    notify::event::EventKind::Create(_), .. }) => {
                        send_and_run_file(&xfer, &fname, true);
                    }
                    _ => {}
                }
            }
        }
    }
    else if let Some(x_opt) = matches.subcommand_matches("-x") {
        let fname = x_opt.get_one::<String>("fname").unwrap();
        xfer.run(fname)?; // .expect("Unable to launch file on CPC.");
    }
    else if let Some(_ls_opt) = matches.subcommand_matches("--ls") {
        let content = xfer.current_folder_content()?;
        for file in content.files() {
            println!("{file:?}");
        }
    }
    else if let Some(_pwd_opt) = matches.subcommand_matches("--pwd") {
        let cwd = xfer.current_working_directory()?;
        println!("{cwd}");
    }
    else if let Some(cd_opt) = matches.subcommand_matches("--cd") {
        xfer.cd(cd_opt.get_one::<String>("directory").unwrap())
            .expect("Unable to move in the requested folder.");
    }
    else if let Some(_interactive_opt) = matches.subcommand_matches("--interactive") {
        println!("Benediction welcomes you to the interactive mode for M4.");
        interact::XferInteractor::start(&xfer);
    }

    Ok(())
}

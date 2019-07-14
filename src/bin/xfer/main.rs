#![deny(
    missing_debug_implementations,
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

use clap;
use std::path::Path;

mod interact;
mod parser;
use hotwatch::{Hotwatch, Event};
use cpclib as cpc;

use crossbeam_channel::unbounded;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::time::Duration;

/// Send and run the file on the CPC.
/// Snapshot V3 are downgraded to the V2 version
fn send_and_run_file(xfer: & cpc::xfer::CpcXfer, fname: &str) {
        let mut done = false;
        // Snapshot needs to be converted in V2 format and handled differently
        if let Some(extension) = std::path::Path::new(fname).extension() {
            let extension = extension.to_str().unwrap().to_ascii_lowercase();
            if extension == "sna" {
                let sna = crate::cpc::sna::Snapshot::load(fname);
                if sna.version_header() == 3 {
                    eprintln!("Need to downgrade SNA version");
                    let sna_fname = std::path::Path::new(fname)
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    sna.save(sna_fname, crate::cpc::sna::SnapshotVersion::V2)
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

    // TODO manage the retreival of env var
    let hostname = matches.value_of("CPCADDR").unwrap();
    let xfer = cpc::xfer::CpcXfer::new(hostname);

    if matches.is_present("-r") {
        xfer.reset_m4()?;
    } else if matches.is_present("-s") {
        xfer.reset_cpc()?;
    } else if let Some(y_opt) = matches.subcommand_matches("-y") {
        let fname = y_opt.value_of("fname").unwrap();

        // Simple file sending
        send_and_run_file(&xfer, fname);

        if y_opt.is_present("WATCH") {
            // Create a channel to receive the events.
            let (tx, rx) = unbounded();

            // Automatically select the best implementation for your platform.
            let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2)).expect("Unable to install watcher");

            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            watcher.watch(fname, RecursiveMode::NonRecursive).expect("Unable to watch file.");

            loop {
                match rx.recv() {
                Ok(Ok(notify::Event{ kind: notify::EventKind::Modify(_),  ..})) |
                Ok(Ok(notify::Event{ kind: notify::EventKind::Create(_),  ..}))
                => {
                    println!("File modified");
                    send_and_run_file(&xfer, fname);
                },
                Err(err) => println!("watch error: {:?}", err),
                _ => {}
                };
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
        interact::start(&xfer);
    }

    Ok(())
}

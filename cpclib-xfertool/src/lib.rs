#[cfg(feature = "interactive")]
pub mod interact;
pub mod parser;

use std::path::PathBuf;

use cpclib_common::clap;
use cpclib_common::clap::builder::TypedValueParser;
use cpclib_xfer::{send_and_run_file, CpcXfer};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::clap::builder::PathBufValueParser;
use crate::clap::{ArgAction, Command};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn build_args_parser() -> clap::Command {
    let cmd = clap::Command::new("CPC xfer to M4")
    .author("Krusty/Benediction")
    .version(built_info::PKG_VERSION)
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
            .value_parser(
                PathBufValueParser::new()
                    .try_map(|p: PathBuf| {
                        if p.exists() {
                            Ok(p)
                        } else {
                            Err(format!("{} does not exists", p.display().to_string()))
                        }
                    })
            )
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
            .value_parser(
                PathBufValueParser::new()
                    .try_map(|p: PathBuf| {
                        if p.exists() {
                            Ok(p)
                        } else {
                            Err(format!("{} does not exists", p.display().to_string()))
                        }
                    })
            )
            .required(true)
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
    );

    if cfg!(feature = "interactive") {
        cmd.subcommand(Command::new("--interactive").about("Start an interactive session"))
    }
    else {
        cmd
    }
}

pub fn process(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    // Retreivethe hostname from the args or from the environment
    let hostname: String = match matches.get_one::<String>("CPCADDR") {
        Some(cpcaddr) => cpcaddr.to_string(),
        None => {
            match std::env::var("CPCIP") {
                Ok(cpcaddr) => cpcaddr,
                Err(_) => {
                    return Err(anyhow::Error::msg("You should provide the CPCADDR argument or set the CPCIP environmeent variable"));
                }
            }
        },
    };

    let xfer = CpcXfer::new(hostname);

    if let Some("-r") = matches.subcommand_name() {
        xfer.reset_m4()?;
    }
    else if let Some("-s") = matches.subcommand_name() {
        xfer.reset_cpc()?;
    }
    else if let Some(p_opt) = matches.subcommand_matches("-p") {
        let fname: &PathBuf = p_opt.get_one("fname").unwrap();
        send_and_run_file(&xfer, &fname, false);
    }
    else if let Some(y_opt) = matches.subcommand_matches("-y") {
        let fname: &PathBuf = y_opt.get_one("fname").unwrap();

        // Simple file sending
        send_and_run_file(&xfer, &fname, true);

        if y_opt.get_flag("WATCH") {
            println!(
                "I will not stop and redo the operation when detecting a modification of the file"
            );
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = RecommendedWatcher::new(
                move |res| tx.send(res).unwrap(),
                notify::Config::default()
            )?;

            watcher.watch(std::path::Path::new(&fname), RecursiveMode::NonRecursive)?;

            for res in rx {
                match res {
                    Ok(notify::event::Event {
                        kind:
                            notify::event::EventKind::Modify(_) | notify::event::EventKind::Create(_),
                        ..
                    }) => {
                        send_and_run_file(&xfer, &fname, true);
                    },
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
        #[cfg(feature = "interactive")]
        {
            println!("Benediction welcomes you to the interactive mode for M4.");
            interact::XferInteractor::start(&xfer);
        }
    }

    Ok(())
}

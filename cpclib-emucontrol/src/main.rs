use std::process::exit;
use std::time::Duration;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use cpclib_bndbuild::runners::emulator::{AceVersion, Emulator};
use cpclib_emucontrol::{get_emulator_window, start_emulator, EmulatorConf, Robot};
use enigo::{Enigo, Settings};

#[derive(Parser)]
struct Cli {
    #[arg(short = 'a', long = "drivea", value_name = "DISCA")]
    drive_a: Option<String>,

    #[arg(short = 'b', long = "driveb", value_name = "DISCB")]
    drive_b: Option<String>,

    #[arg(short, long, default_value = "ace")]
    emulator: Emu,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Keep the emulator open after the interaction")]
    keepemulator: bool,

    #[command(subcommand)]
    command: Commands
}

#[derive(ValueEnum, Clone)]
enum Emu {
    Ace,
    Winape,
    Cpcec
}

#[derive(Subcommand, Clone)]
enum Commands {
    Orgams {
        /// lists test values
        #[arg(short, long, help = "Filename to assemble")]
        src: String,

        #[arg(
            short,
            long,
            help = "Filename to save. By default use the one provided by orgams"
        )]
        dst: Option<String>,

        #[arg(short, long, action = ArgAction::SetTrue, conflicts_with="dst", help="Jump on the program instead of saving it")]
        jump: bool
    }
}

fn main() {
    let cli = Cli::parse();

    let builder = EmulatorConf::builder()
        .maybe_drive_a(cli.drive_a)
        .maybe_drive_b(cli.drive_b);
    let conf = builder.build();

    let emu = match cli.emulator {
        Emu::Ace => Emulator::Ace(AceVersion::default()),
        Emu::Winape => todo!(),
        Emu::Cpcec => todo!()
    };

    let t_emu = emu.clone();
    let emu_thread = std::thread::spawn(move || start_emulator(&t_emu, &conf).unwrap());

    std::thread::sleep(Duration::from_secs(3)); // sleep over 2 seconds

    let window = get_emulator_window(&emu);
    let enigo = Enigo::new(&Settings::default()).unwrap();
    let mut robot = Robot::new(&emu, window, enigo);

    let res = match cli.command {
        Commands::Orgams { src, dst, jump } => {
            if jump && !cli.keepemulator {
                eprintln!("You must request to keep the emulator open with -k");
                robot.close();
                exit(-1);
            }

            println!("!!! Current limitation: Ace must be configure as Amstrad old and must have enough memory. !!!");
            robot.handle_orgams(&src, dst.as_ref().map(|s| s.as_str()), jump)
        }
    };

    if !cli.keepemulator {
        robot.close();
    }

    match res {
        Ok(_) => println!("No error occurred. File has been saved in DSK of drive A"),
        Err(e) => {
            eprintln!("An error occurred. {}", e);
            exit(-1);
        }
    }
}

use std::process::exit;
use std::time::Duration;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use cpclib_bndbuild::runners::emulator::{AceVersion, CpcecVersion, Emulator, WinapeVersion};
use cpclib_emucontrol::{get_emulator_window, start_emulator, EmulatorConf, Robot};
use enigo::{Enigo, Settings};

#[derive(Parser, Debug)]
struct Cli {
    #[arg(
        short = 'a',
        long = "drivea",
        value_name = "DISCA",
        help = "Disc A image"
    )]
    drive_a: Option<String>,

    #[arg(
        short = 'b',
        long = "driveb",
        value_name = "DISCB",
        help = "Disc B image"
    )]
    drive_b: Option<String>,

    #[arg(
        long = "albireo",
        value_name = "FOLDER",
        help = "Albireo content (only for ACE) - WARNING. It is destructive as it completely replaces the existing content"
    )]
    albireo: Option<String>,

    #[arg(short, long, default_value = "ace", alias = "emu")]
    emulator: Emu,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Keep the emulator open after the interaction")]
    keepemulator: bool,

    #[command(subcommand)]
    command: Commands
}

#[derive(ValueEnum, Clone, Debug)]
enum Emu {
    Ace,
    Winape,
    Cpcec
}

#[derive(Subcommand, Clone, Debug)]
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
    },

    Run {
        #[arg(short, long, help = "Simple text to type")]
        text: Option<String>
    }
}

fn main() {
    let mut cli = Cli::parse();

    let builder = EmulatorConf::builder()
        .maybe_drive_a(cli.drive_a.clone())
        .maybe_drive_b(cli.drive_b.clone());
    let conf = builder.build();

    let emu = match cli.emulator {
        Emu::Ace => Emulator::Ace(AceVersion::default()),
        Emu::Winape => Emulator::Winape(WinapeVersion::default()),
        Emu::Cpcec => Emulator::Cpcec(CpcecVersion::default())
    };

    // setup emulator
    // TODO do it conditionally
    // copy the non standard roms
    let needed_roms = [
        "unidos.rom",
        "unitools.rom",
        "albireo.rom",
        "nova.rom",
        "parados12.fixedanyslot.fixedname.quiet.rom"
    ];
    for rom in needed_roms {
        let dst = emu.roms_folder().join(rom);

        if !dst.exists() {
            let src = std::path::Path::new("roms").join(rom);
            std::fs::copy(src, dst).unwrap();
        }
    }

    #[cfg(unix)]
    let albireo_backup_and_original = if let Some(albireo) = &cli.albireo {
        let emu_folder = emu.albireo_folder();
        let backup_folder = emu_folder
            .parent()
            .unwrap()
            .join(emu_folder.file_name().unwrap().to_owned() + ".bak");

        if backup_folder.exists() {
            std::fs::remove_dir_all(&backup_folder).unwrap();
        }

        std::fs::rename(&emu_folder, &backup_folder).unwrap();

        std::os::unix::fs::symlink(
            std::path::absolute(&albireo).unwrap(),
            std::path::absolute(&emu_folder).unwrap()
        )
        .unwrap();

        Some((backup_folder, emu_folder))
    }
    else {
        None
    };

    // I had issues with symlinks on windows. no time to search why
    #[cfg(windows)]
    if let Some(albireo) = &cli.albireo {
        let option = CopyOptions::new()
            .copy_inside(true)
            .overwrite(true)
            .skip_exist(false)
            .content_only(true);
        let emu_folder = emu.albireo_folder();
        if emu_folder.exists() {
            std::fs::remove_dir_all(&emu_folder).unwrap();
        }
        fs_extra::dir::copy(albireo, &emu_folder, &option).unwrap();
    }

    let t_emu = emu.clone();
    let emu_thread = std::thread::spawn(move || start_emulator(&t_emu, &conf).unwrap());

    if cli.albireo.is_some() {
        std::thread::sleep(Duration::from_secs(8));
    }
    else {
        std::thread::sleep(Duration::from_secs(3));
    }

    let window = get_emulator_window(&emu);
    let enigo = Enigo::new(&Settings::default()).unwrap();
    let mut robot = Robot::new(&emu, window, enigo);

    #[cfg(windows)]
    std::thread::sleep(Duration::from_millis(1000 * 3));

    let res = match cli.command {
        Commands::Orgams { src, dst, jump } => {
            if jump && !cli.keepemulator {
                eprintln!("You must request to keep the emulator open with -k");
                robot.close();
                exit(-1);
            }

            println!("!!! Current limitation: Ace must be configure as\n - Amstrad old\n - with a French keyboard\n - a French firmware\n - Unidos with nova and albireo\n - and must have enough memory. !!! No idea yet how to overcome that without modifying ace");

            robot.handle_orgams(
                cli.drive_a.as_ref().map(|s| s.as_str()),
                cli.albireo.as_ref().map(|s| s.as_str()),
                &src,
                dst.as_ref().map(|s| s.as_str()),
                jump
            )
        },

        Commands::Run { text } => {
            cli.keepemulator = true;

            if let Some(text) = text {
                robot.handle_raw_text(text);
            }

            Ok(())
        }
    };

    if !cli.keepemulator {
        robot.close();
    }

    #[cfg(unix)]
    if let Some((backup_folder, emu_folder)) = albireo_backup_and_original {
        std::fs::rename(&backup_folder, &emu_folder).unwrap();
    }

    #[cfg(windows)]
    if let Some(albireo) = &cli.albireo {
        let option = CopyOptions::new()
            .copy_inside(true)
            .overwrite(true)
            .skip_exist(false)
            .content_only(true);
        let emu_folder = emu.albireo_folder();
        std::fs::remove_dir_all(&albireo).unwrap();
        fs_extra::dir::copy(&emu_folder, albireo, &option).unwrap();
    }

    match res {
        Ok(_) => println!("No error occurred."),
        Err(e) => {
            eprintln!("An error occurred. {}", e);
            exit(-1);
        }
    }
}

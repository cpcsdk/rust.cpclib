use cpclib_bndbuild::executor::*;
use cpclib_bndbuild::runners::RunnerWithClap;
use cpclib_bndbuild::{built_info, BndBuilder, BndBuilderError};
use cpclib_common::clap::*;
use cpclib_common::itertools::Itertools;

fn main() {
    match inner_main() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failure\n{}", e);
            std::process::exit(-1);
        }
    }
}

fn inner_main() -> Result<(), BndBuilderError> {
    let cmd = Command::new("bndbuilder")
        .about("Benediction CPC demo project builder")
        .author("Krusty/Benediction")
        .version(built_info::PKG_VERSION)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .value_name("CMD")
                .value_parser(["img2cpc", "basm", "rm", "bndbuild", "xfer"])
                .default_missing_value_os("bndbuild")
                .default_value("bndbuild")
                .num_args(0..=1)
                .help("Show the help of the given subcommand CMD.")
        )
        .arg(
            Arg::new("version")
                .long("version")
                .short('V')
                .help("Print version")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .action(ArgAction::Set)
                .value_name("FILE")
                .value_hint(ValueHint::FilePath)
                .default_value("bndbuild.yml")
                .help("Provide the YAML file for the given project.")
        )
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .action(ArgAction::SetTrue)
                .help("Watch the targets and permanently rebuild them when needed.")
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .action(ArgAction::SetTrue)
                .help("List the available targets")
        )
        .arg(
            Arg::new("target")
                .action(ArgAction::Append)
                .value_name("TARGET")
                .help("Provide the target(s) to run.")
                .conflicts_with("list")
        );

    let matches = cmd.clone().get_matches();

    if matches.value_source("help") == Some(parser::ValueSource::CommandLine) {
        match matches.get_one::<String>("help").unwrap().as_str() {
            "bndbuild" => {
                cmd.clone().print_long_help().unwrap();
            },
            "basm" => {
                BASM_RUNNER.print_help();
            },
            "img2cpc" => {
                IMGCONV_RUNNER.print_help();
            },
            "rm" => {
                RM_RUNNER.print_help();
            },
            "xfer" => {
                XFER_RUNNER.print_help();
            },
            _ => unimplemented!()
        };

        return Ok(());
    }

    if matches.get_flag("version") {
        println!(
            "{}\n{}\n{}\n{}",
            cmd.clone().render_long_version(),
            BASM_RUNNER.get_clap_command().render_long_version(),
            IMGCONV_RUNNER.get_clap_command().render_long_version(),
            XFER_RUNNER.get_clap_command().render_long_version()
        );
        return Ok(());
    }

    // Get the file and read it
    let fname: &String = matches.get_one("file").unwrap();
    if !std::path::Path::new(fname).exists() {
        eprintln!("{fname} does not exists.");
        if let Some(Some(fname)) = matches
            .get_many::<String>("target")
            .map(|s| s.into_iter().next())
        {
            if fname.ends_with("bndbuild.yml") {
                eprintln!("Have you forgotten to do \"-f {}\" ?", fname);
            }
        }
        std::process::exit(1);
    }

    let builder = BndBuilder::from_fname(fname)?;

    // Print list if asked
    if matches.get_flag("list") {
        for rule in builder.rules() {
            println!(
                "{}{}: {}",
                if rule.is_enabled() { "" } else { "[disabled] " },
                rule.targets()
                    .iter()
                    .map(|f| f.display().to_string())
                    .join(" "),
                rule.dependencies()
                    .iter()
                    .map(|f| f.display().to_string())
                    .join(" "),
            );
            if let Some(help) = rule.help() {
                println!("\t{}", help);
            }
        }
        return Ok(());
    }

    // Get the targets
    let targets_provided = matches.contains_id("target");
    let targets = if !targets_provided {
        if let Some(first) = builder.default_target() {
            vec![first]
        }
        else {
            return Err(BndBuilderError::NoTargets);
        }
    }
    else {
        matches
            .get_many::<String>("target")
            .unwrap()
            .into_iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&std::path::Path>>()
    };

    // Execute the targets
    let mut first_loop = true;
    let watch_requested = matches.get_flag("watch");
    loop {
        for tgt in targets.iter() {
            if first_loop || builder.outdated(tgt).unwrap_or(false) {
                builder.execute(tgt).map_err(|e| {
                    if targets_provided {
                        e
                    }
                    else {
                        BndBuilderError::DefaultTargetError {
                            source: Box::new(e)
                        }
                    }
                })?;
            }
        }

        if !watch_requested {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(1000)); // sleep 1s before trying to build
        first_loop = false;
    }

    Ok(())
}

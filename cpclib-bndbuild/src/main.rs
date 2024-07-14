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
    let basm_cmd = cpclib_basm::build_args_parser().name("basm");
    let img2cpc_cmd = cpclib_imgconverter::build_args_parser()
        .name("img2cpc")
        .disable_help_flag(false);
    let xfer_cmd = cpclib_xfertool::build_args_parser().name("xfer");
    let disc_cmd = cpclib_disc::dsk_manager_build_arg_parser().name("disc");

    let cmd = Command::new("bndbuilder")
        .about("Benediction CPC demo project builder")
        .before_help("Can be used as a project builder similar to Make, but using a yaml project description, or can be used as any benedicition crossdev tool (basm, img2cpc, xfer, disc). This way only bndbuild needs to be installed.")
        .author("Krusty/Benediction")
        .version(built_info::PKG_VERSION)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .subcommand_negates_reqs(true)
        .subcommand_precedence_over_arg(true)
        .subcommands(&[basm_cmd, img2cpc_cmd.clone(), xfer_cmd, disc_cmd])
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
            Arg::new("dot")
                .long("dot")
                .alias("grapĥviz")
                .help("Generate the .dot representation of the selected bndbuild.yml file")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("show")
                .long("show")
                .help("Show the file AFTER interpreting the templates")
                .action(ArgAction::SetTrue)
                .conflicts_with("dot")
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
                .conflicts_with_all(["dot", "show"])
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .action(ArgAction::SetTrue)
                .help("List the available targets")
                .conflicts_with("dot")
        )
        .arg(
            Arg::new("DEFINE_SYMBOL")
                .help("Provide a symbol with its value (default set to 1)")
                .long("define")
                .short('D')
                .action(ArgAction::Append)
                .number_of_values(1)
        )
        .arg(
            Arg::new("init")
                .long("init")
                .action(ArgAction::SetTrue)
                .help("Init a new project by creating it")
                .conflicts_with("dot")
        )
        .arg(
            Arg::new("add")
                .long("add")
                .short('a')
                .help("Add a new basm target in an existing bndbuild.yml (or create it)")
                .conflicts_with("dot")
                .action(ArgAction::Set)
        )
        .arg(
            Arg::new("dep")
                .help("The source files")
                .long("dep")
                .short('d')
                .requires("add")
        )
        .arg(
            Arg::new("kind")
                .help("The kind of command to be added in the yaml file")
                .long("kind")
                .short('k')
                .value_parser(["basm", "img2cpc", "xfer"])
                .requires("add")
                .default_missing_value("basm")
        )
        .arg(
            Arg::new("target")
                .action(ArgAction::Append)
                .value_name("TARGET")
                .help("Provide the target(s) to run.")
                .conflicts_with_all(["list", "init", "add"])
        );

    let matches = cmd.clone().get_matches();

    // handle command specific behavior
    if let Some(basm_matches) = matches.subcommand_matches("basm") {
        eprintln!("Switch to basm behavior, not bndbuild one.");
        let start = std::time::Instant::now();
        match cpclib_basm::process(basm_matches) {
            Ok((env, warnings)) => {
                for warning in warnings {
                    eprintln!("{warning}");
                }

                let report = env.report(&start);
                println!("{report}");

                std::process::exit(0);
            },
            Err(e) => {
                eprintln!("Error while assembling.\n{e}");
                std::process::exit(-1);
            }
        }
    }
    else if let Some(img2cpc) = matches.subcommand_matches("img2cpc") {
        eprintln!("Switch to img2cpc behavior, not bndbuild one.");
        cpclib_imgconverter::process(img2cpc, img2cpc_cmd)
            .map_err(|e| e.to_string())
            .expect("Error when launching img2cpc tool");
    }
    else if let Some(xfer) = matches.subcommand_matches("xfer") {
        eprintln!("Switch to xfer behavior, not bndbuild one.");
        cpclib_xfertool::process(xfer)
            .map_err(|e| e.to_string())
            .expect("Error when launching xfer tool");
    }
    else if let Some(disc) = matches.subcommand_matches("disc") {
        eprintln!("Switch to disc behavior, not bndbuild one.");
        cpclib_disc::dsk_manager_handle(disc)
            .map_err(|e| e.to_string())
            .expect("Error when launching disc tool");
    }
    else {
        // handle the real behavior of bndbuild
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

        if matches.get_flag("init") {
            cpclib_bndbuild::init_project(None)?;
            println!("Empty project initialized");
            return Ok(());
        }

        // Get the file
        let fname: &String = matches.get_one("file").unwrap();

        let add = matches.get_one::<String>("add");

        // Read it
        if !std::path::Path::new(fname).exists() {
            if add.is_some() {
                std::fs::File::create(fname).expect("create empty {fname}");
            }
            else {
                eprintln!("{fname} does not exists.");
                if let Some(Some(fname)) = matches
                    .get_many::<String>("target")
                    .map(|s| s.into_iter().next())
                {
                    if fname.ends_with("bndbuild.yml") {
                        eprintln!("Have you forgotten to do \"-f {}\" ?", fname);
                    }
                }

                if matches
                    .get_many::<String>("target")
                    .map(|s| s.into_iter().any(|s| s == "init"))
                    .unwrap_or(false)
                {
                    eprintln!("Maybe you wanted to do --init.");
                }
                std::process::exit(1);
            }
        }


    // Get the variables definition
    let definitions = if let Some(definitions) = matches.get_many::<String>("DEFINE_SYMBOL") {
        definitions.into_iter().map(|definition| {
            let (symbol, value) = {
                match definition.split_once("=") {
                    Some((symbol, value)) => (symbol, value),
                    None => (definition.as_str(), "1")
                }
            };
            (symbol, value)
        })
        .collect_vec()
    } else {
        Default::default()
    };



        let content = BndBuilder::decode_from_fname_with_definitions(fname, &definitions)?;
        if matches.get_flag("show") {
            println!("{content}");
            return Ok(());
        }

        let builder = BndBuilder::from_string(content)?;

        if let Some(add) = matches.get_one::<String>("add") {
            let targets = [add];
            let dependencies = matches
                .get_many::<String>("dep")
                .map(|l| l.collect_vec())
                .unwrap_or_default();
            let kind = matches.get_one::<String>("kind").unwrap();

            let builder = builder.add_default_rule(&targets, &dependencies, kind);
            builder.save(fname).expect("Error when saving the file");
            return Ok(());
        }

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

        if matches.get_flag("dot") {
            let dot = builder.to_dot();
            println!("{dot}")
        }
        else {
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
        }
    }

    Ok(())
}

use cpclib_bndbuild::executor::*;
use cpclib_bndbuild::runners::RunnerWithClap;
use cpclib_bndbuild::{BndBuilder, BndBuilderError};
use cpclib_common::clap::*;

fn main() {
    match inner_main() {
        Ok(_) => {}
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
        .version("0.01")
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
                .default_value("bndbuild.yml")
                .help("Provide the YAML file for the given project.")
        )
        .arg(
            Arg::new("target")
                .action(ArgAction::Append)
                .value_name("TARGET")
                .help("Provide the target(s) to run.")
        );

    let matches = cmd.clone().get_matches();

    if matches.value_source("help") == Some(parser::ValueSource::CommandLine) {
        match matches.get_one::<String>("help").unwrap().as_str() {
            "bndbuild" => {
                cmd.clone().print_long_help().unwrap();
            }
            "basm" => {
                BASM_RUNNER.print_help();
            }
            "img2cpc" => {
                IMGCONV_RUNNER.print_help();
            }
            "rm" => {
                RM_RUNNER.print_help();
            }
            "xfer" => {
                XFER_RUNNER.print_help();
            }
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

    let builder = BndBuilder::from_fname(fname)?;
    if !matches.contains_id("target") {
        if let Some(first) = builder.default_target() {
            builder.execute(first).map_err(|e| {
                BndBuilderError::DefaultTargetError {
                    source: Box::new(e)
                }
            })?;
        }
        else {
            return Err(BndBuilderError::NoTargets);
        }
    }
    else {
        for target in matches.get_many::<String>("target").unwrap() {
            builder.execute(target)?;
        }
    }

    Ok(())
}

use cpclib_common::clap;
use cpclib_common::itertools::Itertools;

use crate::{BasmDocGenerator, UndocumentedConfig};

pub fn build_args_parser() -> clap::Command {
    clap::Command::new("basmdoc")
        .about("Generates assembly documentation in markdown format")
        .arg(
            clap::Arg::new("wildcards")
                .help("Enable wildcard expansion on input files")
                .short('w')
                .long("wildcards")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("input")
                .help("Input assembly file(s) or folder(s) (recursively searches for .asm files in folders)")
                .required(true)
                .num_args(1..)
        )
        .arg(
            clap::Arg::new("output")
                .help("Output markdown file")
                .short('o')
                .long("output")
                .required(true)
        )
        .arg(
            clap::Arg::new("undocumented")
                .help("Include all undocumented symbols (macros, functions, labels, equs)")
                .short('u')
                .long("undocumented")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-macros")
                .help("Include undocumented macros")
                .long("undocumented-macros")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-functions")
                .help("Include undocumented functions")
                .long("undocumented-functions")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-labels")
                .help("Include undocumented labels")
                .long("undocumented-labels")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-equs")
                .help("Include undocumented equs")
                .long("undocumented-equs")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("title")
                .help("Output title")
                .short('t')
                .long("title")
                .required(false)
        )
}

pub fn handle_matches(matches: &clap::ArgMatches, _cmd: &clap::Command) -> Result<(), String> {
    let inputs: Vec<String> = matches
        .get_many::<String>("input")
        .expect("required")
        .cloned()
        .collect();
    let output = matches.get_one::<String>("output").expect("required");
    let enable_wildcards = matches.get_flag("wildcards");
    let title = matches.get_one::<String>("title").cloned();

    // Build undocumented config from flags
    let undocumented_config = if matches.get_flag("undocumented") {
        UndocumentedConfig::all()
    } else {
        UndocumentedConfig {
            macros: matches.get_flag("undocumented-macros"),
            functions: matches.get_flag("undocumented-functions"),
            labels: matches.get_flag("undocumented-labels"),
            equs: matches.get_flag("undocumented-equs"),
        }
    };

    // Create generator with configuration
    let mut generator = BasmDocGenerator::new()
        .add_inputs(inputs)
        .with_wildcards(enable_wildcards)
        .with_undocumented_config(undocumented_config)
        .with_progress(true);

    if let Some(title) = title {
        generator = generator.with_title(title);
    }

    // Generate and save output
    generator.save_to_file(output)?;

    Ok(())
}

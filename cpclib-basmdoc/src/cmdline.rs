use cpclib_common::clap;
use cpclib_common::itertools::Itertools;

use crate::DocumentationPage;

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
                .help("Input assembly file")
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
}

pub fn handle_matches(matches: &clap::ArgMatches, cmd: &clap::Command) -> Result<(), String> {

    if matches.get_flag("help") {
        cmd.clone().print_long_help().map_err(|e| e.to_string())?;
        return Ok(());
    }

    if matches.get_flag("version") {
        todo!()
    }

    let inputs = matches.get_many::<String>("input").expect("required");
    let output = matches.get_one::<String>("output").expect("required");

    let inputs = if matches.get_flag("wildcards") {
        let expanded_inputs = inputs
            .flat_map(|input| {
                glob::glob(input)
                    .map_err(|e| format!("Invalid wildcard pattern {}: {}", input, e))
                    .and_then(|paths| {
                        paths
                            .map(|res| res.map(|p| p.to_string_lossy().to_string()))
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| {
                                format!("Error reading files for pattern {}: {}", input, e)
                            })
                    })
            })
            .flatten()
            .collect_vec();

        if expanded_inputs.is_empty() {
            return Err("No input files found for the given patterns.".to_string());
        }

        expanded_inputs
    }
    else {
        inputs.map(|s| s.to_string()).collect()
    };
    let inputs = inputs.into_iter();

    let output = std::path::Path::new(output);

    let mut docs = inputs
        .map(|input| DocumentationPage::for_file(&input))
        .map(|page| page.map(|page| page.to_markdown()))
        .map(|res| {
            match res {
                Ok(md) => md,
                Err(e) => format!("**Error generating documentation:**\n\n```\n{}\n```\n", e)
            }
        });
    let md = docs.join("\n\n---\n\n");

    if let Some(ext) = output.extension() {
        let is_md = ext.eq_ignore_ascii_case("md");

        let md_fname = if is_md {
            output.to_owned()
        }
        else {
            output.with_extension(".md") // TODO create a temp file instead?
        };

        // save the markdown file
        std::fs::write(&md_fname, md)
            .map_err(|e| format!("Unable to write {} file. {e}", md_fname.display()))?;

        // export to the final output if needed
        if !is_md {
            let mut pandoc = pandoc::new();
            pandoc.add_input(&md_fname);
            pandoc.set_output(pandoc::OutputKind::File(output.into()));
            pandoc.add_option(pandoc::PandocOption::Standalone);
            pandoc.add_option(pandoc::PandocOption::TableOfContents);
            pandoc
                .execute()
                .map_err(|e| format!("Pandoc error: {}", e))?;
        }
    }
    else {
        return Err("Output file must have .md extension".to_string());
    }

    Ok(())
}

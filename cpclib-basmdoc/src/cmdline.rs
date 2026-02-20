use cpclib_common::clap::{self, CommandFactory, Parser};

use crate::{BasmDocGenerator, UndocumentedConfig};

/// Generates assembly documentation in markdown format
#[derive(Parser, Debug)]
#[command(name = "basmdoc")]
#[command(about = "Generates assembly documentation in markdown format", author = "Krusty/Benediction")]
pub struct BasmDocCommand {
    /// Input assembly file(s) or folder(s) (recursively searches for .asm files in folders)
    #[arg(required = true)]
    pub input: Vec<String>,

    /// Output markdown file
    #[arg(short = 'o', long = "output", required = true)]
    pub output: String,

    /// Enable wildcard expansion on input files
    #[arg(short = 'w', long = "wildcards")]
    pub wildcards: bool,

    /// Include all undocumented symbols (macros, functions, labels, equs)
    #[arg(short = 'u', long = "undocumented")]
    pub undocumented: bool,

    /// Include undocumented macros
    #[arg(long = "undocumented-macros")]
    pub undocumented_macros: bool,

    /// Include undocumented functions
    #[arg(long = "undocumented-functions")]
    pub undocumented_functions: bool,

    /// Include undocumented labels
    #[arg(long = "undocumented-labels")]
    pub undocumented_labels: bool,

    /// Include undocumented equs
    #[arg(long = "undocumented-equs")]
    pub undocumented_equs: bool,

    /// Output title
    #[arg(short = 't', long = "title")]
    pub title: Option<String>,

    /// Disable HTML minification (enabled by default)
    #[arg(long = "no-minify")]
    pub no_minify: bool,
}

impl BasmDocCommand {
    /// Execute the command
    pub fn execute(&self) -> Result<(), String> {
        // Build undocumented config from flags
        let undocumented_config = if self.undocumented {
            UndocumentedConfig::all()
        } else {
            UndocumentedConfig {
                macros: self.undocumented_macros,
                functions: self.undocumented_functions,
                labels: self.undocumented_labels,
                equs: self.undocumented_equs,
            }
        };

        let minify = !self.no_minify;

        // Create generator with configuration
        let mut generator = BasmDocGenerator::new()
            .add_inputs(self.input.clone())
            .with_wildcards(self.wildcards)
            .with_undocumented_config(undocumented_config)
            .with_progress(true)
            .with_minify(minify);

        if let Some(ref title) = self.title {
            generator = generator.with_title(title.clone());
        }

        // Generate and save output
        generator.save_to_file(&self.output)?;

        Ok(())
    }
}

/// Build the clap Command for basmdoc (for compatibility)
pub fn build_args_parser() -> clap::Command {
    BasmDocCommand::command()
}

/// Handle command-line matches (for compatibility with existing code)
pub fn handle_matches(matches: &clap::ArgMatches, _cmd: &clap::Command) -> Result<(), String> {
    // Parse matches into BasmCommand structure
    let inputs: Vec<String> = matches
        .get_many::<String>("input")
        .expect("required")
        .cloned()
        .collect();
    let output = matches.get_one::<String>("output").expect("required").clone();
    let enable_wildcards = matches.get_flag("wildcards");
    let title = matches.get_one::<String>("title").cloned();
    let minify = !matches.get_flag("no-minify");

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
        .with_progress(true)
        .with_minify(minify);

    if let Some(title) = title {
        generator = generator.with_title(title);
    }

    // Generate and save output
    generator.save_to_file(&output)?;

    Ok(())
}

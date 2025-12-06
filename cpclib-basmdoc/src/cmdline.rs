use cpclib_common::clap;

use crate::DocumentationPage;

pub fn build_args_parser() -> clap::Command {
	let command = clap::Command::new("basmdoc")
		.about("Generates assembly documentation in markdown format")
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

		);

	command
}

pub fn handle_matches(matches: &clap::ArgMatches) -> Result<(), String>{
	let input = matches.get_one::<String>("input").expect("required");
	let output = matches.get_one::<String>("output").expect("required");

	let output = std::path::Path::new(output);

	let doc = DocumentationPage::for_file(input);
	let md = doc.to_markdown();


	if let Some(ext) = output.extension() {
		let is_md = ext.to_ascii_lowercase() == "md";

		let md_fname = if is_md{
			output.to_owned()
		} else {
			output.with_extension(".md") // TODO create a temp file instead?
		};

		// save the markdown file
		std::fs::write(&md_fname, md)
			.map_err(|e| format!("Unable to write {} file. {e}", md_fname.display()))?;

		// export to the final output if needed
		if !is_md {
			let mut pandoc = pandoc::new();
			pandoc.add_input(&md_fname);
			pandoc.set_output(pandoc::OutputKind::File(output.clone().into()));
			pandoc.add_option(pandoc::PandocOption::Standalone);
			pandoc.execute().map_err(|e| format!("Pandoc error: {}", e))?;
		}

	} else {
		return Err("Output file must have .md extension".to_string());
	}


	Ok(())
}
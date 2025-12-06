use cpclib_common::clap;

use crate::DocumentationPage;

pub fn build_args_parser() -> clap::Command {
	let command = clap::Command::new("basmdoc")
		.about("Generates assembly documentation in markdown format")
		.arg(
			clap::Arg::new("input")
				.help("Input assembly file")
				.required(true)
				.index(1),
		)
		.arg(
			clap::Arg::new("output")
				.help("Output markdown file")
				.required(true)
				.index(2),
		);

	command
}

pub fn handle_matches(matches: &clap::ArgMatches) -> Result<(), String>{
	let input = matches.get_one::<String>("input").expect("required");
	let output = matches.get_one::<String>("output").expect("required");

	let doc = DocumentationPage::for_file(input);
	let md = doc.to_markdown();

	std::fs::write(output, md).expect("Unable to write output file");

	Ok(())
}
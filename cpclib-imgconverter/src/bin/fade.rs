use cpclib::{Ink, Palette, common::{clap::{Arg, ArgAction, ArgMatches, Command, value_parser}, itertools::Itertools}};
use cpclib_imgconverter::{get_requested_palette, specify_palette};


fn build_args() -> Command {
	let cmd = Command::new("fade");
	let cmd = cmd.arg(
		Arg::new("SYMBOLS")
			.help("Use symbols in assembly generated code")
			.action(ArgAction::SetTrue)
			.required(false)
			.long("symbols")
		)
		.arg(
			Arg::new("OUTPUT")
				.help("Filename to store the result. Console otherwise")
				.required(false)
				.short('o')
				.long("output")
		)
		;
	let cmd = specify_palette!(cmd, false);
	

	cmd.subcommand(
		Command::new("rgb")
			.about("Use the algorithm described in http://cpc.sylvestre.org/technique/technique_coul5.html")
			.alias("superlsy")
	)
}

fn output_ga_assembly(palettes: &[Palette]) -> String{
	palettes.iter().map(|palette| {
		let repr = palette.inks()
			.into_iter()
			.map(|ink: Ink| ink.gate_array_value())
			.map(|ga| format!("0x{ga:x}")) 
			.join(",");
		format!("\tdw {repr}")
	})
	.join("\n")
}

fn output_symbols_assembly(palettes: &[Palette]) -> String {
	palettes.iter().map(|palette| {
		let repr = palette.inks()
			.into_iter()
			.map(|ink: Ink| format!("GA_{ink}"))
			.join(",");
		format!("\tdw {repr}")
	})
	.join("\n")
}

fn handle_matches(matches: &ArgMatches) -> Result<(), String>{

	let palette = get_requested_palette(matches)
		.map_err(|e| e.to_string())?;

	let fades = if let Some(_rgb) = matches.subcommand_matches("rgb") {
		palette.rgb_fadout()
	} else {
		return Err("A command is expected".to_owned())
	};

	let content = if matches.get_flag("SYMBOLS") {
		output_symbols_assembly(&fades)
	} else{
		output_ga_assembly(&fades)
	};

	if let Some(fname) = matches.get_one::<String>("OUTPUT") {
		std::fs::write(fname, content).expect("Error while saving file");
	} else {
		println!("{content}");
	}

	Ok(())
}


fn main() {
	let cmd = build_args();
	let matches = cmd.get_matches();
	handle_matches(&matches).expect("Error in the generation");
}
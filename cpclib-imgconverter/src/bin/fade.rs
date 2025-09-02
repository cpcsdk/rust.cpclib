use cpclib::{Ink, Palette, common::{clap::{Arg, ArgAction, ArgMatches, Command, Parser, value_parser}, itertools::Itertools}};
use cpclib_imgconverter::{get_requested_palette, specify_palette};


fn build_args() -> Command {
	let cmd = Command::new("fade");
	let cmd = cmd.arg(
		Arg::new("SYMBOLS")
			.help("Use symbols in assembly generated code")
			.action(ArgAction::SetTrue)
			.required(false)
			.long("symbols")
		);
	let cmd = specify_palette!(cmd, false);
	let cmd = cmd.subcommand(
		Command::new("rgb")
			.about("Use the algorithm described in http://cpc.sylvestre.org/technique/technique_coul5.html")
			.alias("superlsy")
	);

	cmd
}

fn output_ga_assembly(palettes: &[Palette]) {
	for palette in palettes {
		let repr = palette.inks()
			.into_iter()
			.map(|ink: Ink| ink.gate_array_value())
			.map(|ga| format!("0x{ga:x}")) 
			.join(",");
		println!("\tdw {repr}");
	}
}

fn output_symbols_assembly(palettes: &[Palette]) {
	for palette in palettes {
		let repr = palette.inks()
			.into_iter()
			.map(|ink: Ink| format!("GA_{ink}"))
			.join(",");
		println!("\tdw {repr}");
	}
}

fn handle_matches(matches: &ArgMatches) -> Result<(), String>{

	let palette = get_requested_palette(matches)
		.map_err(|e| e.to_string())?;

	let fades = if let Some(rgb) = matches.subcommand_matches("rgb") {
		palette.rgb_fadout()
	} else {
		return Err("A command is expected".to_owned())
	};

	if matches.get_flag("SYMBOLS") {
		output_symbols_assembly(&fades);
	} else{
		output_ga_assembly(&fades);
	}

	Ok(())
}


fn main() {
	let cmd = build_args();
	let matches = cmd.get_matches();
	handle_matches(&matches).expect("Error in the generation");
}
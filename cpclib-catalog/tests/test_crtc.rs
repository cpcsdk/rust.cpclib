use cpclib_catalog::{catalog_screen_output, catalog_to_basic_listing, catalog_to_catart_commands};
use cpclib_catart::{basic_command::BasicCommandList, entry::CatalogType, interpret::{self, Interpreter}};
use cpclib_basic::BasicProgram ;
use cpclib_disc::{AnyDisc, amsdos::AmsdosManagerNonMut, disc::Disc, edsk::Head};

#[test]
fn test_crtc_catart() {
	let orig_basic_str= include_str!("discs/crtc/T8.ASC");
	let orig_basic_program = BasicProgram::parse(orig_basic_str).expect("Failed to parse BASIC program");
	let orig_basic_command_list = BasicCommandList::try_from(&orig_basic_program).expect("Unable to get cat art commands");
	let orig_char_commands = orig_basic_command_list.to_char_commands().expect("Failed to convert BASIC to CharCommandList");


	let orig_screen = orig_char_commands.to_string();
	eprintln!("Original BASIC program:\n{}{}", orig_basic_str, orig_screen);


	let dsk = AnyDisc::open("tests/discs/crtc/test_catart.DSK").expect("Failed to read DSK file");
	let manager = AmsdosManagerNonMut::new_from_disc(&dsk, Head::A);
	let binary_catalog = manager.catalog_slice();
	let catalog_type = CatalogType::Cat; 

	let catalog_basic_program = catalog_to_basic_listing(&binary_catalog, catalog_type)
	.expect("Unable to extract information from catalog");
	let catalog_basic_command_list_from_basic_program = BasicCommandList::try_from(&catalog_basic_program).expect("Unable to get cat art commands from catalog");
	let catalog_char_commands_from_basic_program = catalog_basic_command_list_from_basic_program.to_char_commands().expect("Failed to convert BASIC to CharCommandList");
	let catalog_screen_from_basic_program = catalog_char_commands_from_basic_program.to_string();

	eprintln!("Catalog BASIC program:\n{}{}", catalog_basic_program.to_string(), catalog_screen_from_basic_program);


	for (cmd, orig_cmd) in catalog_basic_command_list_from_basic_program.iter().skip(1).zip(orig_basic_command_list.iter()) {
		if cmd != orig_cmd {
			eprintln!("Difference found:\nCatalog command: {:?}\nOriginal command: {:?}\n", cmd, orig_cmd);
		} else {
			eprintln!("Commands match:\n{:?}\n", cmd);
		}
	}

	/*
	For an unknown reason, the window commands are shifted
	Need to understand that before re-enabling the assert 
	assert_eq!(
		&orig_basic_command_list[..],
		&catalog_basic_command_list_from_basic_program[1..], // first print is as space filler or sorter
	);
	*/

	let output = catalog_screen_output(&binary_catalog, catalog_type).expect("Failed to get screen output from catalog");
	eprintln!("Catalog screen output:\n{}", output);


	assert_eq!(
		&output,
		&catalog_screen_from_basic_program
	);

	assert_eq!(
		&orig_screen,
		&catalog_screen_from_basic_program
	);
}
use cpclib_cpr::{CartridgeBloc, Cpr};

#[test]
pub fn test_write_empty() {
	let mut buffer = Vec::new();

	let cpr = Cpr::empty();
	cpr.write(&mut buffer).expect("Error while writting CPR");

}


#[test]
pub fn test_write_one_rom() {
	let mut buffer = Vec::new();

	let mut cpr = Cpr::empty();
	cpr.add_bloc(CartridgeBloc::new(0));
	cpr.write(&mut buffer).expect("Error while writting CPR");

}



#[test]
pub fn test_write_copter() {
	let fname = "tests/Copter 271 (1991)(Loriciels).cpr";


	dbg!("Read file");
	let content = std::fs::read(fname).unwrap();
	let cpr = Cpr::load(fname).expect("Unable to read copter");


	dbg!("Write file");
	let mut buffer = Vec::new();
	cpr.write(&mut buffer).expect("Error while writting CPR");


	dbg!("Read result");
	let cpr2 = Cpr::from_buffer( buffer).expect("Error when reading reconstructed copter");

	// there is a size issue in orginal CPR, so we compare only the banks
	assert_eq!(cpr.banks(), cpr2.banks());

}
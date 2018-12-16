extern crate cpc;

#[cfg(test)]
mod tests {

	#[test]
	fn open_edsk() {
		let dsk = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();
	}



	#[test]
	fn save_edsk() {
		let tmp_file = "/tmp/tmp.dsk";
		let dsk1 = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();
		dsk1.save(tmp_file);
		let ds2 = cpc::disc::edsk::ExtendedDsk::open(tmp_file).unwrap();

	}
}
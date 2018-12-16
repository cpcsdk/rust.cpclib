extern crate cpc;

#[cfg(test)]
mod tests {

	#[test]
	fn open_edsk() {
		let dsk = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();

		let track = dsk.get_track_information(&cpc::disc::edsk::Side::SideA, 0).unwrap();
		assert_eq!(track.number_of_sectors(), 9);

		for (sector, sum) in &[
			(0xc1, 21413),
			(0xc6, 60263),
			(0xc2, 22014),
			(0xc7, 49447),
			(0xc3, 85780)
		] {

			assert_eq!(
				track.sector(*sector).unwrap().data_sum(), 
				*sum);
		}

		assert_eq!(track.data_sum(), 484121);
	}



	#[test]
	fn save_edsk() {
		let tmp_file = "/tmp/tmp.dsk";
		let dsk1 = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();
		dsk1.save(tmp_file);
		let ds2 = cpc::disc::edsk::ExtendedDsk::open(tmp_file).unwrap();

	}
}
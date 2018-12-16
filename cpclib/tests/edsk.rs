extern crate cpc;

#[cfg(test)]
mod tests {

	#[test]
	fn open_singel_side_edsk() {
		let dsk = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();

		let track = dsk.get_track_information(&cpc::disc::edsk::Side::SideA, 0).unwrap();
		assert_eq!(track.number_of_sectors(), 9);

		for (sector_idx, sum) in &[
			(0xc1, 21413),
			(0xc6, 60263),
			(0xc2, 22014),
			(0xc7, 49447),
			(0xc3, 85780)
		] {

			let sector = track.sector(*sector_idx).unwrap();
			let values = sector.values().iter().map(|&v|{format!("{:x}", v)}).collect::<Vec<_>>();
			println!("0x{:x} => {:?}", sector_idx, values);
			assert_eq!(
				values.len(),
				512
			);
			assert_eq!(
				sector.data_sum(), 
				*sum);
		}

		assert_eq!(track.data_sum(), 484121);
		assert_eq!(
			dsk.get_track_information(&cpc::disc::edsk::Side::SideA, 41).unwrap().data_sum(),
			329484);


	}


	#[test]
	fn open_double_side_edsk() {
		let dsk = cpc::disc::edsk::ExtendedDsk::open("bf2sides.dsk").unwrap();

		assert!(dsk.is_double_sided());
		assert_eq!(
			dsk.data_sum(cpc::disc::edsk::Side::SideA),
			66709468
		);

		assert_eq!(
			dsk.data_sum(cpc::disc::edsk::Side::SideB),
			54340792	
		);
	}

	#[test]
	fn save_edsk() {
		let tmp_file = "/tmp/tmp.dsk";
		let dsk1 = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();
		dsk1.save(tmp_file);
		let ds2 = cpc::disc::edsk::ExtendedDsk::open(tmp_file).unwrap();

	}
}
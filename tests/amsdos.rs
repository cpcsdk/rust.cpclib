
//#[macro_use]
//extern crate pretty_assertions;

#[cfg(test)]
mod tests {
	use cpclib::disc::amsdos::*;
	use cpclib::disc::edsk::ExtendedDsk;
	use cpclib::disc::cfg::DiscConfig;


	#[test]
	fn test_format() {
		let empty_expected = ExtendedDsk::open("./tests/dsk/empty.dsk").unwrap();
		let empty_obtained = DiscConfig::single_side_data_format().into();
		assert_eq!(
			empty_expected,
			empty_obtained
		);
	}

	#[test]
	fn list_catalog() {
		let dsk = cpclib::disc::edsk::ExtendedDsk::open("./tests/dsk/pirate.dsk").unwrap();
		let amsdos = cpclib::disc::amsdos::AmsdosManager::new_from_disc(dsk, 0);
		amsdos.print_catalog();
	}

	#[test]
	fn empty_catalog() {
		use cpclib::disc::cfg::DiscConfig;
		use cpclib::disc::amsdos::AmsdosManager;

		let dsk = DiscConfig::single_side_data_format().into();
		let manager = AmsdosManager::new_from_disc(dsk, 0);
		let catalog = manager.catalog();

		println!("{:?}", catalog);

		assert_eq!(
			catalog.used_entries().count(),
			0
		);

	}

	#[test]
	fn test_hideur() {
		let content = [0x41, 0x42, 0x43, 0x0a];
		let result = [
0,116,101,115,116,32,32,32,32,98,105,110,0,0,0,0,
0,0,2,0,0,16,50,0,4,0,52,18,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
4,0,0,11,4,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
65,66,67,10

];
		let header = &result[0..128];

		let filename = AmsdosFileName::new_incorrect_case(
			0,
			"test",
			"bin"
		).unwrap();
		let result_header = AmsdosManager::compute_binary_header(
			&filename, 
			0x3210, 
			0x1234, 
			&content);

		println!("{:?}", result_header);
		println!("Obtained\t{:?}\nExpected\t{:?}\n", 
			result_header.as_bytes().to_vec(), 
			header.to_vec());
		assert_eq!(
			result_header.as_bytes().to_vec(), 
			header.to_vec());
	}



	#[test]
	fn test_amsdos_file () {
		let content = [0x41, 0x42, 0x43, 0x0a];
		let result = [
0,116,101,115,116,32,32,32,32,98,105,110,0,0,0,0,
0,0,2,0,0,16,50,0,4,0,52,18,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
4,0,0,11,4,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
65,66,67,10

];
		let header = &result[0..128];

		let filename = AmsdosFileName::new_incorrect_case(
			0,
			"test",
			"bin"
		).unwrap();
		let file = AmsdosFile::binary_file_from_buffer(
			&filename, 
			0x3210, 
			0x1234, 
			&content)
			.unwrap();
		
		let obtained_result= file.full_content()
								.map(|&b|{b}).collect::<Vec<_>>();
		assert_eq!(
			obtained_result.len(),
			result.len()
		);
		assert_eq!(
			obtained_result,
			result.to_vec()
		);
	}

	#[test]
	fn test_filename() {
		let fname1 =  AmsdosFileName::new_correct_case(
			0,
			"test",
			"bin"
		).unwrap();

		let fname2:AmsdosFileName = "TEST.BIN".into();

		assert_eq!(fname1, fname2);
		assert_eq!(fname1.extension(), "BIN");
		assert_eq!(fname2.name(), "TEST");
		assert_eq!(fname2.user(), 0);
	}

	#[test]
	fn test_filename_bytes() {
		let bytes  = [0x00,0x2D,0x47,0x57,0x2D,0x46,0x52,0x20,0x20,0x42,0x41,0x53];
		let filename = AmsdosFileName::from_slice(&bytes);
		let result = filename.to_entry_format(false, false);

		println!("{:?}\n{:?}", &bytes, &result);
		assert_eq!(
			filename.user(),
			0
		);
		assert_eq!(
			filename.name(),
			"-GW-FR"
		);
		assert_eq!(
			filename.extension(),
			"BAS"
		);
		assert_eq!(
			filename.filename(),
			"-GW-FR.BAS"
		);
		assert_eq!(
			bytes,
			result
		);
	}


	#[test]
	fn test_entry() {
		let bytes = [0x00,0x2D,0x47,0x57,0x2D,0x46,0x52,0x20,0x20,0x42,0x41,0x53,0x00,0x00,0x00,0x06,0x02,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
		let entry = AmsdosEntry::from_buffer(0, &bytes);
		let file_results = entry.amsdos_filename().to_entry_format(false, false);
		let results = entry.as_bytes();


		println!("Expected:\t{:?}\nObtained:\t{:?}", &bytes[..12], &file_results);

		assert_eq!(
			&bytes[..12],
			&file_results
		);

		println!("Expected:\t{:?}\nObtained:\t{:?}", &bytes, &results);

	

		assert_eq!(
			&bytes,
			&results
		);
	}



	#[test]
	fn add_file() {
		use cpclib::disc::cfg::DiscConfig;
		use cpclib::disc::amsdos::AmsdosManager;

		let dsk = DiscConfig::single_side_data_format().into();
		let mut manager = AmsdosManager::new_from_disc(dsk, 0);
		let catalog = manager.catalog();

		assert_eq!(
			catalog.used_entries().count(),
			0
		);

		assert_eq!(
			catalog.free_entries().count(),
			64
		);

		let filename = AmsdosFileName::new_correct_case(
			0,
			"test",
			"bin"
		).unwrap();
		assert_eq!(
			&filename,
			&AmsdosFileName::from("test.bin")
		);



		let file = AmsdosFile::binary_file_from_buffer(
			&filename, 
			0x3210, 
			0x1234, 
			&[0x41, 0x42, 0x43, 0x0a]).unwrap();
			manager.add_file(&file, false, false).expect("Unable to add file");

		assert_eq!(
			& file.header().amsdos_filename().filename(),
			"TEST.BIN"
		);
		assert_eq!(
			& file.header().amsdos_filename(),
			& filename
		);
		assert_eq!(
			file.header().execution_address(),
			0x1234
		);
		assert_eq!(
			file.header().loading_address(),
			0x3210
		);

		let catalog_data = manager.dsk().sectors_bytes(0, 0, 0xc1, 4).unwrap();
		let entry_data = &catalog_data[..32];
		let entry = AmsdosEntry::from_slice(0, &entry_data); 
		println!("{:?}", entry_data);
		println!("{:?}", entry);
		assert_eq!(
			entry_data[0],
			entry.amsdos_filename().user()
		);
		assert_eq!(
			entry.amsdos_filename().user(),
			0
		);


		let catalog = manager.catalog();


		println!("{:?}", catalog);
	 	assert_eq!(
			catalog.used_entries().count(),
			1
		);
		let entry = catalog.used_entries().next().unwrap();
		assert_eq!(
			entry.amsdos_filename(),
			&AmsdosFileName::from("test.bin")
		);


		// TODO find a way to pass filename by reference
		let file2 = manager.get_file(filename);
		assert!(file2.is_some());
		let file2 = file2.unwrap();
		assert!(file2.header().is_checksum_valid());
		assert_eq!(
			&file.header(),
			&file2.header()
		);

		assert_eq!(
			&file.content().len(),
			&file2.content().len()
		);

		assert_eq!(
			&file.content(),
			&file2.content()
		);

	}

}
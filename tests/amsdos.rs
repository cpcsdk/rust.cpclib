
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
mod tests {
	use cpclib::disc::amsdos::*;



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

		let filename = AmsdosFileName::new(
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

		let filename = AmsdosFileName::new(
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

		let filename = AmsdosFileName::new(
			0,
			"test",
			"bin"
		).unwrap();
		let file = AmsdosFile::binary_file_from_buffer(
			&filename, 
			0x3210, 
			0x1234, 
			&[0x41, 0x42, 0x43, 0x0a]).unwrap();
			manager.add_file(file, false, false).expect("Unable to add file");
		let catalog = manager.catalog();


		println!("{:?}", catalog);
	 	assert_eq!(
			catalog.used_entries().count(),
			1
		);

	}

}
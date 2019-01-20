
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
mod tests {
	use cpclib::disc::amsdos::*;



	#[test]
	fn list_catalog() {
		let dsk = cpclib::disc::edsk::ExtendedDsk::open("./tests/dsk/pirate.dsk").unwrap();
		let amsdos = cpclib::disc::amsdos::AmsdosManager::new(dsk, 0);
		amsdos.print_catalog();

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
			&content);
		
		let obtained_result= file.full_content().map(|&b|{b}).collect::<Vec<_>>();
		assert_eq!(
			obtained_result.len(),
			result.len()
		);
		assert_eq!(
			obtained_result,
			result.to_vec()
		);



}
extern crate cpc;

#[cfg(test)]
mod tests {

	#[test]
	fn list_catalog() {
		let dsk = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();
		let amsdos = cpc::disc::amsdos::AmsdosManager::new(dsk, 0);
		amsdos.print_catalog();

	}
}
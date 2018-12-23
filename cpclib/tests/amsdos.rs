

#[cfg(test)]
mod tests {

	#[test]
	fn list_catalog() {
		let dsk = cpclib::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();
		let amsdos = cpclib::disc::amsdos::AmsdosManager::new(dsk, 0);
		amsdos.print_catalog();

	}
}
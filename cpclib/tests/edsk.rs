extern crate cpc;

#[cfg(test)]
mod tests {

	#[test]
	fn open_edsk() {
		let dsk = cpc::disc::edsk::ExtendedDsk::open("pirate.dsk").unwrap();
	}
}
use cpclib_common::camino::Utf8Path;
use ini::Ini;

pub struct AceConfig {
	ini: Ini
}


impl AceConfig {
	pub fn open(path: &Utf8Path) -> Self {
		let ini = Ini::load_from_file(path).unwrap();
		AceConfig { ini }
	}

	pub fn save(&self, path: &Utf8Path) {
		self.ini.write_to_file(path).unwrap();
	}

	pub fn remove(&mut self, key: &str) {
		self.ini.delete_from::<String>(None, key);
	}

	pub fn set(&mut self, key: String, value: String) {
		self.ini.set_to::<String>(None, key, value)
	}
}
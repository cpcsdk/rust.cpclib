use cpclib_common::camino::Utf8Path;
use ini::Ini;

pub struct AceConfig {
	ini: Ini
}


impl AceConfig {
	pub fn open<P: AsRef<Utf8Path>>(path: P) -> Self {
		let ini = Ini::load_from_file(path.as_ref()).unwrap();
		AceConfig { ini }
	}

	pub fn save<P: AsRef<Utf8Path>>(&self, path: P) {
		self.ini.write_to_file(path.as_ref()).unwrap();
	}

	pub fn remove(&mut self, key: &str) {
		self.ini.delete_from::<String>(None, key);
	}

	pub fn set(&mut self, key: String, value: String) {
		self.ini.set_to::<String>(None, key, value)
	}
}
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use ini::Ini;

pub struct AceConfig {
	path: Utf8PathBuf,
	ini: Ini
}


impl AceConfig {
	pub fn open<P: AsRef<Utf8Path>>(path: P) -> Self {
		let p = path.as_ref();
		let ini = Ini::load_from_file(p).unwrap();
		AceConfig { ini, path: p.to_owned() }
	}

	pub fn save_as<P: AsRef<Utf8Path>>(&self, path: P) -> std::io::Result<()> {
		self.ini.write_to_file(path.as_ref())
	}

	pub fn save(&self) -> std::io::Result<()> {
		self.save_as(&self.path)
	}

	pub fn folder(&self) -> &Utf8Path {
		&self.path
	}
	
	pub fn remove<Key: AsRef<str>>(&mut self, key: &Key) {
		self.ini.delete_from::<String>(None, key.as_ref());
	}

	pub fn set<Key: ToString, Value: ToString>(&mut self, key: Key, value: Value) {
		self.ini.set_to::<String>(None, key.to_string(), value.to_string())
	}
}
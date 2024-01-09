use self::ffi::compress_for_basm;

#[cxx::bridge]
mod ffi {
	unsafe extern "C++" {
		include!("cpclib-crunchers/extra/Shrinkler4.6NoParityContext/basm_bridge.h");

		fn compress_for_basm(data: &[u8], iterations: i32, log: bool) ->  Vec<u8>;
	}
}


pub struct ShrinklerConfiguration {
	pub iterations: u8,
	pub log: bool
}

impl Default for ShrinklerConfiguration {
	fn default() -> Self {
		Self {
			iterations: 9,
			log: true
		}
	}
}

impl ShrinklerConfiguration {
	pub fn compress(&self, data: &[u8]) -> Vec<u8> {
		ffi::compress_for_basm(data, self.iterations as i32, self.log)

	}
}

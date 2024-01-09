use lz49::lz49_encode_legacy;
use shrinkler::ShrinklerConfiguration;

#[cfg(not(target_arch = "wasm32"))]
pub mod apultra;
#[cfg(not(target_arch = "wasm32"))]
pub mod exomizer;
#[cfg(not(target_arch = "wasm32"))]
pub mod lz4;
pub mod lz48;
pub mod lz49;
#[cfg(not(target_arch = "wasm32"))]
pub mod zx0;

#[cfg(not(target_arch = "wasm32"))]
pub mod shrinkler;



pub enum CompressMethod {
	Apultra,
	Exomizer,
	Lz4,
	Lz48,
	Lz49,
	Shrinkler(ShrinklerConfiguration),
	Zx0,
}


impl CompressMethod {
	pub fn compress(&self, data: &[u8]) -> Vec<u8> {
		match self {
			CompressMethod::Apultra => {
				apultra::compress(data)
			},
			CompressMethod::Exomizer => {
				exomizer::compress(data)
			},
			CompressMethod::Lz4 => {
				lz4::compress(data)
			},
			CompressMethod::Lz48 => {
				lz48::lz48_encode_legacy(data)
			},
			CompressMethod::Lz49 => {
				lz49_encode_legacy(data)
			},
			CompressMethod::Shrinkler(conf) => {
				conf.compress(data)
			},
			CompressMethod::Zx0 => {
				zx0::compress(data)
			},
		}
	}
}
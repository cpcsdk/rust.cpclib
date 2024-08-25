use lz49::lz49_encode_legacy;
#[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(not(target_arch = "wasm32"))]
    Apultra,
    #[cfg(not(target_arch = "wasm32"))]
    Exomizer,
    #[cfg(not(target_arch = "wasm32"))]
    Lz4,
    Lz48,
    Lz49,
    #[cfg(not(target_arch = "wasm32"))]
    Shrinkler(ShrinklerConfiguration),
    #[cfg(not(target_arch = "wasm32"))]
    Zx0
}

impl CompressMethod {
    pub fn compress(&self, data: &[u8]) -> Vec<u8> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Apultra => apultra::compress(data),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Exomizer => exomizer::compress(data),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Lz4 => lz4::compress(data),
            CompressMethod::Lz48 => lz48::lz48_encode_legacy(data),
            CompressMethod::Lz49 => lz49_encode_legacy(data),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Shrinkler(conf) => conf.compress(data),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Zx0 => zx0::compress(data)
        }
    }
}

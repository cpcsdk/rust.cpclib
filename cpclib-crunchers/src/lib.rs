#![feature(vec_into_raw_parts)]

use lz49::lz49_encode_legacy;
use lzsa::{LzsaMinMatch, LzsaVersion};
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

pub mod lzsa;

pub enum CompressMethod {
    #[cfg(not(target_arch = "wasm32"))]
    Apultra,
    #[cfg(not(target_arch = "wasm32"))]
    Exomizer,
    #[cfg(not(target_arch = "wasm32"))]
    Lz4,
    Lz48,
    Lz49,
    Lzsa(LzsaVersion, Option<LzsaMinMatch>),
    #[cfg(not(target_arch = "wasm32"))]
    Shrinkler(ShrinklerConfiguration),
    #[cfg(not(target_arch = "wasm32"))]
    Upkr,
    #[cfg(not(target_arch = "wasm32"))]
    Zx0
}

impl CompressMethod {
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>, ()> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Apultra => Ok(apultra::compress(data)),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Exomizer => Ok(exomizer::compress(data)),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Lz4 => Ok(lz4::compress(data)),
            CompressMethod::Lz48 => Ok(lz48::lz48_encode_legacy(data)),
            CompressMethod::Lz49 => Ok(lz49_encode_legacy(data)),
            CompressMethod::Lzsa(version, minmatch) => lzsa::compress(data, *version, *minmatch),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Shrinkler(conf) => Ok(conf.compress(data)),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Upkr => {
                let mut config = upkr::Config::default();
                config.use_bitstream = true;
                config.bitstream_is_big_endian = true;
                config.invert_bit_encoding = true;
                config.simplified_prob_update = true;
                let level = 9;
                Ok(upkr::pack(data, level, &config, None))
            },
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Zx0 => Ok(zx0::compress(data))
        }
    }
}

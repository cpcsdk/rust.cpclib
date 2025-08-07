#![feature(vec_into_raw_parts)]

use std::ops::Deref;

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
#[cfg(not(target_arch = "wasm32"))]
pub mod zx7;

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
    Zx0,
    #[cfg(not(target_arch = "wasm32"))]
    Zx7
}

pub struct CompressionResult {
    /// The compressed stream
    pub stream: Vec<u8>,
    /// gap between compressed end and decompressed end
    pub delta: Option<usize>
}

impl AsRef<[u8]> for CompressionResult {
    fn as_ref(&self) -> &[u8] {
        &self.stream
    }
}
impl Deref for CompressionResult {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}
impl From<Vec<u8>> for CompressionResult {
    fn from(value: Vec<u8>) -> Self {
        CompressionResult {
            stream: value,
            delta: None
        }
    }
}

impl From<CompressionResult> for Vec<u8> {
    fn from(val: CompressionResult) -> Self {
        val.stream
    }
}

impl CompressMethod {
    pub fn compress(&self, data: &[u8]) -> Result<CompressionResult, ()> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Apultra => Ok(apultra::compress(data).into()),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Exomizer => Ok(exomizer::compress(data).into()),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Lz4 => Ok(lz4::compress(data).into()),
            CompressMethod::Lz48 => Ok(lz48::lz48_encode_legacy(data).into()),
            CompressMethod::Lz49 => Ok(lz49_encode_legacy(data).into()),
            CompressMethod::Lzsa(version, minmatch) => {
                lzsa::compress(data, *version, *minmatch).map(|r| r.into())
            },
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Shrinkler(conf) => Ok(conf.compress(data).into()),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Upkr => {
                let mut config = upkr::Config::default();
                config.use_bitstream = true;
                config.bitstream_is_big_endian = true;
                config.invert_bit_encoding = true;
                config.simplified_prob_update = true;
                let level = 9;
                Ok(upkr::pack(data, level, &config, None).into())
            },
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Zx0 => Ok(zx0::compress(data)),
            #[cfg(not(target_arch = "wasm32"))]
            CompressMethod::Zx7 => Ok(zx7::compress(data).into())
        }
    }
}

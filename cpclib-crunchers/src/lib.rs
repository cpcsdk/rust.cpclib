use std::ops::Deref;


#[cfg(feature = "lz49")]
use lz49::lz49_encode_legacy;

#[cfg(feature = "lzsa")]
use lzsa::{LzsaMinMatch, LzsaVersion};

#[cfg(all(feature = "shrinkler", not(target_arch = "wasm32")))]
use shrinkler::ShrinklerConfiguration;

#[cfg(all(feature = "apultra", not(target_arch = "wasm32")))]
pub mod apultra;

#[cfg(all(feature = "exomizer", not(target_arch = "wasm32")))]
pub mod exomizer;

#[cfg(all(feature = "lz4", not(target_arch = "wasm32")))]
pub mod lz4;

#[cfg(all(feature = "pucrunch", not(target_arch = "wasm32")))]
pub mod pucrunch;

#[cfg(all(feature = "lz48"))]
pub mod lz48;
#[cfg(all(feature = "lz49"))]
pub mod lz49;
#[cfg(all(feature = "zx0", not(target_arch = "wasm32")))]
pub mod zx0;

#[cfg(all(feature = "shrinkler", not(target_arch = "wasm32")))]
pub mod shrinkler;

#[cfg(all(feature = "zx7", not(target_arch = "wasm32")  ))]
pub mod zx7;

#[cfg(feature = "lzsa")]
pub mod lzsa;

#[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
pub mod bzpack;

pub enum CompressMethod {
    // No compression at all
    None,
    #[cfg(all(feature = "apultra", not(target_arch = "wasm32")))]
    Apultra,
    #[cfg(all(feature = "exomizer", not(target_arch = "wasm32")))]
    Exomizer,
    #[cfg(all(feature = "lz4", not(target_arch = "wasm32")))]
    Lz4,
    #[cfg(all(feature = "lz48"))]
    Lz48,
    #[cfg(all(feature = "lz49"))]
    Lz49,
    #[cfg(all(feature = "lzsa"))]
    Lzsa(LzsaVersion, Option<LzsaMinMatch>),
    #[cfg(all(feature = "shrinkler", not(target_arch = "wasm32")))]
    Shrinkler(ShrinklerConfiguration),
    #[cfg(all(feature = "pucrunch", not(target_arch = "wasm32")))]
    Pucrunch,
    #[cfg(all(feature = "upkr", not(target_arch = "wasm32")))]
    Upkr,
    #[cfg(all(feature = "zx0", not(target_arch = "wasm32")))]
    Zx0,
    #[cfg(all(feature = "zx0", not(target_arch = "wasm32")))]
    BackwardZx0,
    #[cfg(all(feature = "zx7", not(target_arch = "wasm32")))]
    Zx7,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    Lzm,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    BackwardLzm,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    Ef8,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    BackwardEf8,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    Bx0,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    BackwardBx0,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    Bx2,
    #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
    BackwardBx2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug)]
pub enum CrunchersError {
    CompressionFailed
}

impl CompressMethod {
    pub fn compress(&self, data: &[u8]) -> Result<CompressionResult, CrunchersError> {
        match self {
            CompressMethod::None => Ok(data.to_vec().into()),
            #[cfg(all(feature = "apultra", not(target_arch = "wasm32")))]
            CompressMethod::Apultra => Ok(apultra::compress(data).into()),
            #[cfg(all(feature = "exomizer", not(target_arch = "wasm32")))]
            CompressMethod::Exomizer => Ok(exomizer::compress(data).into()),
            #[cfg(all(feature = "lz4", not(target_arch = "wasm32")))]
            CompressMethod::Lz4 => Ok(lz4::compress(data).into()),
            #[cfg(all(feature = "lz48"))]
            CompressMethod::Lz48 => Ok(lz48::lz48_encode_legacy(data).into()),
            #[cfg(all(feature = "lz49"))]
            CompressMethod::Lz49 => Ok(lz49_encode_legacy(data).into()),
            #[cfg(all(feature = "lzsa"))]
            CompressMethod::Lzsa(version, minmatch) => {
                lzsa::compress(data, *version, *minmatch)
                    .map(|r| r.into())
                    .map_err(|_| CrunchersError::CompressionFailed)
            },
            #[cfg(all(feature = "shrinkler", not(target_arch = "wasm32")))]
            CompressMethod::Shrinkler(conf) => Ok(conf.compress(data).into()),
            #[cfg(all(feature = "pucrunch", not(target_arch = "wasm32")))]
            CompressMethod::Pucrunch => pucrunch::compress(data),
            #[cfg(all(feature = "upkr", not(target_arch = "wasm32")))]
            CompressMethod::Upkr => {
                let config = upkr::Config {
                    use_bitstream: true,
                    bitstream_is_big_endian: true,
                    invert_bit_encoding: true,
                    simplified_prob_update: true,
                    ..Default::default()
                };
                let level = 9;
                Ok(upkr::pack(data, level, &config, None).into())
            },
            #[cfg(all(feature = "zx0", not(target_arch = "wasm32")))]
            CompressMethod::Zx0 => Ok(zx0::compress(data)),
            #[cfg(all(feature = "zx0", not(target_arch = "wasm32")))]
            CompressMethod::BackwardZx0 => Ok(zx0::compress_backward(data)),

            #[cfg(all(feature = "zx7", not(target_arch = "wasm32")))]
            CompressMethod::Zx7 => Ok(zx7::compress(data).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::Lzm => Ok(bzpack::compress(data, bzpack::BzpackFormat::Lzm).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::BackwardLzm => Ok(bzpack::compress_backward(data, bzpack::BzpackFormat::Lzm).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::Ef8 => Ok(bzpack::compress(data, bzpack::BzpackFormat::Ef8).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::BackwardEf8 => Ok(bzpack::compress_backward(data, bzpack::BzpackFormat::Ef8).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::Bx0 => Ok(bzpack::compress(data, bzpack::BzpackFormat::Bx0).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::BackwardBx0 => Ok(bzpack::compress_backward(data, bzpack::BzpackFormat::Bx0).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::Bx2 => Ok(bzpack::compress(data, bzpack::BzpackFormat::Bx2).into()),
            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressMethod::BackwardBx2 => Ok(bzpack::compress_backward(data, bzpack::BzpackFormat::Bx2).into()),
        }
    }
}

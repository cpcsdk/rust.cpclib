use cpclib_crunchers::{CompressMethod, CompressionResult};
use cpclib_tokens::CrunchType as CompressionType;

use crate::error::AssemblerError;

pub trait Compressor {
    /// Crunch the raw data with the dedicated algorithm.
    /// Fail when there is no data to crunch
    fn compress(&self, raw: &[u8]) -> Result<CompressionResult, AssemblerError>;
}

impl Compressor for CompressionType {
    fn compress(&self, raw: &[u8]) -> Result<cpclib_crunchers::CompressionResult, AssemblerError> {
        if raw.is_empty() {
            return Err(AssemblerError::NoDataToCrunch);
        }

        let method = match self {
            CompressionType::LZ48 => Ok::<CompressMethod, AssemblerError>(CompressMethod::Lz48),
            CompressionType::LZ49 => Ok(CompressMethod::Lz49),

            CompressionType::LZSA1 => {
                Ok(CompressMethod::Lzsa(
                    cpclib_crunchers::lzsa::LzsaVersion::V1,
                    None
                ))
            },
            CompressionType::LZSA2 => {
                Ok(CompressMethod::Lzsa(
                    cpclib_crunchers::lzsa::LzsaVersion::V2,
                    None
                ))
            },
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::LZ4 => Ok(CompressMethod::Lz4),
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::LZX0 => Ok(CompressMethod::Zx0),
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::LZX7 => Ok(CompressMethod::Zx7),
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::LZEXO => Ok(CompressMethod::Exomizer),
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::LZAPU => Ok(CompressMethod::Apultra),
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::Shrinkler => Ok(CompressMethod::Shrinkler(Default::default())),
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::Upkr => Ok(CompressMethod::Upkr) /* #[cfg(target_arch = "wasm32")]
                                                               * CrunchType::LZ4 => {
                                                               * Err(AssemblerError::AssemblingError {
                                                               * msg: "LZ4 compression not available".to_owned()
                                                               * })
                                                               * },
                                                               * #[cfg(target_arch = "wasm32")]
                                                               * CrunchType::LZX0 => {
                                                               * Err(AssemblerError::AssemblingError {
                                                               * msg: "LZX0 compression not available".to_owned()
                                                               * })
                                                               * },
                                                               * #[cfg(target_arch = "wasm32")]
                                                               * CrunchType::LZEXO => {
                                                               * Err(AssemblerError::AssemblingError {
                                                               * msg: "LZEXO compression not available".to_owned()
                                                               * })
                                                               * },
                                                               * #[cfg(target_arch = "wasm32")]
                                                               * CrunchType::LZAPU => {
                                                               * Err(AssemblerError::AssemblingError {
                                                               * msg: "LZAPU compression not available".to_owned()
                                                               * })
                                                               * }, */
        }?;

        method.compress(raw).map_err(|_| {
            AssemblerError::AssemblingError {
                msg: "Error when crunching".to_string()
            }
        })
    }
}

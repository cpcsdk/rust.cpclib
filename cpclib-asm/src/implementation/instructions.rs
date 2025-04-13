use cpclib_crunchers::CompressMethod;
use cpclib_tokens::CrunchType;

use crate::error::AssemblerError;

pub trait Cruncher {
    /// Crunch the raw data with the dedicated algorithm.
    /// Fail when there is no data to crunch
    fn crunch(&self, raw: &[u8]) -> Result<Vec<u8>, AssemblerError>;
}

impl Cruncher for CrunchType {
    fn crunch(&self, raw: &[u8]) -> Result<Vec<u8>, AssemblerError> {
        if raw.is_empty() {
            return Err(AssemblerError::NoDataToCrunch);
        }

        let method = match self {
            CrunchType::LZ48 => Ok(CompressMethod::Lz48),
            CrunchType::LZ49 => Ok(CompressMethod::Lz49),

            CrunchType::LZSA1 => {
                Ok(CompressMethod::Lzsa(
                    cpclib_crunchers::lzsa::LzsaVersion::V1,
                    None
                ))
            },
            CrunchType::LZSA2 => {
                Ok(CompressMethod::Lzsa(
                    cpclib_crunchers::lzsa::LzsaVersion::V2,
                    None
                ))
            },

            CrunchType::LZX7 => {
                Err(AssemblerError::AssemblingError {
                    msg: "LZX7 compression not implemented".to_owned()
                })
            },
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZ4 => Ok(CompressMethod::Lz4),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZX0 => Ok(CompressMethod::Zx0),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZEXO => Ok(CompressMethod::Exomizer),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZAPU => Ok(CompressMethod::Apultra),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::Shrinkler => Ok(CompressMethod::Shrinkler(Default::default())),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::Upkr => Ok(CompressMethod::Upkr),
            
            /* #[cfg(target_arch = "wasm32")]
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

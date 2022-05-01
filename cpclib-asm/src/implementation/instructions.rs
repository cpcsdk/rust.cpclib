use cpclib_tokens::CrunchType;

use crate::crunchers;
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
        match self {
            CrunchType::LZ48 => Ok(crunchers::lz48::lz48_encode_legacy(raw)),
            CrunchType::LZ49 => Ok(crunchers::lz49::lz49_encode_legacy(raw)),

            CrunchType::LZX7 => {
                Err(AssemblerError::AssemblingError {
                    msg: "LZX7 compression not implemented".to_owned()
                })
            }
            #[cfg(target_arch = "wasm32")]
            CrunchType::LZ4  => {
                Err(AssemblerError::AssemblingError {
                    msg: "LZ4 compression not available".to_owned()
                })
            }
            #[cfg(target_arch = "wasm32")]
            CrunchType::LZX0 => {
                Err(AssemblerError::AssemblingError {
                    msg: "LZX0 compression not available".to_owned()
                })
            }
            #[cfg(target_arch = "wasm32")]
            CrunchType::LZEXO => {
                Err(AssemblerError::AssemblingError {
                    msg: "LZEXO compression not available".to_owned()
                })
            }
            #[cfg(target_arch = "wasm32")]
            CrunchType::LZAPU => {
                Err(AssemblerError::AssemblingError {
                    msg: "LZAPU compression not available".to_owned()
                })
            }

            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZ4  => Ok(crunchers::lz4::compress(raw)),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZX0 => Ok(crunchers::zx0::compress(raw)),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZEXO => Ok(crunchers::exomizer::compress(raw)),
            #[cfg(not(target_arch = "wasm32"))]
            CrunchType::LZAPU => Ok(crunchers::apultra::compress(raw))
        }
    }
}

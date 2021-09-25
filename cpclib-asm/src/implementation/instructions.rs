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
            CrunchType::LZ4 => Err(AssemblerError::AssemblingError {
                msg: "LZ4 compression not implemented".to_owned(),
            }),
            CrunchType::LZX7 => Err(AssemblerError::AssemblingError {
                msg: "LZX7 compression not implemented".to_owned(),
            }),
            CrunchType::LZEXO => Err(AssemblerError::AssemblingError {
                msg: "LZEXO compression not implemented".to_owned(),
            }),
            CrunchType::LZAPU => Ok(crunchers::apultra::compress(raw)),
        }
    }
}

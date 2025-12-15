use std::ops::Deref;

use cpclib_common::smol_str::SmolStr;
use cpclib_crunchers::CompressMethod;
use cpclib_tokens::{CrunchType as CompressionType, Expr};

use crate::Env;
use crate::error::AssemblerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerCompressionResult {
    cruncher_result: cpclib_crunchers::CompressionResult,
    input_len: usize
}

impl AsRef<[u8]> for AssemblerCompressionResult {
    fn as_ref(&self) -> &[u8] {
        self.cruncher_result.as_ref()
    }
}

impl Into<Vec<u8>> for &AssemblerCompressionResult {
    fn into(self) -> Vec<u8> {
        self.as_ref().to_vec()
    }
}

impl AssemblerCompressionResult {
    pub fn new(input: &[u8], cruncher_result: cpclib_crunchers::CompressionResult) -> Self {
        Self {
            cruncher_result,
            input_len: input.len()
        }
    }

    pub fn empty() -> Self {
        Self {
            cruncher_result: cpclib_crunchers::CompressionResult {
                stream: Vec::new(),
                delta: None
            },
            input_len: 0
        }
    }

    pub fn apply_side_effects(&self, env: &mut Env) -> Result<(), AssemblerError> {
        let to_be_set = [
            (
                "BASM_LATEST_CRUNCH_INPUT_DATA_SIZE".to_string(),
                Expr::Value(self.input_len() as _)
            ),
            (
                "BASM_LATEST_CRUNCH_OUTPUT_DATA_SIZE".to_string(),
                Expr::Value(self.compressed_len() as _)
            ),
            (
                "BASM_LATEST_CRUNCH_DELTA_SIZE".to_string(),
                Expr::Value(self.compressed_delta().map(|v| v as i32).unwrap_or(-1)) as _
            )
        ];

        for (name, expr) in to_be_set.into_iter() {
            env.visit_assign(&SmolStr::from(name), &expr, None)?;
        }

        Ok(())
    }
}

impl Deref for AssemblerCompressionResult {
    type Target = cpclib_crunchers::CompressionResult;

    fn deref(&self) -> &Self::Target {
        &self.cruncher_result
    }
}

impl AssemblerCompressionResult {
    pub fn compressed_len(&self) -> usize {
        self.cruncher_result.stream.len()
    }

    pub fn compressed_delta(&self) -> Option<usize> {
        self.cruncher_result.delta
    }

    pub fn input_len(&self) -> usize {
        self.input_len
    }
}

pub trait Compressor {
    /// Crunch the raw data with the dedicated algorithm.
    /// Fail when there is no data to crunch
    fn compress(&self, raw: &[u8]) -> Result<AssemblerCompressionResult, AssemblerError>;
}

impl Compressor for CompressionType {
    fn compress(&self, raw: &[u8]) -> Result<AssemblerCompressionResult, AssemblerError> {
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
            CompressionType::Zx0 => Ok(CompressMethod::Zx0),
            #[cfg(not(target_arch = "wasm32"))]
            CompressionType::BackwardZx0 => Ok(CompressMethod::BackwardZx0),
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

        method
            .compress(raw)
            .map(|res| {
                AssemblerCompressionResult {
                    cruncher_result: res,
                    input_len: raw.len()
                }
            })
            .map_err(|_| {
                AssemblerError::AssemblingError {
                    msg: "Error when crunching".to_string()
                }
            })
    }
}

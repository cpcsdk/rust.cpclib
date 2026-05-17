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

impl From<&AssemblerCompressionResult> for Vec<u8> {
    fn from(val: &AssemblerCompressionResult) -> Self {
        val.as_ref().to_vec()
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

    pub fn apply_side_effects(&self, env: &mut Env) -> Result<(), Box<AssemblerError>> {
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
            env.visit_assign(SmolStr::from(name), &expr, None)?;
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
    fn compress(&self, raw: &[u8]) -> Result<AssemblerCompressionResult, Box<AssemblerError>>;
}

impl Compressor for CompressionType {
    fn compress(&self, raw: &[u8]) -> Result<AssemblerCompressionResult, Box<AssemblerError>> {
        if raw.is_empty() {
            return Err(Box::new(AssemblerError::NoDataToCrunch));
        }

        let method: CompressMethod = match self {
            #[cfg(feature = "lz48")]
            CompressionType::LZ48 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Lz48)
            },
            #[cfg(not(feature = "lz48"))]
            CompressionType::LZ48 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "LZ48 compression not available".to_owned()
                }))
            },

            #[cfg(feature = "lz49")]
            CompressionType::LZ49 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Lz49)
            },
            #[cfg(not(feature = "lz49"))]
            CompressionType::LZ49 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "LZ49 compression not available".to_owned()
                }))
            },

            #[cfg(feature = "lzsa")]
            CompressionType::LZSA1 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Lzsa(
                    cpclib_crunchers::lzsa::LzsaVersion::V1,
                    None
                ))
            },
            #[cfg(feature = "lzsa")]
            CompressionType::LZSA2 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Lzsa(
                    cpclib_crunchers::lzsa::LzsaVersion::V2,
                    None
                ))
            },
            #[cfg(not(feature = "lzsa"))]
            CompressionType::LZSA1 | CompressionType::LZSA2 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "LZSA compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "lz4", not(target_arch = "wasm32")))]
            CompressionType::LZ4 => Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Lz4),
            #[cfg(not(all(feature = "lz4", not(target_arch = "wasm32"))))]
            CompressionType::LZ4 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "LZ4 compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "zx0", not(target_arch = "wasm32")))]
            CompressionType::Zx0 => Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Zx0),
            #[cfg(not(all(feature = "zx0", not(target_arch = "wasm32"))))]
            CompressionType::Zx0 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "zx0 compression not available".to_owned()
                }))
            },
            #[cfg(all(feature = "zx0", not(target_arch = "wasm32")))]
            CompressionType::BackwardZx0 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::BackwardZx0)
            },
            #[cfg(not(all(feature = "zx0", not(target_arch = "wasm32"))))]
            CompressionType::BackwardZx0 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "zx0 compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "zx7", not(target_arch = "wasm32")))]
            CompressionType::LZX7 => Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Zx7),
            #[cfg(not(all(feature = "zx7", not(target_arch = "wasm32"))))]
            CompressionType::LZX7 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "zx7 compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "exomizer", not(target_arch = "wasm32")))]
            CompressionType::LZEXO => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Exomizer)
            },
            #[cfg(not(all(feature = "exomizer", not(target_arch = "wasm32"))))]
            CompressionType::LZEXO => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "exomizer compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "apultra", not(target_arch = "wasm32")))]
            CompressionType::LZAPU => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Apultra)
            },
            #[cfg(not(all(feature = "apultra", not(target_arch = "wasm32"))))]
            CompressionType::LZAPU => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "apultra compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "shrinkler", not(target_arch = "wasm32")))]
            CompressionType::Shrinkler => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Shrinkler(
                    Default::default()
                ))
            },
            #[cfg(not(all(feature = "shrinkler", not(target_arch = "wasm32"))))]
            CompressionType::Shrinkler => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "shrinkler compression not available".to_owned()
                }))
            },
            #[cfg(all(feature = "pucrunch", not(target_arch = "wasm32")))]
            CompressionType::Pucrunch => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Pucrunch)
            },
            #[cfg(not(all(feature = "pucrunch", not(target_arch = "wasm32"))))]
            CompressionType::Pucrunch => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "pucrunch compression not available".to_owned()
                }))
            },
            #[cfg(all(feature = "upkr", not(target_arch = "wasm32")))]
            CompressionType::Upkr => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Upkr)
            },
            #[cfg(not(all(feature = "upkr", not(target_arch = "wasm32"))))]
            CompressionType::Upkr => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "upkr compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BzLzm => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Lzm)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BzLzm => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BackwardBzLzm => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::BackwardLzm)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BackwardBzLzm => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BzEf8 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Ef8)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BzEf8 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BackwardBzEf8 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::BackwardEf8)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BackwardBzEf8 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BzBx0 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Bx0)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BzBx0 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BackwardBzBx0 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::BackwardBx0)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BackwardBzBx0 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BzBx2 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::Bx2)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BzBx2 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },

            #[cfg(all(feature = "bzpack", not(target_arch = "wasm32")))]
            CompressionType::BackwardBzBx2 => {
                Ok::<CompressMethod, Box<AssemblerError>>(CompressMethod::BackwardBx2)
            },
            #[cfg(not(all(feature = "bzpack", not(target_arch = "wasm32"))))]
            CompressionType::BackwardBzBx2 => {
                Err(Box::new(AssemblerError::AssemblingError {
                    msg: "bzpack compression not available".to_owned()
                }))
            },
        }?;

        Ok(method
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
            })?)
    }
}

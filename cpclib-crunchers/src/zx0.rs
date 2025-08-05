use ::zx0;

use crate::CompressionResult;

impl From<zx0::CompressionResult> for CompressionResult {
    fn from(value: zx0::CompressionResult) -> Self {
        CompressionResult {
            stream: value.output,
            delta: Some(value.delta)
        }
    }
}

/// Compress using zx0
/// Returns both the compressed stream and the delta
pub fn compress(data: &[u8]) -> CompressionResult {
    let mut compressor = zx0::Compressor::new();
    compressor
        .backwards_mode(false)
        .classic_mode(false)
        .quick_mode(false);

    let result = compressor.compress(data);
    result.into()
}

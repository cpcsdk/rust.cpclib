/// Bzpack compression formats: LZM, EF8, BX0, BX2
/// These are LZSS-based formats designed for minimal Z80 decoders.
///
/// Format IDs:
/// - 0: LZM  (byte-aligned LZSS)
/// - 1: EF8  (Elias-Gamma length, raw 8-bit offset)
/// - 2: BX0  (Elias length, combined raw/Elias offset or repeat offset)
/// - 3: BX2  (Elias length, raw 8-bit offset or repeat offset)

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cpclib-crunchers/extra/bzpack/bzpack_bridge.h");

        /// Compress data using one of the bzpack formats.
        /// format_id: 0=LZM, 1=EF8, 2=BX0, 3=BX2
        fn bzpack_compress(
            data: &[u8],
            format_id: u8,
            reverse: bool,
            end_marker: bool,
            extend_offset: bool,
            extend_length: bool,
            natural_stream: bool
        ) -> Vec<u8>;
    }
}

/// bzpack format identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BzpackFormat {
    Lzm = 0,
    Ef8 = 1,
    Bx0 = 2,
    Bx2 = 3
}

/// Compress data with a bzpack format (forward direction).
pub fn compress(data: &[u8], format: BzpackFormat) -> Vec<u8> {
    ffi::bzpack_compress(data, format as u8, false, true, false, false, false)
}

/// Compress data with a bzpack format in backward/reverse direction.
/// The resulting stream is designed to be decompressed by the backward Z80 decoder.
pub fn compress_backward(data: &[u8], format: BzpackFormat) -> Vec<u8> {
    ffi::bzpack_compress(data, format as u8, true, true, false, false, false)
}

unsafe extern "C" {
    unsafe fn lzsa_compress_inmem(
        pInputData: *const libc::c_uchar,
        pOutBuffer: *const libc::c_uchar,
        nInputSize: libc::size_t,
        nMaxOutBufferSize: libc::size_t,
        nFlags: libc::c_uint,
        nMinMatchSize: libc::c_uint,
        nFormatVersion: libc::c_uint
    ) -> libc::size_t;
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum LzsaVersion {
    V1 = 1,
    V2 = 2
}

impl LzsaVersion {
    pub fn default_minmatch(&self) -> LzsaMinMatch {
        match self {
            Self::V1 => LzsaMinMatch::Val5,
            Self::V2 => LzsaMinMatch::Val2
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum LzsaMinMatch {
    Val2 = 2,
    Val3 = 3,
    Val4 = 4,
    Val5 = 5
}

#[derive(Debug)]
pub enum LzsaError {
    CompressionFailed
}

pub fn compress(
    data: &[u8],
    version: LzsaVersion,
    minmatch: Option<LzsaMinMatch>
) -> Result<Vec<u8>, LzsaError> {
    let p_input_data = data.as_ptr();

    let _lenout: libc::c_int = 0;

    let compressed_data = Vec::with_capacity(65536);
    let (p_output_data, n_compressed_size, n_max_compressed_size) =
        compressed_data.into_raw_parts();

    debug_assert_eq!(n_compressed_size, 0);

    let n_flags = 1 << 1;
    let minmatch = minmatch.unwrap_or_else(|| version.default_minmatch());

    unsafe {
        let n_compressed_size = lzsa_compress_inmem(
            p_input_data,
            p_output_data,
            data.len() as _,
            n_max_compressed_size,
            n_flags,
            minmatch as _,
            version as _
        );

        if n_compressed_size as isize == -1 {
            Err(LzsaError::CompressionFailed)
        }
        else {
            let mut compressed =
                Vec::from_raw_parts(p_output_data, n_compressed_size, n_max_compressed_size);
            compressed.shrink_to_fit();
            Ok(compressed)
        }
    }
}

use crate::{CompressionResult, CrunchersError};

unsafe extern "C" {
    fn pucrunch_compress(
        input: *const u8,
        input_len: usize,
        output: *mut u8,
        output_len: *mut usize
    ) -> i32;
}

pub fn compress(data: &[u8]) -> Result<CompressionResult, CrunchersError> {
    // Output buffer: input size + 256 (header worst case)
    let mut out = vec![0u8; data.len() + 256];
    let mut out_len: usize = 0;
    let res = unsafe {
        pucrunch_compress(
            data.as_ptr(),
            data.len(),
            out.as_mut_ptr(),
            &mut out_len as *mut usize
        )
    };
    if res == 0 {
        out.truncate(out_len);
        Ok(CompressionResult {
            stream: out,
            delta: None
        })
    }
    else {
        Err(CrunchersError::CompressionFailed)
    }
}

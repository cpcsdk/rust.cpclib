use crate::{CompressionResult, CrunchersError};

unsafe extern "C" {
    fn pucrunch_compress(
        input: *const u8,
        input_len: usize,
        output_cap: usize,
        output: *mut u8,
        output_len: *mut usize
    ) -> i32;
}

/// pucrunch keeps all its state in C globals, so only one call may run at a
/// time (several formats are routinely crunched concurrently by callers).
static PUCRUNCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn compress(data: &[u8]) -> Result<CompressionResult, CrunchersError> {
    // Generous: output can exceed the input (incompressible data plus the
    // header/decompressor); the C side refuses to write past this capacity.
    let capacity = data.len() * 2 + 4096;
    let mut out = vec![0u8; capacity];
    let mut out_len: usize = 0;
    let _guard = PUCRUNCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let res = unsafe {
        pucrunch_compress(
            data.as_ptr(),
            data.len(),
            capacity,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: `LenRle` looped forever for any input with a long run of
    /// one byte, because `maxrlelen` was never initialised.
    #[test]
    fn a_long_run_of_one_byte_terminates() {
        for len in [256usize, 1000, 5000] {
            let out = compress(&vec![0u8; len]).expect("must not fail or hang");
            assert!(out.stream.len() < len, "{len} zeros should shrink, got {}", out.stream.len());
        }
    }

    /// Regression: with the LZ range left at 0 pucrunch fell back to RLE
    /// only, so repetitive non-run data (text) did not compress at all.
    #[test]
    fn repetitive_data_actually_uses_lz_matching() {
        let text: Vec<u8> = b"the quick brown fox jumps over the lazy dog. "
            .iter()
            .copied()
            .cycle()
            .take(4000)
            .collect();
        let out = compress(&text).unwrap();
        assert!(
            out.stream.len() < 1000,
            "4000 bytes of a repeated sentence should pack far below 1000, got {}",
            out.stream.len()
        );
    }

    /// Incompressible data expands; that must be reported as a result, never
    /// written past the output buffer.
    #[test]
    fn incompressible_data_does_not_overflow_the_output_buffer() {
        let mut state = 0x2545F491u32;
        let noise: Vec<u8> = (0..3000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let out = compress(&noise).unwrap();
        assert!(out.stream.len() > 0);
    }

    /// The globals are shared: back-to-back calls with different data must
    /// not depend on each other.
    #[test]
    fn consecutive_calls_are_independent() {
        let a = vec![7u8; 700];
        let first = compress(&a).unwrap().stream;
        let _ = compress(b"unrelated bytes in between, of some length, to disturb state").unwrap();
        assert_eq!(compress(&a).unwrap().stream, first);
    }
}

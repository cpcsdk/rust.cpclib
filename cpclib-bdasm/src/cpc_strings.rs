/// Detect CPC 7-bit strings (last character has bit 7 set)
/// 
/// CPC strings use 7-bit ASCII (0x20-0x7F) with bit 7 set on the last character (0xA0-0xFF)
/// to mark the end of the string.
/// 
/// Returns a vector of (start_offset, length) tuples for detected strings.
pub fn detect_cpc_strings(bytes: &[u8], min_length: usize) -> Vec<(usize, usize)> {
    let mut strings = Vec::new();
    let mut start = None;
    let mut length = 0;
    
    for (i, &byte) in bytes.iter().enumerate() {
        if (0x20..0x80).contains(&byte) {
            // Valid 7-bit printable character
            if start.is_none() {
                start = Some(i);
                length = 1;
            } else {
                length += 1;
            }
        } else if (0xA0..0xFF).contains(&byte) {
            // Character with bit 7 set (potential end marker)
            if start.is_some() {
                length += 1;
                if length >= min_length {
                    strings.push((start.unwrap(), length));
                }
            }
            start = None;
            length = 0;
        } else {
            // Non-printable character, reset
            start = None;
            length = 0;
        }
    }
    
    strings
}

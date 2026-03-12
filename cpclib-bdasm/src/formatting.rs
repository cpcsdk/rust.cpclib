/// Format bytes as hex comment like mdz80 -d option
/// Format: "; ADDRESS HEX_BYTES ASCII"
/// 
/// TODO: Integrate into output when --comments flag is used
#[allow(dead_code)]
pub fn format_hex_comment(address: u16, bytes: &[u8]) -> String {
    let hex_part: String = bytes.iter()
        .map(|b| format!("{:02X} ", b))
        .collect::<String>()
        .trim_end()
        .to_string();
    
    let ascii_part: String = bytes.iter()
        .map(|&b| if b >= 0x20 && b < 0x7F { b as char } else { '.' })
        .collect();
    
    format!("; {:04X} {} {}", address, hex_part, ascii_part)
}

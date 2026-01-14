//! Orgams binary format structures and parsing
//!
//! The Orgams format is a preprocessed Z80 assembly format with this structure:
//! - Magic bytes: "ORGA"
//! - Version byte
//! - Metadata table (fixed size, purpose TBD)
//! - Content sections with various markers:
//!   - 0x43 ('C'): Comment or text line
//!   - 0x49 ('I'): Indented comment/code
//!   - 0x4A ('J'): New line/continuation
//!   - 0x64 ('d'): Data/directive
//!   - 0x41 ('A'): Assembly instruction  
//!   - "SRC" + metadata: Source section marker

use std::io::{self, Read, Write};

/// Magic bytes at the start of Orgams files: "ORGA"
pub const MAGIC: &[u8; 4] = b"ORGA";

/// Orgams file structure
#[derive(Debug, Clone, PartialEq)]
pub struct OrgamsFile {
    /// Header information
    pub header: OrgamsHeader,
    /// Raw content after header (to be parsed into sections)
    pub content: Vec<u8>,
    /// String table extracted from end of file
    pub string_table: Vec<String>,
}

/// Orgams file header
#[derive(Debug, Clone, PartialEq)]
pub struct OrgamsHeader {
    /// Version byte (typically 0x02)
    pub version: u8,
    /// Metadata table (fixed-size, appears to be offsets or configuration)
    pub metadata: Vec<u8>,
}

/// Line marker types found in Orgams files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LineMarker {
    /// Comment or text line (0x43 'C')
    Comment = 0x43,
    /// Indented comment or code (0x49 'I')
    Indented = 0x49,
    /// New line or continuation (0x4A 'J')
    NewLine = 0x4A,
    /// Data or directive (0x64 'd')
    Data = 0x64,
    /// Assembly instruction (0x41 'A')
    Assembly = 0x41,
}

impl LineMarker {
    /// Try to convert a byte to a LineMarker
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x43 => Some(Self::Comment),
            0x49 => Some(Self::Indented),
            0x4A => Some(Self::NewLine),
            0x64 => Some(Self::Data),
            0x41 => Some(Self::Assembly),
            _ => None,
        }
    }

    /// Convert to byte representation
    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

impl OrgamsFile {
    /// Create a new empty Orgams file
    pub fn new() -> Self {
        Self {
            header: OrgamsHeader {
                version: 2,
                metadata: vec![0; 98], // Standard metadata size
            },
            content: vec![],
            string_table: vec![],
        }
    }

    /// Read an Orgams file from a reader
    pub fn read<R: Read>(mut reader: R) -> io::Result<Self> {
        // Read magic bytes
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        
        if &magic != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid magic bytes: expected ORGA, got {:?}", 
                    String::from_utf8_lossy(&magic))
            ));
        }

        // Read version
        let mut version = [0u8; 1];
        reader.read_exact(&mut version)?;

        // Read all remaining data
        let mut all_data = Vec::new();
        reader.read_to_end(&mut all_data)?;

        // Find the SRC marker to determine metadata size
        let src_pos = all_data.windows(3)
            .position(|w| w == b"SRC")
            .unwrap_or(98); // Default to 98 if not found

        let metadata = all_data[..src_pos].to_vec();
        let mut content = all_data[src_pos + 3..].to_vec(); // Skip "SRC"
        
        // Skip section header after SRC (typically starts with 0x63 0x02)
        // Find the first valid marker to determine where actual content starts
        if !content.is_empty() && content[0] == 0x63 {
            // Find first occurrence of a known marker
            // Note: From 0x40 to 0x7F can be literal bytes
            let first_marker = content.iter().position(|&b| {
                matches!(b, 0x40 | 0x43 | 0x49 | 0x4a | 0x64 | 0x6d | 0x7f)
            });
            
            if let Some(pos) = first_marker {
                // Skip the header bytes before the first marker
                content = content[pos..].to_vec();
            }
        }
        
        // Parse string table from the end if present
        let (content, string_table) = Self::extract_string_table(content);

        Ok(Self {
            header: OrgamsHeader {
                version: version[0],
                metadata,
            },
            content,
            string_table,
        })
    }
    
    /// Extract string table from end of content
    /// Strings are separated by bytes >= 0xE0 which also serve as indices
    fn extract_string_table(content: Vec<u8>) -> (Vec<u8>, Vec<String>) {
        let mut strings = Vec::new();
        
        // Find where string table starts - look for consecutive high bytes (>= 0xE0)
        // and ASCII text patterns at the end
        let mut table_start = content.len();
        
        // Scan backwards to find start of string table
        for i in (0..content.len()).rev() {
            if content[i] < 0x20 {
                // Found non-printable before ASCII - likely table start
                table_start = i + 1;
                break;
            }
        }
        
        // If we found a table, parse it
        if table_start < content.len() {
            let table_data = &content[table_start..];
            let mut pos = 0;
            
            while pos < table_data.len() {
                let byte = table_data[pos];
                
                // String separator (>= 0xE0) or string start
                if byte >= 0xE0 {
                    pos += 1;
                    continue;
                }
                
                // Collect ASCII string
                let start = pos;
                while pos < table_data.len() && table_data[pos] < 0xE0 && table_data[pos] >= 0x20 {
                    pos += 1;
                }
                
                if pos > start {
                    let string = String::from_utf8_lossy(&table_data[start..pos]).to_string();
                    strings.push(string);
                }
            }
        }
        
        // Return content without string table and the parsed strings
        if strings.is_empty() {
            (content, strings)
        } else {
            (content[..table_start].to_vec(), strings)
        }
    }

    /// Write an Orgams file to a writer
    pub fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
        // Write magic
        writer.write_all(MAGIC)?;
        
        // Write version
        writer.write_all(&[self.header.version])?;
        
        // Write metadata
        writer.write_all(&self.header.metadata)?;
        
        // Write content
        writer.write_all(&self.content)?;
        
        Ok(())
    }

    /// Extract text lines from the content
    /// Returns a vector of (marker, text) pairs
    ///
    /// Format: Each line is encoded as: [marker_byte] [length_byte] [text_bytes...]
    /// where length_byte is the number of text bytes following
    pub fn extract_lines(&self) -> Vec<(Option<LineMarker>, String)> {
        let mut lines = Vec::new();
        let mut i = 0;
        let data = &self.content;

        while i < data.len() - 1 {
            let potential_marker = data[i];
            let marker = LineMarker::from_byte(potential_marker);
            
            if marker.is_some() && i + 1 < data.len() {
                // Valid marker found, next byte is length
                let length = data[i + 1] as usize;
                
                if i + 2 + length <= data.len() {
                    let text_bytes = &data[i + 2..i + 2 + length];
                    let text = String::from_utf8_lossy(text_bytes).to_string();
                    
                    lines.push((marker, text));
                    i += 2 + length; // Skip marker + length + text
                } else {
                    // Length extends beyond buffer, skip
                    i += 1;
                }
            } else {
                // Not a valid line marker, skip this byte
                i += 1;
            }
        }

        lines
    }

    /// Convert to Z80 assembly text (best effort reconstruction)
    pub fn to_z80_text(&self) -> String {
        let lines = self.extract_lines();
        let mut output = String::new();

        for (marker, text) in lines {
            match marker {
                Some(LineMarker::Comment) => {
                    output.push_str("; ");
                    output.push_str(&text);
                    output.push('\n');
                }
                Some(LineMarker::Indented) => {
                    output.push_str("    ");
                    output.push_str(&text);
                    output.push('\n');
                }
                Some(LineMarker::NewLine) => {
                    output.push('\n');
                }
                Some(LineMarker::Data) | Some(LineMarker::Assembly) => {
                    output.push_str(&text);
                    output.push('\n');
                }
                None => {
                    output.push_str(&text);
                    output.push('\n');
                }
            }
        }

        output
    }
}

impl Default for OrgamsFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_line_marker_conversion() {
        assert_eq!(LineMarker::from_byte(0x43), Some(LineMarker::Comment));
        assert_eq!(LineMarker::from_byte(0x49), Some(LineMarker::Indented));
        assert_eq!(LineMarker::from_byte(0x4A), Some(LineMarker::NewLine));
        assert_eq!(LineMarker::from_byte(0xFF), None);
    }

    #[test]
    fn test_create_new_file() {
        let file = OrgamsFile::new();
        assert_eq!(file.header.version, 2);
        assert_eq!(file.header.metadata.len(), 98);
    }

    #[test]
    fn test_write_and_read_roundtrip() {
        let mut file = OrgamsFile::new();
        file.content = b"SRCtest".to_vec();

        let mut buffer = Vec::new();
        file.write(&mut buffer).unwrap();

        let read_file = OrgamsFile::read(Cursor::new(&buffer)).unwrap();
        assert_eq!(file.header.version, read_file.header.version);
    }
}

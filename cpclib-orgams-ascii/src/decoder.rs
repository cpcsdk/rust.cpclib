/// Decoder for Orgams binary format to Z80 assembly source
/// 
/// This module handles the complex structure of Orgams files which contain:
/// - Text strings (comments, labels, etc.)
/// - Pre-assembled Z80 instructions (opcodes)
/// - Assembler directives (IF/ELSE/END, IMPORT, ORG, etc.)
/// - Mixed encoding with nested structures

use std::io::{self, Read};

// Label index constants from PARSE.Z80
const SHORT_LABEL: u16 = 0x60;  // de &60 a &df : 128 first labels
const LONG_LABEL: u16 = 0xE0;   // from &E000 to &ffff : 8192 other labels

// Expression encoding constants from PARSE.Z80
const E_STRING: u8 = 0x22;        // String marker
const E_ENDOFDATA: u8 = 0x41;     // 'A' - End of data/expression marker
const E_PC: u8 = 0x24;            // '$' - Program counter
const E_OBJC: u8 = 0x44;          // 'D' - Object counter ($$)

/// Expression encoding types from PARSE.Z80
/// These define how numeric values and complex expressions are encoded in .O files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExpressionType {
    /// Short decimal value (0-31) - encoded directly
    ShortDecimal(u8),
    
    /// Space character
    Space = 0x20,
    /// Plus operator
    Plus = 0x2b,
    /// Minus operator
    Minus = 0x2d,
    /// Multiply operator
    Multiply = 0x2a,
    /// Divide operator
    Divide = 0x2f,
    /// Modulo operator
    Modulo = 0x25,
    /// Open parenthesis
    ParenOpen = 0x28,
    /// Close parenthesis
    ParenClose = 0x29,
    
    /// 8-bit decimal (e_decimal_8)
    Decimal8 = 0x30,
    /// 16-bit decimal (e_decimal_16)
    Decimal16 = 0x31,
    /// Long decimal (e_decimal_long)
    DecimalLong = 0x32,
    /// Custom decimal format (e_decimal_custom)
    DecimalCustom = 0x33,
    
    /// 8-bit hexadecimal (e_hexa_8)
    Hexa8 = 0x34,
    /// 16-bit hexadecimal (e_hexa_16)
    Hexa16 = 0x35,
    /// Long hexadecimal (e_hexa_long)
    HexaLong = 0x36,
    /// Custom hexadecimal format (e_hexa_custom)
    HexaCustom = 0x37,
    
    /// 8-bit binary (e_binary_8)
    Binary8 = 0x38,
    /// 16-bit binary (e_binary_16)
    Binary16 = 0x39,
    /// Long binary (e_binary_long)
    BinaryLong = 0x3a,
    /// Custom binary format (e_binary_custom)
    BinaryCustom = 0x3b,
    
    /// Begin multi-term expression (e_begin = 'B')
    Begin = 0x42,
    /// End multi-term expression (e_end = 'E')
    End = 0x45,
    
    /// Label reference (SHORT_LABEL-0xFF)
    LabelRef(u8),
    
    /// Unknown/unsupported expression type
    Unknown(u8),
}

impl From<u8> for ExpressionType {
    fn from(byte: u8) -> Self {
        match byte {
            0x00..=0x1f => ExpressionType::ShortDecimal(byte),
            0x20 => ExpressionType::Space,
            0x25 => ExpressionType::Modulo,
            0x28 => ExpressionType::ParenOpen,
            0x29 => ExpressionType::ParenClose,
            0x2a => ExpressionType::Multiply,
            0x2b => ExpressionType::Plus,
            0x2d => ExpressionType::Minus,
            0x2f => ExpressionType::Divide,
            0x30 => ExpressionType::Decimal8,
            0x31 => ExpressionType::Decimal16,
            0x32 => ExpressionType::DecimalLong,
            0x33 => ExpressionType::DecimalCustom,
            0x34 => ExpressionType::Hexa8,
            0x35 => ExpressionType::Hexa16,
            0x36 => ExpressionType::HexaLong,
            0x37 => ExpressionType::HexaCustom,
            0x38 => ExpressionType::Binary8,
            0x39 => ExpressionType::Binary16,
            0x3a => ExpressionType::BinaryLong,
            0x3b => ExpressionType::BinaryCustom,
            0x42 => ExpressionType::Begin,
            0x45 => ExpressionType::End,
            0x60..=0xff => ExpressionType::LabelRef(byte),
            _ => ExpressionType::Unknown(byte),
        }
    }
}

/// Marker bytes that structure the Orgams binary format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Marker {
    /// Assembly code marker (0x41)
    Assembly = 0x41,
    /// Comment marker (0x43) - followed by length or string ref
    Comment = 0x43,
    /// Indentation marker (0x49) - followed by space count
    Indentation = 0x49,
    /// Newline marker (0x4a)
    NewLine = 0x4a,
    /// Assignment/Data marker (0x64, 'aequ' in DISA.Z80)
    Assignment = 0x64,
    /// Macro definition marker (0x6d, 'amac' in DISA.Z80)
    MacroDefinition = 0x6d,
    /// Command escape marker (0x7f, 'aesc' in DISA.Z80)
    CommandEscape = 0x7f,
    /// Unknown marker
    Unknown(u8),
}

impl From<u8> for Marker {
    fn from(byte: u8) -> Self {
        match byte {
            0x41 => Marker::Assembly,
            0x43 => Marker::Comment,
            0x49 => Marker::Indentation,
            0x4a => Marker::NewLine,
            0x64 => Marker::Assignment,
            0x6d => Marker::MacroDefinition,
            0x7f => Marker::CommandEscape,
            other => Marker::Unknown(other),
        }
    }
}

impl Marker {
    pub fn as_u8(&self) -> u8 {
        match self {
            Marker::Assembly => 0x41,
            Marker::Comment => 0x43,
            Marker::Indentation => 0x49,
            Marker::NewLine => 0x4a,
            Marker::Assignment => 0x64,
            Marker::MacroDefinition => 0x6d,
            Marker::CommandEscape => 0x7f,
            Marker::Unknown(b) => *b,
        }
    }
}

/// Command codes found after 0x7f marker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Unknown command type
    Unknown(u8),
    /// IF directive (0x09)
    If = 0x09,
    /// ELSE directive (0x0a)
    Else = 0x0a,
    /// END directive (0x0c)
    End = 0x0c,
    /// IMPORT directive (0x17)
    Import = 0x17,
    /// ORG directive (0x01)
    Org = 0x01,
    /// Data/opcode reference (0x03)
    DataRef = 0x03,
    /// String reference (0x06)
    StringRef = 0x06,
    /// Label reference (0x08)
    LabelRef = 0x08,
    /// Expression reference (0x04)
    ExprRef = 0x04,
    /// Symbol reference (0x05)
    SymbolRef = 0x05,
    /// Number reference (0x07)
    NumberRef = 0x07,
    /// Macro definition (0x10)
    MacroDef = 0x10,
    /// ENDM directive - end macro definition (0x14 in 0x7f dispatch)
    Endm = 0x14,
    /// Macro use - call a macro (0x15 in 0x7f dispatch)
    MacroUse = 0x15,
    /// Raw opcode (0x41)
    RawOpcode = 0x41,
    /// Comment marker (0x45)
    CommentMarker = 0x45,
}

impl From<u8> for Command {
    fn from(byte: u8) -> Self {
        match byte {
            0x01 => Command::Org,
            0x03 => Command::DataRef,
            0x04 => Command::ExprRef,
            0x05 => Command::SymbolRef,
            0x06 => Command::StringRef,
            0x07 => Command::NumberRef,
            0x08 => Command::LabelRef,      // EC2_SKIP = 0x08 = SKIP directive
            0x09 => Command::MacroUse,      // 0x09 = unknown, possibly macro-related
            0x0a => Command::Else,
            0x0c => Command::End,           // EC2_END = 0x0C = END directive
            0x10 => Command::MacroDef,
            0x14 => Command::Endm,
            0x15 => Command::If,            // EC2_IF = 0x15 = IF directive
            0x17 => Command::Import,        // EC2_IMPORT = 0x17 = IMPORT directive
            0x41 => Command::RawOpcode,
            0x45 => Command::CommentMarker,
            other => Command::Unknown(other),
        }
    }
}

impl Command {
    pub fn as_str(&self) -> &'static str {
        match self {
            Command::If => "IF",
            Command::Else => "ELSE",
            Command::End => "END",
            Command::Endm => "ENDM",
            Command::MacroUse => "MacroUse",
            Command::Import => "IMPORT",
            Command::Org => "ORG",
            Command::DataRef => "DataRef",
            Command::StringRef => "StringRef",
            Command::LabelRef => "LabelRef",
            Command::ExprRef => "ExprRef",
            Command::SymbolRef => "SymbolRef",
            Command::NumberRef => "NumberRef",
            Command::MacroDef => "MacroDef",
            Command::RawOpcode => "RawOpcode",
            Command::CommentMarker => "CommentMarker",
            Command::Unknown(b) => {
                // Can't return dynamic string in const fn, but this is rare
                "Unknown"
            }
        }
    }
}

/// A decoded element from the Orgams file
#[derive(Debug, Clone)]
pub enum DecodedElement {
    /// Plain text (from Comment/Indented/NewLine markers)
    Text(String),
    /// Assembly command (IF/ELSE/END/IMPORT)
    Command { cmd: Command, args: Vec<u8> },
    /// Z80 instruction (disassembled opcode)
    Instruction { bytes: Vec<u8>, asm: String },
    /// Raw data bytes
    RawData(Vec<u8>),
}

/// Decoder for Orgams content
#[derive(Debug, Clone, Copy, PartialEq)]
enum LineState {
    Empty,           // Start of line, first directive gets 6 spaces
    HasContent,      // Already has content on this line
}

pub struct OrgamsDecoder {
    content: Vec<u8>,
    pos: usize,
    string_table: std::collections::HashMap<u16, String>,
    line_state: LineState,
    column: usize,  // Current column position on line for comment alignment
}

impl OrgamsDecoder {
    /// Helper to push text and update column position
    fn push_text(&mut self, elements: &mut Vec<DecodedElement>, text: String) {
        // Update column tracking (count visible characters, excluding newlines)
        for ch in text.chars() {
            if ch == '\n' {
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
        elements.push(DecodedElement::Text(text));
    }

    /// Extract string table from end of content
    /// Strings are separated by bytes >= 0xE0
    fn extract_string_table(content: &[u8]) -> std::collections::HashMap<u16, String> {
        let mut table = std::collections::HashMap::new();
        
        // Look for start of string table (bytes >= LONG_LABEL acting as separators)
        // Scan backwards from end
        let mut current_string = Vec::new();
        let mut current_idx: Option<u16> = None;
        
        for i in (0..content.len()).rev() {
            let byte = content[i];
            
            if byte as u16 >= LONG_LABEL {
                // This is a string index/separator
                if !current_string.is_empty() {
                    current_string.reverse();
                    if let Ok(s) = String::from_utf8(current_string.clone()) {
                        if let Some(idx) = current_idx {
                            table.insert(idx, s);
                        }
                    }
                    current_string.clear();
                }
                current_idx = Some(byte as u16);
            } else if byte >= 0x20 && byte < 0x7F {
                // Printable ASCII - part of string
                current_string.push(byte);
            } else if byte == 0x7F || (byte >= 0x40 && byte < 0x7F) {
                // Hit a marker - we've gone too far into content
                break;
            }
        }
        
        table
    }
    
    pub fn new(content: Vec<u8>) -> Self {
        // Find string table start ("LBLs" marker) and exclude it from content
        let content_end = content.windows(4)
            .position(|w| w == b"LBLs")
            .unwrap_or(content.len());
        
        let string_table = Self::parse_string_table(&content);
        
        Self {
            content: content[..content_end].to_vec(),
            pos: 0,
            string_table,
            line_state: LineState::Empty,
            column: 0,
        }
    }

    /// Get the string table
    pub fn string_table(&self) -> &std::collections::HashMap<u16, String> {
        &self.string_table
    }

    /// Decode an expression based on PARSE.Z80 encoding
    /// Uses ExpressionType enum for clarity
    fn decode_expression(&self, expr_bytes: &[u8]) -> String {
        if expr_bytes.is_empty() {
            return String::new();
        }
        
        let mut result = String::new();
        let mut i = 0;
        
        while i < expr_bytes.len() {
            let byte = expr_bytes[i];
            i += 1;
            
            let expr_type = ExpressionType::from(byte);
            
            match expr_type {
                ExpressionType::ShortDecimal(value) => {
                    result.push_str(&format!("{}", value));
                }
                
                ExpressionType::Decimal8 => {
                    if i < expr_bytes.len() {
                        result.push_str(&format!("{}", expr_bytes[i]));
                        i += 1;
                    }
                }
                
                ExpressionType::Decimal16 => {
                    if i + 1 < expr_bytes.len() {
                        let value = expr_bytes[i] as u16 | ((expr_bytes[i + 1] as u16) << 8);
                        result.push_str(&format!("{}", value));
                        i += 2;
                    }
                }
                
                ExpressionType::Hexa8 => {
                    if i < expr_bytes.len() {
                        result.push_str(&format!("&{:02X}", expr_bytes[i]));
                        i += 1;
                    }
                }
                
                ExpressionType::Hexa16 => {
                    if i + 1 < expr_bytes.len() {
                        let value = expr_bytes[i] as u16 | ((expr_bytes[i + 1] as u16) << 8);
                        result.push_str(&format!("&{:04X}", value));
                        i += 2;
                    }
                }
                
                ExpressionType::Binary8 => {
                    if i < expr_bytes.len() {
                        result.push_str(&format!("%{:08b}", expr_bytes[i]));
                        i += 1;
                    }
                }
                
                ExpressionType::Binary16 => {
                    if i + 1 < expr_bytes.len() {
                        let value = expr_bytes[i] as u16 | ((expr_bytes[i + 1] as u16) << 8);
                        result.push_str(&format!("%{:016b}", value));
                        i += 2;
                    }
                }
                
                ExpressionType::Begin => {
                    // Recursively decode the expression until End (0x45)
                    let start = i;
                    let mut depth = 1;
                    while i < expr_bytes.len() && depth > 0 {
                        match ExpressionType::from(expr_bytes[i]) {
                            ExpressionType::Begin => depth += 1,
                            ExpressionType::End => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            i += 1;
                        }
                    }
                    
                    if i < expr_bytes.len() {
                        // Decode inner expression
                        let inner = &expr_bytes[start..i];
                        result.push_str(&self.decode_expression(inner));
                        i += 1; // Skip the End marker
                    }
                }
                
                ExpressionType::End => {
                    // Should not happen at top level, but handle gracefully
                }
                
                ExpressionType::Space => result.push(' '),
                ExpressionType::Plus => result.push('+'),
                ExpressionType::Minus => result.push('-'),
                ExpressionType::Multiply => result.push('*'),
                ExpressionType::Divide => result.push('/'),
                ExpressionType::Modulo => result.push_str("MOD"),
                ExpressionType::ParenOpen => result.push('('),
                ExpressionType::ParenClose => result.push(')'),
                
                ExpressionType::LabelRef(index) => {
                    if let Some(label) = self.string_table.get(&(index as u16)) {
                        result.push_str(label);
                    } else {
                        result.push_str(&format!("[ref:0x{:02x}]", index));
                    }
                }
                
                ExpressionType::Unknown(b) => {
                    // Check for special expression markers from PARSE.Z80
                    match b {
                        0x24 => result.push('$'),   // E_PC - program counter
                        0x44 => result.push_str("$$"),  // E_OBJC - object counter
                        _ => result.push_str(&format!("[0x{:02x}]", b)),
                    }
                }
                
                // Handle remaining formats that aren't fully implemented yet
                ExpressionType::DecimalLong
                | ExpressionType::DecimalCustom
                | ExpressionType::HexaLong
                | ExpressionType::HexaCustom
                | ExpressionType::BinaryLong
                | ExpressionType::BinaryCustom => {
                    result.push_str(&format!("[unimpl:0x{:02x}]", byte));
                }
            }
        }
        
        result
    }

    /// Parse the string table from the end of the content
    /// String tables start with "LBLs" marker, followed by 2-byte header,
    /// then packed strings with high-byte encoding:
    /// - Each string is null-terminated or ends with a byte >= 0x80
    /// - Last character in string has bit 7 set (e.g., 0xed = 'm' + 0x80)
    /// - Strings are indexed by position or by explicit index bytes
    fn parse_string_table(data: &[u8]) -> std::collections::HashMap<u16, String> {
        let mut strings = std::collections::HashMap::new();
        
        // Find "LBLs" marker (preceded by 0x00)
        let lbls_pattern = [0x00, b'L', b'B', b'L', b's'];
        if let Some(lbls_pos) = data.windows(5).position(|w| w == &lbls_pattern) {
            // lbls_pos points to the 0x00 before "LBLs"
            // String table layout: 0x00 "LBLs" [count byte] [strings...]
            let count_pos = lbls_pos + 5;
            
            if count_pos >= data.len() {
                return strings;
            }
            
            // Read the count byte - might indicate string count or something else
            let _count_byte = data[count_pos];
            let mut i = count_pos + 1;  // Start after count byte
            
            let mut string_index: u16 = SHORT_LABEL; // Start indexing from SHORT_LABEL per PARSE.Z80
            
            // Parse ALL strings until end of data (not just string_count)
            // This handles files with many labels like CONST.I
            while i < data.len() {
                let mut current_string = String::new();
                let mut hit_null = false;
                
                // Collect characters until we hit a byte with bit 7 set (>= 0x80)
                // That byte is the LAST character of the string (with bit 7 encoding the termination)
                while i < data.len() {
                    let b = data[i];
                    i += 1;
                    
                    if b >= 0x80 {
                        // High bit set - this is the LAST character
                        let actual_char = (b & 0x7f) as char;
                        current_string.push(actual_char);
                        break; // End of this string
                    } else if b >= 0x20 && b < 0x7f {
                        // Regular printable ASCII
                        current_string.push(b as char);
                    } else if b == 0x00 {
                        // Null byte indicates end of string table
                        hit_null = true;
                        break;
                    }
                    // Skip other control characters (< 0x20)
                }
                
                if hit_null {
                    // Stop parsing the string table
                    break;
                }
                
                if !current_string.is_empty() {
                    strings.insert(string_index, current_string);
                }
                string_index = string_index.wrapping_add(1);
            }
        }
        
        strings
    }
    
    /// Decode all elements from the content
    pub fn decode(&mut self) -> io::Result<Vec<DecodedElement>> {
        let mut elements = Vec::new();
        
        while self.pos < self.content.len() {
            let byte = self.content[self.pos];
            
            // Skip mystery control bytes that appear before known markers
            // These seem to be formatting hints or flags that we should ignore
            // Common values: 0x57 ('W'), 0xc2, 0xcb, 0xd1, 0xda, 0xdc
            if self.pos + 1 < self.content.len() {
                let next_byte = self.content[self.pos + 1];
                // If current byte looks like a control byte and next is a known marker, skip it
                if matches!(next_byte, 0x43 | 0x49 | 0x4a | 0x64 | 0x6d | 0x7f) {
                    // Skip bytes that don't fit normal patterns
                    if byte == 0x57 || (byte >= 0xc0 && byte < 0xe0) {
                        self.pos += 1;
                        continue;
                    }
                }
            }
            
            // NewLine marker (0x4a) - standalone, no length byte
            if byte == 0x4a {
                self.pos += 1;
                elements.push(DecodedElement::Text("\n".to_string()));
                self.line_state = LineState::Empty; // Reset for next line
                self.column = 0;
                continue;
            }
            
            // Label address marker (0x40) - followed by address bytes
            // But 0x40 is also '@' ASCII, so be careful
            // For now, treat 0x40 as ASCII unless followed by specific patterns
            
            // Comment (0x43) or Tab (0x49) markers - HAVE length+text
            if matches!(byte, 0x43 | 0x49) {
                if let Some(element) = self.decode_text_marker(byte)? {
                    // Comments terminate the line - append newline
                    if byte == 0x43 {
                        if let DecodedElement::Text(mut text) = element {
                            // If comment comes after content, align to column 24
                            const COMMENT_COLUMN: usize = 24;
                            if self.line_state == LineState::HasContent {
                                if self.column < COMMENT_COLUMN {
                                    let padding = " ".repeat(COMMENT_COLUMN - self.column);
                                    text.insert_str(0, &padding);
                                } else {
                                    // Already past column 24, just add one space
                                    text.insert_str(0, " ");
                                }
                            }
                            text.push('\n');
                            elements.push(DecodedElement::Text(text));
                            self.line_state = LineState::Empty; // Reset for next line
                            self.column = 0;
                        } else {
                            elements.push(element);
                        }
                    } else {
                        elements.push(element);
                    }
                }
                continue;
            }
            
            // Data marker (0x64) - aequ (assignment marker)
            // Format per DISA.Z80: 64 [label_index] [expr_size] [expr_bytes...]
            // Label indices: SHORT_LABEL-0xDF (short, 1 byte), LONG_LABEL-0xFF (long, 2 bytes)
            if byte == 0x64 {
                self.pos += 1;
                if self.pos >= self.content.len() {
                    break;
                }
                
                // Read label index (may be 1 or 2 bytes)
                let first_byte = self.content[self.pos];
                self.pos += 1;
                
                // From PARSE.Z80 fie_big analysis and empirical verification:
                // - Short labels (SHORT_LABEL-0xDF): single byte = index
                // - Long labels (LONG_LABEL-0xFF first byte): index = first_byte + second_byte
                // Example: E0 82 → index = 0xE0 + 0x82 = 0x162
                let label_index: u16 = if (first_byte as u16) >= LONG_LABEL {
                    // Long index: add both bytes
                    if self.pos >= self.content.len() {
                        break;
                    }
                    let second_byte = self.content[self.pos];
                    self.pos += 1;
                    (first_byte as u16) + (second_byte as u16)
                } else {
                    first_byte as u16
                };
                
                // Labels start at SHORT_LABEL per PARSE.Z80
                if label_index >= SHORT_LABEL {
                    // It's a label reference - decode from string table
                    let var_name = self.string_table.get(&label_index)
                        .cloned()
                        .unwrap_or_else(|| format!("[label:0x{:02x}]", label_index));
                    
                    // Next byte is expression size
                    if self.pos >= self.content.len() {
                        elements.push(DecodedElement::Text(format!("{} = ", var_name)));
                        continue;
                    }
                    
                    let expr_size = self.content[self.pos] as usize;
                    self.pos += 1;
                    
                    // Read expression bytes
                    let expr_bytes: Vec<u8> = (0..expr_size)
                        .filter_map(|_| {
                            if self.pos < self.content.len() {
                                let b = self.content[self.pos];
                                self.pos += 1;
                                Some(b)
                            } else {
                                None
                            }
                        })
                        .collect();
                    
                    // Decode expression based on PARSE.Z80 encoding
                    let value_str = self.decode_expression(&expr_bytes);
                    
                    // Format assignment with = aligned to minimum column 6
                    // For short names: "spc   = value" (spc + 3 spaces + = + space + value)
                    // For long names: "max_sources = value" (name + space + = + space + value)
                    const MIN_NAME_LEN: usize = 5;  // "spc" (3) needs 2 more to reach position 5, then space + =
                    let spaces_before_equals = if var_name.len() <= MIN_NAME_LEN {
                        MIN_NAME_LEN - var_name.len() + 1  // Pad to MIN_NAME_LEN, then +1 for space before =
                    } else {
                        1  // Just one space before =
                    };
                    let padding = " ".repeat(spaces_before_equals);
                    
                    self.push_text(&mut elements, format!("{}{}= {}", var_name, padding, value_str));
                    self.line_state = LineState::HasContent;
                } else {
                    // Not a string ref - treat as length byte for data section
                    let length = label_index as usize;
                    let section_end = (self.pos + length).min(self.content.len());
                    
                    // Recursively decode everything inside this Data section
                    while self.pos < section_end {
                        let inner_byte = self.content[self.pos];
                        let inner_marker = Marker::from(inner_byte);
                        
                        // NewLine - standalone
                        if matches!(inner_marker, Marker::NewLine) {
                            self.pos += 1;
                            elements.push(DecodedElement::Text("\n".to_string()));
                            continue;
                        }
                    
                        // Assembly - standalone
                        if matches!(inner_marker, Marker::Assembly) {
                            self.pos += 1;
                            continue;
                        }
                        
                        // Comment/Indented - have length+text
                        if matches!(inner_marker, Marker::Comment | Marker::Indentation) {
                            if let Some(elem) = self.decode_text_marker(inner_byte)? {
                                elements.push(elem);
                            }
                            continue;
                        }
                        
                        // Command marker
                        if inner_byte == 0x7f {
                            if let Some(elem) = self.decode_command()? {
                                elements.push(elem);
                            }
                            continue;
                        }
                        
                        // Unknown byte - skip
                        self.pos += 1;
                    }
                }
                continue;
            }
            
            // Macro definition marker (0x6d, 'amac') - outputs "      MACRO " + name + params
            if byte == 0x6d {
                self.pos += 1;
                // Skip the next byte (seems to be a count or spacing indicator)
                if self.pos < self.content.len() {
                    self.pos += 1;
                }
                // Output "      MACRO " prefix (6 spaces + MACRO + space) - always first on line
                self.push_text(&mut elements, "      MACRO ".to_string());
                self.line_state = LineState::HasContent;
                // Continue parsing - next bytes are macro name and parameters
                continue;
            }
            
            // Command marker (0x7f)
            if byte == 0x7f {
                if let Some(element) = self.decode_command()? {
                    // For IF command, convert to text immediately 
                    if let DecodedElement::Command { cmd: Command::If, args } = &element {
                        // Decode IF condition from args using decode_expression
                        // Remove E_ENDOFDATA if present
                        let expr_bytes: Vec<u8> = args.iter()
                            .copied()
                            .take_while(|&b| b != E_ENDOFDATA)
                            .collect();
                        
                        let condition = self.decode_expression(&expr_bytes);
                        self.push_text(&mut elements, format!("      IF {}", condition));
                        self.line_state = LineState::HasContent;
                    } 
                    // For IMPORT command, convert to text immediately
                    else if let DecodedElement::Command { cmd: Command::Import, args } = &element {
                        let filename = String::from_utf8_lossy(args);
                        let import_text = format!("      IMPORT \"{}\"", filename);
                        self.push_text(&mut elements, import_text);
                        self.line_state = LineState::HasContent;
                    }
                    // For DataRef command with 0x64 assignment marker, decode to text
                    else if let DecodedElement::Command { cmd: Command::DataRef, args } = &element {
                        if !args.is_empty() && args[0] == 0x64 {
                            let mut i = 1;
                            
                            // Read label index (1 or 2 bytes)
                            if i >= args.len() {
                                elements.push(element);
                                continue;
                            }
                            
                            let first_byte = args[i];
                            i += 1;
                            
                            let label_index: u16 = if (first_byte as u16) >= LONG_LABEL {
                                // Long index: add both bytes
                                if i >= args.len() {
                                    elements.push(element);
                                    continue;
                                }
                                let second_byte = args[i];
                                i += 1;
                                (first_byte as u16) + (second_byte as u16)
                            } else {
                                first_byte as u16
                            };
                            
                            // Get variable name from string table
                            let var_name = self.string_table.get(&label_index)
                                .cloned()
                                .unwrap_or_else(|| format!("[label:0x{:02x}]", label_index));
                            
                            // Read expression size
                            if i >= args.len() {
                                self.push_text(&mut elements, format!("{} = ", var_name));
                                self.line_state = LineState::HasContent;
                                continue;
                            }
                            
                            let expr_size = args[i] as usize;
                            i += 1;
                            
                            // Decode expression bytes
                            let expr_bytes = &args[i..i+expr_size.min(args.len()-i)];
                            let value_str = self.decode_expression(expr_bytes);
                            
                            // Format assignment with aligned =
                            const MIN_NAME_LEN: usize = 5;
                            let spaces_before_equals = if var_name.len() <= MIN_NAME_LEN {
                                MIN_NAME_LEN - var_name.len() + 1
                            } else {
                                1
                            };
                            let padding = " ".repeat(spaces_before_equals);
                            
                            self.push_text(&mut elements, format!("{}{}= {}", var_name, padding, value_str));
                            self.line_state = LineState::HasContent;
                        } else {
                            // Other DataRef usage - keep as command
                            elements.push(element);
                        }
                    }
                    // For ORG command with text args (byte count followed by literal text), convert to text
                    else if let DecodedElement::Command { cmd: Command::Org, args } = &element {
                        if args.len() > 2 {
                            // Text mode: args contains the literal text bytes (preserves original spacing)
                            let text: String = args.iter().map(|&b| b as char).collect();
                            self.push_text(&mut elements, text);
                            self.line_state = LineState::HasContent;
                        } else {
                            // Address mode: keep as command for normal ORG processing
                            elements.push(element);
                        }
                    }
                    // For END and ENDM commands with no args, add indentation if first on line
                    else if let DecodedElement::Command { cmd, args } = &element {
                        if matches!(cmd, Command::End | Command::Endm) && args.is_empty() {
                            let cmd_name = cmd.as_str();
                            let prefix = if self.line_state == LineState::Empty { "      " } else { " " };
                            self.push_text(&mut elements, format!("{}{}", prefix, cmd_name));
                            self.line_state = LineState::HasContent;
                        } else {
                            elements.push(element);
                        }
                    } else {
                        elements.push(element);
                    }
                }
                continue;
            }
            
            // Assembly marker (0x41) - acts as separator, outputs space unless followed by newline
            if byte == 0x41 {
                self.pos += 1;
                // Don't add space if next byte is newline
                if self.pos < self.content.len() && self.content[self.pos] != 0x4a {
                    self.push_text(&mut elements, " ".to_string());
                }
                continue;
            }
            
            // Collect raw ASCII text between markers
            if (0x20..0x7f).contains(&byte) || byte >= 0xC0 {
                let mut text = String::new();
                
                while self.pos < self.content.len() {
                    let b = self.content[self.pos];
                    
                    // Stop at confirmed markers:
                    // 0x41 (Assembly), 0x43 (Comment), 0x49 (Tab), 0x4A (NewLine), 0x64 (Data), 0x6D (Macro), 0x7F (Command)
                    if matches!(b, 0x41 | 0x43 | 0x49 | 0x4a | 0x64 | 0x6d | 0x7f) || b < 0x20 {
                        break;
                    }
                    
                    // String table reference (>= SHORT_LABEL per PARSE.Z80)
                    // SHORT_LABEL-0xDF: short labels, LONG_LABEL-0xFF: long labels
                    if (b as u16) >= SHORT_LABEL {
                        if let Some(s) = self.string_table.get(&(b as u16)) {
                            text.push_str(s);
                            // Add space if next byte is also a string ref
                            if self.pos + 1 < self.content.len() && (self.content[self.pos + 1] as u16) >= SHORT_LABEL {
                                text.push(' ');
                            }
                        } else {
                            // Unknown string index - show as hex
                            text.push_str(&format!("[0x{:02x}]", b));
                        }
                        self.pos += 1;
                    }
                    // Normal ASCII text (0x20-0x5F range only, since SHORT_LABEL+ are string refs)
                    else if (0x20..(SHORT_LABEL as u8)).contains(&b) {
                        text.push(b as char);
                        self.pos += 1;
                    }
                    else {
                        break;
                    }
                }
                
                if !text.trim().is_empty() {
                    self.push_text(&mut elements, text);
                }
                continue;
            }
            
            // Unknown byte - skip it
            self.pos += 1;
        }
        
        Ok(elements)
    }
    
    /// Decode a text marker (Comment/Indented/NewLine/Assembly)
    fn decode_text_marker(&mut self, marker_byte: u8) -> io::Result<Option<DecodedElement>> {
        self.pos += 1;  // Skip marker
        
        if self.pos >= self.content.len() {
            return Ok(None);
        }
        
        let marker = Marker::from(marker_byte);
        let next_byte = self.content[self.pos];
        
        // Handle indentation marker specially - next byte is space count
        if matches!(marker, Marker::Indentation) {
            let space_count = next_byte as usize;
            self.pos += 1;
            let spaces = " ".repeat(space_count);
            return Ok(Some(DecodedElement::Text(spaces)));
        }
        
        // For other markers (0x43 Comment, 0x41 Assembly):
        // Disambiguate between string table reference and length-prefixed text:
        // - If next_byte < 0x60, likely a length (typical comment lengths are 1-95)
        // - If next_byte >= 0x60, try string table first (label reference range)
        let text = if next_byte >= 0x60 && self.string_table.contains_key(&(next_byte as u16)) {
            // Likely a string table reference (in label range and exists in table)
            self.pos += 1;
            self.string_table.get(&(next_byte as u16)).unwrap().clone()
        } else {
            // Not in string table - treat as length-prefixed text
            let length = next_byte as usize;
            self.pos += 1;
            
            let end_pos = (self.pos + length).min(self.content.len());
            let text_bytes = &self.content[self.pos..end_pos];
            self.pos = end_pos;
            String::from_utf8_lossy(text_bytes).to_string()
        };
        
        // Format based on marker type
        let formatted = match marker {
            Marker::Comment => format!(";{}", text),  // Comment - no space after semicolon
            Marker::Indentation => text,  // Indented - text already includes indentation from length-prefixed data
            Marker::NewLine => "\n".to_string(),  // NewLine - output actual newline character
            Marker::Assembly => text,  // Assembly
            _ => text,
        };
        
        Ok(Some(DecodedElement::Text(formatted)))
    }
    
    /// Decode a Data section (contains mixed content)
    fn decode_data_section(&mut self) -> io::Result<Option<DecodedElement>> {
        self.pos += 1;  // Skip 0x64 marker
        
        if self.pos >= self.content.len() {
            return Ok(None);
        }
        
        let length = self.content[self.pos] as usize;
        self.pos += 1;
        
        let section_end = (self.pos + length).min(self.content.len());
        
        // Data sections don't return a single element - they are containers
        // Skip through and let decode_next handle sub-elements
        // Just advance past this Data section's length
        self.pos = section_end;
        
        // Return None to continue parsing
        Ok(None)
    }
    
    /// Decode a command (0x7f marker)
    fn decode_command(&mut self) -> io::Result<Option<DecodedElement>> {
        self.pos += 1;  // Skip 0x7f
        
        if self.pos >= self.content.len() {
            return Ok(None);
        }
        
        let cmd_byte = self.content[self.pos];
        self.pos += 1;
        
        let cmd = Command::from(cmd_byte);
        let mut args = Vec::new();
        
        match cmd {
            Command::If => {
                // IF directive: 7F 15 [expr_size] [expression] 41
                // Read expression size
                if self.pos < self.content.len() {
                    let expr_size = self.content[self.pos] as usize;
                    self.pos += 1;
                    
                    // Read expression bytes (expr_size includes E_ENDOFDATA)
                    for _ in 0..expr_size {
                        if self.pos < self.content.len() {
                            args.push(self.content[self.pos]);
                            self.pos += 1;
                        }
                    }
                }
            }
            Command::LabelRef => {
                // SKIP directive: 7F 08 [expr_size] [expression] 41
                // Same format as IF - read expression size and bytes
                if self.pos < self.content.len() {
                    let expr_size = self.content[self.pos] as usize;
                    self.pos += 1;
                    
                    // Read expression bytes
                    for _ in 0..expr_size {
                        if self.pos < self.content.len() {
                            args.push(self.content[self.pos]);
                            self.pos += 1;
                        }
                    }
                }
            }
            Command::End => {
                // END directive: 7F 0C (no parameters)
                // Nothing to read, just the command
            }
            Command::Else => {
                // ELSE: 0x7f 0x0a then 0x4a
                // No arguments needed
            }
            Command::End => {
                // END: 0x7f 0x0c then 0x4a
                // No arguments needed
            }
            Command::Endm => {
                // ENDM: 0x7f 0x14 then 0x4a
                // No arguments needed
            }
            Command::MacroUse => {
                // Macro use: 0x7f 0x15 [macro_name_ref] [args...]
                // Read macro name reference and arguments
                if self.pos < self.content.len() {
                    // First byte is macro name reference
                    args.push(self.content[self.pos]);
                    self.pos += 1;
                    // Read arguments until newline marker
                    while self.pos < self.content.len() {
                        let b = self.content[self.pos];
                        if b == 0x4a { // Newline
                            break;
                        }
                        args.push(b);
                        self.pos += 1;
                    }
                }
            }
            Command::Import => {
                // IMPORT structure per PARSE.Z80 t_import:
                // 0x7f 0x17 [expr_size] E_STRING [str_len] [filename] E_ENDOFDATA
                // Example: 7F 17 0A 22 07 63 6F 6E 73 74 2E 69 41
                if self.pos < self.content.len() {
                    let expr_size = self.content[self.pos];
                    self.pos += 1;
                    
                    // Expect E_STRING marker
                    if self.pos < self.content.len() && self.content[self.pos] == E_STRING {
                        self.pos += 1;
                        
                        // Read string length and filename
                        if self.pos < self.content.len() {
                            let str_len = self.content[self.pos] as usize;
                            self.pos += 1;
                            
                            let end_pos = (self.pos + str_len).min(self.content.len());
                            args.extend_from_slice(&self.content[self.pos..end_pos]);
                            self.pos = end_pos;
                            
                            // Skip E_ENDOFDATA marker if present
                            if self.pos < self.content.len() && self.content[self.pos] == E_ENDOFDATA {
                                self.pos += 1;
                            }
                        }
                    }
                }
            }
            Command::Org => {
                // ORG has two formats:
                // 1. Address: 0x7f 0x01 [low_byte] [high_byte] - forms a 16-bit address
                // 2. Text with count: 0x7f 0x01 [count] [text_bytes...] - count < 0x20 indicates text mode
                
                if self.pos < self.content.len() {
                    let first_byte = self.content[self.pos];
                    
                    if first_byte < 0x20 {
                        // Text mode: first_byte is a count of text bytes to read
                        let count = first_byte as usize;
                        self.pos += 1; // Skip count byte
                        
                        // Read exactly 'count' bytes as text
                        for _ in 0..count {
                            if self.pos < self.content.len() {
                                args.push(self.content[self.pos]);
                                self.pos += 1;
                            }
                        }
                    } else {
                        // Address mode: read 2 bytes for 16-bit address
                        while self.pos < self.content.len() && args.len() < 2 {
                            let b = self.content[self.pos];
                            // Stop at markers (newline, commands, etc.)
                            if b == 0x4a || b == 0x7f || b >= 0x40 && b < 0x60 {
                                break;
                            }
                            args.push(b);
                            self.pos += 1;
                        }
                    }
                }
            }
            Command::DataRef => {
                // DataRef with 0x64 = variable assignment
                // Format: 7F 03 64 [label_idx] [expr_size] [expr_bytes...]
                // We need to read ALL the data for assignment decoding
                if self.pos < self.content.len() && self.content[self.pos] == 0x64 {
                    args.push(0x64); // Assignment marker
                    self.pos += 1;
                    
                    // Read label index (1 or 2 bytes)
                    if self.pos < self.content.len() {
                        let first_byte = self.content[self.pos];
                        args.push(first_byte);
                        self.pos += 1;
                        
                        // Long label?
                        if (first_byte as u16) >= LONG_LABEL && self.pos < self.content.len() {
                            args.push(self.content[self.pos]);
                            self.pos += 1;
                        }
                    }
                    
                    // Read expression size
                    if self.pos < self.content.len() {
                        let expr_size = self.content[self.pos];
                        args.push(expr_size);
                        self.pos += 1;
                        
                        // Read expression bytes
                        for _ in 0..expr_size {
                            if self.pos < self.content.len() {
                                args.push(self.content[self.pos]);
                                self.pos += 1;
                            }
                        }
                    }
                } else {
                    // Not an assignment - just read 1-2 bytes like other ref commands
                    if self.pos < self.content.len() {
                        args.push(self.content[self.pos]);
                        self.pos += 1;
                        
                        if self.pos < self.content.len() {
                            let next = self.content[self.pos];
                            if next >= 0xC0 || (next > 0 && next < 0x20) {
                                args.push(next);
                                self.pos += 1;
                            }
                        }
                    }
                }
            }
            Command::StringRef | Command::LabelRef | Command::ExprRef | 
            Command::SymbolRef | Command::NumberRef => {
                // Reference commands: 0x7f 0x06/0x08/0x04/0x05/0x07 [arg] [string_idx]
                // Read 1-2 argument bytes
                if self.pos < self.content.len() {
                    args.push(self.content[self.pos]);
                    self.pos += 1;
                    
                    // Check for string table index (>= 0xC0)
                    if self.pos < self.content.len() {
                        let next = self.content[self.pos];
                        if next >= 0xC0 || (next > 0 && next < 0x20) {
                            args.push(next);
                            self.pos += 1;
                        }
                    }
                }
            }
            Command::MacroDef => {
                // 0x7f 0x10 - macro definition, might have metadata
                if self.pos < self.content.len() {
                    let next = self.content[self.pos];
                    if next < 0x20 && next != 0x00 {
                        args.push(next);
                        self.pos += 1;
                    }
                }
            }
            Command::MacroDef => {
                // 0x7f 0x10 - macro definition
                // Followed by length and macro name
                if self.pos < self.content.len() {
                    let b = self.content[self.pos];
                    if b < 0x40 {
                        args.push(b);
                        self.pos += 1;
                    }
                }
            }
            _ => {
                // Unknown commands - read a few bytes of arguments carefully
                for _ in 0..4 {
                    if self.pos < self.content.len() {
                        let b = self.content[self.pos];
                        // Stop at markers
                        if matches!(b, 0x43 | 0x49 | 0x4a | 0x64 | 0x7f) || b == 0x41 && self.pos + 1 < self.content.len() {
                            break;
                        }
                        args.push(b);
                        self.pos += 1;
                    }
                }
            }
        }
        
        Ok(Some(DecodedElement::Command { cmd, args }))
    }
}

/// Convert decoded elements to Z80 source text
pub fn elements_to_z80_source(elements: &[DecodedElement]) -> String {
    let mut output = String::new();
    let mut at_line_start = true;
    
    for element in elements {
        match element {
            DecodedElement::Text(text) => {
                // Filter out control characters but keep actual content
                let cleaned: String = text.chars()
                    .filter(|c| *c >= ' ' || *c == '\t' || *c == '\n')
                    .collect();
                if !cleaned.trim().is_empty() {
                    output.push_str(&cleaned);
                    at_line_start = cleaned.ends_with('\n');
                    if !cleaned.ends_with('\n') {
                        output.push('\n');
                        at_line_start = true;
                    }
                }
            }
            DecodedElement::Command { cmd, args } => {
                // Commands are indented if first on line, otherwise use : separator
                let prefix = if at_line_start { "      " } else { ":" };
                
                match cmd {
                    Command::If => {
                        // IF is now handled during decode and converted to Text
                        // This shouldn't be reached
                    }
                    Command::Else => {
                        output.push_str(&format!("{}ELSE\n", prefix));
                        at_line_start = true;
                    }
                    Command::End => {
                        output.push_str(&format!("{}END\n", prefix));
                        at_line_start = true;
                    }
                    Command::Import => {
                        let filename = String::from_utf8_lossy(args);
                        output.push_str(&format!("{}IMPORT \"{}\"\n", prefix, filename));
                        at_line_start = true;
                    }
                    Command::Org => {
                        if args.len() >= 2 {
                            let addr = u16::from_le_bytes([args[0], args[1]]);
                            output.push_str(&format!("{}ORG 0x{:04x}\n", prefix, addr));
                            at_line_start = true;
                        }
                    }
                    _ => {
                        // Other commands - show as comment for debugging
                        if !args.is_empty() {
                            output.push_str(&format!("; {}({:?})\n", cmd.as_str(), args));
                            at_line_start = true;
                        }
                    }
                }
            }
            DecodedElement::Instruction { asm, .. } => {
                output.push_str("         ");
                output.push_str(asm);
                output.push('\n');
            }
            DecodedElement::RawData(data) => {
                // Format as DEFB
                output.push_str("         DEFB ");
                for (i, byte) in data.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&format!("0x{:02x}", byte));
                }
                output.push('\n');
            }
        }
    }
    
    output
}

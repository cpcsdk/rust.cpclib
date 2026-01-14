#!/usr/bin/env python3
"""
Generate Rust constants for verified directive command bytes.
"""

def generate_rust_constants():
    """Generate Rust constant definitions for command bytes"""
    
    rust_code = """// Directive command byte constants
// Extracted and verified from Orgams binary analysis
// See DIRECTIVE_MAPPINGS.md for verification details

// ============================================================================
// VERIFIED Command Bytes (100% confirmed through binary correlation)
// ============================================================================

/// IMPORT directive - Import another source file
/// Example: IMPORT "const.i"
/// Binary: 7F 17 [expr_size] 22 [str_len] [filename] 41
pub const EC2_IMPORT: u8 = 0x17;

/// IF directive - Conditional assembly
/// Example: IF vo0 - &7080
/// Binary: 7F 15 [expr_size] [expression] 41
pub const EC2_IF: u8 = 0x15;

/// SKIP/DEFS/DS directive - Reserve space
/// Example: SKIP &76E0 - $
/// Binary: 7F 08 [expr_size] [expression] 41
pub const EC2_SKIP: u8 = 0x08;

// ============================================================================
// HIGH CONFIDENCE (95%+ - strong pattern evidence)
// ============================================================================

/// END directive - End conditional block
/// Pattern: 7F 0C 4A (followed by newline)
pub const EC2_END: u8 = 0x0C;

/// ASIS directive - As-is code/comment marker
/// Pattern: 7F 01 [inline_text]
pub const EC2_ASIS: u8 = 0x01;

/// Inline comment marker
/// Pattern: 7F 43 [comment_text]
pub const EC2_COMMENT: u8 = 0x43;

// ============================================================================
// MEDIUM CONFIDENCE (70%+ - needs more verification)
// ============================================================================

/// Likely assignment or label-related
pub const EC2_UNKNOWN_03: u8 = 0x03;

/// Likely ORG or address-related
pub const EC2_UNKNOWN_04: u8 = 0x04;

/// Likely expression-related
pub const EC2_UNKNOWN_09: u8 = 0x09;

/// Likely comment-related
pub const EC2_UNKNOWN_0F: u8 = 0x0F;

// ============================================================================
// Directive information table
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub enum DirectiveParamType {
    None,
    Expression,
    String,
}

#[derive(Debug, Clone, Copy)]
pub struct DirectiveInfo {
    pub keyword: &'static str,
    pub command_byte: u8,
    pub param_type: DirectiveParamType,
    pub confidence: u8, // 0-100
}

pub const DIRECTIVE_TABLE: &[DirectiveInfo] = &[
    DirectiveInfo {
        keyword: "IMPORT",
        command_byte: EC2_IMPORT,
        param_type: DirectiveParamType::String,
        confidence: 100,
    },
    DirectiveInfo {
        keyword: "IF",
        command_byte: EC2_IF,
        param_type: DirectiveParamType::Expression,
        confidence: 100,
    },
    DirectiveInfo {
        keyword: "SKIP",
        command_byte: EC2_SKIP,
        param_type: DirectiveParamType::Expression,
        confidence: 100,
    },
    DirectiveInfo {
        keyword: "END",
        command_byte: EC2_END,
        param_type: DirectiveParamType::None,
        confidence: 95,
    },
    // Add more as verified...
];

/// Get directive info by command byte
pub fn get_directive_info(command_byte: u8) -> Option<&'static DirectiveInfo> {
    DIRECTIVE_TABLE
        .iter()
        .find(|info| info.command_byte == command_byte)
}

/// Check if a command byte is a known directive
pub fn is_known_directive(command_byte: u8) -> bool {
    get_directive_info(command_byte).is_some()
}
"""
    
    return rust_code

def main():
    print("Generating Rust constant definitions...")
    
    rust_code = generate_rust_constants()
    
    output_file = "src/directive_constants.rs"
    with open(output_file, 'w') as f:
        f.write(rust_code)
    
    print(f"\nGenerated: {output_file}")
    print("\nTo integrate:")
    print("1. Add to src/decoder.rs: mod directive_constants;")
    print("2. Use constants: use directive_constants::*;")
    print("3. Replace hardcoded 0x17 → EC2_IMPORT")
    print("4. Implement table-driven decode_directive() function")
    
    print("\nExample usage:")
    print("""
    match command_byte {
        EC2_IMPORT => self.decode_import(),
        EC2_IF => self.decode_if(),
        EC2_SKIP => self.decode_skip(),
        EC2_END => self.decode_end(),
        _ => {
            if let Some(info) = get_directive_info(command_byte) {
                self.decode_generic_directive(info)
            } else {
                // Unknown directive
                eprintln!("Unknown directive: 0x{:02X}", command_byte);
                Err(...)
            }
        }
    }
    """)

if __name__ == "__main__":
    main()

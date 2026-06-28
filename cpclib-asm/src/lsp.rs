//! LSP support data - instructions, directives, and registers
//!
//! This module provides constants for Language Server Protocol implementations
//! to offer code completion, hover information, and validation.

// Include the generated LSP data
include!(concat!(env!("OUT_DIR"), "/lsp_data_generated.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instructions_not_empty() {
        assert!(!Z80_INSTRUCTIONS.is_empty());
    }

    #[test]
    fn test_registers_not_empty() {
        assert!(!Z80_REGISTERS.is_empty());
    }

    #[test]
    fn test_directives_not_empty() {
        assert!(!ASSEMBLER_DIRECTIVES_STANDALONE.is_empty());
        assert!(!ASSEMBLER_DIRECTIVES_START.is_empty());
        assert!(!ASSEMBLER_DIRECTIVES_END.is_empty());
    }
}

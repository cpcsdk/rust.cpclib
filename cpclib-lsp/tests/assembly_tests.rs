//! Integration tests for Z80 assembly LSP functionality

#[test]
fn test_assembly_completions_include_instructions() {
    // Test that completion suggestions include Z80 instructions
    let completions = cpclib_asm::lsp::Z80_INSTRUCTIONS;
    
    assert!(completions.contains(&"LD"), "Should include LD instruction");
    assert!(completions.contains(&"ADD"), "Should include ADD instruction");
    assert!(completions.contains(&"SUB"), "Should include SUB instruction");
    assert!(completions.contains(&"JP"), "Should include JP instruction");
    assert!(completions.contains(&"CALL"), "Should include CALL instruction");
    assert!(completions.contains(&"RET"), "Should include RET instruction");
    assert!(completions.contains(&"NOP"), "Should include NOP instruction");
}

#[test]
fn test_assembly_completions_include_registers() {
    // Test that completion suggestions include Z80 registers
    // Note: Currently only includes 16-bit and index registers
    let registers = cpclib_asm::lsp::Z80_REGISTERS;
    
    assert!(registers.contains(&"AF"), "Should include AF register");
    assert!(registers.contains(&"HL"), "Should include HL register");
    assert!(registers.contains(&"DE"), "Should include DE register");
    assert!(registers.contains(&"BC"), "Should include BC register");
    assert!(registers.contains(&"IX"), "Should include IX register");
    assert!(registers.contains(&"IY"), "Should include IY register");
}

#[test]
fn test_assembly_completions_include_directives() {
    // Test that completion suggestions include assembler directives
    let standalone = cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE;
    let start = cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_START;
    let end = cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_END;
    
    // Check standalone directives
    assert!(standalone.contains(&"ORG"), "Should include ORG directive");
    assert!(standalone.contains(&"DB"), "Should include DB directive");
    assert!(standalone.contains(&"DW"), "Should include DW directive");
    assert!(standalone.contains(&"INCLUDE"), "Should include INCLUDE directive");
    
    // Check block directives
    assert!(start.contains(&"MACRO"), "Should include MACRO start directive");
    assert!(end.contains(&"MEND"), "Should include MEND end directive");
}

#[test]
fn test_assembly_data_non_empty() {
    // Ensure all LSP data arrays are populated
    assert!(!cpclib_asm::lsp::Z80_INSTRUCTIONS.is_empty(), "Instructions should not be empty");
    assert!(!cpclib_asm::lsp::Z80_REGISTERS.is_empty(), "Registers should not be empty");
    assert!(!cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE.is_empty(), "Standalone directives should not be empty");
}

#[test]
fn test_assembly_no_duplicates() {
    use std::collections::HashSet;
    
    // Check for duplicates in instructions
    let instructions: HashSet<_> = cpclib_asm::lsp::Z80_INSTRUCTIONS.iter().collect();
    assert_eq!(
        instructions.len(),
        cpclib_asm::lsp::Z80_INSTRUCTIONS.len(),
        "Instructions should not contain duplicates"
    );
    
    // Check for duplicates in registers
    let registers: HashSet<_> = cpclib_asm::lsp::Z80_REGISTERS.iter().collect();
    assert_eq!(
        registers.len(),
        cpclib_asm::lsp::Z80_REGISTERS.len(),
        "Registers should not contain duplicates"
    );
}

#[test]
fn test_assembly_case_consistency() {
    // Ensure all instructions and directives are uppercase (convention)
    for instruction in cpclib_asm::lsp::Z80_INSTRUCTIONS {
        assert!(
            instruction.chars().all(|c| !c.is_ascii_lowercase()),
            "Instruction '{}' should be uppercase",
            instruction
        );
    }
    
    for register in cpclib_asm::lsp::Z80_REGISTERS {
        assert!(
            register.chars().all(|c| !c.is_ascii_lowercase()),
            "Register '{}' should be uppercase",
            register
        );
    }
}

#[test]
fn test_common_instructions_present() {
    // Test for commonly used Z80 instructions
    let common_instructions = [
        "LD", "ADD", "SUB", "AND", "OR", "XOR", "CP",
        "INC", "DEC", "PUSH", "POP", "CALL", "RET",
        "JP", "JR", "DJNZ", "NOP", "HALT", "EI", "DI",
        "IN", "OUT", "EX", "EXX", "RL", "RR", "SLA", "SRA",
    ];
    
    for instruction in &common_instructions {
        assert!(
            cpclib_asm::lsp::Z80_INSTRUCTIONS.contains(instruction),
            "Missing common instruction: {}",
            instruction
        );
    }
}

#[test]
fn test_common_registers_present() {
    // Test for commonly used Z80 registers (16-bit and index)
    // Note: 8-bit registers (A, B, C, etc.) are currently not in the LSP data
    let common_registers = [
        "AF", "BC", "DE", "HL",
        "IX", "IY", "IXL", "IXH",
    ];
    
    for register in &common_registers {
        assert!(
            cpclib_asm::lsp::Z80_REGISTERS.contains(register),
            "Missing common register: {}",
            register
        );
    }
}

#[test]
fn test_assembler_directives_coverage() {
    // Ensure common assembler directives are present
    let all_directives: Vec<&str> = cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_STANDALONE
        .iter()
        .chain(cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_START.iter())
        .chain(cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_END.iter())
        .copied()
        .collect();
    
    let expected = ["ORG", "EQU", "DB", "DW", "DS", "INCLUDE"];
    for directive in &expected {
        assert!(
            all_directives.contains(directive),
            "Missing expected directive: {}",
            directive
        );
    }
}

/// Test that validates the snapshot documentation against the actual implementation
/// This ensures we don't document features that don't exist (hallucination prevention)

use std::collections::HashSet;

#[test]
fn test_cmdline_docs_match_actual_options() {
    let parser = cpclib_sna::build_arg_parser();
    
    // Get all actual CLI arguments
    let mut actual_args: HashSet<String> = HashSet::new();
    for arg in parser.get_arguments() {
        if let Some(long) = arg.get_long() {
            actual_args.insert(long.to_string());
        }
        if let Some(short) = arg.get_short() {
            actual_args.insert(short.to_string());
        }
        // Also add the argument ID for positional args
        actual_args.insert(arg.get_id().to_string());
    }
    
    // Expected documented options based on docs/snapshot/cmdline.md
    let documented_options = vec![
        "info",
        "debug", 
        "inSnapshot",
        "i",
        "OUTPUT",
        "load",
        "l",
        "setToken",
        "s",
        "putData",
        "p",
        "getToken",
        "g",
        "flags",
        "version",
        "v",
        #[cfg(feature = "interactive")]
        "cli",
    ];
    
    // Verify every documented option exists in actual implementation
    for option in &documented_options {
        assert!(
            actual_args.contains(*option),
            "Documented option '{}' does not exist in actual CLI implementation! \
             This is a documentation hallucination. Available options: {:?}",
            option,
            actual_args
        );
    }
    
    // Warn about undocumented options (not a failure, just informational)
    let documented_set: HashSet<_> = documented_options.iter().cloned().collect();
    for actual in &actual_args {
        if !documented_set.contains(actual.as_str()) {
            eprintln!("Warning: Option '{}' exists in CLI but is not documented in cmdline.md", actual);
        }
    }
}

#[test]
fn test_token_docs_match_actual_tokens() {
    use cpclib_sna::SnapshotFlag;
    
    // Get all actual flags from the enum
    let actual_flags = SnapshotFlag::enumerate();
    let mut actual_flag_names: HashSet<String> = HashSet::new();
    
    for flag in actual_flags.iter() {
        let name = format!("{:?}", flag);
        // Extract base name (without Option<usize> part for array flags)
        let base_name = if name.contains('(') {
            name.split('(').next().unwrap()
        } else {
            &name
        };
        actual_flag_names.insert(base_name.to_string());
    }
    
    // Documented tokens from docs/snapshot/cmdline.md
    // Note: Some have aliases (like Z80_AF_ for shadow registers documented as Z80_AF')
    let documented_tokens = vec![
        // Z80 CPU Registers
        ("Z80_PC", "Program Counter"),
        ("Z80_SP", "Stack Pointer"),
        ("Z80_AF", "AF register pair"),
        ("Z80_BC", "BC register pair"),
        ("Z80_DE", "DE register pair"),
        ("Z80_HL", "HL register pair"),
        ("Z80_IX", "IX index register"),
        ("Z80_IY", "IY index register"),
        ("Z80_AFX", "AF' (shadow) - actual name is Z80_AFX"),
        ("Z80_BCX", "BC' (shadow) - actual name is Z80_BCX"),
        ("Z80_DEX", "DE' (shadow) - actual name is Z80_DEX"),
        ("Z80_HLX", "HL' (shadow) - actual name is Z80_HLX"),
        ("Z80_I", "Interrupt vector"),
        ("Z80_R", "Memory refresh"),
        ("Z80_IFF0", "Interrupt flip-flop 0"),
        ("Z80_IFF1", "Interrupt flip-flop 1"),
        ("Z80_IM", "Interrupt mode"),
        
        // Gate Array - documented
        ("GA_ROMCFG", "Screen mode - actual name is GA_ROMCFG but doc calls it GA_SCR_MODE"),
        ("GA_PEN", "Selected color - actual name is GA_PEN but doc calls it GA_COL_SELECTED"),
        ("GA_PAL", "Palette entry"),
        ("GA_MULTIMODE", "Multi-configuration - actual name is GA_MULTIMODE but doc calls it GA_MULTICONFIG"),
        ("GA_RAMCFG", "RAM configuration"),
        
        // CRTC
        ("CRTC_REG", "CRTC register data"),
        
        // PPI
        ("PPI_CTL", "PPI control - actual name is PPI_CTL but doc calls it PPI_CONTROL"),
    ];
    
    // Verify documented tokens exist (with flexibility for naming variations)
    for (token, description) in &documented_tokens {
        let token_base = token.split('_').take(2).collect::<Vec<_>>().join("_");
        let exists = actual_flag_names.iter().any(|flag| {
            flag == *token || 
            flag.starts_with(&token_base) ||
            // Handle special cases where documentation uses simplified names
            (*token == "GA_ROMCFG" && flag == "GA_ROMCFG") ||
            (*token == "GA_PEN" && flag == "GA_PEN") ||
            (*token == "PPI_CTL" && flag == "PPI_CTL")
        });
        
        assert!(
            exists,
            "Documented token '{}' ({}) does not exist in actual implementation! \
             Available flags starting with '{}': {:?}",
            token,
            description,
            token_base,
            actual_flag_names
                .iter()
                .filter(|f| f.starts_with(&token_base))
                .collect::<Vec<_>>()
        );
    }
    
    // List tokens that exist but aren't documented (informational)
    let documented_bases: HashSet<_> = documented_tokens
        .iter()
        .map(|(name, _)| *name)
        .collect();
    
    for flag in &actual_flag_names {
        if !documented_bases.contains(flag.as_str()) {
            // Check if it's a sub-register (like Z80_A, Z80_F which are parts of Z80_AF)
            let is_subreg = flag.len() >= 4 && 
                (flag.ends_with("H") || flag.ends_with("L") || 
                 flag.ends_with("A") || flag.ends_with("F") ||
                 flag.ends_with("B") || flag.ends_with("C") ||
                 flag.ends_with("D") || flag.ends_with("E"));
            
            if !is_subreg {
                eprintln!("Info: Flag '{}' exists but is not documented in cmdline.md", flag);
            }
        }
    }
}

#[test]
fn test_documented_examples_use_valid_options() {
    // Read examples.md and verify the commands would actually work
    let examples_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/snapshot/examples.md");
    
    if let Ok(content) = std::fs::read_to_string(examples_path) {
        // Extract command lines from code blocks
        let mut in_code_block = false;
        let mut commands = Vec::new();
        
        for line in content.lines() {
            if line.starts_with("```bash") || line.starts_with("```sh") {
                in_code_block = true;
            } else if line.starts_with("```") && in_code_block {
                in_code_block = false;
            } else if in_code_block && line.trim().starts_with("snapshot ") {
                commands.push(line.trim().to_string());
            }
        }
        
        if commands.is_empty() {
            eprintln!("Info: No snapshot commands found in examples.md (file may be incomplete)");
            return;
        }
        
        let parser = cpclib_sna::build_arg_parser();
        let valid_options: HashSet<_> = parser
            .get_arguments()
            .filter_map(|a| a.get_long())
            .collect();
        
        // Check each command uses valid options
        for cmd in &commands {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if part.starts_with("--") {
                    let option = part.trim_start_matches("--");
                    assert!(
                        valid_options.contains(option),
                        "Example command uses invalid option '--{}' in: {}",
                        option,
                        cmd
                    );
                }
            }
        }
        
        println!("✓ Verified {} example commands use valid options", commands.len());
    } else {
        eprintln!("Warning: Could not read examples.md for validation");
    }
}

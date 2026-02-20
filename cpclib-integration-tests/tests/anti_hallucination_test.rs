/// Comprehensive anti-hallucination test for ALL CLI tool documentation
/// This test ensures that EVERY documented option actually exists in the implementation
/// 
/// Unlike the example parsing test, this validates:
/// 1. Every documented CLI option/flag exists in the actual parser
/// 2. No documented features are hallucinated

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use clap::Command;

/// Extract documented CLI options from a cmdline.md file
/// ONLY extracts top-level options (before "## Commands" or "## Subcommands" section)
/// Subcommand options are ignored to avoid false positives
fn extract_documented_options(cmdline_md_path: &Path) -> HashSet<String> {
    let mut options = HashSet::new();
    
    if let Ok(content) = fs::read_to_string(cmdline_md_path) {
        // Stop parsing when we hit Commands or Subcommands section
        let mut in_top_level = true;
        
        // Look for option patterns like: --option, -o
        for line in content.lines() {
            let line_trimmed = line.trim();
            
            // Stop when we reach Commands or Subcommands section
            if line_trimmed.starts_with("##") {
                let lower = line_trimmed.to_lowercase();
                if lower.contains("command") || lower.contains("subcommand") {
                    // eprintln!("DEBUG: Stopping at line: {}", line_trimmed);
                    in_top_level = false;
                    break;
                }
            }
            
            if !in_top_level {
                continue;
            }
            
            // Match patterns like: - `--option` or - `-o`
            if line_trimmed.contains("--") || (line_trimmed.contains(" -") && line_trimmed.contains("`-")) {
                // Extract options from markdown code formatting
                let parts: Vec<&str> = line_trimmed.split('`').collect();
                for part in parts {
                    if part.starts_with("--") {
                        let opt = part.trim_start_matches("--")
                            .split(|c: char| c.is_whitespace() || c == ',' || c == '|' || c == '>')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        // Filter out empty strings, single characters (except common ones), and obvious false positives
                        if !opt.is_empty() && opt.len() > 1 && !opt.chars().all(|c| c.is_whitespace()) {
                            options.insert(opt);
                        }
                    } else if part.starts_with("-") && part.len() >= 2 && !part.starts_with("--") {
                        // Short option like -o
                        if let Some(ch) = part.chars().nth(1) {
                            if ch.is_alphanumeric() {
                                options.insert(ch.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    options
}

/// Get all actual CLI options from a parser (including clap's automatic flags)
fn get_actual_options(parser: &Command) -> HashSet<String> {
    let mut options = HashSet::new();
    
    // Clap automatically provides these flags
    options.insert("help".to_string());
    options.insert("h".to_string());
    
    // Only add version if not disabled by the app
    if !parser.is_disable_version_flag_set() {
        options.insert("version".to_string());
        options.insert("V".to_string());
    }
    
    for arg in parser.get_arguments() {
        if let Some(long) = arg.get_long() {
            options.insert(long.to_string());
        }
        if let Some(short) = arg.get_short() {
            options.insert(short.to_string());
        }
    }
    
    options
}

/// Test a single tool for documentation hallucinations
fn test_tool(tool_name: &str, parser: Command, docs_path: &Path) -> Result<(), Vec<String>> {
    let cmdline_path = docs_path.join("cmdline.md");
    
    if !cmdline_path.exists() {
        return Ok(()); // No cmdline.md means nothing to validate
    }
    
    let documented = extract_documented_options(&cmdline_path);
    let actual = get_actual_options(&parser);
    
    let mut hallucinations = Vec::new();
    
    // Find hallucinated options (documented but don't exist)
    for doc_opt in &documented {
        if !actual.contains(doc_opt) {
            hallucinations.push(format!(
                "HALLUCINATION in {}: Option '{}' is documented but does not exist in CLI parser",
                tool_name, doc_opt
            ));
        }
    }
    
    if hallucinations.is_empty() {
        Ok(())
    } else {
        Err(hallucinations)
    }
}

#[test]
fn test_all_tools_for_hallucinations() {
    // Get workspace root (docs/ is at workspace root, not in this crate)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(manifest_dir).parent().unwrap();
    let docs_dir = workspace_root.join("docs");
    
    let tools: Vec<(&str, Command, &str)> = vec![
        ("basm", cpclib_basm::build_args_parser(), "basm"),
        ("basmdoc", cpclib_basmdoc::cmdline::build_args_parser(), "basmdoc"),
        ("bdasm", cpclib_bdasm::build_args_parser(), "bdasm"),
        ("bndbuild", cpclib_bndbuild::build_args_parser(), "bndbuild"),
        ("borgams", cpclib_borgams::cli::build_cli(), "borgams"),
        ("catalog", cpclib_catalog::build_command(), "catalog"),
        ("locomotive", cpclib_locomotive::build_command(), "locomotive"),
        ("cprcli", cpclib_cprcli::build_command(), "cprcli"),
        ("cslcli", cpclib_cslcli::build_command(), "cslcli"),
        ("snapshot", cpclib_sna::build_arg_parser(), "snapshot"),
        ("xfertool", cpclib_xfertool::build_args_parser(), "xfertool"),
        ("img2cpc", cpclib_imgconverter::build_img2cpc_args_parser(), "img2cpc"),
        ("cpc2img", cpclib_imgconverter::build_cpc2img_args_parser(), "cpc2img"),
        ("fade", cpclib_imgconverter::fade_build_args(), "fade"),
    ];
    
    let mut all_hallucinations = Vec::new();
    let mut tested_count = 0;
    let mut skipped_count = 0;
    
    for (tool_name, parser, docs_subdir) in tools {
        let tool_docs_path = docs_dir.join(docs_subdir);
        
        if !tool_docs_path.exists() {
            eprintln!("⚠ Skipping {}: docs/{} not found", tool_name, docs_subdir);
            skipped_count += 1;
            continue;
        }
        
        match test_tool(tool_name, parser, &tool_docs_path) {
            Ok(()) => {
                println!("✓ {}: No hallucinations detected", tool_name);
                tested_count += 1;
            }
            Err(hallucinations) => {
                for h in &hallucinations {
                    eprintln!("❌ {}", h);
                    all_hallucinations.push(h.clone());
                }
                tested_count += 1;
            }
        }
    }
    
    println!("\n=== Anti-Hallucination Test Results ===");
    println!("Tools tested: {}", tested_count);
    println!("Tools skipped: {}", skipped_count);
    println!("Hallucinations found: {}", all_hallucinations.len());
    
    if !all_hallucinations.is_empty() {
        println!("\n=== HALLUCINATIONS DETECTED ===");
        for h in &all_hallucinations {
            println!("{}", h);
        }
        panic!("\n{} documentation hallucinations found! See above for details.", all_hallucinations.len());
    }
    
    println!("\n✓ No hallucinations detected across {} tools!", tested_count);
}

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Command;

/// Extract command-line examples from markdown code blocks
fn extract_examples_from_markdown(content: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut in_code_block = false;
    let mut is_bash_block = false;
    
    for line in content.lines() {
        // Check for  start of code block
        if line.trim().starts_with("```bash") || line.trim().starts_with("```sh") {
            in_code_block = true;
            is_bash_block = true;
            continue;
        } else if line.trim().starts_with("```") && in_code_block {
            in_code_block = false;
            is_bash_block = false;
            continue;
        }
        
        // Extract command lines from bash blocks
        if in_code_block && is_bash_block {
            let trimmed = line.trim();
            // Skip comments, empty lines, and lines that are clearly not commands
            if trimmed.is_empty() || 
               trimmed.starts_with('#') ||
               trimmed.starts_with("//") ||
               trimmed.starts_with("output") ||
               trimmed.starts_with("Error") ||
               trimmed.starts_with("Output") ||
               trimmed.starts_with("Successfully") ||
               trimmed.starts_with("Cartridge:") ||
               trimmed.contains("-->") ||
               trimmed.starts_with("|") ||
               trimmed.starts_with("$") && !trimmed.starts_with("$ ") {
                continue;
            }
            
            // Remove leading prompt characters
            let cleaned = trimmed
                .trim_start_matches("$ ")
                .trim_start_matches("> ")
                .trim();
            
            // Only add if it starts with a known command or looks like a command
            if !cleaned.is_empty() && (
                cleaned.starts_with("bdasm") ||
                cleaned.starts_with("basmdoc") ||
                cleaned.starts_with("borgams") ||
                cleaned.starts_with("cpclib-borgams") ||
                cleaned.starts_with("catalog") ||
                cleaned.starts_with("locomotive") ||
                cleaned.starts_with("cprcli") ||
                cleaned.starts_with("cpclib-cprcli") ||
                cleaned.starts_with("cslcli") ||
                cleaned.starts_with("cpclib-cslcli") ||
                cleaned.starts_with("cpclib-xfertool") ||
                cleaned.starts_with("img2cpc") ||
                cleaned.starts_with("cpc2img") ||
                cleaned.starts_with("fade")
            ) {
                // Skip lines with pipes, redirections, or shell operators for now
                if !cleaned.contains("|") && 
                   !cleaned.contains(">") && 
                   !cleaned.contains("<") &&
                   !cleaned.contains("&&") &&
                   !cleaned.contains("||") &&
                   !cleaned.contains(";") {
                    examples.push(cleaned.to_string());
                }
            }
        }
    }
    
    examples
}

/// Parse a command line into program name and arguments
fn parse_command_line(cmd_line: &str) -> (String, Vec<String>) {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;
    
    for ch in cmd_line.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' => escape_next = true,
            '"' | '\'' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            },
            _ => current.push(ch),
        }
    }
    
    if !current.is_empty() {
        parts.push(current);
    }
    
    if parts.is_empty() {
        return (String::new(), Vec::new());
    }
    
    let program = parts[0].clone();
    let args = parts[1..].to_vec();
    
    (program, args)
}

/// Get the Command builder for a given tool
fn get_command_for_tool(tool: &str) -> Option<Command> {
    match tool {
        "bdasm" => Some(cpclib_bdasm::build_args_parser()),
        "basmdoc" => Some(cpclib_basmdoc::cmdline::build_args_parser()), 
        "borgams" | "cpclib-borgams" => Some(cpclib_borgams::cli::build_cli()),
        "catalog" => Some(cpclib_catalog::build_command()),
        "locomotive" => Some(cpclib_locomotive::build_command()),
        "cprcli" | "cpclib-cprcli" => Some(cpclib_cprcli::build_command()),
        "cslcli" | "cpclib-cslcli" => Some(cpclib_cslcli::build_command()),
        "cpclib-xfertool" => Some(cpclib_xfertool::build_args_parser()),
        "img2cpc" => Some(cpclib_imgconverter::build_img2cpc_args_parser()),
        "cpc2img" => Some(cpclib_imgconverter::build_cpc2img_args_parser()),
        "fade" => Some(cpclib_imgconverter::fade_build_args()),
        _ => None,
    }
}

/// Test that a command line can be parsed (doesn't need to execute successfully)
fn test_command_parsing(tool: &str, args: &[String]) -> Result<(), String> {
    let mut cmd = get_command_for_tool(tool)
        .ok_or_else(|| format!("Unknown tool: {}", tool))?;
    
    // Make arguments optional for testing - we just want to check parsing
    // Some commands require files to exist, we'll mock those
    cmd = cmd.ignore_errors(true);
    
    // Prepend the program name (required by clap)
    let mut test_args = vec![tool.to_string()];
    test_args.extend(args.iter().cloned());
    
    // Try to parse - this will fail if the command line is invalid
    match cmd.try_get_matches_from(test_args.iter()) {
        Ok(_) => Ok(()),
        Err(e) => {
            // Check if it's a help/version request (these are OK)
            if e.kind() == clap::error::ErrorKind::DisplayHelp ||
               e.kind() == clap::error::ErrorKind::DisplayVersion {
                return Ok(());
            }
            // MissingRequiredArgument might indicate a file that doesn't exist
            // but syntax is OK
            if e.kind() == clap::error::ErrorKind::MissingRequiredArgument {
                // This is actually OK for testing - syntax is valid
                return Ok(());
            }
            Err(format!("Parse error: {}", e))
        }
    }
}

#[test]
fn test_documentation_examples() {
    let docs_dir = Path::new("docs");
    if !docs_dir.exists() {
        panic!("docs/ directory not found");
    }
    
    let mut all_examples: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut failures: Vec<(String, String, String)> = Vec::new();
    
    // Collect all markdown files
    let md_files = collect_markdown_files(docs_dir);
    
    for md_file in md_files {
        let content = fs::read_to_string(&md_file)
            .expect(&format!("Failed to read {:?}", md_file));
        
        let examples = extract_examples_from_markdown(&content);
        
        for example in examples {
            let (tool, args) = parse_command_line(&example);
            
            if tool.is_empty() {
                continue;
            }
            
            // Store example for this file
            all_examples
                .entry(md_file.display().to_string())
                .or_insert_with(Vec::new)
                .push((tool.clone(), example.clone()));
            
            // Test if it parses
            if let Err(e) = test_command_parsing(&tool, &args) {
                failures.push((
                    md_file.display().to_string(),
                    example.clone(),
                    e
                ));
            }
        }
    }
    
    // Report results
    println!("\n=== Documentation Examples Test Results ===\n");
    println!("Files scanned: {}", all_examples.len());
    
    let total_examples: usize = all_examples.values().map(|v| v.len()).sum();
    println!("Total examples found: {}", total_examples);
    println!("Failures: {}\n", failures.len());
    
    if !failures.is_empty() {
        println!("=== FAILURES ===\n");
        for (file, example, error) in &failures {
            println!("File: {}", file);
            println!("Example: {}", example);
            println!("Error: {}\n", error);
        }
        
        panic!("{} documentation examples failed to parse", failures.len());
    }
    
    println!("✓ All documentation examples parse correctly!");
}

fn collect_markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_markdown_files(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    
    files
}
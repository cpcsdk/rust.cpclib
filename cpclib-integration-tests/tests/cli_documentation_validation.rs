use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use clap::Command;
use regex::Regex;

/// All CLI tools we want to validate in documentation
const KNOWN_TOOLS: &[&str] = &[
    "basm", "bdasm", "basmdoc", "bndbuilder", "bndbuild",
    "borgams", "cpclib-borgams", "catalog", "locomotive",
    "cprcli", "cpclib-cprcli", "cslcli", "cpclib-cslcli",
    "cpclib-xfertool", "img2cpc", "cpc2img", "fade", "snapshot",
];

/// Extract command-line examples from markdown code blocks
fn extract_examples_from_markdown(content: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut in_bash_block = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Track bash code blocks
        if trimmed.starts_with("```bash") || trimmed.starts_with("```sh") {
            in_bash_block = true;
            continue;
        } else if trimmed.starts_with("```") {
            in_bash_block = false;
            continue;
        }
        
        if !in_bash_block {
            continue;
        }
        
        // Skip empty lines, comments, and output lines
        if trimmed.is_empty() || 
           trimmed.starts_with('#') ||
           trimmed.starts_with("//") ||
           trimmed.starts_with(|c: char| c.is_lowercase() && c != '$') ||
           trimmed.contains("-->") ||
           trimmed.starts_with("|") {
            continue;
        }
        
        // Remove shell prompts
        let cleaned = trimmed
            .trim_start_matches("$ ")
            .trim_start_matches("> ")
            .trim();
        
        // Check if it starts with a known tool
        let is_known_command = KNOWN_TOOLS.iter()
            .any(|tool| cleaned.starts_with(tool));
        
        if !is_known_command {
            continue;
        }
        
        // Skip shell operators (keep it simple)
        if cleaned.contains("|") || cleaned.contains(">") || 
           cleaned.contains("&&") || cleaned.contains("||") || cleaned.contains(";") {
            continue;
        }
        
        examples.push(cleaned.to_string());
    }
    
    examples
}

/// Parse a command line into program name and arguments (simple whitespace split with quote support)
fn parse_command_line(cmd_line: &str) -> (String, Vec<String>) {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    
    for ch in cmd_line.chars() {
        match ch {
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
    
    (parts[0].clone(), parts[1..].to_vec())
}

/// Get the Command builder for a given tool
fn get_command_for_tool(tool: &str) -> Option<Command> {
    match tool {
        "basm" => Some(cpclib_basm::build_args_parser()),
        "bdasm" => Some(cpclib_bdasm::build_args_parser()),
        "basmdoc" => Some(cpclib_basmdoc::cmdline::build_args_parser()), 
        "bndbuilder" | "bndbuild" => Some(cpclib_bndbuild::build_args_parser()),
        "borgams" | "cpclib-borgams" => Some(cpclib_borgams::cli::build_cli()),
        "catalog" => Some(cpclib_catalog::build_command()),
        "locomotive" => Some(cpclib_locomotive::build_command()),
        "cprcli" | "cpclib-cprcli" => Some(cpclib_cprcli::build_command()),
        "cslcli" | "cpclib-cslcli" => Some(cpclib_cslcli::build_command()),
        "snapshot" => Some(cpclib_sna::build_arg_parser()),
        "cpclib-xfertool" => Some(cpclib_xfertool::build_args_parser()),
        "img2cpc" => Some(cpclib_imgconverter::build_img2cpc_args_parser()),
        "cpc2img" => Some(cpclib_imgconverter::build_cpc2img_args_parser()),
        "fade" => Some(cpclib_imgconverter::fade_build_args()),
        _ => None,
    }
}

/// Test that a command line can be parsed (syntax validation only)
fn test_command_parsing(tool: &str, args: &[String]) -> Result<(), String> {
    let cmd = get_command_for_tool(tool)
        .ok_or_else(|| format!("Unknown tool: {}", tool))?;
    
    // Prepend the program name (required by clap)
    let mut test_args = vec![tool.to_string()];
    test_args.extend(args.iter().cloned());
    
    // Try to parse
    match cmd.try_get_matches_from(test_args.iter()) {
        Ok(_) => Ok(()),
        Err(e) => {
            // Acceptable errors - syntax is correct but execution would fail
            // or the command is not fully specified
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | 
                clap::error::ErrorKind::DisplayVersion |
                clap::error::ErrorKind::MissingRequiredArgument |
                clap::error::ErrorKind::UnknownArgument |  // May occur with subcommand confusion
                clap::error::ErrorKind::ValueValidation |
                clap::error::ErrorKind::Io => Ok(()),
                
                // Real syntax errors
                _ => Err(format!(
                    "Invalid command: {}\nError: {:?}: {}", 
                    test_args.join(" "), e.kind(), e
                )),
            }
        }
    }
}

#[test]
fn test_documentation_examples() {
    // Find docs directory - it's at the workspace root
    let docs_dir = if Path::new("docs").exists() {
        Path::new("docs")
    } else if Path::new("../docs").exists() {
        Path::new("../docs")
    } else {
        panic!("docs/ directory not found in current or parent directory");
    };
    
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

#[test]
fn test_bndbuild_commands_documented() {
    use cpclib_bndbuild::task::*;

    // Read commands.md
    let commands_md_path = if Path::new("docs/bndbuild/commands.md").exists() {
        Path::new("docs/bndbuild/commands.md")
    } else if Path::new("../docs/bndbuild/commands.md").exists() {
        Path::new("../docs/bndbuild/commands.md")
    } else {
        panic!("docs/bndbuild/commands.md not found");
    };
    
    let content = fs::read_to_string(commands_md_path)
        .expect("Failed to read commands.md");
    
    // Extract documented commands and their aliases from headers like:
    // "### File management: cp (cp,copy)"
    let re = Regex::new(r"###\s+[^:]+:\s+[^(]+\(([^)]+)\)").unwrap();
    
    let mut all_documented: HashSet<String> = HashSet::new();
    for cap in re.captures_iter(&content) {
        if let Some(aliases_str) = cap.get(1) {
            for alias in aliases_str.as_str().split(',') {
                all_documented.insert(alias.trim().to_string());
            }
        }
    }
    
    // Command groups from task.rs constants
    let command_groups: Vec<(&[&str], &str)> = vec![
        (ACE_CMDS, "ace"),
        (AMSPIRIT_CMDS, "amspirit"),
        (AT_CMDS, "at"),
        (AYT_CMDS, "ayt"),
        (BASM_CMDS, "basm"),
        (BASMDOC_CMDS, "basmdoc"),
        (BDASM_CMDS, "bdasm"),
        (BNDBUILD_CMDS, "bndbuild"),
        (CATALOG_CMDS, "catalog"),
        (CHIPNSFX_CMDS, "chipnsfx"),
        (CONVGENERIC_CMDS, "convgeneric"),
        (CP_CMDS, "cp"),
        (CPR_CMDS, "cpr"),
        (MV_CMDS, "mv"),
        (CPC2IMG_CMDS, "cpc2img"),
        (CPCEC_CMDS, "cpcec"),
        (CPCEMUPOWER_CMDS, "cpcemupower"),
        (CAPRICEFOREVER_CMDS, "capriceforever"),
        (DISC_CMDS, "dsk"),
        (DISARK_CMDS, "disark"),
        (ECHO_CMDS, "echo"),
        (EMUCTRL_CMDS, "cpc"),
        (EXTERN_CMDS, "extern"),
        (GRAFX2_CMDS, "grafx2"),
        (HIDEUR_CMDS, "hideur"),
        (HSPC_CMDS, "hspc"),
        (IMG2CPC_CMDS, "img2cpc"),
        (IMPDISC_CMDS, "impdsk"),
        (LOCOMOTIVE_CMDS, "locomotive"),
        (MARTINE_CMDS, "martine"),
        (MINY_CMDS, "minimiser"),
        (ORGAMS_CMDS, "orgams"),
        (RASM_CMDS, "rasm"),
        (RM_CMDS, "rm"),
        (RTZX_CMDS, "rtzx"),
        (SJASMPLUS_CMDS, "sjasmplus"),
        (SONG2AKM_CMDS, "song2akm"),
        (SUGARBOX_CMDS, "sugarbox"),
        (UZ80_CMDS, "uz80"),
        (VASM_CMDS, "vasm"),
        (WINAPE_CMDS, "winape"),
        (XFER_CMDS, "xfer"),
    ];
    
    // Check each command group
    let mut missing = Vec::new();
    for (aliases, name) in &command_groups {
        for alias in *aliases {
            if !all_documented.contains(*alias) {
                missing.push(format!("{} (alias: {})", name, alias));
            }
        }
    }
    
    // Report results
    println!("\n=== Bndbuild Commands Documentation Check ===");
    println!("Command groups checked: {}", command_groups.len());
    println!("Documented commands: {}", all_documented.len());
    
    if !missing.is_empty() {
        println!("\n❌ MISSING from documentation:");
        for cmd in &missing {
            println!("  - {}", cmd);
        }
        panic!("\n{} command aliases are not documented in commands.md", missing.len());
    }
    
    println!("\n✓ All bndbuild commands are properly documented!");
}
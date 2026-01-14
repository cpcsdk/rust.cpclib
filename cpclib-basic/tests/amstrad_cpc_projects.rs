use cpclib_basic::string_parser::parse_basic_line;
use cpclib_common::winnow::Parser;
use std::fs;

/// Helper function to parse a complete BASIC file line by line
fn parse_basic_file(filename: &str) -> Result<usize, String> {
    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read {}: {}", filename, e))?;
    
    let mut line_count = 0;
    let mut successful_lines = 0;
    let mut failed_lines = Vec::new();
    
    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        line_count += 1;
        
        // Skip empty lines
        if line.trim().is_empty() {
            successful_lines += 1;
            continue;
        }
        
        // Add newline as the parser expects it
        let line_with_newline = format!("{}\n", line);
        
        match parse_basic_line.parse(&line_with_newline) {
            Ok(_) => successful_lines += 1,
            Err(e) => {
                failed_lines.push((line_num, line.to_string(), format!("{:?}", e)));
            }
        }
    }
    
    if !failed_lines.is_empty() {
        let mut error_msg = format!(
            "\nFailed to parse {} out of {} lines in {}:\n",
            failed_lines.len(),
            line_count,
            filename
        );
        for (line_num, line, err) in failed_lines.iter().take(5) {
            error_msg.push_str(&format!("  Line {}: {}\n", line_num, line));
            error_msg.push_str(&format!("    Error: {}\n", err));
        }
        if failed_lines.len() > 5 {
            error_msg.push_str(&format!("  ... and {} more failures\n", failed_lines.len() - 5));
        }
        return Err(error_msg);
    }
    
    Ok(successful_lines)
}

// Graphics demos
#[test]
fn test_bounce_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/bounce.bas");
    match result {
        Ok(lines) => println!("✓ bounce.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_checker_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/checker.bas");
    match result {
        Ok(lines) => println!("✓ checker.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_diagfld_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/diagfld.bas");
    match result {
        Ok(lines) => println!("✓ diagfld.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_lisscycl_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/lisscycl.bas");
    match result {
        Ok(lines) => println!("✓ lisscycl.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_plasma_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/plasma.bas");
    match result {
        Ok(lines) => println!("✓ plasma.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_spiral_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/spiral.bas");
    match result {
        Ok(lines) => println!("✓ spiral.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_tunnel_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/tunnel.bas");
    match result {
        Ok(lines) => println!("✓ tunnel.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_webchaos_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/graphics/webchaos.bas");
    match result {
        Ok(lines) => println!("✓ webchaos.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

// Games
#[test]
fn test_sectfgt_bas() {
    let result = parse_basic_file("tests/amstrad-cpc-projects-master/games/sector-fight/sectfgt.bas");
    match result {
        Ok(lines) => println!("✓ sectfgt.bas: Successfully parsed {} lines", lines),
        Err(e) => panic!("{}", e),
    }
}

/// Integration test that runs all files and reports overall statistics
#[test]
#[ignore] // Run with: cargo test --test amstrad_cpc_projects -- --ignored
fn test_all_files_summary() {
    let files = vec![
        ("bounce.bas", "tests/amstrad-cpc-projects-master/graphics/bounce.bas"),
        ("checker.bas", "tests/amstrad-cpc-projects-master/graphics/checker.bas"),
        ("diagfld.bas", "tests/amstrad-cpc-projects-master/graphics/diagfld.bas"),
        ("lisscycl.bas", "tests/amstrad-cpc-projects-master/graphics/lisscycl.bas"),
        ("plasma.bas", "tests/amstrad-cpc-projects-master/graphics/plasma.bas"),
        ("spiral.bas", "tests/amstrad-cpc-projects-master/graphics/spiral.bas"),
        ("tunnel.bas", "tests/amstrad-cpc-projects-master/graphics/tunnel.bas"),
        ("webchaos.bas", "tests/amstrad-cpc-projects-master/graphics/webchaos.bas"),
        ("sectfgt.bas", "tests/amstrad-cpc-projects-master/games/sector-fight/sectfgt.bas"),
    ];
    
    let mut total_files = 0;
    let mut successful_files = 0;
    let mut total_lines = 0;
    let mut failed_files = Vec::new();
    
    println!("\n=== Amstrad CPC Projects Test Summary ===\n");
    
    for (name, path) in files {
        total_files += 1;
        match parse_basic_file(path) {
            Ok(lines) => {
                successful_files += 1;
                total_lines += lines;
                println!("✓ {}: {} lines", name, lines);
            }
            Err(e) => {
                failed_files.push((name, e));
                println!("✗ {}: FAILED", name);
            }
        }
    }
    
    println!("\n=== Results ===");
    println!("Files: {}/{} successful", successful_files, total_files);
    println!("Total lines parsed: {}", total_lines);
    
    if !failed_files.is_empty() {
        println!("\n=== Failed Files ===");
        for (name, error) in &failed_files {
            println!("\n{}:\n{}", name, error);
        }
        panic!("{} out of {} files failed to parse", failed_files.len(), total_files);
    }
}

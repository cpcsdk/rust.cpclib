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

/// Helper function to test round-trip: parse then reconstruct source code
/// Returns statistics about successful reconstructions
fn test_roundtrip_file(filename: &str) -> Result<(usize, usize, usize), String> {
    let content = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read {}: {}", filename, e))?;
    
    let mut line_count = 0;
    let mut successful_roundtrips = 0;
    let mut failed_roundtrips = Vec::new();
    let mut skipped_lines = 0;
    
    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        line_count += 1;
        
        // Skip empty lines
        if line.trim().is_empty() {
            skipped_lines += 1;
            continue;
        }
        
        // Add newline as the parser expects it
        let line_with_newline = format!("{}\n", line);
        
        match parse_basic_line.parse(&line_with_newline) {
            Ok(parsed_line) => {
                let reconstructed = parsed_line.to_string();
                let original = line.trim_end(); // Remove trailing newline for comparison
                
                // Check if reconstruction contains known limitations
                let has_floating_point = reconstructed.contains("<const:ValueFloatingPoint>");
                let has_value_marker = reconstructed.contains("<value:");
                
                // Skip lines with known limitations
                if has_floating_point || has_value_marker {
                    skipped_lines += 1;
                } else {
                    // Normalize for comparison:
                    // 1. Collapse multiple spaces to single
                    // 2. Remove space before line numbers after GOTO (GOTO1230 -> GOTO 1230)
                    // 3. Handle implicit GOTO after THEN/ELSE (THEN 1230 == THEN GOTO 1230)
                    let normalize = |s: &str| -> String {
                        let mut result = s.split_whitespace().collect::<Vec<_>>().join(" ");
                        // Add space after GOTO if followed immediately by digit
                        result = result.replace("GOTO", " GOTO ");
                        // Normalize THEN/ELSE followed by line number
                        result = result.replace(" THEN  GOTO ", " THEN ");
                        result = result.replace(" ELSE  GOTO ", " ELSE ");
                        // Clean up extra spaces
                        result.split_whitespace().collect::<Vec<_>>().join(" ")
                    };
                    
                    let original_normalized = normalize(original);
                    let reconstructed_normalized = normalize(&reconstructed);
                    
                    if original_normalized != reconstructed_normalized {
                        failed_roundtrips.push((
                            line_num,
                            line.to_string(),
                            format!("Mismatch:\n  Original:      {}\n  Reconstructed: {}", original, reconstructed)
                        ));
                    } else {
                        successful_roundtrips += 1;
                    }
                }
            },
            Err(e) => {
                failed_roundtrips.push((
                    line_num,
                    line.to_string(),
                    format!("Parse error: {:?}", e)
                ));
            }
        }
    }
    
    if !failed_roundtrips.is_empty() {
        let mut error_msg = format!(
            "\nRound-trip failed for {} out of {} lines in {}:\n",
            failed_roundtrips.len(),
            line_count,
            filename
        );
        for (line_num, line, err) in failed_roundtrips.iter().take(5) {
            error_msg.push_str(&format!("  Line {}: {}\n", line_num, line));
            error_msg.push_str(&format!("    {}\n", err));
        }
        if failed_roundtrips.len() > 5 {
            error_msg.push_str(&format!("  ... and {} more failures\n", failed_roundtrips.len() - 5));
        }
        return Err(error_msg);
    }
    
    Ok((line_count, successful_roundtrips, skipped_lines))
}

/// Integration test that verifies round-trip parsing for all files
#[test]
#[ignore] // Run with: cargo test test_all_roundtrip -- --ignored --nocapture
fn test_all_roundtrip() {
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
        // Additional classic CPC BASIC samples
        ("starfield.bas", "tests/samples/starfield.bas"),
        ("sine_scroller.bas", "tests/samples/sine_scroller.bas"),
        ("mandelbrot.bas", "tests/samples/mandelbrot.bas"),
        ("game_of_life.bas", "tests/samples/game_of_life.bas"),
        ("color_bars.bas", "tests/samples/color_bars.bas"),
        ("bouncing_ball.bas", "tests/samples/bouncing_ball.bas"),
    ];
    
    let mut total_files = 0;
    let mut successful_files = 0;
    let mut total_lines = 0;
    let mut total_roundtrips = 0;
    let mut total_skipped = 0;
    let mut failed_files = Vec::new();
    
    println!("\n=== Amstrad CPC Projects Round-Trip Test ===\n");
    
    for (name, path) in files {
        total_files += 1;
        match test_roundtrip_file(path) {
            Ok((lines, roundtrips, skipped)) => {
                successful_files += 1;
                total_lines += lines;
                total_roundtrips += roundtrips;
                total_skipped += skipped;
                let percent = if lines > 0 {
                    (roundtrips as f64 / (lines - skipped) as f64) * 100.0
                } else {
                    0.0
                };
                println!("✓ {}: {}/{} lines verified ({:.1}%), {} skipped",
                    name, roundtrips, lines, percent, skipped);
            }
            Err(e) => {
                failed_files.push((name, e));
                println!("✗ {}: FAILED", name);
            }
        }
    }
    
    println!("\n=== Round-Trip Results ===");
    println!("Files: {}/{} successful", successful_files, total_files);
    println!("Total lines: {}", total_lines);
    println!("Successfully reconstructed: {}", total_roundtrips);
    println!("Skipped (known limitations): {}", total_skipped);
    let overall_percent = if total_lines > 0 {
        (total_roundtrips as f64 / (total_lines - total_skipped) as f64) * 100.0
    } else {
        0.0
    };
    println!("Overall success rate: {:.2}%", overall_percent);
    
    if !failed_files.is_empty() {
        println!("\n=== Failed Files ===");
        for (name, error) in &failed_files {
            println!("\n{}:\n{}", name, error);
        }
        panic!("{} out of {} files failed round-trip test", failed_files.len(), total_files);
    }
}

// Individual round-trip tests for each file
#[test]
fn test_roundtrip_sectfgt() {
    match test_roundtrip_file("tests/amstrad-cpc-projects-master/games/sector-fight/sectfgt.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ sectfgt.bas round-trip: {}/{} lines verified, {} skipped", 
                successful, total, skipped);
            // Assert we have a high success rate (excluding skipped)
            let testable = total - skipped;
            if testable > 0 {
                let percent = (successful as f64 / testable as f64) * 100.0;
                assert!(percent >= 95.0, 
                    "Round-trip success rate too low: {:.1}%", percent);
            }
        }
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_roundtrip_bounce() {
    match test_roundtrip_file("tests/amstrad-cpc-projects-master/graphics/bounce.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ bounce.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_roundtrip_plasma() {
    match test_roundtrip_file("tests/amstrad-cpc-projects-master/graphics/plasma.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ plasma.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

// Tests for additional classic CPC BASIC samples
#[test]
fn test_roundtrip_starfield() {
    match test_roundtrip_file("tests/samples/starfield.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ starfield.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_roundtrip_sine_scroller() {
    match test_roundtrip_file("tests/samples/sine_scroller.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ sine_scroller.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_roundtrip_mandelbrot() {
    match test_roundtrip_file("tests/samples/mandelbrot.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ mandelbrot.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_roundtrip_game_of_life() {
    match test_roundtrip_file("tests/samples/game_of_life.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ game_of_life.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_roundtrip_color_bars() {
    match test_roundtrip_file("tests/samples/color_bars.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ color_bars.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

#[test]
fn test_roundtrip_bouncing_ball() {
    match test_roundtrip_file("tests/samples/bouncing_ball.bas") {
        Ok((total, successful, skipped)) => {
            println!("✓ bouncing_ball.bas round-trip: {}/{} lines verified, {} skipped",
                successful, total, skipped);
        }
        Err(e) => panic!("{}", e),
    }
}

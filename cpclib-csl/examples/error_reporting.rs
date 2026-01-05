/// Example demonstrating the enhanced error reporting feature
/// 
/// This example shows how to use parse_csl_with_rich_errors to get beautiful
/// error messages with line/column numbers and helpful suggestions.

use cpclib_csl::{parse_csl_with_rich_errors, parse_csl};

fn main() {
    println!("=== CSL Parser Error Reporting Example ===\n");
    
    // Example 1: Valid CSL script
    println!("Example 1: Parsing valid CSL script");
    let valid_script = r#"
csl_version 1.1
reset
disk_insert 'game.dsk'
wait 1000000
"#;
    
    match parse_csl_with_rich_errors(valid_script, Some("valid.csl".to_string())) {
        Ok(script) => println!("✓ Successfully parsed {} instructions\n", script.instructions.len()),
        Err(e) => println!("✗ Error: {}\n", e.format_error()),
    }
    
    // Example 2: Script with incomplete syntax
    println!("Example 2: Incomplete syntax (missing argument)");
    let incomplete_script = "disk_insert\nwait 1000\n";
    
    match parse_csl_with_rich_errors(incomplete_script, Some("incomplete.csl".to_string())) {
        Ok(_) => println!("✓ Parsed successfully\n"),
        Err(e) => {
            println!("✗ Parse error detected!");
            println!("{}\n", e.format_error());
        }
    }
    
    // Example 3: Using the legacy parse_csl function
    println!("Example 3: Legacy parse_csl (no rich errors)");
    match parse_csl(incomplete_script) {
        Ok(_) => println!("✓ Parsed successfully\n"),
        Err(e) => println!("✗ Error (basic): {:?}\n", e),
    }
    
    // Example 4: Valid script showing all features work
    println!("Example 4: Complex valid script");
    let complex_script = r#"
csl_version 1.1
reset soft
cpc_model 2
memory_exp 3
disk_dir '/games'
disk_insert A 'menu.dsk'
tape_insert 'loader.cdt'
tape_play
key_output 'RUN"DISK\(RET)'
wait 5000000
screenshot
snapshot
"#;
    
    match parse_csl_with_rich_errors(complex_script, Some("complex.csl".to_string())) {
        Ok(script) => {
            println!("✓ Successfully parsed {} instructions:", script.instructions.len());
            for (i, instr) in script.instructions.iter().enumerate() {
                println!("  {}. {:?}", i + 1, instr);
            }
        },
        Err(e) => {
            println!("✗ Error:");
            println!("{}", e.format_error());
        }
    }
}

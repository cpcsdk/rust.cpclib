// Test to verify that "catalog rename" correctly fails
use cpclib_catalog::cli::CatalogApp;
use clap::Parser;

#[test]
fn test_catalog_rename_should_fail() {
    // This should fail because "rename" is not a valid subcommand
    let result = CatalogApp::try_parse_from(&[
        "catalog",
        "rename",
        "game.dsk",
        "TEMP.BIN",
        "FINAL.BIN"
    ]);
    
    println!("Parse result: {:?}", result);
    
    match result {
        Ok(parsed) => {
            panic!("Command 'catalog rename' should fail but it succeeded! Parsed: {:?}", parsed);
        },
        Err(e) => {
            println!("Error (as expected): {}", e);
            let err_string = e.to_string();
            // Verify it's an invalid subcommand error
            assert!(
                err_string.contains("unrecognized subcommand") || 
                err_string.contains("invalid") ||
                err_string.contains("rename"),
                "Expected 'invalid subcommand' error mentioning 'rename', got: {}", err_string
            );
        }
    }
}

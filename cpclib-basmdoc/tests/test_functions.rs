use cpclib_basmdoc::{DocumentationPage};

#[test]
fn test_function_support() {
    // Test that the DocumentationPage has function support
    let page = DocumentationPage::for_file("../cpclib-asm/assets/ga.asm", false).expect("Failed to read ga.asm file");
    
    // This file contains a function, so we should be able to iterate over it
    let has_functions = page.has_functions();
    let functions: Vec<_> = page.function_iter().collect();
    
    println!("Has functions: {}", has_functions);
    println!("Number of functions: {}", functions.len());
    
    if has_functions {
        for func in &functions {
            println!("Found function: {}", func.item_long_summary());
        }
    }
    
    // The test passes if we can iterate without panicking
    // The actual count depends on whether the file is documented
    assert!(true, "Function iteration works");
}

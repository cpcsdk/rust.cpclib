use cpclib_asm::parse_z80_str;
use cpclib_basmdoc::{aggregate_documentation_on_tokens, build_documentation_page_from_aggregates};

#[test]
fn test_local_labels() {
    let code = std::fs::read_to_string("tests/local_labels.asm").unwrap();
    let tokens = parse_z80_str(&code).unwrap();
    let doc = aggregate_documentation_on_tokens(&tokens);
    
    let page = build_documentation_page_from_aggregates("tests/local_labels.asm", doc);
    
    // Check that we have the expected documented items
    // Should have: file comment + main_loop + main_loop.loop + process_data + process_data.inner
    // .orphan_local should be skipped (no parent)
    
    let label_names: Vec<String> = page.label_iter()
        .map(|item| item.item_key().trim_start_matches("label_").to_string())
        .collect();
    
    println!("Found labels: {:?}", label_names);
    
    // Check that global labels exist
    assert!(label_names.contains(&"main_loop".to_string()), "main_loop should be present");
    assert!(label_names.contains(&"process_data".to_string()), "process_data should be present");
    
    // Check that local labels are prefixed with their parent
    assert!(label_names.contains(&"main_loop.loop".to_string()), "main_loop.loop should be present");
    assert!(label_names.contains(&"process_data.inner".to_string()), "process_data.inner should be present");
    
    // Check that orphan local label (before any global label) is NOT present
    assert!(!label_names.contains(&".orphan_local".to_string()), ".orphan_local should NOT be present (no parent)");
    assert!(!label_names.iter().any(|l| l.contains("orphan")), "No label should contain 'orphan'");
    
    // Test HTML output can be generated
    let html = page.to_html();
    assert!(html.contains("main_loop"), "HTML should contain main_loop");
    assert!(html.contains("main_loop.loop"), "HTML should contain main_loop.loop");
}

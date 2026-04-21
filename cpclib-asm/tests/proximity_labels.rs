/// Tests for proximity labels (inspired by spasm-ng/rasm)
/// Proximity labels allow using _ as an anonymous label
/// with _+ referring to the next _ and _- referring to the previous _
use cpclib_asm;

#[test]
fn test_proximity_label_forward() {
    // Test basic forward reference with _+
    let code = r#"
        org 0
        nop
        jr _+
        nop
_       nop
        djnz _-
    "#;
    
    let result = cpclib_asm::assemble(code);
    assert!(result.is_ok(), "Failed to assemble: {:?}", result.err());
    
    let bytes = result.unwrap();
    // nop = 0x00
    // jr _+ = 0x18 0x01 (jump forward 1 byte, skipping one nop)
    // nop = 0x00  
    // nop = 0x00 (this is the _ label)
    // djnz _- = 0x10 0xFD (loop back to _ label, PC+0xFD from after instruction)
    assert_eq!(bytes.len(), 7);
    assert_eq!(bytes[0], 0x00); // nop
    assert_eq!(bytes[1], 0x18); // jr opcode
    assert_eq!(bytes[2], 0x01); // jr offset (skip 1 nop)
    assert_eq!(bytes[3], 0x00); // nop
    assert_eq!(bytes[4], 0x00); // nop at _ label
    assert_eq!(bytes[5], 0x10); // djnz opcode
    assert_eq!(bytes[6], 0xFD); // djnz offset (PC is at 7, jump to 4: 4-7=-3=0xFD)
}

#[test]
fn test_proximity_label_backward() {
    // Test basic backward reference with _-
    let code = r#"
        org 0
_       nop
        djnz _-
    "#;
    
    let result = cpclib_asm::assemble(code);
    assert!(result.is_ok(), "Failed to assemble: {:?}", result.err());
    
    let bytes = result.unwrap();
    // nop = 0x00 (this is the _ label at address 0)
    // djnz _- = 0x10 0xFD (loop back to address 0; PC=3 after instruction, 0-3=-3=0xFD)
    assert_eq!(bytes.len(), 3);
    assert_eq!(bytes[0], 0x00); // nop at _ label
    assert_eq!(bytes[1], 0x10); // djnz opcode
    assert_eq!(bytes[2], 0xFD); // relative offset to loop back (PC is at 3, target is 0: 0-3=-3 in 2's complement = 0xFD)
}

#[test]
fn test_multiple_proximity_labels() {
    // Test multiple proximity labels in sequence
    let code = r#"
        org 0
        jr _+
_       nop
        jr _+
_       push hl
        djnz _-
    "#;
    
    let result = cpclib_asm::assemble(code);
    assert!(result.is_ok(), "Failed to assemble: {:?}", result.err());
    
    let bytes = result.unwrap();
    // First jr _+ should jump to first _ (nop)
    // Second jr _+ should jump to second _ (push hl)
    // djnz _- should loop back to second _ (push hl)
    assert!(bytes.len() > 0);
}

#[test]
fn test_proximity_labels_in_repeat() {
    // Test proximity labels work inside loops (from rasm test suite)
    let code = r#"
        org 0
        repeat 2
            nop
            jr _+
_           nop
            djnz _-
            defs 256
            jr _+
_           push hl
            djnz _-
            defs 256
        rend
    "#;
    
    let result = cpclib_asm::assemble(code);
    assert!(result.is_ok(), "Proximity labels should work in repeat: {:?}", result.err());
}

#[test]
fn test_proximity_label_error_backward_before_any_definition() {
    // Test error when using _- before any _ is defined
    let code = r#"
        org 0
        nop
        jr _-
    "#;
    
    let result = cpclib_asm::assemble(code);
    // This should fail because we're trying to use _- before defining any _ label
    assert!(result.is_err(), "Should error when using _- before defining _");
    
    let error_msg = format!("{:?}", result.err().unwrap());
    assert!(error_msg.contains("_-") || error_msg.contains("proximity"), 
            "Error should mention proximity label issue: {}", error_msg);
}

#[test]
fn test_proximity_label_error_forward_no_definition() {
    // Test error when using _+ but never defining the next _
    let code = r#"
        org 0
        nop
        jr _+
    "#;
    
    let result = cpclib_asm::assemble(code);
    // This should fail because _+ refers to a label that's never defined
    assert!(result.is_err(), "Should error when _+ refers to undefined label");
}

#[test]
fn test_proximity_labels_complex_sequence() {
    // More complex test with multiple forward and backward references
    let code = r#"
        org 0
        nop           ; address 0
        jr _+         ; jump to first _ (address 3)
        halt          ; should be skipped
_       nop           ; first _ label (address 3)
        djnz _-       ; loop back to first _ (address 3)
        jr _+         ; jump to second _ (address 8)
        halt          ; should be skipped
_       push hl       ; second _ label (address 8)
        djnz _-       ; loop back to second _ (address 8)
    "#;
    
    let result = cpclib_asm::assemble(code);
    assert!(result.is_ok(), "Complex proximity label sequence should work: {:?}", result.err());
    
    let bytes = result.unwrap();
    // Expected: [00, 18, 01, 76, 00, 10, FD, 18, 01, 76, E5, 10, FD]
    //            nop jr +1  halt nop djnz jr +1  halt push djnz
    assert_eq!(bytes.len(), 13);
    // Verify that halt instructions are in code at correct positions
    assert_eq!(bytes[3], 0x76); // first halt should be in code but skipped by jr
    assert_eq!(bytes[9], 0x76); // second halt should also be in code but skipped by jr
    // Verify jr offsets correctly skip the halts
    assert_eq!(bytes[2], 0x01); // first jr offset (+1 to skip halt)
    assert_eq!(bytes[8], 0x01); // second jr offset (+1 to skip halt)
}

#[test]
fn test_proximity_labels_independent_of_normal_labels() {
    // Verify that proximity labels don't interfere with normal labels
    let code = r#"
        org 0
main    nop
        jr _+
_       nop
.local  nop
        jr _+
global  nop
_       push hl
        djnz _-
        jr main
    "#;
    
    let result = cpclib_asm::assemble(code);
    assert!(result.is_ok(), "Proximity labels should coexist with normal labels: {:?}", result.err());
}

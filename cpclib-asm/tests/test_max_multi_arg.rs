#[test]
fn test_max_multi_arg_equ() {
    let src = include_str!("test_max_multi_arg.asm");
    let result = cpclib_asm::assemble(src);
    assert!(result.is_ok(), "Assembler failed: {:?}", result.err());
    let bin = result.unwrap();
    // The value should be the max, i.e., 50
    // ld a, 50 is 3E 32
    assert!(bin.contains(&0x3E));
    assert!(bin.contains(&0x32));
}

#[test]
fn test_max_multi_arg_missing_data() {
    let src = include_str!("test_max_multi_arg_missing_file.asm");
    let result = cpclib_asm::assemble(src);
    assert!(
        result.is_err(),
        "Assembler should have failed but succeeded"
    );

    let msg = result.err().unwrap().to_string();
    eprintln!("Error: {}", &msg);
    assert!(
        msg.contains("not found"),
        "Error message does not mention missing file"
    );
}

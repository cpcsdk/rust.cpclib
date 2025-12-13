use cpclib_asm;

#[test]
fn assemble_with_directives() {
    // Test assembler directives: org, equ, db, dw and label reference
    let code = r#"
    org &400
MYVAL:
    db MYVAL, MYVAL+1, MYVAL*2
    if MYVAL > 5
    db 99
    endif
label:
    dw label
    db 0xFF
"#;
    let bytes = cpclib_asm::assemble(code).expect("assemble failed");
    // Expected:
    // db MYVAL, MYVAL+1, MYVAL*2 => low bytes of label address: 0x00,0x01,0x00
    // label MYVAL is at absolute address 0x400 (org &400) so the db emits
    // the low byte(s) of that value. The conditional `db 99` is included
    // when MYVAL > 5 (true for address 0x400), so there are 4 bytes before
    // `label:` and thus `label` is at 0x404.
    // dw label => little-endian 0x404 => 0x04, 0x04
    // db 0xFF => 255
    let expected = vec![0u8, 1, 0, 99, 4, 4, 255];
    assert_eq!(bytes, expected);
}

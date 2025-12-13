use cpclib_asm;

#[test]
fn assemble_simple_db() {
    let code = "org 0\n db 1,2,3,4\n";
    let bytes = cpclib_asm::assemble(code).expect("assemble failed");
    assert_eq!(bytes, vec![1u8, 2, 3, 4]);
}

use cpclib_asm;

/// A near, forward JQ target must assemble exactly like the equivalent JR.
#[test]
fn jq_in_range_matches_jr() {
    let jq = cpclib_asm::assemble("org 0\n jq label\nlabel: nop\n").expect("assemble failed");
    let jr = cpclib_asm::assemble("org 0\n jr label\nlabel: nop\n").expect("assemble failed");
    assert_eq!(jq, jr);
    assert_eq!(jq, vec![0x18, 0x00, 0x00]);
}

/// A conditional, near, forward JQ target must assemble exactly like the
/// equivalent conditional JR.
#[test]
fn jq_flag_in_range_matches_jr() {
    let jq = cpclib_asm::assemble("org 0\n jq nz, label\nlabel: nop\n").expect("assemble failed");
    let jr = cpclib_asm::assemble("org 0\n jr nz, label\nlabel: nop\n").expect("assemble failed");
    assert_eq!(jq, jr);
}

/// A target far enough away that a relative jump cannot reach it must fall
/// back to the equivalent JP - no reachability analysis, just try/fallback.
#[test]
fn jq_out_of_range_matches_jp() {
    let code = "org 0\n jq label\n defs 200\nlabel: nop\n";
    let jq = cpclib_asm::assemble(code).expect("assemble failed");
    let jp = cpclib_asm::assemble(&code.replace("jq", "jp")).expect("assemble failed");
    assert_eq!(jq, jp);
    assert_eq!(jq[0], 0xC3);
    assert_eq!(jq.len(), 3 + 200 + 1);
    let target = u16::from_le_bytes([jq[1], jq[2]]);
    assert_eq!(target as usize, jq.len() - 1);
}

/// Same out-of-range fallback, but with a flag test.
#[test]
fn jq_flag_out_of_range_matches_jp() {
    let code = "org 0\n jq c, label\n defs 200\nlabel: nop\n";
    let jq = cpclib_asm::assemble(code).expect("assemble failed");
    let jp = cpclib_asm::assemble(&code.replace("jq", "jp")).expect("assemble failed");
    assert_eq!(jq, jp);
}

/// A backward JQ target in range must also assemble like the equivalent JR.
#[test]
fn jq_backward_in_range_matches_jr() {
    let code = "org 0\nlabel: nop\n jq label\n";
    let jq = cpclib_asm::assemble(code).expect("assemble failed");
    let jr = cpclib_asm::assemble(&code.replace("jq", "jr")).expect("assemble failed");
    assert_eq!(jq, jr);
}

/// A backward JQ target too far away must fall back to JP.
#[test]
fn jq_backward_out_of_range_matches_jp() {
    let code = "org 0\nlabel: nop\n defs 200\n jq label\n";
    let jq = cpclib_asm::assemble(code).expect("assemble failed");
    let jp = cpclib_asm::assemble(&code.replace("jq", "jp")).expect("assemble failed");
    assert_eq!(jq, jp);
    assert_eq!(*jq.last_chunk::<3>().unwrap().first().unwrap(), 0xC3);
}

/// The boundary itself: a target exactly at the edge of JR's +127 reach must
/// still use JR, and one byte further must switch to JP.
#[test]
fn jq_boundary_is_exact() {
    // JR's own reach is delta in -128..=127 where delta = target - (pc + 2).
    // With the jq at address 0 (2 bytes if JR), a target at address 129 is
    // delta=127 (still fits); at address 130 delta=128 (does not fit).
    let fits = "org 0\n jq label\n defs 127\nlabel: nop\n";
    let bytes = cpclib_asm::assemble(fits).expect("assemble failed");
    assert_eq!(bytes[0], 0x18);

    let overflows = "org 0\n jq label\n defs 128\nlabel: nop\n";
    let bytes = cpclib_asm::assemble(overflows).expect("assemble failed");
    assert_eq!(bytes[0], 0xC3);
}

/// A genuinely far jump (several KB away, nowhere near the ±127 boundary)
/// must fall back to JP too, and the address bytes must be exact - not just
/// "some fallback happened to fit", covering the case a boundary-only test
/// wouldn't catch (e.g. a sign/overflow bug in the delta computation itself
/// only showing up for a large distance).
#[test]
fn jq_far_jump_matches_jp() {
    let code = "org 0\n jq label\n defs 10000\nlabel: nop\n";
    let jq = cpclib_asm::assemble(code).expect("assemble failed");
    let jp = cpclib_asm::assemble(&code.replace("jq", "jp")).expect("assemble failed");
    assert_eq!(jq, jp);
    assert_eq!(jq[0], 0xC3);
    assert_eq!(jq.len(), 3 + 10000 + 1);
    let target = u16::from_le_bytes([jq[1], jq[2]]);
    assert_eq!(target as usize, jq.len() - 1);
}

/// Same far jump, backward this time.
#[test]
fn jq_far_backward_jump_matches_jp() {
    let code = "org 0\nlabel: nop\n defs 10000\n jq label\n";
    let jq = cpclib_asm::assemble(code).expect("assemble failed");
    let jp = cpclib_asm::assemble(&code.replace("jq", "jp")).expect("assemble failed");
    assert_eq!(jq, jp);
    let target = u16::from_le_bytes([jq[jq.len() - 2], jq[jq.len() - 1]]);
    assert_eq!(target, 0);
}

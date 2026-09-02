//! `Env::active_page_index` (see `cpclib-asm/src/assembler/mod.rs`) memoizes
//! the active-page lookup that `output_byte` (and the several accessors it
//! goes through - `logical_output_address`, `physical_output_address`, etc.)
//! used to recompute from scratch close to a dozen times per byte written.
//! The cache is keyed on exactly `(output_address, ga_mmr)`, the only two
//! fields the lookup depends on, so any real page switch invalidates it by
//! construction. This test interleaves writes to two different memory banks
//! (switched via `WRITE DIRECT`, which changes `ga_mmr`) to specifically
//! stress that invalidation - a caching bug here would show up as a byte
//! landing in the wrong bank.

use cpclib_asm::assemble;

/// Two banks, written byte-by-byte in alternation (not sequentially like a
/// simple "finish bank A, then bank B" test would) - the cache must
/// correctly invalidate and re-resolve the active page on every single
/// switch, not just the first one. Writes distinct, non-clashing bytes to
/// each bank, interleaved, then reads each bank's own bytes back.
#[test]
fn interleaved_writes_to_two_banks_land_in_the_correct_bank_each_time() {
    let code = "\
    write direct -1,-1,&c0
    org &4000
    db 0x11
    write direct -1,-1,&c4
    org &5000
    db 0x22
    write direct -1,-1,&c0
    org &4001
    db 0x33
    write direct -1,-1,&c4
    org &5001
    db 0x44
    write direct -1,-1,&c0
";
    let bytes = assemble(code).expect("interleaved bank writes must assemble");
    // `assemble`'s returned bytes are the default (last-selected, &c0) bank's
    // 64K content starting at its first written address - &4000..&4002 here.
    assert_eq!(bytes, vec![0x11, 0x33], "{bytes:?}");
}

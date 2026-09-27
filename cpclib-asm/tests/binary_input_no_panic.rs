use cpclib_asm::parser::parse_z80_str;

/// The bytes of a real palette file (`birthtro/data/loading.pal`), which the
/// LSP once handed to the parser because the project `incbin`s it.
const REAL_PAL: &str = "TDU\\]LMFW^NOSJKT\nT";

#[test]
fn a_palette_file_is_an_error_or_a_listing_never_a_panic() {
    let _ = parse_z80_str(REAL_PAL);
}

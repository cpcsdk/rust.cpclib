use cpclib_asm::assembler::visit_tokens_all_passes_with_options;
use cpclib_asm::preamble::{parse_z80_str_with_context, ParserContext};
use cpclib_asm::AssemblingOptions;
use cpclib_sna::Snapshot;

fn build_sna(code: &str) -> Snapshot {
    let ctx = ParserContext::default();
    let options = AssemblingOptions::default();
    let listing = parse_z80_str_with_context(code, ctx).expect("Unable to parse z80 code");
    let (_, env) = visit_tokens_all_passes_with_options(&listing, &options, listing.ctx())
        .expect("Unable to assemble z80 code");
    let sna = env.sna().clone();

    return sna;
}

fn main() {
    eprintln!("Launch snapshots generation for manual testing in emulators.");

    let asm = "
		org 0x4000
		run $
		jp $
	";

    let sna = build_sna(asm);
    sna.save("/tmp/loop4000_v3.sna", cpclib_sna::SnapshotVersion::V3);
    sna.save("/tmp/loop4000_v2.sna", cpclib_sna::SnapshotVersion::V2);

    assert_eq!(sna.get_byte(0x4000), 0xC3);
    assert_eq!(sna.get_byte(0x4001), 0x00);
    assert_eq!(sna.get_byte(0x4002), 0x40);

    eprintln!("Everything went fine.");
}
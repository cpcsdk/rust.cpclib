//! An Amstrad Plus snapshot has to announce the machine it runs on.
//!
//! The display code generated for a Plus palette drives the ASIC - it unlocks
//! it, writes 32 bytes of 12-bit colour, and locks it again. An emulator told
//! it is running a plain CPC has none of that hardware, so the snapshot names
//! both the CRTC it wants - type 3, the 6845 inside the ASIC - and the machine.
//! Those fields arrived with version 3 of the snapshot format, so a Plus
//! snapshot is written as V3 where a CPC one stays V2.

use cpclib_common::event::DiscardObserver;
use cpclib_imgconverter::{build_img2cpc_args_parser, process_img2cpc};

const VERSION: usize = 0x10;
const CPC_TYPE: usize = 0x6D;
const CRTC_TYPE: usize = 0xA4;

/// Convert the Plus test image to a snapshot and hand back its header.
fn snapshot_header(extra: &[&str], name: &str) -> Vec<u8> {
    let out = std::env::temp_dir().join(name);
    let _ = std::fs::remove_file(&out);
    let out = out.to_str().unwrap().to_owned();

    let mut argv = vec!["img2cpc", "--mode", "0"];
    argv.extend_from_slice(extra);
    argv.extend_from_slice(&["tests/plus_sprite.png", "sna", &out]);

    let args = build_img2cpc_args_parser();
    let matches = args.clone().get_matches_from(argv);
    process_img2cpc(&matches, args, &DiscardObserver).expect("the conversion must succeed");

    let mut header = std::fs::read(&out).expect("the snapshot must have been written");
    header.truncate(0x100);
    header
}

#[test]
fn a_plus_snapshot_selects_the_asic_crtc() {
    let header = snapshot_header(&["--plus"], "cpclib_plus_test.sna");
    assert_eq!(header[CRTC_TYPE], 3, "the CRTC inside the ASIC");
    assert_eq!(header[CPC_TYPE], 4, "6128 Plus");
    assert_eq!(
        header[VERSION], 3,
        "CRTC_TYPE and CPC_TYPE only exist from version 3"
    );
}

/// The Gate Array path must be left exactly as it was.
#[test]
fn a_cpc_snapshot_is_unchanged() {
    let header = snapshot_header(&[], "cpclib_cpc_test.sna");
    assert_eq!(header[VERSION], 2);
    assert_eq!(header[CRTC_TYPE], 0);
    assert_eq!(header[CPC_TYPE], 0);
}

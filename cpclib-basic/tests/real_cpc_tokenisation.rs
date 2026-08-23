//! Byte-exact regression tests against real Amstrad CPC hardware.
//!
//! Each fixture DSK holds two AMSDOS `.BAS` files: our own tool's output
//! for a source program, and a second file with the same program re-saved
//! by a real CPC after a "Syntax error" edit-and-accept round trip on the
//! first one. Comparing the two is what found several real tokeniser bugs
//! (variable references encoded as raw ASCII with no wire-format marker at
//! all, GOTO/GOSUB targets missing the dedicated LineNumber token,
//! assignment `=` using the wrong token, `PI` not recognised as a keyword,
//! and small integer literals not using the compact encodings a real CPC
//! writes for a `LET`/assignment right-hand side). These tests assert our
//! *current* tokeniser output is byte-identical to the real CPC's own
//! save, so any of those regressing is caught immediately - not just "does
//! this parse", which every one of these bugs still did.

use cpclib_disc::amsdos::AmsdosFileName;
use cpclib_disc::disc::Disc;
use cpclib_disc::edsk::Head;

/// Splits a tokenised program's raw content bytes (i.e. without the
/// 128-byte AMSDOS header) into one slice per line, using each line's own
/// stored length field - the same approach `cpclib_basic::binary_parser`
/// uses, but without decoding, so a byte-level mismatch is never masked by
/// a decode-then-re-encode round trip.
fn raw_lines(content: &[u8]) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    let mut rest = content;
    loop {
        if rest.len() < 4 || (rest[0] == 0 && rest[1] == 0) {
            break;
        }
        let len = u16::from_le_bytes([rest[0], rest[1]]) as usize;
        lines.push(rest[..len].to_vec());
        rest = &rest[len..];
    }
    lines
}

/// The AMSDOS file's content bytes (header stripped), read from `dsk`.
fn read_amsdos_content(dsk: &str, filename: &str) -> Vec<u8> {
    let path = cpclib_common::camino::Utf8PathBuf::from(dsk);
    let disc = cpclib_disc::open_disc(&path, true)
        .unwrap_or_else(|e| panic!("could not open {dsk}: {e}"));
    let fname = AmsdosFileName::try_from(filename)
        .unwrap_or_else(|e| panic!("{filename}: invalid AMSDOS filename: {e:?}"));
    let file = disc
        .get_amsdos_file(Head::A, fname)
        .unwrap_or_else(|e| panic!("{dsk}: error reading {filename}: {e:?}"))
        .unwrap_or_else(|| panic!("{filename} not found on {dsk}"));
    file.header_and_content()[128..].to_vec()
}

/// Asserts our tokeniser's current output for `source` is byte-identical,
/// line by line, to `filename` on `dsk` - a real CPC's own save of the
/// same program.
fn assert_matches_real_cpc(source: &str, dsk: &str, filename: &str) {
    let ours = cpclib_basic::BasicProgram::parse(source)
        .unwrap_or_else(|e| panic!("parse error for the fixture source: {e}"))
        .as_bytes();
    let our_lines = raw_lines(&ours);

    let theirs = read_amsdos_content(dsk, filename);
    let their_lines = raw_lines(&theirs);

    assert_eq!(
        our_lines.len(),
        their_lines.len(),
        "different number of lines than {filename}"
    );

    for (i, (ours, theirs)) in our_lines.iter().zip(their_lines.iter()).enumerate() {
        assert_eq!(
            ours, theirs,
            "line index {i}: our encoding does not match {filename}'s own bytes\nours:   {ours:02X?}\ntheirs: {theirs:02X?}"
        );
    }
}

#[test]
fn hello_matches_a_real_cpcs_own_save() {
    assert_matches_real_cpc(
        "10 print \"hello\"\n20 goto 10\n",
        "tests/fixtures/hello_real_cpc.dsk",
        "HELLO2.BAS"
    );
}

#[test]
fn pendulum_matches_a_real_cpcs_own_save() {
    let source = "10 mode 1\n\
                  20 theta=pi/2\n\
                  30 g=9.81\n\
                  40 l=1\n\
                  50 sp=0 : px=320 : py=300 : bx=px : by=py\n\
                  100 move bx-4,by+4:graphics pen 0:tag:print chr$(231);:tagoff\n\
                  110 move px,py:draw bx,by\n\
                  120 bx=px+l*250*sin(theta)\n\
                  130 by=py+l*250*cos(theta)\n\
                  140 move bx-4,by+4:graphics pen 1:tag:print chr$(231);:tagoff\n\
                  150 move px,py:draw bx,by\n\
                  160 accel=g*sin(theta)/l/100\n\
                  170 sp=sp+accel/100\n\
                  180 theta=theta+sp\n\
                  190 frame\n\
                  200 goto 100\n";

    assert_matches_real_cpc(
        source,
        "tests/fixtures/pendulum_real_cpc.dsk",
        "PENDULU2.BAS"
    );
}

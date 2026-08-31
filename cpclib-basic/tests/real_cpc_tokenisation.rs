//! Byte-exact regression tests against real Amstrad CPC hardware.
//!
//! Each fixture DSK holds two AMSDOS `.BAS` files: our own tool's output
//! for a source program, and a second file with the same program re-saved
//! by a real CPC after a "Syntax error" edit-and-accept round trip on the
//! first one. Comparing the two is what found several real tokeniser bugs:
//! variable references encoded as raw ASCII with no wire-format marker at
//! all; GOTO/GOSUB targets and FOR's start/end/step missing the dedicated
//! LineNumber/compact-literal treatment a real CPC gives them; assignment
//! `=` (including FOR's own) using the wrong token; `PI` not recognised as
//! a keyword; small integer literals not using the compact encodings a
//! real CPC writes for a `LET`/assignment right-hand side (but *not* for a
//! command or function argument - the actual rule, found the hard way);
//! and DEFINT/DEFSTR/DEFREAL not changing the type marker of later
//! sigil-less references to a variable starting with a declared letter.
//! These tests assert our *current* tokeniser output is byte-identical to
//! the real CPC's own save, so any of those regressing is caught
//! immediately - not just "does this parse", which every one of these
//! bugs still did.

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
/// `skip_lines`: 0-based line indices to exempt from the byte comparison.
///
/// Not every line in a *_real_cpc.dsk fixture is trustworthy ground truth:
/// only the lines that actually triggered a "Syntax error" on the real
/// machine got retokenised by it when the user re-accepted them. A line
/// with no variable reference, no assignment, no GOTO/GOSUB and no other
/// bug this crate has ever had never errored, so it was never retyped -
/// its bytes are just whatever *our own* tool originally wrote, silently
/// preserved. Confirmed for the exact lines this skips: neither ever
/// appeared as a difference at any point while these three fixtures were
/// being investigated and fixed one bug at a time, unlike every other line
/// in the same programs (which all changed as bugs were found and fixed) -
/// the telltale sign of a line that was simply never touched.
fn assert_matches_real_cpc(source: &str, dsk: &str, filename: &str, skip_lines: &[usize]) {
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
        if skip_lines.contains(&i) {
            continue;
        }
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
        "HELLO2.BAS",
        &[]
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

    // Line 0 ("10 mode 1") never contains a variable, assignment, or
    // GOTO/GOSUB target - nothing that ever errored on a real CPC - so it
    // was never retyped and is not reliable ground truth for MODE's own
    // argument encoding (see `assert_matches_real_cpc`'s doc comment).
    // magic8b.bas's `MODE 2` line *was* forced to be retyped (by the
    // real, then-unrecognised `TIME` keyword later on the same line) and
    // is the reliable data point that MODE compacts its argument like an
    // assignment does.
    assert_matches_real_cpc(
        source,
        "tests/fixtures/pendulum_real_cpc.dsk",
        "PENDULU2.BAS",
        &[0]
    );
}

#[test]
fn mandelbrot_matches_a_real_cpcs_own_save() {
    // Exercises two bug classes the other two fixtures don't: FOR's own
    // `=`/TO/STEP need the same assignment-style token/compact-literal
    // treatment as a plain LET (not just its start value), and DEFINT
    // changes the *type marker* of every later sigil-less reference to a
    // variable starting with a declared letter - a property of the
    // variable itself, not of where it's referenced (a command argument
    // like `PLOT px,py` gets the DEFINT-implied marker too, unlike the
    // compact-literal question, which is purely about reference context).
    let source = "10 MODE 0:INK 14,1:INK 15,16\n\
                  20 DEFINT c,i,p\n\
                  30 itmax=64\n\
                  40 xs=1/213*4:ys=1/200*2\n\
                  50 y0=-1:FOR py=0 TO 199 STEP 2\n\
                  60 x0=-2:FOR px=0 TO 639 STEP 4\n\
                  65 xm=x0+1:IF xm*xm+y0*y0<0.0625 THEN 100\n\
                  70 it=0:x=0:y=0:x2=0:y2=0\n\
                  80 IF x2+y2<=4 AND it<itmax THEN y=2*x*y+y0:x=x2-y2+x0:x2=x*x:y2=y*y:it=it+1:GOTO 80\n\
                  90 IF it<itmax THEN c=it AND 15:PLOT px,py,c:PLOT px,398-py\n\
                  100 x0=x0+xs:NEXT\n\
                  110 y0=y0+ys:NEXT\n";

    // Line 0 ("10 MODE 0:INK 14,1:INK 15,16") never errored either - same
    // reasoning as pendulum's line 0 above.
    //
    // Line 6 ("65 xm=x0+1:IF ... THEN 100") is skipped for the same
    // "never independently retyped" reason, discovered later: this
    // fixture's own commit (e866270b) never mentions the implicit
    // THEN-linenum construct among the bug classes it verified, and
    // `parse_if` (the only code that produces this shape) was never
    // touched by either commit that built these fixtures. Its bytes here
    // (`... A0 1A 64 ...`, a synthetic `Goto` token plus the generic
    // 16-bit form) is this crate's own original, unverified guess, not
    // confirmed real-hardware ground truth - and it was wrong, reported
    // live: 1984js (a true ROM emulator) rendered these exact bytes as
    // "GO TO 100", a word the user never typed. `parse_if` now uses the
    // same dedicated `LineNumber` token `GOTO`/`GOSUB` already use
    // (verified via hello.bas's own "GOTO 10"), no synthetic `Goto`
    // marker - see `parse_if`'s own doc comment for the full reasoning.
    assert_matches_real_cpc(
        source,
        "tests/fixtures/mandelbrot_real_cpc.dsk",
        "MANDEL2.BAS",
        &[0, 6]
    );
}

#[test]
fn magic8b_matches_a_real_cpcs_own_save() {
    // Exercises several bug classes the other fixtures don't: PI's
    // niladic sibling TIME (`RANDOMIZE TIME`); DEFINT's letter *range*
    // form (`DEFINT a-z`), whose own `-` separator is a plain ASCII
    // hyphen (CharHyphen, &2d), not the arithmetic minus operator
    // (SubstractionOrUnaryMinus, &f5) my first DEFINT-range fix wrongly
    // used - which also silently broke the range being recognised as a
    // range at all, since the wrong token doesn't decode back to '-';
    // the `$` string-variable marker (StringVariableDefinition, &03 - an
    // earlier guess that it would share the VariableDefinition1/2/3
    // family with integer/real, the way `%`/DEFINT-integer do, was
    // wrong); and ON...GOSUB's comma-separated target list, which needs
    // the same LineNumber token as a plain GOTO for every target, not
    // just the last one (a comma is a valid terminator for a bare line
    // number too, missed on the first pass).
    let source = "10 mode 2:defint a-z:randomize time\n\
                  20 input \"Your question (hit Return to quit)\";i$\n\
                  30 if i$=\"\" then print \"Goodbye!\":end\n\
                  40 q=1+19*rnd\n\
                  50 on q gosub 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250, 260, 270, 280, 290\n\
                  60 goto 20\n\
                  100 print \"It is certain\":return\n\
                  110 print \"It is decidedly so\":return\n\
                  120 print \"Without a doubt\":return\n\
                  130 print \"Yes, definitely\":return\n\
                  140 print \"You may rely on it\":return\n\
                  150 print \"As I see it, yes\":return\n\
                  160 print \"Most likely\":return\n\
                  170 print \"Outlook good\":return\n\
                  180 print \"Signs point to yes\":return\n\
                  190 print \"Yes\":return\n\
                  200 print \"Reply hazy, try again\":return\n\
                  210 print \"Ask again later\":return\n\
                  220 print \"Better not tell you now\":return\n\
                  230 print \"Cannot predict now\":return\n\
                  240 print \"Concentrate and ask again\":return\n\
                  250 print \"Don't bet on it\":return\n\
                  260 print \"My reply is no\":return\n\
                  270 print \"My sources say no\":return\n\
                  280 print \"Outlook not so good\":return\n\
                  290 print \"Very doubtful\":return\n";

    assert_matches_real_cpc(
        source,
        "tests/fixtures/magic8b_real_cpc.dsk",
        "MAGIC8B2.BAS",
        &[]
    );
}

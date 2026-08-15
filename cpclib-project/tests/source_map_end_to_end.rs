//! The source map, against a real assemble.
//!
//! The unit tests in `srcmap` build rows by hand, which proves the queries but
//! not that anything ever fills them in. This assembles actual Z80 and checks
//! the addresses against what the instructions must occupy.

use std::path::Path;

use cpclib_project::srcmap::SourceMap;

/// Assemble `source` (written to a real file, since the map is keyed by file)
/// and build the map from it.
fn map_of(source: &str) -> (SourceMap, camino_tempfile::Utf8TempDir) {
    let tmp = camino_tempfile::tempdir().unwrap();
    let path = tmp.path().join("main.asm");
    std::fs::write(&path, source).unwrap();

    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let builder = parse
        .clone()
        .context_builder()
        .set_current_filename(path.as_str());
    let listing = cpclib_asm::parser::parse_z80_with_context_builder(source, builder)
        .expect("the fixture must parse");

    let mut assemble = cpclib_asm::AssemblingOptions::default();
    assemble.record_source_map();

    // A real build is two steps: the passes, then the post actions - and it
    // is the post actions that add the listing pass, which is where the source
    // map is collected. Assembling without them produces bytes but no map.
    let (_processed, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(
            parse,
            assemble,
            std::sync::Arc::new(cpclib_common::event::DiscardObserver)
        )
    )
    .expect("the fixture must assemble");
    env.handle_post_actions(&listing)
        .expect("post actions must succeed");

    let raw = env.source_map().expect("a source map was requested");
    (SourceMap::from_raw(&raw), tmp)
}

/// Three instructions of known size, laid out from `org`.
#[test]
fn addresses_follow_the_instructions() {
    let (map, tmp) = map_of("\torg 0x4000\n\tld a, 0\n\tld hl, 0x1234\n\tnop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    // line 2 `ld a,0` is 2 bytes at 0x4000, line 3 `ld hl,nn` 3 bytes, line 4
    // `nop` 1 byte.
    assert_eq!(map.addresses_at(file, 2), &[0x4000], "ld a, 0");
    assert_eq!(map.addresses_at(file, 3), &[0x4002], "ld hl, 0x1234");
    assert_eq!(map.addresses_at(file, 4), &[0x4005], "nop");

    // ...and every byte in between resolves back to the right line.
    assert_eq!(map.location_at(0x4001).unwrap().line, 2, "second byte of ld a,0");
    assert_eq!(map.location_at(0x4004).unwrap().line, 3, "third byte of ld hl");
    assert_eq!(map.location_at(0x4005).unwrap().line, 4);
    assert!(map.location_at(0x4006).is_none(), "past the end of the program");
}

/// A line that emits nothing holds no address, and a breakpoint on it slides.
#[test]
fn a_comment_line_slides_to_the_next_instruction() {
    let (map, tmp) = map_of("\torg 0x4000\n\t; nothing here\n\n\tnop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    assert!(map.addresses_at(file, 2).is_empty(), "a comment emits nothing");
    let placed = map.breakpoint_at(file, 2).expect("must slide to the nop");
    assert_eq!(placed.line, 4);
    assert_eq!(placed.address, 0x4000);
}

/// A `repeat` emits its body several times; each iteration is a real address
/// and all of them map back to the one source line.
///
/// This used to record two rows for three iterations: the listing grouped
/// tokens by source line, and a re-executed body arrives on the same line as
/// the previous iteration, so iterations two and three were glued into one row.
/// Fixed in `ListingOutput::token_is_on_same_line` by treating a token that
/// does not advance through the source as a re-execution rather than a
/// continuation.
#[test]
fn a_repeat_body_yields_one_address_per_iteration() {
    let (map, tmp) = map_of("\torg 0x4000\n\trepeat 3\n\tnop\n\trend\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    // Assert against what the assembler actually emitted rather than against
    // an assumption about `repeat`'s counting: the map must account for every
    // byte of the program, whatever that number turns out to be.
    let emitted = cpclib_asm::assemble("\torg 0x4000\n\trepeat 3\n\tnop\n\trend\n")
        .expect("assembles")
        .len();
    let addresses = map.addresses_at(file, 3);
    assert_eq!(
        addresses.len(),
        emitted,
        "one address per emitted nop: {addresses:?} vs {emitted} bytes"
    );
    for address in addresses {
        assert_eq!(map.location_at(*address).unwrap().line, 3);
    }
}

/// A multi-statement line is one line, and stays one row - the statements do
/// advance through the source, so they are a continuation, not a re-execution.
/// This is the case the `repeat` fix must not break.
#[test]
fn a_multi_statement_line_stays_one_row() {
    let (map, tmp) = map_of("\torg 0x4000\n\tnop : nop : nop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    assert_eq!(map.addresses_at(file, 2), &[0x4000], "one row for the line");
    // ...and every byte of it still resolves back to that line.
    for address in 0x4000..0x4003 {
        assert_eq!(map.location_at(address).unwrap().line, 2);
    }
}

/// `org` moving the assembly point is followed, not assumed monotonic.
#[test]
fn a_second_org_is_followed() {
    let (map, tmp) = map_of("\torg 0x8000\n\tnop\n\torg 0x4000\n\tnop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    assert_eq!(map.addresses_at(file, 2), &[0x8000]);
    assert_eq!(map.addresses_at(file, 4), &[0x4000]);
    assert_eq!(map.location_at(0x8000).unwrap().line, 2);
    assert_eq!(map.location_at(0x4000).unwrap().line, 4);
}

/// Asking for no source map must leave the assembler exactly as it was.
#[test]
fn a_program_assembled_without_asking_has_no_map() {
    let source = "\torg 0x4000\n\tnop\n";
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let listing = cpclib_asm::parser::parse_z80_str(source).expect("parses");
    let (_processed, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(
            parse,
            cpclib_asm::AssemblingOptions::default(),
            std::sync::Arc::new(cpclib_common::event::DiscardObserver)
        )
    )
    .expect("assembles");
    env.handle_post_actions(&listing).expect("post actions");
    assert!(env.source_map().is_none());
}

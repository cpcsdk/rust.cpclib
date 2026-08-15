//! The source map must account for every byte the program emits.
//!
//! This is the invariant worth pinning: whatever the listing does with
//! grouping, splitting or expansion contexts, the lengths of the rows it
//! records have to add up to the size of the assembled output. A row lost to a
//! grouping quirk shows up here as a shortfall, whatever caused it - which is
//! how the `REPEAT` merge was found.

use std::sync::Arc;

use cpclib_common::event::DiscardObserver;

/// `(bytes the program emits, rows the source map recorded)`.
fn rows(source: &str) -> (usize, Vec<(u16, u32, u32, u16)>) {
    let listing = cpclib_asm::parser::parse_z80_str(source).expect("parses");
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let mut assemble = cpclib_asm::AssemblingOptions::default();
    assemble.record_source_map();
    // A real build is the passes *then* the post actions; the latter is what
    // adds the listing pass, which is where the map is collected.
    let (_p, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(parse, assemble, Arc::new(DiscardObserver))
    )
    .expect("assembles");
    env.handle_post_actions(&listing).expect("post actions");
    let raw = env.source_map().expect("a source map was requested");
    let emitted = cpclib_asm::assemble(source).expect("assembles").len();
    (
        emitted,
        raw.rows
            .iter()
            .map(|r| (r.file, r.line, r.logical, r.len))
            .collect()
    )
}

#[test]
fn every_emitted_byte_is_covered_by_exactly_one_row() {
    for (name, src) in [
        ("straight", "\torg 0x4000\n\tld a,0\n\tld hl,0x1234\n\tnop\n"),
        ("repeat multiline", "\torg 0x4000\n\trepeat 3\n\tnop\n\trend\n"),
        ("repeat on one line", "\torg 0x4000\n\trepeat 3 : nop : rend\n"),
        (
            "repeat with a two-line body",
            "\torg 0x4000\n\trepeat 3\n\tnop\n\tnop\n\trend\n"
        ),
        ("multi-statement line", "\torg 0x4000\n\tnop : nop : nop\n"),
        (
            "macro called twice",
            "\torg 0x4000\n\tmacro M\n\tnop\n\tendm\n\tM (void)\n\tM (void)\n"
        ),
        ("defs", "\torg 0x4000\n\tdefs 16, 0xff\n\tnop\n")
        // Deliberately not "two orgs with a gap between them": `assemble`
        // returns one contiguous image spanning lowest to highest address, so
        // its length counts the gap, which no row emitted. The rows are still
        // right there - `a_second_org_is_followed` in cpclib-project checks
        // them by address instead.
    ] {
        let (emitted, rows) = rows(src);
        let covered: usize = rows.iter().map(|r| r.3 as usize).sum();
        assert_eq!(
            covered, emitted,
            "{name}: rows cover {covered} bytes but the program emits {emitted}\n  {rows:?}"
        );

        // ...and no two rows claim the same byte.
        let mut spans: Vec<(u32, u32)> = rows
            .iter()
            .filter(|r| r.3 > 0)
            .map(|r| (r.2, r.2 + r.3 as u32))
            .collect();
        spans.sort_unstable();
        for pair in spans.windows(2) {
            assert!(
                pair[0].1 <= pair[1].0,
                "{name}: rows overlap: {:?} and {:?}\n  {rows:?}",
                pair[0],
                pair[1]
            );
        }
    }
}

/// Each iteration of a `REPEAT` is its own row, not one row with the bytes of
/// all of them glued together.
#[test]
fn a_repeat_records_one_row_per_iteration() {
    let (_emitted, rows) = rows("\torg 0x4000\n\trepeat 3\n\tnop\n\trend\n");
    let body: Vec<_> = rows.iter().filter(|r| r.1 == 3).collect();
    assert_eq!(body.len(), 3, "three iterations of line 3: {rows:?}");
    assert_eq!(body.iter().map(|r| r.2).collect::<Vec<_>>(), vec![
        0x4000, 0x4001, 0x4002
    ]);
}

/// The statements of a multi-statement line do advance through the source, so
/// they stay one row - the distinction the `REPEAT` fix rests on.
#[test]
fn a_multi_statement_line_is_a_single_row() {
    let (_emitted, rows) = rows("\torg 0x4000\n\tnop : nop : nop\n");
    let line = rows.iter().filter(|r| r.1 == 2 && r.3 > 0).collect::<Vec<_>>();
    assert_eq!(line.len(), 1, "{rows:?}");
    assert_eq!(line[0].3, 3, "all three bytes on the one row");
}

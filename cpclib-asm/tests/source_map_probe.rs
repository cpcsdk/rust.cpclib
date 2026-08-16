//! The source map must account for every byte the program emits.
//!
//! This is the invariant worth pinning: whatever the listing does with
//! grouping, splitting or expansion contexts, the lengths of the rows it
//! records have to add up to the size of the assembled output. A row lost to a
//! grouping quirk shows up here as a shortfall, whatever caused it - which is
//! how the `REPEAT` merge was found.

use std::sync::Arc;

use cpclib_common::event::DiscardObserver;

/// The map exactly as the assembler recorded it, file names included.
fn raw_map(source: &str) -> cpclib_asm::assembler::listing_output::RawSourceMap {
    let listing = cpclib_asm::parser::parse_z80_str(source).expect("parses");
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let mut assemble = cpclib_asm::AssemblingOptions::default();
    assemble.record_source_map();
    let (_p, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(parse, assemble, Arc::new(DiscardObserver))
    )
    .expect("assembles");
    env.handle_post_actions(&listing).expect("post actions");
    env.source_map().expect("a source map was requested")
}

/// `(bytes the program emits, rows the source map recorded)`.
fn rows(
    source: &str
) -> (
    usize,
    Vec<cpclib_asm::assembler::listing_output::SourceMapRow>
) {
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
    (emitted, raw.rows.clone())
}

#[test]
fn every_emitted_byte_is_covered_by_exactly_one_row() {
    for (name, src) in [
        (
            "straight",
            "\torg 0x4000\n\tld a,0\n\tld hl,0x1234\n\tnop\n"
        ),
        (
            "repeat multiline",
            "\torg 0x4000\n\trepeat 3\n\tnop\n\trend\n"
        ),
        (
            "repeat on one line",
            "\torg 0x4000\n\trepeat 3 : nop : rend\n"
        ),
        (
            "repeat with a two-line body",
            "\torg 0x4000\n\trepeat 3\n\tnop\n\tnop\n\trend\n"
        ),
        ("multi-statement line", "\torg 0x4000\n\tnop : nop : nop\n"),
        (
            "macro called twice",
            "\torg 0x4000\n\tmacro M\n\tnop\n\tendm\n\tM (void)\n\tM (void)\n"
        ),
        ("defs", "\torg 0x4000\n\tdefs 16, 0xff\n\tnop\n") /* Deliberately not "two orgs with a gap between them": `assemble`
                                                            * returns one contiguous image spanning lowest to highest address, so
                                                            * its length counts the gap, which no row emitted. The rows are still
                                                            * right there - `a_second_org_is_followed` in cpclib-project checks
                                                            * them by address instead. */
    ] {
        let (emitted, rows) = rows(src);
        let covered: usize = rows.iter().map(|r| r.len as usize).sum();
        assert_eq!(
            covered, emitted,
            "{name}: rows cover {covered} bytes but the program emits {emitted}\n  {rows:?}"
        );

        // ...and no two rows claim the same byte.
        let mut spans: Vec<(u32, u32)> = rows
            .iter()
            .filter(|r| r.len > 0)
            .map(|r| (r.logical, r.logical + r.len as u32))
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
    let body: Vec<_> = rows.iter().filter(|r| r.line == 3).collect();
    assert_eq!(body.len(), 3, "three iterations of line 3: {rows:?}");
    assert_eq!(
        body.iter().map(|r| r.logical).collect::<Vec<_>>(),
        vec![0x4000, 0x4001, 0x4002]
    );
}

/// A multi-statement line is one row *per instruction*, each with the columns
/// it occupies.
///
/// `ld a,l : inc a : ld (.p),a` is three instructions at three addresses, and a
/// debugger stopped at the second should point at the second - not at the start
/// of the line, which is what a per-line row can only ever say.
#[test]
fn a_multi_statement_line_is_one_row_per_instruction() {
    let (_emitted, rows) = rows("\torg 0x4000\n\tnop : nop : nop\n");
    let line = rows
        .iter()
        .filter(|r| r.line == 2 && r.len > 0)
        .collect::<Vec<_>>();

    assert_eq!(line.len(), 3, "one per instruction: {rows:?}");
    for (index, row) in line.iter().enumerate() {
        assert_eq!(row.len, 1, "{rows:?}");
        assert_eq!(row.logical, 0x4000 + index as u32, "{rows:?}");
        assert!(row.column_end > row.column, "{row:?}");
    }

    // The columns are distinct and in order, and none overlaps its neighbour -
    // which is what lets an address pick exactly one of them.
    for pair in line.windows(2) {
        assert!(
            pair[0].column_end <= pair[1].column,
            "columns overlap: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }

    // ...and they point at the real text: "\tnop : nop : nop" has `nop` at
    // columns 2, 8 and 14.
    assert_eq!(
        line.iter().map(|r| r.column).collect::<Vec<_>>(),
        vec![2, 8, 14],
        "{rows:?}"
    );
}

/// The same logical address in two pages must be recorded as two rows that can
/// be told apart.
///
/// Without the page, `&4000` in the base 64K and `&4000` in page 5 are the same
/// key, and a debugger stopping there has no way to know which line it is on.
#[test]
fn code_in_two_banks_records_two_distinct_pages() {
    let (_, rows) = rows(
        "\tbuildsna\n\tbankset 0\n\torg 0x4000\n\tnop\n\tbankset 1\n\torg \
         0x4000\n\tnop\n\tnop\n"
    );

    let at_4000: Vec<_> = rows.iter().filter(|r| r.logical == 0x4000).collect();
    assert_eq!(at_4000.len(), 2, "one row per bank: {rows:?}");

    let pages: Vec<u8> = at_4000.iter().map(|r| r.page).collect();
    assert_ne!(
        pages[0], pages[1],
        "the two rows are distinguishable: {rows:?}"
    );

    // ...and the physical positions differ even though the logical ones do not,
    // which is what makes the whole program addressable as one image.
    assert_ne!(at_4000[0].physical, at_4000[1].physical, "{rows:?}");
}

/// An instruction's location is the *instruction's*, never its operands'.
///
/// A `function` runs at assembly time: it computes a number and emits nothing.
/// But evaluating one walks its body, and every token walked announces itself
/// to the listing - so without a guard the bytes of
/// `ld a, START + integral(...)` are recorded against the `return` inside
/// `integral`, and a debugger stopping there jumps to a line the program never
/// executes. Found by stepping through a real demo.
#[test]
fn a_function_called_from_an_expression_steals_no_line() {
    let (_, rows) = rows(
        "\tfunction integral, n\n\
         \t\tacc = 0\n\
         \t\trepeat {n}, i\n\
         \t\t\tacc = acc + 1\n\
         \t\tendrepeat\n\
         \t\treturn acc\n\
         \tendfunction\n\
         \torg 0x4000\n\
         \tld a, 3 + integral(4)\n\
         \tnop\n"
    );

    // Lines 1..7 are the function. Nothing may be attributed to them - they
    // emit no bytes, and a row there is a line the debugger would jump to.
    let inside: Vec<_> = rows.iter().filter(|r| (1..=7).contains(&r.line)).collect();
    assert!(
        inside.is_empty(),
        "the function body claimed rows: {inside:?}"
    );

    // The `ld a,...` is on line 9 and is where its two bytes belong - at the
    // address the code is at, not at the value the function returned. The
    // second half of this caught a separate leak: assigning a symbol overrides
    // the listing's address column, so `acc = 0` inside the function body was
    // relabelling the instruction that followed it.
    let ld = rows
        .iter()
        .find(|r| r.line == 9)
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(
        ld.logical, 0x4000,
        "not integral()'s return value: {rows:?}"
    );
    assert_eq!(ld.len, 2);
}

/// The same rule, without a function in sight.
///
/// `assert` and `print` take expressions too, and reach the listing by the same
/// path. Guarding the choke point rather than the function is what makes these
/// work without being named.
#[test]
fn directives_between_instructions_steal_no_line() {
    let (_, rows) = rows(
        "\torg 0x4000\n\
         \tnop\n\
         \tassert 1 == 1\n\
         \tprint \"building\"\n\
         \tld a, 1\n"
    );

    let first = rows.iter().find(|r| r.logical == 0x4000).expect("{rows:?}");
    assert_eq!(first.line, 2, "{rows:?}");
    let second = rows.iter().find(|r| r.logical == 0x4001).expect("{rows:?}");
    assert_eq!(second.line, 5, "the ld, not the print above it: {rows:?}");
}

/// Code assembled inside a macro body belongs to the file the macro is
/// written in.
///
/// The parser records it against a *context* - `demo.asm:12:3 > MACRO DRAW:` -
/// which a listing prints happily and a debugger cannot open: "could not load
/// source". The lines in those rows are already the file's own, so only the
/// name needed recovering.
#[test]
fn a_macro_body_is_recorded_against_a_real_file() {
    let (_, rows) = rows(
        "\tmacro DRAW, n\n\t\tld a, {n}\n\t\tnop\n\tendm\n\torg 0x4000\n\tnop\n\tDRAW 3\n\tnop\n"
    );

    // No parser context reaches the map: every recorded name is something a
    // debugger could open, never `demo.asm:12:3 > MACRO DRAW:`.
    let files = raw_map(
        "\tmacro DRAW, n\n\t\tld a, {n}\n\t\tnop\n\tendm\n\torg 0x4000\n\tnop\n\tDRAW 3\n\tnop\n"
    )
    .files;
    assert!(
        files.iter().all(|name| !name.contains(" > MACRO ")),
        "a parser context reached the map: {files:?}"
    );

    // The macro's two instructions are there, on the lines they are written on.
    assert!(
        rows.iter().any(|r| r.line == 2 && r.len == 2),
        "ld a,{{n}}: {rows:?}"
    );
    assert!(
        rows.iter().any(|r| r.line == 3 && r.len == 1),
        "nop: {rows:?}"
    );
}

/// A local label written before the first global one.
///
/// Local labels are stored as `<current global>.<local>`, and the current
/// global used to survive from one pass into the next - so `.loop` was `.loop`
/// in pass 1 and `message.loop` in pass 2, and the assembler blamed
/// "conditional code" in a program with no condition in it.
#[test]
fn a_local_label_before_any_global_one_assembles() {
    let source = "\torg 0x8000\n\
                  \tld hl, message\n\
                  .loop\n\
                  \tld a, (hl) : inc hl\n\
                  \tor a : jr z, .done\n\
                  \tjr .loop\n\
                  .done\n\
                  \tjp $\n\
                  message\n\
                  \tdb \"HELLO\", 0\n";

    let bytes = cpclib_asm::assemble(source).expect("assembles in every pass");
    assert!(!bytes.is_empty());
}

/// ...and it still works when a global label *does* come first, which is the
/// case that always worked and must keep working.
#[test]
fn a_local_label_after_a_global_one_still_belongs_to_it() {
    let source = "\torg 0x8000\n\
                  routine\n\
                  .loop\n\
                  \tjr .loop\n\
                  other\n\
                  .loop\n\
                  \tjr .loop\n";

    // Two `.loop`s under different globals are two labels, not a redefinition.
    let bytes = cpclib_asm::assemble(source).expect("assembles");
    assert_eq!(bytes.len(), 4, "two 2-byte jr");
}

/// `snainit "inner://cpc6128.sna"` starts from a booted machine without the
/// project having to carry a copy of one.
///
/// A program that calls the firmware needs the firmware set up, and requiring a
/// binary blob beside the source made every such build depend on a file nothing
/// in the repository produced.
#[test]
fn snainit_can_start_from_an_embedded_snapshot() {
    let source = "\tsnainit \"inner://cpc6128.sna\"\n\
                  \torg 0x8000\n\
                  \trun $\n\
                  \tcall 0xBB5A\n\
                  \tjp $\n";

    let listing = cpclib_asm::parser::parse_z80_str(source).expect("parses");
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let (_, env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(
            parse,
            cpclib_asm::AssemblingOptions::default(),
            Arc::new(DiscardObserver)
        )
    )
    .expect("assembles from the embedded snapshot");

    // The machine it started from is a real one: the firmware ROM is paged in
    // and its memory is the size the header claims.
    let sna = env.sna();
    assert!(sna.memory_dump().len() >= 0x1_0000, "a whole machine");
}

/// A name that is not embedded says which ones are, rather than reporting a
/// missing file that was never on disc.
#[test]
fn an_unknown_embedded_snapshot_lists_the_real_ones() {
    let source = "\tsnainit \"inner://nope.sna\"\n\torg 0x8000\n\tnop\n";
    let listing = cpclib_asm::parser::parse_z80_str(source).expect("parses");
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let outcome = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(
            parse,
            cpclib_asm::AssemblingOptions::default(),
            Arc::new(DiscardObserver)
        )
    );

    let (_, _, problem) = outcome.err().expect("refused");
    let problem = problem.to_string();
    assert!(problem.contains("inner://cpc6128.sna"), "{problem}");
    assert!(problem.contains("inner://cpc6128_v2.sna"), "{problem}");
}

//! The source map, against a real assemble.
//!
//! The unit tests in `srcmap` build rows by hand, which proves the queries but
//! not that anything ever fills them in. This assembles actual Z80 and checks
//! the addresses against what the instructions must occupy.

use std::path::Path;

use cpclib_asm::assembler::listing_output::RawSourceMap;
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
    assert_eq!(
        map.location_at(0x4001).unwrap().line,
        2,
        "second byte of ld a,0"
    );
    assert_eq!(
        map.location_at(0x4004).unwrap().line,
        3,
        "third byte of ld hl"
    );
    assert_eq!(map.location_at(0x4005).unwrap().line, 4);
    assert!(
        map.location_at(0x4006).is_none(),
        "past the end of the program"
    );
}

/// A line that emits nothing holds no address, and a breakpoint on it slides.
#[test]
fn a_comment_line_slides_to_the_next_instruction() {
    let (map, tmp) = map_of("\torg 0x4000\n\t; nothing here\n\n\tnop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    assert!(
        map.addresses_at(file, 2).is_empty(),
        "a comment emits nothing"
    );
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

/// A multi-statement line is one line but several instructions, and each gets
/// its own row - with the columns that let a debugger point at the one being
/// executed rather than at the start of all three.
#[test]
fn a_multi_statement_line_yields_one_row_per_instruction() {
    let (map, tmp) = map_of("\torg 0x4000\n\tnop : nop : nop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    assert_eq!(
        map.addresses_at(file, 2),
        &[0x4000, 0x4001, 0x4002],
        "one address per instruction on the line"
    );

    // A breakpoint on the line still lands on its *first* instruction, which is
    // where a line is entered.
    let placement = map.breakpoint_at(file, 2).expect("placed");
    assert_eq!(placement.address, 0x4000);
    assert_eq!(placement.line, 2);

    // ...and each address resolves to its own instruction, not to the line.
    let columns: Vec<(u32, u32)> = (0x4000..0x4003)
        .map(|address| {
            let location = map.location_at(address).unwrap();
            assert_eq!(location.line, 2);
            (location.column, location.column_end)
        })
        .collect();
    assert_eq!(
        columns,
        vec![(2, 5), (8, 11), (14, 17)],
        "\tnop : nop : nop"
    );
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

/// The editor needs a path it can open.
///
/// The assembler records a file by the name it was reached by, which for an
/// `include` is relative. A relative path is not openable, so the editor asks
/// the adapter for the contents - and, failing that, shows disassembly instead
/// of the code. Resolving them once, at build time, is what makes a stop land
/// on the source line.
#[test]
fn included_files_are_recorded_with_a_path_the_editor_can_open() {
    let tmp = camino_tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("lib.asm"), "\tnop\n").unwrap();
    let entry = tmp.path().join("main.asm");
    std::fs::write(&entry, "\torg 0x4000\n\tinclude \"lib.asm\"\n\tnop\n").unwrap();

    let source = std::fs::read_to_string(&entry).unwrap();
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let _ = parse.add_search_path(tmp.path().to_string());
    let builder = parse
        .clone()
        .context_builder()
        .set_current_filename(entry.as_str());
    let listing =
        cpclib_asm::parser::parse_z80_with_context_builder(&source, builder).expect("parses");

    let mut assemble = cpclib_asm::AssemblingOptions::default();
    assemble.record_source_map();
    let (_p, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(
            parse,
            assemble,
            std::sync::Arc::new(cpclib_common::event::DiscardObserver)
        )
    )
    .expect("assembles");
    env.handle_post_actions(&listing).expect("post actions");

    let map = SourceMap::from_raw(&env.source_map().expect("a map"))
        .resolved_against(Path::new(entry.as_str()));

    for file in map.files() {
        assert!(
            file.is_absolute(),
            "{} is not openable by an editor",
            file.display()
        );
        assert!(file.exists(), "{} does not exist", file.display());
    }
}

/// Each address on a multi-instruction line resolves to *its own* instruction.
///
/// Reported from real use: stopping on the second instruction of
/// `ld e,(hl) : inc hl : ld d,(hl)` highlighted the third, and the next line's
/// instruction was not highlighted at all.
#[test]
fn every_instruction_of_a_shared_line_resolves_to_itself() {
    let (map, tmp) = map_of("\torg 0x4000\n\tld e, (hl) : inc hl : ld d, (hl)\n\tnop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    let at = |address: u32| {
        let location = map
            .location_at(address)
            .unwrap_or_else(|| panic!("nothing at 0x{address:04X}"));
        (location.line, location.column, location.column_end)
    };

    // `\tld e, (hl) : inc hl : ld d, (hl)` - the tab is column 1.
    assert_eq!(at(0x4000), (2, 2, 12), "ld e, (hl)");
    assert_eq!(at(0x4001), (2, 15, 21), "inc hl");
    assert_eq!(at(0x4002), (2, 24, 34), "ld d, (hl)");
    // ...and the line after it is reached too.
    assert_eq!(at(0x4003).0, 3, "the next line");

    // Nothing claims a byte it does not own.
    assert_eq!(map.addresses_at(file, 2), &[0x4000, 0x4001, 0x4002]);
    assert_eq!(map.addresses_at(file, 3), &[0x4003]);
}

/// The same, with a label in front - the shape real code actually has.
#[test]
fn a_label_before_a_shared_line_does_not_shift_the_instructions() {
    let (map, _tmp) = map_of("\torg 0x4000\n.loop\tld e, (hl) : inc hl : ld d, (hl)\n\tnop\n");

    let at = |address: u32| {
        let l = map
            .location_at(address)
            .unwrap_or_else(|| panic!("nothing at 0x{address:04X}"));
        (l.line, l.column)
    };
    assert_eq!(at(0x4000).0, 2);
    assert_eq!(at(0x4001).0, 2);
    assert_eq!(at(0x4002).0, 2);
    // Three distinct columns, in order.
    let columns = [at(0x4000).1, at(0x4001).1, at(0x4002).1];
    assert!(
        columns[0] < columns[1] && columns[1] < columns[2],
        "{columns:?}"
    );
    assert_eq!(at(0x4003).0, 3, "and the next line is still reachable");
}

/// Instructions of differing length on one line: the offsets have to follow the
/// bytes, not the count.
#[test]
fn instructions_of_different_lengths_share_a_line_correctly() {
    let (map, _tmp) = map_of("\torg 0x4000\n\tld hl, 0x1234 : nop : ld a, 5\n\tnop\n");

    // 3 bytes, then 1, then 2.
    assert_eq!(map.location_at(0x4000).unwrap().line, 2);
    assert_eq!(
        map.location_at(0x4002).unwrap().column,
        map.location_at(0x4000).unwrap().column
    );
    let nop = map.location_at(0x4003).unwrap();
    let lda = map.location_at(0x4004).unwrap();
    assert!(nop.column < lda.column, "{nop:?} then {lda:?}");
    assert_eq!(map.location_at(0x4006).unwrap().line, 3, "the next line");
}

/// `is_data` and `len` survive `from_raw`, `location_at` and
/// `location_at_long` unchanged - the plumbing `-dv` relies on to tell a `db`
/// row from a real instruction.
#[test]
fn is_data_and_len_are_pinned_through_location_at() {
    let (map, tmp) =
        map_of("\torg 0x4000\n\tdb \"HELLO, WORLD!\", 10, 13, 0\n\tnop\n\tnop\n\tnop\n");
    let file = tmp.path().join("main.asm");
    let file = Path::new(file.as_str());

    // The listing's own `bytes_per_line` chunking can split one `db` token
    // into several rows - existing, unrelated behaviour this test must not
    // assume away. What matters here is that every row the db line produced
    // is marked data, and the last one's span reaches the line's last byte.
    let db_addresses = map.addresses_at(file, 2);
    assert!(!db_addresses.is_empty(), "the db line occupies addresses");
    let first_address = db_addresses[0];
    let mut last_end = first_address;
    for address in db_addresses {
        let location = map.location_at(*address).expect("the db row resolves");
        assert!(location.is_data, "{location:?}");
        last_end = last_end.max(*address + location.len);
    }
    let bytes_emitted = "HELLO, WORLD!".len() as u32 + 3; // the string, then 10, 13, 0
    assert_eq!(
        last_end,
        first_address + bytes_emitted,
        "the rows together cover the whole directive"
    );

    let nop_address = map.addresses_at(file, 3)[0];
    let nop_location = map.location_at(nop_address).expect("the nop row resolves");
    assert!(!nop_location.is_data, "{nop_location:?}");
    assert_eq!(nop_location.len, 1);

    // The paged lookup answers the same way.
    let long_location = map
        .location_at_long(0, nop_address as u16)
        .expect("resolves by page too");
    assert!(!long_location.is_data);
}

/// A source map from before `is_data` existed still loads, and its rows read
/// back as "not marked" - `is_data: false`, today's existing behaviour -
/// rather than being rejected outright.
///
/// This is the concrete proof of the `#[serde(default)]` compatibility
/// decision behind `SourceMapRow::is_data`: bumping `SourceMapFile::VERSION`
/// for a purely additive field would force every cached `--sourcemap` file a
/// user already has to be thrown away and re-assembled, defeating the point
/// of that cache. `#[serde(default)]` has direct precedent in this exact
/// struct family already (`SourceMapFile::address_symbols`).
#[test]
fn a_v1_row_missing_is_data_still_loads_and_reads_as_not_data() {
    let v1_json = r#"{
        "files": ["main.asm"],
        "rows": [
            {
                "file": 0,
                "line": 2,
                "logical": 16384,
                "physical": 16384,
                "page": 0,
                "column": 1,
                "column_end": 1,
                "len": 3
            }
        ]
    }"#;
    let raw: RawSourceMap =
        serde_json::from_str(v1_json).expect("a v1 map without is_data still parses");
    assert!(!raw.rows[0].is_data, "a missing field defaults to false");

    let map = SourceMap::from_raw(&raw);
    let location = map.location_at(0x4000).expect("the row still resolves");
    assert!(!location.is_data, "and so does the location built from it");
    assert_eq!(location.len, 3);
}

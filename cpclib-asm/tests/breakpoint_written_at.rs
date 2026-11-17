//! A `BREAKPOINT` directive must say where it is written.
//!
//! The address it arms is the *next* instruction's, so a directive inside a
//! macro body stops the program in whatever file used the macro - a line with
//! nothing on it to explain the stop, in a file the user never marked. Without
//! the directive's own location a debugger has nothing to point them at.

use std::sync::Arc;

use cpclib_common::event::DiscardObserver;

/// Assemble `entry` for real and hand back the breakpoints it asked for.
fn breakpoints(
    directory: &std::path::Path,
    entry: &std::path::Path
) -> Vec<cpclib_asm::assembler::delayed_command::AssembledBreakpoint> {
    let text = fs_err::read_to_string(entry).unwrap();
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    let _ = parse.add_search_path(directory);
    let builder = parse
        .clone()
        .context_builder()
        .set_current_filename(entry.to_str().unwrap());
    let listing =
        cpclib_asm::parser::parse_z80_with_context_builder(&text, builder).expect("parses");

    let assemble = cpclib_asm::AssemblingOptions::default();
    let (_, env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(parse, assemble, Arc::new(DiscardObserver))
    )
    .expect("assembles");
    env.assembled_breakpoints()
}

/// The exact shape the report came from: `BREAKPOINT` in a macro body, the
/// macro used from another file.
fn a_macro_and_its_caller(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!("cpclib-brk-{name}-{}", std::process::id()));
    let _ = fs_err::create_dir_all(&directory);
    fs_err::write(
        directory.join("macros.asm"),
        "\tmacro DEBUG col\n\tbreakpoint\n\tld bc,0x7f00 + {col}\n\tendm\n"
    )
    .unwrap();
    let main = directory.join("main.asm");
    fs_err::write(
        &main,
        "\tinclude \"macros.asm\"\n\torg 0x4000\n\tnop\n\tDEBUG(4)\n\tnop\n\tbreakpoint\n\tnop\n"
    )
    .unwrap();
    (directory, main)
}

#[test]
fn a_breakpoint_in_a_macro_body_points_at_the_macro() {
    let (directory, main) = a_macro_and_its_caller("macro");
    let found = breakpoints(&directory, &main);

    // The macro is used once, so the first directive is its own; it arms the
    // instruction after it, which is inside the expansion at `main.asm:4`.
    let written = found[0]
        .written_at
        .as_ref()
        .unwrap_or_else(|| panic!("{found:#?}"));
    assert!(
        written.file.ends_with("macros.asm"),
        "the file it is written in, not the one it stops in: {written:?}"
    );
    // Line 1 is `macro DEBUG col`; the directive is on line 2. A macro body is
    // re-parsed as a source of its own, so this is the shift that has to be
    // undone.
    assert_eq!(written.line, 2, "{written:?}");
    assert_eq!(written.column, 2, "after the tab: {written:?}");
}

#[test]
fn a_breakpoint_written_in_the_file_points_at_itself() {
    let (directory, main) = a_macro_and_its_caller("plain");
    let found = breakpoints(&directory, &main);

    let written = found[1]
        .written_at
        .as_ref()
        .unwrap_or_else(|| panic!("{found:#?}"));
    assert!(
        written.file.ends_with("main.asm"),
        "{written:?} in {found:#?}"
    );
    assert_eq!(written.line, 6, "{written:?}");
}

/// A directive may name a label defined further on, as an address or as
/// `ADDR=`: known only in a later pass, it is not an unknown symbol (nor, for
/// the plain form, a crash).
#[test]
fn a_breakpoint_may_name_a_label_defined_further_on() {
    let directory = std::env::temp_dir().join(format!("cpclib-brk-forward-{}", std::process::id()));
    let _ = fs_err::create_dir_all(&directory);
    let main = directory.join("main.asm");
    fs_err::write(
        &main,
        "\torg 0x4000\nstart\n\tbreakpoint later\n\tbreakpoint type=mem, access=write, addr=buffer\n\tld a,1\nlater\n\tret\nbuffer\n\tdefb 0\n"
    )
    .unwrap();
    let found = breakpoints(&directory, &main);
    let addresses: Vec<u16> = found.iter().map(|b| b.address).collect();
    assert_eq!(addresses, vec![0x4002, 0x4003], "{found:?}");
}

/// Every attribute of the advanced form reaches the debugger as written.
#[test]
fn an_advanced_breakpoint_keeps_every_attribute() {
    use cpclib_asm::assembler::delayed_command::AssembledBreakpointKind;
    let directory = std::env::temp_dir().join(format!("cpclib-brk-attributes-{}", std::process::id()));
    let _ = fs_err::create_dir_all(&directory);
    let main = directory.join("main.asm");
    fs_err::write(
        &main,
        "\torg 0x4000\n\tbreakpoint 0x7f00, type=io, access=write, runmode=watch, mask=0xff00, size=2, value=0x40, valmask=0xc0, condition=\"A = &55\", name=\"ink\", step=3\n\tret\n"
    )
    .unwrap();
    let found = breakpoints(&directory, &main);
    assert_eq!(found.len(), 1, "{found:?}");
    let brk = &found[0];
    assert_eq!((brk.address, brk.kind, brk.name.as_deref()), (0x7f00, AssembledBreakpointKind::Io, Some("ink")));
    let details = brk.details.as_ref().expect("the attributes");
    assert_eq!(details.access, "w");
    assert!(details.watch);
    assert_eq!((details.mask, details.size, details.value, details.value_mask), (0xff00, 2, 0x40, 0xc0));
    assert_eq!(details.condition.as_deref(), Some("A = &55"));
    assert_eq!(details.step, Some(3));
}

/// rasm's rule: a label named `BRK...` - or a local one `.BRK...` - is a
/// breakpoint on its address, named after it, located where it is defined; a
/// value (`EQU`) is not, nor a label that merely contains the letters.
#[test]
fn a_brk_label_is_a_breakpoint() {
    use cpclib_asm::assembler::delayed_command::AssembledBreakpointKind;
    let directory = std::env::temp_dir().join(format!("cpclib-brk-labels-{}", std::process::id()));
    let _ = fs_err::create_dir_all(&directory);
    let main = directory.join("main.asm");
    fs_err::write(
        &main,
        "\torg 0x4000\n\tld a,1\nbrk_here\n\tinc a\nmain\n\tnop\n.brkloop\n\tdec a\n.notbrk\n\tnop\nBRKVALUE equ 0x1234\nbreaking\n\tret\n"
    )
    .unwrap();
    let found = breakpoints(&directory, &main);
    let described: Vec<(u16, Option<&str>, u32)> = found
        .iter()
        .map(|b| (b.address, b.name.as_deref(), b.written_at.as_ref().map_or(0, |w| w.line)))
        .collect();
    assert_eq!(described, vec![(0x4002, Some("brk_here"), 3), (0x4004, Some("main.brkloop"), 7)], "{found:?}");
    assert!(found.iter().all(|b| b.kind == AssembledBreakpointKind::Execution && b.details.is_none()));
}

#[test]
fn which_labels_are_breakpoints() {
    use cpclib_asm::assembler::delayed_command::is_brk_label;
    for name in ["BRK", "brk_loop", "Brk1", "@brkmacro", "main.brkloop", "MAIN.BRK"] {
        assert!(is_brk_label(name), "{name}");
    }
    for name in ["breaking", "loop_brk", "brkmain.loop", "obrk"] {
        assert!(!is_brk_label(name), "{name}");
    }
}

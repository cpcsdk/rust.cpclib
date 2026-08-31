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
    let text = std::fs::read_to_string(entry).unwrap();
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
    let _ = std::fs::create_dir_all(&directory);
    std::fs::write(
        directory.join("macros.asm"),
        "\tmacro DEBUG col\n\tbreakpoint\n\tld bc,0x7f00 + {col}\n\tendm\n"
    )
    .unwrap();
    let main = directory.join("main.asm");
    std::fs::write(
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

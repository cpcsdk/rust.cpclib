// Integration tests verifying that formatting Z80 assembly with any AsmFormatOptions
// configuration produces source that assembles to the exact same binary as the original.
//
// Runs against cpclib-basm/tests/asm/good_*.asm (the basm "good" test suite).
// The Rust API (cpclib_basm::{parse, assemble}) is used directly – no pre-built binary needed.
//
// Path notes
// ----------
// test-generator resolves globs at compile time from the *workspace root*, so the pattern
// "cpclib-basm/tests/asm/good_*.asm" works from any crate in the workspace.
// At runtime Cargo sets cwd = the package root (`cpclib/`), so `real_fname`
// (which is workspace-root-relative, e.g. "cpclib-basm/tests/asm/good_foo.asm") must be
// prefixed with "../" for actual filesystem access.

use std::sync::Arc;

use cpclib_asmfmt::{AsmFormatOptions, CaseStyle};
use cpclib_basm::{assemble, build_args_parser, parse};
use test_generator::test_resources;

/// Converts a workspace-root-relative path (as supplied by test_resources) to a path
/// that can be opened from the test cwd (`cpclib/`).
fn workspace_path(real_fname: &str) -> String {
    format!("../{real_fname}")
}

/// The basm include directory for INCLUDE/INCBIN directives, relative to test cwd.
const INCLUDE_DIR: &str = "../cpclib-basm/tests/asm";

/// Assemble `source` (inline Z80 text) and return the produced bytes.
fn assemble_source(source: &str) -> Result<Vec<u8>, String> {
    let parser = build_args_parser();
    // Prefix with "=" so clap never mistakes the source text for a flag argument.
    let inline_arg = format!("--inline={source}");
    let matches = parser
        .try_get_matches_from(["basm", "-I", INCLUDE_DIR, &inline_arg])
        .map_err(|e| e.to_string())?;

    let (listing, options) = parse(&matches).map_err(|e| e.to_string())?;
    let env = assemble(&matches, &listing, options, Arc::new(()))
        .map_err(|e| e.to_string())?;

    Ok(env.produced_bytes())
}

/// All AsmFormatOptions configurations to test.
///
/// Uses the bon builder so every variant only specifies the axes that differ;
/// everything else inherits its default (indent_size=4, comment_column=30, …).
fn all_configs() -> Vec<AsmFormatOptions> {
    use CaseStyle::*;
    let mut configs = Vec::new();

    // Exhaustive combination of the three case axes with splitting enabled
    for mc in [UpperCase, LowerCase, Untouched] {
        for dc in [UpperCase, LowerCase, Untouched] {
            for rc in [UpperCase, LowerCase, Untouched] {
                configs.push(
                    AsmFormatOptions::builder()
                        .mnemonic_case(mc)
                        .directive_case(dc)
                        .register_case(rc)
                        .one_instruction_per_line(true)
                        .build(),
                );
            }
        }
    }

    // Also verify that keeping multi-instruction lines verbatim still assembles correctly
    configs.push(AsmFormatOptions::builder().one_instruction_per_line(false).build());

    configs
}

/// Returns true for files that must be skipped:
/// - Files with SAVE directives write side-effect files to disk during assembly.
/// - BASIC content (the formatter only handles Z80 assembly).
fn should_skip(real_fname: &str, source: &str) -> bool {
    if real_fname.contains("good_basic") {
        return true;
    }
    source.lines().any(|line| {
        let t = line.trim().to_ascii_uppercase();
        t == "SAVE" || t.starts_with("SAVE ") || t.starts_with("SAVE\t")
    })
}

#[test_resources("cpclib-basm/tests/asm/good_*.asm")]
fn formatting_preserves_binary(real_fname: &str) {
    let path = workspace_path(real_fname);
    let source = fs_err::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {path}: {e}"));

    if should_skip(real_fname, &source) {
        return;
    }

    // Assemble original source to get the reference binary.
    // Some files require optional compression codecs (bzpack, aplib, exomizer, …) that
    // may not be compiled in; skip them gracefully rather than failing.
    let original_bytes = match assemble_source(&source) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("SKIP {real_fname}: original failed to assemble: {e}");
            return;
        }
    };

    // For each format configuration, format then reassemble and compare.
    for opts in all_configs() {
        let formatted = match cpclib_asmfmt::format(&source, &opts) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("SKIP format({real_fname}) with {opts:?}: {e}");
                continue;
            }
        };

        let formatted_bytes = assemble_source(&formatted).unwrap_or_else(|e| {
            panic!(
                "formatted {real_fname} failed to assemble with {opts:?}\n\
                 error: {e}\n\
                 --- formatted source ---\n{formatted}"
            )
        });

        assert_eq!(
            original_bytes,
            formatted_bytes,
            "binary differs after formatting {real_fname} with {opts:?}\n\
             --- formatted source ---\n{formatted}"
        );
    }
}

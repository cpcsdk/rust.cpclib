//! Hover preview for macro/struct calls (expanded content) and `FUNCTION`
//! calls (evaluated return value), via the *real* cpclib-asm assembler
//! machinery (`cpclib_asm::assembler::r#macro::{Expandable, MacroWithArgs,
//! StructWithArgs}`, `Expr::resolve`) — not a reimplementation.
//!
//! Safety: hovering must never trigger a real build's side effects
//! (`SAVE`/`BUILDSNA`/`BUILDCPR` writing files, `PAUSE` blocking on stdin,
//! ...). The `Env` used here runs the *real, whole* document (so included
//! files, at any depth, are resolved exactly like a real build) with
//! `AssemblingOptions::dry_run` set — which guarantees none of those
//! effects can happen, regardless of what the document contains — and a
//! `DiscardObserver`, so nothing is ever written to the LSP's real stdout
//! (which carries JSON-RPC protocol traffic, not build output). Expanding a
//! macro/struct call is additionally pure text substitution (it never
//! re-parses or executes the result), so even a macro body that happens to
//! contain something like `SAVE` is inert here: shown as text in the
//! hover, never actually run.

use std::sync::Arc;

use cpclib_asm::AssemblingOptions;
use cpclib_asm::assembler::r#macro::{Expandable, MacroWithArgs, StructWithArgs};
use cpclib_asm::assembler::{Env, EnvOptions};
use cpclib_asm::implementation::expression::ExprEvaluationExt;
use cpclib_asm::parser::obtained::{LocatedListing, MayHaveSpan};
use cpclib_common::event::DiscardObserver;
use cpclib_tokens::symbols::{SymbolsTableTrait, Value};
use cpclib_tokens::{ListingElement, MacroParamElement};
use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

/// Depth cap for iterative macro/struct expansion — mirrors the `MAX_DEPTH`
/// idiom already used for symbol-alias resolution elsewhere in this crate
/// (`basm/color.rs`, `locomotive/color.rs`).
const MAX_EXPANSION_DEPTH: usize = 8;

/// Scan `text` for `{name}` occurrences (curly braces wrapping a bare
/// identifier, no `{eval}` prefix — `{eval}...` is basm's real
/// evaluate-this-expression call-argument syntax, resolved by the assembler
/// on its own) and seed a placeholder value (`0`) for each `name` not
/// already a known symbol in `env`.
///
/// This is how basm code interpolates a `REPEAT` loop's counter into
/// identifiers/arguments (`PLY_AKG_Channel{channelNumber}_PitchTrack`
/// inside `REPEAT 3, channelNumber, 1`) — outside of a real per-iteration
/// assembling pass, `channelNumber` has no value at all, which fails
/// resolution outright with an unknown-symbol error. A placeholder value
/// (standing in for the first iteration) lets expansion/evaluation proceed
/// instead of failing, at the cost of showing iteration-0 values in what's
/// a preview, not a real build.
///
/// Seeded under *two* keys: the bare name (`channelNumber`) and the
/// bracketed form (`{channelNumber}`, braces included). basm's own label
/// resolution (`is_label()` in `cpclib-asm/src/implementation/expression.rs`)
/// looks up a `{...}`-embedding label's *raw, unsubstituted* text first —
/// `{channelNumber}`, literally — and only expands the `{}` segment
/// (substituting the counter's value) to build a *nicer error message* if
/// that first lookup fails; it never retries the lookup with the expanded
/// name. So the bracketed key is the one that actually needs to resolve;
/// the bare name is seeded too since some contexts (e.g. `{eval}name`
/// without its own inner braces) do look it up directly.
fn seed_bracketed_identifier_placeholders(text: &str, env: &mut Env) {
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        if rest[open..].to_uppercase().starts_with("{EVAL}") {
            rest = &rest[open + "{EVAL}".len()..];
            continue;
        }
        let Some(close_rel) = rest[open + 1..].find('}')
        else {
            break;
        };
        let bracketed = &rest[open..open + 1 + close_rel + 1];
        let inner = rest[open + 1..open + 1 + close_rel].trim();
        let is_bare_identifier = inner
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if is_bare_identifier {
            for key in [inner, bracketed] {
                if env.symbols().any_value(key).ok().flatten().is_none() {
                    let _ = env
                        .symbols_mut()
                        .assign_symbol_to_value(key, Value::from(0i32));
                }
            }
        }
        rest = &rest[open + 1 + close_rel + 1..];
    }
}

/// As [`seed_bracketed_identifier_placeholders`], over the raw text of
/// every argument of a macro/struct call.
fn seed_repeat_style_argument_placeholders<P: MacroParamElement>(args: &[P], env: &mut Env) {
    for arg in args {
        if arg.is_list() {
            for item in arg.list_argument() {
                seed_repeat_style_argument_placeholders(std::slice::from_ref(item.as_ref()), env);
            }
        }
        else {
            seed_bracketed_identifier_placeholders(&arg.single_argument(), env);
        }
    }
}

/// Assemble `listing` for real (following real `include`s, exactly like a
/// real build) with `dry_run` set and a silent observer, and return the
/// resulting `Env` regardless of whether assembling itself succeeded — a
/// hover only needs whatever got registered into the symbol table before
/// any error, if any.
///
/// `doc_uri` is the hovered document's own location: without a search path
/// covering it, a real `include` referenced by a relative path — anything
/// before the first `include` line, in practice — fails to resolve and
/// aborts assembling before later definitions (e.g. a `MACRO` after the
/// includes) are ever registered. Real `include`s are conventionally
/// written relative to a project root rather than the including file's own
/// directory, so every ancestor directory up to the project root is added
/// (`super::definition::ancestor_search_directories`, the same walk that
/// already powers the working "hover an include filename" / goto-definition
/// features) — not just the file's own directory.
pub(super) fn dry_run_env(listing: &LocatedListing, doc_uri: &Url) -> Env {
    let mut assemble = AssemblingOptions::default();
    assemble.set_dry_run(true);
    // `quiet`: re-parsing an `include`d file during assembling goes through
    // this `ParserOptions` too — without it, a `PRINT_PARSE` inside an
    // included file would still hit the real stdout despite `dry_run`.
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    for dir in super::definition::ancestor_search_directories(doc_uri) {
        let _ = parse.add_search_path(dir);
    }
    let options = EnvOptions::new(parse, assemble, Arc::new(DiscardObserver));

    match cpclib_asm::assembler::visit_tokens_all_passes_with_options(listing, options) {
        Ok((_tokens, env)) => env,
        Err((_tokens, env, _err)) => env
    }
}

/// Expand `text` (the result of one macro/struct `expand()` call) further:
/// for any of its lines that is itself a whole macro/struct call resolvable
/// in `env`, replace that line with *its* expansion too — repeated up to
/// `MAX_EXPANSION_DEPTH` times, or until no more resolvable nested calls are
/// found. `expand()` is pure text substitution (never re-parses/executes
/// its own output), so without this, a macro body that calls another macro
/// shows that nested call as raw unexpanded text.
///
/// Works at line granularity rather than exact byte-span splicing: a call
/// token's span only gives its *start* location (`MayHaveSpan`), not where
/// its argument list ends, and macro bodies conventionally write nested
/// calls one per line — matching how the reported real-world case
/// (`PRINT_FORMATTED_STRING_FROM_HL()` calling further macros on their own
/// lines) is actually written.
fn expand_nested_calls(mut text: String, env: &mut Env) -> String {
    for _ in 0..MAX_EXPANSION_DEPTH {
        let Ok(listing) = LocatedListing::new_complete_source(
            &text,
            cpclib_asm::parser::context::ParserContextBuilder::default().set_quiet(true)
        )
        else {
            break;
        };

        let Some(call) = super::token::flatten_listing(listing.iter())
            .into_iter()
            .find(|t| t.is_call_macro_or_build_struct())
        else {
            break; // nothing left to expand
        };

        let name = call.macro_call_name().to_string();
        seed_repeat_style_argument_placeholders(call.macro_call_arguments(), env);
        let expanded = if let Ok(Some(value_macro)) = env.symbols().macro_value(name.as_str()) {
            let value_macro = value_macro.clone();
            MacroWithArgs::build(&value_macro, call.macro_call_arguments())
                .and_then(|m| m.expand(env))
                .ok()
        }
        else if let Ok(Some(r#struct)) = env.symbols().struct_value(name.as_str()) {
            let r#struct = r#struct.clone();
            StructWithArgs::build(&r#struct, call.macro_call_arguments())
                .and_then(|s| s.expand(env))
                .ok()
        }
        else {
            None
        };

        // Unresolvable (unknown definition, wrong arity, ...) — leave this
        // call as raw text rather than failing the whole hover.
        let Some(expanded) = expanded
        else {
            break;
        };

        let (line_1based, _col) = call.span().relative_line_and_column();
        let line_idx = line_1based.saturating_sub(1);
        let lines: Vec<&str> = text.lines().collect();
        let Some(&target_line) = lines.get(line_idx)
        else {
            break;
        };
        let indent: String = target_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        let replacement: String = expanded
            .trim_end()
            .lines()
            .map(|l| format!("{indent}{l}"))
            .collect::<Vec<_>>()
            .join("\n");

        let mut new_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        new_lines[line_idx] = replacement;
        text = new_lines.join("\n");
    }
    text
}

impl AssemblyAnalyzer {
    /// Hover preview for a macro/struct call under the cursor: the exact
    /// expanded source it produces for these arguments, or a graceful
    /// explanation of why it couldn't be resolved/expanded.
    pub(super) fn macro_or_struct_call_hover(
        &self,
        document: &Document,
        position: Position
    ) -> Option<String> {
        let listing = self.parse_document(document).ok()?;
        let call = super::token::flatten_listing(listing.iter())
            .into_iter()
            .find(|t| t.is_call_macro_or_build_struct() && span_contains(*t, position))?;
        let name = call.macro_call_name().to_string();

        let mut env = dry_run_env(&listing, &document.uri);

        let header = format!("**{name}({})**", macro_call_args_display(call));

        let is_macro = matches!(env.symbols().macro_value(name.as_str()), Ok(Some(_)));
        if is_macro {
            let Ok(Some(value_macro)) = env.symbols().macro_value(name.as_str())
            else {
                return None;
            };
            let value_macro = value_macro.clone();
            seed_repeat_style_argument_placeholders(call.macro_call_arguments(), &mut env);
            return match MacroWithArgs::build(&value_macro, call.macro_call_arguments())
                .and_then(|m| m.expand(&mut env))
            {
                Ok(expanded) => {
                    let expanded = expand_nested_calls(expanded, &mut env);
                    Some(format!(
                        "{header} expands to:\n\n```z80\n{}\n```",
                        expanded.trim_end()
                    ))
                },
                Err(e) => {
                    Some(format!(
                        "{header} — could not expand this macro call:\n\n```\n{e}\n```"
                    ))
                },
            };
        }

        match env.symbols().struct_value(name.as_str()) {
            Ok(Some(r#struct)) => {
                let r#struct = r#struct.clone();
                seed_repeat_style_argument_placeholders(call.macro_call_arguments(), &mut env);
                match StructWithArgs::build(&r#struct, call.macro_call_arguments())
                    .and_then(|s| s.expand(&mut env))
                {
                    Ok(expanded) => {
                        let expanded = expand_nested_calls(expanded, &mut env);
                        Some(format!(
                            "{header} expands to:\n\n```z80\n{}\n```",
                            expanded.trim_end()
                        ))
                    },
                    Err(e) => {
                        Some(format!(
                            "{header} — could not expand this struct call:\n\n```\n{e}\n```"
                        ))
                    },
                }
            },
            _ => Some(format!("{header} — definition of `{name}` not found."))
        }
    }

    /// Hover preview for a user-defined `FUNCTION` call under the cursor:
    /// its evaluated return value, or a graceful explanation of why it
    /// couldn't be evaluated (e.g. a parameter that isn't yet resolvable).
    pub(super) fn function_call_hover(
        &self,
        document: &Document,
        line: &str,
        col: usize
    ) -> Option<String> {
        let (name, call_text) = function_call_at(line, col)?;

        let listing = self.parse_document(document).ok()?;
        let mut env = dry_run_env(&listing, &document.uri);

        if env.user_defined_function(&name).is_err() {
            // Not a user-defined FUNCTION — could be a hard-coded builtin
            // (list_new, mode0_byte_to_pen_at, ...) or just an ordinary
            // identifier that happens to be followed by `(`; either way,
            // this feature only covers user-defined FUNCTION calls.
            return None;
        }

        let Ok(call_listing) = cpclib_asm::parser::obtained::LocatedListing::new_complete_source(
            &format!("DEFB {call_text}"),
            cpclib_asm::parser::context::ParserContextBuilder::default().set_quiet(true)
        )
        else {
            return Some(format!(
                "**{call_text}** — could not parse this expression."
            ));
        };
        let expr = call_listing
            .iter()
            .find(|t| t.is_db())
            .and_then(|t| t.data_exprs().first().cloned())?;

        seed_bracketed_identifier_placeholders(&call_text, &mut env);
        match expr.resolve(&mut env) {
            Ok(value) => Some(format!("**{call_text}** = `{value}`")),
            Err(e) => {
                Some(format!(
                    "**{call_text}** — could not evaluate this call:\n\n```\n{e}\n```"
                ))
            },
        }
    }
}

/// Does `token`'s own span start on the line/column `position` points at?
/// A macro/struct call's span starts at its name, so this matches when the
/// cursor sits on (or just after the start of) that name.
fn span_contains<T: cpclib_asm::parser::obtained::MayHaveSpan + ListingElement>(
    token: &T,
    position: Position
) -> bool {
    let span = token.span();
    let (line_1based, col_1based) = span.relative_line_and_column();
    let line = line_1based.saturating_sub(1) as u32;
    let start_col = col_1based.saturating_sub(1) as u32;
    if line != position.line {
        return false;
    }
    let name_len = token.macro_call_name().len() as u32;
    position.character >= start_col && position.character < start_col + name_len
}

/// A human-readable rendering of a macro/struct call's arguments, for the
/// hover header (e.g. `1, "text", label`).
fn macro_call_args_display<T: cpclib_tokens::ListingElement>(token: &T) -> String {
    token
        .macro_call_arguments()
        .iter()
        .map(macro_param_display)
        .collect::<Vec<_>>()
        .join(", ")
}

fn macro_param_display<P: cpclib_tokens::MacroParamElement>(param: &P) -> String {
    if param.is_list() {
        format!(
            "[{}]",
            param
                .list_argument()
                .iter()
                .map(|p| macro_param_display(p.as_ref()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
    else {
        param.single_argument().to_string()
    }
}

/// If `col` sits on a call to a user-defined `FUNCTION` (an identifier
/// immediately, or after whitespace, followed by `(...)`), return its name
/// and the raw `name(args)` call text.
fn function_call_at(line: &str, col: usize) -> Option<(String, String)> {
    let bytes = line.as_bytes();
    let col = col.min(bytes.len());
    let mut start = col;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    if start >= end {
        return None;
    }
    let name = &line[start..end];

    let mut i = end;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'(' {
        return None;
    }
    i += 1;
    let mut depth = 1i32;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    if depth != 0 {
        return None;
    }
    Some((name.to_string(), line[start..i].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    fn hover_md_at(text: &str, line: u32, character: u32) -> Option<String> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().macro_or_struct_call_hover(&doc, Position { line, character })
    }

    #[test]
    fn macro_call_hover_shows_the_expanded_body() {
        let text = "MACRO greet name\n  db \"{name}\"\nENDM\n\ngreet \"hello\"\n";
        // Cursor on "greet" at the call site (line 4).
        let hover = hover_md_at(text, 4, 1).expect("expected an expansion hover");
        assert!(hover.contains("hello"), "{hover}");
    }

    #[test]
    fn macro_call_hover_expands_nested_macro_calls_iteratively() {
        // A macro body calling another macro (one call per line, as macro
        // bodies conventionally write them) must show the fully expanded
        // result, not the inner call left as raw unexpanded text.
        let text = "MACRO inner\n  db \"nested\"\nENDM\n\n\
                     MACRO outer\n  inner()\nENDM\n\n\
                     outer()\n";
        // Cursor on "outer" at the call site (line 8).
        let hover = hover_md_at(text, 8, 1).expect("expected an expansion hover");
        assert!(hover.contains("nested"), "{hover}");
        assert!(!hover.contains("inner()"), "{hover}");
    }

    #[test]
    fn macro_call_hover_with_wrong_arity_fails_gracefully() {
        let text = "MACRO greet name\n  db \"{name}\"\nENDM\n\ngreet \"a\", \"b\"\n";
        let hover = hover_md_at(text, 4, 1).expect("expected a graceful failure hover");
        assert!(hover.contains("could not expand"), "{hover}");
    }

    /// Regression test for the real-world pattern the user pointed out:
    /// `SOME_MACRO({eval}{channelNumber})`, where `channelNumber` is
    /// conventionally a `REPEAT` loop's counter — hovering this call in
    /// isolation used to fail outright ("Unknown symbol: {channelNumber}"),
    /// since `channelNumber` has no value outside of a real per-iteration
    /// assembling pass. A seeded placeholder now lets it expand instead.
    #[test]
    fn macro_call_hover_seeds_a_placeholder_for_a_repeat_style_brace_argument() {
        let text = "MACRO show val\n  db {eval}{val}\nENDM\n\nshow({eval}{channelNumber})\n";
        // Cursor on "show" at the call site (line 4).
        let hover = hover_md_at(text, 4, 1).expect("expected an expansion hover");
        assert!(hover.contains("expands to"), "{hover}");
        assert!(!hover.contains("could not expand"), "{hover}");
        assert!(!hover.contains("Unknown symbol"), "{hover}");
    }

    #[test]
    fn struct_call_hover_shows_the_expanded_fields() {
        let text = "STRUCT point\n  x DB 0\n  y DB 0\nENDSTRUCT\n\npoint 1, 2\n";
        // Cursor on "point" at the instantiation site (line 5).
        let hover = hover_md_at(text, 5, 1).expect("expected an expansion hover");
        assert!(hover.contains('1') && hover.contains('2'), "{hover}");
    }

    #[test]
    fn macro_or_struct_call_hover_on_an_unrelated_word_is_none() {
        let text = "MACRO greet name\n  db \"{name}\"\nENDM\n\nLD A,1\n";
        assert!(hover_md_at(text, 4, 1).is_none());
    }

    fn function_hover_at(text: &str, line: &str, character: usize) -> Option<String> {
        let uri = Url::parse("file:///t.asm").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        AssemblyAnalyzer::new().function_call_hover(&doc, line, character)
    }

    #[test]
    fn function_call_hover_shows_the_evaluated_result() {
        let text = "FUNCTION double(x)\n  RETURN x*2\nENDFUNCTION\n\nDB double(5)\n";
        let line = "DB double(5)";
        let col = line.find("double").unwrap() + 2;
        let hover = function_hover_at(text, line, col).expect("expected an evaluated hover");
        assert!(hover.contains("10"), "{hover}");
    }

    #[test]
    fn function_call_hover_on_an_unresolvable_symbol_fails_gracefully() {
        let text = "FUNCTION double(x)\n  RETURN x*UNDEFINED_SYM\nENDFUNCTION\n\nDB double(5)\n";
        let line = "DB double(5)";
        let col = line.find("double").unwrap() + 2;
        let hover = function_hover_at(text, line, col).expect("expected a graceful failure hover");
        assert!(hover.contains("could not evaluate"), "{hover}");
    }

    #[test]
    fn function_call_hover_on_a_non_function_identifier_is_none() {
        let text = "LD A,1\n";
        let line = "LD A,1";
        assert!(function_hover_at(text, line, 1).is_none());
    }

    #[test]
    fn function_call_at_extracts_name_and_call_text() {
        let line = "  db double(5)";
        let col = line.find("double").unwrap() + 2;
        let (name, call) = function_call_at(line, col).unwrap();
        assert_eq!(name, "double");
        assert_eq!(call, "double(5)");
    }

    /// The core safety invariant this whole module is built around:
    /// hovering a macro call must never trigger the real side effects
    /// (file writes, in this case `SAVE`) of *other*, unrelated statements
    /// elsewhere in the same document. The synthetic snippet built for the
    /// assembler run must never include the `SAVE` line at all.
    #[test]
    fn hovering_a_macro_call_never_writes_a_file_from_an_unrelated_save() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let target = tmp.path().join("should_not_be_written.bin");
        let uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let text = format!(
            "MACRO greet name\n  db \"{{name}}\"\nENDM\n\n\
             ORG 0\n\
             SAVE \"{}\", 0, 1\n\n\
             greet \"hello\"\n",
            target
        );
        let doc = Document::new(uri, text, 1);

        // Cursor on "greet" at the call site (line 7).
        let hover = AssemblyAnalyzer::new()
            .macro_or_struct_call_hover(
                &doc,
                Position {
                    line: 7,
                    character: 1
                }
            )
            .expect("expected an expansion hover");
        assert!(hover.contains("hello"), "{hover}");

        assert!(
            !target.exists(),
            "hovering must never execute an unrelated SAVE directive"
        );
    }

    /// Regression test: a real `include` before a macro definition must not
    /// prevent that macro from being found. `dry_run_env` assembles the real
    /// whole document, which means real `include`s are really resolved — if
    /// the document's own directory isn't in the search path, the first
    /// `include` fails to resolve, aborting assembling before later
    /// definitions (like this macro) are ever registered.
    #[test]
    fn macro_defined_after_a_real_include_is_still_found() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("sub.asm"), "; nothing needed here\n").unwrap();
        let uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let text = "include \"sub.asm\"\n\n\
                     MACRO greet name\n  db \"{name}\"\nENDM\n\n\
                     greet \"hello\"\n";
        let doc = Document::new(uri, text.to_string(), 1);

        // Cursor on "greet" at the call site (line 6).
        let hover = AssemblyAnalyzer::new()
            .macro_or_struct_call_hover(
                &doc,
                Position {
                    line: 6,
                    character: 1
                }
            )
            .expect("expected an expansion hover");
        assert!(hover.contains("hello"), "{hover}");
    }

    /// Regression test for the real-world shape of the reported bug: the
    /// including file lives one directory below a project root
    /// (`<root>/src/main.asm`), and its `include` is written relative to
    /// that root (`include 'src/demosystem/foo.asm'`), not relative to its
    /// own directory (`src/`) — a single search-path directory (just the
    /// file's own) doesn't resolve this; the ancestor walk up to the
    /// project-root marker (here a `Makefile`) is what makes it work.
    #[test]
    fn macro_found_when_include_is_relative_to_a_project_root_above_the_file() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Makefile"), "").unwrap(); // project-root marker
        std::fs::create_dir_all(tmp.path().join("src/demosystem")).unwrap();
        std::fs::write(
            tmp.path().join("src/demosystem/basic_macros.asm"),
            "; nothing needed here\n"
        )
        .unwrap();
        let uri = Url::from_file_path(tmp.path().join("src/main.asm")).unwrap();
        let text = "include 'src/demosystem/basic_macros.asm'\n\n\
                     MACRO greet name\n  db \"{name}\"\nENDM\n\n\
                     greet \"hello\"\n";
        let doc = Document::new(uri, text.to_string(), 1);

        // Cursor on "greet" at the call site (line 6).
        let hover = AssemblyAnalyzer::new()
            .macro_or_struct_call_hover(
                &doc,
                Position {
                    line: 6,
                    character: 1
                }
            )
            .expect("expected an expansion hover");
        assert!(hover.contains("hello"), "{hover}");
    }
}

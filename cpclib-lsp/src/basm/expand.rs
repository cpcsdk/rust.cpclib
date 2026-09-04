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
use super::parse::disabled_assembling_warning_categories;
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
///
/// `case_sensitive`/`disabled_categories` forward this document's config
/// (see `common::config::AsmConfig`) into the real assembling pipeline -
/// `disabled_categories` is the real gate for `override_memory`/`overflow`
/// (only known at real assemble time), and a belt-and-suspenders backstop
/// for `fake_instructions`/`redundant_accumulator_prefix` (whose real gate
/// is in the parser itself, see `parse_source`).
pub(super) fn dry_run_env(
    listing: &LocatedListing,
    doc_uri: &Url,
    case_sensitive: bool,
    disabled_categories: enumflags2::BitFlags<cpclib_asm::WarningCategory>
) -> (Env, bool) {
    let mut assemble = AssemblingOptions::default();
    assemble.set_dry_run(true);
    assemble.set_case_sensitive(case_sensitive);
    // One extra HashMap insert per token during an already-cached assemble is
    // noise; this is what lets address-aware peephole-optimizer constraints
    // (`cpclib-asmoptim`'s `reachableByJr`) work off the same dry run instead
    // of needing a second cached `Env` variant.
    assemble.set_record_token_addresses(true);
    for category in disabled_categories.iter() {
        assemble.disable_warning_category(category);
    }
    // `quiet`: re-parsing an `include`d file during assembling goes through
    // this `ParserOptions` too — without it, a `PRINT_PARSE` inside an
    // included file would still hit the real stdout despite `dry_run`.
    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    parse.set_disabled_warning_categories(disabled_categories);
    for dir in super::definition::ancestor_search_directories(doc_uri) {
        let _ = parse.add_search_path(dir);
    }
    let options = EnvOptions::new(parse, assemble, Arc::new(DiscardObserver));

    // This is the one call in the whole diagnostics/hover/semantic-tokens
    // path that can genuinely mean a real, full, multi-pass assemble -
    // tens of seconds on a real demo. Timed unconditionally (a single
    // `Instant`/`.elapsed()` is negligible next to the call itself) so a
    // slow open is directly diagnosable from a `cpclib-lsp.toml`-enabled
    // log instead of only being inferable from a gap between other lines.
    let start = std::time::Instant::now();
    let result = match cpclib_asm::assembler::visit_tokens_all_passes_with_options(listing, options) {
        Ok((_tokens, env)) => (env, true),
        // The partial `Env` is still returned: hover and `EQU` resolution use
        // it happily, and a half-built symbol table is better than none for
        // those. But it is flagged, because anything *address*-shaped read
        // from it is fiction - the addresses recorded before the failure
        // describe a program that was never finished being laid out.
        Err((_tokens, env, _err)) => (env, false)
    };
    tracing::debug!("dry_run_env for {} took {:?}", doc_uri, start.elapsed());
    result
}

/// Build an `Env` good enough to resolve *this file's own* `EQU`/`SET`
/// values, without a real assemble.
///
/// `dry_run_env` (a real, full multi-pass assemble) only produces a
/// correct result when run from a project's actual root/entry file — it
/// needs the real include graph, `ORG`/address tracking, and everything
/// else a real build does. For any other file (which is most files a user
/// might be hovering in — a shared routines/library file, anything that's
/// only ever `include`d, never assembled standalone), it either aborts on
/// an unresolvable `include` or produces an incomplete/wrong symbol table
/// — and either way, it's needlessly expensive for what most hover
/// features actually need, which is just "what number did this `EQU`
/// resolve to."
///
/// This instead walks the document's own `EQU`/`SET`/`ASSIGN` tokens
/// top-to-bottom, resolving each one's expression against whatever's
/// already been resolved so far (so `B EQU A+1` sees `A`'s value once `A`'s
/// own line has been processed) and inserting the result directly into a
/// fresh `Env`'s symbol table via `assign_symbol_to_value` — the same
/// direct-insert primitive `seed_bracketed_identifier_placeholders` already
/// uses, no assembler pipeline involved. Handles bare literals (needs no
/// symbols at all — `Env::default()` alone already resolves those) and
/// local, non-forward-referencing `EQU`/`SET` chains, the overwhelming
/// common case for hover value substitution. Does **not** resolve label
/// addresses (needs real address tracking) or symbols defined in another
/// file (needs real `include`-following) — those stay unresolved, same as
/// any other symbol this document doesn't itself define.
pub(super) fn local_symbols_env(listing: &LocatedListing) -> Env {
    let mut env = Env::default();
    for token in super::token::flatten_listing(listing.iter()) {
        if token.is_equ() {
            let sym = token.equ_symbol().to_string();
            if let Ok(v) = token.equ_value().resolve(&mut env)
                && let Ok(i) = v.int_value()
            {
                let _ = env
                    .symbols_mut()
                    .assign_symbol_to_value(sym, Value::from(i));
            }
        }
        else if token.is_assign() {
            let sym = token.assign_symbol().to_string();
            if let Ok(v) = token.assign_value().resolve(&mut env)
                && let Ok(i) = v.int_value()
            {
                let _ = env
                    .symbols_mut()
                    .assign_symbol_to_value(sym, Value::from(i));
            }
        }
    }
    env
}

impl AssemblyAnalyzer {
    /// `dry_run_env`, cached per `(document.uri, document.version)` -
    /// reuses the same `(i32, Arc<T>)` shape and eviction (`evict`, on
    /// `textDocument/didClose`) as `parse_cache`. Only for features that
    /// genuinely need a real assemble (cross-file macro/`FUNCTION`/`STRUCT`
    /// lookup, or real assembler warnings) — most hover value-substitution
    /// needs should use `local_symbols_env_cached` instead, see its own
    /// doc comment for why.
    pub(super) fn dry_run_env_cached(&self, document: &Document, listing: &LocatedListing) -> Env {
        self.dry_run_env_cached_checked(document, listing).0
    }

    /// [`Self::dry_run_env_cached`], plus whether the assemble that produced
    /// the `Env` actually finished.
    ///
    /// Every *address*-shaped question has to ask for this. A failed assemble
    /// still hands back a usable partial `Env` - which is right for hover and
    /// `EQU` values - but the addresses in it belong to a program that was
    /// never fully laid out. Reading `reachableByJr` off one told a user a
    /// jump target was 127 bytes away when the real build measured 146, and
    /// the resulting `jr` did not assemble.
    ///
    /// Keyed by `(document.version, workspace fingerprint)`, not just
    /// version: `dry_run_env` follows `include`s at any depth, so editing an
    /// included file (in another buffer, or on disk) changes this result
    /// without touching this document's own version. Same treatment as
    /// `peephole::address_source_cache` - see [`super::workspace_fingerprint_of`].
    pub(super) fn dry_run_env_cached_checked(
        &self,
        document: &Document,
        listing: &LocatedListing
    ) -> (Env, bool) {
        let key = (document.version, super::workspace_fingerprint_of(&document.uri));
        if let Some(entry) = self.env_cache.get(&document.uri)
            && entry.0 == key
        {
            return ((*entry.1).clone(), entry.2);
        }
        let config = self.config();
        let disabled = disabled_assembling_warning_categories(&config.warnings);
        let (env, complete) = dry_run_env(listing, &document.uri, config.case_sensitive, disabled);
        self.env_cache
            .insert(document.uri.clone(), (key, Arc::new(env.clone()), complete));
        (env, complete)
    }

    /// `local_symbols_env`, cached per `(document.uri, document.version)` -
    /// the right choice for most hover value-substitution needs (see
    /// `local_symbols_env`'s own doc comment for why it's preferred over
    /// `dry_run_env_cached`'s real assemble). Same cache shape, own cache
    /// map (`local_env_cache`) since it holds a different `Env` than
    /// `dry_run_env_cached` for the same document/version.
    pub(super) fn local_symbols_env_cached(
        &self,
        document: &Document,
        listing: &LocatedListing
    ) -> Env {
        if let Some(entry) = self.local_env_cache.get(&document.uri)
            && entry.0 == document.version
        {
            return (*entry.1).clone();
        }
        let env = local_symbols_env(listing);
        self.local_env_cache.insert(
            document.uri.clone(),
            (document.version, Arc::new(env.clone()))
        );
        env
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
            .find(|t| t.is_call_macro_or_build_struct() && span_contains(*t, position))?;
        let name = call.macro_call_name().to_string();

        let mut env = self.dry_run_env_cached(document, &listing);

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
        let mut env = self.dry_run_env_cached(document, &listing);

        if env.user_defined_function(&name).is_err() {
            // Not a user-defined FUNCTION — could be a hard-coded builtin
            // (list_new, mode0_byte_to_pen_at, ...) or just an ordinary
            // identifier that happens to be followed by `(`; either way,
            // this feature only covers user-defined FUNCTION calls.
            return None;
        }

        let Ok(call_listing) = cpclib_asm::parser::obtained::LocatedListing::new_complete_source(
            format!("DEFB {call_text}"),
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

#[cfg(test)]
mod dry_run_env_cache_tests {
    use super::*;

    fn doc(text: &str, version: i32) -> Document {
        Document::new(
            Url::parse("file:///t.asm").unwrap(),
            text.to_string(),
            version
        )
    }

    #[test]
    fn same_version_reuses_the_cached_env_without_recomputing() {
        let analyzer = AssemblyAnalyzer::new();
        let d = doc("val equ 9\n", 1);
        let listing = analyzer.parse_document(&d).ok().unwrap();

        let _ = analyzer.dry_run_env_cached(&d, &listing);
        let first = analyzer.env_cache.get(&d.uri).unwrap().1.clone();

        let _ = analyzer.dry_run_env_cached(&d, &listing);
        let second = analyzer.env_cache.get(&d.uri).unwrap().1.clone();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_version_bump_recomputes_the_cached_env() {
        let analyzer = AssemblyAnalyzer::new();
        let d1 = doc("val equ 9\n", 1);
        let listing1 = analyzer.parse_document(&d1).ok().unwrap();
        let _ = analyzer.dry_run_env_cached(&d1, &listing1);
        let first = analyzer.env_cache.get(&d1.uri).unwrap().1.clone();

        let d2 = doc("val equ 10\n", 2);
        let listing2 = analyzer.parse_document(&d2).ok().unwrap();
        let _ = analyzer.dry_run_env_cached(&d2, &listing2);
        let second = analyzer.env_cache.get(&d2.uri).unwrap().1.clone();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn evict_clears_the_cached_env() {
        let analyzer = AssemblyAnalyzer::new();
        let d = doc("val equ 9\n", 1);
        let listing = analyzer.parse_document(&d).ok().unwrap();
        let _ = analyzer.dry_run_env_cached(&d, &listing);
        assert_eq!(analyzer.env_cache.len(), 1);

        analyzer.evict(&d.uri);
        assert_eq!(analyzer.env_cache.len(), 0);
    }

    /// Regression test for the missing-fingerprint bug: `dry_run_env`
    /// follows `include`s at any depth, but the cache check used to be
    /// only `entry.0 == document.version` - no awareness of the include
    /// tree - so editing an included file (without touching the
    /// "root" document's own version) silently served a stale `Env`
    /// forever, even though the real assemble would have produced a
    /// different one. A workspace-roots fingerprint (`Cargo.toml` here,
    /// so `cpclib_project::entry::root_of` finds a project root to
    /// fingerprint) must now be part of the cache key.
    #[test]
    fn editing_an_included_file_recomputes_the_cached_env_even_though_the_document_version_did_not_change()
     {
        let tmp = camino_tempfile::tempdir().unwrap();
        // A project-root marker so `workspace_fingerprint_of` finds
        // something to fingerprint instead of falling back to a constant
        // `0` (which would defeat this very test).
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let helper_path = tmp.path().join("helper.asm");
        std::fs::write(&helper_path, "val equ 1\n").unwrap();
        let main_path = tmp.path().join("main.asm");
        let text = "include \"helper.asm\"\n";
        std::fs::write(&main_path, text).unwrap();

        let uri = Url::from_file_path(&main_path).unwrap();
        let d = Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let listing = analyzer.parse_document(&d).ok().unwrap();

        let (env1, _) = analyzer.dry_run_env_cached_checked(&d, &listing);
        assert_eq!(
            env1.symbols()
                .int_value("val")
                .ok()
                .flatten(),
            Some(1)
        );

        // Edit the *included* file only - `d`'s own version is untouched -
        // and bump its mtime forward like the sibling
        // `disk_file_version_tests`/`address_source_cache` tests do, since
        // the fingerprint is mtime-based and a same-second write could
        // otherwise leave it unchanged.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::write(&helper_path, "val equ 2\n").unwrap();
        std::fs::File::open(&helper_path)
            .unwrap()
            .set_modified(later)
            .unwrap();
        // `cpclib_project::entry::fingerprint_of` now memoizes its own
        // result for a short while (`FINGERPRINT_CACHE_TTL`, currently
        // 300ms) - added precisely so a burst of near-simultaneous callers
        // for the same, unchanged root (a workspace-restore `did_open`
        // storm) collapses into one real tree walk instead of one per
        // caller. Wait past it so this test observes a real recomputation,
        // not a still-warm memo from the lookup two lines up.
        std::thread::sleep(std::time::Duration::from_millis(350));

        let (env2, _) = analyzer.dry_run_env_cached_checked(&d, &listing);
        assert_eq!(
            env2.symbols()
                .int_value("val")
                .ok()
                .flatten(),
            Some(2),
            "stale cache: the included file's edit was not picked up"
        );
    }
}

#[cfg(test)]
mod local_symbols_env_tests {
    use cpclib_asm::parser::context::ParserContextBuilder;

    use super::*;

    fn env_for(text: &str) -> Env {
        let builder = ParserContextBuilder::default().set_quiet(true);
        let listing = LocatedListing::new_complete_source(text, builder)
            .unwrap_or_else(|_| panic!("expected {text:?} to parse cleanly"));
        local_symbols_env(&listing)
    }

    fn resolve(env: &mut Env, symbol: &str) -> Option<i32> {
        cpclib_asm::preamble::Expr::Label(symbol.into())
            .resolve(env)
            .ok()
            .and_then(|v| v.int_value().ok())
    }

    #[test]
    fn resolves_a_bare_equ() {
        let mut env = env_for("val equ 9\n");
        assert_eq!(resolve(&mut env, "val"), Some(9));
    }

    #[test]
    fn resolves_a_local_equ_chain_in_order() {
        let mut env = env_for("a equ 5\nb equ a+1\n");
        assert_eq!(resolve(&mut env, "a"), Some(5));
        assert_eq!(resolve(&mut env, "b"), Some(6));
    }

    #[test]
    fn resolves_a_set_assignment_too() {
        let mut env = env_for("val: set 3\n");
        assert_eq!(resolve(&mut env, "val"), Some(3));
    }

    #[test]
    fn never_errors_on_a_file_that_cannot_be_assembled_standalone() {
        // The user's own reported scenario: a file that `include`s
        // something that doesn't exist on disk here (as would be true for
        // almost any file that isn't a project's actual root/entry file) -
        // a real assemble (`dry_run_env`) would either abort or produce an
        // incomplete/wrong symbol table; `local_symbols_env` doesn't
        // attempt real inclusion at all, so this file's own local EQU
        // still resolves correctly regardless.
        let text = "include \"this/file/does/not/exist.asm\"\nval equ 42\n";
        let mut env = env_for(text);
        assert_eq!(resolve(&mut env, "val"), Some(42));
    }
}

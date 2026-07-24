//! Call hierarchy for bndbuild: YAML target dependencies ("rule A depends
//! on rule B" as the "call" relation, function = a target's rule) and Jinja
//! macro definitions/calls (function = the macro's own body).
//!
//! **Targets** are scanned from Jinja-*expanded* text (via
//! `super::sourcemap::expand_with_source_map`), not raw text, so
//! `{% for %}`-generated rules are visible — a real, common pattern in
//! bndbuild files. Every found position is translated back to the raw
//! document via the source map, applying the same "was this original line
//! templated?" fallback `diagnostics.rs`'s `validate_build_structure`
//! already established: an untemplated original line gets a precise
//! sub-range, a templated one (where expanded-text columns don't correspond
//! to any real source position) gets its whole line highlighted instead.
//!
//! **Macros**, by contrast, are scanned from the *raw* document — a
//! `{% macro %}...{% endmacro %}` block produces no output at all when
//! merely defined (Jinja only emits a macro's *result* at a call site), so
//! its own `{% macro %}` syntax never survives into rendered output; there
//! is nothing to find there. This matches the pre-existing
//! `jinja::macro_definition_names` (`jinja.rs:160-187`), which already
//! scans raw text for the same reason. No source-map translation is needed
//! for macro positions — they're already raw-document coordinates.
//!
//! Cross-document orchestration (via the `{% include %}` graph) lives in
//! `backend.rs`, mirroring `rename_jinja_variable_across_workspace` — a
//! shared cross-file source map isn't possible for targets (see the plan
//! this was designed from), and macros never had one to begin with.

use std::ops::Range as LineRange;

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use super::sourcemap::SourceMap;
use crate::common::call_hierarchy::CallHierarchyData;
use crate::common::document::Document;

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Translate a single `(line, col, len)` span in expanded-text coordinates
/// to a `Range` in the raw document. `None` when the expanded line has no
/// source counterpart at all (pure Jinja control-flow output). When the
/// *original* line was itself templated (contains `{{`/`{%`), the expanded
/// column layout can't be trusted, so the whole original line is
/// highlighted instead of a bogus sub-range — same fallback
/// `validate_build_structure` (`diagnostics.rs:216-222`) already uses.
fn translate_span(
    source_map: &SourceMap,
    raw_lines: &[&str],
    exp_line: u32,
    col: u32,
    len: u32
) -> Option<Range> {
    let orig_line = source_map.to_original(exp_line)?;
    let orig_text = raw_lines.get(orig_line as usize).copied().unwrap_or("");
    let was_templated = orig_text.contains("{{") || orig_text.contains("{%");
    if was_templated {
        Some(Range {
            start: Position {
                line: orig_line,
                character: 0
            },
            end: Position {
                line: orig_line,
                character: orig_text.chars().count() as u32
            }
        })
    }
    else {
        Some(Range {
            start: Position {
                line: orig_line,
                character: col
            },
            end: Position {
                line: orig_line,
                character: col + len
            }
        })
    }
}

/// Translate a `[exp_start, exp_end)` expanded-text line range (e.g. a
/// whole rule or macro body) into a raw-document `Range` spanning from the
/// first translatable line's start to the last translatable line's end.
/// Lines with no raw counterpart are skipped when finding the first/last
/// translatable line; `None` if none of them translate.
fn translate_line_range(
    source_map: &SourceMap,
    raw_lines: &[&str],
    exp_start: u32,
    exp_end: u32
) -> Option<Range> {
    let mut first = None;
    let mut last = None;
    for exp_line in exp_start..exp_end {
        if let Some(orig) = source_map.to_original(exp_line) {
            first.get_or_insert(orig);
            last = Some(orig);
        }
    }
    let first = first?;
    let last = last?;
    let last_len = raw_lines
        .get(last as usize)
        .map(|l| l.chars().count() as u32)
        .unwrap_or(0);
    Some(Range {
        start: Position {
            line: first,
            character: 0
        },
        end: Position {
            line: last,
            character: last_len
        }
    })
}

/// A `Range` for a `(line, col, len)` span that's already in raw-document
/// coordinates (no source-map translation needed) — used by the macro side,
/// which scans the raw document directly (see
/// `BuildFileAnalyzer::call_hierarchy_item_for_macro`'s doc comment).
fn span_range(line: u32, col: u32, len: u32) -> Range {
    Range {
        start: Position {
            line,
            character: col
        },
        end: Position {
            line,
            character: col + len
        }
    }
}

/// A `Range` spanning raw-document lines `[start, end]` (inclusive) in full
/// — the macro-side, already-raw-coordinates counterpart of
/// `translate_line_range`.
fn line_span_range(lines: &[&str], start: u32, end: u32) -> Range {
    let end_len = lines
        .get(end as usize)
        .map(|l| l.chars().count() as u32)
        .unwrap_or(0);
    Range {
        start: Position {
            line: start,
            character: 0
        },
        end: Position {
            line: end,
            character: end_len
        }
    }
}

// ─── Targets ────────────────────────────────────────────────────────────────

/// The line and indentation of the `- ` marker that starts the rule
/// containing `line_idx` — either that line itself (if it starts with
/// `- `), or found by walking backward to the nearest line whose own
/// indentation is `<=` this line's (similar backward-walk shape to
/// `enclosing_key_for_list_item`, `cpclib-lsp/src/bndbuild/token.rs:126-153`,
/// but not the same stop condition: that one wants strictly-less indentation).
fn rule_marker(lines: &[&str], line_idx: u32) -> (u32, usize) {
    let line = lines[line_idx as usize];
    let indent = indent_of(line);
    if line.trim_start().starts_with("- ") {
        return (line_idx, indent);
    }
    for i in (0..line_idx).rev() {
        let l = lines[i as usize];
        if l.trim().is_empty() {
            continue;
        }
        let li = indent_of(l);
        if li <= indent {
            return (i, li);
        }
    }
    (0, 0)
}

/// `(start, end_exclusive)` line range of the rule containing `line_idx` —
/// `start` is that rule's own `- ` marker line; `end` is the next line
/// (scanning forward, skipping blanks — YAML blanks never close a block)
/// whose indentation is `<=` the marker's, or EOF.
fn rule_bounds(lines: &[&str], line_idx: u32) -> (u32, u32) {
    let (start, marker_indent) = rule_marker(lines, line_idx);
    let mut end = lines.len() as u32;
    for i in (start + 1)..(lines.len() as u32) {
        let l = lines[i as usize];
        if l.trim().is_empty() {
            continue;
        }
        if indent_of(l) <= marker_indent {
            end = i;
            break;
        }
    }
    (start, end)
}

/// Every rule's own `(target_field_text, tgt_line)` — the multi-match
/// sibling of `find_target_line` (`definition.rs:373-409`, which stops at
/// the first exact-token match); collects every rule instead.
fn all_target_definitions(expanded_text: &str, tgt_key_names: &[&str]) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for (line_num, line) in expanded_text.lines().enumerate() {
        let trimmed = line.trim_start();
        let content = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        for &key in tgt_key_names {
            if let Some(rest) = content.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
                let value = rest.split('#').next().unwrap_or("").trim();
                if !value.is_empty() && value != ">" && value != "|" {
                    out.push((value.to_string(), line_num as u32));
                }
                break;
            }
        }
    }
    out
}

/// The column and length of `token` within `tgt_line`'s target-field value
/// — the same per-token matching `find_target_line` does, but returning the
/// token's own span instead of just the line number.
fn target_token_span(
    lines: &[&str],
    tgt_line: u32,
    token: &str,
    tgt_key_names: &[&str]
) -> Option<(u32, u32)> {
    let line = lines[tgt_line as usize];
    let trimmed = line.trim_start();
    let content = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    let content_col = (line.len() - content.len()) as u32;
    for &key in tgt_key_names {
        if let Some(rest) = content.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
            let value_full = rest.split('#').next().unwrap_or("");
            let leading_ws = (value_full.len() - value_full.trim_start().len()) as u32;
            let mut col = content_col + key.len() as u32 + 1 + leading_ws;
            for tok in value_full.trim().split_whitespace() {
                if tok == token {
                    return Some((col, tok.len() as u32));
                }
                col += tok.len() as u32 + 1;
            }
        }
    }
    None
}

/// Every dependency token in the rule bounded by `rule_bounds(tgt_line)`, as
/// `(token_text, line, col, len)` in EXPANDED-text coordinates (the caller
/// translates via `translate_span`) — handles both the inline scalar form
/// (`dep: a b c`, column-tracked like `validate_build_structure`,
/// `diagnostics.rs:224-243`) and the multi-line list form (`dep:\n  - a\n
/// - b`, mirroring `filename_under_cursor`'s handling).
fn rule_dependency_tokens(
    lines: &[&str],
    tgt_line: u32,
    dep_key_names: &[&str]
) -> Vec<(String, u32, u32, u32)> {
    let (start, end) = rule_bounds(lines, tgt_line);
    let mut out = Vec::new();
    let mut i = start;

    while i < end {
        let line = lines[i as usize];
        let trimmed = line.trim_start();
        let content = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        let content_col = (line.len() - content.len()) as u32;

        let Some((key, rest)) = dep_key_names.iter().find_map(|&key| {
            content
                .strip_prefix(key)
                .and_then(|r| r.strip_prefix(':'))
                .map(|rest| (key, rest))
        })
        else {
            i += 1;
            continue;
        };

        let value_full = rest.split('#').next().unwrap_or("");
        let value_trimmed_start = value_full.trim_start();
        let leading_ws = (value_full.len() - value_trimmed_start.len()) as u32;
        let value = value_trimmed_start.trim_end();
        let value_col = content_col + key.len() as u32 + 1 + leading_ws;

        if value.is_empty() {
            // Multi-line list form: subsequent more-indented `- item` lines.
            let dep_indent = indent_of(line);
            let mut j = i + 1;
            while j < end {
                let jl = lines[j as usize];
                if jl.trim().is_empty() {
                    j += 1;
                    continue;
                }
                if indent_of(jl) <= dep_indent {
                    break;
                }
                let jtrimmed = jl.trim_start();
                if let Some(item_rest) = jtrimmed.strip_prefix("- ") {
                    let item = item_rest.split('#').next().unwrap_or("").trim();
                    if !item.is_empty() {
                        let item_col = (jl.len() - jtrimmed.len()) as u32 + 2;
                        out.push((item.to_string(), j, item_col, item.len() as u32));
                    }
                }
                j += 1;
            }
            i = j;
        }
        else if value != ">" && value != "|" {
            let mut col = value_col;
            for tok in value.split_whitespace() {
                out.push((tok.to_string(), i, col, tok.len() as u32));
                col += tok.len() as u32 + 1;
            }
            i += 1;
        }
        else {
            i += 1; // block scalar — no discrete tokens to extract
        }
    }
    out
}

// ─── Jinja macros ───────────────────────────────────────────────────────────

/// Every `{% macro NAME(...) %}` definition on a single line, as `(name,
/// start_col, end_col)` — the per-line primitive shared by
/// `macro_definition_locations` (whole document) and
/// `macro_definition_name_at` (cursor-position check). Same matching shape
/// as `jinja::macro_definition_names` (`jinja.rs:160-187`), plus position.
pub(super) fn macro_definitions_in_line(line: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = line[search_from..].find("{%") {
        let stmt_start = search_from + rel + 2;
        let mut p = stmt_start;
        if line.as_bytes().get(p) == Some(&b'-') {
            p += 1;
        }
        while line.as_bytes().get(p) == Some(&b' ') {
            p += 1;
        }
        if line[p..].starts_with("macro") {
            let after_kw = p + "macro".len();
            if line
                .as_bytes()
                .get(after_kw)
                .is_some_and(u8::is_ascii_whitespace)
            {
                let mut name_start = after_kw;
                while line
                    .as_bytes()
                    .get(name_start)
                    .is_some_and(u8::is_ascii_whitespace)
                {
                    name_start += 1;
                }
                let mut name_end = name_start;
                while line
                    .as_bytes()
                    .get(name_end)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    name_end += 1;
                }
                if name_end > name_start {
                    out.push((line[name_start..name_end].to_string(), name_start, name_end));
                }
            }
        }
        search_from = stmt_start;
    }
    out
}

fn macro_definition_locations(expanded_text: &str) -> Vec<(String, u32, u32, u32)> {
    let mut out = Vec::new();
    for (line_num, line) in expanded_text.lines().enumerate() {
        for (name, start, end) in macro_definitions_in_line(line) {
            out.push((name, line_num as u32, start as u32, (end - start) as u32));
        }
    }
    out
}

/// If the cursor at `col` sits on a macro's own name in a
/// `{% macro NAME(...) %}` definition line, return that name.
fn macro_definition_name_at(line: &str, col: usize) -> Option<String> {
    macro_definitions_in_line(line)
        .into_iter()
        .find(|(_, start, end)| col >= *start && col < *end)
        .map(|(name, ..)| name)
}

/// Nesting-depth-aware `{% macro %}` → matching `{% endmacro %}` line range
/// (mirrors the open/close depth-tracking style used for basm block scopes,
/// e.g. `block_end_line` in `cpclib-lsp/src/basm/token.rs`).
fn macro_body_bounds(expanded_lines: &[&str], def_line: u32) -> (u32, u32) {
    let mut depth = 0i32;
    for (i, line) in expanded_lines.iter().enumerate().skip(def_line as usize) {
        if line.contains("{% macro") || line.contains("{%- macro") {
            depth += 1;
        }
        if line.contains("{% endmacro") || line.contains("{%- endmacro") {
            depth -= 1;
            if depth <= 0 {
                return (def_line, i as u32);
            }
        }
    }
    (def_line, expanded_lines.len().saturating_sub(1) as u32)
}

/// Every `NAME(` call-site position on any line, gated by
/// `jinja::jinja_context_at` (so a bare filename string is never matched) —
/// whole-document multi-match adaptation of `macro_call_at`'s
/// identifier/paren logic (`macro_expand.rs:99-137`).
fn macro_call_sites(text: &str, macro_name: &str) -> Vec<(u32, u32, u32)> {
    let mut out = Vec::new();
    for (line_num, line) in text.lines().enumerate() {
        // A `{% macro NAME(...) %}` definition's own name is immediately
        // followed by `(` too (its parameter list) — syntactically
        // identical to a real call under the "identifier + (" heuristic
        // below. Exclude the definition's own name span so a macro is never
        // reported as calling itself just by being defined.
        let def_spans: Vec<(usize, usize)> = macro_definitions_in_line(line)
            .into_iter()
            .map(|(_, s, e)| (s, e))
            .collect();

        let bytes = line.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                i += 1;
                continue;
            }
            let start = i;
            let mut end = i;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            let name = &line[start..end];
            if name == macro_name && !def_spans.contains(&(start, end)) {
                let mut p = end;
                while bytes.get(p) == Some(&b' ') {
                    p += 1;
                }
                if bytes.get(p) == Some(&b'(')
                    && super::jinja::jinja_context_at(line, start).is_some()
                {
                    out.push((line_num as u32, start as u32, (end - start) as u32));
                }
            }
            i = end.max(start + 1);
        }
    }
    out
}

/// Which macro's body (from `macro_definition_locations` +
/// `macro_body_bounds`) contains `line` — the macro-domain equivalent of
/// basm's `label_scope_at_line`. Picks the *innermost* containing macro
/// when macros nest. `None` when `line` isn't inside any macro's body (a
/// top-level call — dropped from `incoming_calls`, same convention basm
/// uses for a call with no enclosing label scope).
fn macro_scope_at_line(expanded_text: &str, line: u32) -> Option<(String, LineRange<u32>)> {
    let lines: Vec<&str> = expanded_text.lines().collect();
    macro_definition_locations(expanded_text)
        .into_iter()
        .filter_map(|(name, def_line, ..)| {
            let (start, end) = macro_body_bounds(&lines, def_line);
            (start <= line && line <= end).then_some((name, start..(end + 1)))
        })
        .min_by_key(|(_, range)| range.end - range.start)
}

// ─── Public entry points ────────────────────────────────────────────────────

impl BuildFileAnalyzer {
    /// The `CallHierarchyItem` for `target` (already resolved, e.g. from a
    /// cursor position or a dependency token) if it's defined in
    /// `document`. `range` = the defining rule's own bounds; `selection_range`
    /// = the target token's own span within its `tgt:`-alias field.
    pub fn call_hierarchy_item_for_target(
        &self,
        document: &Document,
        target: &str
    ) -> Option<CallHierarchyItem> {
        let expand_result = self.expand_or_identity(document);
        let (expanded, source_map) = (&expand_result.0, &expand_result.1);
        let lines: Vec<&str> = expanded.lines().collect();
        let raw_text = document.text();
        let raw_lines: Vec<&str> = raw_text.lines().collect();

        let tgt_exp_line = Self::find_target_line(&expanded, target, &super::token::TGT_KEY_NAMES)?;
        // The `tgt:` line itself must have a real counterpart in *this*
        // document's own raw text - otherwise it was spliced in wholesale
        // from an `{% include %}`d file (content from an include has no
        // marker, see `sourcemap.rs`), and this document doesn't actually
        // define it. Without this check, a rule-bounds window that happens
        // to swallow a Jinja-synthesized blank line mapping back to some
        // unrelated real line in this document (e.g. the `{% include %}`
        // statement's own line) would falsely "resolve" here instead of
        // leaving it for cross-file orchestration in `backend.rs` to find
        // in the file that actually defines it.
        source_map.to_original(tgt_exp_line)?;
        let (body_start, body_end) = rule_bounds(&lines, tgt_exp_line);
        let range = translate_line_range(&source_map, &raw_lines, body_start, body_end)?;
        let (col, len) =
            target_token_span(&lines, tgt_exp_line, target, &super::token::TGT_KEY_NAMES)
                .unwrap_or((0, 0));
        let selection_range =
            translate_span(&source_map, &raw_lines, tgt_exp_line, col, len).unwrap_or(range);

        Some(CallHierarchyItem {
            name: target.to_string(),
            kind: SymbolKind::FILE,
            tags: None,
            detail: None,
            uri: document.uri.clone(),
            range,
            selection_range,
            data: Some(
                CallHierarchyData::BndbuildTarget {
                    target: target.to_string()
                }
                .to_json()
            )
        })
    }

    /// Every distinct dependency of `target`'s own rule in `document`, as
    /// `(dependency_text, ranges)` — multiple references to the same
    /// dependency within the rule collapse into one entry.
    pub fn outgoing_call_targets_in(
        &self,
        document: &Document,
        target: &str
    ) -> Vec<(String, Vec<Range>)> {
        let expand_result = self.expand_or_identity(document);
        let (expanded, source_map) = (&expand_result.0, &expand_result.1);
        let lines: Vec<&str> = expanded.lines().collect();
        let raw_text = document.text();
        let raw_lines: Vec<&str> = raw_text.lines().collect();

        let Some(tgt_exp_line) =
            Self::find_target_line(&expanded, target, &super::token::TGT_KEY_NAMES)
        else {
            return Vec::new();
        };
        // See `call_hierarchy_item_for_target` for why this check is needed:
        // a `tgt:` line with no real counterpart in this document's own raw
        // text was spliced in from an `{% include %}`d file, so this
        // document doesn't actually own this rule's outgoing dependencies.
        if source_map.to_original(tgt_exp_line).is_none() {
            return Vec::new();
        }

        let mut groups: Vec<(String, Vec<Range>)> = Vec::new();
        for (tok, exp_line, col, len) in
            rule_dependency_tokens(&lines, tgt_exp_line, &super::token::DEP_KEY_NAMES)
        {
            let Some(range) = translate_span(&source_map, &raw_lines, exp_line, col, len)
            else {
                continue;
            };
            match groups.iter_mut().find(|(t, _)| *t == tok) {
                Some(g) => g.1.push(range),
                None => groups.push((tok, vec![range]))
            }
        }
        groups
    }

    /// Every rule in `document` that depends on `target`, as `(caller_target,
    /// ranges)` — a multi-target rule is identified by its own first-listed
    /// target for grouping purposes.
    pub fn incoming_calls_in(
        &self,
        document: &Document,
        target: &str
    ) -> Vec<(String, Vec<Range>)> {
        let expand_result = self.expand_or_identity(document);
        let (expanded, source_map) = (&expand_result.0, &expand_result.1);
        let lines: Vec<&str> = expanded.lines().collect();
        let raw_text = document.text();
        let raw_lines: Vec<&str> = raw_text.lines().collect();

        let mut groups: Vec<(String, Vec<Range>)> = Vec::new();
        for (owner_tgt, owner_line) in
            all_target_definitions(&expanded, &super::token::TGT_KEY_NAMES)
        {
            // Same rationale as `call_hierarchy_item_for_target`: only
            // attribute an incoming call to a rule this document actually
            // defines, not one merely visible here via `{% include %}`.
            if source_map.to_original(owner_line).is_none() {
                continue;
            }
            let matching: Vec<Range> =
                rule_dependency_tokens(&lines, owner_line, &super::token::DEP_KEY_NAMES)
                    .into_iter()
                    .filter(|(t, ..)| t == target)
                    .filter_map(|(_, l, c, len)| translate_span(&source_map, &raw_lines, l, c, len))
                    .collect();
            if matching.is_empty() {
                continue;
            }
            let owner_name = owner_tgt
                .split_whitespace()
                .next()
                .unwrap_or(&owner_tgt)
                .to_string();
            match groups.iter_mut().find(|(t, _)| *t == owner_name) {
                Some(g) => g.1.extend(matching),
                None => groups.push((owner_name, matching))
            }
        }
        groups
    }

    /// The `CallHierarchyItem` for the macro `name` if it's defined in
    /// `document`. Unlike the target side, macro definitions/bodies are
    /// scanned from the *raw* document, not the Jinja-expanded text: a
    /// `{% macro %}...{% endmacro %}` block produces no output at all when
    /// merely defined (Jinja only emits a macro's *result* at a call site),
    /// so its own `{% macro %}` syntax never survives into rendered
    /// output — searching for it there would never find anything. This
    /// matches the pre-existing `jinja::macro_definition_names`
    /// (`jinja.rs:160-187`), which already scans `document.text()` for the
    /// same reason. No source-map translation is needed either, since
    /// these positions are already raw-document coordinates.
    pub fn call_hierarchy_item_for_macro(
        &self,
        document: &Document,
        name: &str
    ) -> Option<CallHierarchyItem> {
        let text = document.text();
        let lines: Vec<&str> = text.lines().collect();

        let (def_name, def_line, col, len) = macro_definition_locations(&text)
            .into_iter()
            .find(|(n, ..)| n == name)?;
        let (body_start, body_end) = macro_body_bounds(&lines, def_line);
        let selection_range = span_range(def_line, col, len);
        let range = line_span_range(&lines, body_start, body_end);

        Some(CallHierarchyItem {
            name: def_name,
            kind: SymbolKind::FUNCTION,
            tags: None,
            detail: None,
            uri: document.uri.clone(),
            range,
            selection_range,
            data: Some(
                CallHierarchyData::JinjaMacro {
                    name: name.to_string()
                }
                .to_json()
            )
        })
    }

    /// Every distinct macro called from within `name`'s own body in
    /// `document`, as `(callee_name, ranges)` — raw text, see
    /// `call_hierarchy_item_for_macro`'s doc comment for why.
    pub fn outgoing_calls_for_macro_targets_in(
        &self,
        document: &Document,
        name: &str
    ) -> Vec<(String, Vec<Range>)> {
        let text = document.text();
        let lines: Vec<&str> = text.lines().collect();

        let defs = macro_definition_locations(&text);
        let Some((_, def_line, ..)) = defs.iter().find(|(n, ..)| n == name)
        else {
            return Vec::new();
        };
        let (body_start, body_end) = macro_body_bounds(&lines, *def_line);

        let mut groups: Vec<(String, Vec<Range>)> = Vec::new();
        for (known, ..) in &defs {
            for (line, col, len) in macro_call_sites(&text, known) {
                if line < body_start || line > body_end {
                    continue;
                }
                let range = span_range(line, col, len);
                match groups.iter_mut().find(|(n, _)| n == known) {
                    Some(g) => g.1.push(range),
                    None => groups.push((known.clone(), vec![range]))
                }
            }
        }
        groups
    }

    /// Every call site of macro `name` in `document`, grouped by caller and
    /// resolved to a full `CallHierarchyIncomingCall` directly (unlike the
    /// other `*_in` methods, which return raw `(name, ranges)` pairs for
    /// `backend.rs` to resolve) — because a caller here can be *either* an
    /// enclosing macro (`macro_scope_at_line`) *or* the enclosing bndbuild
    /// rule (`rule_bounds`, same as the target side), and those two resolve
    /// through different `call_hierarchy_item_for_*` methods. The rule case
    /// is the common one in practice: calling a macro directly from a
    /// `cmd:` field (not from inside another macro) is the normal,
    /// load-bearing pattern (confirmed against a real project — see the
    /// plan this was designed from), so it must not be dropped the way
    /// basm drops a call with no enclosing label. A call inside neither any
    /// rule nor any macro (e.g. a bare top-level `{% set %}`) has no
    /// meaningful caller and is the only case actually dropped.
    pub fn incoming_calls_for_macro_in(
        &self,
        document: &Document,
        name: &str
    ) -> Vec<CallHierarchyIncomingCall> {
        let text = document.text();
        let lines: Vec<&str> = text.lines().collect();

        let mut macro_groups: Vec<(String, Vec<Range>)> = Vec::new();
        let mut rule_groups: Vec<(String, Vec<Range>)> = Vec::new();

        for (line, col, len) in macro_call_sites(&text, name) {
            let range = span_range(line, col, len);
            if let Some((caller, _)) = macro_scope_at_line(&text, line) {
                match macro_groups.iter_mut().find(|(n, _)| *n == caller) {
                    Some(g) => g.1.push(range),
                    None => macro_groups.push((caller, vec![range]))
                }
                continue;
            }
            let owner = all_target_definitions(&text, &super::token::TGT_KEY_NAMES)
                .into_iter()
                .find_map(|(owner_tgt, owner_line)| {
                    let (start, end) = rule_bounds(&lines, owner_line);
                    (start <= line && line < end).then(|| {
                        owner_tgt
                            .split_whitespace()
                            .next()
                            .unwrap_or(&owner_tgt)
                            .to_string()
                    })
                });
            if let Some(owner) = owner {
                match rule_groups.iter_mut().find(|(n, _)| *n == owner) {
                    Some(g) => g.1.push(range),
                    None => rule_groups.push((owner, vec![range]))
                }
            }
            // else: not inside any rule or macro - no meaningful caller.
        }

        let mut calls = Vec::new();
        for (caller, ranges) in macro_groups {
            if let Some(from) = self.call_hierarchy_item_for_macro(document, &caller) {
                calls.push(CallHierarchyIncomingCall {
                    from,
                    from_ranges: ranges
                });
            }
        }
        for (caller, ranges) in rule_groups {
            if let Some(from) = self.call_hierarchy_item_for_target(document, &caller) {
                calls.push(CallHierarchyIncomingCall {
                    from,
                    from_ranges: ranges
                });
            }
        }
        calls
    }

    /// Resolve the cursor to a call-hierarchy item: a target/dependency
    /// field (via the raw document, matching `goto_definition`'s own cursor
    /// resolution), else a Jinja macro call site or the macro's own
    /// definition name.
    pub fn prepare_call_hierarchy(
        &self,
        document: &Document,
        position: Position
    ) -> Option<CallHierarchyItem> {
        let line = document.line(position.line as usize)?;
        let col = document.byte_column(position);

        if let Some((token, _is_target_field)) = Self::filename_under_cursor(
            document,
            position.line as usize,
            &line,
            &super::token::FILE_KEY_NAMES,
            &super::token::TGT_KEY_NAMES,
            col
        ) && !token.contains("{{")
            && !token.contains("{%")
            && let Some(item) = self.call_hierarchy_item_for_target(document, token)
        {
            return Some(item);
        }

        if let Some((name, _args)) = super::macro_expand::macro_call_at(&line, col) {
            let text = document.text();
            if macro_definition_locations(&text)
                .iter()
                .any(|(n, ..)| *n == name)
            {
                return self.call_hierarchy_item_for_macro(document, &name);
            }
        }

        if let Some(name) = macro_definition_name_at(&line, col) {
            return self.call_hierarchy_item_for_macro(document, &name);
        }

        None
    }

    /// What the cursor names, when `prepare_call_hierarchy` finds nothing
    /// *locally* — a target/dependency token or a macro call/definition
    /// name — without requiring it to actually resolve in this document.
    /// `backend.rs` uses this to retry resolution across the `{% include %}`
    /// graph (mirroring `resolve_bndbuild_item`) when the definition lives
    /// in another file, e.g. a macro call site in an includer whose
    /// `{% macro %}` definition lives only in the included file.
    pub fn call_hierarchy_candidate_at(
        &self,
        document: &Document,
        position: Position
    ) -> Option<CallHierarchyCandidate> {
        let line = document.line(position.line as usize)?;
        let col = document.byte_column(position);

        if let Some((token, _is_target_field)) = Self::filename_under_cursor(
            document,
            position.line as usize,
            &line,
            &super::token::FILE_KEY_NAMES,
            &super::token::TGT_KEY_NAMES,
            col
        ) && !token.contains("{{")
            && !token.contains("{%")
        {
            return Some(CallHierarchyCandidate::Target(token.to_string()));
        }

        if let Some((name, _args)) = super::macro_expand::macro_call_at(&line, col) {
            return Some(CallHierarchyCandidate::Macro(name));
        }

        if let Some(name) = macro_definition_name_at(&line, col) {
            return Some(CallHierarchyCandidate::Macro(name));
        }

        None
    }
}

/// See [`BuildFileAnalyzer::call_hierarchy_candidate_at`].
pub enum CallHierarchyCandidate {
    Target(String),
    Macro(String)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(
            Url::parse("file:///build.bnd").unwrap(),
            text.to_string(),
            1
        )
    }

    // ─── Targets ────────────────────────────────────────────────────────

    #[test]
    fn prepare_call_hierarchy_names_the_rule_under_the_cursor() {
        let d = doc("- tgt: a.bin\n  dep: b.bin\n\n- tgt: b.bin\n");
        let analyzer = BuildFileAnalyzer::new();
        let item = analyzer
            .prepare_call_hierarchy(
                &d,
                Position {
                    line: 0,
                    character: 9
                }
            )
            .expect("expected an item on the tgt: field");
        assert_eq!(item.name, "a.bin");
    }

    #[test]
    fn outgoing_and_incoming_agree_on_a_simple_dependency() {
        let d = doc("- tgt: a.bin\n  dep: b.bin\n\n- tgt: b.bin\n");
        let analyzer = BuildFileAnalyzer::new();

        let out = analyzer.outgoing_call_targets_in(&d, "a.bin");
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].0, "b.bin");

        let inc = analyzer.incoming_calls_in(&d, "b.bin");
        assert_eq!(inc.len(), 1, "{inc:?}");
        assert_eq!(inc[0].0, "a.bin");
    }

    #[test]
    fn multiline_dep_list_form_is_scanned() {
        let d =
            doc("- tgt: a.bin\n  dep:\n    - b.bin\n    - c.bin\n\n- tgt: b.bin\n\n- tgt: c.bin\n");
        let analyzer = BuildFileAnalyzer::new();
        let out = analyzer.outgoing_call_targets_in(&d, "a.bin");
        let names: Vec<&str> = out.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["b.bin", "c.bin"], "{out:?}");
    }

    #[test]
    fn a_dep_field_written_before_tgt_is_still_found() {
        let d = doc("- dep: b.bin\n  tgt: a.bin\n\n- tgt: b.bin\n");
        let analyzer = BuildFileAnalyzer::new();
        let out = analyzer.outgoing_call_targets_in(&d, "a.bin");
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].0, "b.bin");
    }

    #[test]
    fn repeated_deps_on_the_same_target_collapse_into_one_entry() {
        let d = doc("- tgt: a.bin\n  dep: b.bin b.bin\n\n- tgt: b.bin\n");
        let analyzer = BuildFileAnalyzer::new();
        let out = analyzer.outgoing_call_targets_in(&d, "a.bin");
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].0, "b.bin");
        assert_eq!(out[0].1.len(), 2);
    }

    #[test]
    fn a_target_with_no_dependents_yields_no_incoming_calls() {
        let d = doc("- tgt: a.bin\n\n- tgt: b.bin\n  dep: a.bin\n");
        let analyzer = BuildFileAnalyzer::new();
        assert!(analyzer.incoming_calls_in(&d, "b.bin").is_empty());
    }

    #[test]
    fn a_multi_target_rule_gives_each_target_its_own_distinct_identity() {
        let d = doc("- tgt: a.bin b.bin\n  dep: c.bin\n\n- tgt: c.bin\n");
        let analyzer = BuildFileAnalyzer::new();
        let out_a = analyzer.outgoing_call_targets_in(&d, "a.bin");
        let out_b = analyzer.outgoing_call_targets_in(&d, "b.bin");
        assert_eq!(out_a.len(), 1, "{out_a:?}");
        assert_eq!(out_b.len(), 1, "{out_b:?}");
        assert_eq!(out_a[0].0, "c.bin");
        assert_eq!(out_b[0].0, "c.bin");

        let item_a = analyzer
            .call_hierarchy_item_for_target(&d, "a.bin")
            .unwrap();
        let item_b = analyzer
            .call_hierarchy_item_for_target(&d, "b.bin")
            .unwrap();
        assert_ne!(item_a.selection_range, item_b.selection_range);
    }

    #[test]
    fn a_for_loop_generated_target_and_dependency_are_visible() {
        // The whole reason target scanning works on Jinja-*expanded* text:
        // raw-text-only scanning would never see `file0.bin`/`file1.bin`/
        // `file2.bin` at all, only the literal `file{{i}}.bin` template.
        let d = doc(
            "{% for i in range(3) %}\n- tgt: file{{i}}.bin\n  dep: common.asm\n{% endfor %}\n\n- tgt: common.asm\n"
        );
        let analyzer = BuildFileAnalyzer::new();

        let out = analyzer.outgoing_call_targets_in(&d, "file1.bin");
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].0, "common.asm");

        let inc = analyzer.incoming_calls_in(&d, "common.asm");
        let names: Vec<&str> = inc.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec!["file0.bin", "file1.bin", "file2.bin"],
            "{inc:?}"
        );
    }

    #[test]
    fn a_templated_dependency_line_falls_back_to_a_whole_line_range() {
        // `dep: {{B}}` itself contains `{{`/`}}` in the *original* source,
        // so the expanded-text column for the substituted `b.bin` can't be
        // trusted - the whole original line is highlighted instead (same
        // fallback `validate_build_structure` uses).
        let d = doc("{% set B = \"b.bin\" %}\n- tgt: a.bin\n  dep: {{B}}\n\n- tgt: b.bin\n");
        let analyzer = BuildFileAnalyzer::new();
        let out = analyzer.outgoing_call_targets_in(&d, "a.bin");
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].0, "b.bin");
        let range = out[0].1[0];
        assert_eq!(range.start.character, 0);
        assert_eq!(range.start.line, range.end.line);
        assert!(range.end.character > 0);
    }

    // ─── Jinja macros ───────────────────────────────────────────────────

    fn macro_fixture() -> Document {
        doc(concat!(
            "{% macro emu_launch_sna(sna) -%}\n",
            "emu --snapshot {{sna}}\n",
            "{%- endmacro %}\n",
            "\n",
            "- tgt: testlink\n",
            "  dep: link.sna\n",
            "  cmd: {{emu_launch_sna(\"link.sna\")}}\n"
        ))
    }

    /// Regression test for treating `position.character` (UTF-16 code
    /// units) as a raw byte offset: a supplementary-plane character (😀, 4
    /// UTF-8 bytes / 2 UTF-16 units) earlier on the line must not desync
    /// the two for `prepare_call_hierarchy`'s macro-call detection.
    #[test]
    fn prepare_call_hierarchy_handles_utf16_columns_with_a_supplementary_plane_char_before_it() {
        let d = doc(concat!(
            "{% macro assemble(src) %}basm {{ src }}{% endmacro %}\n",
            "- cmd: \u{1F600}{{ assemble(\"main.asm\") }}\n"
        ));
        let analyzer = BuildFileAnalyzer::new();
        // UTF-16 column 15 on line 1 lands 3 chars into "assemble" once
        // correctly converted to a byte offset.
        let item = analyzer
            .prepare_call_hierarchy(
                &d,
                Position {
                    line: 1,
                    character: 15
                }
            )
            .expect("expected an item at the macro call site");
        assert_eq!(item.name, "assemble");
    }

    #[test]
    fn prepare_call_hierarchy_on_a_macro_call_site_resolves_the_definition() {
        let d = macro_fixture();
        let analyzer = BuildFileAnalyzer::new();
        let line = d.line(6).unwrap();
        let col = line.find("emu_launch_sna(\"link").unwrap() as u32 + 2;
        let item = analyzer
            .prepare_call_hierarchy(
                &d,
                Position {
                    line: 6,
                    character: col
                }
            )
            .expect("expected an item at the macro call site");
        assert_eq!(item.name, "emu_launch_sna");
    }

    #[test]
    fn a_call_from_a_cmd_field_is_attributed_to_its_enclosing_rule() {
        let d = macro_fixture();
        let analyzer = BuildFileAnalyzer::new();
        // The one real call site is in `testlink`'s `cmd:` field, not
        // inside another macro's body - calling a macro directly from a
        // rule is the normal, load-bearing pattern (confirmed against a
        // real project), so the enclosing *rule* is reported as the caller
        // rather than being dropped for "no enclosing macro" the way basm
        // drops a call with no enclosing label.
        let incoming = analyzer.incoming_calls_for_macro_in(&d, "emu_launch_sna");
        assert_eq!(incoming.len(), 1, "{incoming:?}");
        assert_eq!(incoming[0].from.name, "testlink");

        // The `{% macro NAME( %}` definition's own parameter-list paren
        // must never be mistaken for a call to itself: the definition's
        // own body has no real calls in it (just plain
        // `emu --snapshot {{sna}}`), so outgoing must be empty.
        assert!(
            analyzer
                .outgoing_calls_for_macro_targets_in(&d, "emu_launch_sna")
                .is_empty()
        );
    }

    #[test]
    fn a_macro_calling_another_from_inside_its_own_body_is_an_outgoing_call() {
        let d = doc(concat!(
            "{% macro inner() -%}\nx\n{%- endmacro %}\n",
            "{% macro outer() -%}\n{{inner()}}\n{%- endmacro %}\n"
        ));
        let analyzer = BuildFileAnalyzer::new();
        let out = analyzer.outgoing_calls_for_macro_targets_in(&d, "outer");
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].0, "inner");

        let inc = analyzer.incoming_calls_for_macro_in(&d, "inner");
        assert_eq!(inc.len(), 1, "{inc:?}");
        assert_eq!(inc[0].from.name, "outer");
    }

    #[test]
    fn a_builtin_jinja_function_call_is_never_treated_as_a_macro_call() {
        let d = doc("- tgt: a.bin\n  cmd: {{basename(\"x/y.bin\")}}\n");
        let analyzer = BuildFileAnalyzer::new();
        let line = d.line(1).unwrap();
        let col = line.find("basename(").unwrap() as u32 + 2;
        assert!(
            analyzer
                .prepare_call_hierarchy(
                    &d,
                    Position {
                        line: 1,
                        character: col
                    }
                )
                .is_none()
        );
    }
}

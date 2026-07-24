//! Jinja-template support shared by the bndbuild feature modules: cursor
//! context detection (inside `{{ }}` / `{% %}` or not) and `{% set %}`
//! variable definitions.

use tower_lsp::lsp_types::*;

use crate::common::document::Document;

/// Where the cursor sits relative to Jinja delimiters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum JinjaContext {
    /// Inside `{{ ... }}` — an expression.
    Expression,
    /// Inside `{% ... %}` — a statement.
    Statement
}

/// Returns the Jinja context when the cursor at `col` is inside an *open*
/// Jinja delimiter on `line` (the opener appears before the cursor with no
/// matching closer in between).
pub(super) fn jinja_context_at(line: &str, col: usize) -> Option<JinjaContext> {
    let chars: Vec<char> = line.chars().collect();
    let col = col.min(chars.len());
    let before: String = chars[..col].iter().collect();

    let last_expr_open = before.rfind("{{");
    let last_stmt_open = before.rfind("{%");

    match (last_expr_open, last_stmt_open) {
        (None, None) => None,
        (expr, stmt) => {
            if expr > stmt {
                let open = expr.unwrap();
                let closed = before[open + 2..].contains("}}");
                (!closed).then_some(JinjaContext::Expression)
            }
            else {
                let open = stmt.unwrap();
                let closed = before[open + 2..].contains("%}");
                (!closed).then_some(JinjaContext::Statement)
            }
        },
    }
}

/// The identifier (alphanumeric + `_`, byte-based) at byte column `col` on
/// `line`, as `(word, start, end)` — shared by `definition.rs::jinja_word_at`
/// and `macro_expand.rs::macro_call_at`, which independently reimplemented
/// this exact scan. Does **not** gate on `jinja_context_at` itself (both of
/// those callers already do that check with their own semantics around it);
/// this is purely the character-boundary walk.
pub(super) fn identifier_at(line: &str, col: usize) -> Option<(String, usize, usize)> {
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
    if start < end {
        Some((line[start..end].to_string(), start, end))
    }
    else {
        None
    }
}

/// All Jinja statement keywords worth completing inside `{% ... %}`.
pub(super) const JINJA_STATEMENT_KEYWORDS: &[(&str, &str)] = &[
    ("if", "Conditional block"),
    ("elif", "Alternative condition"),
    ("else", "Fallback branch"),
    ("endif", "Close an if block"),
    ("for", "Loop block"),
    ("endfor", "Close a for loop"),
    ("set", "Define a variable"),
    ("in", "Loop source"),
    ("include", "Include another template"),
    ("macro", "Define a macro"),
    ("endmacro", "Close a macro"),
    ("import", "Import macros")
];

/// Collect the variables defined with `{% set NAME = ... %}` (or `{%- set`),
/// with the value expression's source text and the location of each
/// definition. First definition wins. The value is the raw text of the
/// expression as written (e.g. `"src"` or `["a", "b"]`), not evaluated.
pub(crate) fn collect_jinja_variables(document: &Document) -> Vec<(String, String, Location)> {
    let text = document.text();
    let mut vars: Vec<(String, String, Location)> = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        let mut search_from = 0usize;
        while let Some(rel) = line[search_from..].find("{%") {
            let stmt_start = search_from + rel + 2;
            let inner = &line[stmt_start..];
            let inner = inner.strip_prefix('-').unwrap_or(inner);
            let inner_trimmed = inner.trim_start();
            if let Some(rest) = inner_trimmed.strip_prefix("set")
                && rest.starts_with(char::is_whitespace)
            {
                let rest_trimmed = rest.trim_start();
                let name: String = rest_trimmed
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() && !vars.iter().any(|(n, ..)| *n == name) {
                    let name_col = line.len() - rest_trimmed.len();

                    // Value expression: after the name, skip whitespace,
                    // expect `=`, then take everything up to the closing
                    // `%}`/`-%}`, trimmed. Left empty when the statement
                    // doesn't parse as a simple `name = expr` assignment.
                    let value = rest_trimmed[name.len()..]
                        .trim_start()
                        .strip_prefix('=')
                        .map(|v| {
                            let v = v.trim();
                            let v = v.strip_suffix("-%}").map(str::trim_end).unwrap_or(v);
                            let v = v.strip_suffix("%}").map(str::trim_end).unwrap_or(v);
                            v.to_string()
                        })
                        .unwrap_or_default();

                    vars.push((
                        name.clone(),
                        value,
                        Location {
                            uri: document.uri.clone(),
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: name_col as u32
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: (name_col + name.len()) as u32
                                }
                            }
                        }
                    ));
                }
            }
            search_from = stmt_start;
        }
    }
    vars
}

/// Every `{% include "PATH" %}` (or `{%- include`) target path in `text`,
/// raw as written (relative, to be resolved against the containing file's
/// own directory). `{% import %}` isn't handled — not used anywhere in the
/// real project this was designed against, and its namespaced references
/// would need different reference-matching than a plain `{% include %}`'s
/// shared scope.
pub(crate) fn extract_jinja_include_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in text.lines() {
        let mut search_from = 0usize;
        while let Some(rel) = line[search_from..].find("{%") {
            let stmt_start = search_from + rel + 2;
            let inner = &line[stmt_start..];
            let inner = inner.strip_prefix('-').unwrap_or(inner);
            let inner_trimmed = inner.trim_start();
            if let Some(rest) = inner_trimmed.strip_prefix("include")
                && rest.starts_with(char::is_whitespace)
                && let Some(q1) = rest.find('"')
                && let Some(q2_rel) = rest[q1 + 1..].find('"')
            {
                paths.push(rest[q1 + 1..q1 + 1 + q2_rel].to_string());
            }
            search_from = stmt_start;
        }
    }
    paths
}

/// The names of every `{% macro NAME(...) %}` defined in `document` — used
/// to tell a genuine user-defined macro call apart from a call to a
/// built-in Jinja function (`fail(...)`, `basename(...)`, ...), which hover
/// expansion deliberately doesn't handle.
pub(super) fn macro_definition_names(document: &Document) -> Vec<String> {
    let text = document.text();
    let mut names = Vec::new();
    for line in text.lines() {
        for (name, ..) in super::call_hierarchy::macro_definitions_in_line(line) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// All variables a bndbuild template could reference: locally `{% set %}`
/// variables (paired with their source-text value) plus the built-in globals
/// injected by `create_template_env`, for completion. Local variables shadow
/// a built-in of the same name.
pub(super) fn known_variables(document: &Document) -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = collect_jinja_variables(document)
        .into_iter()
        .map(|(name, value, _)| {
            let detail = if value.is_empty() {
                "Jinja variable ({% set %})".to_string()
            }
            else {
                format!("{{% set %}} variable = {value}")
            };
            (name, detail)
        })
        .collect();

    for global in cpclib_bndbuild::lsp::BUILTIN_JINJA_GLOBALS {
        if !vars.iter().any(|(n, _)| n == global.name) {
            vars.push((global.name.to_string(), global.description.to_string()));
        }
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_inside_statement_braces() {
        let line = "{% if X ";
        assert_eq!(
            jinja_context_at(line, line.len()),
            Some(JinjaContext::Statement)
        );
    }

    #[test]
    fn detects_inside_expression_braces() {
        let line = "  cmd: basm {{ma";
        assert_eq!(
            jinja_context_at(line, line.len()),
            Some(JinjaContext::Expression)
        );
    }

    #[test]
    fn closed_braces_are_outside() {
        let line = "  cmd: basm {{main}} -o out";
        assert_eq!(jinja_context_at(line, line.len()), None);
    }

    #[test]
    fn collects_set_variables() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text = "{% set root = \"src\" %}\n{%- set count = 3 -%}\n- tgt: {{root}}/x\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let vars = collect_jinja_variables(&doc);
        let names: Vec<&str> = vars.iter().map(|(n, ..)| n.as_str()).collect();
        assert_eq!(names, vec!["root", "count"]);
        assert_eq!(vars[0].2.range.start.line, 0);
        assert_eq!(vars[1].2.range.start.line, 1);
    }

    #[test]
    fn captures_the_value_expression_source_text() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text =
            "{% set root = \"src\" %}\n{%- set count = 3 -%}\n{% set files = [\"a\", \"b\"] %}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let vars = collect_jinja_variables(&doc);
        let values: Vec<(&str, &str)> = vars
            .iter()
            .map(|(n, v, _)| (n.as_str(), v.as_str()))
            .collect();
        assert_eq!(
            values,
            vec![
                ("root", "\"src\""),
                ("count", "3"),
                ("files", "[\"a\", \"b\"]")
            ]
        );
    }

    #[test]
    fn known_variables_includes_builtin_globals() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text = "{% set root = \"src\" %}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let names: Vec<String> = known_variables(&doc).into_iter().map(|(n, _)| n).collect();
        assert!(names.contains(&"root".to_string()));
        assert!(names.contains(&"AKG_PLAYER_PATH".to_string()));
    }

    #[test]
    fn macro_definition_names_finds_a_defined_macro() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text = "{% macro assemble(src) %}\nbasm {{ src }}\n{% endmacro %}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        assert_eq!(macro_definition_names(&doc), vec!["assemble".to_string()]);
    }

    #[test]
    fn macro_definition_names_ignores_other_statements() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text = "{% set root = \"src\" %}\n{% if root %}{% endif %}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        assert!(macro_definition_names(&doc).is_empty());
    }

    /// Regression test for the dedup with `call_hierarchy.rs::macro_definitions_in_line`:
    /// both must agree on the same input.
    #[test]
    fn macro_definition_names_agrees_with_macro_definitions_in_line() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text = "{% macro assemble(src) %}\nbasm {{ src }}\n{% endmacro %}\n{%- macro link(a, b) %}\n{% endmacro %}\n";
        let doc = Document::new(uri, text.to_string(), 1);

        let from_names_only = macro_definition_names(&doc);
        let from_positions: Vec<String> = text
            .lines()
            .flat_map(|line| {
                super::super::call_hierarchy::macro_definitions_in_line(line)
                    .into_iter()
                    .map(|(name, ..)| name)
            })
            .collect();

        assert_eq!(from_names_only, from_positions);
        assert_eq!(
            from_names_only,
            vec!["assemble".to_string(), "link".to_string()]
        );
    }
}

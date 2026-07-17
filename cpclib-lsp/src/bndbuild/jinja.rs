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
/// with the location of each definition. First definition wins.
pub(super) fn collect_jinja_variables(document: &Document) -> Vec<(String, Location)> {
    let text = document.text();
    let mut vars: Vec<(String, Location)> = Vec::new();

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
                if !name.is_empty() && !vars.iter().any(|(n, _)| *n == name) {
                    let name_col = line.len() - rest_trimmed.len();
                    vars.push((
                        name.clone(),
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
        let names: Vec<&str> = vars.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["root", "count"]);
        assert_eq!(vars[0].1.range.start.line, 0);
        assert_eq!(vars[1].1.range.start.line, 1);
    }
}

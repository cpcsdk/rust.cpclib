//! Hover preview for a Jinja macro *call* (e.g. `{{ assemble(src) }}` where
//! `assemble` was defined via `{% macro assemble(src) %}...{% endmacro %}`):
//! render the whole file once (so every `{% set %}` and macro definition is
//! in scope, in document order — the same as a real build) via minijinja's
//! own `Template::render_captured`, then call *that* macro directly with
//! the call site's own arguments via `State::call_macro` — minijinja's own
//! primitive for exactly this, not a reimplementation.
//!
//! Reuses `sourcemap::build_environment` — the same lenient, LSP-safe
//! environment `diagnostics.rs`/`symbols.rs` already render with — so a
//! file mid-edit (or missing `-D` definitions) still renders as far as
//! possible instead of erroring immediately, consistent with the rest of
//! this module.

use minijinja::Value;
use tower_lsp::lsp_types::*;

use crate::common::document::Document;

/// If the cursor sits on a call to a `{% macro %}` defined in `document`,
/// render just that call and return markdown showing the expansion (or a
/// graceful explanation of why it couldn't be rendered).
pub(super) fn macro_call_hover(document: &Document, line: &str, col: usize) -> Option<String> {
    let (name, args_text) = macro_call_at(line, col)?;
    if !super::jinja::macro_definition_names(document)
        .iter()
        .any(|n| n == &name)
    {
        // Not a user-defined macro (e.g. a call to a built-in Jinja
        // function like `fail(...)`/`basename(...)`) — nothing to expand.
        return None;
    }

    let file_dir = document
        .uri
        .to_file_path()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let env = super::sourcemap::build_environment(file_dir.as_deref());
    let text = document.text();

    let header = format!("**{name}({args_text})**");

    let tmpl = match env.template_from_str(&text) {
        Ok(t) => t,
        Err(e) => return Some(render_error(&header, "render this file", &e))
    };
    let captured = match tmpl.render_captured(minijinja::context!()) {
        Ok(c) => c,
        Err(e) => return Some(render_error(&header, "render this file", &e))
    };
    let state = captured.state();

    // Every `{% set %}` variable and global currently in scope, so an
    // argument expression referencing one (e.g. `assemble(root)`) resolves.
    let mut ctx: std::collections::BTreeMap<String, Value> = std::collections::BTreeMap::new();
    for var_name in state.known_variables() {
        if let Some(v) = state.lookup(&var_name) {
            ctx.insert(var_name.into_owned(), v);
        }
    }

    let mut arg_values = Vec::new();
    for arg in split_top_level_args(&args_text) {
        let value = match env
            .compile_expression(&arg)
            .and_then(|expr| expr.eval(&ctx))
        {
            Ok(v) => v,
            Err(e) => {
                return Some(render_error(
                    &header,
                    &format!("evaluate argument `{arg}`"),
                    &e
                ));
            }
        };
        arg_values.push(value);
    }

    match state.call_macro(&name, &arg_values) {
        Ok(rendered) => {
            Some(format!(
                "{header} expands to:\n\n```\n{}\n```",
                rendered.trim_end()
            ))
        },
        Err(e) => Some(render_error(&header, "expand this macro call", &e))
    }
}

fn render_error(header: &str, action: &str, err: &minijinja::Error) -> String {
    format!("{header} — could not {action}:\n\n```\n{err:#}\n```")
}

/// If `col` sits on an identifier immediately (optionally after whitespace)
/// followed by `(...)`, inside `{{ }}`/`{% %}`, return its name and the raw
/// text between the parens (single-line only, matching `jinja_context_at`).
fn macro_call_at(line: &str, col: usize) -> Option<(String, String)> {
    super::jinja::jinja_context_at(line, col)?;

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
    let args_start = i;
    let mut depth = 1i32;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            i += 1;
        }
    }
    if depth != 0 {
        return None; // unterminated call on this line
    }
    Some((name.to_string(), line[args_start..i].to_string()))
}

/// Split a call's argument text on top-level commas (respecting nested
/// `()`/`[]`/`{}` and quoted strings), trimmed. Empty input yields no args.
fn split_top_level_args(text: &str) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if let Some(q) = in_str {
            if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                args.push(text[start..i].trim().to_string());
                start = i + 1;
            },
            _ => {}
        }
        i += 1;
    }
    args.push(text[start..].trim().to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_top_level_args_handles_nested_calls_and_strings() {
        let args = split_top_level_args("a, foo(1, 2), \"x, y\"");
        assert_eq!(args, vec!["a", "foo(1, 2)", "\"x, y\""]);
    }

    #[test]
    fn split_top_level_args_of_empty_text_is_empty() {
        assert!(split_top_level_args("").is_empty());
        assert!(split_top_level_args("   ").is_empty());
    }

    #[test]
    fn macro_call_at_extracts_name_and_raw_args() {
        let line = "  cmd: {{ assemble(src, \"out.bin\") }}";
        // Cursor inside "assemble".
        let col = line.find("assemble").unwrap() + 3;
        let (name, args) = macro_call_at(line, col).unwrap();
        assert_eq!(name, "assemble");
        assert_eq!(args, "src, \"out.bin\"");
    }

    #[test]
    fn macro_call_at_outside_jinja_braces_is_none() {
        let line = "  cmd: assemble(src)";
        let col = line.find("assemble").unwrap() + 3;
        assert!(macro_call_at(line, col).is_none());
    }

    #[test]
    fn macro_call_hover_renders_the_expansion() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text = "{% macro assemble(src) %}basm {{ src }}{% endmacro %}\n\
                     - cmd: {{ assemble(\"main.asm\") }}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let line = "- cmd: {{ assemble(\"main.asm\") }}";
        let col = line.find("assemble").unwrap() + 3;
        let hover = macro_call_hover(&doc, line, col).expect("expected an expansion hover");
        assert!(hover.contains("basm main.asm"), "{hover}");
    }

    #[test]
    fn macro_call_hover_on_a_builtin_function_call_is_none() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        let text = "- cmd: {{ basename(\"a/b.asm\") }}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let line = "- cmd: {{ basename(\"a/b.asm\") }}";
        let col = line.find("basename").unwrap() + 3;
        assert!(macro_call_hover(&doc, line, col).is_none());
    }

    #[test]
    fn macro_call_hover_reports_an_evaluation_error_gracefully() {
        let uri = Url::parse("file:///b.bnd").unwrap();
        // `undefined_var` referenced in the macro's own body triggers a
        // real minijinja error under Chainable-but-still-erroring cases
        // (attribute access) — here we force a parse error in the
        // *argument* expression itself, which is simpler to trigger
        // deterministically: an unterminated string literal.
        let text = "{% macro assemble(src) %}basm {{ src }}{% endmacro %}\n\
                     - cmd: {{ assemble(\"unterminated) }}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let line = "- cmd: {{ assemble(\"unterminated) }}";
        let col = line.find("assemble").unwrap() + 3;
        let hover = macro_call_hover(&doc, line, col).expect("expected an error hover");
        assert!(hover.contains("could not"), "{hover}");
    }
}

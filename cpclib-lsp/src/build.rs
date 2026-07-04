use std::collections::HashSet;
use std::sync::LazyLock;

use tower_lsp::lsp_types::*;

use crate::document::Document;

// Reuse the same token type indices as asm.rs (same SemanticTokensLegend)
const TT_KEYWORD: u32 = 0;
const TT_VARIABLE: u32 = 4;
const TT_NUMBER: u32 = 5;
const TT_STRING: u32 = 6;
const TT_COMMENT: u32 = 7;
const TT_OPERATOR: u32 = 8;
const TT_ENUM_MEMBER: u32 = 9;
const MOD_DECLARATION: u32 = 1 << 0;

static RULE_KEYS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    cpclib_bndbuild::lsp::RULE_KEYS
        .iter()
        .flat_map(|k| k.names.iter().copied())
        .collect()
});

#[derive(PartialEq)]
enum Collecting {
    Nothing,
    Target(u32), // original line of the tgt key
    Help
}

/// Analyzer for build files (YAML with Jinja templates)
pub struct BuildFileAnalyzer {}

impl BuildFileAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze the build file and return diagnostics
    pub fn analyze(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let text = document.text();

        // Try to parse as YAML
        match serde_yaml::from_str::<serde_yaml::Value>(&text) {
            Ok(_) => {
                // Valid YAML - now check for build file structure
                diagnostics.extend(self.validate_build_structure(document));
            },
            Err(e) => {
                // Check if it contains Jinja templates (which would cause YAML parse errors)
                if text.contains("{{") || text.contains("{%") {
                    // This is likely a Jinja template - provide a warning
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: 0,
                                character: 0
                            },
                            end: Position {
                                line: 0,
                                character: 0
                            }
                        },
                        severity: Some(DiagnosticSeverity::INFORMATION),
                        code: None,
                        code_description: None,
                        source: Some("bndbuild".to_string()),
                        message: "File contains Jinja templates. YAML validation is limited."
                            .to_string(),
                        related_information: None,
                        tags: None,
                        data: None
                    });
                }
                else {
                    // Real YAML error
                    let line = e.location().map(|l| l.line()).unwrap_or(0);
                    let column = e.location().map(|l| l.column()).unwrap_or(0);

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line.saturating_sub(1) as u32,
                                character: column as u32
                            },
                            end: Position {
                                line: line.saturating_sub(1) as u32,
                                character: (column + 10) as u32
                            }
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("yaml".to_string()),
                        message: format!("YAML parse error: {}", e),
                        related_information: None,
                        tags: None,
                        data: None
                    });
                }
            }
        }

        diagnostics
    }

    fn validate_build_structure(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let text = document.text();

        // Check for common build file keys
        let has_targets = text.contains("targets:");
        let has_tasks = text.contains("tasks:");

        if !has_targets && !has_tasks {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0
                    },
                    end: Position {
                        line: 0,
                        character: 0
                    }
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("bndbuild".to_string()),
                message: "Build file should contain 'targets:' or 'tasks:' section".to_string(),
                ..Default::default()
            });
        }

        // ── Collect declared target names ──────────────────────────────────
        let tgt_keys: &[&str] = &["targets", "tgt", "target", "build"];
        let dep_keys: &[&str] = &["dependencies", "dep", "dependency", "requires"];

        let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in text.lines() {
            let trimmed = line.trim_start();
            let content = if trimmed.starts_with("- ") {
                trimmed[2..].trim_start()
            }
            else {
                trimmed
            };
            for &key in tgt_keys {
                let prefix = format!("{}:", key);
                if let Some(rest) = content.strip_prefix(prefix.as_str()) {
                    let value = rest.split('#').next().unwrap_or("").trim();
                    // Skip Jinja expressions and block-scalar indicators
                    if !value.starts_with('>') && !value.starts_with('|') {
                        for tok in value.split_whitespace() {
                            if !tok.contains("{{") && !tok.contains("{%") {
                                declared.insert(tok.to_string());
                            }
                        }
                    }
                }
            }
        }

        // ── Resolve base directory for file-existence checks ───────────────
        let base_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));

        // ── Check each dependency value ────────────────────────────────────
        for (line_num, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            let content = if trimmed.starts_with("- ") {
                trimmed[2..].trim_start()
            }
            else {
                trimmed
            };

            for &key in dep_keys {
                let prefix = format!("{}:", key);
                if let Some(rest) = content.strip_prefix(prefix.as_str()) {
                    let value = rest.split('#').next().unwrap_or("").trim();
                    if value.starts_with('>') || value.starts_with('|') {
                        break;
                    }

                    let mut col_offset = line.len() - content.len() + prefix.len();
                    // skip spaces after colon
                    let value_bytes = value.as_bytes();
                    let mut vi = 0;
                    while vi < value_bytes.len() && value_bytes[vi] == b' ' {
                        vi += 1;
                        col_offset += 1;
                    }

                    for tok in value.split_whitespace() {
                        // Skip Jinja expressions
                        if tok.contains("{{") || tok.contains("{%") {
                            col_offset += tok.len() + 1;
                            continue;
                        }

                        let tok_start = col_offset;
                        let tok_end = tok_start + tok.len();

                        if !declared.contains(tok) {
                            // Also skip if the file exists on disk
                            let file_exists = base_dir
                                .as_ref()
                                .map(|d| d.join(tok).exists())
                                .unwrap_or(false);

                            if !file_exists {
                                diagnostics.push(Diagnostic {
                                    range: Range {
                                        start: Position { line: line_num as u32, character: tok_start as u32 },
                                        end:   Position { line: line_num as u32, character: tok_end   as u32 },
                                    },
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    source:   Some("bndbuild".to_string()),
                                    message:  format!("Undefined dependency '{}': not a declared target or existing file", tok),
                                    ..Default::default()
                                });
                            }
                        }

                        col_offset += tok.len() + 1;
                    }
                    break; // matched this dep_key, no need to check others
                }
            }
        }

        diagnostics
    }

    /// Provide hover information for build file keywords
    pub fn hover(&self, document: &Document, position: Position) -> Option<Hover> {
        let line_idx = position.line as usize;
        let line = document.line(line_idx)?;

        // Extract word at cursor
        let word = self.extract_word_at_position(&line, position.character as usize)?;

        // Check if it's a build file keyword
        if let Some(description) = self.get_keyword_help(&word) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: description
                }),
                range: None
            });
        }

        // Check if it's a task type
        if let Some(description) = self.get_task_type_help(&word) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: description
                }),
                range: None
            });
        }

        None
    }

    /// Provide completion suggestions for build files
    pub fn completion(&self, document: &Document, position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        let line_idx = position.line as usize;
        if let Some(line) = document.line(line_idx) {
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();

            // If we're at the start of a line or after whitespace, suggest top-level keys
            if indent == 0 || line.trim().is_empty() {
                completions.extend(self.get_top_level_completions());
            }
            else if line.trim_start().starts_with("- ") {
                // Inside a list - suggest task types
                completions.extend(self.get_task_completions());
            }
        }

        // Add Jinja template completions
        completions.extend(self.get_jinja_completions());

        completions
    }

    /// Semantic tokens for bndbuild (YAML + Jinja2) files.
    pub fn semantic_tokens(&self, document: &Document) -> Vec<SemanticToken> {
        let mut raw: Vec<(u32, u32, u32, u32, u32)> = Vec::new();

        let line_count = document.rope.len_lines();
        for line_num in 0..line_count {
            let line_str = match document.line(line_num) {
                Some(s) => s,
                None => continue
            };
            // Strip trailing newline for length accounting
            let line_str = line_str.trim_end_matches(['\n', '\r']);
            let bytes = line_str.as_bytes();
            let len = bytes.len();
            let line_u = line_num as u32;
            let mut col = 0usize;

            // Skip leading whitespace
            while col < len && matches!(bytes[col], b' ' | b'\t') {
                col += 1;
            }
            if col >= len {
                continue;
            }

            // Full-line comment
            if bytes[col] == b'#' {
                raw.push((line_u, col as u32, (len - col) as u32, TT_COMMENT, 0));
                continue;
            }

            // YAML list item marker `- `
            if bytes[col] == b'-' && (col + 1 >= len || matches!(bytes[col + 1], b' ' | b'\t')) {
                raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                col += 1;
                while col < len && matches!(bytes[col], b' ' | b'\t') {
                    col += 1;
                }
            }

            while col < len {
                // Inline YAML comment — but only if not inside a Jinja construct.
                // A bare `#` that follows `{` or `%` is handled by the Jinja branches below.
                if bytes[col] == b'#' && (col == 0 || !matches!(bytes[col - 1], b'{' | b'%')) {
                    raw.push((line_u, col as u32, (len - col) as u32, TT_COMMENT, 0));
                    break;
                }

                // Jinja {# comment #} — skip entirely; TM grammar handles it.
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'#' {
                    col += 2;
                    while col + 1 < len && !(bytes[col] == b'#' && bytes[col + 1] == b'}') {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    continue;
                }

                // Jinja {{ expression }} — skip; TM grammar colors the internals.
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'{' {
                    col += 2;
                    while col + 1 < len && !(bytes[col] == b'}' && bytes[col + 1] == b'}') {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    continue;
                }

                // Jinja {% statement %} — skip; TM grammar colors keywords inside.
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'%' {
                    col += 2;
                    while col + 1 < len && !(bytes[col] == b'%' && bytes[col + 1] == b'}') {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    continue;
                }

                // Double-quoted string
                if bytes[col] == b'"' {
                    let start = col;
                    col += 1;
                    while col < len {
                        if bytes[col] == b'"' && (col == start + 1 || bytes[col - 1] != b'\\') {
                            col += 1;
                            break;
                        }
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    continue;
                }

                // Single-quoted string
                if bytes[col] == b'\'' {
                    let start = col;
                    col += 1;
                    while col < len && bytes[col] != b'\'' {
                        col += 1;
                    }
                    if col < len {
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_STRING, 0));
                    continue;
                }

                // Identifier / keyword / YAML key
                if bytes[col].is_ascii_alphabetic() || bytes[col] == b'_' {
                    let start = col;
                    while col < len
                        && (bytes[col].is_ascii_alphanumeric() || matches!(bytes[col], b'_' | b'-'))
                    {
                        col += 1;
                    }
                    let word = &line_str[start..col];

                    // YAML key: word followed by ':'
                    if col < len && bytes[col] == b':' && (col + 1 >= len || bytes[col + 1] != b':')
                    {
                        let mods = if RULE_KEYS.contains(word) {
                            MOD_DECLARATION
                        }
                        else {
                            0
                        };
                        raw.push((
                            line_u,
                            start as u32,
                            (col - start) as u32,
                            TT_ENUM_MEMBER,
                            mods
                        ));
                        raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                        col += 1;
                        continue;
                    }

                    // Boolean / null
                    if matches!(
                        word,
                        "true"
                            | "false"
                            | "yes"
                            | "no"
                            | "True"
                            | "False"
                            | "Yes"
                            | "No"
                            | "null"
                            | "Null"
                    ) {
                        raw.push((line_u, start as u32, (col - start) as u32, TT_KEYWORD, 0));
                    }
                    // (other identifiers emitted with no token — they're uncoloured)
                    continue;
                }

                // Numbers in bndbuild are almost always part of filenames, version strings,
                // or build arguments — don't color them to avoid visual noise.
                if bytes[col].is_ascii_digit() {
                    while col < len && (bytes[col].is_ascii_alphanumeric() || bytes[col] == b'.') {
                        col += 1;
                    }
                    continue;
                }

                col += 1;
            }
        }

        // Delta-encode for LSP protocol
        let mut result = Vec::with_capacity(raw.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        for (line, start, length, tok_type, modifiers) in raw {
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                start - prev_start
            }
            else {
                start
            };
            result.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: tok_type,
                token_modifiers_bitset: modifiers
            });
            prev_line = line;
            prev_start = start;
        }
        result
    }

    /// Emit a CodeLens "▶ Run" button on each target declared in a bndbuild file.
    /// Delegates target detection to `document_symbols` so that Jinja expansion,
    /// block scalars, and all key aliases are handled consistently.
    pub fn code_lens(&self, document: &Document) -> Vec<CodeLens> {
        let file_path = document
            .uri
            .to_file_path()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        self.document_symbols(document)
            .into_iter()
            .map(|sym| {
                CodeLens {
                    range: sym.selection_range,
                    command: Some(Command {
                        title: format!("▶ Run: {}", sym.name),
                        command: "cpclib.runRule".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(sym.name),
                            serde_json::json!(file_path),
                        ])
                    }),
                    data: None
                }
            })
            .collect()
    }

    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let line = document.line(position.line as usize)?;
        let col = position.character as usize;
        let bytes = line.as_bytes();
        let len = bytes.len();

        // Scan for every {% include "…" %} / {% include '…' %} on this line
        // and check whether the cursor falls inside a filename string.
        let mut i = 0;
        while i + 1 < len {
            // Find {%
            if bytes[i] != b'{' || bytes[i + 1] != b'%' {
                i += 1;
                continue;
            }
            i += 2;
            if i < len && bytes[i] == b'-' {
                i += 1;
            } // {%-

            // Skip whitespace
            while i < len && bytes[i] == b' ' {
                i += 1;
            }

            // Must be the "include" keyword
            if !line[i..].starts_with("include") {
                // Skip to end of this tag
                while i + 1 < len && !(bytes[i] == b'%' && bytes[i + 1] == b'}') {
                    i += 1;
                }
                continue;
            }
            i += 7; // len("include")

            // Skip whitespace between keyword and filename
            while i < len && bytes[i] == b' ' {
                i += 1;
            }

            // Filename must be a quoted string
            if i >= len || !matches!(bytes[i], b'"' | b'\'') {
                continue;
            }
            let delim = bytes[i];
            i += 1;
            let fname_start = i;
            while i < len && bytes[i] != delim {
                i += 1;
            }
            let fname_end = i;

            // Is the cursor inside [fname_start, fname_end)?
            if col >= fname_start && col < fname_end {
                let filename = &line[fname_start..fname_end];
                let base_dir = document
                    .uri
                    .to_file_path()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))?;
                let target = base_dir.join(filename);
                if target.exists() {
                    let target_uri = Url::from_file_path(target).ok()?;
                    return Some(Location {
                        uri: target_uri,
                        range: Range::default()
                    });
                }
            }

            i = fname_end + 1; // past closing quote
        }

        // ── tgt / dep field navigation ───────────────────────────────────────
        let file_key_names: Vec<&'static str> = cpclib_bndbuild::lsp::RULE_KEYS
            .iter()
            .filter(|k| k.names.contains(&"targets") || k.names.contains(&"dependencies"))
            .flat_map(|k| k.names.iter().copied())
            .collect();

        if let Some(filename) = Self::filename_under_cursor(&line, &file_key_names, col) {
            // Skip Jinja expressions — can't resolve them statically.
            if !filename.contains("{{") && !filename.contains("{%") {
                let tgt_key_names: Vec<&'static str> = cpclib_bndbuild::lsp::RULE_KEYS
                    .iter()
                    .find(|k| k.names.contains(&"targets"))
                    .map(|k| k.names.to_vec())
                    .unwrap_or_default();

                // Priority 1: filename matches a target declared in this file.
                // Use a direct raw-text scan — it's reliable even when Jinja
                // expansion fails — and navigate to that rule, never opening the file.
                let raw = document.text();
                if let Some(tgt_line) = Self::find_target_line(&raw, filename, &tgt_key_names) {
                    return Some(Location {
                        uri: document.uri.clone(),
                        range: Range {
                            start: Position {
                                line: tgt_line,
                                character: 0
                            },
                            end: Position {
                                line: tgt_line,
                                character: 0
                            }
                        }
                    });
                }

                // Priority 2: not a declared target — open the file if it exists.
                if let Some(base_dir) = document
                    .uri
                    .to_file_path()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                {
                    let path = base_dir.join(filename);
                    if path.exists() {
                        if let Ok(uri) = Url::from_file_path(path) {
                            return Some(Location {
                                uri,
                                range: Range::default()
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Return the space-separated token at column `col` in the value part of
    /// `key: value` when `key` is one of `key_names`.  Handles the inline
    /// `- key: value` list-item form and strips trailing YAML comments.
    fn filename_under_cursor<'a>(line: &'a str, key_names: &[&str], col: usize) -> Option<&'a str> {
        let bytes = line.as_bytes();
        let len = bytes.len();

        // Skip leading whitespace and optional `- ` list marker.
        let mut i = 0;
        while i < len && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b' ' {
            i += 2;
            while i < len && bytes[i] == b' ' {
                i += 1;
            }
        }

        for &key in key_names {
            let prefix = format!("{}:", key);
            if !line[i..].starts_with(prefix.as_str()) {
                continue;
            }

            let mut v = i + prefix.len();
            // Skip spaces after colon.
            while v < len && bytes[v] == b' ' {
                v += 1;
            }
            // Skip block-scalar indicators — no inline filename to navigate.
            if v < len && matches!(bytes[v], b'>' | b'|') {
                return None;
            }

            // Walk space-separated tokens until end-of-line or YAML comment.
            while v < len && bytes[v] != b'#' {
                while v < len && bytes[v] == b' ' {
                    v += 1;
                }
                if v >= len || bytes[v] == b'#' {
                    break;
                }
                let tok_start = v;
                while v < len && bytes[v] != b' ' && bytes[v] != b'#' {
                    v += 1;
                }
                let tok_end = v;
                if col >= tok_start && col < tok_end {
                    return Some(&line[tok_start..tok_end]);
                }
            }
            return None; // key matched but cursor not on any token
        }
        None
    }

    /// Scan the raw (unexpanded) text for a rule whose target value contains
    /// `filename`.  Exact matches take priority; Jinja template patterns
    /// (e.g., `HBL.{{hbl.nb}}`) are used as a fallback so that clicking
    /// `HBL.002` in a dep field navigates to the template line.
    fn find_target_line(text: &str, filename: &str, tgt_key_names: &[&str]) -> Option<u32> {
        let mut template_match: Option<u32> = None;

        for (line_num, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            let content = if trimmed.starts_with("- ") {
                trimmed[2..].trim_start()
            }
            else {
                trimmed
            };

            for &key in tgt_key_names {
                let prefix = format!("{}:", key);
                if let Some(rest) = content.strip_prefix(prefix.as_str()) {
                    let value = rest.split('#').next().unwrap_or("").trim();
                    for tok in value.split_whitespace() {
                        if tok == filename {
                            return Some(line_num as u32); // exact match wins immediately
                        }
                        if template_match.is_none()
                            && tok.contains("{{")
                            && Self::matches_jinja_pattern(tok, filename)
                        {
                            template_match = Some(line_num as u32);
                        }
                    }
                }
            }
        }

        template_match
    }

    /// Returns `true` if `filename` could have been generated from `template`
    /// by substituting every `{{ … }}` expression with any string.
    /// E.g., `"HBL.{{hbl.nb}}"` matches `"HBL.002"`.
    fn matches_jinja_pattern(template: &str, filename: &str) -> bool {
        // Split template on {{…}} wildcards into literal segments.
        let mut segments: Vec<&str> = Vec::new();
        let mut rem = template;
        while let Some(open) = rem.find("{{") {
            segments.push(&rem[..open]);
            rem = &rem[open + 2..];
            if let Some(close) = rem.find("}}") {
                rem = &rem[close + 2..];
            }
            else {
                break; // unclosed {{ — stop here
            }
        }
        segments.push(rem);

        // Match segments against filename left-to-right.
        let mut fname = filename;
        for (i, seg) in segments.iter().enumerate() {
            if seg.is_empty() {
                continue; // wildcard: matches anything
            }
            if i == 0 {
                if !fname.starts_with(seg) {
                    return false;
                }
                fname = &fname[seg.len()..];
            }
            else if i == segments.len() - 1 {
                if !fname.ends_with(seg) {
                    return false;
                }
            }
            else {
                match fname.find(seg) {
                    Some(pos) => fname = &fname[pos + seg.len()..],
                    None => return false
                }
            }
        }
        true
    }

    pub fn find_references(&self, _document: &Document, _position: Position) -> Vec<Location> {
        // TODO: Find all references to targets, variables, etc.
        Vec::new()
    }

    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let target_names: Vec<&'static str> = cpclib_bndbuild::lsp::RULE_KEYS
            .iter()
            .find(|k| k.names.contains(&"targets"))
            .map(|k| k.names.to_vec())
            .unwrap_or_default();

        // Try Jinja expansion so loop-generated rules appear in the outline.
        // Fall back to raw text when expansion fails (missing variables, syntax
        // errors, etc.) so the outline still works on template-only edits.
        let file_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let raw_text = document.text();
        let (expanded_text, source_map) =
            match crate::sourcemap::expand_with_source_map(&raw_text, file_dir.as_deref()) {
                Ok((text, map)) => (text, map),
                Err(_) => {
                    let lines = raw_text.lines().count();
                    (
                        raw_text.clone(),
                        crate::sourcemap::SourceMap::identity(lines)
                    )
                }
            };

        self.scan_symbols_from_text(&expanded_text, &source_map, &target_names)
    }

    fn scan_symbols_from_text(
        &self,
        text: &str,
        source_map: &crate::sourcemap::SourceMap,
        target_names: &[&'static str]
    ) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        let mut rule_tgt: Option<(String, u32)> = None;
        let mut rule_help: Option<String> = None;
        let mut rule_start: u32 = 0;
        let mut rule_end: u32 = 0;
        let mut in_rule: bool = false;
        let mut collecting: Collecting = Collecting::Nothing;
        let mut block_base: Option<usize> = None; // indent of first content line
        let mut block_buf: String = String::new();

        // Finalise whatever block scalar was being accumulated and reset state.
        macro_rules! finalize_block {
            () => {
                if collecting != Collecting::Nothing {
                    let val = std::mem::take(&mut block_buf);
                    match collecting {
                        Collecting::Target(tgt_line) if rule_tgt.is_none() && !val.is_empty() => {
                            rule_tgt = Some((val, tgt_line));
                        },
                        Collecting::Help if rule_help.is_none() && !val.is_empty() => {
                            rule_help = Some(val);
                        },
                        _ => {}
                    }
                    collecting = Collecting::Nothing;
                    block_base = None;
                }
            };
        }

        // Emit DocumentSymbol entries for a completed rule.
        macro_rules! flush_rule {
            () => {
                if let Some((tgt_str, tgt_line)) = rule_tgt.take() {
                    for target in tgt_str.split_whitespace() {
                        let sel_end_char = target.len() as u32;

                        // VS Code requires selectionRange ⊆ fullRange.
                        // Source-map translation can produce out-of-order line
                        // numbers (e.g. {% for %} marker maps backwards), so
                        // build range defensively from the actual sel coordinates.
                        let range_start = rule_start.min(tgt_line);
                        let range_end = rule_end.max(tgt_line);
                        let range_end_char = if range_end == tgt_line {
                            sel_end_char
                        }
                        else {
                            0
                        };

                        let range = Range {
                            start: Position {
                                line: range_start,
                                character: 0
                            },
                            end: Position {
                                line: range_end,
                                character: range_end_char
                            }
                        };
                        let sel = Range {
                            start: Position {
                                line: tgt_line,
                                character: 0
                            },
                            end: Position {
                                line: tgt_line,
                                character: sel_end_char
                            }
                        };
                        #[allow(deprecated)]
                        symbols.push(DocumentSymbol {
                            name: target.to_string(),
                            detail: rule_help.clone(),
                            kind: SymbolKind::FILE,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range: sel,
                            children: None
                        });
                    }
                }
                rule_help = None;
            };
        }

        for (exp_idx, line_str) in text.lines().enumerate() {
            let orig = source_map
                .to_original(exp_idx as u32)
                .unwrap_or(exp_idx as u32);

            let indent = line_str.len() - line_str.trim_start().len();
            let trimmed = line_str.trim_start();

            // Skip blank lines; they don't terminate block scalars in YAML.
            if trimmed.is_empty() {
                continue;
            }

            // ── Block-scalar continuation ────────────────────────────────────
            if collecting != Collecting::Nothing {
                let base = *block_base.get_or_insert(indent);
                if indent >= base {
                    // Still inside the block — accumulate (strip trailing
                    // whitespace but keep words separated by a single space).
                    if !block_buf.is_empty() {
                        block_buf.push(' ');
                    }
                    block_buf.push_str(trimmed.trim_end());
                    if in_rule {
                        rule_end = orig;
                    }
                    continue;
                }
                // Indentation dropped → block is done; fall through to normal
                // processing of the current line.
                finalize_block!();
            }

            // ── Normal line processing ───────────────────────────────────────
            if trimmed.starts_with("- ") {
                if in_rule {
                    finalize_block!();
                    flush_rule!();
                }
                in_rule = true;
                rule_start = orig;
                rule_end = orig;

                let rest = trimmed[2..].trim_start();
                self.process_key_value(
                    rest,
                    orig,
                    target_names,
                    &mut rule_tgt,
                    &mut rule_help,
                    &mut collecting,
                    &mut block_base,
                    &mut block_buf
                );
            }
            else if in_rule {
                rule_end = orig;
                self.process_key_value(
                    trimmed,
                    orig,
                    target_names,
                    &mut rule_tgt,
                    &mut rule_help,
                    &mut collecting,
                    &mut block_base,
                    &mut block_buf
                );
            }
        }

        finalize_block!();
        if in_rule {
            flush_rule!();
        }

        symbols
    }

    /// Parse one line as `key: value` and update rule state.
    /// Recognises `>` and `|` as block-scalar indicators and switches
    /// `collecting` so the caller accumulates subsequent indented lines.
    fn process_key_value(
        &self,
        line: &str,
        orig: u32,
        target_names: &[&'static str],
        rule_tgt: &mut Option<(String, u32)>,
        rule_help: &mut Option<String>,
        collecting: &mut Collecting,
        block_base: &mut Option<usize>,
        block_buf: &mut String
    ) {
        let colon = match line.find(':') {
            Some(i) => i,
            None => return
        };
        let key = line[..colon].trim();
        let value = line[colon + 1..].split('#').next().unwrap_or("").trim();

        if rule_tgt.is_none() && target_names.contains(&key) {
            match value {
                ">" | "|" => {
                    *collecting = super::build::Collecting::Target(orig);
                    *block_base = None;
                    block_buf.clear();
                },
                "" => {},
                v => {
                    *rule_tgt = Some((v.to_string(), orig));
                }
            }
        }
        else if rule_help.is_none() && key == "help" {
            match value {
                ">" | "|" => {
                    *collecting = super::build::Collecting::Help;
                    *block_base = None;
                    block_buf.clear();
                },
                "" => {},
                v => {
                    *rule_help = Some(v.to_string());
                }
            }
        }
    }

    // Helper methods

    fn extract_word_at_position(&self, line: &str, column: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        if column >= chars.len() {
            return None;
        }

        let mut start = column;
        let mut end = column;

        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '-')
        {
            start -= 1;
        }

        while end < chars.len()
            && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == '-')
        {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        }
        else {
            None
        }
    }

    fn get_keyword_help(&self, word: &str) -> Option<String> {
        // Rule-level keys (with aliases) take priority
        for key in cpclib_bndbuild::lsp::RULE_KEYS {
            if key.names.contains(&word) {
                let canonical = key.names[0];
                let aliases: Vec<_> = key.names[1..].iter().copied().collect();
                let mut md = format!("**{}**\n\n{}", canonical, key.description);
                if !aliases.is_empty() {
                    md.push_str(&format!("\n\nAliases: `{}`", aliases.join("`, `")));
                }
                return Some(md);
            }
        }
        // Top-level file keywords
        for (keyword, description) in cpclib_bndbuild::lsp::BUILD_KEYWORDS {
            if *keyword == word {
                return Some(format!("**{}**\n\n{}", keyword, description));
            }
        }
        None
    }

    fn get_task_type_help(&self, word: &str) -> Option<String> {
        // Use cpclib-bndbuild's task types
        for task in cpclib_bndbuild::lsp::TASK_TYPES {
            for name in task.names {
                if *name == word {
                    return Some(format!(
                        "**{}**\n\n{}\n\nExample:\n```yaml\n{}\n```",
                        name, task.description, task.example
                    ));
                }
            }
        }
        None
    }

    fn get_top_level_completions(&self) -> Vec<CompletionItem> {
        // Use cpclib-bndbuild's build keywords
        cpclib_bndbuild::lsp::BUILD_KEYWORDS
            .iter()
            .map(|(keyword, description)| {
                CompletionItem {
                    label: keyword.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(description.to_string()),
                    insert_text: Some(format!("{}:\n  ", keyword)),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                }
            })
            .collect()
    }

    fn get_task_completions(&self) -> Vec<CompletionItem> {
        // Use cpclib-bndbuild's task types
        cpclib_bndbuild::lsp::TASK_TYPES
            .iter()
            .flat_map(|task| {
                task.names.iter().map(|name| {
                    CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some(task.description.to_string()),
                        insert_text: Some(format!("{}: ", name)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    }
                })
            })
            .collect()
    }

    fn get_jinja_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "{{ }}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja variable".to_string()),
                insert_text: Some("{{ $0 }}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "{% %}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja statement".to_string()),
                insert_text: Some("{% $0 %}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "{% if %}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja if statement".to_string()),
                insert_text: Some("{% if $1 %}\n  $0\n{% endif %}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "{% for %}".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Jinja for loop".to_string()),
                insert_text: Some("{% for $1 in $2 %}\n  $0\n{% endfor %}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }
}

impl Default for BuildFileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

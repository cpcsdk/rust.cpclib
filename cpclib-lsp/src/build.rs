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
            }
            Err(e) => {
                // Check if it contains Jinja templates (which would cause YAML parse errors)
                if text.contains("{{") || text.contains("{%") {
                    // This is likely a Jinja template - provide a warning
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 0 },
                        },
                        severity: Some(DiagnosticSeverity::INFORMATION),
                        code: None,
                        code_description: None,
                        source: Some("bndbuild".to_string()),
                        message: "File contains Jinja templates. YAML validation is limited.".to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                } else {
                    // Real YAML error
                    let line = e.location().map(|l| l.line()).unwrap_or(0);
                    let column = e.location().map(|l| l.column()).unwrap_or(0);
                    
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line.saturating_sub(1) as u32,
                                character: column as u32,
                            },
                            end: Position {
                                line: line.saturating_sub(1) as u32,
                                character: (column + 10) as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("yaml".to_string()),
                        message: format!("YAML parse error: {}", e),
                        related_information: None,
                        tags: None,
                        data: None,
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
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("bndbuild".to_string()),
                message: "Build file should contain 'targets:' or 'tasks:' section".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
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
                    value: description,
                }),
                range: None,
            });
        }
        
        // Check if it's a task type
        if let Some(description) = self.get_task_type_help(&word) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: description,
                }),
                range: None,
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
            } else if line.trim_start().starts_with("- ") {
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
                None => continue,
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
            if bytes[col] == b'-'
                && (col + 1 >= len || matches!(bytes[col + 1], b' ' | b'\t'))
            {
                raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                col += 1;
                while col < len && matches!(bytes[col], b' ' | b'\t') {
                    col += 1;
                }
            }

            while col < len {
                // Inline comment
                if bytes[col] == b'#' {
                    raw.push((line_u, col as u32, (len - col) as u32, TT_COMMENT, 0));
                    break;
                }

                // Jinja {{ expression }}
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'{' {
                    let start = col;
                    col += 2;
                    while col + 1 < len
                        && !(bytes[col] == b'}' && bytes[col + 1] == b'}')
                    {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    raw.push((line_u, start as u32, (col - start) as u32, TT_VARIABLE, 0));
                    continue;
                }

                // Jinja {% statement %}
                if col + 1 < len && bytes[col] == b'{' && bytes[col + 1] == b'%' {
                    let start = col;
                    col += 2;
                    while col + 1 < len
                        && !(bytes[col] == b'%' && bytes[col + 1] == b'}')
                    {
                        col += 1;
                    }
                    col = if col + 1 < len { col + 2 } else { len };
                    raw.push((line_u, start as u32, (col - start) as u32, TT_KEYWORD, 0));
                    continue;
                }

                // Double-quoted string
                if bytes[col] == b'"' {
                    let start = col;
                    col += 1;
                    while col < len {
                        if bytes[col] == b'"'
                            && (col == start + 1 || bytes[col - 1] != b'\\')
                        {
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
                        && (bytes[col].is_ascii_alphanumeric()
                            || matches!(bytes[col], b'_' | b'-'))
                    {
                        col += 1;
                    }
                    let word = &line_str[start..col];

                    // YAML key: word followed by ':'
                    if col < len
                        && bytes[col] == b':'
                        && (col + 1 >= len || bytes[col + 1] != b':')
                    {
                        let mods = if RULE_KEYS.contains(word) {
                            MOD_DECLARATION
                        } else {
                            0
                        };
                        raw.push((line_u, start as u32, (col - start) as u32, TT_ENUM_MEMBER, mods));
                        raw.push((line_u, col as u32, 1, TT_OPERATOR, 0));
                        col += 1;
                        continue;
                    }

                    // Boolean / null
                    if matches!(
                        word,
                        "true" | "false" | "yes" | "no"
                        | "True" | "False" | "Yes" | "No"
                        | "null" | "Null"
                    ) {
                        raw.push((line_u, start as u32, (col - start) as u32, TT_KEYWORD, 0));
                    }
                    // (other identifiers emitted with no token — they're uncoloured)
                    continue;
                }

                // Number
                if bytes[col].is_ascii_digit() {
                    let start = col;
                    while col < len
                        && (bytes[col].is_ascii_alphanumeric() || bytes[col] == b'.')
                    {
                        col += 1;
                    }
                    raw.push((line_u, start as u32, (col - start) as u32, TT_NUMBER, 0));
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
            let delta_start = if delta_line == 0 { start - prev_start } else { start };
            result.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: tok_type,
                token_modifiers_bitset: modifiers,
            });
            prev_line = line;
            prev_start = start;
        }
        result
    }

    /// Emit a CodeLens "Run" button on each line that declares a rule target.
    pub fn code_lens(&self, document: &Document) -> Vec<CodeLens> {
        let mut lenses = Vec::new();
        let line_count = document.rope.len_lines();

        for line_num in 0..line_count {
            let line_str = match document.line(line_num) {
                Some(s) => s,
                None => continue,
            };
            let trimmed = line_str.trim();

            // Look for `tgt:`, `target:`, or `build:` on this line
            let target = if let Some(v) = trimmed
                .strip_prefix("tgt:")
                .or_else(|| trimmed.strip_prefix("target:"))
                .or_else(|| trimmed.strip_prefix("build:"))
            {
                // Strip inline comment and trim
                let raw = v.split('#').next().unwrap_or("").trim();
                if raw.is_empty() {
                    continue;
                }
                raw.to_string()
            } else {
                continue;
            };

            let lsp_line = line_num as u32;
            let range = Range {
                start: Position { line: lsp_line, character: 0 },
                end:   Position { line: lsp_line, character: line_str.trim_end().len() as u32 },
            };

            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title:     format!("▶ Run: {}", target),
                    command:   "cpclib.runRule".to_string(),
                    arguments: Some(vec![serde_json::json!(target)]),
                }),
                data: None,
            });
        }

        lenses
    }

    pub fn goto_definition(&self, _document: &Document, _position: Position) -> Option<Location> {
        // TODO: Implement navigation to target definitions, included files, etc.
        None
    }

    pub fn find_references(&self, _document: &Document, _position: Position) -> Vec<Location> {
        // TODO: Find all references to targets, variables, etc.
        Vec::new()
    }

    pub fn document_symbols(&self, document: &Document) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        let text = document.text();
        
        // Parse YAML and extract structure
        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&text) {
            if let Some(mapping) = value.as_mapping() {
                for (key, _value) in mapping {
                    if let Some(key_str) = key.as_str() {
                        // Create a symbol for each top-level key
                        symbols.push(DocumentSymbol {
                            name: key_str.to_string(),
                            detail: None,
                            kind: SymbolKind::OBJECT,
                            tags: None,
                            deprecated: None,
                            range: Range {
                                start: Position { line: 0, character: 0 },
                                end: Position { line: 0, character: 0 },
                            },
                            selection_range: Range {
                                start: Position { line: 0, character: 0 },
                                end: Position { line: 0, character: 0 },
                            },
                            children: None,
                        });
                    }
                }
            }
        }
        
        symbols
    }

    // Helper methods

    fn extract_word_at_position(&self, line: &str, column: usize) -> Option<String> {
        let chars: Vec<char> = line.chars().collect();
        if column >= chars.len() {
            return None;
        }
        
        let mut start = column;
        let mut end = column;
        
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_' || chars[start - 1] == '-') {
            start -= 1;
        }
        
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == '-') {
            end += 1;
        }
        
        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
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
            .map(|(keyword, description)| CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(description.to_string()),
                insert_text: Some(format!("{}:\n  ", keyword)),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            })
            .collect()
    }

    fn get_task_completions(&self) -> Vec<CompletionItem> {
        // Use cpclib-bndbuild's task types
        cpclib_bndbuild::lsp::TASK_TYPES
            .iter()
            .flat_map(|task| {
                task.names.iter().map(|name| CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some(task.description.to_string()),
                    insert_text: Some(format!("{}: ", name)),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
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

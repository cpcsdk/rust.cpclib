//! Goto-definition and references for bndbuild files: jump from a
//! dependency to the rule producing it, or to the file on disk.

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let line = document.line(position.line as usize)?;
        let col = position.character as usize;

        // Jinja variable: the definition is its `{% set NAME = ... %}` line;
        // every other occurrence is a reference.
        if let Some(word) = jinja_word_at(&line, col)
            && let Some((_, location)) = super::jinja::collect_jinja_variables(document)
                .into_iter()
                .find(|(name, _)| *name == word)
        {
            return Some(location);
        }

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
    pub(super) fn find_target_line(
        text: &str,
        filename: &str,
        tgt_key_names: &[&str]
    ) -> Option<u32> {
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

    /// References of a Jinja variable: every whole-word occurrence in the
    /// document (the `{% set %}` line is the definition, the rest are uses).
    pub fn find_references(&self, document: &Document, position: Position) -> Vec<Location> {
        let Some(line) = document.line(position.line as usize)
        else {
            return Vec::new();
        };
        let Some(word) = jinja_word_at(&line, position.character as usize)
        else {
            return Vec::new();
        };
        // Only meaningful for variables that actually have a definition.
        if !super::jinja::collect_jinja_variables(document)
            .iter()
            .any(|(name, _)| *name == word)
        {
            return Vec::new();
        }

        let text = document.text();
        let mut refs = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            let bytes = line.as_bytes();
            let mut start = 0;
            while let Some(pos) = line[start..].find(&word) {
                let abs = start + pos;
                let before_ok =
                    abs == 0 || !(bytes[abs - 1].is_ascii_alphanumeric() || bytes[abs - 1] == b'_');
                let after = abs + word.len();
                let after_ok = after >= bytes.len()
                    || !(bytes[after].is_ascii_alphanumeric() || bytes[after] == b'_');
                if before_ok && after_ok {
                    refs.push(Location {
                        uri: document.uri.clone(),
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: abs as u32
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: after as u32
                            }
                        }
                    });
                }
                start = abs + 1;
            }
        }
        refs
    }
}

/// The identifier under the cursor, when the cursor is inside Jinja braces
/// (`{{ }}` / `{% %}`) on this line.
fn jinja_word_at(line: &str, col: usize) -> Option<String> {
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
    if start < end {
        Some(line[start..end].to_string())
    }
    else {
        None
    }
}

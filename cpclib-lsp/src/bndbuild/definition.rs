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
            && let Some((_, _, location)) = super::jinja::collect_jinja_variables(document)
                .into_iter()
                .find(|(name, ..)| *name == word)
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
        let tgt_key_names: Vec<&'static str> = cpclib_bndbuild::lsp::RULE_KEYS
            .iter()
            .find(|k| k.names.contains(&"targets"))
            .map(|k| k.names.to_vec())
            .unwrap_or_default();

        if let Some((filename, is_target_field)) = Self::filename_under_cursor(
            document,
            position.line as usize,
            &line,
            &file_key_names,
            &tgt_key_names,
            col
        ) {
            // Skip Jinja expressions — can't resolve them statically.
            if !filename.contains("{{") && !filename.contains("{%") {
                // Priority 1: only for dependency fields — jump to the rule
                // that produces this filename elsewhere. For a target field
                // this rule itself IS the producer, so "jumping to the
                // declaring rule" would just point back at the same line;
                // go straight to opening the file instead.
                if !is_target_field {
                    // Use a direct raw-text scan — it's reliable even when
                    // Jinja expansion fails — and navigate to that rule,
                    // never opening the file.
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
                }

                // Priority 2: open the file on disk (always for a target
                // field; fallback for a dependency not produced elsewhere).
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
    /// a `targets:`/`dependencies:` field, when `key` is one of `key_names`,
    /// plus whether the matched key is a `targets:` alias (as opposed to a
    /// `dependencies:` one) — callers need to distinguish the two, since a
    /// target field is never "produced elsewhere" the way a dependency is.
    /// Handles every form the field can take: the inline `- key: value`
    /// list-item form, the scalar `key: value` form, and the multi-line list
    /// form (`dep:\n  - a.bin\n  - b.bin`) — where a bare `- ` item's
    /// governing key is resolved via `enclosing_key_for_list_item`, since
    /// the item's own line never repeats it. Strips trailing YAML comments.
    fn filename_under_cursor<'a>(
        document: &Document,
        line_idx: usize,
        line: &'a str,
        key_names: &[&str],
        tgt_key_names: &[&str],
        col: usize
    ) -> Option<(&'a str, bool)> {
        let bytes = line.as_bytes();
        let len = bytes.len();

        // Skip leading whitespace and optional `- ` list marker.
        let mut i = 0;
        while i < len && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        let is_dash = i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b' ';
        if is_dash {
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

            let filename = Self::token_at_col(line, v, col)?;
            return Some((filename, tgt_key_names.contains(&key)));
        }

        // Multi-line list form: a bare `- ` item with no key of its own — its
        // governing key is on an earlier, less-indented line.
        if is_dash && !line[i..].contains(':') {
            let key = Self::enclosing_key_for_list_item(document, line_idx)?;
            if !key_names.contains(&key.as_str()) {
                return None;
            }
            let filename = Self::token_at_col(line, i, col)?;
            return Some((filename, tgt_key_names.contains(&key.as_str())));
        }

        None
    }

    /// Walk space-separated tokens in `line[start..]` until end-of-line or a
    /// YAML comment, returning the one spanning column `col` (if any).
    ///
    /// `document.line()` (ropey) includes the trailing line terminator, so
    /// `\n`/`\r` must stop scanning just like a `#` comment would.
    fn token_at_col(line: &str, start: usize, col: usize) -> Option<&str> {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let is_end = |b: u8| matches!(b, b'#' | b'\n' | b'\r');
        let mut v = start;
        while v < len && !is_end(bytes[v]) {
            while v < len && bytes[v] == b' ' {
                v += 1;
            }
            if v >= len || is_end(bytes[v]) {
                break;
            }
            let tok_start = v;
            while v < len && bytes[v] != b' ' && !is_end(bytes[v]) {
                v += 1;
            }
            let tok_end = v;
            if col >= tok_start && col < tok_end {
                return Some(&line[tok_start..tok_end]);
            }
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

        // A template with no literal characters at all (e.g. a bare
        // `{{SNA}}`) provides no constraint and would otherwise match any
        // filename whatsoever — refuse to treat that as a match, or every
        // unrelated dependency would spuriously "resolve" to this target.
        if segments.iter().all(|seg| seg.is_empty()) {
            return false;
        }

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
            .any(|(name, ..)| *name == word)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::document::Document;

    #[test]
    fn goto_definition_on_a_multi_line_dep_list_item_opens_the_file() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("helper.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  dep:\n    - helper.asm\n  cmd: basm helper.asm\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "helper.asm" within the "- helper.asm" list item (line 2).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 2,
                    character: 8
                }
            )
            .expect("goto-definition on a multi-line dep list item");
        assert_eq!(
            loc.uri,
            Url::from_file_path(tmp.path().join("helper.asm")).unwrap()
        );
    }

    #[test]
    fn goto_definition_on_a_multi_line_dep_list_item_jumps_to_the_producing_rule() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text = "- tgt: helper.o\n  cmd: basm helper.asm -o helper.o\n- tgt: out.bin\n  dep:\n    - helper.o\n  cmd: link helper.o\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "helper.o" within the "- helper.o" list item (line 4).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 4,
                    character: 8
                }
            )
            .expect("goto-definition on a multi-line dep list item to a declared target");
        assert_eq!(
            loc.range.start.line, 0,
            "should jump to the rule declaring helper.o"
        );
    }

    #[test]
    fn root_level_dash_item_is_not_treated_as_a_dep_list_value() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        // A bare `- ` item with no preceding (and thus no enclosing) key at
        // all must not be misinterpreted as a dep/tgt list value.
        let text = "- some_file.bin\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let loc = BuildFileAnalyzer::new().goto_definition(
            &doc,
            Position {
                line: 0,
                character: 4
            }
        );
        assert!(loc.is_none(), "{loc:?}");
    }

    #[test]
    fn goto_definition_on_a_target_s_own_filename_opens_the_file_not_the_rule() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("out.bin"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  cmd: basm main.asm -o out.bin\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "out.bin" within the "tgt: out.bin" field (line 0).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 0,
                    character: 9
                }
            )
            .expect("goto-definition on a target's own filename");
        assert_eq!(
            loc.uri,
            Url::from_file_path(tmp.path().join("out.bin")).unwrap(),
            "clicking a target's own filename should open the file, not jump back to its own rule"
        );
    }

    #[test]
    fn goto_definition_on_a_dep_does_not_spuriously_match_a_bare_jinja_target() {
        // The target is *entirely* a Jinja expression with no literal
        // characters ("{{SNA}}"), so it must not act as a wildcard that
        // matches every unrelated dependency filename.
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("sna.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: {{SNA}}\n  dep:\n    - sna.asm\n    - demo_code.asm\n  cmd: basm sna.asm -o {{SNA}}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "sna.asm" within the "- sna.asm" list item (line 2).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 2,
                    character: 8
                }
            )
            .expect("goto-definition on sna.asm dependency");
        assert_eq!(
            loc.uri,
            Url::from_file_path(tmp.path().join("sna.asm")).unwrap(),
            "clicking a dependency should open the file, not jump to an unrelated bare-Jinja target"
        );
    }
}

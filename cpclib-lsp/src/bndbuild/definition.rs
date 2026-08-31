//! Goto-definition and references for bndbuild files: jump from a
//! dependency to the rule producing it, or to the file on disk.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::*;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

impl BuildFileAnalyzer {
    pub fn goto_definition(&self, document: &Document, position: Position) -> Option<Location> {
        let line = document.line(position.line as usize)?;
        let col = document.byte_column(position);

        // Jinja variable: the definition is its `{% set NAME = ... %}` line;
        // every other occurrence is a reference.
        if let Some((word, ..)) = jinja_word_at(&line, col)
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
        if let Some((filename, is_target_field)) = Self::filename_under_cursor(
            document,
            position.line as usize,
            &line,
            &super::token::FILE_KEY_NAMES,
            &super::token::TGT_KEY_NAMES,
            col
        ) && let Some(loc) = Self::resolve_filename_location(
            document,
            filename,
            &super::token::TGT_KEY_NAMES,
            is_target_field
        ) {
            return Some(loc);
        }

        // ── cmd / tasks argument navigation ──────────────────────────────────
        if let Some(filename) = Self::command_filename_under_cursor(
            document,
            position.line as usize,
            &line,
            &super::token::TASK_KEY_NAMES,
            col
        ) && let Some(loc) =
            Self::resolve_filename_location(document, filename, &super::token::TGT_KEY_NAMES, false)
        {
            return Some(loc);
        }

        None
    }

    /// Resolve an already-extracted filename to a `Location`: jump to the
    /// rule that declares it as a target (unless `is_target_field` — that
    /// rule already IS the current one, so this would just point back at
    /// itself), or fall back to opening the file on disk if it exists.
    /// Skips Jinja expressions, which can't be resolved statically.
    fn resolve_filename_location(
        document: &Document,
        filename: &str,
        tgt_key_names: &[&str],
        is_target_field: bool
    ) -> Option<Location> {
        if filename.contains("{{") || filename.contains("{%") {
            return None;
        }

        if !is_target_field {
            // Use a direct raw-text scan — it's reliable even when Jinja
            // expansion fails — and navigate to that rule, never opening
            // the file.
            let raw = document.text();
            if let Some(tgt_line) = Self::find_target_line(&raw, filename, tgt_key_names) {
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

        // Open the file on disk (always for a target field; fallback for a
        // dependency/argument not produced elsewhere).
        let base_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))?;
        let path = base_dir.join(filename);
        if path.exists() {
            let uri = Url::from_file_path(path).ok()?;
            return Some(Location {
                uri,
                range: Range::default()
            });
        }
        None
    }

    /// Like `filename_under_cursor`, but for `cmd:`/`tasks:` lines: the
    /// first argv token is the command name (never a filename) and a
    /// `-`-prefixed token is a flag — every other space-separated token is
    /// offered, matching what completion already offers there
    /// (`command_argv_at_cursor` in `autocomplete.rs`). Uses simple
    /// whitespace splitting rather than shlex-aware tokenizing, consistent
    /// with `token_at_col`'s handling of `tgt:`/`dep:` values.
    fn command_filename_under_cursor<'a>(
        document: &Document,
        line_idx: usize,
        line: &'a str,
        task_key_names: &[&str],
        col: usize
    ) -> Option<&'a str> {
        let bytes = line.as_bytes();
        let len = bytes.len();

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

        let value_start = task_key_names.iter().find_map(|&key| {
            line[i..].strip_prefix(key)?.strip_prefix(':')?;
            let mut v = i + key.len() + 1;
            while v < len && bytes[v] == b' ' {
                v += 1;
            }
            Some(v)
        });
        let value_start = value_start.or_else(|| {
            if is_dash && !line[i..].contains(':') {
                let key = Self::enclosing_key_for_list_item(document, line_idx)?;
                task_key_names.contains(&key.as_str()).then_some(i)
            }
            else {
                None
            }
        })?;

        let mut v = value_start;
        let mut arg_index = 0usize;
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
                let tok = &line[tok_start..tok_end];
                return (arg_index != 0 && !tok.starts_with('-')).then_some(tok);
            }
            arg_index += 1;
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
    pub(super) fn filename_under_cursor<'a>(
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
            if line[i..]
                .strip_prefix(key)
                .and_then(|r| r.strip_prefix(':'))
                .is_none()
            {
                continue;
            }

            let mut v = i + key.len() + 1;
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
                if let Some(rest) = content.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
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
        let Some((word, ..)) = jinja_word_at(&line, document.byte_column(position))
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

        self.find_word_references(document, &word)
    }

    /// As [`find_references`](Self::find_references), for an already-known
    /// variable name rather than one resolved from a cursor position — used
    /// when rename reaches a file via the `{% include %}` graph, where
    /// there's no cursor to resolve from, only the name found at the
    /// definition site.
    pub(crate) fn find_word_references(&self, document: &Document, word: &str) -> Vec<Location> {
        let text = document.text();
        let mut refs = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            let bytes = line.as_bytes();
            let mut start = 0;
            while let Some(pos) = line[start..].find(word) {
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

    /// `textDocument/prepareRename`: only offer rename when the cursor is on
    /// a Jinja variable that actually has a `{% set %}` definition.
    pub fn prepare_rename(&self, document: &Document, position: Position) -> Option<Range> {
        let line = document.line(position.line as usize)?;
        let col = document.byte_column(position);
        let (word, start, _end) = jinja_word_at(&line, col)?;
        super::jinja::collect_jinja_variables(document)
            .iter()
            .any(|(name, ..)| *name == word)
            .then_some(())?;
        let start_utf16 = crate::common::document::byte_offset_to_utf16_col(&line, start);
        Some(Range {
            start: Position {
                line: position.line,
                character: start_utf16 as u32
            },
            end: Position {
                line: position.line,
                character: (start_utf16 + word.len()) as u32
            }
        })
    }

    /// `textDocument/rename` for a Jinja variable, within this single
    /// document only — see `crate::server::backend` for the transitive
    /// `{% include %}`-graph expansion across the workspace.
    pub fn rename(
        &self,
        document: &Document,
        position: Position,
        new_name: &str
    ) -> Option<WorkspaceEdit> {
        let refs = self.find_references(document, position);
        if refs.is_empty() {
            return None;
        }
        let edits: Vec<TextEdit> = refs
            .into_iter()
            .map(|loc| {
                TextEdit {
                    range: loc.range,
                    new_text: new_name.to_string()
                }
            })
            .collect();
        Some(WorkspaceEdit {
            changes: Some(std::collections::HashMap::from([(
                document.uri.clone(),
                edits
            )])),
            ..Default::default()
        })
    }
}

/// The identifier under the cursor, when the cursor is inside Jinja braces
/// (`{{ }}` / `{% %}`) on this line, as `(word, start, end)` byte-offset
/// span. `prepare_rename` needs the span (to build its `Range` without
/// re-scanning); other callers just discard it via `(word, ..)`.
pub(crate) fn jinja_word_at(line: &str, col: usize) -> Option<(String, usize, usize)> {
    super::jinja::jinja_context_at(line, col)?;
    super::jinja::identifier_at(line, col)
}

// ─── `{% include %}` graph (for workspace-wide Jinja variable rename) ────────

/// `true` when `path` is worth scanning for `{% include %}` directives: its
/// name exactly matches a conventional bndbuild entry-point filename
/// (`cpclib_bndbuild::builder::EXPECTED_FILENAMES`, e.g. `build.bnd`), or
/// its extension is `.bnd`/`.build` case-insensitively — covers arbitrary
/// included files too (`common.build`, `font.bnd`, `img.build`), which
/// don't follow the entry-point naming convention at all.
fn is_bndbuild_candidate(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str())
        && cpclib_bndbuild::builder::EXPECTED_FILENAMES.contains(&name)
    {
        return true;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("bnd") || e.eq_ignore_ascii_case("build"))
}

/// The `{% include %}` graph for every candidate bndbuild file under
/// `roots`: an edge `A -> B` means `A` has `{% include "..." %}` resolving
/// to `B` (paths canonicalized where possible, so the same file reached via
/// different relative paths still compares equal).
pub(crate) fn build_include_graph(roots: &[PathBuf]) -> HashMap<PathBuf, Vec<PathBuf>> {
    let mut graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for root in roots {
        for entry in crate::common::walk::files_under(root) {
            let path = entry.path();
            if !is_bndbuild_candidate(path) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path)
            else {
                continue;
            };
            let Some(dir) = path.parent()
            else {
                continue;
            };
            let edges: Vec<PathBuf> = super::jinja::extract_jinja_include_paths(&text)
                .into_iter()
                .map(|rel| {
                    let candidate = dir.join(&rel);
                    candidate.canonicalize().unwrap_or(candidate)
                })
                .collect();
            if !edges.is_empty() {
                let from = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                graph.insert(from, edges);
            }
        }
    }
    graph
}

/// Every file under `roots` that transitively `{% include %}`s `target`
/// (directly or indirectly) — the set of files a rename of a variable
/// defined in `target` needs to also update. Found by inverting
/// [`build_include_graph`] and walking its reverse edges breadth-first from
/// `target`. `target` itself is never included in the result.
pub(crate) fn files_transitively_including(roots: &[PathBuf], target: &Path) -> Vec<PathBuf> {
    let graph = build_include_graph(roots);
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());

    let mut reverse: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for (from, tos) in &graph {
        for to in tos {
            reverse.entry(to.clone()).or_default().push(from.clone());
        }
    }

    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    visited.insert(target.clone());
    queue.push_back(target);

    let mut result = Vec::new();
    while let Some(current) = queue.pop_front() {
        let Some(includers) = reverse.get(&current)
        else {
            continue;
        };
        for includer in includers {
            if visited.insert(includer.clone()) {
                result.push(includer.clone());
                queue.push_back(includer.clone());
            }
        }
    }
    result
}

/// Every file transitively `{% include %}`d *by* `from` (directly or
/// indirectly) — the downward counterpart of
/// [`files_transitively_including`], used to search for a target/macro
/// definition that isn't in `from` itself but lives in a shared/common file
/// it pulls in. Same [`build_include_graph`] adjacency map, walked forward
/// over its direct edges instead of the other function's reverse-edge walk.
/// `from` itself is never included in the result.
pub(crate) fn files_transitively_included_by(roots: &[PathBuf], from: &Path) -> Vec<PathBuf> {
    let graph = build_include_graph(roots);
    let from = from.canonicalize().unwrap_or_else(|_| from.to_path_buf());

    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    visited.insert(from.clone());
    queue.push_back(from);

    let mut result = Vec::new();
    while let Some(current) = queue.pop_front() {
        let Some(includes) = graph.get(&current)
        else {
            continue;
        };
        for included in includes {
            if visited.insert(included.clone()) {
                result.push(included.clone());
                queue.push_back(included.clone());
            }
        }
    }
    result
}

#[cfg(test)]
mod rename_tests {
    use super::*;
    use crate::common::document::Document;

    /// Mirrors the real project's shape (`demo.bnd5`): a shared
    /// `common.build` at the workspace root, `{% include %}`d by a
    /// `build.bnd` one directory down.
    #[test]
    fn build_include_graph_finds_edges_across_bnd_and_build_extensions() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("common.build"),
            "{% set CPCIP = \"192.168.1.1\" %}\n"
        )
        .unwrap();
        std::fs::create_dir_all(tmp.path().join("linking")).unwrap();
        std::fs::write(
            tmp.path().join("linking/build.bnd"),
            "{% include \"../common.build\" %}\n- tgt: t\n  cmd: xfer {{CPCIP}} -y $<\n"
        )
        .unwrap();

        let graph = build_include_graph(&[tmp.path().to_path_buf().into()]);
        let linking_bnd = tmp.path().join("linking/build.bnd").canonicalize().unwrap();
        let common_build = tmp.path().join("common.build").canonicalize().unwrap();
        assert_eq!(
            graph.get(&linking_bnd),
            Some(&vec![common_build]),
            "{graph:?}"
        );
    }

    #[test]
    fn files_transitively_including_finds_every_direct_includer() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("common.build"), "{% set X = 1 %}\n").unwrap();
        std::fs::create_dir_all(tmp.path().join("linking")).unwrap();
        std::fs::write(
            tmp.path().join("linking/build.bnd"),
            "{% include \"../common.build\" %}\n"
        )
        .unwrap();
        std::fs::create_dir_all(tmp.path().join("polar_dots")).unwrap();
        std::fs::write(
            tmp.path().join("polar_dots/build.bnd"),
            "{% include \"../common.build\" %}\n"
        )
        .unwrap();
        // A file that does NOT include common.build — must not show up.
        std::fs::write(
            tmp.path().join("unrelated.bnd"),
            "- tgt: t\n  cmd: echo hi\n"
        )
        .unwrap();

        let common_build = tmp.path().join("common.build").canonicalize().unwrap();
        let includers =
            files_transitively_including(&[tmp.path().to_path_buf().into()], &common_build);
        assert_eq!(includers.len(), 2, "{includers:?}");
        assert!(includers.contains(&tmp.path().join("linking/build.bnd").canonicalize().unwrap()));
        assert!(
            includers.contains(
                &tmp.path()
                    .join("polar_dots/build.bnd")
                    .canonicalize()
                    .unwrap()
            )
        );
    }

    #[test]
    fn files_transitively_including_follows_indirect_chains() {
        // root.build <- middle.build <- leaf.bnd
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("root.build"), "{% set X = 1 %}\n").unwrap();
        std::fs::write(
            tmp.path().join("middle.build"),
            "{% include \"root.build\" %}\n"
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("leaf.bnd"),
            "{% include \"middle.build\" %}\n"
        )
        .unwrap();

        let root_build = tmp.path().join("root.build").canonicalize().unwrap();
        let includers =
            files_transitively_including(&[tmp.path().to_path_buf().into()], &root_build);
        assert_eq!(includers.len(), 2, "{includers:?}");
        assert!(includers.contains(&tmp.path().join("middle.build").canonicalize().unwrap()));
        assert!(includers.contains(&tmp.path().join("leaf.bnd").canonicalize().unwrap()));
    }

    /// Simulates the workspace-wide expansion `backend.rs` performs: a
    /// variable name resolved from one document (the definition site) is
    /// applied to a completely different document's own text.
    #[test]
    fn word_references_can_be_computed_in_an_includer_file_independently() {
        let uri = Url::parse("file:///linking/build.bnd").unwrap();
        let text = "{% include \"../common.build\" %}\n- tgt: t\n  cmd: xfer {{CPCIP}} -y $<\n";
        let doc = Document::new(uri, text.to_string(), 1);

        let analyzer = BuildFileAnalyzer::new();
        let refs = analyzer.find_word_references(&doc, "CPCIP");
        assert_eq!(refs.len(), 1, "{refs:?}");
        assert_eq!(refs[0].range.start.line, 2);
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

    #[test]
    fn goto_definition_on_a_cmd_argument_opens_the_file() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  cmd: basm main.asm -o out.bin\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "main.asm" within the cmd argument list (line 1).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 1,
                    character: 12
                }
            )
            .expect("goto-definition on a cmd argument");
        assert_eq!(
            loc.uri,
            Url::from_file_path(tmp.path().join("main.asm")).unwrap()
        );
    }

    #[test]
    fn goto_definition_on_a_cmd_argument_that_is_a_declared_target_jumps_to_its_rule() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        let text = "- tgt: helper.o\n  cmd: basm helper.asm -o helper.o\n- tgt: out.bin\n  cmd: link helper.o -o out.bin\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "helper.o" within the second rule's cmd line (line 3).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 3,
                    character: 14
                }
            )
            .expect("goto-definition on a cmd argument matching a declared target");
        assert_eq!(loc.range.start.line, 0);
    }

    #[test]
    fn goto_definition_on_the_command_name_itself_yields_nothing() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  cmd: basm main.asm -o out.bin\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "basm" itself (the command name, not an argument).
        let loc = BuildFileAnalyzer::new().goto_definition(
            &doc,
            Position {
                line: 1,
                character: 8
            }
        );
        assert!(loc.is_none(), "{loc:?}");
    }

    #[test]
    fn goto_definition_on_a_flag_yields_nothing() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  cmd: basm main.asm -o out.bin\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "-o", a flag, not a filename.
        let loc = BuildFileAnalyzer::new().goto_definition(
            &doc,
            Position {
                line: 1,
                character: 22
            }
        );
        assert!(loc.is_none(), "{loc:?}");
    }

    #[test]
    fn goto_definition_on_a_dep_field_handles_utf16_columns_with_multibyte_content_before_it() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("helper.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        // "comment_é" is 9 Rust `char`s but 10 UTF-8 bytes ('é' is 2 bytes) -
        // the LSP `character` field counts UTF-16 code units (9, since 'é'
        // is in the Basic Multilingual Plane), so treating it as a raw byte
        // offset would land one byte short of "helper.asm"'s start and fail
        // to resolve it.
        let text = "- tgt: out.bin\n  dep: comment_\u{e9} helper.asm\n  cmd: basm helper.asm\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor right at the start of "helper.asm" (line 1, UTF-16 column 17).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 1,
                    character: 17
                }
            )
            .expect(
                "goto-definition should resolve helper.asm despite the multibyte char earlier \
                 on the line"
            );
        assert_eq!(
            loc.uri,
            Url::from_file_path(tmp.path().join("helper.asm")).unwrap()
        );
    }

    /// Regression test for the `jinja_word_at`/`identifier_at` dedup: the
    /// returned span must still convert to UTF-16 columns correctly for
    /// `prepare_rename`'s `Range`, even with multi-byte content earlier on
    /// the line.
    #[test]
    fn prepare_rename_handles_utf16_columns_with_multibyte_content_before_the_variable() {
        let uri = Url::parse("file:///build.bnd").unwrap();
        // "café" is 4 chars / 4 UTF-16 units but 5 bytes ('é' is 2 bytes) -
        // the returned Range's `character` must be reported in UTF-16
        // units (matching the LSP spec), not raw bytes.
        let text = "{% set x = 1 %}\n; café {{ x }}\n";
        let doc = Document::new(uri, text.to_string(), 1);
        let range = BuildFileAnalyzer::new()
            .prepare_rename(
                &doc,
                Position {
                    line: 1,
                    character: 10
                }
            )
            .expect("expected a rename range for the Jinja variable 'x'");
        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 10);
        assert_eq!(range.end.character, 11);
    }

    #[test]
    fn goto_definition_on_a_multi_line_cmd_list_argument_opens_the_file() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.asm"), "").unwrap();
        let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
        let text = "- tgt: out.bin\n  cmd:\n    - basm main.asm -o out.bin\n";
        let doc = Document::new(uri, text.to_string(), 1);
        // Cursor on "main.asm" within the list-form cmd item (line 2).
        let loc = BuildFileAnalyzer::new()
            .goto_definition(
                &doc,
                Position {
                    line: 2,
                    character: 12
                }
            )
            .expect("goto-definition on a multi-line cmd list argument");
        assert_eq!(
            loc.uri,
            Url::from_file_path(tmp.path().join("main.asm")).unwrap()
        );
    }
}

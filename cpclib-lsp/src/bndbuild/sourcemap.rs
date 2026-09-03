use std::sync::Arc;

use super::BuildFileAnalyzer;
use crate::common::document::Document;

const MARKER_PREFIX: &str = "#_SRCL:";
const MARKER_SUFFIX: &str = "_#";

/// Mapping from lines in the Jinja-expanded text back to lines in the original source.
/// Index is the expanded line number (0-based); value is the original line number (0-based),
/// or `None` for lines that were synthesised by Jinja with no direct source counterpart.
pub struct SourceMap {
    pub expanded_to_original: Vec<Option<u32>>
}

impl SourceMap {
    /// Convert an expanded-file line number to the original-file line number.
    pub fn to_original(&self, expanded_line: u32) -> Option<u32> {
        self.expanded_to_original
            .get(expanded_line as usize)
            .copied()
            .flatten()
    }

    /// As [`Self::to_original`], but when `expanded_line` has no direct
    /// mapping of its own (content spliced in by `{% include %}`, which
    /// carries no line marker of its own - see `annotate`'s doc comment)
    /// falls forward to the *next* expanded line that does map to
    /// something real. By construction (verified directly:
    /// `expand_with_source_map` keeps each template's own trailing newline,
    /// see `build_environment`'s doc comment on `set_keep_trailing_newline`)
    /// that next real line is always the still-annotated line the
    /// `{% include %}` directive itself was written on - its own marker
    /// survives immediately *after* all of the spliced content, since the
    /// included file's own raw text is never annotated and so never carries
    /// a marker of its own. This makes every line of the spliced content
    /// resolve to the *include statement's* own original line, so a
    /// symbol/code-lens built from it lands "on top of" the directive that
    /// spliced it in, rather than at a meaningless raw expanded-line index.
    /// Falls back to `expanded_line` itself only when there's no following
    /// mapped line at all (an `{% include %}` with nothing after it).
    pub fn to_original_or_nearest_following(&self, expanded_line: u32) -> u32 {
        if let Some(orig) = self.to_original(expanded_line) {
            return orig;
        }
        ((expanded_line as usize + 1)..self.expanded_to_original.len())
            .find_map(|i| self.expanded_to_original[i])
            .unwrap_or(expanded_line)
    }

    /// Identity map for when we're working directly on the raw file.
    pub fn identity(line_count: usize) -> Self {
        Self {
            expanded_to_original: (0..line_count as u32).map(Some).collect()
        }
    }
}

/// Build the same lenient, LSP-safe `minijinja::Environment` used throughout
/// this module: same custom functions as bndbuild's real
/// `create_template_env` (so templates calling them don't abort the LSP's
/// own rendering), but stubbed to never fail/write/shell out, and
/// `Chainable` undefined-variable behavior so a file mid-edit (or missing
/// `-D` definitions the real build would otherwise require) still renders
/// as far as possible instead of erroring immediately. Shared by
/// `expand_with_source_map` (diagnostics/symbols) and macro-call hover.
///
/// `file_dir` is the directory used as base path for `{% include %}`
/// directives. Pass `None` when no file-system includes are expected.
pub(super) use cpclib_project::jinja::build_environment;

/// Expand a Jinja template, returning the rendered text and a source map.
///
/// The source map maps each line in the output back to the line in `source`
/// that generated it.  Lines that come from Jinja control structures
/// (`{% for %}`, `{% if %}`, etc.) with no data payload map to `None`.
///
/// `file_dir` is the directory used as base path for `{% include %}` directives.
/// Pass `None` when no file-system includes are expected.
pub fn expand_with_source_map(
    source: &str,
    file_dir: Option<&std::path::Path>
) -> Result<(String, SourceMap), minijinja::Error> {
    // Step 1 — annotate every line with its original line number.
    let annotated = annotate(source);

    // Step 2 — render through minijinja.
    let env = build_environment(file_dir);

    // No variable context — variables defined via {% set %} in the template
    // are sufficient; external definitions resolve to Undefined (lenient).
    let rendered = env.render_str(&annotated, minijinja::context!())?;

    // Step 3 — build the source map and strip markers.
    let mut map = Vec::new();
    let mut clean_lines = Vec::new();

    for line in rendered.lines() {
        let (clean, original) = strip_marker(line);
        clean_lines.push(clean);
        map.push(original);
    }

    let clean_text = clean_lines.join("\n");
    // Preserve trailing newline if the rendered output had one.
    let clean_text = if rendered.ends_with('\n') {
        format!("{}\n", clean_text)
    }
    else {
        clean_text
    };

    Ok((
        clean_text,
        SourceMap {
            expanded_to_original: map
        }
    ))
}

impl BuildFileAnalyzer {
    /// `expand_with_source_map` on `document`'s own text/directory, cached
    /// per document version — every hover/definition/symbols/diagnostics/
    /// call-hierarchy/semantic-tokens request that needs the expanded text
    /// reuses the same `Arc` for a given `(uri, version)` instead of
    /// redoing a full minijinja parse+render (including a filesystem read
    /// for every `{% include %}`) from scratch. Errors are never cached
    /// (cheap to reproduce, and `minijinja::Error` isn't `Clone`) — every
    /// caller already has its own fallback (typically an identity source
    /// map) for that case.
    pub(super) fn expand_cached(
        &self,
        document: &Document
    ) -> Result<Arc<(String, SourceMap)>, minijinja::Error> {
        if let Some(entry) = self.expand_cache.get(&document.uri)
            && entry.0 == document.version
        {
            return Ok(Arc::clone(&entry.1));
        }

        let raw_text = document.text();
        let file_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let result = Arc::new(expand_with_source_map(&raw_text, file_dir.as_deref())?);

        self.expand_cache.insert(
            document.uri.clone(),
            (document.version, Arc::clone(&result))
        );
        Ok(result)
    }

    /// `expand_cached`, falling back to an identity source map on expansion
    /// failure (missing variables, unresolvable includes, syntax errors,
    /// ...) — the common "best-effort" shape every caller that doesn't
    /// itself need the specific error wants. Returns the cached `Arc`
    /// directly rather than cloning its contents, so a cache hit stays
    /// cheap regardless of document size.
    pub(super) fn expand_or_identity(&self, document: &Document) -> Arc<(String, SourceMap)> {
        match self.expand_cached(document) {
            Ok(result) => result,
            Err(_) => {
                let raw_text = document.text();
                let lines = raw_text.lines().count();
                Arc::new((raw_text, SourceMap::identity(lines)))
            }
        }
    }
}

// ─── internal helpers ────────────────────────────────────────────────────────

fn annotate(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + source.lines().count() * 16);
    let mut in_block_tag = false; // inside a {%…%} that hasn't closed yet

    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim_end();
        // Decide before annotating: does this line end inside an open block?
        let ends_in_block = line_ends_in_block_tag(trimmed, in_block_tag);

        if in_block_tag || ends_in_block {
            // Line is inside or starts a multi-line {% %} tag —
            // injecting the marker here would corrupt the Jinja expression.
            out.push_str(trimmed);
        }
        else {
            out.push_str(trimmed);
            out.push(' ');
            out.push_str(MARKER_PREFIX);
            out.push_str(&i.to_string());
            out.push_str(MARKER_SUFFIX);
        }
        out.push('\n');
        in_block_tag = ends_in_block;
    }
    out
}

/// Returns `true` if `line` ends while still inside an open `{%…%}` block tag.
/// `starts_inside` carries the state from the previous line.
fn line_ends_in_block_tag(line: &str, starts_inside: bool) -> bool {
    let bytes = line.as_bytes();
    let mut inside = starts_inside;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if !inside && bytes[i] == b'{' && bytes[i + 1] == b'%' {
            inside = true;
            i += 2;
        }
        else if inside && bytes[i] == b'%' && bytes[i + 1] == b'}' {
            inside = false;
            i += 2;
        }
        else {
            i += 1;
        }
    }
    inside
}

fn strip_marker(line: &str) -> (String, Option<u32>) {
    if let Some(marker_start) = line.rfind(MARKER_PREFIX) {
        let after = &line[marker_start + MARKER_PREFIX.len()..];
        if let Some(end) = after.find(MARKER_SUFFIX) {
            if let Ok(n) = after[..end].parse::<u32>() {
                // Remove the marker (and any whitespace before it).
                let clean = line[..marker_start].trim_end().to_string();
                return (clean, Some(n));
            }
        }
    }
    (line.to_string(), None)
}

#[cfg(test)]
mod expand_cache_tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(text: &str, version: i32) -> Document {
        Document::new(
            Url::parse("file:///build.bnd").unwrap(),
            text.to_string(),
            version
        )
    }

    #[test]
    fn same_version_returns_the_identical_cached_arc() {
        let analyzer = BuildFileAnalyzer::new();
        let d = doc("- tgt: a\n  cmd: echo hi\n", 1);
        let first = analyzer.expand_cached(&d).unwrap();
        let second = analyzer.expand_cached(&d).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_version_bump_reexpands_and_returns_a_different_arc() {
        let analyzer = BuildFileAnalyzer::new();
        let d1 = doc("- tgt: a\n  cmd: echo hi\n", 1);
        let first = analyzer.expand_cached(&d1).unwrap();

        let d2 = doc("- tgt: b\n  cmd: echo hi\n", 2);
        let second = analyzer.expand_cached(&d2).unwrap();

        assert!(!Arc::ptr_eq(&first, &second));
        assert!(second.0.contains("tgt: b"), "{}", second.0);
    }

    #[test]
    fn evict_forces_a_fresh_expansion_even_at_the_same_version() {
        let analyzer = BuildFileAnalyzer::new();
        let d = doc("- tgt: a\n  cmd: echo hi\n", 1);
        let first = analyzer.expand_cached(&d).unwrap();
        analyzer.evict(&d.uri);
        let second = analyzer.expand_cached(&d).unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn expand_or_identity_also_hits_the_cache() {
        let analyzer = BuildFileAnalyzer::new();
        let d = doc("- tgt: a\n  cmd: echo hi\n", 1);
        let via_cached = analyzer.expand_cached(&d).unwrap();
        let via_identity = analyzer.expand_or_identity(&d);
        assert!(Arc::ptr_eq(&via_cached, &via_identity));
    }

    /// Regression test for the double-expand bug in `diagnostics::analyze`:
    /// it used to call `expand_with_source_map` once directly, then again
    /// (from scratch) via `validate_build_structure`. Both call sites now
    /// go through `expand_cached`/`expand_or_identity`, so `analyze()`
    /// leaves a populated cache entry behind — a plain call to
    /// `expand_with_source_map` bypassing the cache entirely (the old
    /// behavior) would leave `expand_cache` empty.
    #[test]
    fn diagnostics_analyze_populates_the_expand_cache() {
        let analyzer = BuildFileAnalyzer::new();
        let d = doc(
            "{% for i in [1, 2] %}\n- tgt: out{{i}}.bin\n  cmd: echo {{i}}\n{% endfor %}\n",
            1
        );
        analyzer.analyze(&d);

        let cached = analyzer
            .expand_cache
            .get(&d.uri)
            .map(|entry| Arc::clone(&entry.1))
            .expect("analyze() should have populated the expand cache");
        // A further lookup for the same version must be the exact same
        // `Arc` `analyze()` already produced — i.e. everything downstream
        // of `analyze()`'s first expansion, including its own
        // `validate_build_structure` call, shares this one result.
        let again = analyzer.expand_cached(&d).unwrap();
        assert!(Arc::ptr_eq(&cached, &again));
    }
}

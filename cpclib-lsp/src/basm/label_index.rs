//! A per-file, incrementally-maintained cache of "every identifier-like word
//! and where it occurs in this file" (`FileLabelFacts`), plus a
//! workspace-wide reverse index (`AssemblyAnalyzer::global_name_index`)
//! mapping a word to every file *this session has ever seen* mention it.
//!
//! The point: every workspace-wide label feature today
//! (`server::backend::find_references_across_workspace`,
//! `rename_label_across_workspace`, `find_unreferenced_labels_in_workspace`)
//! re-reads and re-scans every candidate file's *text* from scratch on
//! every single call - confirmed by reading all three, none of them cache
//! anything across calls. `ensure_label_facts` replaces that per-call
//! re-scan with a per-file cache keyed by document version (the same
//! `(i32, T)` shape and `DashMap::entry` Occupied/Vacant coalescing
//! `cpclib_project::cache::ProjectCache::graph_for` already uses for a
//! different cross-file cache - see that function's own doc comment for why
//! the coalescing matters: two concurrent callers that both miss on the same
//! file must not duplicate the recompute), so a file touched once by any
//! query - or by simply being edited, since `server::backend::spawn_deferred_analysis`
//! calls `ensure_label_facts` on every debounced `did_open`/`did_change`/
//! `did_save`, right alongside its existing `update_embedded_bndbuild_index`
//! call - stays warm for every later query, workspace-wide, until it is
//! actually edited again.
//!
//! `global_name_index` is a complementary, best-effort structure: it lets a
//! caller skip touching files *known* not to mention a word at all, without
//! needing to consult every candidate file's own cache entry first. It is
//! deliberately not treated as a complete workspace-wide oracle on its own -
//! a file this session has never opened, edited, or had touched by some
//! prior workspace-wide query is simply absent from it - so consumers that
//! need a *guaranteed-complete* answer (`find_references_across_workspace`,
//! `find_unreferenced_labels_in_workspace`) still walk `candidate_asm_paths`
//! and call `ensure_label_facts` on each candidate (cheap once warm, exactly
//! today's cost once cold); only a consumer that can already tolerate
//! "complete among what's been seen so far, converging as the workspace is
//! used" (nothing currently does, but this is the intended future home for
//! e.g. an always-workspace-wide reference-count CodeLens) may read
//! `global_name_index` alone, with no directory walk at all.
//!
//! Deliberately *not* handled here: a file deleted from disk while indexed
//! leaves a harmless phantom entry in both maps until the process restarts
//! (nothing currently prunes on delete - the existing `candidate_asm_paths`
//! walk simply stops returning that path, so it stops being *discovered*
//! via the guaranteed-complete callers above; a `did_close` without a save
//! also needs no special reconciliation, since `did_close` already removes
//! the URI from `documents`, so the next touch of that file - by anything -
//! loads it fresh from disk via `disk_file_version`, whose value is
//! essentially never equal to whatever small LSP version was last cached
//! here, which is exactly what makes `ensure_label_facts`'s own
//! version-mismatch check recompute it self-healingly).

use std::collections::HashSet;
use std::sync::Arc;

use dashmap::mapref::entry::Entry;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

use super::AssemblyAnalyzer;
use super::token::is_ident_byte;
use crate::common::document::Document;

/// One file's own indexed facts.
#[derive(Debug, Default)]
pub struct FileLabelFacts {
    /// Every global AND local label defined in this file: canonical name
    /// (normalized per case-sensitivity, used as the map key so a lookup
    /// matches `occurrences`' own keying) -> `(name as actually written in
    /// source, definition range)`. Built via `token::label_definitions_in`
    /// - the shared primitive `symbols.rs`'s outline and
    /// `autocomplete.rs::collect_symbols` also use - so a name's presence
    /// here means exactly what it already means there. A global's
    /// canonical name is its own bare name; a local's is qualified against
    /// its owning global (`global.local`) - see the module doc comment.
    ///
    /// `IndexMap`, not `HashMap`: iteration order matters to
    /// `reference_count_code_lenses` (`references_lens.rs`), which reads
    /// this to place one CodeLens per definition - an `IndexMap` preserves
    /// the document order `label_definitions_in`'s own listing walk
    /// produces (a plain insertion order, no separate sort step needed),
    /// while still giving O(1) lookups like a `HashMap`.
    pub definitions: IndexMap<String, (String, Range)>,
    /// Every whole-word occurrence of every identifier-like word anywhere in
    /// this file's text, bucketed by *canonical* name - one linear pass,
    /// not one pass per queried name. `.` is itself an identifier byte (see
    /// `is_ident_byte`), so an already-qualified `global.local` tokenizes as
    /// one word on its own, matching its own canonical form with no further
    /// work; a *bare* local (`.local`) is re-keyed to that same canonical
    /// form during tokenizing (`owning_global_of`), since its bare text
    /// alone can't tell two same-named locals under different globals
    /// apart - see the module doc comment.
    ///
    /// `FxHashMap`, not `std::collections::HashMap`: this is rebuilt from
    /// scratch on every edit (`ensure_label_facts` recomputes on any
    /// version bump) with one insert per identifier occurrence in the file
    /// - tens of thousands on a real large file - so the default hasher's
    /// DoS-resistant SipHash is paying for a guarantee this in-process,
    /// non-adversarial-input cache has no use for. `FxHash` (the same
    /// hasher rustc itself uses internally for exactly this kind of
    /// many-small-string-keys hot path) trades that guarantee for speed.
    pub occurrences: FxHashMap<String, Vec<Range>>
}

/// Maximal identifier-byte runs on `line`, as `(word, start_col, end_col)` -
/// the tokenization `compute_label_facts` buckets every occurrence by.
fn tokenize_line(line: &str) -> Vec<(&str, usize, usize)> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if is_ident_byte(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            out.push((&line[start..i], start, i));
        }
        else {
            i += 1;
        }
    }
    out
}

fn normalize(word: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        word.to_string()
    }
    else {
        word.to_uppercase()
    }
}

/// Same case-folding as `normalize`, but for a caller that already owns
/// `word` outright (nothing else keeps its own reference to it) and can
/// fold it in place instead of allocating a second copy just to hold the
/// folded result - `make_ascii_uppercase` mutates rather than allocating,
/// which is safe here since every basm identifier this indexes is ASCII
/// (case-folding never changes an ASCII string's byte length).
fn normalize_owned(mut word: String, case_sensitive: bool) -> String {
    if !case_sensitive {
        word.make_ascii_uppercase();
    }
    word
}

impl AssemblyAnalyzer {
    /// Build `document`'s `FileLabelFacts` from scratch. Parses once and
    /// computes `global_label_scopes` once, reused for both `definitions`
    /// and the occurrence-canonicalizing tokenizer below - `references_lens.rs`'s
    /// `all_label_definitions` and this used to each independently re-parse
    /// and re-walk the listing to get the same definitions, and this
    /// function separately recomputed the scopes a second time on top of
    /// that just for its own tokenizing pass.
    fn compute_label_facts(&self, document: &Document) -> FileLabelFacts {
        let case_sensitive = self.config().case_sensitive;

        let parsed = self.parse_document(document).ok();
        // `None` when the document doesn't cleanly parse - a bare local
        // occurrence just keeps its own, non-canonical text as its key in
        // that case (via `qualify_local_at_line` returning `None`), rather
        // than the whole pass failing.
        let scopes = parsed
            .as_ref()
            .map(|listing| super::token::global_label_scopes(listing.iter()));

        let definitions = match (&parsed, &scopes) {
            (Some(listing), Some(scopes)) => super::token::label_definitions_in(listing.iter(), scopes),
            _ => Vec::new()
        }
        .into_iter()
        .map(|(name, range)| (normalize(&name, case_sensitive), (name, range)))
        .collect();

        let mut occurrences: FxHashMap<String, Vec<Range>> = FxHashMap::default();
        for (line_idx, line) in document.text().lines().enumerate() {
            let line_idx = line_idx as u32;
            for (word, start, end) in tokenize_line(&line) {
                // A bare local is only ever meaningful relative to whatever
                // global's scope contains *this occurrence* - not
                // necessarily the same global that owns the file's very
                // first `.foo` definition, since a file can (and often
                // does) have several globals each with their own same-named
                // locals. Shares `token::qualify_local_at_line` with
                // `definitions` above (and the outline), so a local's
                // canonical form is computed the same way everywhere - a
                // global word, or a local with no enclosing scope, is
                // returned unchanged.
                // One allocation, not two: `qualify_local_at_line` already
                // returns an owned `String` (it has to - the qualified form
                // doesn't exist in the source text to borrow from), so
                // there is no separately-kept "canonical" value here worth
                // allocating again just to case-fold - fold this same
                // owned `String` in place instead (`normalize_owned`).
                let owned = scopes
                    .as_ref()
                    .and_then(|scopes| super::token::qualify_local_at_line(scopes, line_idx, word))
                    .unwrap_or_else(|| word.to_string());
                let key = normalize_owned(owned, case_sensitive);
                occurrences.entry(key).or_default().push(Range {
                    start: Position {
                        line: line_idx,
                        character: start as u32
                    },
                    end: Position {
                        line: line_idx,
                        character: end as u32
                    }
                });
            }
        }

        FileLabelFacts {
            definitions,
            occurrences
        }
    }

    /// `document`'s cached facts if still fresh for its current `version`,
    /// else recomputed, stored, and diffed into `global_name_index` - see
    /// the module doc comment for the coalescing and self-healing
    /// properties this relies on.
    pub fn ensure_label_facts(&self, document: &Document) -> Arc<FileLabelFacts> {
        match self.label_index.entry(document.uri.clone()) {
            Entry::Occupied(occ) if occ.get().0 == document.version => occ.get().1.clone(),
            Entry::Occupied(mut occ) => {
                let old_words: HashSet<String> = occ.get().1.occurrences.keys().cloned().collect();
                let facts = Arc::new(self.compute_label_facts(document));
                self.reindex_global_names(&document.uri, &old_words, &facts);
                occ.insert((document.version, facts.clone()));
                facts
            },
            Entry::Vacant(vac) => {
                let facts = Arc::new(self.compute_label_facts(document));
                self.reindex_global_names(&document.uri, &HashSet::new(), &facts);
                vac.insert((document.version, facts.clone()));
                facts
            }
        }
    }

    /// Whether `label_index` already holds a fresh entry for `uri` at
    /// `version` - a pure map lookup, no file IO at all. Lets a caller that
    /// only has a *cheap* way to learn a file's current version (a
    /// `disk_file_version` `stat()`, say) decide whether it's even worth
    /// paying for the expensive part (reading the file's full content)
    /// before doing so - see `reference_count_code_lenses`'s own use of
    /// this for its current document's includes, re-checked on essentially
    /// every keystroke.
    pub fn is_label_index_fresh(&self, uri: &Url, version: i32) -> bool {
        self.label_index.get(uri).is_some_and(|entry| entry.0 == version)
    }

    /// Diff `old_words` (this file's previously-indexed occurrence keys, or
    /// empty if this is the first time) against `new_facts`, adding/removing
    /// `uri` from `global_name_index`'s per-word entry sets accordingly.
    fn reindex_global_names(&self, uri: &Url, old_words: &HashSet<String>, new_facts: &FileLabelFacts) {
        for word in old_words {
            if !new_facts.occurrences.contains_key(word) {
                if let Entry::Occupied(mut occ) = self.global_name_index.entry(word.clone()) {
                    occ.get_mut().remove(uri);
                    if occ.get().is_empty() {
                        occ.remove();
                    }
                }
            }
        }
        for word in new_facts.occurrences.keys() {
            if !old_words.contains(word) {
                self.global_name_index
                    .entry(word.clone())
                    .or_default()
                    .insert(uri.clone());
            }
        }
    }

    /// Every file this session currently knows mentions `word` (definition
    /// or reference alike) - a best-effort, in-memory-only read with no
    /// directory walk and no file IO. See the module doc comment for why
    /// this is not a substitute for a guaranteed-complete workspace scan.
    pub fn known_files_mentioning(&self, word: &str) -> Vec<Url> {
        let key = normalize(word, self.config().case_sensitive);
        self.global_name_index
            .get(&key)
            .map(|entry| entry.value().iter().cloned().collect())
            .unwrap_or_default()
    }

    /// `Location`s of every whole-word occurrence of `word` in `document`,
    /// backed by its cached `FileLabelFacts` (built once per document
    /// version, not re-scanned per call) - the index-backed replacement for
    /// `definition::find_references_in`'s own per-call text scan.
    ///
    /// Normalization uses this analyzer's own `case_sensitive` config,
    /// exactly what every real (non-test) call site of
    /// `find_references_in` already passes - unlike that function, there is
    /// no per-call override here, since a cache keyed by one fixed
    /// normalization can't correctly answer a query issued under a
    /// different one. `find_references_in` itself is untouched and remains
    /// the function to reach for when a genuinely different case-
    /// sensitivity is needed for one specific call (as some of its own
    /// tests do).
    pub fn label_locations_in(&self, document: &Document, word: &str) -> Vec<Location> {
        let facts = self.ensure_label_facts(document);
        let key = normalize(word, self.config().case_sensitive);
        facts
            .occurrences
            .get(&key)
            .map(|ranges| {
                ranges
                    .iter()
                    .map(|range| {
                        Location {
                            uri: document.uri.clone(),
                            range: *range
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every location referencing `name` this session currently knows about:
    /// `document`'s own occurrences (via `label_locations_in`, which ensures
    /// them fresh right now - always accurate for the file actually being
    /// looked at, no indexing lag) plus every *other* file
    /// `known_files_mentioning` already has on record.
    ///
    /// Best-effort and workspace-wide beyond `document` itself: a file
    /// nothing has opened, edited, or otherwise touched yet simply isn't
    /// known, so it's silently absent here rather than causing a directory
    /// walk or a file read to go find out. That's the deliberate trade-off
    /// that makes this safe to call from a live `CodeLens` - see the module
    /// doc comment - and it only gets *more* complete as the workspace is
    /// used (opening a file, or running any on-demand workspace-wide
    /// command, warms it for every later call).
    pub fn known_workspace_locations(&self, document: &Document, name: &str) -> Vec<Location> {
        let mut locations = self.label_locations_in(document, name);
        let key = normalize(name, self.config().case_sensitive);
        for uri in self.known_files_mentioning(name) {
            if uri == document.uri {
                continue;
            }
            if let Some(entry) = self.label_index.get(&uri)
                && let Some(ranges) = entry.1.occurrences.get(&key)
            {
                locations.extend(ranges.iter().map(|range| {
                    Location {
                        uri: uri.clone(),
                        range: *range
                    }
                }));
            }
        }
        locations
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 1)
    }

    #[test]
    fn a_label_defined_and_used_is_indexed_with_both_occurrences() {
        let analyzer = AssemblyAnalyzer::new();
        let facts = analyzer.ensure_label_facts(&doc("start:\n    call start\n"));
        assert!(facts.definitions.contains_key("start"));
        assert_eq!(facts.occurrences["start"].len(), 2);
    }

    #[test]
    fn two_same_named_locals_under_different_globals_are_distinct_buckets() {
        // The whole reason a bare local is canonicalized against its
        // owning global rather than kept as its own bare `.local` text:
        // two unrelated locals sharing a name under different globals must
        // never collide in one bucket.
        let analyzer = AssemblyAnalyzer::new();
        let facts = analyzer.ensure_label_facts(&doc(
            "one:\n.local\n    ret\ntwo:\n.local\n    ret\n"
        ));
        let keys: Vec<_> = facts.occurrences.keys().cloned().collect();
        assert!(facts.occurrences.contains_key("one.local"), "{keys:?}");
        assert!(facts.occurrences.contains_key("two.local"), "{keys:?}");
        assert_eq!(facts.occurrences["one.local"].len(), 1, "{keys:?}");
        assert_eq!(facts.occurrences["two.local"].len(), 1, "{keys:?}");
    }

    #[test]
    fn a_bare_local_and_its_explicitly_qualified_form_share_the_same_bucket() {
        let analyzer = AssemblyAnalyzer::new();
        let facts = analyzer.ensure_label_facts(&doc(
            "one:\n.local\n    ret\ntwo:\n    call one.local\n    ret\n"
        ));
        // The bare definition (line 1) and the qualified reference from an
        // unrelated global's own body (line 4) must land in the same
        // canonical bucket.
        assert_eq!(
            facts.occurrences["one.local"].len(),
            2,
            "{:?}",
            facts.occurrences.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn editing_the_document_updates_the_global_name_index_without_closing_it() {
        let analyzer = AssemblyAnalyzer::new();
        let v1 = Document::new(Url::parse("file:///main.asm").unwrap(), "used_word:\n    ret\n".to_string(), 1);
        analyzer.ensure_label_facts(&v1);
        assert_eq!(analyzer.known_files_mentioning("used_word").len(), 1);

        // Same URI, new version, the word is gone - a real edit, not a close.
        let v2 = Document::new(Url::parse("file:///main.asm").unwrap(), "renamed:\n    ret\n".to_string(), 2);
        analyzer.ensure_label_facts(&v2);
        assert_eq!(analyzer.known_files_mentioning("used_word").len(), 0);
        assert_eq!(analyzer.known_files_mentioning("renamed").len(), 1);
    }

    #[test]
    fn two_different_files_mentioning_the_same_word_both_appear_in_the_reverse_index() {
        let analyzer = AssemblyAnalyzer::new();
        let a = Document::new(Url::parse("file:///a.asm").unwrap(), "    call shared\n".to_string(), 1);
        let b = Document::new(Url::parse("file:///b.asm").unwrap(), "shared:\n    ret\n".to_string(), 1);
        analyzer.ensure_label_facts(&a);
        analyzer.ensure_label_facts(&b);
        let mut files = analyzer.known_files_mentioning("shared");
        files.sort_by_key(|u| u.to_string());
        assert_eq!(files.len(), 2, "{files:?}");
    }

    #[test]
    fn a_cached_hit_does_not_recompute_when_the_version_is_unchanged() {
        let analyzer = AssemblyAnalyzer::new();
        let document = doc("start:\n    ret\n");
        let first = analyzer.ensure_label_facts(&document);
        let second = analyzer.ensure_label_facts(&document);
        // Same version -> the exact same cached Arc is handed back, not a
        // freshly-built one.
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn is_label_index_fresh_is_a_pure_lookup_with_no_file_i_o() {
        // The primitive `references_lens.rs::ensure_includes_indexed` uses
        // to decide whether an include's content is even worth reading -
        // must report "not fresh" for anything never indexed, "fresh" once
        // indexed at that exact version, and "not fresh" again for any
        // other version (an edit having since happened).
        let analyzer = AssemblyAnalyzer::new();
        let uri = Url::parse("file:///main.asm").unwrap();
        assert!(!analyzer.is_label_index_fresh(&uri, 1));

        let document = Document::new(uri.clone(), "start:\n    ret\n".to_string(), 1);
        analyzer.ensure_label_facts(&document);
        assert!(analyzer.is_label_index_fresh(&uri, 1));
        assert!(!analyzer.is_label_index_fresh(&uri, 2));
    }
}

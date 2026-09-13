//! A "N references" `CodeLens` above every label definition, global or local
//! - the `references`-side analogue of `peephole.rs`'s "Fix All" lens.
//!
//! Workspace-wide, backed by `label_index::AssemblyAnalyzer::known_workspace_locations`:
//! the current document plus its own direct `INCLUDE`s are always ensured
//! fresh right here (so *this* file's own count never lags an edit), and
//! every other file the session already knows about (opened, edited, or
//! touched by any on-demand workspace scan) contributes too - all as cached
//! index reads, no directory walk, no extra file IO beyond the current
//! document's own includes. `code_lens` is re-requested on essentially
//! every edit (see `server/backend.rs`'s own "always cheap" doc comment on
//! that handler), which is exactly what this design keeps safe: it costs
//! what it always cost (parse the current document plus its own includes),
//! now with a warm cache behind it instead of a cold re-scan every time.
//!
//! This means the count is *not* a hard guarantee of completeness on a
//! server that just started and has touched nothing else yet - a workspace
//! file nobody has opened, edited, or scanned is invisible until something
//! does. It only grows more complete as the workspace is used, same as
//! most IDE features backed by an incremental index. For a guaranteed-
//! complete, one-shot answer regardless of what's been touched so far, use
//! the on-demand `cpclib.findUnreferencedLabels` command (`server/backend.rs`),
//! which still walks the whole workspace explicitly.
//!
//! Local (dotted) labels get a lens too, via `label_index::FileLabelFacts::definitions`:
//! a local is only ever meaningful relative to the global whose scope
//! contains it (`global1: / .local1` means something different from
//! another, unrelated `.local1` under a different global elsewhere), so
//! every local occurrence - bare (`.local1`, only valid within its owning
//! global's own scope) or explicitly qualified (`global1.local1`, valid
//! anywhere) - is bucketed in `label_index::FileLabelFacts` under one
//! canonical key, `global1.local1`, rather than under the bare, ambiguous
//! `.local1` text. See `label_index`'s own module doc comment for the
//! tokenizing details.

use tower_lsp::lsp_types::*;

use super::AssemblyAnalyzer;
use crate::common::document::Document;

impl AssemblyAnalyzer {
    pub fn reference_count_code_lenses(&self, document: &Document) -> Vec<CodeLens> {
        // Ensure the current document's own direct includes are indexed
        // too - feeding them through `ensure_label_facts` means they also
        // become "known" for every other file's own lens from now on, not
        // just for this one call.
        self.ensure_includes_indexed(document);

        // The current document's own facts, already ensured fresh - this
        // is also where the label definitions come from now (`definitions`
        // is exactly `token::label_definitions_in`'s own output, computed
        // once here rather than by a second, separate parse-and-walk of
        // the same listing this call already needed anyway for the
        // occurrence side).
        let facts = self.ensure_label_facts(document);

        // `facts.definitions` is an `IndexMap`, so this iterates in the
        // same document order `token::label_definitions_in`'s listing walk
        // produced it in - no separate sort needed to keep lens order
        // stable across calls.
        let lenses: Vec<CodeLens> = facts
            .definitions
            .values()
            .map(|(raw, def_range)| {
                let def_range = *def_range;
                let mut locations = self.known_workspace_locations(document, raw);
                // Exclude the definition's own occurrence - every source of
                // locations here counts every whole-word match, definition
                // line included.
                locations.retain(|l| !(l.uri == document.uri && l.range == def_range));

                let count = locations.len();
                let title = match count {
                    0 => "no references".to_string(),
                    1 => "1 reference".to_string(),
                    n => format!("{n} references")
                };
                CodeLens {
                    range: def_range,
                    command: Some(Command {
                        title,
                        // Not `editor.action.showReferences` directly: that
                        // built-in command validates its arguments with
                        // `arg instanceof vscode.Uri`/`Position`/`Location`,
                        // which a CodeLens `Command` coming straight from the
                        // LSP response never satisfies (`vscode-languageclient`
                        // hands it the plain JSON values, not reconstructed
                        // class instances) - clicking the lens threw exactly
                        // that constraint error. `cpclib.showReferenceLocations`
                        // is a client-side-only command
                        // (`cpclib-vscode/src/commands/references.ts`) that
                        // does the reconstruction before forwarding to the real
                        // built-in - same reasoning as `cpclib.debugAssembly`/
                        // `cpclib.runAsm` being client-side-only commands
                        // referenced by `embedded_bndbuild.rs`'s own lenses.
                        command: "cpclib.showReferenceLocations".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(document.uri.to_string()),
                            serde_json::json!(def_range.start),
                            serde_json::json!(locations),
                        ])
                    }),
                    data: None
                }
            })
            .collect();
        lenses
    }

    /// Ensure every file `document` itself directly `INCLUDE`s/`INCBIN`s/
    /// `BINCLUDE`s is freshly indexed (`ensure_label_facts`) - same URI/
    /// version rules as `autocomplete::collect_symbols_from_includes` - but
    /// checks `is_label_index_fresh` *before* reading the file's content,
    /// skipping the read entirely on a cache hit.
    ///
    /// This matters specifically because `reference_count_code_lenses`
    /// calls this on essentially every keystroke: computing an include's
    /// synthetic URI and version (a `stat()`, or the fixed version `0` for
    /// an `inner://` resource) is cheap and unavoidable either way, but
    /// unconditionally reading its full content first - as this used to,
    /// building a throwaway `Document` just to hand it to
    /// `ensure_label_facts`, which would then immediately discover the
    /// version already matched and drop that content on the floor - paid
    /// for a real disk read on every single call even when nothing about
    /// the include had changed since the last one.
    fn ensure_includes_indexed(&self, document: &Document) {
        for filename in super::definition::extract_include_filenames(&document.text()) {
            let uri = super::autocomplete::synthetic_include_uri(&filename, &document.uri);
            let version = if super::includes::is_inner_uri(&filename) {
                0
            }
            else {
                super::definition::resolve_include_path(&filename, &document.uri)
                    .map(|path| crate::server::backend::disk_file_version(&path))
                    .unwrap_or(0)
            };
            if self.is_label_index_fresh(&uri, version) {
                continue;
            }
            let Some(content) = super::includes::read_included_file(&filename, &document.uri)
            else {
                continue;
            };
            self.ensure_label_facts(&Document::new(uri, content, version));
        }
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///main.asm").unwrap(), text.to_string(), 0)
    }

    #[test]
    fn a_label_used_twice_in_the_same_file_reports_two_references() {
        let analyzer = AssemblyAnalyzer::new();
        let lenses = analyzer.reference_count_code_lenses(&doc(
            "start:\n    call start\n    jp start\n"
        ));
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command.as_ref().unwrap().title, "2 references");
    }

    #[test]
    fn an_unused_label_reports_no_references() {
        let analyzer = AssemblyAnalyzer::new();
        let lenses = analyzer.reference_count_code_lenses(&doc("start:\n    ret\n"));
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command.as_ref().unwrap().title, "no references");
    }

    #[test]
    fn a_reference_in_a_sibling_file_nobody_has_touched_yet_is_not_counted() {
        // Proves the "always cheap" property is real: a sibling file that
        // exists on disk but was never opened, edited, or otherwise indexed
        // is simply unknown - this lens never walks the directory to go
        // find it.
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("sibling.asm"),
            "    call start\n"
        )
        .unwrap();
        let uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let document = Document::new(uri, "start:\n    ret\n".to_string(), 0);

        let analyzer = AssemblyAnalyzer::new();
        let lenses = analyzer.reference_count_code_lenses(&document);
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command.as_ref().unwrap().title, "no references");
    }

    #[test]
    fn a_reference_in_a_directly_included_file_is_counted() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("caller.asm"),
            "    call start\n"
        )
        .unwrap();
        let uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let document = Document::new(
            uri,
            "include \"caller.asm\"\nstart:\n    ret\n".to_string(),
            0
        );

        let analyzer = AssemblyAnalyzer::new();
        let lenses = analyzer.reference_count_code_lenses(&document);
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command.as_ref().unwrap().title, "1 reference");
    }

    #[test]
    fn a_reference_in_a_file_the_session_already_knows_about_is_counted_workspace_wide() {
        // The extension-parity case: once some other file has been indexed
        // by *anything* (here, simulating a prior `did_open`/edit/workspace
        // scan by calling `ensure_label_facts` directly), this lens must
        // pick it up too - even though it's neither the current document
        // nor one of its own includes.
        let tmp = camino_tempfile::tempdir().unwrap();
        let other_uri = Url::from_file_path(tmp.path().join("other.asm")).unwrap();
        let other = Document::new(other_uri, "    call start\n".to_string(), 1);

        let analyzer = AssemblyAnalyzer::new();
        analyzer.ensure_label_facts(&other);

        let main_uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
        let document = Document::new(main_uri, "start:\n    ret\n".to_string(), 0);
        let lenses = analyzer.reference_count_code_lenses(&document);
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command.as_ref().unwrap().title, "1 reference");
    }

    #[test]
    fn a_local_label_gets_its_own_lens_counting_its_bare_in_scope_reference() {
        // The user's own example: `global1` / `.local1`, no colons, `.local1`
        // used bare within `global1`'s own scope.
        let analyzer = AssemblyAnalyzer::new();
        let document = doc("global1\n.local1\n    call .local1\n    ret\n");
        let lenses = analyzer.reference_count_code_lenses(&document);
        assert_eq!(lenses.len(), 2, "{lenses:?}");
        let local_lens = lenses
            .iter()
            .find(|l| l.range.start.line == 1)
            .unwrap_or_else(|| panic!("no lens on .local1's own definition line: {lenses:?}"));
        assert_eq!(local_lens.command.as_ref().unwrap().title, "1 reference");
    }

    #[test]
    fn a_local_labels_qualified_reference_from_another_scope_is_counted_too() {
        // `global1.local1` used from *outside* `global1`'s own scope - the
        // form a same-named `.local1` under a different global can never be
        // mistaken for.
        let analyzer = AssemblyAnalyzer::new();
        let document = doc(
            "global1\n.local1\n    ret\nglobal2\n    call global1.local1\n    ret\n"
        );
        let lenses = analyzer.reference_count_code_lenses(&document);
        let local_lens = lenses
            .iter()
            .find(|l| l.range.start.line == 1)
            .unwrap_or_else(|| panic!("no lens on .local1's own definition line: {lenses:?}"));
        assert_eq!(local_lens.command.as_ref().unwrap().title, "1 reference");
    }

    #[test]
    fn two_same_named_locals_under_different_globals_are_counted_independently() {
        // `global1`'s own `.local1` and `global2`'s own `.local1` are
        // different symbols - a reference to one must never inflate the
        // other's count.
        let analyzer = AssemblyAnalyzer::new();
        let document = doc(
            "global1\n.local1\n    call .local1\n    call .local1\nglobal2\n.local1\n    call .local1\n    ret\n"
        );
        let lenses = analyzer.reference_count_code_lenses(&document);
        let global1_local = lenses
            .iter()
            .find(|l| l.range.start.line == 1)
            .unwrap_or_else(|| panic!("no lens for global1's .local1: {lenses:?}"));
        let global2_local = lenses
            .iter()
            .find(|l| l.range.start.line == 5)
            .unwrap_or_else(|| panic!("no lens for global2's .local1: {lenses:?}"));
        assert_eq!(global1_local.command.as_ref().unwrap().title, "2 references");
        assert_eq!(global2_local.command.as_ref().unwrap().title, "1 reference");
    }
}

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use dashmap::DashMap;
use rayon::prelude::*;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::basm::AssemblyAnalyzer;
use crate::bndbuild::BuildFileAnalyzer;
use crate::bndbuild::call_hierarchy::CallHierarchyCandidate;
use crate::common::call_hierarchy::CallHierarchyData;
use crate::common::document::{Document, DocumentType};
use crate::locomotive::BasicAnalyzer;

/// How long `did_change` waits for edits to stop arriving before actually
/// re-analyzing and publishing diagnostics - matches the VS Code
/// extension's own existing client-side debounce for color-swatch refresh
/// (`cpclib-vscode/src/extension.ts`, 300ms), the established precedent for
/// "the right timescale for this app."
const DID_CHANGE_DEBOUNCE: Duration = Duration::from_millis(250);

pub struct CpcLspBackend {
    client: Client,
    documents: Arc<DashMap<Url, Document>>,
    asm_analyzer: Arc<AssemblyAnalyzer>,
    build_analyzer: Arc<BuildFileAnalyzer>,
    basic_analyzer: Arc<BasicAnalyzer>,
    /// The latest version `did_change` has requested analysis for, per URI -
    /// lets a debounced re-analysis task scheduled by an *older* edit detect
    /// "a newer edit has since arrived, I'm stale" and no-op instead of
    /// publishing outdated diagnostics over newer ones.
    pending_versions: Arc<DashMap<Url, i32>>,
    /// Workspace folder(s) reported at `initialize`, used to bound the eager
    /// cross-file goto-definition fallback (`find_definition_via_workspace_scan`).
    /// Empty when the client sent neither `workspaceFolders` nor `rootUri`
    /// (e.g. single-file mode) - the fallback then derives a root from the
    /// document itself, see `basm::definition::project_root_for`.
    workspace_roots: RwLock<Vec<PathBuf>>,
    /// A bndbuild rule failure's diagnostic for a *referenced source file*
    /// (see `bndbuild::command::RuleRunOutcome::build_error`), keyed by that
    /// file's own URI. Deliberately *not* version-gated like `parse_cache`/
    /// `symbols_cache` - it must specifically survive `did_change`/`did_save`
    /// of its own target URI (a build error persists until the next build,
    /// unlike a live parse error, which clears on re-edit); `compute_diagnostics`
    /// merges an entry in here into every analyze/publish cycle for that URI
    /// until `execute_command`'s `"cpclib.runRule"` handler clears it (on the
    /// next successful run) or overwrites it (on the next failing one).
    build_error_diagnostics: Arc<DashMap<Url, Diagnostic>>
}

impl CpcLspBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DashMap::new()),
            asm_analyzer: Arc::new(AssemblyAnalyzer::new()),
            build_analyzer: Arc::new(BuildFileAnalyzer::new()),
            basic_analyzer: Arc::new(BasicAnalyzer::new()),
            pending_versions: Arc::new(DashMap::new()),
            workspace_roots: RwLock::new(Vec::new()),
            build_error_diagnostics: Arc::new(DashMap::new())
        }
    }

    async fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn analyze_document(&self, document: &Document) {
        let diagnostics = compute_diagnostics(
            &self.asm_analyzer,
            &self.build_analyzer,
            &self.basic_analyzer,
            document,
            &self.workspace_roots(),
            &self.build_error_diagnostics
        );
        self.publish_diagnostics(document.uri.clone(), diagnostics)
            .await;
    }

    /// Symbol defined in a file explicitly `INCLUDE`/`INCBIN`/`BINCLUDE`d by
    /// `document_text` (the document at `from_uri`), even when that file was
    /// never opened by the editor. Prefers the already-open in-memory
    /// version when there is one (it may have unsaved changes), otherwise
    /// reads straight from disk.
    fn find_definition_via_includes(
        &self,
        document_text: &str,
        from_uri: &Url,
        word_upper: &str
    ) -> Option<Location> {
        for filename in crate::basm::definition::extract_include_filenames(document_text) {
            let Some(path) = crate::basm::definition::resolve_include_path(&filename, from_uri)
            else {
                continue;
            };
            if let Some(loc) = self.find_definition_at_path(&path, word_upper) {
                return Some(loc);
            }
        }
        None
    }

    /// Symbol defined in any `.asm` file under the workspace root(s) (or, if
    /// the client reported none, the nearest project-root ancestor of
    /// `from_uri`), even when never opened by the editor. `.git`/`target`/
    /// etc. are pruned from the walk.
    ///
    /// The directory walk itself runs on a blocking-pool thread (see
    /// `candidate_asm_paths`), but candidate files are then searched in
    /// parallel via rayon's `find_map_any`, since parsing+scanning each file
    /// is the actually expensive part on a large workspace. This changes
    /// "first match wins" from strict directory-walk order to "first match
    /// found across parallel workers" - an acceptable difference, since a
    /// symbol should only have one real definition.
    async fn find_definition_via_workspace_scan(
        &self,
        from_uri: &Url,
        word_upper: &str
    ) -> Option<Location> {
        let paths = self.candidate_asm_paths(from_uri).await;
        paths
            .par_iter()
            .find_map_any(|path| self.find_definition_at_path(path, word_upper))
    }

    /// The configured workspace root(s), or an empty `Vec` if the client
    /// reported none. Uses `.unwrap_or_else(|e| e.into_inner())` rather than
    /// `.unwrap()` so a panic while any thread holds this lock can't poison
    /// it for the rest of the server's lifetime - `RwLock` poisoning never
    /// clears on its own, and every goto-definition/rename/call-hierarchy
    /// workspace fallback reads this lock.
    fn workspace_roots(&self) -> Vec<PathBuf> {
        self.workspace_roots
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Every `.asm` file under the workspace root(s) (or, if the client
    /// reported none, the nearest project-root ancestor of `from_uri`),
    /// excluding `from_uri` itself and pruning `.git`/`target`/etc. - the
    /// candidate-file list shared by both `find_definition_via_workspace_scan`
    /// and `rename_label_across_workspace`.
    ///
    /// The walk itself runs on a blocking-pool thread via `spawn_blocking`,
    /// mirroring how `execute_command`'s `cpclib.runRule` already offloads
    /// its own heavy synchronous work - a large workspace's directory walk
    /// (filesystem metadata for every entry under every root) can take long
    /// enough to noticeably stall the async runtime's worker threads, which
    /// `tower-lsp` shares across every concurrent request (hover,
    /// completion, ...), not just this one.
    async fn candidate_asm_paths(&self, from_uri: &Url) -> Vec<PathBuf> {
        let configured = self.workspace_roots();
        let roots: Vec<PathBuf> = if configured.is_empty() {
            crate::basm::definition::project_root_for(from_uri)
                .into_iter()
                .collect()
        }
        else {
            configured
        };

        let from_path = from_uri.to_file_path().ok();

        tokio::task::spawn_blocking(move || {
            let mut paths = Vec::new();
            for root in roots {
                let walker = walkdir::WalkDir::new(&root)
                    .into_iter()
                    .filter_entry(|e| !is_ignored_dir(e));
                for entry in walker.filter_map(|e| e.ok()) {
                    if !entry.file_type().is_file() {
                        continue;
                    }
                    let path = entry.path();
                    let is_asm = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("asm"));
                    if !is_asm || from_path.as_deref() == Some(path) {
                        continue;
                    }
                    paths.push(path.to_path_buf());
                }
            }
            paths
        })
        .await
        .unwrap_or_default()
    }

    /// Look up `uri` among open documents; fall back to reading it from disk
    /// (as a synthetic document versioned by the file's own mtime - see
    /// `disk_file_version`) if it isn't open. `None` only when the URI isn't
    /// a file path or the file can't be read. The "open doc, else disk"
    /// shape this replaces was independently duplicated 9 times across this
    /// file's cross-file features (goto-definition/rename workspace
    /// fallbacks, call-hierarchy include-graph traversal, and several
    /// `executeCommand` handlers).
    fn load_document(&self, uri: &Url) -> Option<Document> {
        if let Some(entry) = self.documents.get(uri) {
            return Some(entry.value().clone());
        }
        let path = uri.to_file_path().ok()?;
        let text = std::fs::read_to_string(&path).ok()?;
        Some(Document::new(uri.clone(), text, disk_file_version(&path)))
    }

    /// Shared by both cross-file fallbacks: look up `word` (case-sensitively
    /// - basm labels are case-sensitive by default, see
    /// `AssemblyAnalyzer::find_definition_in`'s own doc comment) in the file
    /// at `path`, using the already-open in-memory document if there is one.
    fn find_definition_at_path(&self, path: &std::path::Path, word: &str) -> Option<Location> {
        let target_uri = Url::from_file_path(path).ok()?;
        let doc = self.load_document(&target_uri)?;
        self.asm_analyzer.find_definition_in(&doc, word, true)
    }

    /// Workspace-wide rename of a `Global` basm label: unlike
    /// `find_definition_via_workspace_scan` (stops at the first match),
    /// this collects edits from *every* matching file — `.asm` files this
    /// document itself `include`s, plus every `.asm` file under
    /// `workspace_roots` (open or on disk). Inserts into `changes`,
    /// skipping any URI already present (the current document's own edits,
    /// added by the caller before this runs).
    async fn rename_label_across_workspace(
        &self,
        from_uri: &Url,
        document_text: &str,
        target: &crate::basm::definition::RenameTarget,
        new_name: &str,
        changes: &mut std::collections::HashMap<Url, Vec<TextEdit>>
    ) {
        for filename in crate::basm::definition::extract_include_filenames(document_text) {
            if let Some(path) = crate::basm::definition::resolve_include_path(&filename, from_uri) {
                self.rename_label_at_path(&path, target, new_name, changes);
            }
        }

        // The workspace-wide part (unlike the small include list above) is
        // where parallelizing actually matters: compute every candidate
        // file's edits independently via rayon (no shared mutable state
        // during the parallel phase), then merge sequentially, preserving
        // today's "skip a URI already present" semantics via `or_insert`.
        let paths = self.candidate_asm_paths(from_uri).await;
        let results: Vec<(Url, Vec<TextEdit>)> = paths
            .par_iter()
            .filter_map(|path| self.rename_edits_at_path(path, target, new_name))
            .collect();
        for (uri, edits) in results {
            changes.entry(uri).or_insert(edits);
        }
    }

    /// Shared by `rename_label_across_workspace`'s two sources of candidate
    /// files: compute `target`'s rename edits for the file at `path` (the
    /// already-open in-memory version if there is one, else read from
    /// disk), inserting into `changes` if non-empty and not already present.
    fn rename_label_at_path(
        &self,
        path: &std::path::Path,
        target: &crate::basm::definition::RenameTarget,
        new_name: &str,
        changes: &mut std::collections::HashMap<Url, Vec<TextEdit>>
    ) {
        if let Some((uri, edits)) = self.rename_edits_at_path(path, target, new_name) {
            changes.entry(uri).or_insert(edits);
        }
    }

    /// Compute `target`'s rename edits for the file at `path` (the
    /// already-open in-memory version if there is one, else read from
    /// disk), or `None` if the path doesn't resolve to a URI, can't be read,
    /// or yields no edits. Stateless (no shared `changes` map) so it's safe
    /// to call from parallel workers.
    fn rename_edits_at_path(
        &self,
        path: &std::path::Path,
        target: &crate::basm::definition::RenameTarget,
        new_name: &str
    ) -> Option<(Url, Vec<TextEdit>)> {
        let target_uri = Url::from_file_path(path).ok()?;
        let doc = self.load_document(&target_uri)?;
        let edits = self
            .asm_analyzer
            .rename_occurrences_in(&doc, target, new_name);

        (!edits.is_empty()).then_some((target_uri, edits))
    }

    /// Workspace-wide, all-or-nothing removal of an unused MACRO/FUNCTION
    /// parameter: every call site (the current document itself, its own
    /// direct `INCLUDE`s, and every other `.asm` file under the workspace
    /// root - mirrors `rename_label_across_workspace`'s own three-source
    /// discovery shape) plus the definition's own header, rewritten as one
    /// atomic multi-file edit - or, if even one call site anywhere couldn't
    /// be confidently found and safely rewritten (or the name turned out to
    /// be ambiguous), nothing at all, with every blocker reported so the
    /// user knows exactly what to check manually. Unlike rename (where a
    /// missed occurrence is merely a stale name), a missed call site here
    /// would break that call's own arity - see `basm::remove_parameter`'s
    /// own module doc comment for why this can never be a partial edit.
    ///
    /// Unlike `rename_label_across_workspace`, every candidate file is
    /// *parsed*, not just text-scanned - real work, wrapped in
    /// `spawn_blocking` (off the async runtime's own worker threads) with
    /// the same rayon-parallel-then-sequential-merge shape.
    async fn remove_unused_parameter_across_workspace(
        &self,
        from_uri: &Url,
        from_document: &Document,
        target: &crate::basm::remove_parameter::RemoveParameterTarget
    ) -> RemoveParameterOutcome {
        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        let mut blockers: Vec<crate::basm::remove_parameter::RemovalBlocker> = Vec::new();
        let mut total_matching_definitions = 0usize;
        let mut seen: std::collections::HashSet<Url> = std::collections::HashSet::new();

        {
            let mut record = |uri: Url, scan: crate::basm::remove_parameter::FileScan| {
                total_matching_definitions += scan.matching_definitions.len();
                if !scan.edits.is_empty() {
                    changes.insert(uri.clone(), scan.edits);
                }
                for mut blocker in scan.blockers {
                    blocker.uri.get_or_insert_with(|| uri.clone());
                    blockers.push(blocker);
                }
            };

            seen.insert(from_uri.clone());
            let from_scan = self
                .asm_analyzer
                .scan_document_for_parameter_removal(from_document, target);
            record(from_uri.clone(), from_scan);

            for filename in
                crate::basm::definition::extract_include_filenames(&from_document.text())
            {
                if let Some(path) =
                    crate::basm::definition::resolve_include_path(&filename, from_uri)
                    && let Ok(uri) = Url::from_file_path(&path)
                    && seen.insert(uri.clone())
                    && let Some(doc) = self.load_document(&uri)
                {
                    let scan = self
                        .asm_analyzer
                        .scan_document_for_parameter_removal(&doc, target);
                    record(uri, scan);
                }
            }
        }

        let paths = self.candidate_asm_paths(from_uri).await;
        let candidate_docs: Vec<(Url, Document)> = paths
            .into_iter()
            .filter_map(|path| {
                let uri = Url::from_file_path(&path).ok()?;
                if seen.contains(&uri) {
                    return None;
                }
                let doc = self.load_document(&uri)?;
                Some((uri, doc))
            })
            .collect();

        let analyzer = self.asm_analyzer.clone();
        let target_owned = target.clone();
        let results: Vec<(Url, crate::basm::remove_parameter::FileScan)> =
            tokio::task::spawn_blocking(move || {
                candidate_docs
                    .into_par_iter()
                    .map(|(uri, doc)| {
                        let scan =
                            analyzer.scan_document_for_parameter_removal(&doc, &target_owned);
                        (uri, scan)
                    })
                    .collect()
            })
            .await
            .unwrap_or_default();

        for (uri, scan) in results {
            total_matching_definitions += scan.matching_definitions.len();
            if !scan.edits.is_empty() {
                changes.insert(uri.clone(), scan.edits);
            }
            for mut blocker in scan.blockers {
                blocker.uri.get_or_insert_with(|| uri.clone());
                blockers.push(blocker);
            }
        }

        if total_matching_definitions > 1 {
            blockers.push(crate::basm::remove_parameter::RemovalBlocker {
                uri: None,
                line: None,
                reason: format!(
                    "'{}' is ambiguous - more than one MACRO/FUNCTION/STRUCT definition with \
                     this name was found across the workspace, so it isn't safe to guess which \
                     one every call site refers to",
                    target.owner_name
                )
            });
        }

        if !blockers.is_empty() {
            return RemoveParameterOutcome::Blocked(blockers);
        }

        match non_empty_workspace_edit(changes) {
            Some(edit) => RemoveParameterOutcome::Ready(edit),
            None => {
                RemoveParameterOutcome::Blocked(vec![
                    crate::basm::remove_parameter::RemovalBlocker {
                        uri: None,
                        line: None,
                        reason: format!(
                            "no call sites or definition were found for '{}' - nothing to remove",
                            target.owner_name
                        )
                    },
                ])
            },
        }
    }

    /// Workspace-wide rename of a bndbuild Jinja variable: every file that
    /// transitively `{% include %}`s the document containing the `{% set %}`
    /// definition being renamed — e.g. renaming `CPCIP` in `common.build`
    /// must also reach every `build.bnd` that (directly or indirectly)
    /// `{% include %}`s it.
    fn rename_jinja_variable_across_workspace(
        &self,
        from_uri: &Url,
        document_text: &str,
        position: Position,
        new_name: &str,
        changes: &mut std::collections::HashMap<Url, Vec<TextEdit>>
    ) {
        let Some(line) = document_text.lines().nth(position.line as usize)
        else {
            return;
        };
        let col =
            crate::common::document::utf16_col_to_byte_offset(line, position.character as usize);
        let Some((word, ..)) = crate::bndbuild::definition::jinja_word_at(line, col)
        else {
            return;
        };
        // Only a *definition* site's own file is a graph root — renaming
        // from a mere usage site only ever needs the current document
        // (already handled by the caller), since other files can only be
        // reached transitively from the file that actually defines it.
        if !crate::bndbuild::jinja::collect_jinja_variables(&Document::new(
            from_uri.clone(),
            document_text.to_string(),
            0
        ))
        .iter()
        .any(|(name, ..)| *name == word)
        {
            return;
        }
        let Some(from_path) = from_uri.to_file_path().ok()
        else {
            return;
        };

        let roots: Vec<PathBuf> = self.workspace_roots();
        let includers =
            crate::bndbuild::definition::files_transitively_including(&roots, &from_path);

        for path in includers {
            let Some(target_uri) = Url::from_file_path(&path).ok()
            else {
                continue;
            };
            if changes.contains_key(&target_uri) {
                continue;
            }
            let Some(doc) = self.load_document(&target_uri)
            else {
                continue;
            };
            let edits: Vec<TextEdit> = self
                .build_analyzer
                .find_word_references(&doc, &word)
                .into_iter()
                .map(|loc| {
                    TextEdit {
                        range: loc.range,
                        new_text: new_name.to_string()
                    }
                })
                .collect();
            if !edits.is_empty() {
                changes.insert(target_uri, edits);
            }
        }
    }

    /// `current` plus every bndbuild file that transitively `{% include %}`s
    /// the document at `item_uri` — the set of documents whose own scan
    /// could contain an incoming call to something defined there. Mirrors
    /// `rename_jinja_variable_across_workspace`'s load-each-file-and-scan
    /// shape exactly.
    fn bndbuild_incoming_candidate_docs(
        &self,
        current: &Document,
        item_uri: &Url
    ) -> Vec<Document> {
        let mut docs = vec![current.clone()];
        let Some(path) = item_uri.to_file_path().ok()
        else {
            return docs;
        };
        let roots: Vec<PathBuf> = self.workspace_roots();
        for includer_path in
            crate::bndbuild::definition::files_transitively_including(&roots, &path)
        {
            let Some(includer_uri) = Url::from_file_path(&includer_path).ok()
            else {
                continue;
            };
            if let Some(doc) = self.load_document(&includer_uri) {
                docs.push(doc);
            }
        }
        docs
    }

    /// Resolve `name` to a `CallHierarchyItem` via `resolve`, trying
    /// `current` first, then every bndbuild file transitively
    /// `{% include %}`d by the document at `item_uri` (where a
    /// shared/common definition typically lives, e.g. macros in a
    /// project's `common.build`).
    fn resolve_bndbuild_item(
        &self,
        current: &Document,
        item_uri: &Url,
        name: &str,
        resolve: impl Fn(&Document, &str) -> Option<CallHierarchyItem>
    ) -> Option<CallHierarchyItem> {
        if let Some(item) = resolve(current, name) {
            return Some(item);
        }
        let path = item_uri.to_file_path().ok()?;
        let roots: Vec<PathBuf> = self.workspace_roots();
        for included_path in
            crate::bndbuild::definition::files_transitively_included_by(&roots, &path)
        {
            let Some(included_uri) = Url::from_file_path(&included_path).ok()
            else {
                continue;
            };
            let Some(doc) = self.load_document(&included_uri)
            else {
                continue;
            };
            if let Some(item) = resolve(&doc, name) {
                return Some(item);
            }
        }
        None
    }

    /// Resolves a `CallHierarchyItem.uri` back to its document for
    /// `incoming_calls`/`outgoing_calls`. The item may name a document that
    /// isn't open in the editor (e.g. a shared bndbuild file pulled in only
    /// via `resolve_bndbuild_item`'s include-graph walk), so this falls
    /// back to reading it from disk - same "open doc, else read from disk"
    /// shape as `bndbuild_incoming_candidate_docs`.
    fn document_for_call_hierarchy_item(
        &self,
        uri: &Url,
        _data: &CallHierarchyData
    ) -> Option<(DocumentType, Document)> {
        let document = self.load_document(uri)?;
        Some((document.doc_type, document))
    }

    /// Fallback for `prepare_call_hierarchy` when the cursor names a
    /// target/dependency or macro call site that isn't defined in this
    /// document itself - e.g. a macro call whose `{% macro %}` definition
    /// lives only in an `{% include %}`d file. Extracts what the cursor is
    /// pointing at via `call_hierarchy_candidate_at`, then retries
    /// resolution across the include graph the same way outgoing-call
    /// resolution already does.
    fn bndbuild_cross_file_prepare(
        &self,
        document: &Document,
        uri: &Url,
        position: Position
    ) -> Option<CallHierarchyItem> {
        match self
            .build_analyzer
            .call_hierarchy_candidate_at(document, position)?
        {
            CallHierarchyCandidate::Target(name) => {
                self.resolve_bndbuild_item(document, uri, &name, |doc, n| {
                    self.build_analyzer.call_hierarchy_item_for_target(doc, n)
                })
            },
            CallHierarchyCandidate::Macro(name) => {
                self.resolve_bndbuild_item(document, uri, &name, |doc, n| {
                    self.build_analyzer.call_hierarchy_item_for_macro(doc, n)
                })
            },
        }
    }
}

/// Dispatches `document` to whichever analyzer its `doc_type` owns, then
/// relativizes every diagnostic's message against `workspace_roots` and
/// merges in a sticky `build_error_diagnostics` entry for this URI, if any
/// (see that field's own doc comment on `CpcLspBackend`). A free function
/// (not a `&self` method) so it's callable from `did_change`'s debounced
/// `tokio::spawn` task, which only holds the individually `Arc`-cloned
/// analyzers/maps (and an owned `workspace_roots` snapshot taken before
/// spawning), not a full `&CpcLspBackend`.
fn compute_diagnostics(
    asm_analyzer: &AssemblyAnalyzer,
    build_analyzer: &BuildFileAnalyzer,
    basic_analyzer: &BasicAnalyzer,
    document: &Document,
    workspace_roots: &[PathBuf],
    build_error_diagnostics: &DashMap<Url, Diagnostic>
) -> Vec<Diagnostic> {
    let mut diagnostics = match document.doc_type {
        DocumentType::Assembly => asm_analyzer.analyze(document),
        DocumentType::BuildFile => build_analyzer.analyze(document),
        DocumentType::Basic => basic_analyzer.analyze(document),
        DocumentType::CatartBasic => {
            let mut d = basic_analyzer.analyze(document);
            d.extend(basic_analyzer.catart_diagnostics(document));
            d
        },
        DocumentType::Unknown => Vec::new()
    };
    relativize_diagnostic_messages(&mut diagnostics, workspace_roots);
    if let Some(build_error) = build_error_diagnostics.get(&document.uri) {
        diagnostics.push(build_error.clone());
    }
    diagnostics
}

/// Rewrites every absolute path under one of `workspace_roots` that appears
/// in `diagnostics`' messages to be relative to that root - e.g.
/// `/home/x/project/src/sna.asm:24:5` becomes `src/sna.asm:24:5` when
/// `/home/x/project` is a configured root. `AssemblerError`'s `Display`
/// (rendered via `codespan-reporting`, see `collect_asm_diagnostics`) embeds
/// the parser's `current_filename` - necessarily absolute, since that's what
/// `parse_source` hands it, built from the document's real on-disk path -
/// which is noisy in a workspace (and can be genuinely long for a
/// deeply-nested project). Only the message *text* is touched here; a
/// `Diagnostic`'s own `range` and the URI it's published under (what the
/// editor actually uses to place the squiggle and jump to it) are untouched
/// - this only affects what a human reads in the Problems panel/hover
/// tooltip. A no-op with no configured root, or when a message contains no
/// root's path at all (e.g. an error inside an `inner://` firmware asset).
fn relativize_diagnostic_messages(diagnostics: &mut [Diagnostic], workspace_roots: &[PathBuf]) {
    let prefixes: Vec<String> = workspace_roots
        .iter()
        .filter_map(|r| r.to_str())
        .map(|r| format!("{r}{}", std::path::MAIN_SEPARATOR))
        .collect();
    if prefixes.is_empty() {
        return;
    }
    for diag in diagnostics.iter_mut() {
        for prefix in &prefixes {
            if diag.message.contains(prefix.as_str()) {
                diag.message = diag.message.replace(prefix.as_str(), "");
            }
        }
    }
}

/// Dispatch by `document.doc_type` to the matching analyzer, returning
/// `unknown_default` for `DocumentType::Unknown`. Shared by the handlers
/// whose per-arm bodies are all `self.X_analyzer.method(document, ...)`
/// with no other per-arm logic (`hover`, `prepare_rename`,
/// `document_symbol`, `semantic_tokens_full`). Most other doc_type
/// dispatches in this file have real per-arm differences (extra fallback
/// logic, differing argument counts, a different match key entirely) and
/// are deliberately not routed through this — forcing them through a
/// shared shape would obscure real behavioral differences rather than
/// remove genuine duplication.
fn dispatch_by_doc_type<T>(
    document: &Document,
    unknown_default: T,
    on_assembly: impl FnOnce(&Document) -> T,
    on_build_file: impl FnOnce(&Document) -> T,
    on_basic: impl FnOnce(&Document) -> T
) -> T {
    match document.doc_type {
        DocumentType::Assembly => on_assembly(document),
        DocumentType::BuildFile => on_build_file(document),
        DocumentType::Basic | DocumentType::CatartBasic => on_basic(document),
        DocumentType::Unknown => unknown_default
    }
}

/// Outcome of `remove_unused_parameter_across_workspace`: either every call
/// site was confidently found and safely rewritten (`Ready`, carrying the
/// complete multi-file edit), or something, somewhere, blocked the whole
/// operation (`Blocked`) - nothing is ever partially applied.
enum RemoveParameterOutcome {
    Blocked(Vec<crate::basm::remove_parameter::RemovalBlocker>),
    Ready(WorkspaceEdit)
}

/// Reports every blocker from a `RemoveParameterOutcome::Blocked` to the
/// client: one `file:line - reason` line per blocker (or `- reason` for a
/// workspace-wide one with no location) via `log_message` - detail suited
/// to the output channel, since there can be several - followed by a single
/// summary `show_message(ERROR, ...)` pointing the user at it. Mirrors
/// `cpclib.runRule`'s own established "stream detail via log_message,
/// summarize via show_message" shape.
async fn report_removal_blockers(
    client: &Client,
    owner_name: &str,
    blockers: &[crate::basm::remove_parameter::RemovalBlocker]
) {
    for blocker in blockers {
        let location = blocker.uri.as_ref().map(|uri| {
            uri.to_file_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| uri.to_string())
        });
        let line = match (location, blocker.line) {
            (Some(path), Some(l)) => format!("{path}:{l} - {}", blocker.reason),
            (Some(path), None) => format!("{path} - {}", blocker.reason),
            (None, _) => format!("- {}", blocker.reason)
        };
        client.log_message(MessageType::WARNING, line).await;
    }
    client
        .show_message(
            MessageType::ERROR,
            format!(
                "Couldn't safely remove the parameter from '{owner_name}': {} issue(s) found - \
                 see the output panel for details.",
                blockers.len()
            )
        )
        .await;
}

/// `None` when `changes` is empty (nothing to rename — the client should
/// see "no changes" rather than an edit touching zero files).
fn non_empty_workspace_edit(
    changes: std::collections::HashMap<Url, Vec<TextEdit>>
) -> Option<WorkspaceEdit> {
    if changes.is_empty() {
        None
    }
    else {
        Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        })
    }
}

/// Directories never worth descending into while scanning the workspace for
/// `.asm` files: VCS metadata and build output can be huge and are never
/// where hand-written assembly sources live.
pub(crate) fn is_ignored_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        && matches!(
            entry.file_name().to_str(),
            Some(".git" | ".hg" | ".svn" | "target" | "node_modules")
        )
}

/// A version for a not-open-in-the-editor document, derived from the
/// file's own mtime rather than a fixed `0` - `parse_document`'s cache is
/// keyed on `(uri, version)`, and a fixed version would serve a stale
/// cached parse forever for a file that changes on disk without ever being
/// opened in the editor (no `didClose` ever fires to `evict()` it).
/// Truncating a Unix timestamp to `i32` wraps every ~136 years - harmless
/// here, since all that matters is "did this differ from the last read",
/// not an accurate absolute time. Falls back to 0 (the previous fixed
/// behavior) if the mtime can't be read.
fn disk_file_version(path: &std::path::Path) -> i32 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0)
}

#[tower_lsp::async_trait]
impl LanguageServer for CpcLspBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        tracing::info!("Initializing cpclib-lsp server");
        tracing::info!("Client capabilities: {:?}", params.capabilities);

        // Record the workspace root(s) for the eager cross-file
        // goto-definition fallback. `workspaceFolders` is the modern,
        // possibly-multi-root source; `rootUri` is the deprecated
        // single-root predecessor some clients still send instead.
        let mut roots: Vec<PathBuf> = params
            .workspace_folders
            .iter()
            .flatten()
            .filter_map(|f| f.uri.to_file_path().ok())
            .collect();
        if roots.is_empty() {
            #[allow(deprecated)]
            let root_uri = params.root_uri.as_ref();
            if let Some(path) = root_uri.and_then(|u| u.to_file_path().ok()) {
                roots.push(path);
            }
        }
        *self
            .workspace_roots
            .write()
            .unwrap_or_else(|e| e.into_inner()) = roots;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false)
                        }))
                    }
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        "#".to_string(),
                        "%".to_string(),
                        "$".to_string(),
                        "{".to_string(),
                    ]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                    completion_item: None
                }),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false)
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "cpclib.getTargets".to_string(),
                        "cpclib.selectRange".to_string(),
                        "cpclib.runRule".to_string(),
                        "cpclib.cycleCountForSelection".to_string(),
                        "cpclib.removeUnusedParameter".to_string(),
                    ],
                    work_done_progress_options: WorkDoneProgressOptions::default()
                }),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
                    first_trigger_character: "\n".to_string(),
                    more_trigger_character: None
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::REFACTOR_REWRITE,
                            CodeActionKind::REFACTOR_EXTRACT,
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::EMPTY,
                        ]),
                        work_done_progress_options: Default::default(),
                        resolve_provider: Some(false)
                    }
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                            legend: crate::basm::semantic_tokens_legend(),
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true))
                        }
                    )
                ),
                color_provider: Some(ColorProviderCapability::Simple(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default()
                })),
                call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "cpclib-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string())
            })
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Server initialized");
        self.client
            .log_message(MessageType::INFO, "cpclib-lsp server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("Server shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        tracing::info!("Document opened: {}", params.text_document.uri);

        let document = Document::new_with_language(
            params.text_document.uri.clone(),
            params.text_document.text,
            params.text_document.version,
            Some(params.text_document.language_id.as_str())
        );

        self.analyze_document(&document).await;
        self.documents.insert(params.text_document.uri, document);
    }

    /// Applies the edit immediately (needed right away by hover/completion/
    /// etc.), but defers the actual re-analysis + diagnostics publish by
    /// `DID_CHANGE_DEBOUNCE` — a rapid burst of keystrokes would otherwise
    /// re-run full analysis on every single one. `pending_versions` lets a
    /// task scheduled by an edit that's since been superseded detect that
    /// and no-op, rather than publish stale diagnostics after a newer edit's
    /// own (possibly still-pending) analysis.
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        tracing::info!("Document changed: {}", params.text_document.uri);

        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(mut entry) = self.documents.get_mut(&uri) {
            for change in params.content_changes {
                entry.apply_change(&change, version);
            }
        }
        else {
            return;
        }

        self.pending_versions.insert(uri.clone(), version);

        let client = self.client.clone();
        let documents = Arc::clone(&self.documents);
        let pending_versions = Arc::clone(&self.pending_versions);
        let asm_analyzer = Arc::clone(&self.asm_analyzer);
        let build_analyzer = Arc::clone(&self.build_analyzer);
        let basic_analyzer = Arc::clone(&self.basic_analyzer);
        let workspace_roots = self.workspace_roots();
        let build_error_diagnostics = Arc::clone(&self.build_error_diagnostics);

        tokio::spawn(async move {
            tokio::time::sleep(DID_CHANGE_DEBOUNCE).await;

            // A newer edit arrived while we slept - that edit's own
            // (already-scheduled) task will publish instead.
            if pending_versions.get(&uri).map(|v| *v) != Some(version) {
                return;
            }

            let Some(document) = documents.get(&uri).map(|d| d.value().clone())
            else {
                return; // closed in the meantime
            };
            // Guards a close-then-reopen race: same URI, but the version
            // sequence restarted.
            if document.version != version {
                return;
            }

            let diagnostics = compute_diagnostics(
                &asm_analyzer,
                &build_analyzer,
                &basic_analyzer,
                &document,
                &workspace_roots,
                &build_error_diagnostics
            );
            client.publish_diagnostics(uri, diagnostics, None).await;
        });
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        tracing::info!("Document saved: {}", params.text_document.uri);

        if let Some(entry) = self.documents.get(&params.text_document.uri) {
            let document = entry.value().clone();
            drop(entry);

            self.analyze_document(&document).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        tracing::info!("Document closed: {}", params.text_document.uri);
        self.documents.remove(&params.text_document.uri);
        self.pending_versions.remove(&params.text_document.uri);
        self.basic_analyzer.evict(&params.text_document.uri);
        self.asm_analyzer.evict(&params.text_document.uri);
        self.build_analyzer.evict(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        tracing::debug!("Hover request at {}:{}", uri, position.line);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();

            let hover = dispatch_by_doc_type(
                document,
                None,
                |doc| self.asm_analyzer.hover(doc, position),
                |doc| self.build_analyzer.hover(doc, position),
                |doc| self.basic_analyzer.hover(doc, position)
            );

            return Ok(hover);
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        tracing::debug!("Completion request at {}:{}", uri, position.line);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();

            let completions = match document.doc_type {
                DocumentType::Assembly => {
                    // Labels from the other open assembly files are offered too.
                    let others: Vec<Document> = self
                        .documents
                        .iter()
                        .filter(|e| *e.key() != uri && e.value().doc_type == DocumentType::Assembly)
                        .map(|e| e.value().clone())
                        .collect();
                    self.asm_analyzer
                        .completion_with_documents(document, position, &others)
                },
                DocumentType::BuildFile => self.build_analyzer.completion(document, position),
                DocumentType::Basic | DocumentType::CatartBasic => {
                    self.basic_analyzer.completion(document, position)
                },
                DocumentType::Unknown => Vec::new()
            };

            if !completions.is_empty() {
                return Ok(Some(CompletionResponse::Array(completions)));
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        tracing::debug!("Goto definition request at {}:{}", uri, position.line);

        let Some(entry) = self.documents.get(&uri)
        else {
            return Ok(None);
        };
        let doc_type = entry.value().doc_type;

        // Try the primary document first.
        let location = match doc_type {
            DocumentType::Assembly => self.asm_analyzer.goto_definition(entry.value(), position),
            DocumentType::BuildFile => self.build_analyzer.goto_definition(entry.value(), position),
            DocumentType::Basic | DocumentType::CatartBasic => {
                self.basic_analyzer.goto_definition(entry.value(), position)
            },
            DocumentType::Unknown => None
        };
        if let Some(loc) = location {
            return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
        }

        // For Assembly: if the symbol was not defined locally, search all other
        // open Assembly documents (cross-file navigation). Kept as-typed
        // (not uppercased) - basm labels are case-sensitive by default, and
        // every cross-file lookup below now compares case-sensitively too
        // (see `AssemblyAnalyzer::find_definition_in`'s own doc comment).
        if doc_type == DocumentType::Assembly {
            let word = match self.asm_analyzer.word_at_position(entry.value(), position) {
                Some(w) => w,
                None => return Ok(None)
            };
            let document_text = entry.value().text();
            drop(entry); // release the DashMap read guard before iterating/reading files

            for other in self.documents.iter() {
                if *other.key() == uri {
                    continue;
                }
                if other.value().doc_type != DocumentType::Assembly {
                    continue;
                }
                if let Some(loc) = self
                    .asm_analyzer
                    .find_definition_in(other.value(), &word, true)
                {
                    return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
                }
            }

            // Not found among already-open documents either: the symbol is
            // presumably defined in a file the editor was never told to
            // open. Eagerly try, in order: (1) files this document itself
            // `INCLUDE`s, then (2) any `.asm` file under the workspace -
            // real-world sources are made of many files, most of which are
            // never individually opened, so without this goto-definition
            // would only ever work by accident.
            if let Some(loc) = self.find_definition_via_includes(&document_text, &uri, &word) {
                return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
            }
            if let Some(loc) = self.find_definition_via_workspace_scan(&uri, &word).await {
                return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        tracing::debug!("References request at {}:{}", uri, position.line);

        let Some(entry) = self.documents.get(&uri)
        else {
            return Ok(None);
        };
        let doc_type = entry.value().doc_type;

        if doc_type != DocumentType::Assembly {
            let references = match doc_type {
                DocumentType::BuildFile => {
                    self.build_analyzer.find_references(entry.value(), position)
                },
                DocumentType::Basic | DocumentType::CatartBasic => {
                    self.basic_analyzer.find_references(entry.value(), position)
                },
                _ => Vec::new()
            };
            return if references.is_empty() {
                Ok(None)
            }
            else {
                Ok(Some(references))
            };
        }

        // Assembly: collect references across ALL open Assembly documents.
        let word = match self.asm_analyzer.word_at_position(entry.value(), position) {
            Some(w) => w.to_uppercase(),
            None => return Ok(None)
        };
        drop(entry);

        let mut all_refs: Vec<Location> = Vec::new();
        for doc_entry in self.documents.iter() {
            if doc_entry.value().doc_type != DocumentType::Assembly {
                continue;
            }
            all_refs.extend(
                self.asm_analyzer
                    .find_references_in(doc_entry.value(), &word)
            );
        }

        if all_refs.is_empty() {
            Ok(None)
        }
        else {
            Ok(Some(all_refs))
        }
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;

        let Some(entry) = self.documents.get(&uri)
        else {
            return Ok(None);
        };

        let range = dispatch_by_doc_type(
            entry.value(),
            None,
            |doc| self.asm_analyzer.prepare_rename(doc, position),
            |doc| self.build_analyzer.prepare_rename(doc, position),
            |doc| self.basic_analyzer.prepare_rename(doc, position)
        );

        Ok(range.map(PrepareRenameResponse::Range))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        tracing::debug!(
            "Rename request at {}:{} to '{}'",
            uri,
            position.line,
            new_name
        );

        let Some(entry) = self.documents.get(&uri)
        else {
            return Ok(None);
        };

        match entry.value().doc_type {
            DocumentType::Basic | DocumentType::CatartBasic => {
                Ok(self
                    .basic_analyzer
                    .rename(entry.value(), position, &new_name))
            },

            DocumentType::BuildFile => {
                let document_text = entry.value().text();
                let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
                    std::collections::HashMap::new();
                if let Some(edit) = self
                    .build_analyzer
                    .rename(entry.value(), position, &new_name)
                {
                    if let Some(local_changes) = edit.changes {
                        changes.extend(local_changes);
                    }
                }
                drop(entry);
                if !changes.is_empty() {
                    self.rename_jinja_variable_across_workspace(
                        &uri,
                        &document_text,
                        position,
                        &new_name,
                        &mut changes
                    );
                }
                Ok(non_empty_workspace_edit(changes))
            },

            DocumentType::Assembly => {
                // Determine whether this is workspace-wide (a `Global`
                // label) — if not (a `Local`/`Qualified` label, or the
                // cursor is inside a `LOCOMOTIVE` block), the single-file
                // result from `asm_analyzer.rename` is already complete.
                let target = self
                    .asm_analyzer
                    .resolve_rename_target(entry.value(), position);
                let Some(target @ crate::basm::definition::RenameTarget::Global(_)) = target
                else {
                    return Ok(self.asm_analyzer.rename(entry.value(), position, &new_name));
                };

                let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
                    std::collections::HashMap::new();
                let current_edits =
                    self.asm_analyzer
                        .rename_occurrences_in(entry.value(), &target, &new_name);
                if !current_edits.is_empty() {
                    changes.insert(uri.clone(), current_edits);
                }
                let document_text = entry.value().text();
                drop(entry);

                self.rename_label_across_workspace(
                    &uri,
                    &document_text,
                    &target,
                    &new_name,
                    &mut changes
                )
                .await;

                Ok(non_empty_workspace_edit(changes))
            },

            DocumentType::Unknown => Ok(None)
        }
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(entry) = self.documents.get(&uri)
        else {
            return Ok(None);
        };

        let item = match entry.value().doc_type {
            DocumentType::Assembly => {
                self.asm_analyzer
                    .prepare_call_hierarchy(entry.value(), position)
            },
            DocumentType::Basic | DocumentType::CatartBasic => {
                self.basic_analyzer
                    .prepare_call_hierarchy(entry.value(), position)
            },
            DocumentType::BuildFile => {
                self.build_analyzer
                    .prepare_call_hierarchy(entry.value(), position)
                    .or_else(|| self.bndbuild_cross_file_prepare(entry.value(), &uri, position))
            },
            DocumentType::Unknown => None
        };

        Ok(item.map(|i| vec![i]))
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let item = params.item;
        let Some(data) = item.data.as_ref().and_then(CallHierarchyData::from_json)
        else {
            return Ok(None);
        };
        let Some((doc_type, document)) = self.document_for_call_hierarchy_item(&item.uri, &data)
        else {
            return Ok(None);
        };

        let calls = match (doc_type, data) {
            (DocumentType::Assembly, CallHierarchyData::AsmLabel { name }) => {
                let name_upper = name.to_uppercase();
                let mut calls = Vec::new();
                for doc_entry in self.documents.iter() {
                    if doc_entry.value().doc_type != DocumentType::Assembly {
                        continue;
                    }
                    calls.extend(
                        self.asm_analyzer
                            .incoming_calls_in(doc_entry.value(), &name_upper)
                    );
                }
                calls
            },
            (
                DocumentType::Assembly,
                CallHierarchyData::BasicLine {
                    line_number,
                    block_start_line: Some(start)
                }
            ) => {
                self.asm_analyzer.incoming_calls_for_embedded_basic_line(
                    &document,
                    line_number,
                    start
                )
            },
            (
                DocumentType::Basic | DocumentType::CatartBasic,
                CallHierarchyData::BasicLine {
                    line_number,
                    block_start_line: None
                }
            ) => self.basic_analyzer.incoming_calls(&document, line_number),
            (
                DocumentType::Assembly,
                CallHierarchyData::BndbuildTarget {
                    target,
                    block_start_line: Some(start)
                }
            ) => {
                self.asm_analyzer
                    .incoming_calls_for_embedded_bndbuild_target(&document, &target, start)
            },
            (
                DocumentType::BuildFile,
                CallHierarchyData::BndbuildTarget {
                    target,
                    block_start_line: None
                }
            ) => {
                let docs = self.bndbuild_incoming_candidate_docs(&document, &item.uri);

                let mut calls = Vec::new();
                for doc in &docs {
                    for (caller, ranges) in self.build_analyzer.incoming_calls_in(doc, &target) {
                        if let Some(from) = self
                            .build_analyzer
                            .call_hierarchy_item_for_target(doc, &caller)
                        {
                            calls.push(CallHierarchyIncomingCall {
                                from,
                                from_ranges: ranges
                            });
                        }
                    }
                }
                calls
            },
            (DocumentType::BuildFile, CallHierarchyData::JinjaMacro { name }) => {
                let docs = self.bndbuild_incoming_candidate_docs(&document, &item.uri);

                let mut calls = Vec::new();
                for doc in &docs {
                    calls.extend(self.build_analyzer.incoming_calls_for_macro_in(doc, &name));
                }
                calls
            },
            _ => Vec::new() // stale/mismatched `data` (doc_type changed since prepare)
        };

        Ok(if calls.is_empty() { None } else { Some(calls) })
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let item = params.item;
        let Some(data) = item.data.as_ref().and_then(CallHierarchyData::from_json)
        else {
            return Ok(None);
        };
        let Some((doc_type, document)) = self.document_for_call_hierarchy_item(&item.uri, &data)
        else {
            return Ok(None);
        };

        let calls = match (doc_type, data) {
            (DocumentType::Assembly, CallHierarchyData::AsmLabel { name }) => {
                let name_upper = name.to_uppercase();

                let targets = self
                    .asm_analyzer
                    .outgoing_call_targets(&document, &name_upper);

                // Collected once per request rather than once per target:
                // the loop below used to re-filter `self.documents` (a full
                // DashMap iteration) for every one of a routine's outgoing
                // calls, which gets worse exactly when it matters most (a
                // routine with many calls in a workspace with many open
                // files).
                let other_asm_docs: Vec<Document> = self
                    .documents
                    .iter()
                    .filter(|e| {
                        *e.key() != item.uri && e.value().doc_type == DocumentType::Assembly
                    })
                    .map(|e| e.value().clone())
                    .collect();

                let mut calls = Vec::new();
                for (target, ranges) in targets {
                    let target_upper = target.to_uppercase();
                    // Current document first, then every other open
                    // Assembly document - same "current, then others" shape
                    // as `goto_definition`'s own cross-file fallback (minus
                    // the disk-scan step, per this feature's scoping).
                    let to = self
                        .asm_analyzer
                        .call_hierarchy_item_for_label(&document, &target_upper)
                        .or_else(|| {
                            other_asm_docs.iter().find_map(|doc| {
                                self.asm_analyzer
                                    .call_hierarchy_item_for_label(doc, &target_upper)
                            })
                        });
                    if let Some(to) = to {
                        calls.push(CallHierarchyOutgoingCall {
                            to,
                            from_ranges: ranges
                        });
                    }
                }
                calls
            },
            (
                DocumentType::Assembly,
                CallHierarchyData::BasicLine {
                    line_number,
                    block_start_line: Some(start)
                }
            ) => {
                self.asm_analyzer.outgoing_calls_for_embedded_basic_line(
                    &document,
                    line_number,
                    start
                )
            },
            (
                DocumentType::Basic | DocumentType::CatartBasic,
                CallHierarchyData::BasicLine {
                    line_number,
                    block_start_line: None
                }
            ) => self.basic_analyzer.outgoing_calls(&document, line_number),
            (
                DocumentType::Assembly,
                CallHierarchyData::BndbuildTarget {
                    target,
                    block_start_line: Some(start)
                }
            ) => {
                self.asm_analyzer
                    .outgoing_calls_for_embedded_bndbuild_target(&document, &target, start)
            },
            (
                DocumentType::BuildFile,
                CallHierarchyData::BndbuildTarget {
                    target,
                    block_start_line: None
                }
            ) => {
                let targets = self
                    .build_analyzer
                    .outgoing_call_targets_in(&document, &target);

                let mut calls = Vec::new();
                for (target_name, ranges) in targets {
                    let to = self.resolve_bndbuild_item(
                        &document,
                        &item.uri,
                        &target_name,
                        |doc, name| {
                            self.build_analyzer
                                .call_hierarchy_item_for_target(doc, name)
                        }
                    );
                    if let Some(to) = to {
                        calls.push(CallHierarchyOutgoingCall {
                            to,
                            from_ranges: ranges
                        });
                    }
                }
                calls
            },
            (DocumentType::BuildFile, CallHierarchyData::JinjaMacro { name }) => {
                let targets = self
                    .build_analyzer
                    .outgoing_calls_for_macro_targets_in(&document, &name);

                let mut calls = Vec::new();
                for (callee_name, ranges) in targets {
                    let to =
                        self.resolve_bndbuild_item(&document, &item.uri, &callee_name, |doc, n| {
                            self.build_analyzer.call_hierarchy_item_for_macro(doc, n)
                        });
                    if let Some(to) = to {
                        calls.push(CallHierarchyOutgoingCall {
                            to,
                            from_ranges: ranges
                        });
                    }
                }
                calls
            },
            _ => Vec::new()
        };

        Ok(if calls.is_empty() { None } else { Some(calls) })
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        tracing::debug!("Document symbol request for {}", uri);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();

            let symbols = dispatch_by_doc_type(
                document,
                Vec::new(),
                |doc| self.asm_analyzer.document_symbols(doc),
                |doc| self.build_analyzer.document_symbols(doc),
                |doc| self.basic_analyzer.document_symbols(doc)
            );

            if !symbols.is_empty() {
                return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
            }
        }

        Ok(None)
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;
        tracing::debug!("CodeLens request for {}", uri);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            let lenses = match document.doc_type {
                DocumentType::BuildFile => self.build_analyzer.code_lens(document),
                DocumentType::Assembly => self.asm_analyzer.code_lens(document),
                _ => Vec::new()
            };
            if !lenses.is_empty() {
                return Ok(Some(lenses));
            }
        }
        Ok(None)
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams
    ) -> Result<Option<serde_json::Value>> {
        if params.command == "cpclib.runRule" {
            let mut args = params.arguments.into_iter();
            let rule = args.next().and_then(|v| v.as_str().map(|s| s.to_string()));
            let fname = args.next().and_then(|v| v.as_str().map(|s| s.to_string()));
            let (Some(rule), Some(fname)) = (rule, fname)
            else {
                return Ok(None);
            };
            let Ok(uri) = Url::from_file_path(&fname)
            else {
                return Ok(None);
            };

            // Use the open document when available, else load from disk.
            let Some(document) = self.load_document(&uri)
            else {
                return Ok(None);
            };

            self.client
                .log_message(MessageType::INFO, format!("Building rule '{rule}'..."))
                .await;

            // Stream build output to the client's output channel as it
            // happens, the same way a terminal used to show it. The channel
            // closes on its own once `run_rule` (and its observer) drop `tx`
            // at the end of the blocking task, ending `log_task`.
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let log_task = {
                let client = self.client.clone();
                tokio::spawn(async move {
                    while let Some((is_err, line)) = rx.recv().await {
                        client
                            .log_message(
                                if is_err {
                                    MessageType::ERROR
                                }
                                else {
                                    MessageType::LOG
                                },
                                line
                            )
                            .await;
                    }
                })
            };

            // The build is heavy and synchronous: run it on a worker thread.
            let outcome = {
                let document = document.clone();
                let rule = rule.clone();
                let asm_analyzer = Arc::clone(&self.asm_analyzer);
                tokio::task::spawn_blocking(move || {
                    if document.doc_type == DocumentType::Assembly {
                        let blocks = asm_analyzer.embedded_bndbuild_blocks(&document);
                        let file_dir = document
                            .uri
                            .to_file_path()
                            .ok()
                            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
                        match crate::basm::embedded_bndbuild::find_block_for_rule(
                            &blocks,
                            &rule,
                            file_dir.as_deref()
                        ) {
                            Some(block) => {
                                crate::bndbuild::BuildFileAnalyzer::new().run_embedded_rule(
                                    &document,
                                    &block.yaml_text,
                                    block.yaml_start_line as u32,
                                    &rule,
                                    Some(tx)
                                )
                            },
                            None => {
                                crate::bndbuild::command::RuleRunOutcome {
                                    message: format!(
                                        "No embedded bndbuild rule '{rule}' found in this file"
                                    ),
                                    diagnostics: Vec::new(),
                                    build_error: None,
                                    success: false
                                }
                            },
                        }
                    }
                    else {
                        crate::bndbuild::BuildFileAnalyzer::new().run_rule(
                            &document,
                            &rule,
                            Some(tx)
                        )
                    }
                })
                .await
                .map_err(|e| {
                    tower_lsp::jsonrpc::Error {
                        code: tower_lsp::jsonrpc::ErrorCode::InternalError,
                        message: format!("build task panicked: {e}").into(),
                        data: None
                    }
                })?
            };
            let _ = log_task.await;

            // Static analysis diagnostics + the failure highlight (if any):
            // publishing replaces the previous set, so a successful build
            // clears an earlier failure marker.
            let mut diagnostics = self.build_analyzer.analyze(&document);
            let failed = !outcome.success;
            diagnostics.extend(outcome.diagnostics);
            self.publish_diagnostics(uri, diagnostics).await;

            if failed {
                // A referenced *source* file's own build-error diagnostic
                // (distinct from the bnd-file diagnostic above) - sticky
                // until the next successful build, not just this file's own
                // next edit. Stored, then republished immediately so it's
                // visible without waiting for that.
                if let Some((target_uri, diagnostic)) = outcome.build_error {
                    self.build_error_diagnostics
                        .insert(target_uri.clone(), diagnostic);
                    if let Some(target_document) = self.load_document(&target_uri) {
                        self.analyze_document(&target_document).await;
                    }
                }
            }
            else {
                // Every previously-stored build error is now stale - clear
                // them all and republish so the clearing is reflected right
                // away rather than only on each affected file's next
                // unrelated edit.
                let stale: Vec<Url> = self
                    .build_error_diagnostics
                    .iter()
                    .map(|entry| entry.key().clone())
                    .collect();
                self.build_error_diagnostics.clear();
                for stale_uri in stale {
                    if let Some(stale_document) = self.load_document(&stale_uri) {
                        self.analyze_document(&stale_document).await;
                    }
                }
            }

            self.client
                .show_message(
                    if failed {
                        MessageType::ERROR
                    }
                    else {
                        MessageType::INFO
                    },
                    &outcome.message
                )
                .await;
            return Ok(None);
        }

        if params.command == "cpclib.selectRange" {
            if let Some(arg) = params.arguments.into_iter().next() {
                let uri = arg
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<Url>().ok());
                let range = arg
                    .get("range")
                    .and_then(|v| serde_json::from_value::<Range>(v.clone()).ok());
                if let (Some(uri), Some(range)) = (uri, range) {
                    let _ = self
                        .client
                        .show_document(ShowDocumentParams {
                            uri,
                            external: Some(false),
                            take_focus: Some(true),
                            selection: Some(range)
                        })
                        .await;
                }
            }
            return Ok(None);
        }

        if params.command == "cpclib.cycleCountForSelection" {
            let Some(arg) = params.arguments.into_iter().next()
            else {
                return Ok(None);
            };
            let uri = arg
                .get("uri")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<Url>().ok());
            let range = arg
                .get("range")
                .and_then(|v| serde_json::from_value::<Range>(v.clone()).ok());
            let (Some(uri), Some(range)) = (uri, range)
            else {
                return Ok(None);
            };

            // Use the open document when available, else load from disk -
            // mirrors `document_for_call_hierarchy_item`'s fallback.
            let Some(document) = self.load_document(&uri)
            else {
                return Ok(None);
            };

            let summary = self
                .asm_analyzer
                .cycle_count_for_selection(&document, range);
            return Ok(summary.map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null)));
        }

        if params.command == "cpclib.removeUnusedParameter" {
            let Some(arg) = params.arguments.into_iter().next()
            else {
                return Ok(None);
            };
            let uri = arg
                .get("uri")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<Url>().ok());
            let kind = arg.get("kind").and_then(|v| v.as_str()).and_then(|s| {
                match s {
                    "macro" => Some(crate::basm::remove_parameter::RemoveParameterKind::Macro),
                    "function" => {
                        Some(crate::basm::remove_parameter::RemoveParameterKind::Function)
                    },
                    _ => None
                }
            });
            let owner_name = arg
                .get("ownerName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let param_index = arg
                .get("paramIndex")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            let (Some(uri), Some(kind), Some(owner_name), Some(param_index)) =
                (uri, kind, owner_name, param_index)
            else {
                return Ok(None);
            };
            let Some(document) = self.load_document(&uri)
            else {
                return Ok(None);
            };

            let target = match self.asm_analyzer.resolve_remove_parameter_target(
                &document,
                kind,
                &owner_name,
                param_index
            ) {
                Ok(t) => t,
                Err(blocker) => {
                    report_removal_blockers(&self.client, &owner_name, &[blocker]).await;
                    return Ok(None);
                }
            };

            match self
                .remove_unused_parameter_across_workspace(&uri, &document, &target)
                .await
            {
                RemoveParameterOutcome::Blocked(blockers) => {
                    report_removal_blockers(&self.client, &owner_name, &blockers).await;
                },
                RemoveParameterOutcome::Ready(edit) => {
                    let file_count = edit.changes.as_ref().map_or(0, |c| c.len());
                    match self.client.apply_edit(edit).await {
                        Ok(response) if response.applied => {
                            self.client
                                .show_message(
                                    MessageType::INFO,
                                    format!(
                                        "Removed unused parameter '{}' from '{owner_name}' and \
                                         updated call sites across {file_count} file(s) under \
                                         the workspace root. If any Z80 source lives outside \
                                         this workspace root, verify it separately.",
                                        target.param_name
                                    )
                                )
                                .await;
                        },
                        Ok(response) => {
                            self.client
                                .show_message(
                                    MessageType::ERROR,
                                    format!(
                                        "The edit to remove '{}' from '{owner_name}' was not \
                                         applied{}.",
                                        target.param_name,
                                        response
                                            .failure_reason
                                            .map(|r| format!(": {r}"))
                                            .unwrap_or_default()
                                    )
                                )
                                .await;
                        },
                        Err(_) => {
                            self.client
                                .show_message(
                                    MessageType::ERROR,
                                    "Failed to apply the edit to the client."
                                )
                                .await;
                        }
                    }
                }
            }
            return Ok(None);
        }

        if params.command != "cpclib.getTargets" {
            return Ok(None);
        }

        let uri_str = params
            .arguments
            .into_iter()
            .next()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let Some(uri_str) = uri_str
        else {
            return Ok(Some(serde_json::json!([])));
        };
        let Ok(uri) = uri_str.parse::<Url>()
        else {
            return Ok(Some(serde_json::json!([])));
        };

        // Use cached document if available; otherwise read from disk.
        let targets: Vec<String> = match self.load_document(&uri) {
            Some(doc) => {
                self.build_analyzer
                    .target_symbols(&doc)
                    .into_iter()
                    .map(|s| s.name)
                    .collect()
            },
            None => vec![]
        };

        Ok(Some(serde_json::json!(targets)))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;

        let Some(entry) = self.documents.get(&uri)
        else {
            return Ok(None);
        };
        let doc_type = entry.value().doc_type;

        let actions: Vec<CodeAction> = match doc_type {
            DocumentType::Assembly => self.asm_analyzer.code_actions(entry.value(), range),
            DocumentType::Basic | DocumentType::CatartBasic => {
                self.basic_analyzer.code_actions(entry.value(), range)
            },
            _ => vec![]
        };

        if actions.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            actions
                .into_iter()
                .map(CodeActionOrCommand::CodeAction)
                .collect()
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        tracing::debug!("Semantic tokens request for {}", uri);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            let data = dispatch_by_doc_type(
                document,
                vec![],
                |doc| self.asm_analyzer.semantic_tokens(doc),
                |doc| self.build_analyzer.semantic_tokens(doc),
                |doc| self.basic_analyzer.semantic_tokens(doc)
            );
            if !data.is_empty() {
                return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data
                })));
            }
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        tracing::debug!("Formatting request for {}", uri);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            if document.doc_type == DocumentType::Assembly {
                // Load the project/user config file, reporting any parse error to the client.
                let base_opt = match cpclib_asmfmt::find_config_file() {
                    None => cpclib_asmfmt::AsmFormatOptions::default(),
                    Some(path) => {
                        match cpclib_asmfmt::load_config_from(&path) {
                            Ok(cfg) => cfg,
                            Err(e) => {
                                self.client
                                    .show_message(
                                        MessageType::ERROR,
                                        format!("basm-fmt config error: {e}")
                                    )
                                    .await;
                                cpclib_asmfmt::AsmFormatOptions::default()
                            }
                        }
                    },
                };
                // Let the editor's tab-size setting override the config's indent_size.
                let opt = cpclib_asmfmt::AsmFormatOptions {
                    indent_size: params.options.tab_size as usize,
                    ..base_opt
                };
                return Ok(self.asm_analyzer.format(document, &opt));
            }
            if matches!(
                document.doc_type,
                DocumentType::Basic | DocumentType::CatartBasic
            ) {
                return Ok(self.basic_analyzer.format(document));
            }
        }
        Ok(None)
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        let uri = params.text_document.uri;
        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            return Ok(match document.doc_type {
                DocumentType::Assembly => self.asm_analyzer.document_colors(document),
                DocumentType::Basic | DocumentType::CatartBasic => {
                    self.basic_analyzer.document_colors(document)
                },
                _ => Vec::new()
            });
        }
        Ok(Vec::new())
    }

    async fn color_presentation(
        &self,
        params: ColorPresentationParams
    ) -> Result<Vec<ColorPresentation>> {
        let uri = params.text_document.uri;
        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            return Ok(match document.doc_type {
                DocumentType::Assembly => {
                    self.asm_analyzer
                        .color_presentations(document, params.color, params.range)
                },
                DocumentType::Basic | DocumentType::CatartBasic => {
                    self.basic_analyzer
                        .color_presentations(params.color, params.range)
                },
                _ => Vec::new()
            });
        }
        Ok(Vec::new())
    }

    async fn on_type_formatting(
        &self,
        params: DocumentOnTypeFormattingParams
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        if params.ch != "\n" {
            return Ok(None);
        }
        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            if matches!(
                document.doc_type,
                DocumentType::Basic | DocumentType::CatartBasic
            ) {
                // Continue BASIC line numbering on the new line.
                return Ok(self.basic_analyzer.on_type_newline(document, position));
            }
            if document.doc_type == DocumentType::Assembly {
                // Same, but for BASIC embedded in a LOCOMOTIVE block.
                return Ok(self.asm_analyzer.on_type_newline(document, position));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod did_change_debounce_tests {
    use tower_lsp::LspService;

    use super::*;

    fn init_params() -> InitializeParams {
        InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceValue::Off),
            workspace_folders: None,
            client_info: None,
            locale: None
        }
    }

    #[tokio::test]
    async fn rapid_edits_leave_only_the_latest_version_pending() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        backend.initialize(init_params()).await.unwrap();

        let uri = Url::parse("file:///t.asm").unwrap();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "z80-asm".to_string(),
                    version: 1,
                    text: "    ld a, 1\n".to_string()
                }
            })
            .await;

        // Fire a burst of rapid edits, all comfortably inside one debounce
        // window - this is the scenario a real editor produces while the
        // user is actively typing.
        for v in 2..=5 {
            backend
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: v
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: format!("    ld a, {v}\n")
                    }]
                })
                .await;
        }

        // `pending_versions` must reflect only the *last* requested version
        // immediately - this is exactly what lets the four earlier-
        // scheduled debounce tasks recognize they've been superseded and
        // no-op instead of racing to publish stale diagnostics.
        assert_eq!(backend.pending_versions.get(&uri).map(|v| *v), Some(5));
        // The document text itself is updated synchronously, independent of
        // the debounce (hover/completion must never see a stale edit).
        assert_eq!(backend.documents.get(&uri).unwrap().version, 5);

        // Let every spawned debounce task (the four superseded ones and the
        // one live one) run to completion - must not panic/deadlock under
        // this rapid-fire burst.
        tokio::time::sleep(DID_CHANGE_DEBOUNCE + Duration::from_millis(150)).await;
        assert_eq!(backend.pending_versions.get(&uri).map(|v| *v), Some(5));
    }

    #[tokio::test]
    async fn did_close_evicts_the_pending_version() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        backend.initialize(init_params()).await.unwrap();

        let uri = Url::parse("file:///t.asm").unwrap();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "z80-asm".to_string(),
                    version: 1,
                    text: "    ld a, 1\n".to_string()
                }
            })
            .await;
        backend
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "    ld a, 2\n".to_string()
                }]
            })
            .await;
        assert!(backend.pending_versions.get(&uri).is_some());

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() }
            })
            .await;
        assert!(backend.pending_versions.get(&uri).is_none());
    }
}

#[cfg(test)]
mod load_document_tests {
    use tower_lsp::LspService;

    use super::*;

    #[tokio::test]
    async fn returns_the_open_in_memory_document_when_present() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        let uri = Url::parse("file:///open.asm").unwrap();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "z80-asm".to_string(),
                    version: 3,
                    text: "    ld a, 1\n".to_string()
                }
            })
            .await;

        let doc = backend.load_document(&uri).expect("document is open");
        assert_eq!(doc.version, 3);
    }

    #[tokio::test]
    async fn falls_back_to_reading_the_file_from_disk_when_not_open() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        let tmp = camino_tempfile::tempdir().unwrap();
        let path = tmp.path().join("disk.asm");
        std::fs::write(&path, "    ld a, 2\n").unwrap();
        let uri = Url::from_file_path(&path).unwrap();

        let doc = backend
            .load_document(&uri)
            .expect("document should be read from disk");
        assert_eq!(doc.text(), "    ld a, 2\n");
        // Not a fixed 0 - `disk_file_version` (see `disk_file_version_tests`
        // for why) - the exact value depends on the file's own mtime, so
        // just confirm `load_document` actually uses it rather than a
        // stale hardcoded constant.
        assert_eq!(doc.version, disk_file_version(path.as_std_path()));
    }

    #[tokio::test]
    async fn returns_none_for_a_path_that_does_not_exist() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        let uri = Url::parse("file:///does/not/exist.asm").unwrap();
        assert!(backend.load_document(&uri).is_none());
    }
}

#[cfg(test)]
mod workspace_roots_tests {
    use tower_lsp::LspService;

    use super::*;

    #[tokio::test]
    async fn workspace_roots_survives_a_poisoned_lock() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        // Poison the lock the same way a panic while holding it would:
        // acquire a write guard, then unwind without releasing it cleanly.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = backend.workspace_roots.write().unwrap();
            panic!("simulated panic while holding the write lock");
        }));
        assert!(result.is_err());
        assert!(backend.workspace_roots.is_poisoned());

        // Both the read helper and a fresh write must still work instead of
        // panicking on every subsequent call for the rest of the process.
        assert_eq!(backend.workspace_roots(), Vec::<PathBuf>::new());
        *backend
            .workspace_roots
            .write()
            .unwrap_or_else(|e| e.into_inner()) = vec![PathBuf::from("/tmp")];
        assert_eq!(backend.workspace_roots(), vec![PathBuf::from("/tmp")]);
    }
}

#[cfg(test)]
mod relativize_diagnostic_messages_tests {
    use super::*;

    fn diag(message: &str) -> Diagnostic {
        Diagnostic {
            message: message.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn strips_a_configured_root_s_absolute_prefix() {
        let mut diagnostics = vec![diag(
            "error: Syntax error\n  --> /home/x/project/src/sna.asm:24:5"
        )];
        relativize_diagnostic_messages(&mut diagnostics, &[PathBuf::from("/home/x/project")]);
        assert_eq!(
            diagnostics[0].message,
            "error: Syntax error\n  --> src/sna.asm:24:5"
        );
    }

    #[test]
    fn leaves_the_message_untouched_when_no_root_is_configured() {
        let mut diagnostics = vec![diag("--> /home/x/project/src/sna.asm:24:5")];
        relativize_diagnostic_messages(&mut diagnostics, &[]);
        assert_eq!(
            diagnostics[0].message,
            "--> /home/x/project/src/sna.asm:24:5"
        );
    }

    #[test]
    fn leaves_the_message_untouched_when_it_matches_no_configured_root() {
        let mut diagnostics = vec![diag("--> /home/x/project/src/sna.asm:24:5")];
        relativize_diagnostic_messages(&mut diagnostics, &[PathBuf::from("/some/other/root")]);
        assert_eq!(
            diagnostics[0].message,
            "--> /home/x/project/src/sna.asm:24:5"
        );
    }

    #[test]
    fn strips_every_occurrence_across_multiple_configured_roots() {
        let mut diagnostics = vec![diag(
            "In included file: /home/x/lib/util.asm:3:1\n  --> /home/x/project/src/sna.asm:24:5"
        )];
        relativize_diagnostic_messages(
            &mut diagnostics,
            &[
                PathBuf::from("/home/x/project"),
                PathBuf::from("/home/x/lib")
            ]
        );
        assert_eq!(
            diagnostics[0].message,
            "In included file: util.asm:3:1\n  --> src/sna.asm:24:5"
        );
    }
}

#[cfg(test)]
mod build_error_diagnostics_tests {
    use tower_lsp::LspService;

    use super::*;

    fn build_error_diag(message: &str) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0
                },
                end: Position {
                    line: 0,
                    character: 1
                }
            },
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("bndbuild".to_string()),
            message: message.to_string(),
            ..Default::default()
        }
    }

    /// The core persistence mechanism: `compute_diagnostics` is the one
    /// function every analyze path (`analyze_document`, and `did_change`'s
    /// debounced task) funnels through - proving it merges in a stored
    /// entry proves both call sites keep a build-error diagnostic visible
    /// across a live re-analysis of its own target file, unlike a live
    /// parse-error diagnostic (which only exists when re-analysis itself
    /// finds one).
    #[test]
    fn a_stored_build_error_survives_a_normal_analyze_cycle() {
        let asm_analyzer = AssemblyAnalyzer::new();
        let build_analyzer = BuildFileAnalyzer::new();
        let basic_analyzer = BasicAnalyzer::new();
        let build_error_diagnostics: DashMap<Url, Diagnostic> = DashMap::new();

        let uri = Url::parse("file:///sna.asm").unwrap();
        let stored = build_error_diag("Build error (from rule 'x'): boom");
        build_error_diagnostics.insert(uri.clone(), stored.clone());

        // Clean, valid code - live analysis alone finds nothing.
        let document = Document::new(uri, "org 0x8000\n    ret\n".to_string(), 1);
        let diagnostics = compute_diagnostics(
            &asm_analyzer,
            &build_analyzer,
            &basic_analyzer,
            &document,
            &[],
            &build_error_diagnostics
        );

        assert_eq!(diagnostics, vec![stored], "{diagnostics:?}");
    }

    #[test]
    fn a_stored_build_error_for_a_different_uri_does_not_leak_in() {
        let asm_analyzer = AssemblyAnalyzer::new();
        let build_analyzer = BuildFileAnalyzer::new();
        let basic_analyzer = BasicAnalyzer::new();
        let build_error_diagnostics: DashMap<Url, Diagnostic> = DashMap::new();

        build_error_diagnostics.insert(
            Url::parse("file:///other.asm").unwrap(),
            build_error_diag("Build error (from rule 'x'): boom")
        );

        let document = Document::new(
            Url::parse("file:///sna.asm").unwrap(),
            "org 0x8000\n    ret\n".to_string(),
            1
        );
        let diagnostics = compute_diagnostics(
            &asm_analyzer,
            &build_analyzer,
            &basic_analyzer,
            &document,
            &[],
            &build_error_diagnostics
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[tokio::test]
    async fn a_successful_build_clears_previously_stored_build_error_diagnostics() {
        use futures_util::StreamExt;

        let (service, socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        // `execute_command`'s "cpclib.runRule" branch calls
        // `self.client.log_message`/`show_message`, both of which (unlike
        // `publish_diagnostics`) send unconditionally - not gated behind the
        // server having gone through `initialize` - over a `ClientSocket`
        // backed by a capacity-1 channel. With nothing draining `socket`,
        // that first send blocks forever once the single buffer slot fills,
        // hanging the whole test (confirmed empirically: it does). Draining
        // it here in the background is what a real client's message loop
        // does, and what a `_socket`-only test (fine for handlers that never
        // proactively message the client) can't skip once one actually does.
        tokio::spawn(socket.for_each(|_| async {}));

        // Seed a stale entry, as if an earlier failing build had stored one.
        let target_uri = Url::parse("file:///sna.asm").unwrap();
        backend.build_error_diagnostics.insert(
            target_uri,
            build_error_diag("Build error (from rule 'x'): stale")
        );

        let tmp = camino_tempfile::tempdir().unwrap();
        let bnd_path = tmp.path().as_std_path().join("bndbuild.yml");
        let bnd_content = "- tgt: fine\n  phony: true\n  cmd: echo all good\n";
        std::fs::write(&bnd_path, bnd_content).unwrap();
        let bnd_uri = Url::from_file_path(&bnd_path).unwrap();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: bnd_uri,
                    language_id: "bndbuild".to_string(),
                    version: 1,
                    text: bnd_content.to_string()
                }
            })
            .await;

        let result = backend
            .execute_command(ExecuteCommandParams {
                command: "cpclib.runRule".to_string(),
                arguments: vec![
                    serde_json::Value::String("fine".to_string()),
                    serde_json::Value::String(bnd_path.to_string_lossy().to_string()),
                ],
                work_done_progress_params: Default::default()
            })
            .await;

        assert!(result.is_ok(), "{result:?}");
        assert!(
            backend.build_error_diagnostics.is_empty(),
            "{:?}",
            backend.build_error_diagnostics
        );
    }
}

#[cfg(test)]
mod outgoing_calls_tests {
    use tower_lsp::LspService;

    use super::*;

    /// Regression test for the `outgoing_calls` `AsmLabel` branch refactor
    /// (collecting `other_asm_docs` once per request instead of re-filtering
    /// `self.documents` once per target): a call target defined only in a
    /// *different* open document must still resolve, exercising exactly the
    /// cross-file fallback path that was restructured.
    #[tokio::test]
    async fn outgoing_call_resolves_a_target_defined_in_another_open_document() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        let caller_uri = Url::parse("file:///caller.asm").unwrap();
        let callee_uri = Url::parse("file:///callee.asm").unwrap();

        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: caller_uri.clone(),
                    language_id: "z80-asm".to_string(),
                    version: 1,
                    text: "start:\n  call target\n  ret\n".to_string()
                }
            })
            .await;
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: callee_uri.clone(),
                    language_id: "z80-asm".to_string(),
                    version: 1,
                    text: "target:\n  ret\n".to_string()
                }
            })
            .await;

        let item = CallHierarchyItem {
            name: "start".to_string(),
            kind: SymbolKind::FUNCTION,
            tags: None,
            detail: None,
            uri: caller_uri,
            range: Range::default(),
            selection_range: Range::default(),
            data: Some(
                CallHierarchyData::AsmLabel {
                    name: "start".to_string()
                }
                .to_json()
            )
        };

        let calls = backend
            .outgoing_calls(CallHierarchyOutgoingCallsParams {
                item,
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default()
            })
            .await
            .unwrap()
            .expect("expected outgoing calls");

        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].to.uri, callee_uri);
        assert_eq!(calls[0].to.name.to_uppercase(), "TARGET");
    }
}

/// Regression tests for the `dispatch_by_doc_type` refactor of `hover`/
/// `prepare_rename`/`document_symbol`/`semantic_tokens_full`: each must
/// still dispatch correctly end-to-end through the LSP service after
/// factoring their shared match block into the helper. The underlying
/// analyzer logic already has extensive dedicated coverage elsewhere in the
/// crate - these exist to catch a wiring mistake in the dispatch helper
/// itself, not to re-test hover/rename/symbol/token logic.
#[cfg(test)]
mod dispatch_by_doc_type_tests {
    use tower_lsp::LspService;

    use super::*;

    async fn open_asm_doc(backend: &CpcLspBackend, uri: &Url) {
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "z80-asm".to_string(),
                    version: 1,
                    text: "start:\n  ld a,1\n  ret\n".to_string()
                }
            })
            .await;
    }

    async fn open_catart_doc(backend: &CpcLspBackend, uri: &Url) {
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "catart-basic".to_string(),
                    version: 1,
                    text: "10 INK 1,26\n20 PAPER 1\n30 PRINT \"HI\"\n".to_string()
                }
            })
            .await;
    }

    #[tokio::test]
    async fn hover_still_dispatches_to_the_assembly_analyzer() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        let uri = Url::parse("file:///t.asm").unwrap();
        open_asm_doc(backend, &uri).await;

        let hover = backend
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 1,
                        character: 5
                    }
                },
                work_done_progress_params: Default::default()
            })
            .await
            .unwrap();

        assert!(hover.is_some(), "{hover:?}");
    }

    #[tokio::test]
    async fn prepare_rename_still_dispatches_to_the_assembly_analyzer() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        let uri = Url::parse("file:///t.asm").unwrap();
        open_asm_doc(backend, &uri).await;

        let response = backend
            .prepare_rename(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 0
                }
            })
            .await
            .unwrap();

        assert!(response.is_some(), "{response:?}");
    }

    #[tokio::test]
    async fn document_symbol_still_dispatches_to_the_assembly_analyzer() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        let uri = Url::parse("file:///t.asm").unwrap();
        open_asm_doc(backend, &uri).await;

        let response = backend
            .document_symbol(DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default()
            })
            .await
            .unwrap();

        match response {
            Some(DocumentSymbolResponse::Nested(symbols)) => {
                // "start" is nested one level deep now (under a "Labels"
                // container, see basm/symbols.rs's own nested-outline tests
                // for the full shape) - this test only cares that the
                // dispatch reached the assembly analyzer at all, not the
                // exact tree shape.
                let found = symbols.iter().any(|s| {
                    s.name == "start"
                        || s.children
                            .as_ref()
                            .is_some_and(|c| c.iter().any(|child| child.name == "start"))
                });
                assert!(found, "{symbols:?}");
            },
            other => panic!("expected nested document symbols, got {other:?}")
        }
    }

    #[tokio::test]
    async fn semantic_tokens_full_still_dispatches_to_the_assembly_analyzer() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        let uri = Url::parse("file:///t.asm").unwrap();
        open_asm_doc(backend, &uri).await;

        let response = backend
            .semantic_tokens_full(SemanticTokensParams {
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                text_document: TextDocumentIdentifier { uri }
            })
            .await
            .unwrap();

        match response {
            Some(SemanticTokensResult::Tokens(tokens)) => {
                assert!(!tokens.data.is_empty(), "{tokens:?}");
            },
            other => panic!("expected semantic tokens, got {other:?}")
        }
    }

    #[tokio::test]
    async fn hover_still_dispatches_to_the_basic_analyzer_for_catart_documents() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        let uri = Url::parse("file:///t.asc").unwrap();
        open_catart_doc(backend, &uri).await;

        let hover = backend
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 0,
                        character: 4
                    }
                },
                work_done_progress_params: Default::default()
            })
            .await
            .unwrap();

        assert!(hover.is_some(), "{hover:?}");
    }

    #[tokio::test]
    async fn semantic_tokens_full_still_dispatches_to_the_basic_analyzer_for_catart_documents() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();
        let uri = Url::parse("file:///t.asc").unwrap();
        open_catart_doc(backend, &uri).await;

        let response = backend
            .semantic_tokens_full(SemanticTokensParams {
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                text_document: TextDocumentIdentifier { uri }
            })
            .await
            .unwrap();

        match response {
            Some(SemanticTokensResult::Tokens(tokens)) => {
                assert!(!tokens.data.is_empty(), "{tokens:?}");
            },
            other => panic!("expected semantic tokens, got {other:?}")
        }
    }
}

#[cfg(test)]
mod candidate_asm_paths_tests {
    use tower_lsp::LspService;

    use super::*;

    /// Regression test for moving the workspace directory walk onto a
    /// blocking-pool thread (`tokio::task::spawn_blocking`): it must still
    /// find every `.asm` file under the configured root, still exclude the
    /// file being searched *from*, and still ignore non-`.asm` files -
    /// exactly the behavior the synchronous version had.
    #[tokio::test]
    async fn finds_every_asm_file_under_the_root_except_the_source_file() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("from.asm"), "").unwrap();
        std::fs::write(tmp.path().join("other.asm"), "").unwrap();
        std::fs::write(tmp.path().join("readme.txt"), "").unwrap();

        *backend
            .workspace_roots
            .write()
            .unwrap_or_else(|e| e.into_inner()) = vec![tmp.path().to_path_buf().into()];

        let from_uri = Url::from_file_path(tmp.path().join("from.asm")).unwrap();
        let paths = backend.candidate_asm_paths(&from_uri).await;

        assert_eq!(paths.len(), 1, "{paths:?}");
        assert_eq!(
            paths[0].file_name().and_then(|n| n.to_str()),
            Some("other.asm")
        );
    }
}

#[cfg(test)]
mod disk_file_version_tests {
    use tower_lsp::LspService;

    use super::*;

    #[test]
    fn changes_when_the_file_s_mtime_changes() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let path = tmp.path().join("t.asm");
        std::fs::write(&path, "org 0\n").unwrap();
        let v1 = disk_file_version(path.as_std_path());

        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();
        let v2 = disk_file_version(path.as_std_path());

        assert_ne!(v1, v2);
    }

    /// Regression test for the actual bug `disk_file_version` fixes: a file
    /// that changes on disk *without ever being opened in the editor*
    /// between two workspace scans must not keep serving its stale
    /// first-visit parse forever (the previous hardcoded `version = 0`
    /// meant `parse_document`'s `(uri, version)` cache always "hit" for an
    /// unopened file, regardless of on-disk changes).
    #[tokio::test]
    async fn workspace_scan_picks_up_a_file_that_changed_on_disk_without_being_opened() {
        let (service, _socket) = LspService::build(CpcLspBackend::new).finish();
        let backend = service.inner();

        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("from.asm"), "").unwrap();
        let target_path = tmp.path().join("target.asm");
        std::fs::write(&target_path, "foo:\n  ret\n").unwrap();

        *backend
            .workspace_roots
            .write()
            .unwrap_or_else(|e| e.into_inner()) = vec![tmp.path().to_path_buf().into()];

        let from_uri = Url::from_file_path(tmp.path().join("from.asm")).unwrap();

        // Queried in the same case as declared - goto-definition's
        // cross-file lookup is case-sensitive by default (basm labels are).
        let found_foo = backend
            .find_definition_via_workspace_scan(&from_uri, "foo")
            .await;
        assert!(found_foo.is_some(), "{found_foo:?}");

        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::write(&target_path, "bar:\n  ret\n").unwrap();
        std::fs::File::open(&target_path)
            .unwrap()
            .set_modified(later)
            .unwrap();

        let found_bar = backend
            .find_definition_via_workspace_scan(&from_uri, "bar")
            .await;
        assert!(found_bar.is_some(), "{found_bar:?}");
    }
}

#[cfg(test)]
mod remove_unused_parameter_tests {
    use futures_util::{SinkExt, StreamExt};
    use tower_lsp::LspService;
    use tower_lsp::jsonrpc::Response;

    use super::*;

    /// Drains a `ClientSocket`'s outgoing requests: a `workspace/applyEdit`
    /// *request* is answered with a synthetic `{applied: true}` response
    /// (required - unlike a notification such as `log_message`/
    /// `show_message`, the sender's own `self.client.apply_edit(...).await`
    /// blocks forever without one, since it correlates a real response by
    /// id) and its `WorkspaceEdit` is forwarded onto the returned channel
    /// for the test to assert against. Every other message is simply
    /// dropped, matching the drain-and-discard precedent already
    /// established for `log_message`/`show_message` elsewhere in this file
    /// (`build_error_diagnostics_tests`) - that pattern alone is
    /// insufficient here since it never answers anything.
    fn drain_client_socket_applying_edits(
        socket: tower_lsp::ClientSocket
    ) -> tokio::sync::mpsc::UnboundedReceiver<WorkspaceEdit> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let (mut requests, mut responses) = socket.split();
            while let Some(request) = requests.next().await {
                if request.method() != "workspace/applyEdit" {
                    continue;
                }
                if let Some(params) = request.params()
                    && let Ok(apply_params) =
                        serde_json::from_value::<ApplyWorkspaceEditParams>(params.clone())
                {
                    let _ = tx.send(apply_params.edit);
                }
                if let Some(id) = request.id() {
                    let response = Response::from_parts(
                        id.clone(),
                        Ok(serde_json::json!({ "applied": true }))
                    );
                    let _ = responses.send(response).await;
                }
            }
        });
        rx
    }

    fn write_file(root: &camino::Utf8Path, name: &str, content: &str) -> Url {
        let path = root.join(name);
        std::fs::write(path.as_std_path(), content).unwrap();
        Url::from_file_path(path.as_std_path()).unwrap()
    }

    async fn open(backend: &CpcLspBackend, uri: &Url, content: &str) {
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "z80-asm".to_string(),
                    version: 1,
                    text: content.to_string()
                }
            })
            .await;
    }

    async fn run_remove(
        backend: &CpcLspBackend,
        uri: &Url,
        kind: &str,
        owner_name: &str,
        param_index: usize
    ) -> Result<Option<serde_json::Value>> {
        backend
            .execute_command(ExecuteCommandParams {
                command: "cpclib.removeUnusedParameter".to_string(),
                arguments: vec![serde_json::json!({
                    "uri": uri.to_string(),
                    "kind": kind,
                    "ownerName": owner_name,
                    "paramIndex": param_index,
                })],
                work_done_progress_params: Default::default()
            })
            .await
    }

    fn set_workspace_root(backend: &CpcLspBackend, root: &camino::Utf8Path) {
        *backend
            .workspace_roots
            .write()
            .unwrap_or_else(|e| e.into_inner()) = vec![root.to_path_buf().into()];
    }

    /// Drives a real `initialize` request through the *full* tower
    /// `Service` stack (not just calling `CpcLspBackend::initialize`
    /// directly, which bypasses it) - required because `Client::apply_edit`
    /// uses the gated `send_request`, which unconditionally errors out
    /// without ever touching the socket unless the server's shared
    /// `ServerState` has actually transitioned to `Initialized`. That
    /// transition itself happens in a `tower::Layer` wrapping the service
    /// (`tower_lsp::service::layers`), which only ever runs requests
    /// dispatched through `Service::call` on `service` itself - calling the
    /// `LanguageServer::initialize` trait method on `backend` directly (as
    /// every other test in this file already does, since none of them
    /// needed a *state-gated* outgoing request before this one) would skip
    /// that layer entirely and leave the state stuck at `Uninitialized`.
    /// Mirrors tower-lsp's own test suite (`service.rs`'s
    /// `initializes_only_once`), the only real precedent for this exact
    /// need.
    async fn initialize_backend(service: &mut LspService<CpcLspBackend>) {
        use tower::{Service, ServiceExt};
        let request = tower_lsp::jsonrpc::Request::build("initialize")
            .params(serde_json::json!({ "capabilities": {} }))
            .id(1)
            .finish();
        let _ = service.ready().await.unwrap().call(request).await;
    }

    #[tokio::test]
    async fn single_file_case_removes_the_call_site_and_the_definition_as_one_edit() {
        let (mut service, socket) = LspService::build(CpcLspBackend::new).finish();
        initialize_backend(&mut service).await;
        let backend = service.inner();
        let mut edits = drain_client_socket_applying_edits(socket);

        let tmp = camino_tempfile::tempdir().unwrap();
        set_workspace_root(backend, tmp.path());

        let content = "MACRO foo, a, b, c\nENDM\nfoo(1, 2, 3)\n";
        let uri = write_file(tmp.path(), "main.asm", content);
        open(backend, &uri, content).await;

        let result = run_remove(backend, &uri, "macro", "foo", 2).await;
        assert!(result.is_ok(), "{result:?}");

        let edit = edits.recv().await.expect("expected an applied edit");
        let changes = edit.changes.expect("expected changes");
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert_eq!(changes[&uri].len(), 2, "{:?}", changes[&uri]); // call site + header
    }

    #[tokio::test]
    async fn multi_file_case_touches_every_file_with_a_call_site() {
        let (mut service, socket) = LspService::build(CpcLspBackend::new).finish();
        initialize_backend(&mut service).await;
        let backend = service.inner();
        let mut edits = drain_client_socket_applying_edits(socket);

        let tmp = camino_tempfile::tempdir().unwrap();
        set_workspace_root(backend, tmp.path());

        let def_content = "MACRO foo, a, b\nENDM\n";
        let def_uri = write_file(tmp.path(), "def.asm", def_content);
        open(backend, &def_uri, def_content).await;

        // Not opened - discovered purely via `candidate_asm_paths`' disk walk.
        write_file(tmp.path(), "caller.asm", "foo(1, 2)\n");

        let result = run_remove(backend, &def_uri, "macro", "foo", 1).await;
        assert!(result.is_ok(), "{result:?}");

        let edit = edits.recv().await.expect("expected an applied edit");
        let changes = edit.changes.expect("expected changes");
        assert_eq!(changes.len(), 2, "{changes:?}"); // def.asm + caller.asm
    }

    #[tokio::test]
    async fn a_blocked_call_site_prevents_any_edit_from_ever_being_applied() {
        let (mut service, socket) = LspService::build(CpcLspBackend::new).finish();
        initialize_backend(&mut service).await;
        let backend = service.inner();
        let mut edits = drain_client_socket_applying_edits(socket);

        let tmp = camino_tempfile::tempdir().unwrap();
        set_workspace_root(backend, tmp.path());

        // A bracketed-list argument at the target position can't be safely
        // rewritten (see `remove_parameter`'s own module doc comment) - the
        // whole operation must abort, not apply the (independently
        // computed, otherwise-valid) definition-header edit on its own.
        let content = "MACRO foo, a, b\nENDM\nfoo([1, 2, 3], 5)\n";
        let uri = write_file(tmp.path(), "main.asm", content);
        open(backend, &uri, content).await;

        let result = run_remove(backend, &uri, "macro", "foo", 0).await;
        assert!(result.is_ok(), "{result:?}");

        tokio::time::timeout(std::time::Duration::from_millis(500), edits.recv())
            .await
            .expect_err("no edit should ever have been sent to apply_edit");
    }

    #[tokio::test]
    async fn an_ambiguous_name_across_files_prevents_any_edit_from_ever_being_applied() {
        let (mut service, socket) = LspService::build(CpcLspBackend::new).finish();
        initialize_backend(&mut service).await;
        let backend = service.inner();
        let mut edits = drain_client_socket_applying_edits(socket);

        let tmp = camino_tempfile::tempdir().unwrap();
        set_workspace_root(backend, tmp.path());

        let content = "MACRO foo, a, b\nENDM\nfoo(1, 2)\n";
        let uri = write_file(tmp.path(), "main.asm", content);
        open(backend, &uri, content).await;
        // A second, same-named macro elsewhere in the workspace.
        write_file(tmp.path(), "other.asm", "MACRO foo, x\nENDM\n");

        let result = run_remove(backend, &uri, "macro", "foo", 1).await;
        assert!(result.is_ok(), "{result:?}");

        tokio::time::timeout(std::time::Duration::from_millis(500), edits.recv())
            .await
            .expect_err("no edit should ever have been sent to apply_edit");
    }
}

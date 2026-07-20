use std::path::PathBuf;
use std::sync::RwLock;

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::basm::AssemblyAnalyzer;
use crate::bndbuild::BuildFileAnalyzer;
use crate::common::document::{Document, DocumentType};
use crate::locomotive::BasicAnalyzer;

pub struct CpcLspBackend {
    client: Client,
    documents: DashMap<Url, Document>,
    asm_analyzer: AssemblyAnalyzer,
    build_analyzer: BuildFileAnalyzer,
    basic_analyzer: BasicAnalyzer,
    /// Workspace folder(s) reported at `initialize`, used to bound the eager
    /// cross-file goto-definition fallback (`find_definition_via_workspace_scan`).
    /// Empty when the client sent neither `workspaceFolders` nor `rootUri`
    /// (e.g. single-file mode) - the fallback then derives a root from the
    /// document itself, see `basm::definition::project_root_for`.
    workspace_roots: RwLock<Vec<PathBuf>>
}

impl CpcLspBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            asm_analyzer: AssemblyAnalyzer::new(),
            build_analyzer: BuildFileAnalyzer::new(),
            basic_analyzer: BasicAnalyzer::new(),
            workspace_roots: RwLock::new(Vec::new())
        }
    }

    async fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn analyze_document(&self, document: &Document) {
        let diagnostics = match document.doc_type {
            DocumentType::Assembly => self.asm_analyzer.analyze(document),
            DocumentType::BuildFile => self.build_analyzer.analyze(document),
            DocumentType::Basic => self.basic_analyzer.analyze(document),
            DocumentType::Unknown => Vec::new()
        };

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
    /// `from_uri`), even when never opened by the editor. Stops at the first
    /// match; `.git`/`target`/etc. are pruned from the walk.
    fn find_definition_via_workspace_scan(
        &self,
        from_uri: &Url,
        word_upper: &str
    ) -> Option<Location> {
        let roots: Vec<PathBuf> = {
            let guard = self.workspace_roots.read().unwrap();
            if guard.is_empty() {
                crate::basm::definition::project_root_for(from_uri)
                    .into_iter()
                    .collect()
            }
            else {
                guard.clone()
            }
        };

        let from_path = from_uri.to_file_path().ok();

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
                if let Some(loc) = self.find_definition_at_path(path, word_upper) {
                    return Some(loc);
                }
            }
        }
        None
    }

    /// Shared by both cross-file fallbacks: look up `word_upper` in the file
    /// at `path`, using the already-open in-memory document if there is one.
    fn find_definition_at_path(
        &self,
        path: &std::path::Path,
        word_upper: &str
    ) -> Option<Location> {
        let target_uri = Url::from_file_path(path).ok()?;

        if let Some(open_doc) = self.documents.get(&target_uri) {
            return self
                .asm_analyzer
                .find_definition_in(open_doc.value(), word_upper);
        }

        let text = std::fs::read_to_string(path).ok()?;
        let doc = Document::new(target_uri, text, 0);
        self.asm_analyzer.find_definition_in(&doc, word_upper)
    }

    /// Workspace-wide rename of a `Global` basm label: unlike
    /// `find_definition_via_workspace_scan` (stops at the first match),
    /// this collects edits from *every* matching file — `.asm` files this
    /// document itself `include`s, plus every `.asm` file under
    /// `workspace_roots` (open or on disk). Inserts into `changes`,
    /// skipping any URI already present (the current document's own edits,
    /// added by the caller before this runs).
    fn rename_label_across_workspace(
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

        let roots: Vec<PathBuf> = {
            let guard = self.workspace_roots.read().unwrap();
            if guard.is_empty() {
                crate::basm::definition::project_root_for(from_uri)
                    .into_iter()
                    .collect()
            }
            else {
                guard.clone()
            }
        };
        let from_path = from_uri.to_file_path().ok();

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
                self.rename_label_at_path(path, target, new_name, changes);
            }
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
        let Some(target_uri) = Url::from_file_path(path).ok()
        else {
            return;
        };
        if changes.contains_key(&target_uri) {
            return;
        }

        let edits = if let Some(open_doc) = self.documents.get(&target_uri) {
            self.asm_analyzer
                .rename_occurrences_in(open_doc.value(), target, new_name)
        }
        else {
            let Ok(text) = std::fs::read_to_string(path)
            else {
                return;
            };
            let doc = Document::new(target_uri.clone(), text, 0);
            self.asm_analyzer
                .rename_occurrences_in(&doc, target, new_name)
        };

        if !edits.is_empty() {
            changes.insert(target_uri, edits);
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
        let Some(word) =
            crate::bndbuild::definition::jinja_word_at(line, position.character as usize)
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

        let roots: Vec<PathBuf> = self.workspace_roots.read().unwrap().clone();
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
            let doc = if let Some(open_doc) = self.documents.get(&target_uri) {
                open_doc.value().clone()
            }
            else {
                let Ok(text) = std::fs::read_to_string(&path)
                else {
                    continue;
                };
                Document::new(target_uri.clone(), text, 0)
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
fn is_ignored_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        && matches!(
            entry.file_name().to_str(),
            Some(".git" | ".hg" | ".svn" | "target" | "node_modules")
        )
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
        *self.workspace_roots.write().unwrap() = roots;

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

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        tracing::info!("Document changed: {}", params.text_document.uri);

        if let Some(mut entry) = self.documents.get_mut(&params.text_document.uri) {
            for change in params.content_changes {
                entry.apply_change(&change, params.text_document.version);
            }

            let document = entry.value().clone();
            drop(entry); // Release the lock before async call

            self.analyze_document(&document).await;
        }
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
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        tracing::debug!("Hover request at {}:{}", uri, position.line);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();

            let hover = match document.doc_type {
                DocumentType::Assembly => self.asm_analyzer.hover(document, position),
                DocumentType::BuildFile => self.build_analyzer.hover(document, position),
                DocumentType::Basic => self.basic_analyzer.hover(document, position),
                DocumentType::Unknown => None
            };

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
                DocumentType::Basic => self.basic_analyzer.completion(document, position),
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
            DocumentType::Basic => self.basic_analyzer.goto_definition(entry.value(), position),
            DocumentType::Unknown => None
        };
        if let Some(loc) = location {
            return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
        }

        // For Assembly: if the symbol was not defined locally, search all other
        // open Assembly documents (cross-file navigation).
        if doc_type == DocumentType::Assembly {
            let word = match self.asm_analyzer.word_at_position(entry.value(), position) {
                Some(w) => w.to_uppercase(),
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
                if let Some(loc) = self.asm_analyzer.find_definition_in(other.value(), &word) {
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
            if let Some(loc) = self.find_definition_via_workspace_scan(&uri, &word) {
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
                DocumentType::Basic => self.basic_analyzer.find_references(entry.value(), position),
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

        let range = match entry.value().doc_type {
            DocumentType::Assembly => self.asm_analyzer.prepare_rename(entry.value(), position),
            DocumentType::BuildFile => self.build_analyzer.prepare_rename(entry.value(), position),
            DocumentType::Basic => self.basic_analyzer.prepare_rename(entry.value(), position),
            DocumentType::Unknown => None
        };

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
            DocumentType::Basic => {
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
                let Some(crate::basm::definition::RenameTarget::Global(_)) = target
                else {
                    return Ok(self.asm_analyzer.rename(entry.value(), position, &new_name));
                };
                let target = target.unwrap();

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
                );

                Ok(non_empty_workspace_edit(changes))
            },

            DocumentType::Unknown => Ok(None)
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        tracing::debug!("Document symbol request for {}", uri);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();

            let symbols = match document.doc_type {
                DocumentType::Assembly => self.asm_analyzer.document_symbols(document),
                DocumentType::BuildFile => self.build_analyzer.document_symbols(document),
                DocumentType::Basic => self.basic_analyzer.document_symbols(document),
                DocumentType::Unknown => Vec::new()
            };

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
            if document.doc_type == DocumentType::BuildFile {
                let lenses = self.build_analyzer.code_lens(document);
                if !lenses.is_empty() {
                    return Ok(Some(lenses));
                }
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
            let document = if let Some(entry) = self.documents.get(&uri) {
                entry.value().clone()
            }
            else if let Ok(text) = std::fs::read_to_string(&fname) {
                Document::new(uri.clone(), text, 0)
            }
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
                tokio::task::spawn_blocking(move || {
                    crate::bndbuild::BuildFileAnalyzer::new().run_rule(&document, &rule, Some(tx))
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
        let targets: Vec<String> = if let Some(entry) = self.documents.get(&uri) {
            self.build_analyzer
                .target_symbols(entry.value())
                .into_iter()
                .map(|s| s.name)
                .collect()
        }
        else if let Ok(path) = uri.to_file_path() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                let doc = Document::new(uri, text, 0);
                self.build_analyzer
                    .target_symbols(&doc)
                    .into_iter()
                    .map(|s| s.name)
                    .collect()
            }
            else {
                vec![]
            }
        }
        else {
            vec![]
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
            DocumentType::Basic => self.basic_analyzer.code_actions(entry.value(), range),
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
            let data = match document.doc_type {
                DocumentType::Assembly => self.asm_analyzer.semantic_tokens(document),
                DocumentType::BuildFile => self.build_analyzer.semantic_tokens(document),
                DocumentType::Basic => self.basic_analyzer.semantic_tokens(document),
                DocumentType::Unknown => vec![]
            };
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
            if document.doc_type == DocumentType::Basic {
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
                DocumentType::Basic => self.basic_analyzer.document_colors(document),
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
                DocumentType::Basic => {
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
            if document.doc_type == DocumentType::Basic {
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

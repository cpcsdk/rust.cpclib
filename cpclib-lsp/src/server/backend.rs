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
    basic_analyzer: BasicAnalyzer
}

impl CpcLspBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            asm_analyzer: AssemblyAnalyzer::new(),
            build_analyzer: BuildFileAnalyzer::new(),
            basic_analyzer: BasicAnalyzer::new()
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
}

#[tower_lsp::async_trait]
impl LanguageServer for CpcLspBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        tracing::info!("Initializing cpclib-lsp server");
        tracing::info!("Client capabilities: {:?}", params.capabilities);

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
            drop(entry); // release the DashMap read guard before iterating

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
                .document_symbols(entry.value())
                .into_iter()
                .map(|s| s.name)
                .collect()
        }
        else if let Ok(path) = uri.to_file_path() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                let doc = Document::new(uri, text, 0);
                self.build_analyzer
                    .document_symbols(&doc)
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
        }
        Ok(None)
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
        }
        Ok(None)
    }
}

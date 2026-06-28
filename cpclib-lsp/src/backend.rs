use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::document::{Document, DocumentType};
use crate::asm::AssemblyAnalyzer;
use crate::basic::BasicAnalyzer;
use crate::build::BuildFileAnalyzer;

pub struct CpcLspBackend {
    client: Client,
    documents: DashMap<Url, Document>,
    asm_analyzer: AssemblyAnalyzer,
    build_analyzer: BuildFileAnalyzer,
    basic_analyzer: BasicAnalyzer,
}

impl CpcLspBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            asm_analyzer: AssemblyAnalyzer::new(),
            build_analyzer: BuildFileAnalyzer::new(),
            basic_analyzer: BasicAnalyzer::new(),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn analyze_document(&self, document: &Document) {
        let diagnostics = match document.doc_type {
            DocumentType::Assembly  => self.asm_analyzer.analyze(document),
            DocumentType::BuildFile => self.build_analyzer.analyze(document),
            DocumentType::Basic     => self.basic_analyzer.analyze(document),
            DocumentType::Unknown   => Vec::new(),
        };

        self.publish_diagnostics(document.uri.clone(), diagnostics).await;
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
                            include_text: Some(false),
                        })),
                    },
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
                    completion_item: None,
                }),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["cpclib.getTargets".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                            legend: crate::asm::semantic_tokens_legend(),
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "cpclib-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
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
            Some(params.text_document.language_id.as_str()),
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
                DocumentType::Assembly  => self.asm_analyzer.hover(document, position),
                DocumentType::BuildFile => self.build_analyzer.hover(document, position),
                DocumentType::Basic     => self.basic_analyzer.hover(document, position),
                DocumentType::Unknown   => None,
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
                DocumentType::Assembly  => self.asm_analyzer.completion(document, position),
                DocumentType::BuildFile => self.build_analyzer.completion(document, position),
                DocumentType::Basic     => self.basic_analyzer.completion(document, position),
                DocumentType::Unknown   => Vec::new(),
            };
            
            if !completions.is_empty() {
                return Ok(Some(CompletionResponse::Array(completions)));
            }
        }
        
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        tracing::debug!("Goto definition request at {}:{}", uri, position.line);
        
        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            
            let location = match document.doc_type {
                DocumentType::Assembly  => self.asm_analyzer.goto_definition(document, position),
                DocumentType::BuildFile => self.build_analyzer.goto_definition(document, position),
                DocumentType::Basic     => self.basic_analyzer.goto_definition(document, position),
                DocumentType::Unknown   => None,
            };
            
            if let Some(location) = location {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }
        
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        
        tracing::debug!("References request at {}:{}", uri, position.line);
        
        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            
            let references = match document.doc_type {
                DocumentType::Assembly  => self.asm_analyzer.find_references(document, position),
                DocumentType::BuildFile => self.build_analyzer.find_references(document, position),
                DocumentType::Basic     => self.basic_analyzer.find_references(document, position),
                DocumentType::Unknown   => Vec::new(),
            };
            
            if !references.is_empty() {
                return Ok(Some(references));
            }
        }
        
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        tracing::debug!("Document symbol request for {}", uri);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();

            let symbols = match document.doc_type {
                DocumentType::Assembly  => self.asm_analyzer.document_symbols(document),
                DocumentType::BuildFile => self.build_analyzer.document_symbols(document),
                DocumentType::Basic     => self.basic_analyzer.document_symbols(document),
                DocumentType::Unknown   => Vec::new(),
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

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<serde_json::Value>> {
        if params.command != "cpclib.getTargets" {
            return Ok(None);
        }

        let uri_str = params.arguments
            .into_iter()
            .next()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let Some(uri_str) = uri_str else { return Ok(Some(serde_json::json!([]))); };
        let Ok(uri) = uri_str.parse::<Url>() else { return Ok(Some(serde_json::json!([]))); };

        // Use cached document if available; otherwise read from disk.
        let targets: Vec<String> = if let Some(entry) = self.documents.get(&uri) {
            self.build_analyzer
                .document_symbols(entry.value())
                .into_iter()
                .map(|s| s.name)
                .collect()
        } else if let Ok(path) = uri.to_file_path() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                let doc = Document::new(uri, text, 0);
                self.build_analyzer
                    .document_symbols(&doc)
                    .into_iter()
                    .map(|s| s.name)
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        Ok(Some(serde_json::json!(targets)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        tracing::debug!("Semantic tokens request for {}", uri);

        if let Some(entry) = self.documents.get(&uri) {
            let document = entry.value();
            let data = match document.doc_type {
                DocumentType::Assembly  => self.asm_analyzer.semantic_tokens(document),
                DocumentType::BuildFile => self.build_analyzer.semantic_tokens(document),
                DocumentType::Basic     => self.basic_analyzer.semantic_tokens(document),
                DocumentType::Unknown   => vec![],
            };
            if !data.is_empty() {
                return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data,
                })));
            }
        }

        Ok(None)
    }
}

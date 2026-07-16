//! Integration tests that actually launch the LSP server and query it

use cpclib_lsp::CpcLspBackend;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};

#[tokio::test]
async fn test_server_initialization() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    let params = InitializeParams {
        process_id: None,
        root_path: None,

        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    let result = backend.initialize(params).await.unwrap();

    // Verify server capabilities
    assert!(result.capabilities.text_document_sync.is_some());
    assert!(result.capabilities.hover_provider.is_some());
    assert!(result.capabilities.completion_provider.is_some());

    // Verify completion trigger characters
    if let Some(completion_options) = result.capabilities.completion_provider {
        let triggers = completion_options.trigger_characters.unwrap();
        assert!(triggers.contains(&".".to_string()));
        assert!(triggers.contains(&":".to_string()));
    }

    // Verify server info
    assert_eq!(result.server_info.as_ref().unwrap().name, "cpclib-lsp");
    assert!(result.server_info.as_ref().unwrap().version.is_some());
}

#[tokio::test]
async fn test_assembly_completion_after_document_open() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize the server
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open an assembly document
    let uri = Url::parse("file:///test.asm").unwrap();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "z80-asm".to_string(),
            version: 1,
            text: "    LD A, 5\n    ".to_string()
        }
    };

    backend.did_open(open_params).await;

    // Give the server a moment to process
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Request completions at the end of the file
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 1,
                character: 4
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result = backend.completion(completion_params).await.unwrap();

    if let Some(completion_result) = result {
        let items = match completion_result {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => list.items
        };

        // Verify we got completions
        assert!(!items.is_empty(), "Should have completion items");

        // Check for some common Z80 instructions
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.iter().any(|l| l == "LD"),
            "Should include LD instruction, got: {:?}",
            labels
        );
        assert!(
            labels.iter().any(|l| l == "ADD"),
            "Should include ADD instruction"
        );
    }
    else {
        panic!("Expected completion result");
    }
}

#[tokio::test]
async fn test_build_file_completion() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open a build file
    let uri = Url::parse("file:///build.build").unwrap();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "yaml".to_string(),
            version: 1,
            text: "targets:\n  main:\n    tasks:\n      - ".to_string()
        }
    };

    backend.did_open(open_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Request completions for task type
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: 10
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result = backend.completion(completion_params).await.unwrap();

    if let Some(completion_result) = result {
        let items = match completion_result {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => list.items
        };

        // Verify we got task completions
        assert!(!items.is_empty(), "Should have task completion items");

        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.iter().any(|l| l == "basm"),
            "Should include basm task, got: {:?}",
            labels
        );
    }
    else {
        panic!("Expected completion result");
    }
}

#[tokio::test]
async fn test_internal_command_argument_completion() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    let uri = Url::parse("file:///build2.build").unwrap();
    let text = "targets:\n  main:\n    tasks:\n      - basm --sn";
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "yaml".to_string(),
            version: 1,
            text: text.to_string()
        }
    };

    backend.did_open(open_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Cursor at the end of "      - basm --sn" (line 3, 0-indexed)
    let last_line_len = text.lines().last().unwrap().chars().count() as u32;
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: last_line_len
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result = backend.completion(completion_params).await.unwrap();

    let Some(completion_result) = result
    else {
        panic!("Expected completion result");
    };
    let items = match completion_result {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items
    };

    let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
    assert!(
        labels.iter().any(|l| l == "--snapshot"),
        "Should offer basm's real --snapshot flag, got: {:?}",
        labels
    );
}

#[tokio::test]
async fn test_assembly_hover() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open assembly file with an instruction
    let uri = Url::parse("file:///test.asm").unwrap();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "z80-asm".to_string(),
            version: 1,
            text: "    LD A, 5\n    ADD A, B\n".to_string()
        }
    };

    backend.did_open(open_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Request hover over the LD instruction
    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 5 // On the "LD" instruction
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default()
    };

    let result = backend.hover(hover_params).await.unwrap();

    if let Some(hover) = result {
        // Verify hover content exists
        match hover.contents {
            HoverContents::Scalar(content) => {
                match content {
                    MarkedString::String(s) => {
                        assert!(!s.is_empty(), "Hover should have content");
                    },
                    MarkedString::LanguageString(ls) => {
                        assert!(!ls.value.is_empty(), "Hover should have content");
                    }
                }
            },
            HoverContents::Array(contents) => {
                assert!(!contents.is_empty(), "Hover should have content");
            },
            HoverContents::Markup(markup) => {
                assert!(!markup.value.is_empty(), "Hover should have content");
            }
        }
    }
    // Note: Hover may return None if the position doesn't match a keyword
}

#[tokio::test]
async fn test_document_change() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open a document
    let uri = Url::parse("file:///test.asm").unwrap();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "z80-asm".to_string(),
            version: 1,
            text: "    LD A, 5\n".to_string()
        }
    };

    backend.did_open(open_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Change the document
    let change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "    ADD A, B\n".to_string()
        }]
    };

    backend.did_change(change_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify document was updated by requesting completion
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 12
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result = backend.completion(completion_params).await;
    // Just verify no error occurred
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_build_file_keyword_completion() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open empty build file
    let uri = Url::parse("file:///build.build").unwrap();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "yaml".to_string(),
            version: 1,
            text: "".to_string()
        }
    };

    backend.did_open(open_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Request top-level completions
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 0
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result = backend.completion(completion_params).await.unwrap();

    if let Some(completion_result) = result {
        let items = match completion_result {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => list.items
        };

        // Verify we got keyword completions
        assert!(!items.is_empty(), "Should have keyword completion items");

        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.iter().any(|l| l == "targets"),
            "Should include 'targets' keyword, got: {:?}",
            labels
        );
    }
}

#[tokio::test]
async fn test_multiple_documents() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open multiple documents
    let uri1 = Url::parse("file:///test1.asm").unwrap();
    let uri2 = Url::parse("file:///build.build").unwrap();

    // Open first document
    let open_params1 = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri1.clone(),
            language_id: "z80-asm".to_string(),
            version: 1,
            text: "    LD A, 5\n".to_string()
        }
    };

    backend.did_open(open_params1).await;

    // Open second document
    let open_params2 = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri2.clone(),
            language_id: "yaml".to_string(),
            version: 1,
            text: "targets:\n".to_string()
        }
    };

    backend.did_open(open_params2).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Request completions from first document
    let completion_params1 = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri1.clone() },
            position: Position {
                line: 0,
                character: 8
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result1 = backend.completion(completion_params1).await;

    // Request completions from second document
    let completion_params2 = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri2.clone() },
            position: Position {
                line: 0,
                character: 8
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result2 = backend.completion(completion_params2).await;

    // Both should succeed
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_document_close() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open a document
    let uri = Url::parse("file:///test.asm").unwrap();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "z80-asm".to_string(),
            version: 1,
            text: "    LD A, 5\n".to_string()
        }
    };

    backend.did_open(open_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Close the document
    let close_params = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() }
    };

    backend.did_close(close_params).await;

    // After closing, completion requests should return None or empty
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 8
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result = backend.completion(completion_params).await.unwrap();
    // Document is closed, so we expect None
    assert!(result.is_none(), "Should return None for closed document");
}

#[tokio::test]
async fn test_directive_completion() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();

    let backend = service.inner();

    // Initialize
    let params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: Some(TraceValue::Off),
        workspace_folders: None,
        client_info: None,
        locale: None
    };

    backend.initialize(params).await.unwrap();

    // Open assembly file
    let uri = Url::parse("file:///test.asm").unwrap();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "z80-asm".to_string(),
            version: 1,
            text: "    ".to_string()
        }
    };

    backend.did_open(open_params).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Request completions
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 4
            }
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None
    };

    let result = backend.completion(completion_params).await.unwrap();

    if let Some(completion_result) = result {
        let items = match completion_result {
            CompletionResponse::Array(items) => items,
            CompletionResponse::List(list) => list.items
        };

        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();

        // Check for common assembler directives
        assert!(
            labels.iter().any(|l| l == "ORG"),
            "Should include ORG directive, got: {:?}",
            labels
        );
        assert!(
            labels.iter().any(|l| l == "DB"),
            "Should include DB directive"
        );
        assert!(
            labels.iter().any(|l| l == "DW"),
            "Should include DW directive"
        );
    }
    else {
        panic!("Expected completion result");
    }
}

//! Integration tests that actually launch the LSP server and query it

use cpclib_lsp::CpcLspBackend;
use futures_util::StreamExt;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};

async fn initialized_backend() -> (LspService<CpcLspBackend>, tower_lsp::ClientSocket) {
    let (service, socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    service
        .inner()
        .initialize(InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceValue::Off),
            workspace_folders: None,
            client_info: None,
            locale: None
        })
        .await
        .unwrap();
    (service, socket)
}

/// Like `initialized_backend`, but drives `initialize` through the *full*
/// tower `Service` stack instead of calling `CpcLspBackend::initialize`
/// directly on `service.inner()`. The state transition to
/// `State::Initialized` happens in a `tower::Layer` wrapping the service,
/// which only runs for requests dispatched through `Service::call` on the
/// service itself - calling the trait method directly skips that layer,
/// leaving the state stuck at `Uninitialized`. That's invisible for
/// `log_message`/`show_message` (`Client` sends those via the *unchecked*
/// notification path regardless of state, so `initialized_backend` is fine
/// for tests that only check those) but `publish_diagnostics` uses the
/// *gated* `send_notification`, which silently drops the message outside
/// `Initialized`/`ShutDown` - needed for `cpclib.runRule`'s failure
/// diagnostics to reach a test at all. Mirrors the identical, previously
/// established fix in `cpclib-lsp/src/server/backend.rs`'s own
/// `remove_unused_parameter_tests::initialize_backend`.
///
/// The notification drain is spawned *before* the real `initialize` call
/// (not left to the caller, unlike `drain_client_notifications`'s other
/// callers) - once genuinely `Initialized`, `publish_diagnostics`'s gated
/// send can actually reach the socket's channel, and with nothing reading
/// it, a call sending on a full channel would block forever, hanging any
/// caller here that produces enough notification traffic before getting
/// around to draining it themselves (`did_open`'s own diagnostics pass
/// included, not just `cpclib.runRule`'s).
async fn initialized_backend_with_drained_notifications() -> (
    LspService<CpcLspBackend>,
    tokio::sync::mpsc::UnboundedReceiver<tower_lsp::jsonrpc::Request>
) {
    use tower::{Service, ServiceExt};

    let (mut service, socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let notifications = drain_client_notifications(socket);
    let request = tower_lsp::jsonrpc::Request::build("initialize")
        .params(serde_json::json!({ "capabilities": {} }))
        .id(1)
        .finish();
    let _ = service.ready().await.unwrap().call(request).await;
    (service, notifications)
}

/// Collects every outgoing notification (`log_message`/`show_message`/
/// `publishDiagnostics`) sent through `socket` onto a channel for later
/// inspection. None of `cpclib.runRule`'s outgoing messages are *requests*
/// (unlike `workspace/applyEdit` elsewhere in this server), so nothing here
/// needs to send a response back - just drain and forward.
fn drain_client_notifications(
    socket: tower_lsp::ClientSocket
) -> tokio::sync::mpsc::UnboundedReceiver<tower_lsp::jsonrpc::Request> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        let (mut requests, _responses) = socket.split();
        while let Some(request) = requests.next().await {
            let _ = tx.send(request);
        }
    });
    rx
}

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
async fn test_dependency_filename_completion() {
    let tmp = camino_tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("hello.asm"), "").unwrap();
    std::fs::write(tmp.path().join("hello2.dsk"), "").unwrap();

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

    let uri = Url::from_file_path(tmp.path().join("build.bnd")).unwrap();
    let text = "- tgt: out.bin\n  dep: hel";
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

    let last_line_len = text.lines().last().unwrap().chars().count() as u32;
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 1,
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
        labels.iter().any(|l| l == "hello.asm") && labels.iter().any(|l| l == "hello2.dsk"),
        "Should offer real files from the build file's directory, got: {:?}",
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

#[tokio::test]
async fn test_goto_definition_finds_symbol_in_an_unopened_included_file() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    let tmp = camino_tempfile::tempdir().unwrap();
    // helper.asm is never opened by the editor - only INCLUDEd from main.asm.
    std::fs::write(tmp.path().join("helper.asm"), "HELPER_LABEL:\n    ret\n").unwrap();

    backend
        .initialize(InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceValue::Off),
            workspace_folders: None,
            client_info: None,
            locale: None
        })
        .await
        .unwrap();

    let main_uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: main_uri.clone(),
                language_id: "z80-asm".to_string(),
                version: 1,
                text: "    include \"helper.asm\"\n    call helper_label\n".to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Cursor on "helper_label" (the call target), line 1.
    let result = backend
        .goto_definition(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: main_uri.clone()
                },
                position: Position {
                    line: 1,
                    character: 11
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap();

    let GotoDefinitionResponse::Scalar(location) = result.expect("definition should be found")
    else {
        panic!("expected a scalar location");
    };
    assert_eq!(
        location.uri,
        Url::from_file_path(tmp.path().join("helper.asm")).unwrap()
    );
    assert_eq!(location.range.start.line, 0);
}

#[tokio::test]
async fn test_goto_definition_finds_symbol_via_workspace_scan_without_an_include() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    let tmp = camino_tempfile::tempdir().unwrap();
    // other.asm is never opened and never INCLUDEd by main.asm - only
    // findable by scanning the workspace for .asm files.
    std::fs::write(tmp.path().join("other.asm"), "SOME_LABEL:\n    ret\n").unwrap();

    backend
        .initialize(InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceValue::Off),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(tmp.path()).unwrap(),
                name: "test-workspace".to_string()
            }]),
            client_info: None,
            locale: None
        })
        .await
        .unwrap();

    let main_uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: main_uri.clone(),
                language_id: "z80-asm".to_string(),
                version: 1,
                text: "    call some_label\n".to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let result = backend
        .goto_definition(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: main_uri.clone()
                },
                position: Position {
                    line: 0,
                    character: 11
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap();

    let GotoDefinitionResponse::Scalar(location) = result.expect("definition should be found")
    else {
        panic!("expected a scalar location");
    };
    assert_eq!(
        location.uri,
        Url::from_file_path(tmp.path().join("other.asm")).unwrap()
    );
    assert_eq!(location.range.start.line, 0);
}

#[tokio::test]
async fn test_goto_definition_workspace_scan_finds_the_one_match_among_many_candidates() {
    // The workspace scan searches candidate files in parallel (rayon) - this
    // exercises that path with several decoy .asm files plus exactly one
    // real match, guarding against a panic/race/wrong-result from
    // concurrent access to `self.documents`/disk during the parallel scan.
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    let tmp = camino_tempfile::tempdir().unwrap();
    for i in 0..8 {
        std::fs::write(
            tmp.path().join(format!("decoy{i}.asm")),
            format!("UNRELATED_LABEL_{i}:\n    ret\n")
        )
        .unwrap();
    }
    std::fs::write(tmp.path().join("real.asm"), "SOME_LABEL:\n    ret\n").unwrap();

    backend
        .initialize(InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceValue::Off),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(tmp.path()).unwrap(),
                name: "test-workspace".to_string()
            }]),
            client_info: None,
            locale: None
        })
        .await
        .unwrap();

    let main_uri = Url::from_file_path(tmp.path().join("main.asm")).unwrap();
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: main_uri.clone(),
                language_id: "z80-asm".to_string(),
                version: 1,
                text: "    call some_label\n".to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let result = backend
        .goto_definition(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: main_uri.clone()
                },
                position: Position {
                    line: 0,
                    character: 11
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap();

    let GotoDefinitionResponse::Scalar(location) = result.expect("definition should be found")
    else {
        panic!("expected a scalar location");
    };
    assert_eq!(
        location.uri,
        Url::from_file_path(tmp.path().join("real.asm")).unwrap()
    );
    assert_eq!(location.range.start.line, 0);
}

// ─── cpclib.cycleCountForSelection ─────────────────────────────────────────

#[tokio::test]
async fn test_cycle_count_for_selection_command() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    backend
        .initialize(InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceValue::Off),
            workspace_folders: None,
            client_info: None,
            locale: None
        })
        .await
        .unwrap();

    let uri = Url::parse("file:///cycle_count.asm").unwrap();
    // A conditional jump (djnz) mixed with a plain instruction - exercises
    // both the min/max range and the plain-sum paths in one selection.
    let text = "loop: djnz loop\n    nop\n";
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "basm".to_string(),
                version: 1,
                text: text.to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let range = Range {
        start: Position {
            line: 0,
            character: 0
        },
        end: Position {
            line: 2,
            character: 0
        }
    };
    let result = backend
        .execute_command(ExecuteCommandParams {
            command: "cpclib.cycleCountForSelection".to_string(),
            arguments: vec![serde_json::json!({
                "uri": uri.to_string(),
                "range": range,
            })],
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap()
        .expect("expected a cycle count result");

    // djnz loop: 3 (not taken) or 4 (taken), plus nop's own fixed 1.
    assert_eq!(result["min_nops"], 4);
    assert_eq!(result["max_nops"], 5);
    assert_eq!(result["instruction_count"], 2);
    assert_eq!(result["unrecognized_count"], 0);
}

#[tokio::test]
async fn test_cycle_count_for_selection_command_with_no_selection_returns_none() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    backend
        .initialize(InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some(TraceValue::Off),
            workspace_folders: None,
            client_info: None,
            locale: None
        })
        .await
        .unwrap();

    let uri = Url::parse("file:///cycle_count_none.asm").unwrap();
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "basm".to_string(),
                version: 1,
                text: "    nop\n".to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let collapsed = Range {
        start: Position {
            line: 0,
            character: 0
        },
        end: Position {
            line: 0,
            character: 0
        }
    };
    let result = backend
        .execute_command(ExecuteCommandParams {
            command: "cpclib.cycleCountForSelection".to_string(),
            arguments: vec![serde_json::json!({
                "uri": uri.to_string(),
                "range": collapsed,
            })],
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_code_lens_appears_for_an_asm_file_with_an_embedded_bndbuild_block() {
    let (service, _socket) = initialized_backend().await;
    let backend = service.inner();

    let uri = Url::parse("file:///shadebobs.asm").unwrap();
    let text = "; #!bndbuild\n; - tgt: test\n;   phony: true\n;   cmd:\n;    - basm --snapshot shadebobs.asm -o shadebobs.sna --lst shadebobs.lst\n;    - -ace shadebobs.sna\nORG 0x8000\n";
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "basm".to_string(),
                version: 1,
                text: text.to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let lenses = backend
        .code_lens(CodeLensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("expected a code lens for the embedded rule");

    assert_eq!(lenses.len(), 1);
    let command = lenses[0].command.as_ref().expect("expected a command");
    assert_eq!(command.title, "▶ Run: test");
    assert_eq!(command.command, "cpclib.runRule");
    let args = command.arguments.as_ref().unwrap();
    assert_eq!(args[0], serde_json::json!("test"));
    assert_eq!(
        args[1],
        serde_json::json!(uri.to_file_path().unwrap().to_string_lossy().to_string())
    );
    // The lens sits on the "- tgt: test" line inside the embedded block
    // (line 1), not on the "#!bndbuild" marker line (line 0).
    assert_eq!(lenses[0].range.start.line, 1);
}

#[tokio::test]
async fn test_run_embedded_bndbuild_rule_command_executes_and_streams_output() {
    let (service, socket) = initialized_backend().await;
    let backend = service.inner();
    let mut notifications = drain_client_notifications(socket);

    // A real tempdir-backed path, not a bare `file:///embedded_ok.asm` -
    // `cpclib.runRule` execution sets the process-wide working directory to
    // this URI's own parent (`BndBuilder::decode_from_reader`), and doing
    // that against a real directory (rather than filesystem root) keeps
    // this test from disturbing any other test that runs afterward in the
    // same process.
    let tmp = camino_tempfile::tempdir().unwrap();
    let uri = Url::from_file_path(tmp.path().join("embedded_ok.asm")).unwrap();
    let text = "; #!bndbuild\n; - tgt: fine\n;   cmd: echo hello from embedded lens\nORG 0x8000\n";
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "basm".to_string(),
                version: 1,
                text: text.to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let result = backend
        .execute_command(ExecuteCommandParams {
            command: "cpclib.runRule".to_string(),
            arguments: vec![
                serde_json::json!("fine"),
                serde_json::json!(uri.to_file_path().unwrap().to_string_lossy().to_string()),
            ],
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap();
    assert!(result.is_none());

    // `execute_command` has already awaited every `self.client.*` call, so
    // the notifications are queued on the socket - but `drain_client_notifications`'s
    // background task still needs a chance to actually be scheduled and
    // pull them off before a non-blocking `try_recv` below will see them.
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Drain the notifications sent while the command ran and confirm a
    // success `window/showMessage` arrived.
    let mut saw_success = false;
    while let Ok(request) = notifications.try_recv() {
        if request.method() == "window/showMessage"
            && let Some(params) = request.params()
            && params
                .get("message")
                .and_then(|m| m.as_str())
                .is_some_and(|m| m.contains("built successfully"))
        {
            saw_success = true;
        }
    }
    assert!(saw_success, "expected a success showMessage notification");
}

#[tokio::test]
async fn test_run_embedded_bndbuild_rule_command_failure_produces_a_diagnostic() {
    let (service, mut notifications) = initialized_backend_with_drained_notifications().await;
    let backend = service.inner();

    let tmp = camino_tempfile::tempdir().unwrap();
    let uri = Url::from_file_path(tmp.path().join("embedded_fail.asm")).unwrap();
    let text = "; #!bndbuild\n; - tgt: broken\n;   cmd: cp does_not_exist_anywhere.src dst.bin\nORG 0x8000\n";
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "basm".to_string(),
                version: 1,
                text: text.to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    backend
        .execute_command(ExecuteCommandParams {
            command: "cpclib.runRule".to_string(),
            arguments: vec![
                serde_json::json!("broken"),
                serde_json::json!(uri.to_file_path().unwrap().to_string_lossy().to_string()),
            ],
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let mut saw_diagnostic_on_asm_file = false;
    while let Ok(request) = notifications.try_recv() {
        if request.method() == "textDocument/publishDiagnostics"
            && let Some(params) = request.params()
            && params.get("uri").and_then(|u| u.as_str()) == Some(uri.as_str())
            && params
                .get("diagnostics")
                .and_then(|d| d.as_array())
                .is_some_and(|d| !d.is_empty())
        {
            saw_diagnostic_on_asm_file = true;
        }
    }
    assert!(
        saw_diagnostic_on_asm_file,
        "expected a non-empty publishDiagnostics notification for the .asm file"
    );
}

#[tokio::test]
async fn test_asm_file_without_an_embedded_block_has_no_code_lens_and_bnd_file_code_lens_is_unaffected()
 {
    let (service, _socket) = initialized_backend().await;
    let backend = service.inner();

    let asm_uri = Url::parse("file:///plain.asm").unwrap();
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: asm_uri.clone(),
                language_id: "basm".to_string(),
                version: 1,
                text: "; just a normal comment\nORG 0x8000\n".to_string()
            }
        })
        .await;

    let bnd_uri = Url::parse("file:///bndbuild.yml").unwrap();
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: bnd_uri.clone(),
                language_id: "bndbuild".to_string(),
                version: 1,
                text: "- tgt: real\n  phony: true\n  cmd: echo hi\n".to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let asm_lenses = backend
        .code_lens(CodeLensParams {
            text_document: TextDocumentIdentifier { uri: asm_uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap();
    assert!(
        asm_lenses.is_none(),
        "expected no code lens for a plain .asm file"
    );

    let bnd_lenses = backend
        .code_lens(CodeLensParams {
            text_document: TextDocumentIdentifier { uri: bnd_uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("a real .bnd file should still get its own code lens");
    // One "▶ Run: real" rule lens, plus one "▶ Run this command" lens for
    // its single task (`cmd: echo hi`).
    assert_eq!(bnd_lenses.len(), 2, "{bnd_lenses:?}");
    assert!(
        bnd_lenses
            .iter()
            .any(|l| l.command.as_ref().unwrap().title == "▶ Run: real")
    );
    assert!(
        bnd_lenses
            .iter()
            .any(|l| l.command.as_ref().unwrap().command == "cpclib.runTask")
    );
}

/// Regression test for `.CAT`/`.ASC` CatArt support: opening a document
/// containing a statement outside the CatArt whitelist (`GOTO`) must publish
/// an ERROR-severity diagnostic, and one using `CURSOR` (structurally valid
/// but a documented no-op in the real CatArt renderer) must publish a
/// WARNING-severity diagnostic instead.
#[tokio::test]
async fn test_catart_document_gets_error_and_warning_diagnostics() {
    let (service, mut notifications) = initialized_backend_with_drained_notifications().await;
    let backend = service.inner();

    let uri = Url::parse("file:///t.asc").unwrap();
    let text = "10 GOTO 20\n20 CURSOR 1\n30 PRINT \"X\"\n";
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "catart-basic".to_string(),
                version: 1,
                text: text.to_string()
            }
        })
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let mut severities: Vec<i64> = Vec::new();
    while let Ok(request) = notifications.try_recv() {
        if request.method() == "textDocument/publishDiagnostics"
            && let Some(params) = request.params()
            && params.get("uri").and_then(|u| u.as_str()) == Some(uri.as_str())
            && let Some(diags) = params.get("diagnostics").and_then(|d| d.as_array())
        {
            severities.extend(diags.iter().filter_map(|d| d.get("severity")?.as_i64()));
        }
    }

    // DiagnosticSeverity::ERROR = 1, WARNING = 2 in the LSP wire format.
    assert!(
        severities.contains(&1),
        "expected an ERROR diagnostic for GOTO: {severities:?}"
    );
    assert!(
        severities.contains(&2),
        "expected a WARNING diagnostic for CURSOR: {severities:?}"
    );
}

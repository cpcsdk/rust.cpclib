//! Integration tests for LSP call hierarchy: cross-document basm CALL/RET
//! and BASIC embedded in a `LOCOMOTIVE` block, both of which need the real
//! `backend.rs` request/response round-trip (the `data` tag surviving a
//! `prepare` -> `incoming`/`outgoing` hop, and cross-document resolution
//! that only `backend.rs` can do since it alone sees every open document).

use cpclib_lsp::CpcLspBackend;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};

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

fn open_params(uri: Url, language_id: &str, text: &str) -> DidOpenTextDocumentParams {
    DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri,
            language_id: language_id.to_string(),
            version: 1,
            text: text.to_string()
        }
    }
}

#[tokio::test]
async fn test_call_hierarchy_capability_advertised() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    let result = backend.initialize(init_params()).await.unwrap();
    assert!(result.capabilities.call_hierarchy_provider.is_some());
}

#[tokio::test]
async fn test_cross_document_incoming_and_outgoing_calls() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();
    backend.initialize(init_params()).await.unwrap();

    let caller_uri = Url::parse("file:///caller.asm").unwrap();
    let callee_uri = Url::parse("file:///callee.asm").unwrap();

    backend
        .did_open(open_params(
            caller_uri.clone(),
            "z80-asm",
            "caller:\n    call target\n    ret\n"
        ))
        .await;
    backend
        .did_open(open_params(
            callee_uri.clone(),
            "z80-asm",
            "target:\n    ret\n"
        ))
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Outgoing: "caller" (in caller.asm) calls "target", defined in callee.asm.
    let caller_item = backend
        .prepare_call_hierarchy(CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: caller_uri.clone()
                },
                position: Position {
                    line: 0,
                    character: 0
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap()
        .expect("expected an item at caller:")
        .remove(0);
    assert_eq!(caller_item.name, "caller");
    assert_eq!(caller_item.uri, caller_uri);

    let outgoing = backend
        .outgoing_calls(CallHierarchyOutgoingCallsParams {
            item: caller_item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("expected one outgoing call");
    assert_eq!(outgoing.len(), 1, "{outgoing:?}");
    assert_eq!(outgoing[0].to.name, "target");
    assert_eq!(outgoing[0].to.uri, callee_uri);

    // Incoming: "target" (in callee.asm) is called from caller.asm.
    let target_item = backend
        .prepare_call_hierarchy(CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: callee_uri.clone()
                },
                position: Position {
                    line: 0,
                    character: 0
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap()
        .expect("expected an item at target:")
        .remove(0);
    assert_eq!(target_item.name, "target");

    let incoming = backend
        .incoming_calls(CallHierarchyIncomingCallsParams {
            item: target_item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("expected one incoming call");
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "caller");
    assert_eq!(incoming[0].from.uri, caller_uri);
}

#[tokio::test]
async fn test_locomotive_embedded_call_hierarchy_round_trips_through_data() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();
    backend.initialize(init_params()).await.unwrap();

    let uri = Url::parse("file:///with_basic.asm").unwrap();
    // BASIC content occupies document lines 3-5 (basic_range is exclusive of
    // the LOCOMOTIVE/ENDLOCOMOTIVE lines themselves).
    let text =
        "start:\n    ret\nLOCOMOTIVE\n10 GOSUB 100\n100 PRINT 1\n110 RETURN\nENDLOCOMOTIVE\n";
    backend
        .did_open(open_params(uri.clone(), "z80-asm", text))
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Cursor on document line 4 ("100 PRINT 1").
    let target_item = backend
        .prepare_call_hierarchy(CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 4,
                    character: 0
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap()
        .expect("expected an item at BASIC line 100")
        .remove(0);
    assert_eq!(target_item.name, "Line 100");
    assert_eq!(target_item.range.start.line, 4);

    let incoming = backend
        .incoming_calls(CallHierarchyIncomingCallsParams {
            item: target_item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("expected one incoming call from line 10");
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "Line 10");
    // Line 10 is document line 3 (the block's first BASIC-content line).
    assert_eq!(incoming[0].from.range.start.line, 3);
}

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

// ─── bndbuild: cross-file targets and Jinja macros ─────────────────────────
//
// Mirrors the real project's shape (`demo.bnd5`): a shared `common.build` at
// the workspace root, `{% include %}`d by a `build.bnd` one directory down -
// same fixture shape as `cpclib-lsp/src/bndbuild/definition.rs`'s existing
// `rename_tests`. `common.build` is deliberately never opened by the editor
// (read from disk on demand), matching how a real workspace only has the
// scene file the user is actively editing open.

#[tokio::test]
async fn test_bndbuild_target_dependency_resolves_across_an_include() {
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    let tmp = camino_tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("common.build"),
        "- tgt: shared.bin\n  cmd: echo build\n"
    )
    .unwrap();
    std::fs::create_dir_all(tmp.path().join("scene")).unwrap();
    std::fs::write(
        tmp.path().join("scene/build.bnd"),
        "{% include \"../common.build\" %}\n\n- tgt: scene.bin\n  dep: shared.bin\n"
    )
    .unwrap();

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

    let scene_uri = Url::from_file_path(tmp.path().join("scene/build.bnd")).unwrap();
    let common_uri = Url::from_file_path(tmp.path().join("common.build")).unwrap();
    backend
        .did_open(open_params(
            scene_uri.clone(),
            "bndbuild",
            &std::fs::read_to_string(tmp.path().join("scene/build.bnd")).unwrap()
        ))
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Outgoing: scene.bin (in scene/build.bnd) depends on shared.bin,
    // defined only in the included (never-opened) common.build.
    let scene_item = backend
        .prepare_call_hierarchy(CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: scene_uri.clone()
                },
                position: Position {
                    line: 2,
                    character: 7
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap()
        .expect("expected an item at scene.bin's tgt: field")
        .remove(0);
    assert_eq!(scene_item.name, "scene.bin");

    let outgoing = backend
        .outgoing_calls(CallHierarchyOutgoingCallsParams {
            item: scene_item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("expected one outgoing call to shared.bin");
    assert_eq!(outgoing.len(), 1, "{outgoing:?}");
    assert_eq!(outgoing[0].to.name, "shared.bin");
    assert_eq!(outgoing[0].to.uri, common_uri);

    // Incoming: shared.bin (in common.build) is depended on by scene.bin,
    // found by walking upward to every file that includes common.build.
    // Unlike the outgoing half above, `prepareCallHierarchy` always starts
    // from the file the user's cursor is actually in - real LSP usage
    // requires it open, so open it here too (cross-file *resolution* on the
    // outgoing/incoming side is what's still exercised without opening the
    // other file).
    backend
        .did_open(open_params(
            common_uri.clone(),
            "bndbuild",
            &std::fs::read_to_string(tmp.path().join("common.build")).unwrap()
        ))
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let shared_item = backend
        .prepare_call_hierarchy(CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: common_uri.clone()
                },
                position: Position {
                    line: 0,
                    character: 7
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap()
        .expect("expected an item at shared.bin's tgt: field")
        .remove(0);
    assert_eq!(shared_item.name, "shared.bin");

    let incoming = backend
        .incoming_calls(CallHierarchyIncomingCallsParams {
            item: shared_item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("expected one incoming call from scene.bin");
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.name, "scene.bin");
    assert_eq!(incoming[0].from.uri, scene_uri);
}

#[tokio::test]
async fn test_bndbuild_macro_call_resolves_across_an_include() {
    // Directly mirrors the real, load-bearing shape confirmed in
    // `demo.bnd5`: `common.build` defines a macro, a sibling scene
    // directory's `build.bnd` includes it and calls it from a `cmd:` field.
    let (service, _socket) = LspService::build(|client| CpcLspBackend::new(client)).finish();
    let backend = service.inner();

    let tmp = camino_tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("common.build"),
        "{% macro emu_launch_sna(sna) -%}\nemu --snapshot {{sna}}\n{%- endmacro %}\n"
    )
    .unwrap();
    std::fs::create_dir_all(tmp.path().join("polar_dots")).unwrap();
    let scene_text = "{% include \"../common.build\" %}\n\n- tgt: testlink\n  dep: link.sna\n  cmd: {{emu_launch_sna(\"link.sna\")}}\n";
    std::fs::write(tmp.path().join("polar_dots/build.bnd"), scene_text).unwrap();

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

    let scene_uri = Url::from_file_path(tmp.path().join("polar_dots/build.bnd")).unwrap();
    let common_uri = Url::from_file_path(tmp.path().join("common.build")).unwrap();
    backend
        .did_open(open_params(scene_uri.clone(), "bndbuild", scene_text))
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let call_line = scene_text.lines().nth(4).unwrap();
    let col = call_line.find("emu_launch_sna(\"link").unwrap() as u32 + 2;
    let call_item = backend
        .prepare_call_hierarchy(CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: scene_uri.clone()
                },
                position: Position {
                    line: 4,
                    character: col
                }
            },
            work_done_progress_params: WorkDoneProgressParams::default()
        })
        .await
        .unwrap()
        .expect("expected an item at the emu_launch_sna( call site")
        .remove(0);
    assert_eq!(call_item.name, "emu_launch_sna");
    assert_eq!(
        call_item.uri, common_uri,
        "the macro's own item should point at its definition in common.build"
    );

    let incoming = backend
        .incoming_calls(CallHierarchyIncomingCallsParams {
            item: call_item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default()
        })
        .await
        .unwrap()
        .expect("expected one incoming call from polar_dots/build.bnd");
    assert_eq!(incoming.len(), 1, "{incoming:?}");
    assert_eq!(incoming[0].from.uri, scene_uri);
}

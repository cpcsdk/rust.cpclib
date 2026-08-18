//! A stop in the emulator must reach the editor.
//!
//! Driven through the *real* transport - the loopback server, one JSON body per
//! SSE line, a framed reply POSTed back - with a scripted peer standing in for
//! the page. This is the loop that was silently broken: the frames the adapter
//! sent downstream could not be parsed by the emulator, so nothing ever
//! attached and no stop was ever reported.

use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::peer::{DapPeer, RecordingPeer};
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::json;

fn map() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["demo_code.asm".into()],
        rows: vec![SourceMapRow::flat(0, 12, 0x4000, 3)]
    })
}

/// The whole sequence an editor performs around a stop, in order.
#[test]
fn a_breakpoint_stop_becomes_a_usable_editor_state() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.on_attached().unwrap();

    // The editor sets a breakpoint on a real line...
    let request = json!({
        "seq": 1, "type": "request", "command": "setBreakpoints",
        "arguments": {
            "source": {"path": "demo_code.asm"},
            "breakpoints": [{"line": 12}]
        }
    });
    let answer = session.on_editor_message(&request).unwrap();
    assert_eq!(answer[0]["body"]["breakpoints"][0]["verified"], json!(true));

    // ...the emulator reaches it and says so.
    let stopped = json!({
        "type": "event", "event": "stopped",
        "body": {"reason": "instruction breakpoint", "hitBreakpointIds": [1]}
    });
    let forwarded = session.on_emulator_message(&stopped);
    assert_eq!(forwarded.len(), 1, "the stop reaches the editor");
    assert_eq!(
        forwarded[0]["body"]["threadId"],
        json!(1),
        "with a thread to act on"
    );

    // The editor asks who stopped...
    let threads = session
        .on_editor_message(&json!({"seq": 2, "type": "request", "command": "threads"}))
        .unwrap();
    assert_eq!(threads[0]["body"]["threads"][0]["id"], json!(1));

    // ...and where. The emulator answers in addresses; the editor needs a file.
    let stack = session.on_emulator_message(&json!({
        "type": "response", "command": "stackTrace", "success": true,
        "body": {"stackFrames": [{
            "id": 1, "name": "Z80 @ 0x4001", "line": 0,
            "instructionPointerReference": "0x4001"
        }]}
    }));
    // The stack trace is the answer; a `cpclib/stoppedAt` event may precede it.
    let frame = &stack.last().unwrap()["body"]["stackFrames"][0];
    assert_eq!(
        frame["line"],
        json!(12),
        "the source line, from the middle of the instruction"
    );
    assert_eq!(frame["source"]["name"], json!("demo_code.asm"));

    // And the toolbar works.
    for button in ["continue", "next", "stepIn", "stepOut", "stepBack", "pause"] {
        session
            .on_editor_message(&json!({"seq": 3, "type": "request", "command": button}))
            .unwrap();
    }
    let sent = session.peer().commands();
    for button in ["continue", "next", "stepIn", "stepOut", "stepBack", "pause"] {
        assert!(
            sent.contains(&button.to_string()),
            "{button} reached the emulator"
        );
    }
}

/// Put the session where the program is stopped at `0x4000`.
///
/// `PC` is learned from the register pane rather than from the stack trace, so
/// both have to arrive - which is what the editor really does on a stop.
fn stopped_at_4000(session: &mut Session<RecordingPeer>) {
    session.on_emulator_message(&json!({
        "type": "response", "command": "stackTrace", "success": true,
        "body": {"stackFrames": [{
            "id": 1, "name": "Z80 @ 0x4000", "line": 0,
            "instructionPointerReference": "0x4000"
        }]}
    }));
    session.on_emulator_message(&json!({
        "type": "response", "command": "variables", "success": true,
        "body": {"variables": [{"name": "PC", "value": "0x4000", "variablesReference": 0}]}
    }));
}

/// Every emulator tried so far needs the breakpoint under `PC` out of the way
/// before it can step off it - including the one this fixture stands in for.
#[test]
fn stepping_lifts_the_breakpoint_for_an_emulator_that_needs_it() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.on_attached().unwrap();
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setBreakpoints",
            "arguments": {
                "source": {"path": "demo_code.asm"},
                "breakpoints": [{"line": 12}]
            }
        }))
        .unwrap();
    stopped_at_4000(&mut session);

    let before = session.peer().commands().len();
    session
        .on_editor_message(&json!({"seq": 2, "type": "request", "command": "next"}))
        .unwrap();
    let after: Vec<String> = session.peer().commands()[before..].to_vec();
    assert_eq!(
        after,
        vec![
            "setInstructionBreakpoints".to_string(),
            "next".to_string()
        ],
        "the breakpoint under PC is lifted first: {after:?}"
    );
}

/// 64K holding `ld a,0x01` at 0x4000 - two bytes, the whole of the program
/// these tests stop in.
fn image_holding_ld_a_1() -> Vec<u8> {
    let mut memory = vec![0u8; 0x1_0000];
    memory[0x4000] = 0x3E;
    memory[0x4001] = 0x01;
    memory
}

/// Stop the program at 0x4000, over a source file whose line 3 reads `written`,
/// and hand back the `cpclib/stoppedAt` event the extension acts on.
///
/// The walk over the stack has to be answered when there is an image, because
/// having one is what makes the adapter attempt it: `SP` at the top of stack
/// means there is nothing pushed to walk, which ends it immediately.
fn stop_over(written: &str, image: Option<Vec<u8>>) -> serde_json::Value {
    let directory = camino_tempfile::tempdir().unwrap();
    let file = directory.path().join("resolved.asm");
    std::fs::write(&file, format!("\torg 0x4000\nSTATE equ 1\n{written}\n")).unwrap();

    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec![file.to_string()],
        rows: vec![SourceMapRow::flat(0, 3, 0x4000, 2)]
    });
    let mut session = Session::new(RecordingPeer::new(), map).with_top_of_stack(0xBFF0);
    if let Some(image) = image {
        session = session.with_image(image);
    }
    session.on_attached().unwrap();

    let mut out = session.on_emulator_message(&json!({
        "type": "response", "command": "stackTrace", "success": true,
        "body": {"stackFrames": [{
            "id": 1, "name": "Z80 @ 0x4000", "line": 0,
            "instructionPointerReference": "0x4000"
        }]}
    }));
    if out.is_empty() {
        answer(
            &mut session,
            "scopes",
            json!({"scopes": [{
                "name": "Registers", "presentationHint": "registers",
                "variablesReference": 18
            }]})
        );
        out = answer(
            &mut session,
            "variables",
            json!({"variables": [
                {"name": "SP", "value": "0xBFF0", "variablesReference": 0},
                {"name": "PC", "value": "0x4000", "variablesReference": 0}
            ]})
        );
    }

    out.into_iter()
        .find(|message| message["event"] == json!("cpclib/stoppedAt"))
        .expect("the stop is announced")
}

/// Answer the last request the adapter sent with `command`.
fn answer(
    session: &mut Session<RecordingPeer>,
    command: &str,
    body: serde_json::Value
) -> Vec<serde_json::Value> {
    let seq = session.peer().last(command).unwrap()["seq"]
        .as_i64()
        .unwrap();
    session.on_emulator_message(&json!({
        "type": "response", "command": command, "request_seq": seq,
        "success": true, "body": body
    }))
}

/// The source names a constant; the machine holds its value. Saying which is
/// the whole point of the hint.
#[test]
fn the_stop_carries_the_instruction_really_in_memory() {
    let stop = stop_over("\tld a,STATE", Some(image_holding_ld_a_1()));

    let instruction = stop["body"]["instruction"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    assert!(
        instruction.starts_with("ld a") && instruction.contains('1'),
        "the resolved instruction: {stop}"
    );
}

/// A line already written as the machine holds it has nothing to disambiguate,
/// and a hint repeating it is noise - including when only the base, the case
/// or the spacing differ.
#[test]
fn a_line_that_already_says_it_gets_no_hint() {
    for written in ["\tLD A, 0x01", "\tld a,1", ".here\tld a,0x1 ; the state"] {
        let stop = stop_over(written, Some(image_holding_ld_a_1()));
        assert_eq!(
            stop["body"]["instruction"],
            json!(null),
            "`{written}` says it already: {stop}"
        );
    }
}

/// Without the program's image there are no bytes to decode, and asking the
/// emulator for them would cost a round trip on every single stop.
#[test]
fn no_image_means_no_hint_rather_than_a_round_trip() {
    let stop = stop_over("\tld a,STATE", None);

    assert_eq!(stop["body"]["instruction"], json!(null), "{stop}");
    assert_eq!(
        stop["body"]["line"],
        json!(3),
        "the stop still reaches the editor"
    );
}

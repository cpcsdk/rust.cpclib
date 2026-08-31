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
        vec!["setInstructionBreakpoints".to_string(), "next".to_string()],
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
/// and hand back everything the adapter said about it.
///
/// `held` is what the emulator is to answer for the four bytes at `PC`; `None`
/// makes the read fail, which is how the fall back to the image is exercised.
///
/// The walk over the stack has to be answered when there is an image, because
/// having one is what makes the adapter attempt it: `SP` at the top of stack
/// means there is nothing pushed to walk, which ends it immediately.
fn stop_over(written: &str, image: Option<Vec<u8>>, held: Option<&[u8]>) -> Vec<serde_json::Value> {
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
    assert!(
        out.iter()
            .any(|message| message["event"] == json!("cpclib/stoppedAt")),
        "the stop is announced: {out:?}"
    );

    // The hint follows the reveal: the editor is told where the program is
    // before the bytes at `PC` have even been asked for.
    out.extend(answer(
        &mut session,
        "readMemory",
        match held {
            Some(bytes) => json!({"address": "0x4000", "data": base64(bytes)}),
            None => json!({})
        }
    ));
    out
}

/// Bytes as DAP carries them.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut block = [0u8; 3];
        block[..chunk.len()].copy_from_slice(chunk);
        let word = u32::from(block[0]) << 16 | u32::from(block[1]) << 8 | u32::from(block[2]);
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(ALPHABET[(word >> (18 - 6 * index)) as usize & 0x3F] as char);
            }
            else {
                out.push('=');
            }
        }
    }
    out
}

/// What the extension is finally told to show beside the line.
///
/// The hint arrives in `cpclib/stoppedInstruction` when the emulator was
/// asked, and in `cpclib/stoppedAt` when there was nobody to ask - the last
/// word on the subject either way.
fn hint(messages: &[serde_json::Value]) -> serde_json::Value {
    messages
        .iter()
        .rev()
        .find(|message| {
            message["event"] == json!("cpclib/stoppedInstruction")
                || message["event"] == json!("cpclib/stoppedAt")
        })
        .expect("the stop is announced")["body"]["instruction"]
        .clone()
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
    let out = stop_over(
        "\tld a,STATE",
        Some(image_holding_ld_a_1()),
        Some(&[0x3E, 0x01])
    );

    let instruction = hint(&out).as_str().unwrap_or_default().to_lowercase();
    assert!(
        instruction.starts_with("ld a") && instruction.contains('1'),
        "the resolved instruction: {out:?}"
    );
}

/// The bytes come from the emulator, not from the assembled image.
///
/// Every reason the hint exists at all is a reason the two differ: an
/// instruction can have been modified in place, one written instruction can be
/// several real ones, and code generated at run time sits on a line that
/// assembles to filler.
#[test]
fn the_hint_reads_the_emulator_rather_than_the_image() {
    // The image says `ld a,0x01`; the machine has since been made to hold
    // `ld a,0x42`.
    let out = stop_over(
        "\tld a,STATE",
        Some(image_holding_ld_a_1()),
        Some(&[0x3E, 0x42])
    );

    let instruction = hint(&out).as_str().unwrap_or_default().to_lowercase();
    assert!(
        instruction.contains("42"),
        "what the machine holds, not what was assembled: {out:?}"
    );
}

/// The case the hint is mandatory for: a line that assembles to filler and
/// holds code written there at run time.
///
/// Nothing in the image can answer this - the bytes it has are the `defs`
/// fill - so a hint read from it would be actively misleading rather than
/// merely absent.
#[test]
fn a_line_that_only_reserves_space_still_gets_a_hint() {
    let mut image = vec![0u8; 0x1_0000];
    // `defs 2` - zeroes, which decode as `nop`.
    image[0x4000] = 0x00;
    image[0x4001] = 0x00;
    let out = stop_over("\tdefs 2", Some(image), Some(&[0xCD, 0x00, 0xC0]));

    let instruction = hint(&out).as_str().unwrap_or_default().to_lowercase();
    assert!(
        instruction.starts_with("call"),
        "the generated code, not the reserved space: {out:?}"
    );
}

/// A line already written as the machine holds it has nothing to disambiguate,
/// and a hint repeating it is noise - including when only the base, the case
/// or the spacing differ.
#[test]
fn a_line_that_already_says_it_gets_no_hint() {
    for written in ["\tLD A, 0x01", "\tld a,1", ".here\tld a,0x1 ; the state"] {
        let out = stop_over(written, Some(image_holding_ld_a_1()), Some(&[0x3E, 0x01]));
        assert_eq!(
            hint(&out),
            json!(null),
            "`{written}` says it already: {out:?}"
        );
    }
}

/// A read the emulator could not answer falls back to the assembled image
/// rather than leaving the line bare.
#[test]
fn an_unanswerable_read_falls_back_to_the_image() {
    let out = stop_over("\tld a,STATE", Some(image_holding_ld_a_1()), None);

    let instruction = hint(&out).as_str().unwrap_or_default().to_lowercase();
    assert!(
        instruction.starts_with("ld a") && instruction.contains('1'),
        "the image, since the emulator said nothing: {out:?}"
    );
}

/// The reveal is not held up for the hint: the editor is told where the
/// program stopped before the bytes at `PC` have been asked for.
#[test]
fn the_stop_reaches_the_editor_before_the_hint_does() {
    let out = stop_over(
        "\tld a,STATE",
        Some(image_holding_ld_a_1()),
        Some(&[0x3E, 0x01])
    );

    let reveal = out
        .iter()
        .position(|message| message["event"] == json!("cpclib/stoppedAt"))
        .expect("the stop is announced");
    let announced = out
        .iter()
        .position(|message| message["event"] == json!("cpclib/stoppedInstruction"))
        .expect("the hint follows");
    assert!(reveal < announced, "{out:?}");
    assert_eq!(
        out[reveal]["body"]["line"],
        json!(3),
        "and carries the line on its own: {out:?}"
    );
}

/// Without the program's image there is still the emulator, which is the
/// better answer anyway.
#[test]
fn no_image_is_no_obstacle_to_a_hint() {
    let out = stop_over("\tld a,STATE", None, Some(&[0x3E, 0x01]));

    let instruction = hint(&out).as_str().unwrap_or_default().to_lowercase();
    assert!(instruction.starts_with("ld a"), "{out:?}");
}

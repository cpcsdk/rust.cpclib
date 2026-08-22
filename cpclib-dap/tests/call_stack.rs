//! The call stack, end to end.
//!
//! The emulator answers `stackTrace` with the program counter alone. These
//! drive the whole recovery - scopes, registers, memory, walk - through a
//! recording peer, because the interesting failures are in the chain rather
//! than in the walk (which `callstack`'s own tests cover).

use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::peer::RecordingPeer;
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::{Value, json};

/// `main.asm`: line 10 is a `CALL` at 0x4000, line 20 is the routine at 0x5000.
fn map() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![
            SourceMapRow::flat(0, 10, 0x4000, 3),
            SourceMapRow::flat(0, 20, 0x5000, 3),
        ]
    })
    .with_symbols(
        [("play_music".to_string(), 0x5000u32)]
            .into_iter()
            .collect()
    )
}

/// 64K with `CALL 0x5000` assembled at 0x4000.
fn image() -> Vec<u8> {
    let mut memory = vec![0u8; 0x1_0000];
    memory[0x4000] = 0xCD;
    memory[0x4001] = 0x00;
    memory[0x4002] = 0x50;
    memory
}

fn session() -> Session<RecordingPeer> {
    let mut session = Session::new(RecordingPeer::new(), map())
        .with_image(image())
        .with_top_of_stack(0xC000);
    session.on_attached().unwrap();
    session
}

/// The emulator's answer to the editor's `stackTrace`: one frame, at 0x5001.
fn emulator_stack_trace() -> Value {
    json!({
        "seq": 5, "type": "response", "request_seq": 3, "success": true,
        "command": "stackTrace",
        "body": {
            "stackFrames": [{
                "id": 17, "name": "Z80 @ 0x5001", "line": 0, "column": 0,
                "instructionPointerReference": "0x5001"
            }],
            "totalFrames": 1
        }
    })
}

fn scopes_answer(seq: i64) -> Value {
    json!({
        "seq": 6, "type": "response", "request_seq": seq, "success": true,
        "command": "scopes",
        "body": {"scopes": [{
            "name": "Registers", "presentationHint": "registers",
            "variablesReference": 18
        }]}
    })
}

fn registers_answer(seq: i64, sp: u16) -> Value {
    json!({
        "seq": 7, "type": "response", "request_seq": seq, "success": true,
        "command": "variables",
        "body": {"variables": [
            {"name": "AF", "value": "0x0044", "variablesReference": 0},
            {"name": "SP", "value": format!("0x{sp:04X}"), "variablesReference": 0},
            {"name": "PC", "value": "0x5001", "variablesReference": 0},
        ]}
    })
}

fn memory_answer(seq: i64, address: u16, words: &[u16]) -> Value {
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    json!({
        "seq": 8, "type": "response", "request_seq": seq, "success": true,
        "command": "readMemory",
        "body": {
            "address": format!("0x{address:04X}"),
            "data": base64(&bytes)
        }
    })
}

/// The seq of the last request the adapter sent with `command`.
fn seq_of(session: &Session<RecordingPeer>, command: &str) -> i64 {
    session
        .peer()
        .last(command)
        .and_then(|m| m.get("seq"))
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("no {command} request was sent"))
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0)
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * i)) & 0x3F) as usize] as char);
            }
            else {
                out.push('=');
            }
        }
    }
    out
}

/// The whole chain: one frame in, two frames out, the caller named and placed
/// on the line of the `CALL`.
#[test]
fn a_return_address_on_the_stack_becomes_a_second_frame() {
    let mut session = session();

    // The emulator answers the editor's stackTrace. Nothing goes out yet.
    let answered = session.on_emulator_message(&emulator_stack_trace());
    assert!(
        answered.is_empty(),
        "the answer waits while the stack is read"
    );

    let scopes = seq_of(&session, "scopes");
    assert!(
        session
            .on_emulator_message(&scopes_answer(scopes))
            .is_empty()
    );

    let variables = seq_of(&session, "variables");
    assert!(
        session
            .on_emulator_message(&registers_answer(variables, 0xBFF0))
            .is_empty()
    );

    // ...and the stack is read from SP, stopping at the top of stack.
    let read = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(read["arguments"]["memoryReference"], json!("0xbff0"));
    assert_eq!(read["arguments"]["count"], json!(16), "0xC000 - 0xBFF0");

    let seq = read["seq"].as_i64().unwrap();
    let out = session.on_emulator_message(&memory_answer(seq, 0xBFF0, &[0x4003]));

    let response = out.last().unwrap();
    let frames = response["body"]["stackFrames"].as_array().unwrap();
    assert_eq!(frames.len(), 2, "{response}");
    assert_eq!(frames[0]["instructionPointerReference"], json!("0x5001"));

    let caller = &frames[1];
    assert_eq!(
        caller["name"],
        json!("play_music @ 0x5000"),
        "named by the CALL target"
    );
    assert_eq!(
        caller["instructionPointerReference"],
        json!("0x4000"),
        "the frame sits on the CALL, not on the return address"
    );
    assert_eq!(caller["line"], json!(10), "and gets its source line");
    assert_eq!(caller["source"]["name"], json!("main.asm"));
    assert_eq!(response["body"]["totalFrames"], json!(2));
}

/// Values the routine pushed are reported with the frame that pushed them,
/// rather than becoming frames of their own.
#[test]
fn pushed_values_are_counted_not_invented() {
    let mut session = session();
    session.on_emulator_message(&emulator_stack_trace());
    let scopes = seq_of(&session, "scopes");
    session.on_emulator_message(&scopes_answer(scopes));
    let variables = seq_of(&session, "variables");
    session.on_emulator_message(&registers_answer(variables, 0xBFF0));
    let read = seq_of(&session, "readMemory");

    let out = session.on_emulator_message(&memory_answer(read, 0xBFF0, &[0x1234, 0x5678, 0x4003]));
    let frames = out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(frames.len(), 2, "two words of data made no frames");
    assert_eq!(frames[1]["name"], json!("play_music @ 0x5000 [2 pushed]"));
}

/// An emulator that cannot answer part of the chain still gets a stack trace
/// out - the one frame it did give.
#[test]
fn a_broken_chain_falls_back_to_the_single_frame() {
    let mut session = session();
    session.on_emulator_message(&emulator_stack_trace());
    let scopes = seq_of(&session, "scopes");

    // No registers scope in the answer at all.
    let out = session.on_emulator_message(&json!({
        "seq": 6, "type": "response", "request_seq": scopes, "success": true,
        "command": "scopes", "body": {"scopes": []}
    }));

    let frames = out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0]["line"], json!(20), "and is still annotated");
}

/// Without the program's bytes there is nothing to check candidates against,
/// so the old single-frame behaviour stands and no extra requests are made.
#[test]
fn a_session_without_an_image_asks_nothing_extra() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.on_attached().unwrap();

    let out = session.on_emulator_message(&emulator_stack_trace());
    // The stack trace, plus the "we stopped here" event the extension uses to
    // open the line itself. Nothing was *asked of the emulator*, which is what
    // this test is about.
    let answer = out.last().unwrap();
    assert_eq!(
        answer["command"],
        json!("stackTrace"),
        "answered straight through"
    );
    assert_eq!(answer["body"]["stackFrames"].as_array().unwrap().len(), 1);
    assert!(
        out.iter().all(|m| {
            m["command"] == json!("stackTrace") || m["event"] == json!("cpclib/stoppedAt")
        }),
        "and nothing else was sent: {out:?}"
    );
    assert!(!session.peer().commands().contains(&"scopes".to_string()));
}

/// The frames we invent must not be mistaken for the emulator's own, whose ids
/// it validates and whose registers are not ours to hand out.
#[test]
fn synthetic_frames_get_ids_of_their_own() {
    let mut session = session();
    session.on_emulator_message(&emulator_stack_trace());
    let scopes = seq_of(&session, "scopes");
    session.on_emulator_message(&scopes_answer(scopes));
    let variables = seq_of(&session, "variables");
    session.on_emulator_message(&registers_answer(variables, 0xBFF0));
    let read = seq_of(&session, "readMemory");
    let out = session.on_emulator_message(&memory_answer(read, 0xBFF0, &[0x4003]));

    let frames = out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        frames[0]["id"],
        json!(17),
        "the emulator's own is untouched"
    );
    assert_ne!(frames[1]["id"], json!(17));
    assert!(frames[1]["id"].as_i64().unwrap() > 0x1000_0000);
}

/// The reads that are not this stop's instruction hint.
///
/// Every stop asks the emulator for the four bytes at `PC`: what is really
/// executing there is not always what was assembled, and only the emulator
/// knows. That read is not what any of these tests is about.
fn reads_beyond_the_stop_hint(session: &Session<RecordingPeer>, pc: u16) -> usize {
    session
        .peer()
        .sent
        .iter()
        .filter(|message| message["command"] == json!("readMemory"))
        .filter(|message| {
            let arguments = &message["arguments"];
            !(arguments["count"] == json!(4)
                && arguments["memoryReference"] == json!(format!("0x{pc:04x}")))
        })
        .count()
}

/// A top of stack below SP means the stack is empty; nothing is read, and the
/// single frame goes out rather than the whole address space being walked.
#[test]
fn an_empty_stack_reads_no_memory() {
    let mut session = Session::new(RecordingPeer::new(), map())
        .with_image(image())
        .with_top_of_stack(0xBFF0);
    session.on_attached().unwrap();

    session.on_emulator_message(&emulator_stack_trace());
    let scopes = seq_of(&session, "scopes");
    session.on_emulator_message(&scopes_answer(scopes));
    let variables = seq_of(&session, "variables");
    let out = session.on_emulator_message(&registers_answer(variables, 0xBFF0));

    assert_eq!(reads_beyond_the_stop_hint(&session, 0x5001), 0);
    assert_eq!(
        out.last().unwrap()["body"]["stackFrames"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

/// The top of stack is taken from the program's own symbols when the launch
/// configuration does not name one.
#[test]
fn the_top_of_stack_comes_from_the_program_when_not_configured() {
    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![SourceMapRow::flat(0, 10, 0x4000, 3)]
    })
    .with_symbols([("stack_top".to_string(), 0xB000u32)].into_iter().collect());

    let mut session = Session::new(RecordingPeer::new(), map)
        .with_image(image())
        .top_of_stack_from_symbols();
    session.on_attached().unwrap();

    session.on_emulator_message(&emulator_stack_trace());
    let scopes = seq_of(&session, "scopes");
    session.on_emulator_message(&scopes_answer(scopes));
    let variables = seq_of(&session, "variables");
    session.on_emulator_message(&registers_answer(variables, 0xAFF0));

    let read = session.peer().last("readMemory").unwrap();
    assert_eq!(
        read["arguments"]["count"],
        json!(16),
        "0xB000 - 0xAFF0, so the walk stops at the program's own stack top"
    );
}

/// Drive the whole chain and hand back the frames.
fn frames_after_walk(session: &mut Session<RecordingPeer>, stack: &[u16]) -> Vec<Value> {
    session.on_emulator_message(&emulator_stack_trace());
    let scopes = seq_of(session, "scopes");
    session.on_emulator_message(&scopes_answer(scopes));
    let variables = seq_of(session, "variables");
    session.on_emulator_message(&registers_answer(variables, 0xBFF0));
    let read = seq_of(session, "readMemory");
    let out = session.on_emulator_message(&memory_answer(read, 0xBFF0, stack));
    out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap()
        .clone()
}

/// Clicking a reconstructed frame shows what the stack still holds, instead of
/// "Stack frame reference has expired".
///
/// The emulator validates frame ids against its own and refuses every other
/// one - which is not a useful answer to "what is in this frame". These are
/// frames we invented, so they are ours to describe.
#[test]
fn a_reconstructed_frame_answers_for_itself() {
    let mut session = session();
    let frames = frames_after_walk(&mut session, &[0xDEAD, 0x4003]);
    let frame_id = frames[1]["id"].as_i64().unwrap();

    let scopes = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "scopes",
            "arguments": {"frameId": frame_id}
        }))
        .unwrap();
    assert_eq!(scopes.len(), 1, "answered here, never forwarded");
    let reference = scopes[0]["body"]["scopes"][0]["variablesReference"]
        .as_i64()
        .unwrap();

    let variables = session
        .on_editor_message(&json!({
            "seq": 2, "type": "request", "command": "variables",
            "arguments": {"variablesReference": reference}
        }))
        .unwrap();
    let listed = variables[0]["body"]["variables"].as_array().unwrap();

    let value_of = |name: &str| {
        listed
            .iter()
            .find(|v| v["name"] == json!(name))
            .and_then(|v| v["value"].as_str())
            .unwrap_or_else(|| panic!("no {name}: {listed:?}"))
    };
    assert_eq!(value_of("called"), "0x5000 (play_music)");
    assert_eq!(value_of("call site"), "0x4000 (main.asm:10)");
    assert_eq!(value_of("returns to"), "0x4003");

    // The word that frame pushed is there too - it fell out of the walk.
    assert_eq!(value_of("pushed[0]"), "0xDEAD");

    // ...and the registers are honestly reported as gone, with the reason.
    let registers = listed
        .iter()
        .find(|v| v["name"] == json!("registers"))
        .unwrap();
    assert_eq!(
        registers["value"],
        json!("not available for an outer frame")
    );
    let why = registers["type"].as_str().unwrap();
    assert!(why.contains("CALL pushes the return address"), "{why}");
}

/// The innermost frame is still the emulator's: its registers are real, and
/// asking us for them would be wrong.
#[test]
fn the_emulators_own_frame_is_still_forwarded() {
    let mut session = session();
    let frames = frames_after_walk(&mut session, &[0x4003]);

    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "scopes",
            "arguments": {"frameId": frames[0]["id"]}
        }))
        .unwrap();
    assert!(
        session
            .peer()
            .commands()
            .iter()
            .filter(|c| *c == "scopes")
            .count()
            >= 2,
        "the emulator was asked"
    );
}

/// The synthetic ids must not swallow the other references this adapter
/// answers for.
///
/// The flag and chip scopes live at `0x7C00_000x`, which is numerically *above*
/// the synthetic base - so an open-ended "is it big enough" test answers a
/// register pane with a stack frame. It did, until this.
#[test]
fn synthetic_ids_do_not_swallow_the_other_scopes() {
    let mut session = session();

    // The flags scope answers here and immediately.
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "variables",
            "arguments": {"variablesReference": 0x7C00_0003i64}
        }))
        .unwrap();
    let listed = out[0]["body"]["variables"].as_array().unwrap();
    assert!(
        listed.iter().any(|v| v["name"] == json!("Z")),
        "the flags, not a stack frame: {listed:?}"
    );

    // The chip scopes go to the machine-state path, not the frame path: they
    // are held while the emulator writes a snapshot of itself.
    for reference in [0x7C00_0001i64, 0x7C00_0002, 0x7C00_0004, 0x7C00_0005] {
        let out = session
            .on_editor_message(&json!({
                "seq": 1, "type": "request", "command": "variables",
                "arguments": {"variablesReference": reference}
            }))
            .unwrap();
        assert!(
            out.is_empty(),
            "0x{reference:X} was answered as a stack frame: {out:?}"
        );
    }
    assert!(
        session
            .peer()
            .commands()
            .contains(&"cpclib/machineState".to_string()),
        "the chips were asked for, not invented"
    );
}

/// Two pages, but only one of them has code at the address in question.
fn one_page_claims_it() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["lib.asm".into(), "gfx.asm".into()],
        rows: vec![
            // page 0 holds the `ret` at 0x04A5...
            SourceMapRow {
                file: 0,
                line: 40,
                logical: 0x04A5,
                physical: 0x04A5,
                page: 0,
                column: 2,
                column_end: 5,
                len: 1,
                is_data: false
            },
            // ...and page 1 has code, but somewhere else entirely.
            SourceMapRow {
                file: 1,
                line: 7,
                logical: 0x8000,
                physical: 0x1_8000,
                page: 1,
                column: 2,
                column_end: 5,
                len: 1,
                is_data: false
            },
            // Something both pages claim, so the program counts as banked.
            SourceMapRow {
                file: 0,
                line: 71,
                logical: 0x5C3A,
                physical: 0x5C3A,
                page: 0,
                column: 2,
                column_end: 5,
                len: 1,
                is_data: false
            },
            SourceMapRow {
                file: 1,
                line: 242,
                logical: 0x5C3A,
                physical: 0x1_5C3A,
                page: 1,
                column: 2,
                column_end: 5,
                len: 1,
                is_data: false
            },
        ]
    })
}

fn stopped_at(address: u16) -> Value {
    json!({
        "seq": 5, "type": "response", "request_seq": 3, "success": true,
        "command": "stackTrace",
        "body": {
            "stackFrames": [{
                "id": 17,
                "name": format!("Z80 @ 0x{address:04X}"),
                "line": 0, "column": 0,
                "instructionPointerReference": format!("0x{address:04X}")
            }],
            "totalFrames": 1
        }
    })
}

/// An address only one page assembled into resolves to that page's line, in a
/// banked program, with nothing asked of the emulator.
///
/// The program banks *somewhere*, but that says nothing about `0x04A5` - and
/// showing disassembly for an address exactly one line was assembled at is
/// giving up on a question with one answer.
#[test]
fn an_address_only_one_page_claims_needs_no_disambiguation() {
    let mut session = Session::new(RecordingPeer::new(), one_page_claims_it());
    session.on_attached().unwrap();

    let out = session.on_emulator_message(&stopped_at(0x04A5));
    let frames = out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap();

    assert_eq!(frames[0]["line"], json!(40), "{out:?}");
    assert_eq!(frames[0]["source"]["name"], json!("lib.asm"));
    assert_eq!(
        reads_beyond_the_stop_hint(&session, 0x04A5),
        0,
        "nothing to tell apart, so nothing is read"
    );
}

/// An address two pages claim is decided by what is really in memory.
///
/// The stack trace is what the editor uses to choose between your source and
/// the disassembly view, so the bank has to be settled *before* it goes out.
#[test]
fn a_contested_address_is_decided_by_the_bytes_in_memory() {
    let mut image = vec![0u8; 0x2_0000];
    // Page 0 holds `ld a,1` at 0x5C3A; page 1 holds nothing there.
    image[0x5C3A] = 0x3E;
    image[0x5C3B] = 0x01;
    let mut session = Session::new(RecordingPeer::new(), one_page_claims_it()).with_image(image);
    session.on_attached().unwrap();

    // The walk runs first, then the page probe.
    let held = session.on_emulator_message(&stopped_at(0x5C3A));
    assert!(held.is_empty(), "the answer waits");
    let scopes = seq_of(&session, "scopes");
    session.on_emulator_message(&scopes_answer(scopes));
    let variables = seq_of(&session, "variables");
    session.on_emulator_message(&registers_answer(variables, 0xBFF0));
    let stack = seq_of(&session, "readMemory");
    session.on_emulator_message(&memory_answer(stack, 0xBFF0, &[]));

    // Now the probe: what is really at the contested address?
    let probe = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(probe["arguments"]["memoryReference"], json!("0x5c3a"));
    let out = session.on_emulator_message(&json!({
        "seq": 8, "type": "response", "request_seq": probe["seq"], "success": true,
        "command": "readMemory",
        "body": {"address": "0x5C3A", "data": base64(&[0x3E, 0x01])}
    }));

    let frames = out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap();
    assert_eq!(
        frames[0]["line"],
        json!(71),
        "page 0's line, from its bytes"
    );
    assert_eq!(frames[0]["source"]["name"], json!("lib.asm"));
}

/// When the bytes match both pages equally there is nothing to choose, and the
/// message says that is what happened rather than leaving it a mystery.
#[test]
fn bytes_that_match_both_pages_are_reported_not_guessed() {
    let mut session =
        Session::new(RecordingPeer::new(), one_page_claims_it()).with_image(vec![0u8; 0x2_0000]);
    session.on_attached().unwrap();
    session.on_emulator_message(&stopped_at(0x5C3A));
    let scopes = seq_of(&session, "scopes");
    session.on_emulator_message(&scopes_answer(scopes));
    let variables = seq_of(&session, "variables");
    session.on_emulator_message(&registers_answer(variables, 0xBFF0));
    let stack = seq_of(&session, "readMemory");
    session.on_emulator_message(&memory_answer(stack, 0xBFF0, &[]));

    let probe = session.peer().last("readMemory").unwrap().clone();
    let out = session.on_emulator_message(&json!({
        "seq": 8, "type": "response", "request_seq": probe["seq"], "success": true,
        "command": "readMemory",
        "body": {"address": "0x5C3A", "data": base64(&[0x00, 0x00])}
    }));

    let note = out
        .iter()
        .find(|m| m["event"] == json!("output"))
        .expect("says why");
    let text = note["body"]["output"].as_str().unwrap();
    assert!(text.contains("matched them equally well"), "{text}");
    assert!(text.contains("-dv"), "and points at what to do: {text}");
}

/// A session with no program image cannot compare anything, and says so -
/// naming the configuration problem rather than reporting a mysterious tie.
#[test]
fn without_an_image_the_report_names_the_real_problem() {
    let mut session = Session::new(RecordingPeer::new(), one_page_claims_it());
    session.on_attached().unwrap();

    let out = session.on_emulator_message(&stopped_at(0x5C3A));
    let note = out
        .iter()
        .find(|m| m["event"] == json!("output"))
        .expect("says why");
    let text = note["body"]["output"].as_str().unwrap();
    assert!(text.contains("not available to this session"), "{text}");
    assert_eq!(
        reads_beyond_the_stop_hint(&session, 0x5C3A),
        0,
        "and does not spend a read to learn nothing"
    );
}

/// A stop resolves through the emulator's paging, with no stack walk involved.
///
/// This is the `0x79F3` bug: two files hold code at one logical address -
/// `writter.asm` in page 0, `animate.asm` in page 1 - and the answer fell back
/// to the lower page, so a breakpoint in one opened the other. The assembler
/// was right throughout: its listing records the physical address `0x179F3`,
/// which names page 1 unambiguously.
#[test]
fn a_stop_resolves_through_the_emulators_paging_without_a_walk() {
    let peer = RecordingPeer::new().also_supporting(&["cpclib/memmap"]);
    // No image: the page comes from the Gate Array's MMR, not from comparing
    // bytes, so none is needed.
    let mut session = Session::new(peer, one_page_claims_it());
    session.on_attached().unwrap();

    let held = session.on_emulator_message(&stopped_at(0x5C3A));
    assert!(held.is_empty(), "answered once the page is known");

    let asked = session
        .peer()
        .last("cpclib/memmap")
        .expect("the emulator was asked which page is where");
    // MM 2: the whole address space comes from the selected page.
    let out = session.on_emulator_message(&json!({
        "seq": 8, "type": "response", "request_seq": asked["seq"], "success": true,
        "command": "cpclib/memmap",
        "body": { "ram_mode": 2, "ram_page": 1, "rmr": 140 }
    }));

    let frames = out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap();
    assert_eq!(frames[0]["line"], json!(242), "page 1's line: {frames:?}");
    assert_eq!(frames[0]["source"]["name"], json!("gfx.asm"));
    assert!(
        !session.peer().commands().contains(&"scopes".to_string()),
        "no stack walk was needed: {:?}",
        session.peer().commands()
    );
}

/// The region list wins over `ram_page`, because `ram_page` lies.
///
/// AmspiritLite answers mode 5 with `ram_page: 0` and the truth in
/// `regions[].ram_bank`, which is an **absolute** 16K bank index: bank 5 is
/// page 1's second bank, physical `0x14000-0x17FFF`. Believing `ram_page` gave
/// page 0 and opened the wrong file - the whole `0x79F3` hunt in one fixture.
#[test]
fn the_region_list_is_believed_over_the_page_number() {
    let peer = RecordingPeer::new().also_supporting(&["cpclib/memmap"]);
    let mut session = Session::new(peer, one_page_claims_it());
    session.on_attached().unwrap();

    session.on_emulator_message(&stopped_at(0x5C3A));
    let asked = session.peer().last("cpclib/memmap").expect("asked");
    let out = session.on_emulator_message(&json!({
        "seq": 8, "type": "response", "request_seq": asked["seq"], "success": true,
        "command": "cpclib/memmap",
        "body": {
            "ram_mode": 5,
            "ram_page": 0,
            "regions": [
                { "base": 0, "rom": false, "ram_bank": 0 },
                { "base": 16384, "ext": true, "rom": false, "ram_bank": 5 },
                { "base": 32768, "rom": false, "ram_bank": 2 },
                { "base": 49152, "rom": false, "ram_bank": 3 }
            ]
        }
    }));

    let frames = out.last().unwrap()["body"]["stackFrames"]
        .as_array()
        .unwrap();
    assert_eq!(
        frames[0]["source"]["name"],
        json!("gfx.asm"),
        "page 1, not the page `ram_page` claimed: {frames:?}"
    );
    assert_eq!(frames[0]["line"], json!(242));
}

/// The `call` in the source says which label was meant.
///
/// Two labels at one address and no rule about their spelling separates them:
/// a real stack named a frame `PLY_AKG_DisarkWordRegionEnd_50` where the source
/// plainly read `call spectral_sprite_move_along_curve` - the wrong name being
/// both *shorter* and not ending in `_end`. The line the call was made from is
/// the evidence, and it is unambiguous.
#[test]
fn the_call_site_says_which_label_was_called() {
    let source = std::env::temp_dir().join("cpclib-call-site-test.asm");
    std::fs::write(
        &source,
        "\torg 0x4000\n\tnop\n\tcall spectral_sprite_move_along_curve\n\tret\n"
    )
    .unwrap();

    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec![source.to_string_lossy().to_string()],
        rows: vec![
            SourceMapRow::flat(0, 2, 0x4000, 1),
            // The `call`, on line 3, at 0x4001.
            SourceMapRow::flat(0, 3, 0x4001, 3),
        ]
    })
    .with_symbols(
        [
            ("PLY_AKG_DisarkWordRegionEnd_50".to_string(), 0x2E4Au32),
            ("spectral_sprite_move_along_curve".to_string(), 0x2E4A)
        ]
        .into_iter()
        .collect()
    );

    // The bytes have to *be* a `call 0x2E4A` at 0x4001, or the walk rightly
    // refuses to believe 0x4004 is a return address.
    let mut image = vec![0u8; 0x1_0000];
    image[0x4001] = 0xCD;
    image[0x4002] = 0x4A;
    image[0x4003] = 0x2E;
    let mut session = Session::new(RecordingPeer::new(), map).with_image(image);
    session.on_attached().unwrap();

    // A stack holding the return address just after that `call`.
    let frames = frames_after_walk(&mut session, &[0x4004]);
    let named = frames
        .iter()
        .map(|f| f["name"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(
        named
            .iter()
            .any(|name| name.starts_with("spectral_sprite_move_along_curve @ 0x2E4A")),
        "the label the call names, not the shorter one: {named:?}"
    );

    let _ = std::fs::remove_file(&source);
}

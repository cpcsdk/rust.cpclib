//! The editor speaks files and lines; the emulator speaks addresses. These
//! pin the translation in both directions, against a recording peer rather
//! than a browser.

use std::path::{Path, PathBuf};

use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::peer::{LineAtPc, RecordingPeer};
use cpclib_dap::protocol;
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::{Value, json};

/// A program: `main.asm` lines 10/11/12 at 0x4000/0x4003/0x4004, and
/// `inc.asm` line 5 at 0x5000. Line 20 of main.asm emits nothing.
fn fixture() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into(), "inc.asm".into()],
        rows: vec![
            SourceMapRow::flat(0, 10, 0x4000, 3),
            SourceMapRow::flat(0, 11, 0x4003, 1),
            SourceMapRow::flat(0, 12, 0x4004, 2),
            SourceMapRow::flat(1, 5, 0x5000, 1),
        ]
    })
}

fn session() -> Session<RecordingPeer> {
    let mut session = Session::new(RecordingPeer::new(), fixture());
    session.on_attached().unwrap();
    session
}

fn set_breakpoints(session: &mut Session<RecordingPeer>, file: &str, lines: &[u32]) -> Value {
    let request = json!({
        "seq": 1, "type": "request", "command": "setBreakpoints",
        "arguments": {
            "source": {"path": file},
            "breakpoints": lines.iter().map(|l| json!({"line": l})).collect::<Vec<_>>()
        }
    });
    session
        .on_editor_message(&request)
        .unwrap()
        .into_iter()
        .next()
        .expect("setBreakpoints is answered by the adapter itself")
}

#[test]
fn a_source_breakpoint_becomes_an_address() {
    let mut session = session();
    let answer = set_breakpoints(&mut session, "main.asm", &[11]);

    let breakpoints = answer["body"]["breakpoints"].as_array().unwrap();
    assert_eq!(breakpoints.len(), 1);
    assert_eq!(breakpoints[0]["verified"], json!(true));
    assert_eq!(breakpoints[0]["line"], json!(11));
    assert_eq!(breakpoints[0]["instructionReference"], json!("0x4003"));

    // ...and the emulator was told, in its own vocabulary.
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4003"}])
    );
}

/// A breakpoint on a line with no code slides, and the editor is told where it
/// went so it can move the dot.
#[test]
fn a_breakpoint_on_a_line_without_code_reports_where_it_landed() {
    let mut session = session();
    // Line 9 has no row; the next line that does is 10.
    let answer = set_breakpoints(&mut session, "main.asm", &[9]);
    let breakpoint = &answer["body"]["breakpoints"][0];
    assert_eq!(breakpoint["verified"], json!(true));
    assert_eq!(breakpoint["line"], json!(10), "the dot moves to the code");
    assert_eq!(session.moved_breakpoints(), vec![(9, 10)]);
}

/// Past the end of the program there is nothing to slide to, and saying so is
/// better than pretending.
#[test]
fn a_breakpoint_with_no_code_after_it_is_unverified() {
    let mut session = session();
    let answer = set_breakpoints(&mut session, "main.asm", &[900]);
    let breakpoint = &answer["body"]["breakpoints"][0];
    assert_eq!(breakpoint["verified"], json!(false));
    assert!(
        breakpoint["message"].as_str().unwrap().contains("no code"),
        "the reason is given: {breakpoint}"
    );
    // Nothing unverified is sent to the emulator.
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(sent["arguments"]["breakpoints"], json!([]));
}

/// `setBreakpoints` is per file; the emulator's set is global. Setting one
/// file's breakpoints must not clear another's.
#[test]
fn breakpoints_in_two_files_are_sent_as_a_union() {
    let mut session = session();
    set_breakpoints(&mut session, "main.asm", &[10]);
    set_breakpoints(&mut session, "inc.asm", &[5]);

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    let addresses: Vec<&str> = sent["arguments"]["breakpoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["instructionReference"].as_str().unwrap())
        .collect();
    assert_eq!(addresses, vec!["0x4000", "0x5000"], "both files survive");
}

/// Clearing one file's breakpoints leaves the other's alone.
#[test]
fn clearing_one_file_keeps_the_other() {
    let mut session = session();
    set_breakpoints(&mut session, "main.asm", &[10]);
    set_breakpoints(&mut session, "inc.asm", &[5]);
    set_breakpoints(&mut session, "main.asm", &[]);

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x5000"}])
    );
}

/// The emulator answers a stack trace with addresses and `line: 0`; the editor
/// needs a file and a line.
#[test]
fn a_stack_trace_gets_its_source_back() {
    let mut session = session();
    let from_emulator = json!({
        "seq": 5, "type": "response", "request_seq": 4, "success": true,
        "command": "stackTrace",
        "body": {
            "stackFrames": [{
                "id": 17, "name": "Z80 @ 0x4004", "line": 0, "column": 0,
                "instructionPointerReference": "0x4004"
            }],
            "totalFrames": 1
        }
    });
    let out = session.on_emulator_message(&from_emulator);
    // The answer is last; a `cpclib/stoppedAt` event may precede it.
    let frame = &out.last().unwrap()["body"]["stackFrames"][0];
    assert_eq!(frame["line"], json!(12));
    assert_eq!(frame["source"]["name"], json!("main.asm"));
}

/// A PC in the middle of an instruction still belongs to that instruction.
#[test]
fn a_stack_trace_inside_an_instruction_resolves_to_its_line() {
    let mut session = session();
    let from_emulator = json!({
        "type": "response", "command": "stackTrace", "success": true,
        "body": {"stackFrames": [{"instructionPointerReference": "0x4001"}]}
    });
    let out = session.on_emulator_message(&from_emulator);
    assert_eq!(
        out.last().unwrap()["body"]["stackFrames"][0]["line"],
        json!(10)
    );
}

/// An address belonging to no source line is left alone rather than attributed
/// to whatever is nearest - the editor then shows disassembly, which is honest.
#[test]
fn a_stack_trace_outside_the_program_is_left_unannotated() {
    let mut session = session();
    let from_emulator = json!({
        "type": "response", "command": "stackTrace", "success": true,
        "body": {"stackFrames": [{"instructionPointerReference": "0xbb5a", "line": 0}]}
    });
    let out = session.on_emulator_message(&from_emulator);
    let frame = &out[0]["body"]["stackFrames"][0];
    assert_eq!(frame["line"], json!(0), "not invented");
    assert!(frame.get("source").is_none());
}

/// Everything that is not source-shaped goes straight through, unchanged -
/// stepping, memory, disassembly.
#[test]
fn other_requests_are_forwarded_untouched() {
    // Every button on the debug toolbar, plus the memory and disassembly views.
    const FORWARDED: &[&str] = &[
        "continue",
        "next",
        "stepIn",
        "stepOut",
        "stepBack",
        "reverseContinue",
        "pause",
        "scopes",
        "variables",
        "readMemory",
        "writeMemory"
    ];

    let mut session = session();
    for command in FORWARDED {
        let request = json!({"seq": 1, "type": "request", "command": command});
        let answered = session.on_editor_message(&request).unwrap();
        assert!(answered.is_empty(), "{command} is not answered locally");
    }
    let commands = session.peer().commands();
    for command in FORWARDED {
        assert!(
            commands.contains(&command.to_string()),
            "{command} never reached the emulator"
        );
    }
}

/// Events from the emulator reach the editor unchanged.
#[test]
fn emulator_events_pass_through() {
    let mut session = session();
    let output = json!({
        "type": "event", "event": "output",
        "body": {"category": "console", "output": "hello"}
    });
    assert_eq!(session.on_emulator_message(&output), vec![output.clone()]);
}

/// A stop is what turns the editor's toolbar on, so it must carry a thread to
/// act on even when the emulator did not name one.
#[test]
fn a_stop_always_names_a_thread() {
    let mut session = session();
    let stopped = json!({
        "type": "event", "event": "stopped",
        "body": {"reason": "instruction breakpoint", "hitBreakpointIds": [2]}
    });
    let out = session.on_emulator_message(&stopped);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["body"]["threadId"], json!(1));
    assert_eq!(out[0]["body"]["allThreadsStopped"], json!(true));
    // ...and what the emulator did say is left alone.
    assert_eq!(out[0]["body"]["hitBreakpointIds"], json!([2]));
}

/// A thread the emulator already named is not overwritten.
///
/// Any reason but `pause` - the adapter halts the program itself at launch, and
/// swallows that one stop so the editor does not act on a pause it never asked
/// for (`program_breakpoints.rs`).
#[test]
fn a_stop_that_names_its_thread_is_left_alone() {
    let mut session = session();
    let stopped = json!({
        "type": "event", "event": "stopped",
        "body": {"reason": "breakpoint", "threadId": 7}
    });
    let out = session.on_emulator_message(&stopped);
    assert_eq!(out[0]["body"]["threadId"], json!(7));
}

/// The editor is told `initialized` once, by us, when the source map exists.
/// The emulator's own is about its readiness and would make VS Code re-send
/// every breakpoint.
#[test]
fn the_emulators_initialized_event_is_not_forwarded() {
    let mut session = session();
    let initialized = json!({"type": "event", "event": "initialized"});
    assert!(session.on_emulator_message(&initialized).is_empty());
}

/// The editor asks for threads as soon as it hears about a stop; answering
/// locally means that never races the emulator's attach.
#[test]
fn threads_is_answered_without_asking_the_emulator() {
    let mut session = session();
    let request = json!({"seq": 9, "type": "request", "command": "threads"});
    let answered = session.on_editor_message(&request).unwrap();
    assert_eq!(answered.len(), 1);
    assert_eq!(answered[0]["body"]["threads"][0]["id"], json!(1));
    assert!(
        !session.peer().commands().contains(&"threads".to_string()),
        "not forwarded"
    );
}

/// The emulator refuses breakpoints before `attach`, so they are held until it
/// is ready rather than sent and lost.
#[test]
fn breakpoints_set_before_attach_are_held_then_flushed() {
    let mut session = Session::new(RecordingPeer::new(), fixture());
    set_breakpoints(&mut session, "main.asm", &[10]);
    assert!(
        session.peer().last("setInstructionBreakpoints").is_none(),
        "nothing is sent before the emulator is attached"
    );

    session.on_attached().unwrap();
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}])
    );
}

/// A file the program was not built from has no addresses, and must not be
/// silently attributed to one that was.
#[test]
fn an_unknown_file_yields_unverified_breakpoints() {
    let mut session = session();
    let answer = set_breakpoints(&mut session, "elsewhere.asm", &[10]);
    let breakpoint = &answer["body"]["breakpoints"][0];
    assert_eq!(breakpoint["verified"], json!(false));
    assert!(
        breakpoint["message"]
            .as_str()
            .unwrap()
            .contains("not part of the program"),
        "the reason distinguishes it from a line without code: {breakpoint}"
    );
}

/// With no source map at all, every breakpoint fails - and saying "no code on
/// this line" for a file full of code sends the user hunting in the wrong
/// place. The message names the real cause and its fix.
#[test]
fn an_empty_source_map_says_so_rather_than_blaming_the_line() {
    let mut session = Session::new(RecordingPeer::new(), SourceMap::default());
    session.on_attached().unwrap();
    let answer = set_breakpoints(&mut session, "main.asm", &[10]);
    let breakpoint = &answer["body"]["breakpoints"][0];
    assert_eq!(breakpoint["verified"], json!(false));
    let message = breakpoint["message"].as_str().unwrap();
    assert!(message.contains("entry point"), "{message}");
    assert!(
        message.contains("cpclib-lsp.toml"),
        "and how to fix it: {message}"
    );
}

/// Breakpoint ids are unique and stable, so a `stopped` naming one can be
/// matched back to the dot the user clicked.
#[test]
fn breakpoint_ids_are_unique() {
    let mut session = session();
    let first = set_breakpoints(&mut session, "main.asm", &[10, 11]);
    let ids: Vec<i64> = first["body"]["breakpoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

/// The framing survives a full exchange.
#[test]
fn a_conversation_round_trips_through_the_wire_format() {
    let request = json!({"seq": 1, "type": "request", "command": "initialize"});
    let mut buffer = protocol::encode(&request).into_bytes();
    assert_eq!(protocol::decode(&mut buffer), vec![request]);
}

/// The capabilities we advertise are the ones the emulator can actually back.
#[test]
fn advertised_capabilities_match_what_the_emulator_supports() {
    let capabilities = Session::<RecordingPeer>::capabilities();
    for flag in [
        "supportsStepBack",
        "supportsReadMemoryRequest",
        "supportsWriteMemoryRequest",
        "supportsConfigurationDoneRequest"
    ] {
        assert_eq!(capabilities[flag], json!(true), "{flag}");
    }
    // Data breakpoints *are* claimed, even though the emulator has no such
    // request: they are answered here and turned into its write-watch
    // channels, which is the only way to reach those from the editor's own UI
    // rather than from launch.json. What the channels cannot do - stop the
    // program - is said in the description the editor shows, not hidden behind
    // the flag.
    assert_eq!(capabilities["supportsDataBreakpoints"], json!(true));

    // Not claimed on purpose: an editor told the adapter can disassemble opens
    // that view the moment a frame lacks a source line, and stays there. A stop
    // should land on your source; `-dv` opens disassembly when you want it.
    assert!(capabilities.get("supportsDisassembleRequest").is_none());

    // Claimed, and answered here rather than forwarded: only `PC` can really
    // be set, and only on an emulator that offers it - but the capability is
    // what puts an edit box on the register pane, and a refusal naming the
    // register beats a pane that looks read-only for no stated reason.
    assert_eq!(capabilities.get("supportsSetVariable"), Some(&json!(true)));

    // Still not claimed: nothing here implements them, and claiming one would
    // put a button in the UI that answers with a protocol error.
    for absent in [
        "supportsRestartRequest",
        "supportsConditionalBreakpoints",
        "supportsHitConditionalBreakpoints",
        "supportsFunctionBreakpoints"
    ] {
        assert!(capabilities.get(absent).is_none(), "{absent}");
    }
}

fn _unused(_: &Path, _: PathBuf) {}

/// The disassembly view should show your source, not just opcodes - and be
/// able to jump to it.
#[test]
fn disassembled_instructions_carry_their_source_line() {
    let mut session = session();
    let from_emulator = json!({
        "type": "response", "command": "disassemble", "success": true,
        "body": {"instructions": [
            {"address": "0x4000", "instruction": "ld a,0"},
            {"address": "0x4003", "instruction": "nop"},
            {"address": "0xbb5a", "instruction": "ret"}
        ]}
    });
    let out = session.on_emulator_message(&from_emulator);
    let instructions = out[0]["body"]["instructions"].as_array().unwrap();

    assert_eq!(instructions[0]["line"], json!(10));
    assert_eq!(instructions[0]["location"]["name"], json!("main.asm"));
    assert_eq!(instructions[1]["line"], json!(11));
    // Firmware is not ours; leaving it bare is the honest answer.
    assert!(instructions[2].get("location").is_none());
}

/// `AF` is one hex word. The flags inside it are what gets read while stepping.
#[test]
fn the_register_pane_decodes_the_flags() {
    let mut session = session();
    let from_emulator = json!({
        "type": "response", "command": "variables", "success": true,
        "body": {"variables": [
            {"name": "AF", "value": "0x4A45", "variablesReference": 0},
            {"name": "BC", "value": "0x1234", "variablesReference": 0}
        ]}
    });
    let out = session.on_emulator_message(&from_emulator);
    let variables = out[0]["body"]["variables"].as_array().unwrap();
    let flags = variables
        .iter()
        .find(|v| v["name"] == json!("F (flags)"))
        .expect("the flags are shown");
    // 0x45 = 0100_0101: Z, P/V and C.
    assert_eq!(flags["value"], json!("-Z---P/V-C"));
    assert!(
        flags["variablesReference"].as_i64().unwrap() != 0,
        "expandable"
    );
}

/// Expanding the flags row lists every bit, answered locally - the emulator has
/// never heard of that reference.
#[test]
fn expanding_the_flags_lists_every_bit() {
    let mut session = session();
    // The session has to have seen the registers to know the flags.
    let _ = session.on_emulator_message(&json!({
        "type": "response", "command": "variables", "success": true,
        "body": {"variables": [{"name": "AF", "value": "0x0041"}]}
    }));

    let reference = {
        let out = session.on_emulator_message(&json!({
            "type": "response", "command": "variables", "success": true,
            "body": {"variables": [{"name": "AF", "value": "0x0041"}]}
        }));
        out[0]["body"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == json!("F (flags)"))
            .unwrap()["variablesReference"]
            .as_i64()
            .unwrap()
    };

    let answered = session
        .on_editor_message(&json!({
            "seq": 5, "type": "request", "command": "variables",
            "arguments": {"variablesReference": reference}
        }))
        .unwrap();
    assert_eq!(answered.len(), 1, "answered here, not forwarded");
    let bits = answered[0]["body"]["variables"].as_array().unwrap();
    assert_eq!(bits.len(), 8);
    // 0x41 = 0100_0001: Z and C.
    assert_eq!(
        bits.iter().find(|b| b["name"] == json!("Z")).unwrap()["value"],
        json!("1")
    );
    assert_eq!(
        bits.iter().find(|b| b["name"] == json!("C")).unwrap()["value"],
        json!("1")
    );
    assert_eq!(
        bits.iter().find(|b| b["name"] == json!("S")).unwrap()["value"],
        json!("0")
    );
}

/// The CPC is not only its CPU. The chips that decide what the Z80's work looks
/// like get their own scopes - and they are answered here, because the emulator
/// exposes no API for any of them.
#[test]
fn the_chip_scopes_are_offered_and_answer_for_themselves() {
    let mut session = session();
    let out = session.on_emulator_message(&json!({
        "type": "response", "command": "scopes", "success": true,
        "body": {"scopes": [{"name": "Registers", "variablesReference": 17}]}
    }));
    let scopes = out[0]["body"]["scopes"].as_array().unwrap();
    let names: Vec<&str> = scopes.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        vec!["Registers", "CRTC", "Gate Array", "PSG", "PPI", "Disc"]
    );

    // Asking for one makes the machine describe itself rather than reaching an
    // emulator that would refuse the reference.
    let reference = scopes[1]["variablesReference"].as_i64().unwrap();
    let answered = session
        .on_editor_message(&json!({
            "seq": 6, "type": "request", "command": "variables",
            "arguments": {"variablesReference": reference}
        }))
        .unwrap();
    assert!(answered.is_empty(), "held until the snapshot arrives");
    assert!(
        session
            .peer()
            .commands()
            .contains(&"cpclib/machineState".to_string())
    );
    assert!(
        !session.peer().commands().contains(&"variables".to_string()),
        "the emulator is never asked for a reference it does not know"
    );
}

/// A register reference the emulator owns is still forwarded to it.
#[test]
fn the_emulators_own_variable_references_are_forwarded() {
    let mut session = session();
    let answered = session
        .on_editor_message(&json!({
            "seq": 7, "type": "request", "command": "variables",
            "arguments": {"variablesReference": 17}
        }))
        .unwrap();
    assert!(answered.is_empty(), "not answered locally");
    assert!(session.peer().commands().contains(&"variables".to_string()));
}

/// Watching a label: the user types `animation_state`, and what they want is
/// the byte living there.
#[test]
fn a_watch_on_a_label_reads_the_memory_at_it() {
    use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
    let mut symbols = std::collections::HashMap::new();
    symbols.insert("animation_state".to_string(), 0x8000u32);
    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![SourceMapRow::flat(0, 1, 0x4000, 1)]
    })
    .with_symbols(symbols);

    let mut session = Session::new(RecordingPeer::new(), map);
    session.on_attached().unwrap();

    // The editor asks...
    let answered = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "animation_state", "context": "watch"}
        }))
        .unwrap();
    assert!(
        answered.is_empty(),
        "answered once the emulator reports the bytes"
    );

    // ...which becomes a memory read at the label's address.
    let read = session
        .peer()
        .last("readMemory")
        .expect("a read was issued")
        .clone();
    assert_eq!(read["arguments"]["memoryReference"], json!("0x8000"));
    assert_eq!(read["arguments"]["count"], json!(1));

    // The emulator answers *that* request - matching its seq is what tells the
    // adapter the answer is its own rather than the editor's.
    let read_seq = read["seq"].as_i64().unwrap();
    let out = session.on_emulator_message(&json!({
        "type": "response", "command": "readMemory", "request_seq": read_seq,
        "success": true, "body": {"address": "0x8000", "data": "Bw=="}
    }));
    assert_eq!(out.len(), 1, "the read is turned into the watch's answer");
    assert_eq!(out[0]["command"], json!("evaluate"));
    assert_eq!(out[0]["body"]["result"], json!("0x8000 -> 0x07 (7)"));
}

/// `label,w` watches a 16-bit value, little-endian like the Z80.
#[test]
fn a_watch_can_ask_for_a_word() {
    use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
    let mut symbols = std::collections::HashMap::new();
    symbols.insert("counter".to_string(), 0x9000u32);
    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![SourceMapRow::flat(0, 1, 0x4000, 1)]
    })
    .with_symbols(symbols);

    let mut session = Session::new(RecordingPeer::new(), map);
    session.on_attached().unwrap();
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "counter,w"}
        }))
        .unwrap();
    let read = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(read["arguments"]["count"], json!(2));

    // 0x34 0x12 little-endian == 0x1234. Base64 of [0x34,0x12] is "NBI=".
    let out = session.on_emulator_message(&json!({
        "type": "response", "command": "readMemory",
        "request_seq": read["seq"].as_i64().unwrap(),
        "success": true, "body": {"data": "NBI="}
    }));
    // Both halves: the address the label stands for, and what is there. For a
    // code label the address is the answer; for a variable, the contents.
    assert_eq!(out[0]["body"]["result"], json!("0x9000 -> 0x1234 (4660)"));
}

/// An expression that is not one of our labels is refused *here*, with a
/// reason.
///
/// It used to be forwarded to the emulator, which implements no `evaluate` at
/// all and answers "not supported" - telling the user nothing about the name
/// they actually typed.
#[test]
fn an_unknown_expression_is_refused_with_a_reason() {
    let mut session = session();
    let answered = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "no_such_label"}
        }))
        .unwrap();
    assert_eq!(answered.len(), 1);
    assert_eq!(answered[0]["success"], json!(false));
    assert!(
        answered[0]["message"]
            .as_str()
            .unwrap()
            .contains("not a label of the program"),
        "{:?}",
        answered[0]
    );
    assert!(
        !session.peer().commands().contains(&"evaluate".to_string()),
        "the emulator is never asked something it cannot answer"
    );
}

/// The answer to a request *we* sent must never reach the editor.
///
/// Both sides number their requests from 1, so an emulator reply to our
/// `attach` (request_seq 2) is indistinguishable from a reply to the editor's
/// request 2 - which is its `launch`. Forwarding one told VS Code its launch
/// had been answered by an attach, and the whole session fell apart: no panes,
/// no disassembly, nothing.
#[test]
fn answers_to_our_own_requests_are_never_forwarded() {
    let mut session = Session::new(RecordingPeer::new(), fixture());

    session.send_own_request("initialize", json!({})).unwrap();
    session.send_own_request("attach", json!({})).unwrap();
    let sent = session.peer().sent.clone();
    let seqs: Vec<i64> = sent.iter().map(|m| m["seq"].as_i64().unwrap()).collect();

    // Numbered far from the editor's, so a transcript stays readable.
    assert!(seqs.iter().all(|s| *s >= 1_000_000), "{seqs:?}");

    for (seq, command) in seqs.iter().zip(["initialize", "attach"]) {
        let answer = json!({
            "type": "response", "command": command, "request_seq": seq,
            "success": true, "body": {}
        });
        assert!(
            session.on_emulator_message(&answer).is_empty(),
            "{command} answer must be consumed, not forwarded"
        );
    }
}

/// An answer to something the *editor* asked is still forwarded.
#[test]
fn answers_to_the_editors_requests_still_reach_it() {
    let mut session = session();
    let answer = json!({
        "type": "response", "command": "continue", "request_seq": 4,
        "success": true, "body": {"allThreadsContinued": true}
    });
    assert_eq!(session.on_emulator_message(&answer), vec![answer.clone()]);
}

/// Attaching releases the breakpoints held while the emulator was starting -
/// and that has to work through the request we actually sent.
#[test]
fn the_emulators_attach_answer_releases_held_breakpoints() {
    let mut session = Session::new(RecordingPeer::new(), fixture());
    set_breakpoints(&mut session, "main.asm", &[10]);
    assert!(session.peer().last("setInstructionBreakpoints").is_none());

    session.send_own_request("attach", json!({})).unwrap();
    let attach_seq = session.peer().last("attach").unwrap()["seq"]
        .as_i64()
        .unwrap();
    let consumed = session.on_emulator_message(&json!({
        "type": "response", "command": "attach", "request_seq": attach_seq,
        "success": true, "body": {}
    }));
    assert!(consumed.is_empty(), "the attach answer is ours");

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}])
    );
}

/// The editor asks for a file's contents when it cannot open the path itself.
/// The emulator has never heard of a source file and rejects what it does not
/// implement - which is how "Could not load source 'lib.asm'" happened.
#[test]
fn a_source_request_is_answered_from_disk() {
    let tmp = camino_tempfile::tempdir().unwrap();
    let file = tmp.path().join("lib.asm");
    std::fs::write(&file, "\tnop\n").unwrap();

    let map = SourceMap::from_raw(&cpclib_asm::assembler::listing_output::RawSourceMap {
        files: vec![file.to_string()],
        rows: vec![cpclib_asm::assembler::listing_output::SourceMapRow::flat(
            0, 1, 0x4000, 1
        )]
    });
    let mut session = Session::new(RecordingPeer::new(), map);

    let answered = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "source",
            "arguments": {"source": {"name": "lib.asm", "path": file.as_str()}}
        }))
        .unwrap();
    assert_eq!(answered.len(), 1, "answered here, never forwarded");
    assert_eq!(answered[0]["success"], json!(true));
    assert_eq!(answered[0]["body"]["content"], json!("\tnop\n"));
    assert!(
        !session.peer().commands().contains(&"source".to_string()),
        "the emulator is never asked"
    );
}

/// A source named without a usable path is still found, if it belongs to the
/// program being debugged.
#[test]
fn a_source_known_only_by_name_is_found_in_the_map() {
    let tmp = camino_tempfile::tempdir().unwrap();
    let file = tmp.path().join("lib.asm");
    std::fs::write(&file, "; contents\n").unwrap();

    let map = SourceMap::from_raw(&cpclib_asm::assembler::listing_output::RawSourceMap {
        files: vec![file.to_string()],
        rows: vec![cpclib_asm::assembler::listing_output::SourceMapRow::flat(
            0, 1, 0x4000, 1
        )]
    });
    let mut session = Session::new(RecordingPeer::new(), map);

    let answered = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "source",
            "arguments": {"source": {"name": "lib.asm"}}
        }))
        .unwrap();
    assert_eq!(answered[0]["body"]["content"], json!("; contents\n"));
}

/// A file that is not part of the program says so, rather than producing a
/// protocol error about an unsupported request.
#[test]
fn an_unknown_source_is_refused_with_a_reason() {
    let mut session = session();
    let answered = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "source",
            "arguments": {"source": {"name": "nowhere.asm"}}
        }))
        .unwrap();
    assert_eq!(answered[0]["success"], json!(false));
    assert!(
        answered[0]["message"]
            .as_str()
            .unwrap()
            .contains("not part of the program"),
        "{:?}",
        answered[0]
    );
}

/// A request the emulator does not implement is refused here, not forwarded.
///
/// 1984js answers an unknown request with a protocol error, and VS Code shows
/// that error *in place of* whatever the user asked for. That is how `source`
/// once produced "DAP request 'source' is not supported" where a file's
/// contents should have been.
#[test]
fn a_request_the_emulator_cannot_serve_is_refused_here() {
    let mut session = session();
    let answered = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setFunctionBreakpoints",
            "arguments": {"breakpoints": []}
        }))
        .unwrap();

    assert_eq!(answered.len(), 1);
    assert_eq!(answered[0]["success"], json!(false));
    assert!(
        !session
            .peer()
            .commands()
            .contains(&"setDataBreakpoints".to_string()),
        "and the emulator was never asked"
    );
}

/// Editing a file mid-session does not move the code that is running, so a
/// breakpoint set afterwards lands at the address the line *used to* have.
/// Saying so is the difference between a known limitation and the debugger
/// looking broken.
#[test]
fn a_breakpoint_in_an_edited_file_says_it_is_stale() {
    let directory = camino_tempfile::tempdir().unwrap();
    let file = directory.path().join("main.asm");
    std::fs::write(&file, "  nop\n  nop\n").unwrap();

    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec![file.to_string()],
        rows: vec![SourceMapRow::flat(0, 1, 0x4000, 1)]
    });
    let mut session = Session::new(RecordingPeer::new(), map);
    session.record_source_state();
    session.on_attached().unwrap();

    // Unchanged: no complaint.
    let answered = set_breakpoints(&mut session, file.as_str(), &[1]);
    assert_eq!(answered["body"]["breakpoints"][0]["verified"], json!(true));
    assert!(answered["body"]["breakpoints"][0].get("message").is_none());

    // Edited: the same request now carries a warning, and the breakpoint is
    // still placed - at the address that line had when the program was built.
    std::fs::write(&file, "  ld a,0\n  nop\n  nop\n").unwrap();
    let answered = set_breakpoints(&mut session, file.as_str(), &[1]);
    let breakpoint = &answered["body"]["breakpoints"][0];
    assert_eq!(breakpoint["verified"], json!(true));
    let message = breakpoint["message"].as_str().unwrap();
    assert!(message.contains("has changed"), "{message}");
    assert!(message.contains("main.asm"), "{message}");
}

/// A file we never fingerprinted is not reported as stale: not knowing is not
/// the same as knowing, and a warning on every breakpoint is worth nothing.
#[test]
fn a_file_that_was_never_fingerprinted_is_not_called_stale() {
    let mut session = session();
    let answered = set_breakpoints(&mut session, "main.asm", &[10]);
    assert!(answered["body"]["breakpoints"][0].get("message").is_none());
}

/// The editor's own disassembly view is decoded here, not by the emulator.
///
/// Its mnemonics are *its* mnemonics: swap the emulator and the view changes
/// under you for a program that has not. Reading the bytes and decoding them
/// with the assembler's own tables makes the built-in view read exactly like
/// `-dv` and like the source beside it.
#[test]
fn a_forward_disassembly_is_decoded_here() {
    let mut session = session();
    let held = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "disassemble",
            "arguments": {
                "memoryReference": "0x4000",
                "instructionCount": 2,
                "instructionOffset": 0
            }
        }))
        .unwrap();
    assert!(held.is_empty(), "answered once the bytes arrive");

    // Bytes, not a disassembly.
    let asked = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(asked["arguments"]["memoryReference"], json!("0x4000"));
    assert!(
        !session
            .peer()
            .commands()
            .contains(&"disassemble".to_string()),
        "the emulator is never asked to decode"
    );

    // ld a,0 ; nop
    let out = session.on_emulator_message(&json!({
        "seq": 3, "type": "response", "request_seq": asked["seq"], "success": true,
        "command": "readMemory",
        "body": {"address": "0x4000", "data": "PgAA"}
    }));
    let instructions = out[0]["body"]["instructions"].as_array().unwrap();
    assert_eq!(instructions.len(), 2, "{instructions:?}");
    assert!(
        instructions[0]["instruction"]
            .as_str()
            .unwrap()
            .to_uppercase()
            .contains("LD A"),
        "{instructions:?}"
    );
}

/// Looking *backwards* is decoded here too.
///
/// The editor's view asks for context before the program counter, and that
/// half was still the emulator's - so the built-in tab read half in our
/// mnemonics and half in its own. Z80 cannot be read backwards, so the bytes
/// before the anchor are read and every alignment is tried; the one whose
/// instruction boundaries land on the anchor wins.
#[test]
fn a_backwards_disassembly_is_decoded_here_when_an_alignment_fits() {
    let mut session = session();
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "disassemble",
            "arguments": {
                "memoryReference": "0x4003",
                "instructionCount": 4,
                "instructionOffset": -2
            }
        }))
        .unwrap();

    // The read starts *before* the anchor, to have context to align against.
    let asked = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(asked["arguments"]["memoryReference"], json!("0x3ffb"));
    assert!(
        !session
            .peer()
            .commands()
            .contains(&"disassemble".to_string()),
        "the emulator is not asked to decode"
    );

    // Five filler bytes, then ld a,1 ; nop ; ld hl,0x3456 ; ret - laid out so
    // 0x4003 is a real instruction boundary.
    let out = session.on_emulator_message(&json!({
        "seq": 3, "type": "response", "request_seq": asked["seq"], "success": true,
        "command": "readMemory",
        "body": {"address": "0x3FFB", "data": "AAAAAAA+AQAhVjTJ"}
    }));
    let instructions = out[0]["body"]["instructions"].as_array().unwrap();
    assert!(
        instructions.iter().any(|i| i["address"] == json!("0x4003")),
        "the address asked about is in the answer: {instructions:?}"
    );
}

/// An address more than one page claims still gets a source line - the most
/// likely one - rather than none at all.
///
/// `-dv` showed that address's source happily, because it asks `location_at`,
/// which answers with the span covering the address. Only the stack frame was
/// stricter, and refusing there is what left the editor with nothing to open
/// and a bare disassembly view instead. The knowledge was the same in both
/// places; the caution was costing more than it saved.
#[test]
fn a_contested_address_still_names_its_most_likely_line() {
    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec!["lib.asm".into()],
        rows: vec![
            SourceMapRow {
                file: 0,
                line: 40,
                logical: 0x04A5,
                physical: 0x04A5,
                page: 0,
                column: 2,
                column_end: 5,
                len: 1
            },
            SourceMapRow {
                file: 0,
                line: 900,
                logical: 0x04A5,
                physical: 0x1_04A5,
                page: 1,
                column: 2,
                column_end: 5,
                len: 1
            },
        ]
    });
    let mut session = Session::new(RecordingPeer::new(), map);

    let out = session.on_emulator_message(&json!({
        "seq": 5, "type": "response", "request_seq": 3, "success": true,
        "command": "stackTrace",
        "body": {
            "stackFrames": [{
                "id": 17, "name": "Z80 @ 0x04A5", "line": 0, "column": 0,
                "instructionPointerReference": "0x04A5"
            }],
            "totalFrames": 1
        }
    }));

    let frames = out
        .iter()
        .find_map(|m| m["body"]["stackFrames"].as_array())
        .expect("a stack trace went out");
    assert_eq!(
        frames[0]["source"]["name"],
        json!("lib.asm"),
        "a line, not an empty disassembly view: {frames:?}"
    );

    // ...and it says so once, rather than presenting a guess as certainty.
    let note = out
        .iter()
        .find(|m| m["event"] == json!("output"))
        .expect("explained");
    let text = note["body"]["output"].as_str().unwrap();
    assert!(text.contains("most likely line"), "{text}");
}

/// `PC` can be moved where the emulator offers it, and only `PC`.
#[test]
fn only_the_program_counter_can_be_set() {
    let mut session = Session::new(
        RecordingPeer::new().also_supporting(&["cpclib/setPc"]),
        fixture()
    );
    session.on_attached().unwrap();

    let set = |name: &str, value: &str| {
        json!({
            "seq": 9, "type": "request", "command": "setVariable",
            "arguments": {"variablesReference": 1, "name": name, "value": value}
        })
    };

    let out = session.on_editor_message(&set("PC", "0x4000")).unwrap();
    assert_eq!(out[0]["success"], json!(true), "{out:?}");
    assert_eq!(out[0]["body"]["value"], json!("0x4000"));
    assert!(
        session.peer().commands().contains(&"cpclib/setPc".to_string()),
        "the emulator was told: {:?}",
        session.peer().commands()
    );

    // Every other register is refused by name rather than silently ignored.
    let out = session.on_editor_message(&set("HL", "0x1234")).unwrap();
    assert_eq!(out[0]["success"], json!(false));
    let why = out[0]["message"].as_str().unwrap_or_default();
    assert!(why.contains("HL"), "says which one: {why}");
}

/// An emulator with no register write at all says so.
#[test]
fn an_emulator_that_cannot_move_pc_says_so() {
    let mut session = Session::new(RecordingPeer::new(), fixture());
    session.on_attached().unwrap();

    let out = session
        .on_editor_message(&json!({
            "seq": 9, "type": "request", "command": "setVariable",
            "arguments": {"variablesReference": 1, "name": "PC", "value": "0x4000"}
        }))
        .unwrap();
    assert_eq!(out[0]["success"], json!(false));
    assert!(
        out[0]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("no way to move PC"),
        "{:?}",
        out[0]["message"]
    );
}

/// The raster-timing idiom, as a program on disk with a source map to match.
///
/// ```text
///     ld b, 20                       ; line 2, 0x4000
/// .wait_lines                        ; line 3, no bytes
///     defs 64 - duration(djnz $)-1   ; line 4, 60 bytes at 0x4002
///     djnz .wait_lines               ; line 5, 0x403E
///     nop                            ; line 6, 0x4040
/// ```
///
/// In its own directory, and the directory is handed back: these tests run in
/// parallel in one process, and a fixture written to a shared path is a
/// fixture another test can be reading while this one truncates it.
fn defs_program() -> (camino_tempfile::Utf8TempDir, SourceMap) {
    let directory = camino_tempfile::tempdir().unwrap();
    let source = directory.path().join("demo_code.asm");
    std::fs::write(
        &source,
        "\torg 0x4000\n\tld b, 20\n.wait_lines\n\tdefs 64 - duration(djnz $)-1\n\tdjnz \
         .wait_lines\n\tnop\n"
    )
    .unwrap();

    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec![source.as_str().into()],
        rows: vec![
            SourceMapRow::flat(0, 2, 0x4000, 2),
            SourceMapRow::flat(0, 4, 0x4002, 60),
            SourceMapRow::flat(0, 5, 0x403E, 2),
            SourceMapRow::flat(0, 6, 0x4040, 1),
        ]
    });
    (directory, map)
}

/// Where the program stopped, learned the way it really is - from the register
/// pane the editor asks for on every stop.
fn stopped_at(session: &mut Session<RecordingPeer>, pc: u16) {
    session.on_emulator_message(&json!({
        "type": "response", "command": "variables", "success": true,
        "body": {"variables": [
            {"name": "PC", "value": format!("0x{pc:04X}"), "variablesReference": 0}
        ]}
    }));
}

/// What the session told the emulator about the source before the last step
/// over.
fn the_line_offered(session: &mut Session<RecordingPeer>) -> LineAtPc {
    session
        .on_editor_message(&json!({"seq": 2, "type": "request", "command": "next"}))
        .unwrap();
    session
        .peer()
        .lines_at_pc
        .last()
        .cloned()
        .expect("a step over asks about the source")
}

/// Stepping over a `defs` hands the emulator the whole run.
///
/// `defs 60` runs as sixty `NOP`s, so stepping it one instruction at a time is
/// sixty presses to reach the `djnz` under it. Only the source says this is a
/// `defs` rather than sixty hand-written `nop`s, which is why the answer is
/// worked out here and not from the bytes at `PC`.
#[test]
fn stepping_over_a_defs_offers_the_whole_run() {
    let (_directory, map) = defs_program();
    let mut session = Session::new(RecordingPeer::new(), map);
    stopped_at(&mut session, 0x4002);

    assert_eq!(the_line_offered(&mut session), LineAtPc::Defs(0x4002..0x403E));
}

/// From the middle of the run, the answer is the same run.
///
/// The deliberate choice: a `defs` is a repetition, and stepping over a
/// repetition from inside it finishes it. Running to the next instruction
/// instead would be one `NOP` - which is exactly the hand-stepping this was
/// asked for to replace.
#[test]
fn stepping_over_from_inside_a_defs_run_offers_the_rest_of_it() {
    let (_directory, map) = defs_program();
    let mut session = Session::new(RecordingPeer::new(), map);
    stopped_at(&mut session, 0x4020);

    assert_eq!(the_line_offered(&mut session), LineAtPc::Defs(0x4002..0x403E));
}

/// The `djnz` written under the `defs` is a `djnz`, not part of the run.
///
/// It is the address the step over lands on, and stepping over *it* is the
/// emulator's own business - a loop it runs out by the rules it already had.
#[test]
fn the_line_after_a_defs_run_is_not_part_of_it() {
    let (_directory, map) = defs_program();
    let mut session = Session::new(RecordingPeer::new(), map);
    stopped_at(&mut session, 0x403E);

    assert_eq!(the_line_offered(&mut session), LineAtPc::Ordinary);
}

/// A `nop` somebody wrote is an ordinary line, said in so many words.
///
/// It assembles to the same byte a `defs` does, so "not a `defs`" has to be a
/// different answer from "no idea": the emulator falls back on the bytes only
/// for the second, and would otherwise swallow a `nop` the user meant to step.
#[test]
fn a_hand_written_nop_line_is_called_ordinary() {
    let (_directory, map) = defs_program();
    let mut session = Session::new(RecordingPeer::new(), map);
    stopped_at(&mut session, 0x4040);

    assert_eq!(the_line_offered(&mut session), LineAtPc::Ordinary);
}

/// An address no source line claims leaves it to the bytes.
///
/// A snapshot debugged without its listing, the firmware, code the program
/// wrote itself: there is nothing to consult, and the emulator is told so
/// rather than being told the line is ordinary.
#[test]
fn an_address_with_no_source_line_is_unknown() {
    let (_directory, map) = defs_program();
    let mut session = Session::new(RecordingPeer::new(), map);
    stopped_at(&mut session, 0x9000);

    assert_eq!(the_line_offered(&mut session), LineAtPc::Unknown);
}

/// A listing whose source has moved is no source at all.
#[test]
fn a_line_whose_file_cannot_be_read_is_unknown() {
    let (directory, map) = defs_program();
    let mut session = Session::new(RecordingPeer::new(), map);
    stopped_at(&mut session, 0x4002);
    drop(directory); // the file the map names is gone

    assert_eq!(the_line_offered(&mut session), LineAtPc::Unknown);
}

/// Only a step *over* asks. Stepping in is an instruction by definition, and
/// stepping out is aimed at a return address no line describes.
#[test]
fn stepping_in_and_out_ask_nothing_about_the_source() {
    let (_directory, map) = defs_program();
    let mut session = Session::new(RecordingPeer::new(), map);
    stopped_at(&mut session, 0x4002);

    for button in ["stepIn", "stepOut"] {
        session
            .on_editor_message(&json!({"seq": 2, "type": "request", "command": button}))
            .unwrap();
    }
    assert!(
        session.peer().lines_at_pc.is_empty(),
        "{:?}",
        session.peer().lines_at_pc
    );
}

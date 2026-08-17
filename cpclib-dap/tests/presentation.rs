//! What the register pane says.
//!
//! Registers, addresses and costs are all numbers to the emulator. Turning
//! them back into the things the program is made of - labels, NOPs - is what
//! makes them readable while stepping.

use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::inspect;
use cpclib_dap::peer::RecordingPeer;
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::{Value, json};

/// Base64, so a test can hand the session bytes the way a real emulator does.
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

fn map_with(symbols: &[(&str, u32)]) -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![SourceMapRow::flat(0, 3, 0x4000, 2)]
    })
    .with_symbols(
        symbols
            .iter()
            .map(|(name, address)| (name.to_string(), *address))
            .collect()
    )
}

fn registers(pairs: &[(&str, &str)]) -> Vec<Value> {
    pairs
        .iter()
        .map(|(name, value)| json!({"name": name, "value": value, "variablesReference": 0}))
        .collect()
}

fn value_of<'a>(variables: &'a [Value], name: &str) -> &'a str {
    variables
        .iter()
        .find(|v| v["name"] == json!(name))
        .and_then(|v| v["value"].as_str())
        .unwrap_or_else(|| panic!("no {name} in the pane"))
}

/// A register pointing at a label says so.
#[test]
fn a_register_holding_a_known_address_shows_its_label() {
    let map = map_with(&[("screen_buffer", 0xC000)]);
    let mut variables = registers(&[("AF", "0x0044"), ("HL", "0xC000")]);
    inspect::annotate_registers_with(&mut variables, 1, Some(&map));

    assert_eq!(value_of(&variables, "HL"), "0xC000 (screen_buffer)");
}

/// A register pointing *into* a labelled area says how far in.
#[test]
fn a_register_inside_a_labelled_area_shows_the_offset() {
    let map = map_with(&[("screen_buffer", 0xC000)]);
    let mut variables = registers(&[("AF", "0x0044"), ("DE", "0xC003")]);
    inspect::annotate_registers_with(&mut variables, 1, Some(&map));

    assert_eq!(value_of(&variables, "DE"), "0xC003 (screen_buffer+3)");
}

/// A register nowhere near anything is left as the number it is: a label two
/// kilobytes back says nothing about where it points.
#[test]
fn a_register_far_from_every_label_is_left_alone() {
    let map = map_with(&[("screen_buffer", 0xC000)]);
    let mut variables = registers(&[("AF", "0x0044"), ("BC", "0x0010")]);
    inspect::annotate_registers_with(&mut variables, 1, Some(&map));

    assert_eq!(value_of(&variables, "BC"), "0x0010");
}

/// `AF` is a value and a flag byte, not a pointer; labelling it would be noise
/// on every single step.
#[test]
fn af_is_never_labelled() {
    let map = map_with(&[("screen_buffer", 0xC000)]);
    let mut variables = registers(&[("AF", "0xC000")]);
    inspect::annotate_registers_with(&mut variables, 1, Some(&map));

    assert_eq!(value_of(&variables, "AF"), "0xC000");
}

/// Costs come from the assembler's own table, so the pane and the build cannot
/// disagree about what an instruction costs.
#[test]
fn a_source_line_is_priced_in_nops() {
    assert_eq!(inspect::nops_of_source_line("  nop"), Some(1));
    assert_eq!(inspect::nops_of_source_line("  ld a,0"), Some(2));
    // `ldir` is the assembler's own answer: 5, the cost of a single pass. A
    // repeating instruction has no one cost - it is 6 per iteration except the
    // last - and this deliberately reports what `basm` reports rather than a
    // second opinion the build would disagree with.
    assert_eq!(inspect::nops_of_source_line("  ldir"), Some(5));
    // Several instructions on one line is what the line costs, which is the
    // question actually being asked.
    assert_eq!(inspect::nops_of_source_line("  nop : nop : nop"), Some(3));
}

/// A line that is not an instruction is priced at nothing rather than wrongly.
#[test]
fn a_line_with_no_instruction_costs_nothing() {
    assert_eq!(inspect::nops_of_source_line("; just a comment"), Some(0));
    assert_eq!(inspect::nops_of_source_line("label:"), Some(0));
}

/// The cost of the line at `PC` reaches the pane.
#[test]
fn the_register_pane_reports_the_cost_of_the_current_line() {
    let directory = camino_tempfile::tempdir().unwrap();
    let file = directory.path().join("main.asm");
    std::fs::write(&file, "  org 0x4000\n\n  ld a,0\n  nop\n").unwrap();

    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec![file.to_string()],
        // Line 3 is `ld a,0`, two bytes at 0x4000 - and two NOPs.
        rows: vec![SourceMapRow::flat(0, 3, 0x4000, 2)]
    });
    let mut session = Session::new(RecordingPeer::new(), map);

    let out = session.on_emulator_message(&json!({
        "seq": 4, "type": "response", "request_seq": 2, "success": true,
        "command": "variables",
        "body": {"variables": [
            {"name": "AF", "value": "0x0044", "variablesReference": 0},
            {"name": "PC", "value": "0x4000", "variablesReference": 0}
        ]}
    }));

    let variables = out[0]["body"]["variables"].as_array().unwrap();
    assert_eq!(value_of(variables, "cost"), "2 NOPs");
}

/// `-mv` reads memory and hands the dump to a panel, with a receipt in the
/// console so the line that was typed gets an answer.
#[test]
fn the_memory_view_command_reads_and_reports() {
    let map = map_with(&[("animation_state", 0xC000)]);
    let mut session = Session::new(RecordingPeer::new(), map);

    let held = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-mv 0xC000 4", "context": "repl"}
        }))
        .unwrap();
    assert!(held.is_empty(), "the answer waits for the bytes");

    let read = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(read["arguments"]["memoryReference"], json!("0xc000"));
    assert_eq!(read["arguments"]["count"], json!(4));

    let out = session.on_emulator_message(&json!({
        "seq": 3, "type": "response", "request_seq": read["seq"], "success": true,
        "command": "readMemory",
        "body": {"address": "0xC000", "data": "AQIDBA=="}
    }));

    assert_eq!(out[0]["event"], json!("cpclib/memoryView"));
    assert_eq!(out[0]["body"]["address"], json!(0xC000));
    assert_eq!(out[0]["body"]["bytes"], json!([1, 2, 3, 4]));
    assert_eq!(
        out[0]["body"]["marks"],
        json!([{"offset": 0, "name": "animation_state"}]),
        "the labels inside the range are marked"
    );
    assert_eq!(
        out[1]["success"],
        json!(true),
        "and the console line answered"
    );
}

/// A label is as good as an address, and rather more likely to be what someone
/// wants to look at.
#[test]
fn the_memory_view_accepts_a_label() {
    let map = map_with(&[("animation_state", 0xC000)]);
    let mut session = Session::new(RecordingPeer::new(), map);

    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-mv animation_state", "context": "repl"}
        }))
        .unwrap();

    let read = session.peer().last("readMemory").unwrap();
    assert_eq!(read["arguments"]["memoryReference"], json!("0xc000"));
    assert_eq!(
        read["arguments"]["count"],
        json!(0x40),
        "a sensible default"
    );
}

/// An unknown command says so and lists what there is, rather than being
/// forwarded to an emulator that would answer "evaluate is not supported".
#[test]
fn an_unknown_console_command_is_refused_here() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-wat", "context": "repl"}
        }))
        .unwrap();

    assert_eq!(out[0]["success"], json!(false));
    let message = out[0]["body"]["error"]["format"]
        .as_str()
        .or_else(|| out[0]["message"].as_str())
        .unwrap();
    assert!(message.contains("-help"), "{message}");
    assert!(!session.peer().commands().contains(&"evaluate".to_string()));
}

/// `-help` is answered from here too - the emulator has no idea what it is.
#[test]
fn help_lists_the_commands() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-help", "context": "repl"}
        }))
        .unwrap();

    let text = out[0]["body"]["result"].as_str().unwrap();
    assert!(text.contains("-mv"), "{text}");
}

/// Helper: drive a `-mv` to completion and hand back the emitted event.
fn open_memory_view(session: &mut Session<RecordingPeer>, expression: &str, data: &str) -> Value {
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": expression, "context": "repl"}
        }))
        .unwrap();
    let read = session.peer().last("readMemory").unwrap().clone();
    let out = session.on_emulator_message(&json!({
        "seq": 3, "type": "response", "request_seq": read["seq"], "success": true,
        "command": "readMemory", "body": {"address": "0xC000", "data": data}
    }));
    out[0].clone()
}

/// A memory view is something you keep open while stepping, so it re-reads
/// itself on every stop rather than showing what memory looked like three
/// steps ago.
#[test]
fn the_memory_view_refreshes_itself_on_every_stop() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    open_memory_view(&mut session, "-mv 0xC000 4", "AQIDBA==");

    let out = session.on_emulator_message(&json!({
        "seq": 5, "type": "event", "event": "stopped",
        "body": {"reason": "breakpoint"}
    }));
    assert_eq!(out[0]["event"], json!("stopped"), "the stop goes out first");

    let read = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(read["arguments"]["memoryReference"], json!("0xc000"));
    assert_eq!(read["arguments"]["count"], json!(4));

    // Byte 1 moved from 2 to 0x99.
    let out = session.on_emulator_message(&json!({
        "seq": 6, "type": "response", "request_seq": read["seq"], "success": true,
        "command": "readMemory", "body": {"address": "0xC000", "data": "AZkDBA=="}
    }));

    assert_eq!(out.len(), 1, "a refresh nobody typed prints no receipt");
    assert_eq!(out[0]["event"], json!("cpclib/memoryView"));
    assert_eq!(out[0]["body"]["bytes"], json!([1, 0x99, 3, 4]));
    assert_eq!(
        out[0]["body"]["changed"],
        json!([1]),
        "and says which byte moved"
    );
}

/// Nothing is marked as changed the first time: everything would be.
#[test]
fn the_first_look_marks_nothing_as_changed() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    let event = open_memory_view(&mut session, "-mv 0xC000 4", "AQIDBA==");
    assert_eq!(event["body"]["changed"], json!([]));
}

/// With no view open, a stop asks for no memory at all.
#[test]
fn a_stop_with_no_memory_view_reads_nothing() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    session.on_emulator_message(&json!({
        "seq": 5, "type": "event", "event": "stopped", "body": {"reason": "step"}
    }));
    assert!(
        !session
            .peer()
            .commands()
            .contains(&"readMemory".to_string())
    );
}

/// `-dv` disassembles memory and hands the instructions to a panel, each
/// carrying the source line it came from.
#[test]
fn the_disassembly_command_annotates_what_it_reads() {
    let map = map_with(&[("draw_sprite", 0x4000)]);
    let mut session = Session::new(RecordingPeer::new(), map);

    let held = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-dv draw_sprite 2", "context": "repl"}
        }))
        .unwrap();
    assert!(held.is_empty(), "the answer waits for the instructions");

    // Bytes, not a disassembly: the decoding is ours so the view reads the
    // same whatever emulator is underneath.
    let asked = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(asked["arguments"]["memoryReference"], json!("0x4000"));

    let out = session.on_emulator_message(&json!({
        "seq": 3, "type": "response", "request_seq": asked["seq"], "success": true,
        "command": "readMemory",
        // ld a,0 ; nop
        "body": {"address": "0x4000", "data": base64(&[0x3E, 0x00, 0x00])}
    }));

    assert_eq!(out[0]["event"], json!("cpclib/disassemblyView"));
    let instructions = out[0]["body"]["instructions"].as_array().unwrap();
    assert!(
        instructions[0]["instruction"]
            .as_str()
            .unwrap()
            .to_uppercase()
            .contains("LD A"),
        "decoded here, in basm's own spelling: {instructions:?}"
    );
    assert_eq!(instructions[0]["instructionBytes"], json!("3E 00"));
    // The row at 0x4000 is inside the span of line 3, and carries a label.
    assert_eq!(instructions[0]["line"], json!(3));
    assert_eq!(instructions[0]["location"]["name"], json!("main.asm"));
    assert_eq!(
        instructions[0]["symbol"],
        json!("draw_sprite"),
        "labels are what separate one macro expansion from the next"
    );
    // 0x4002 is past the two-byte span, so it belongs to no line - and says so
    // by carrying none, rather than borrowing the nearest.
    assert!(instructions[1].get("line").is_none());

    assert_eq!(
        out[1]["success"],
        json!(true),
        "and the console line answered"
    );
}

/// `-dv` on nothing recognisable is refused with a suggestion, not forwarded.
#[test]
fn the_disassembly_command_refuses_an_unknown_place() {
    let map = map_with(&[("draw_sprite", 0x4000)]);
    let mut session = Session::new(RecordingPeer::new(), map);
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-dv draw_sprit", "context": "repl"}
        }))
        .unwrap();

    assert_eq!(out[0]["success"], json!(false));
    let message = out[0]["message"].as_str().unwrap();
    assert!(
        message.contains("draw_sprite"),
        "suggests the real one: {message}"
    );
    assert!(
        !session
            .peer()
            .commands()
            .contains(&"readMemory".to_string())
    );
}

/// Tell the session where the program is, the way a real stop does: the editor
/// asks for the register pane and `PC` is in it.
fn report_pc(session: &mut Session<RecordingPeer>, pc: u16) -> Vec<Value> {
    session.on_emulator_message(&json!({
        "seq": 4, "type": "response", "request_seq": 2, "success": true,
        "command": "variables",
        "body": {"variables": [
            {"name": "AF", "value": "0x0044", "variablesReference": 0},
            {"name": "PC", "value": format!("0x{pc:04X}"), "variablesReference": 0}
        ]}
    }))
}

fn answer_disassembly(session: &mut Session<RecordingPeer>) -> Vec<Value> {
    let asked = session.peer().last("readMemory").unwrap().clone();
    session.on_emulator_message(&json!({
        "seq": 9, "type": "response", "request_seq": asked["seq"], "success": true,
        "command": "readMemory",
        "body": {"address": asked["arguments"]["memoryReference"], "data": base64(&[0x00])}
    }))
}

/// `-dv` with no argument disassembles from where the program is.
#[test]
fn the_disassembly_command_defaults_to_the_program_counter() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    report_pc(&mut session, 0x4000);

    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-dv", "context": "repl"}
        }))
        .unwrap();

    let asked = session.peer().last("readMemory").unwrap();
    assert_eq!(asked["arguments"]["memoryReference"], json!("0x4000"));
}

/// Before the program has ever stopped there is no `PC` to default to, and
/// saying so beats disassembling address zero.
#[test]
fn the_disassembly_command_says_when_there_is_no_pc_yet() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-dv", "context": "repl"}
        }))
        .unwrap();

    assert_eq!(out[0]["success"], json!(false));
    let message = out[0]["message"].as_str().unwrap();
    assert!(message.contains("has not stopped yet"), "{message}");
    assert!(
        !session
            .peer()
            .commands()
            .contains(&"readMemory".to_string())
    );
}

/// A view opened with no argument follows `PC`, so it can be read beside the
/// source at every step.
#[test]
fn a_pc_anchored_view_follows_the_program() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    report_pc(&mut session, 0x4000);
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-dv", "context": "repl"}
        }))
        .unwrap();
    answer_disassembly(&mut session);

    // Step: the stop arrives first, then the new PC.
    session.on_emulator_message(&json!({
        "seq": 5, "type": "event", "event": "stopped", "body": {"reason": "step"}
    }));
    report_pc(&mut session, 0x4004);

    let asked = session.peer().last("readMemory").unwrap();
    assert_eq!(
        asked["arguments"]["memoryReference"],
        json!("0x4004"),
        "the view moved with the program"
    );

    let out = answer_disassembly(&mut session);
    assert_eq!(out.len(), 1, "a refresh nobody typed prints no receipt");
    assert_eq!(out[0]["body"]["pc"], json!(0x4004), "and marks where PC is");
    assert_eq!(out[0]["body"]["followsPc"], json!(true));
}

/// A view opened at an explicit place stays there: a view of `draw_sprite` that
/// wanders off is not a view of `draw_sprite`.
#[test]
fn a_fixed_view_does_not_wander() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[("draw_sprite", 0x4000)]));
    report_pc(&mut session, 0x9000);
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-dv draw_sprite", "context": "repl"}
        }))
        .unwrap();
    answer_disassembly(&mut session);

    session.on_emulator_message(&json!({
        "seq": 5, "type": "event", "event": "stopped", "body": {"reason": "step"}
    }));
    report_pc(&mut session, 0x9004);

    let asked = session.peer().last("readMemory").unwrap();
    assert_eq!(
        asked["arguments"]["memoryReference"],
        json!("0x4000"),
        "still looking at draw_sprite"
    );
    // ...but it *is* re-read, because self-modifying code is the reason to
    // watch memory rather than source.
    assert!(
        session
            .peer()
            .commands()
            .iter()
            .filter(|c| *c == "readMemory")
            .count()
            >= 2
    );
}

/// Register names are answered from the values the pane just fetched.
#[test]
fn a_register_name_in_the_console_gives_its_value() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[("screen", 0xC000)]));
    session.on_emulator_message(&json!({
        "seq": 4, "type": "response", "request_seq": 2, "success": true,
        "command": "variables",
        "body": {"variables": [
            {"name": "AF", "value": "0x3F41", "variablesReference": 0},
            {"name": "BC", "value": "0x1234", "variablesReference": 0},
            {"name": "HL", "value": "0xC000", "variablesReference": 0},
            {"name": "PC", "value": "0x4000", "variablesReference": 0}
        ]}
    }));

    let ask = |session: &mut Session<RecordingPeer>, expression: &str| -> String {
        let out = session
            .on_editor_message(&json!({
                "seq": 1, "type": "request", "command": "evaluate",
                "arguments": {"expression": expression, "context": "repl"}
            }))
            .unwrap();
        out[0]["body"]["result"].as_str().unwrap().to_string()
    };

    assert_eq!(ask(&mut session, "hl"), "0xC000 (49152) screen");
    assert_eq!(ask(&mut session, "BC"), "0x1234 (4660)");
    // The halves come out too, without the pane having to list them.
    assert_eq!(ask(&mut session, "a"), "0x3F (63)");
    assert_eq!(ask(&mut session, "b"), "0x12 (18)");
    assert_eq!(ask(&mut session, "c"), "0x34 (52)");
    // ...and `f` is worth reading as flags.
    assert!(
        ask(&mut session, "f").contains("-Z-----C"),
        "{}",
        ask(&mut session, "f")
    );

    // No round trip: these were already on hand.
    assert!(!session.peer().commands().contains(&"evaluate".to_string()));
}

/// A register wins over a label of the same name: typing `hl` never means a
/// label called `hl`.
#[test]
fn a_register_name_is_not_shadowed_by_a_label() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[("hl", 0x9000)]));
    session.on_emulator_message(&json!({
        "seq": 4, "type": "response", "request_seq": 2, "success": true,
        "command": "variables",
        "body": {"variables": [{"name": "HL", "value": "0x1111", "variablesReference": 0}]}
    }));

    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "hl", "context": "repl"}
        }))
        .unwrap();
    assert_eq!(out[0]["body"]["result"], json!("0x1111 (4369)"));
    assert!(
        !session
            .peer()
            .commands()
            .contains(&"readMemory".to_string())
    );
}

/// `(hl)` is the byte at that address - what you want when a pointer register
/// is involved.
#[test]
fn an_indirect_register_reads_the_byte_there() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    session.on_emulator_message(&json!({
        "seq": 4, "type": "response", "request_seq": 2, "success": true,
        "command": "variables",
        "body": {"variables": [{"name": "HL", "value": "0xC000", "variablesReference": 0}]}
    }));

    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "(hl)", "context": "repl"}
        }))
        .unwrap();
    let read = session.peer().last("readMemory").unwrap().clone();
    assert_eq!(read["arguments"]["memoryReference"], json!("0xc000"));
    assert_eq!(read["arguments"]["count"], json!(1));
}

/// A register asked for before the program has stopped falls through to the
/// symbol table rather than inventing a value.
#[test]
fn a_register_before_any_stop_is_not_invented() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "hl", "context": "repl"}
        }))
        .unwrap();
    assert_eq!(out[0]["success"], json!(false), "no value is made up");
}

/// The chip scopes are declared expensive, because reading them means saving a
/// whole machine.
#[test]
fn the_chip_scopes_are_declared_expensive() {
    for scope in cpclib_dap::inspect::extra_scopes() {
        assert_eq!(scope["expensive"], json!(true), "{scope}");
    }
}

/// Expanding a chip scope asks the machine to describe itself, once, and every
/// scope waiting is answered from that one snapshot.
#[test]
fn the_chip_scopes_share_one_snapshot() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));

    for reference in [
        cpclib_dap::inspect::CRTC_REFERENCE,
        cpclib_dap::inspect::GATE_ARRAY_REFERENCE
    ] {
        let held = session
            .on_editor_message(&json!({
                "seq": 1, "type": "request", "command": "variables",
                "arguments": {"variablesReference": reference}
            }))
            .unwrap();
        assert!(
            held.is_empty(),
            "answered once the machine has described itself"
        );
    }
    assert_eq!(
        session
            .peer()
            .commands()
            .iter()
            .filter(|c| *c == "cpclib/machineState")
            .count(),
        1,
        "two scopes, one whole-machine save"
    );
}

/// A machine that cannot describe itself says why, in the pane, instead of
/// showing an empty scope.
#[test]
fn a_refused_machine_state_says_why() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "variables",
            "arguments": {"variablesReference": cpclib_dap::inspect::CRTC_REFERENCE}
        }))
        .unwrap();

    let asked = session.peer().last("cpclib/machineState").unwrap().clone();
    let out = session.on_emulator_message(&json!({
        "seq": 3, "type": "response", "request_seq": asked["seq"], "success": true,
        "command": "cpclib/machineState",
        "body": {"error": "the snapshot encoder rejected the machine state"}
    }));

    let listed = out[0]["body"]["variables"].as_array().unwrap();
    assert_eq!(listed.len(), 1);
    let text = listed[0]["value"].as_str().unwrap();
    assert!(text.contains("rejected"), "{text}");
}

/// The chip state really comes out of a snapshot, register for register.
///
/// The emulator exposes nothing for the CRTC or the Gate Array; this is the
/// whole mechanism that makes them visible, so it is worth checking against a
/// snapshot with known values rather than only checking that *something*
/// appears.
#[test]
fn a_snapshot_yields_the_chip_registers_and_counters() {
    use cpclib_sna::{Snapshot, SnapshotFlag};

    let mut sna = Snapshot::default();
    sna.set_value(SnapshotFlag::CRTC_REG(Some(1)), 40).unwrap(); // horizontal displayed
    sna.set_value(SnapshotFlag::CRTC_REG(Some(6)), 25).unwrap(); // vertical displayed
    sna.set_value(SnapshotFlag::CRTC_REG(Some(9)), 7).unwrap(); // maximum raster
    sna.set_value(SnapshotFlag::CRTC_SEL, 12).unwrap();
    sna.set_value(SnapshotFlag::CRTC_HCC, 33).unwrap();
    sna.set_value(SnapshotFlag::CRTC_RLC, 200).unwrap();
    sna.set_value(SnapshotFlag::GA_PEN, 3).unwrap();
    sna.set_value(SnapshotFlag::GA_PAL(Some(0)), 0x54).unwrap(); // ink 0
    sna.set_value(SnapshotFlag::GA_PAL(Some(16)), 0x4B).unwrap(); // border
    sna.set_value(SnapshotFlag::GA_ROMCFG, 0b1001).unwrap(); // mode 1
    sna.set_value(SnapshotFlag::PSG_SEL, 7).unwrap();
    sna.set_value(SnapshotFlag::PPI_C, 0x8F).unwrap();

    let crtc =
        cpclib_dap::inspect::chip_variables(cpclib_dap::inspect::CRTC_REFERENCE, &sna).unwrap();
    let of = |list: &[Value], name: &str| -> String {
        list.iter()
            .find(|v| v["name"] == json!(name))
            .and_then(|v| v["value"].as_str())
            .unwrap_or_else(|| panic!("no {name} in {list:?}"))
            .to_string()
    };
    assert_eq!(of(&crtc, "R1"), "0x28 (40)");
    assert_eq!(of(&crtc, "R6"), "0x19 (25)");
    assert_eq!(of(&crtc, "R9"), "0x07 (7)");
    assert_eq!(of(&crtc, "selected"), "0x0C (12)");
    // The counters are why this is worth a whole-machine save on a demo: a
    // raster effect is a race against them.
    assert_eq!(of(&crtc, "HCC"), "0x21 (33)");
    assert_eq!(of(&crtc, "RLC"), "0xC8 (200)");

    let ga = cpclib_dap::inspect::chip_variables(cpclib_dap::inspect::GATE_ARRAY_REFERENCE, &sna)
        .unwrap();
    assert_eq!(of(&ga, "pen"), "0x03 (3)");
    assert_eq!(
        of(&ga, "mode"),
        "1",
        "the low two bits of the ROM/mode byte"
    );

    let psg =
        cpclib_dap::inspect::chip_variables(cpclib_dap::inspect::PSG_REFERENCE, &sna).unwrap();
    assert_eq!(of(&psg, "selected"), "0x07 (7)");

    let ppi =
        cpclib_dap::inspect::chip_variables(cpclib_dap::inspect::PPI_REFERENCE, &sna).unwrap();
    assert_eq!(of(&ppi, "C"), "0x8F (143)");
}

/// The Gate Array palette: pens, not inks; the byte you would write; and the
/// colour it stands for.
#[test]
fn the_palette_reads_as_pens_with_their_colours() {
    use cpclib_sna::{Snapshot, SnapshotFlag};

    let mut sna = Snapshot::default();
    // These are the bytes a program writes to &7Fxx: 0x54 is ink 0 (black),
    // 0x4B is ink 20 (bright white).
    sna.set_value(SnapshotFlag::GA_PAL(Some(0)), 0x4B).unwrap();
    sna.set_value(SnapshotFlag::GA_PAL(Some(3)), 0x54).unwrap();
    sna.set_value(SnapshotFlag::GA_PAL(Some(16)), 0x54).unwrap();
    // The Gate Array is pointed at pen 3.
    sna.set_value(SnapshotFlag::GA_PEN, 3).unwrap();

    let ga = cpclib_dap::inspect::chip_variables(cpclib_dap::inspect::GATE_ARRAY_REFERENCE, &sna)
        .unwrap();
    let names: Vec<String> = ga
        .iter()
        .map(|v| v["name"].as_str().unwrap_or_default().to_string())
        .collect();

    // Sixteen pens and a border - a pen is the slot, an ink is what is in it,
    // and calling the slots inks was simply wrong.
    assert!(names.iter().any(|n| n == "pen 0"), "{names:?}");
    assert!(names.iter().any(|n| n == "pen 15"), "{names:?}");
    assert!(names.iter().any(|n| n == "border"), "{names:?}");
    assert!(!names.iter().any(|n| n.starts_with("ink ")), "{names:?}");

    // The selected pen is underlined - the next &7Fxx colour write lands
    // there - so its name is not the plain one.
    assert!(
        !names.iter().any(|n| n == "pen 3"),
        "pen 3 is marked: {names:?}"
    );
    let selected = names
        .iter()
        .find(|n| n.starts_with('p') && n.contains('3') && n.contains('\u{0332}'))
        .unwrap_or_else(|| panic!("no underlined pen: {names:?}"));
    assert!(selected.contains('\u{0332}'), "{selected:?}");

    let value_of = |name: &str| {
        ga.iter()
            .find(|v| v["name"] == json!(name))
            .and_then(|v| v["value"].as_str())
            .unwrap_or_else(|| panic!("no {name}: {ga:?}"))
            .to_string()
    };
    // The byte a program writes is the colour with bit 6 set, and that is the
    // one to show: it is what you would look for in your own source. Beside it
    // the ink number, which is how the colour is named everywhere else.
    let pen0 = value_of("pen 0");
    assert!(pen0.contains("GA 0x4B"), "the byte you would write: {pen0}");
    assert!(pen0.contains("ink 26"), "and which ink that is: {pen0}");
    // The exact sRGB triple answers no question anyone was asking, so it picks
    // the square and is then dropped.
    assert!(!pen0.contains('#'), "no RGB hex: {pen0}");
    assert!(
        pen0.chars()
            .any(|c| c == '\u{2B1C}' || c == '\u{2B1B}' || c > '\u{1F7E0}'),
        "and a coloured square: {pen0}"
    );

    let border = value_of("border");
    assert!(border.contains("GA 0x54"), "{border}");
}

/// The CRTC register the next `&BDxx` write lands in is underlined too.
#[test]
fn the_selected_crtc_register_is_marked() {
    use cpclib_sna::{Snapshot, SnapshotFlag};

    let mut sna = Snapshot::default();
    sna.set_value(SnapshotFlag::CRTC_SEL, 12).unwrap();

    let crtc =
        cpclib_dap::inspect::chip_variables(cpclib_dap::inspect::CRTC_REFERENCE, &sna).unwrap();
    let names: Vec<String> = crtc
        .iter()
        .map(|v| v["name"].as_str().unwrap_or_default().to_string())
        .collect();

    assert!(
        !names.iter().any(|n| n == "R12"),
        "R12 is marked: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "R11"),
        "and the others are not: {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n.contains('\u{0332}') && n.contains("1") && n.contains("2")),
        "{names:?}"
    );
}

/// The disc scope carries what the snapshot has, and says plainly what it has
/// not - an empty-looking scope invites the question this answers.
#[test]
fn the_disc_scope_states_what_is_missing() {
    use cpclib_sna::{Snapshot, SnapshotFlag};

    let mut sna = Snapshot::default();
    sna.set_value(SnapshotFlag::FDD_MOTOR, 1).unwrap();
    sna.set_value(SnapshotFlag::FDD_TRACK, 9).unwrap();

    let disc =
        cpclib_dap::inspect::chip_variables(cpclib_dap::inspect::DISC_REFERENCE, &sna).unwrap();
    let of = |name: &str| {
        disc.iter()
            .find(|v| v["name"] == json!(name))
            .unwrap_or_else(|| panic!("no {name}: {disc:?}"))
            .clone()
    };
    assert_eq!(of("motor")["value"], json!("0x01 (1)"));
    assert_eq!(of("track")["value"], json!("0x09 (9)"));

    let missing = of("(FDC registers)");
    assert_eq!(missing["value"], json!("not available"));
    let why = missing["type"].as_str().unwrap();
    assert!(
        why.contains("snapshot format carries only motor and track"),
        "{why}"
    );
}

/// A watch may name a place, not just a label.
///
/// `demo_frame_run.wait_lines-1` is what you write when the byte you care about
/// is the one *before* a label - which the self-modifying-code idiom puts there
/// constantly. The emulator implements no expression evaluation at all, but it
/// never needed to: the symbols and the arithmetic are both on this side and
/// only the final address is ever asked of it.
#[test]
fn a_watch_can_carry_an_offset() {
    let map = map_with(&[("wait_lines", 0x8000), ("other", 0x10)]);
    let mut session = Session::new(RecordingPeer::new(), map);

    let asked = |session: &mut Session<RecordingPeer>, expression: &str| -> String {
        session
            .on_editor_message(&json!({
                "seq": 1, "type": "request", "command": "evaluate",
                "arguments": {"expression": expression, "context": "watch"}
            }))
            .unwrap();
        session.peer().last("readMemory").unwrap()["arguments"]["memoryReference"]
            .as_str()
            .unwrap()
            .to_string()
    };

    assert_eq!(asked(&mut session, "wait_lines-1"), "0x7fff");
    assert_eq!(asked(&mut session, "wait_lines + 3"), "0x8003");
    assert_eq!(asked(&mut session, "wait_lines+other"), "0x8010");
    assert_eq!(asked(&mut session, "0x4000+2"), "0x4002");
    // ...and the width suffix still works on top of it.
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "wait_lines-2,w", "context": "watch"}
        }))
        .unwrap();
    let read = session.peer().last("readMemory").unwrap();
    assert_eq!(read["arguments"]["memoryReference"], json!("0x7ffe"));
    assert_eq!(read["arguments"]["count"], json!(2));
}

/// A label that itself contains `-` is still one label, not a subtraction.
#[test]
fn a_label_containing_a_dash_is_not_arithmetic() {
    let map = map_with(&[("demo-frame", 0x9000)]);
    let mut session = Session::new(RecordingPeer::new(), map);
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "demo-frame", "context": "watch"}
        }))
        .unwrap();
    assert_eq!(
        session.peer().last("readMemory").unwrap()["arguments"]["memoryReference"],
        json!("0x9000")
    );
}

/// An offset from something that is not a place is refused, with the near
/// misses named - not silently watched at whatever the sum came to.
#[test]
fn an_offset_from_an_unknown_label_is_refused() {
    let map = map_with(&[("wait_lines", 0x8000)]);
    let mut session = Session::new(RecordingPeer::new(), map);
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "wait_line-1", "context": "watch"}
        }))
        .unwrap();

    assert_eq!(out[0]["success"], json!(false));
    let message = out[0]["message"].as_str().unwrap();
    assert!(
        message.contains("wait_lines"),
        "suggests the real one: {message}"
    );
}

/// `ld a,0` at 0x4000 (2 bytes, 2 NOPs) then `nop` at 0x4002.
fn timed_session() -> Session<RecordingPeer> {
    let mut image = vec![0u8; 0x1_0000];
    image[0x4000] = 0x3E; // ld a,
    image[0x4001] = 0x00; //      0
    image[0x4002] = 0x00; // nop
    Session::new(RecordingPeer::new(), map_with(&[])).with_image(image)
}

fn timer_command(session: &mut Session<RecordingPeer>, line: &str) -> String {
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": line, "context": "repl"}
        }))
        .unwrap();
    out[0]["body"]["result"]
        .as_str()
        .unwrap_or_else(|| panic!("{out:?}"))
        .to_string()
}

/// A timer counts what the program spends, in NOPs, priced by the assembler's
/// own table.
#[test]
fn a_timer_counts_the_nops_of_each_step() {
    let mut session = timed_session();
    report_pc(&mut session, 0x4000);
    assert!(timer_command(&mut session, "-timer add raster").contains("raster"));

    // Step over `ld a,0`: two bytes, two NOPs.
    report_pc(&mut session, 0x4002);
    assert_eq!(timer_command(&mut session, "-timer"), "raster: 2 NOPs");

    // ...and over the `nop`: one more.
    report_pc(&mut session, 0x4003);
    assert_eq!(timer_command(&mut session, "-timer"), "raster: 3 NOPs");
}

/// A timer that spans a free run reports a floor, not a measurement.
///
/// Between a `continue` and the next stop the program runs unobserved, and this
/// emulator exposes no elapsed-time call of any kind - so the honest answer is
/// "at least this much", said plainly.
#[test]
fn a_timer_that_spans_a_free_run_says_so() {
    let mut session = timed_session();
    report_pc(&mut session, 0x4000);
    timer_command(&mut session, "-timer add run");
    report_pc(&mut session, 0x4002);

    // A jump to somewhere unrelated: not a step, so the cost is unknown.
    report_pc(&mut session, 0x9000);

    let listed = timer_command(&mut session, "-timer");
    assert!(listed.starts_with("run: 2 NOPs"), "{listed}");
    assert!(listed.contains("at least"), "{listed}");
    assert!(listed.contains("ran on unobserved"), "{listed}");

    // The pane says the same thing, and marks the total as a floor.
    let shown = session.timer_variables();
    assert_eq!(shown[0]["value"], json!(">= 2 NOPs"));
    assert!(
        shown[0]["type"]
            .as_str()
            .unwrap()
            .contains("reports no elapsed time"),
        "{shown:?}"
    );
}

/// Named, reset and removed.
#[test]
fn timers_can_be_named_reset_and_removed() {
    let mut session = timed_session();
    report_pc(&mut session, 0x4000);

    timer_command(&mut session, "-timer add first");
    timer_command(&mut session, "-timer add second");
    report_pc(&mut session, 0x4002);
    assert_eq!(
        timer_command(&mut session, "-timer"),
        "first: 2 NOPs\nsecond: 2 NOPs"
    );

    assert!(timer_command(&mut session, "-timer reset first").contains("first"));
    assert_eq!(
        timer_command(&mut session, "-timer"),
        "first: 0 NOPs\nsecond: 2 NOPs"
    );

    assert!(timer_command(&mut session, "-timer rm second").contains("removed 1"));
    assert_eq!(timer_command(&mut session, "-timer"), "first: 0 NOPs");

    // An unnamed one gets a name rather than being anonymous.
    timer_command(&mut session, "-timer add");
    assert!(
        timer_command(&mut session, "-timer").contains("timer2"),
        "auto-named"
    );
}

/// With no timers, saying so beats an empty answer.
#[test]
fn listing_no_timers_explains_how_to_start_one() {
    let mut session = timed_session();
    let listed = timer_command(&mut session, "-timer");
    assert!(listed.contains("no timers"), "{listed}");
    assert!(listed.contains("-timer add"), "{listed}");
}

/// An unknown action is refused with the ones that exist.
#[test]
fn an_unknown_timer_action_lists_the_real_ones() {
    let mut session = timed_session();
    let out = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-timer frobnicate", "context": "repl"}
        }))
        .unwrap();
    assert_eq!(out[0]["success"], json!(false));
    let message = out[0]["message"].as_str().unwrap();
    assert!(message.contains("-timer add"), "{message}");
}

/// A stop lands on your source, not on a disassembly view.
///
/// The panel is opened by `-dv` when you want it, never on your behalf: an
/// editor told the adapter can disassemble opens that view the moment any frame
/// lacks a source line - and a reconstructed frame in firmware legitimately
/// does - then switches stepping to instruction granularity and stays there.
#[test]
fn nothing_is_disassembled_unless_asked_for() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[]));

    session.on_emulator_message(&json!({
        "seq": 5, "type": "event", "event": "stopped",
        "body": {"reason": "breakpoint"}
    }));
    report_pc(&mut session, 0x4000);

    assert!(
        !session
            .peer()
            .commands()
            .contains(&"readMemory".to_string()),
        "no view was asked for: {:?}",
        session.peer().commands()
    );

    // ...and the capability that makes the editor open its own is not claimed.
    let capabilities = Session::<RecordingPeer>::capabilities();
    assert!(
        capabilities.get("supportsDisassembleRequest").is_none(),
        "{capabilities}"
    );
}

/// A view the user aimed somewhere is not replaced by the automatic one.
#[test]
fn an_open_view_is_not_hijacked_by_the_next_stop() {
    let mut session = Session::new(RecordingPeer::new(), map_with(&[("draw_sprite", 0x4000)]));
    report_pc(&mut session, 0x9000);
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "evaluate",
            "arguments": {"expression": "-dv draw_sprite", "context": "repl"}
        }))
        .unwrap();
    answer_disassembly(&mut session);

    session.on_emulator_message(&json!({
        "seq": 5, "type": "event", "event": "stopped", "body": {"reason": "step"}
    }));
    report_pc(&mut session, 0x9004);

    let asked = session.peer().last("readMemory").unwrap();
    assert_eq!(
        asked["arguments"]["memoryReference"],
        json!("0x4000"),
        "still where it was pointed"
    );
}

//! Breakpoints the program itself asked for.
//!
//! basm's `BREAKPOINT` directive already expresses everything DeZog spells with
//! `; ASSERTION` / `; WPMEM` comments - type, access, condition, size, mask. The
//! adapter's job is to carry what it can and *say* what it cannot, rather than
//! looking as though a conditional watchpoint was honoured when only its
//! address was.

use cpclib_asm::assembler::delayed_command::{AssembledBreakpoint, AssembledBreakpointKind};
use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::peer::RecordingPeer;
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::json;

fn map() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![
            SourceMapRow::flat(0, 10, 0x4000, 3),
            SourceMapRow::flat(0, 20, 0x8000, 1),
        ]
    })
}

/// The same program, but with the 0x8000 breakpoint living in a second file.
fn two_file_map() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into(), "inc.asm".into()],
        rows: vec![
            SourceMapRow::flat(0, 10, 0x4000, 3),
            SourceMapRow::flat(1, 5, 0x8000, 1),
        ]
    })
}

fn attached() -> Session<RecordingPeer> {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.on_attached().unwrap();
    session
}



fn execution(address: u16) -> AssembledBreakpoint {
    AssembledBreakpoint {
        address,
        page: 0,
        kind: AssembledBreakpointKind::Execution,
        extra: None,
        name: None,
        written_at: None
    }
}

/// A plain `BREAKPOINT` reaches the emulator with no complaint.
#[test]
fn a_plain_directive_is_carried_silently() {
    let mut session = attached();
    let notices = session.adopt_program_breakpoints(&[execution(0x4000)]);
    assert!(notices.is_empty(), "nothing was lost: {notices:?}");

    session.on_attached().unwrap();
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}])
    );
}

/// Clearing the red dot clears the breakpoint - even one a `BREAKPOINT`
/// directive put there.
///
/// In this extension the gutter *writes* the directive: clicking the gutter
/// puts `breakpoint` in the source. So a directive that was present at build
/// time and a red dot the user has since removed are the same breakpoint, and
/// treating directives as untouchable made every gutter breakpoint unremovable
/// for the whole session.
#[test]
fn clearing_the_red_dot_clears_the_directive_behind_it() {
    let mut session = attached();
    session.adopt_program_breakpoints(&[execution(0x8000)]);

    // The editor asks for line 20 - the line the directive is on - then clears
    // it.
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setBreakpoints",
            "arguments": {
                "source": {"path": "main.asm"},
                "breakpoints": [{"line": 20}]
            }
        }))
        .unwrap();
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x8000"}])
    );

    session
        .on_editor_message(&json!({
            "seq": 2, "type": "request", "command": "setBreakpoints",
            "arguments": {"source": {"path": "main.asm"}, "breakpoints": []}
        }))
        .unwrap();
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([]),
        "removing the dot removes the breakpoint"
    );

    // ...and putting it back brings it back, without reassembling anything.
    session
        .on_editor_message(&json!({
            "seq": 3, "type": "request", "command": "setBreakpoints",
            "arguments": {
                "source": {"path": "main.asm"},
                "breakpoints": [{"line": 20}]
            }
        }))
        .unwrap();
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x8000"}])
    );
}

/// What *is* still true: `setBreakpoints` speaks for one file, so a directive
/// in another file is not being spoken about and must not be touched.
#[test]
fn a_directive_in_another_file_is_left_alone() {
    let mut session = Session::new(RecordingPeer::new(), two_file_map());
    session.on_attached().unwrap();
    // 0x8000 is line 5 of inc.asm.
    session.adopt_program_breakpoints(&[execution(0x8000)]);

    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setBreakpoints",
            "arguments": {"source": {"path": "main.asm"}, "breakpoints": []}
        }))
        .unwrap();

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x8000"}]),
        "clearing main.asm says nothing about inc.asm"
    );
}

/// A conditional breakpoint becomes an address breakpoint, and the user is told
/// which half was dropped - naming the line, not just the address.
#[test]
fn a_condition_is_reported_as_unsupported() {
    let mut session = attached();
    let notices = session.adopt_program_breakpoints(&[AssembledBreakpoint {
        address: 0x4000,
        page: 0,
        kind: AssembledBreakpointKind::Execution,
        extra: Some("condition A==3".to_string()),
        name: None,
        written_at: None
    }]);

    assert_eq!(notices.len(), 1);
    let notice = &notices[0];
    assert!(notice.contains("condition A==3"), "{notice}");
    assert!(
        notice.contains("main.asm:10"),
        "names where it is: {notice}"
    );
    assert!(
        notice.contains("not applied"),
        "and that it was not applied: {notice}"
    );
}

/// A memory watchpoint is the one advanced form the emulator can honour - it
/// has watch slots - so it goes to those rather than becoming a stop.
#[test]
fn a_memory_watchpoint_becomes_a_watch_not_a_breakpoint() {
    let mut session = attached();
    session.adopt_program_breakpoints(&[AssembledBreakpoint {
        address: 0x8000,
        page: 0,
        kind: AssembledBreakpointKind::Memory {
            read: false,
            write: true
        },
        extra: None,
        name: Some("animation_state".to_string()),
        written_at: None
    }]);

    let watches = session.watch_requests();
    assert_eq!(watches.len(), 1);
    assert_eq!(watches[0].address, 0x8000);
    assert!(watches[0].write && !watches[0].read);
    assert_eq!(watches[0].label, "animation_state");

    // ...and it is *not* also an execution breakpoint.
    session.on_attached().unwrap();
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(sent["arguments"]["breakpoints"], json!([]));
}

/// An I/O breakpoint has no equivalent at all here; saying so is the whole
/// point.
#[test]
fn an_io_breakpoint_is_reported_as_impossible() {
    let mut session = attached();
    let notices = session.adopt_program_breakpoints(&[AssembledBreakpoint {
        address: 0x4000,
        page: 0,
        kind: AssembledBreakpointKind::Io,
        extra: None,
        name: None,
        written_at: None
    }]);
    assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("I/O breakpoint"), "{}", notices[0]);
    assert!(session.watch_requests().is_empty());
}

/// Watches are armed on the emulator, through the bridge rather than through
/// its DAP session - the write-watch channels are not a DAP concept.
#[test]
fn watches_are_armed_when_the_emulator_attaches() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.adopt_program_breakpoints(&[AssembledBreakpoint {
        address: 0x8000,
        page: 0,
        kind: AssembledBreakpointKind::Memory {
            read: false,
            write: true
        },
        extra: None,
        name: Some("animation_state".to_string()),
        written_at: None
    }]);
    session.on_attached().unwrap();

    let armed = session.peer().last("cpclib/setWatches").unwrap();
    assert_eq!(
        armed["arguments"]["watches"],
        json!([{
            "label": "animation_state", "address": 0x8000,
            "read": false, "write": true
        }])
    );
}

/// The `watchLabels` launch attribute resolves through the program's symbols.
#[test]
fn configured_watch_labels_are_armed_too() {
    let map = map().with_symbols(
        [("scroll_offset".to_string(), 0x9000u32)]
            .into_iter()
            .collect()
    );
    let mut session = Session::new(RecordingPeer::new(), map);

    let problems = session.add_watch_labels(&["scroll_offset".to_string()]);
    assert!(problems.is_empty(), "{problems:?}");
    session.on_attached().unwrap();

    let armed = session.peer().last("cpclib/setWatches").unwrap();
    assert_eq!(
        armed["arguments"]["watches"][0]["label"],
        json!("scroll_offset")
    );
    assert_eq!(armed["arguments"]["watches"][0]["address"], json!(0x9000));
}

/// A label that is not in the program is a typo, not a silent no-op.
#[test]
fn an_unknown_watch_label_is_reported() {
    let map = map().with_symbols(
        [("scroll_offset".to_string(), 0x9000u32)]
            .into_iter()
            .collect()
    );
    let mut session = Session::new(RecordingPeer::new(), map);

    let problems = session.add_watch_labels(&["scroll_ofset".to_string()]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("scroll_ofset"), "{}", problems[0]);
    assert!(
        problems[0].contains("scroll_offset"),
        "suggests it: {}",
        problems[0]
    );
}

/// A watch the emulator had no channel for is named, because a watch that
/// silently does nothing is indistinguishable from a variable never written.
#[test]
fn a_watch_that_did_not_fit_is_reported() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.adopt_program_breakpoints(&[AssembledBreakpoint {
        address: 0x8000,
        page: 0,
        kind: AssembledBreakpointKind::Memory {
            read: false,
            write: true
        },
        extra: None,
        name: Some("animation_state".to_string()),
        written_at: None
    }]);
    session.on_attached().unwrap();

    let seq = session.peer().last("cpclib/setWatches").unwrap()["seq"]
        .as_i64()
        .unwrap();
    let out = session.on_emulator_message(&json!({
        "seq": 9, "type": "response", "request_seq": seq, "success": true,
        "command": "cpclib/setWatches",
        "body": {"applied": [], "rejected": ["animation_state"]}
    }));

    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["event"], json!("output"));
    let text = out[0]["body"]["output"].as_str().unwrap();
    assert!(text.contains("animation_state"), "{text}");
}

/// `stopOnEntry` is a breakpoint at the snapshot's start address, and it is not
/// the editor's to clear.
#[test]
fn stop_on_entry_breaks_where_the_program_starts() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.stop_on_entry(0x4000);
    session.on_attached().unwrap();

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}])
    );

    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setBreakpoints",
            "arguments": {"source": {"path": "main.asm"}, "breakpoints": []}
        }))
        .unwrap();
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}]),
        "clearing the editor's breakpoints does not clear the entry stop"
    );
}

/// `stopOnEntry` means *entry*, once.
///
/// Left armed it would stop again every time a main loop came back past the
/// entry address - and it would hold one of the emulator's scarce breakpoint
/// channels for the whole session.
#[test]
fn the_entry_stop_is_retired_after_the_first_stop() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.stop_on_entry(0x4000);
    session.on_attached().unwrap();

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}])
    );

    let out = session.on_emulator_message(&json!({
        "seq": 9, "type": "event", "event": "stopped", "body": {"reason": "breakpoint"}
    }));
    assert_eq!(out[0]["event"], json!("stopped"), "the stop still goes out");

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([]),
        "and the entry breakpoint is gone"
    );
}

/// A breakpoint the *program* asked for is not retired - it is in the source,
/// and it means every time.
#[test]
fn a_program_breakpoint_survives_the_first_stop() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session.adopt_program_breakpoints(&[execution(0x4000)]);
    session.on_attached().unwrap();

    session.on_emulator_message(&json!({
        "seq": 9, "type": "event", "event": "stopped", "body": {"reason": "breakpoint"}
    }));

    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}])
    );
}

/// A label can be watched from the editor's own UI, not only from a directive
/// or from launch.json.
#[test]
fn a_label_can_be_watched_from_the_editor() {
    let map = map().with_symbols(
        [("animation_state".to_string(), 0x9000u32)]
            .into_iter()
            .collect()
    );
    let mut session = Session::new(RecordingPeer::new(), map);
    session.on_attached().unwrap();

    // The editor asks whether the name can be watched...
    let info = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "dataBreakpointInfo",
            "arguments": {"name": "animation_state"}
        }))
        .unwrap();
    let data_id = info[0]["body"]["dataId"].as_str().unwrap().to_string();
    assert_eq!(info[0]["body"]["accessTypes"], json!(["write"]));
    // ...and is told, where the user reads it, what this emulator cannot do.
    let description = info[0]["body"]["description"].as_str().unwrap();
    assert!(description.contains("cannot stop"), "{description}");

    // ...then sets it, and the channel is armed.
    let set = session
        .on_editor_message(&json!({
            "seq": 2, "type": "request", "command": "setDataBreakpoints",
            "arguments": {"breakpoints": [{"dataId": data_id, "accessType": "write"}]}
        }))
        .unwrap();
    assert_eq!(set[0]["body"]["breakpoints"][0]["verified"], json!(true));

    let armed = session.peer().last("cpclib/setWatches").unwrap();
    assert_eq!(
        armed["arguments"]["watches"],
        json!([{"label": "animation_state", "address": 0x9000, "read": false, "write": true}])
    );
}

/// Clearing them disarms the channel rather than leaving it armed forever.
#[test]
fn clearing_the_editors_watchpoints_disarms_the_channel() {
    let map = map().with_symbols(
        [("animation_state".to_string(), 0x9000u32)]
            .into_iter()
            .collect()
    );
    let mut session = Session::new(RecordingPeer::new(), map);
    session.add_watch_labels(&["animation_state".to_string()]);
    session.on_attached().unwrap();

    session
        .on_editor_message(&json!({
            "seq": 2, "type": "request", "command": "setDataBreakpoints",
            "arguments": {"breakpoints": []}
        }))
        .unwrap();

    let armed = session.peer().last("cpclib/setWatches").unwrap();
    assert_eq!(armed["arguments"]["watches"], json!([]));
}

/// A name the program does not have is refused with a null dataId, which is the
/// protocol's way of saying "not here" - and the editor shows the reason.
#[test]
fn an_unwatchable_name_is_refused() {
    let mut session = Session::new(RecordingPeer::new(), map());
    let info = session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "dataBreakpointInfo",
            "arguments": {"name": "no_such_label"}
        }))
        .unwrap();

    assert_eq!(info[0]["body"]["dataId"], json!(null));
    let description = info[0]["body"]["description"].as_str().unwrap();
    assert!(description.contains("no_such_label"), "{description}");
}

/// `configurationDone` still reaches the emulator - it wants it too.
#[test]
fn configuration_done_is_still_forwarded() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "configurationDone"
        }))
        .unwrap();
    assert!(
        session
            .peer()
            .commands()
            .contains(&"configurationDone".to_string()),
        "{:?}",
        session.peer().commands()
    );
}

/// Report where the program is, the way a real stop does.
fn at_pc(session: &mut Session<RecordingPeer>, pc: u16) {
    session.on_emulator_message(&json!({
        "seq": 4, "type": "response", "request_seq": 2, "success": true,
        "command": "variables",
        "body": {"variables": [{"name": "PC", "value": format!("0x{pc:04X}"),
                                "variablesReference": 0}]}
    }));
}

/// A banked program: 0x5C3A is claimed by two pages, both in the same file.
fn banked_map() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["writter.asm".into()],
        rows: vec![
            SourceMapRow {
                file: 0,
                line: 77,
                logical: 0x5C3A,
                physical: 0x5C3A,
                page: 0,
                column: 5,
                column_end: 10,
                len: 1
            },
            SourceMapRow {
                file: 0,
                line: 242,
                logical: 0x5C3A,
                physical: 0x1_5C3A,
                page: 1,
                column: 5,
                column_end: 10,
                len: 1
            },
        ]
    })
}

/// Clearing a breakpoint works on a banked program too.
///
/// `location_at` answers nothing at all for an address two banks both hold, so
/// asking it whether the breakpoint was in the file being cleared said "no" for
/// every banked breakpoint - and removing the red dot did nothing.
#[test]
fn a_breakpoint_at_a_banked_address_can_still_be_cleared() {
    let mut session = Session::new(RecordingPeer::new(), banked_map());
    session.adopt_program_breakpoints(&[execution(0x5C3A)]);
    session.on_attached().unwrap();

    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setBreakpoints",
            "arguments": {"source": {"path": "writter.asm"}, "breakpoints": []}
        }))
        .unwrap();

    let armed = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        armed["arguments"]["breakpoints"],
        json!([]),
        "cleared, even though two banks claim the address"
    );
}

/// ...and putting it back re-arms it.
#[test]
fn a_banked_breakpoint_comes_back_when_asked_for_again() {
    let mut session = Session::new(RecordingPeer::new(), banked_map());
    session.adopt_program_breakpoints(&[execution(0x5C3A)]);
    session.on_attached().unwrap();

    for breakpoints in [json!([]), json!([{"line": 77}])] {
        session
            .on_editor_message(&json!({
                "seq": 1, "type": "request", "command": "setBreakpoints",
                "arguments": {"source": {"path": "writter.asm"}, "breakpoints": breakpoints}
            }))
            .unwrap();
    }

    let armed = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        armed["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x5c3a"}])
    );
}

/// The adapter does **not** resume the program at launch.
///
/// It used to hold it with a `pause` while breakpoints were armed, then release
/// it - the protocol's own answer to the emulator starting execution the moment
/// its snapshot loads. That pause stopped the CPU dead: every `stepIn` was
/// accepted and answered "Instruction step completed" while `PC`, `AF` and `HL`
/// stayed identical across fifty steps. The machine does not come back from it.
#[test]
fn launching_does_not_touch_the_run_state() {
    let mut session = Session::new(RecordingPeer::new(), map());
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "configurationDone"
        }))
        .unwrap();
    session.on_attached().unwrap();

    let commands = session.peer().commands();
    assert!(!commands.contains(&"pause".to_string()), "{commands:?}");
    assert!(!commands.contains(&"continue".to_string()), "{commands:?}");
    // The breakpoints still go out, which is the part that always worked.
    assert!(commands.contains(&"setInstructionBreakpoints".to_string()));
}

/// Stepping off an address that has a breakpoint on it lifts it first.
///
/// The emulator cannot leave an address it has a breakpoint on: it resumes,
/// re-detects the breakpoint at the address it has not left yet, and stops
/// again - answering "Instruction step completed" with the program counter
/// unmoved. Removed twice on mistaken readings, and restored both times by a
/// transcript showing `PC` sitting still across six consecutive steps.
#[test]
fn stepping_lifts_the_breakpoint_under_the_program_counter() {
    let mut session = attached();
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setBreakpoints",
            "arguments": {"source": {"path": "main.asm"}, "breakpoints": [{"line": 10}]}
        }))
        .unwrap();
    // The program is sitting on it.
    session.on_emulator_message(&json!({
        "seq": 4, "type": "response", "request_seq": 2, "success": true,
        "command": "variables",
        "body": {"variables": [{"name": "PC", "value": "0x4000", "variablesReference": 0}]}
    }));

    session
        .on_editor_message(&json!({
            "seq": 2, "type": "request", "command": "stepIn",
            "arguments": {"threadId": 1}
        }))
        .unwrap();
    let armed = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        armed["arguments"]["breakpoints"],
        json!([]),
        "the one under our feet is out of the way"
    );
    assert!(session.peer().commands().contains(&"stepIn".to_string()));

    // Once it has moved, the breakpoint goes back.
    session.on_emulator_message(&json!({
        "seq": 9, "type": "event", "event": "stopped", "body": {"reason": "step"}
    }));
    let armed = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        armed["arguments"]["breakpoints"],
        json!([{"instructionReference": "0x4000"}]),
        "put back at the next stop"
    );
}

/// The directives are named in the console before the program runs.
///
/// Reported from a real session as "continue does not work properly - the
/// emulator stopped at random locations without any breakpoint". Every one of
/// those locations was a `BREAKPOINT` the program itself asked for: one written
/// inside a macro body, armed once for each of the eight places the macro was
/// used, in a file the user had never put a red dot in. The editor's gutter
/// cannot show them, so the only way they are not invisible is to say so.
#[test]
fn the_directives_are_listed_before_the_program_runs() {
    let mut session = attached();
    session.adopt_program_breakpoints(&[execution(0x4000), execution(0x8000)]);

    let notice = session
        .program_breakpoint_notice()
        .expect("the user is told what is armed");
    assert!(notice.contains("main.asm:10"), "{notice}");
    assert!(notice.contains("main.asm:20"), "{notice}");
    assert!(
        notice.contains("macro"),
        "and why there are so many: {notice}"
    );
}

/// Nothing to say about a program with no directives in it.
#[test]
fn a_program_without_directives_says_nothing() {
    let session = attached();
    assert!(session.program_breakpoint_notice().is_none());
}

/// A directive the editor has taken back is no longer armed, so it is no longer
/// announced either - the notice describes what will stop the program, not what
/// the assembler happened to emit.
#[test]
fn a_suppressed_directive_is_not_announced() {
    let mut session = attached();
    session.adopt_program_breakpoints(&[execution(0x4000), execution(0x8000)]);

    // main.asm with no red dots at all takes both of them back.
    session
        .on_editor_message(&json!({
            "seq": 1, "type": "request", "command": "setBreakpoints",
            "arguments": {"source": {"path": "main.asm"}, "breakpoints": []}
        }))
        .unwrap();

    assert!(session.program_breakpoint_notice().is_none());
}

/// A directive written in a macro body, armed at the address of the
/// instruction the expansion put after it.
fn from_a_macro(address: u16, file: &str, line: u32) -> AssembledBreakpoint {
    AssembledBreakpoint {
        address,
        page: 0,
        kind: AssembledBreakpointKind::Execution,
        extra: None,
        name: None,
        written_at: Some(
            cpclib_asm::assembler::delayed_command::BreakpointSource {
                file: file.to_string(),
                line,
                column: 2
            }
        )
    }
}

/// Stop the program at `address` on a breakpoint, and hand back everything the
/// adapter said.
fn stop_on_a_breakpoint(
    session: &mut Session<RecordingPeer>,
    address: u16
) -> Vec<serde_json::Value> {
    session.on_emulator_message(&json!({
        "type": "event", "event": "stopped",
        "body": {"reason": "breakpoint", "threadId": 1}
    }));
    session.on_emulator_message(&json!({
        "type": "response", "command": "stackTrace", "success": true,
        "body": {"stackFrames": [{
            "id": 1, "name": format!("Z80 @ 0x{address:04X}"), "line": 0,
            "instructionPointerReference": format!("0x{address:04X}")
        }]}
    }))
}

/// The whole reason a per-stop link is needed: the directive is in one file
/// and the stop is in another, with no red dot anywhere to explain it.
#[test]
fn a_stop_on_a_macro_directive_links_to_where_it_is_written() {
    let mut session = attached();
    session.adopt_program_breakpoints(&[from_a_macro(0x4000, "macros.asm", 4)]);

    let out = stop_on_a_breakpoint(&mut session, 0x4000);
    let note = out
        .iter()
        .find(|message| {
            message["event"] == json!("output")
                && message["body"]["output"]
                    .as_str()
                    .is_some_and(|text| text.contains("BREAKPOINT"))
        })
        .unwrap_or_else(|| panic!("{out:#?}"));

    let text = note["body"]["output"].as_str().unwrap();
    assert!(
        text.contains("macros.asm:4:2"),
        "a location an editor turns into a link: {text}"
    );
}

/// A directive on the line the program stopped on is already on screen; a link
/// to it would be noise.
#[test]
fn a_directive_on_the_stopped_line_gets_no_link() {
    let mut session = attached();
    // `main.asm:10` is what `0x4000` resolves to in this map.
    session.adopt_program_breakpoints(&[from_a_macro(0x4000, "main.asm", 10)]);

    let out = stop_on_a_breakpoint(&mut session, 0x4000);
    assert!(
        !out.iter().any(|message| {
            message["body"]["output"]
                .as_str()
                .is_some_and(|text| text.contains("BREAKPOINT"))
        }),
        "{out:#?}"
    );
}

/// Stepping onto an armed address is the user walking there themselves, and
/// saying so on every step through a macro-heavy demo would be noise.
#[test]
fn a_step_onto_a_directive_says_nothing() {
    let mut session = attached();
    session.adopt_program_breakpoints(&[from_a_macro(0x4000, "macros.asm", 4)]);

    session.on_emulator_message(&json!({
        "type": "event", "event": "stopped",
        "body": {"reason": "step", "threadId": 1}
    }));
    let out = session.on_emulator_message(&json!({
        "type": "response", "command": "stackTrace", "success": true,
        "body": {"stackFrames": [{
            "id": 1, "name": "Z80 @ 0x4000", "line": 0,
            "instructionPointerReference": "0x4000"
        }]}
    }));
    assert!(
        !out.iter().any(|message| {
            message["body"]["output"]
                .as_str()
                .is_some_and(|text| text.contains("BREAKPOINT"))
        }),
        "{out:#?}"
    );
}

/// A stop that no program directive armed says nothing about directives.
#[test]
fn a_stop_at_an_unarmed_address_gets_no_link() {
    let mut session = attached();
    session.adopt_program_breakpoints(&[from_a_macro(0x8000, "macros.asm", 4)]);

    let out = stop_on_a_breakpoint(&mut session, 0x4000);
    assert!(
        !out.iter().any(|message| {
            message["body"]["output"]
                .as_str()
                .is_some_and(|text| text.contains("BREAKPOINT"))
        }),
        "{out:#?}"
    );
}

/// A directive names its file the way the `include` that pulled it in did,
/// which is routinely a bare name - and a bare name is not a link. The source
/// map holds the same file as an absolute path, and that is what goes out.
#[test]
fn a_relative_directive_file_is_resolved_against_the_source_map() {
    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec!["/somewhere/src/main.asm".into()],
        rows: vec![SourceMapRow::flat(0, 10, 0x4000, 3)]
    });
    let mut session = Session::new(RecordingPeer::new(), map);
    session.on_attached().unwrap();
    session.adopt_program_breakpoints(&[from_a_macro(0x4000, "macros.asm", 4)]);

    let out = stop_on_a_breakpoint(&mut session, 0x4000);
    let note = out
        .iter()
        .find(|message| {
            message["body"]["output"]
                .as_str()
                .is_some_and(|text| text.contains("BREAKPOINT"))
        })
        .unwrap_or_else(|| panic!("{out:#?}"));
    let text = note["body"]["output"].as_str().unwrap();
    // `macros.asm` is not in this map at all, so it stays as the directive
    // spelled it - there is nothing better to say.
    assert!(text.contains("macros.asm:4:2"), "{text}");
}

/// The same, for a file the map does know: the absolute path wins.
#[test]
fn a_known_relative_file_becomes_the_path_the_editor_can_open() {
    let map = SourceMap::from_raw(&RawSourceMap {
        files: vec!["/somewhere/src/main.asm".into(), "/somewhere/src/macros.asm".into()],
        rows: vec![
            SourceMapRow::flat(0, 10, 0x4000, 3),
            SourceMapRow::flat(1, 6, 0x8000, 1),
        ]
    });
    let mut session = Session::new(RecordingPeer::new(), map);
    session.on_attached().unwrap();
    session.adopt_program_breakpoints(&[from_a_macro(0x4000, "macros.asm", 5)]);

    let out = stop_on_a_breakpoint(&mut session, 0x4000);
    let note = out
        .iter()
        .find(|message| {
            message["body"]["output"]
                .as_str()
                .is_some_and(|text| text.contains("BREAKPOINT"))
        })
        .unwrap_or_else(|| panic!("{out:#?}"));
    let text = note["body"]["output"].as_str().unwrap();
    assert!(text.contains("/somewhere/src/macros.asm:5:2"), "{text}");
}

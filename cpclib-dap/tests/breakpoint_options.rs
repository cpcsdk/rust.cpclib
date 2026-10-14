//! Conditions, hit counts, log points and stopping data breakpoints: carried
//! to an emulator that applies them (bemu), and refused with a reason by one
//! that cannot, rather than turned into something else.

use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::peer::RecordingPeer;
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::{Value, json};

/// `main.asm` lines 10/11/12 at 0x4000/0x4003/0x4004.
fn fixture() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![
            SourceMapRow::flat(0, 10, 0x4000, 3),
            SourceMapRow::flat(0, 11, 0x4003, 1),
            SourceMapRow::flat(0, 12, 0x4004, 2),
        ]
    })
}

const BEMU: &[&str] = &["cpclib/conditionalBreakpoints", "cpclib/setDataBreakpoints"];

fn session(peer: RecordingPeer) -> Session<RecordingPeer> {
    let mut session = Session::new(peer, fixture());
    session.on_attached().unwrap();
    session
}

fn request(session: &mut Session<RecordingPeer>, command: &str, arguments: Value) -> Value {
    let request = json!({ "seq": 1, "type": "request", "command": command, "arguments": arguments });
    session
        .on_editor_message(&request)
        .unwrap()
        .into_iter()
        .find(|m| m["type"] == "response")
        .expect("answered by the adapter")
}

fn set_breakpoints(session: &mut Session<RecordingPeer>, breakpoints: Value) -> Value {
    request(session, "setBreakpoints", json!({ "source": { "path": "main.asm" }, "breakpoints": breakpoints }))
}

#[test]
fn the_capabilities_are_advertised() {
    let capabilities = Session::<RecordingPeer>::capabilities();
    for capability in ["supportsConditionalBreakpoints", "supportsHitConditionalBreakpoints", "supportsLogPoints"] {
        assert_eq!(capabilities[capability], true, "{capability}");
    }
}

#[test]
fn conditions_hit_counts_and_log_messages_reach_an_emulator_that_applies_them() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    let answer = set_breakpoints(
        &mut session,
        json!([
            { "line": 10 },
            { "line": 11, "condition": "A == 3", "hitCondition": ">=4" },
            { "line": 12, "logMessage": "A is {A}" }
        ])
    );
    for breakpoint in answer["body"]["breakpoints"].as_array().unwrap() {
        assert_eq!(breakpoint["verified"], true, "{breakpoint}");
        assert!(breakpoint.get("message").is_none(), "{breakpoint}");
    }
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([
            { "instructionReference": "0x4000" },
            { "instructionReference": "0x4003", "condition": "A == 3", "after": 3 },
            { "instructionReference": "0x4004", "logMessage": "A is {A}" }
        ])
    );
}

#[test]
fn hit_counts_are_the_hits_to_let_pass() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    for (hit_condition, after) in [("1", 0), ("5", 4), (">=5", 4), (">5", 5), (" > 2 ", 2)] {
        set_breakpoints(&mut session, json!([{ "line": 11, "hitCondition": hit_condition }]));
        let sent = session.peer().last("setInstructionBreakpoints").unwrap();
        assert_eq!(sent["arguments"]["breakpoints"][0]["after"], after, "{hit_condition}");
    }
    let answer = set_breakpoints(&mut session, json!([{ "line": 11, "hitCondition": "%3" }]));
    let breakpoint = &answer["body"]["breakpoints"][0];
    assert_eq!(breakpoint["verified"], true, "placed, without the count");
    assert!(breakpoint["message"].as_str().unwrap().contains("hit count"), "{breakpoint}");
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(sent["arguments"]["breakpoints"], json!([{ "instructionReference": "0x4003" }]));
}

/// A plain breakpoint stops every time: one at the same address as a
/// conditional one wins.
#[test]
fn a_plain_breakpoint_wins_over_a_conditional_one_at_its_address() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    // Lines 9 and 10 both land on 0x4000.
    set_breakpoints(&mut session, json!([{ "line": 9, "condition": "A == 1" }, { "line": 10 }]));
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(sent["arguments"]["breakpoints"], json!([{ "instructionReference": "0x4000" }]));
}

/// A condition and a log point at one address are two breakpoints: neither may
/// be dropped in favour of the other.
#[test]
fn two_different_advanced_breakpoints_at_one_address_both_reach_the_emulator() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    set_breakpoints(
        &mut session,
        json!([{ "line": 9, "condition": "A == 1" }, { "line": 10, "logMessage": "hit" }])
    );
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([
            { "instructionReference": "0x4000", "condition": "A == 1" },
            { "instructionReference": "0x4000", "logMessage": "hit" }
        ])
    );
}

#[test]
fn an_emulator_that_cannot_apply_them_says_so() {
    let mut session = session(RecordingPeer::new());
    let answer = set_breakpoints(
        &mut session,
        json!([{ "line": 11, "condition": "A == 3" }, { "line": 12, "logMessage": "hello" }])
    );
    let conditional = &answer["body"]["breakpoints"][0];
    assert_eq!(conditional["verified"], true, "still a breakpoint");
    assert!(conditional["message"].as_str().unwrap().contains("stops on every pass"), "{conditional}");
    let log_point = &answer["body"]["breakpoints"][1];
    assert_eq!(log_point["verified"], false, "a log point that would stop is not placed");
    assert!(log_point["message"].as_str().unwrap().contains("log"), "{log_point}");
    let sent = session.peer().last("setInstructionBreakpoints").unwrap();
    assert_eq!(sent["arguments"]["breakpoints"], json!([{ "instructionReference": "0x4003" }]));
}

#[test]
fn data_breakpoints_stop_on_an_emulator_that_can() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    let info = request(&mut session, "dataBreakpointInfo", json!({ "name": "0xC000" }));
    assert_eq!(info["body"]["accessTypes"], json!(["write", "read", "readWrite"]));
    assert!(info["body"]["description"].as_str().unwrap().contains("stops"), "{info}");

    let data_id = info["body"]["dataId"].clone();
    let answer = request(
        &mut session,
        "setDataBreakpoints",
        json!({ "breakpoints": [{ "dataId": data_id, "accessType": "readWrite", "condition": "value == 2", "hitCondition": "2" }] })
    );
    assert_eq!(answer["body"]["breakpoints"][0]["verified"], true, "{answer}");
    let sent = session.peer().last("cpclib/setDataBreakpoints").unwrap();
    assert_eq!(
        sent["arguments"]["breakpoints"],
        json!([{ "label": "0xC000", "address": 0xC000, "access": "rw", "condition": "value == 2", "after": 1 }])
    );
}

#[test]
fn data_breakpoints_stay_write_watches_elsewhere() {
    let mut session = session(RecordingPeer::new());
    let info = request(&mut session, "dataBreakpointInfo", json!({ "name": "0xC000" }));
    assert_eq!(info["body"]["accessTypes"], json!(["write"]));
    let data_id = info["body"]["dataId"].clone();
    request(&mut session, "setDataBreakpoints", json!({ "breakpoints": [{ "dataId": data_id }] }));
    assert!(session.peer().last("cpclib/setDataBreakpoints").is_none());
    assert!(session.peer().last("cpclib/setWatches").is_some());
}

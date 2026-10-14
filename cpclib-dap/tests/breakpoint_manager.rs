//! The breakpoint manager (ACE's breakpoint window): breakpoints made by
//! hand - execution, memory, I/O port - listed by tab with every other
//! breakpoint of the session.

use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::peer::RecordingPeer;
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::{Value, json};

/// `main.asm` lines 10/11 at 0x4000/0x4003.
fn map() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into()],
        rows: vec![SourceMapRow::flat(0, 10, 0x4000, 3), SourceMapRow::flat(0, 11, 0x4003, 1)]
    })
}

const BEMU: &[&str] = &["cpclib/conditionalBreakpoints", "cpclib/setDataBreakpoints", "cpclib/setManagerBreakpoints"];

fn session(peer: RecordingPeer) -> Session<RecordingPeer> {
    let mut session = Session::new(peer, map());
    session.on_attached().unwrap();
    session
}

fn request(session: &mut Session<RecordingPeer>, command: &str, arguments: Value) -> Value {
    let request = json!({ "seq": 1, "type": "request", "command": command, "arguments": arguments });
    session.on_editor_message(&request).unwrap().into_iter().find(|m| m["type"] == "response").unwrap()
}

fn set(session: &mut Session<RecordingPeer>, breakpoint: Value) -> Value {
    request(session, "cpclib/managerSet", json!({ "breakpoint": breakpoint }))
}

fn tabs(session: &mut Session<RecordingPeer>) -> Value {
    request(session, "cpclib/breakpointManager", json!({}))["body"]["tabs"].clone()
}

#[test]
fn breakpoints_made_by_hand_reach_an_emulator_that_takes_them_whole() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    let exec = set(&mut session, json!({ "kind": "exec", "address": "&4003", "condition": "BC=7 AND FlagC", "name": "TrackDone" }));
    assert_eq!(exec["success"], true, "{exec}");
    let port = set(&mut session, json!({ "kind": "io", "address": "&FB01", "mask": "&FF81", "access": "rw", "mode": "watch", "step": "2" }));
    assert_eq!(port["body"]["id"], 2);

    let sent = session.peer().last("cpclib/setManagerBreakpoints").unwrap();
    assert_eq!(sent["arguments"]["breakpoints"], json!([
        { "id": 1, "label": "TrackDone", "kind": "exec", "access": "rw", "run": "stop", "addr": 0x4003, "mask": 0xffff,
          "size": 1, "value": 0, "valmask": 0, "condition": "BC=7 AND FlagC" },
        { "id": 2, "label": "&FB01", "kind": "io", "access": "rw", "run": "watch", "addr": 0xfb01, "mask": 0xff81,
          "size": 1, "value": 0, "valmask": 0, "after": 2 }
    ]));
    // Not doubled as a plain address.
    assert_eq!(session.peer().last("setInstructionBreakpoints").unwrap()["arguments"]["breakpoints"], json!([]));

    // Listed by tab, editable, beside a red dot that is not.
    request(&mut session, "setBreakpoints", json!({ "source": { "path": "main.asm" }, "breakpoints": [{ "line": 10 }] }));
    let listed = tabs(&mut session);
    let exec = listed["exec"].as_array().unwrap();
    assert_eq!(exec.len(), 2, "{listed}");
    let red_dot = exec.iter().find(|r| r["origin"] == "source").unwrap();
    assert_eq!((red_dot["editable"].as_bool(), red_dot["key"].as_str()), (Some(false), Some("source:0x4000")));
    let manual = exec.iter().find(|r| r["origin"] == "manager").unwrap();
    assert_eq!((manual["address"].as_str(), manual["condition"].as_str(), manual["key"].as_str()), (Some("&4003"), Some("BC=7 AND FlagC"), Some("manager:1")));
    assert_eq!(listed["io"][0]["mask"], "&FF81");
    assert_eq!(listed["io"][0]["mode"], "watch");

    // Updated in place, removed, cleared by tab.
    set(&mut session, json!({ "id": 1, "kind": "exec", "address": "&4000", "name": "moved" }));
    assert_eq!(tabs(&mut session)["exec"].as_array().unwrap().iter().filter(|r| r["origin"] == "manager").count(), 1);
    assert_eq!(request(&mut session, "cpclib/managerRemove", json!({ "id": 1 }))["success"], true);
    assert_eq!(request(&mut session, "cpclib/managerRemove", json!({ "id": 1 }))["success"], false, "already gone");
    request(&mut session, "cpclib/managerClear", json!({ "kind": "io" }));
    assert_eq!(session.peer().last("cpclib/setManagerBreakpoints").unwrap()["arguments"]["breakpoints"], json!([]));
}

#[test]
fn a_label_is_resolved_and_a_bad_form_refused() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    let refused = set(&mut session, json!({ "kind": "exec", "address": "nowhere" }));
    assert_eq!(refused["success"], false);
    assert!(refused["message"].as_str().unwrap().contains("label"), "{refused}");
}

/// Elsewhere a plain execution breakpoint still stops on its address; the
/// others are listed as not armed, and why.
#[test]
fn an_emulator_without_them_keeps_what_it_can() {
    let mut session = session(RecordingPeer::new());
    set(&mut session, json!({ "kind": "exec", "address": "&4003" }));
    set(&mut session, json!({ "kind": "mem", "address": "&C000", "access": "w" }));
    assert_eq!(
        session.peer().last("setInstructionBreakpoints").unwrap()["arguments"]["breakpoints"],
        json!([{ "instructionReference": "0x4003" }])
    );
    assert!(session.peer().last("cpclib/setManagerBreakpoints").is_none());
    let tabs = tabs(&mut session);
    assert_eq!(tabs["exec"][0]["status"], "armed");
    assert_eq!(tabs["mem"][0]["status"], "inactive");
    assert!(tabs["mem"][0]["note"].as_str().unwrap().contains("bemu"));
}

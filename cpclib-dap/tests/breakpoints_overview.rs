//! `cpclib/breakpoints`: every kind of breakpoint the session holds, each
//! explained, for the extension's breakpoints view - with whether this
//! emulator applies it as asked.

use cpclib_asm::assembler::delayed_command::{AssembledBreakpoint, AssembledBreakpointKind, BreakpointDetails};
use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_dap::peer::RecordingPeer;
use cpclib_dap::session::Session;
use cpclib_project::srcmap::SourceMap;
use serde_json::{Value, json};

/// `main.asm` lines 10/11/12 at 0x4000/0x4003/0x4004; `ink.asm` line 5 at
/// 0x8000, where the I/O directive is (red dots set in `main.asm` do not
/// speak about it).
fn map() -> SourceMap {
    SourceMap::from_raw(&RawSourceMap {
        files: vec!["main.asm".into(), "ink.asm".into()],
        rows: vec![
            SourceMapRow::flat(0, 10, 0x4000, 3),
            SourceMapRow::flat(0, 11, 0x4003, 1),
            SourceMapRow::flat(0, 12, 0x4004, 2),
            SourceMapRow::flat(1, 5, 0x8000, 1),
        ]
    })
}

const BEMU: &[&str] = &["cpclib/conditionalBreakpoints", "cpclib/setDataBreakpoints", "cpclib/setProgramBreakpoints"];

fn request(session: &mut Session<RecordingPeer>, command: &str, arguments: Value) -> Value {
    let request = json!({ "seq": 1, "type": "request", "command": command, "arguments": arguments });
    session
        .on_editor_message(&request)
        .unwrap()
        .into_iter()
        .find(|m| m["type"] == "response")
        .expect("answered by the adapter")
}

fn io_directive() -> AssembledBreakpoint {
    AssembledBreakpoint {
        address: 0x8000,
        page: 0,
        kind: AssembledBreakpointKind::Io,
        extra: Some("condition A = &55".to_owned()),
        name: Some("ink".to_owned()),
        details: Some(BreakpointDetails {
            access: "w".to_owned(),
            watch: false,
            mask: 0xff00,
            size: 1,
            value: 0x40,
            value_mask: 0xc0,
            condition: Some("A = &55".to_owned()),
            step: None
        }),
        written_at: None
    }
}

/// A session with one of each: a conditional red dot, an I/O directive, a
/// data breakpoint, a watch label and the stop on entry.
fn session(peer: RecordingPeer) -> Session<RecordingPeer> {
    let mut session = Session::new(peer, map());
    session.adopt_program_breakpoints(&[io_directive()]);
    session.stop_on_entry(0x4000);
    session.on_attached().unwrap();
    request(
        &mut session,
        "setBreakpoints",
        json!({ "source": { "path": "main.asm" }, "breakpoints": [{ "line": 11, "condition": "A = 5", "hitCondition": "3" }] })
    );
    let info = request(&mut session, "dataBreakpointInfo", json!({ "name": "0x9000" }));
    request(
        &mut session,
        "setDataBreakpoints",
        json!({ "breakpoints": [{ "dataId": info["body"]["dataId"], "accessType": "write" }] })
    );
    session
}

fn kinds(session: &mut Session<RecordingPeer>) -> Vec<Value> {
    let answer = request(session, "cpclib/breakpoints", json!({}));
    answer["body"]["kinds"].as_array().unwrap().clone()
}

fn kind<'k>(kinds: &'k [Value], id: &str) -> &'k Value {
    kinds.iter().find(|k| k["id"] == id).unwrap_or_else(|| panic!("no kind {id}"))
}

#[test]
fn every_kind_is_listed_and_explained() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    let kinds = kinds(&mut session);
    let ids: Vec<&str> = kinds.iter().map(|k| k["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["source", "directive", "data", "watch", "entry"]);
    for kind in &kinds {
        assert!(kind["title"].as_str().is_some_and(|t| !t.is_empty()), "{kind}");
        assert!(kind["description"].as_str().is_some_and(|d| d.len() > 60), "{kind}");
    }

    let source = &kind(&kinds, "source")["entries"][0];
    assert_eq!(source["label"], "main.asm:11");
    assert_eq!(source["detail"], "0x4003 if A = 5, from hit 3");
    assert_eq!((source["status"].as_str(), source["line"].as_u64()), (Some("armed"), Some(11)));
    assert_eq!(source["conditional"], true);

    let directive = &kind(&kinds, "directive")["entries"][0];
    assert_eq!(directive["label"], "ink");
    assert_eq!(directive["detail"], "0x8000 I/O w, if A = &55");
    assert_eq!(directive["status"], "armed", "applied whole: {directive}");

    let data = &kind(&kinds, "data")["entries"][0];
    assert_eq!(data["detail"], "0x9000 w");
    assert!(kind(&kinds, "data")["description"].as_str().unwrap().contains("stops on them"));

    assert_eq!(kind(&kinds, "entry")["entries"][0]["detail"], "0x4000 execution");
}

/// On an emulator that cannot apply them, the breakpoints say so.
#[test]
fn what_an_emulator_cannot_apply_is_marked() {
    let mut session = session(RecordingPeer::new());
    let kinds = kinds(&mut session);
    let source = &kind(&kinds, "source")["entries"][0];
    assert_eq!(source["status"], "partial", "{source}");
    assert!(source["note"].as_str().unwrap().contains("stops on every pass"));
    let directive = &kind(&kinds, "directive")["entries"][0];
    assert_eq!(directive["status"], "partial", "{directive}");
    assert!(directive["note"].as_str().unwrap().contains("I/O"), "{directive}");
    assert!(kind(&kinds, "data")["description"].as_str().unwrap().contains("cannot stop"));
    // The data breakpoint became a watch there.
    let watches = &kind(&kinds, "watch")["entries"];
    assert!(watches.as_array().unwrap().iter().any(|w| w["detail"] == "0x9000 w"), "{watches}");
}

/// A directive whose red dot was cleared is listed as not armed, and why.
#[test]
fn a_cleared_directive_is_inactive() {
    let mut session = session(RecordingPeer::new().also_supporting(BEMU));
    // ink.asm line 5 (0x8000) carries the directive: setting and clearing a
    // dot there takes it back.
    request(&mut session, "setBreakpoints", json!({ "source": { "path": "ink.asm" }, "breakpoints": [{ "line": 5 }] }));
    request(&mut session, "setBreakpoints", json!({ "source": { "path": "ink.asm" }, "breakpoints": [] }));
    let kinds = kinds(&mut session);
    let directive = &kind(&kinds, "directive")["entries"][0];
    assert_eq!(directive["status"], "inactive", "{directive}");
    assert!(directive["note"].as_str().unwrap().contains("red dot"));
}

/// The emulator's own list is asked of a peer that has one, and refused with
/// a reason by the others.
#[test]
fn the_emulator_list_is_asked_only_of_an_emulator_that_has_one() {
    let mut session = Session::new(RecordingPeer::new(), map());
    let answer = request(&mut session, "cpclib/emulatorBreakpoints", json!({}));
    assert_eq!(answer["success"], false);

    let mut session = Session::new(RecordingPeer::new().also_supporting(&["cpclib/emulatorBreakpoints"]), map());
    let request = json!({ "seq": 7, "type": "request", "command": "cpclib/emulatorBreakpoints", "arguments": {} });
    assert!(session.on_editor_message(&request).unwrap().is_empty(), "forwarded, answered by the peer");
    assert!(session.peer().last("cpclib/emulatorBreakpoints").is_some());
}

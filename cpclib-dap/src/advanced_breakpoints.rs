//! bemu's advanced breakpoints (`/api/breakpoints`, an extension to the
//! AMSpiriT-Lite API): what an editor asks for beyond an address.
//!
//! - source breakpoints with a condition, a hit count or a log message
//!   (execution breakpoints that stop, or for a log point only report);
//! - data breakpoints that stop on a read and/or write of an address;
//! - the program's and the launch configuration's watches (`TYPE=MEM`
//!   directives, `watchLabels`), reported without stopping.
//!
//! The emulator keeps one list, shared with the breakpoints a snapshot
//! brought: the ones armed here are named `cpclib:...`, and only those are
//! replaced when the editor changes its set.

use cpclib_runner::runner::emulator::amspiritlite_api::{Call, perform};
use serde_json::{Value, json};

/// The prefix of the names of the breakpoints this adapter armed.
const OURS: &str = "cpclib:";

/// What this session armed through `/api/breakpoints`, as the internal
/// requests carried it.
#[derive(Debug, Default, Clone)]
pub(crate) struct Advanced {
    /// `setInstructionBreakpoints` entries with a condition, hit count or
    /// log message (`instructionReference`, `condition`, `after`,
    /// `logMessage`).
    pub exec: Vec<Value>,
    /// `cpclib/setDataBreakpoints` entries (`label`, `address`, `access`,
    /// `condition`, `after`).
    pub data: Vec<Value>,
    /// `cpclib/setWatches` entries (`label`, `address`, `read`, `write`).
    pub watches: Vec<Value>
}

/// Whether a `setInstructionBreakpoints` entry needs more than `/api/z80_bp`.
pub(crate) fn is_advanced(entry: &Value) -> bool {
    ["condition", "after", "logMessage"].iter().any(|key| entry.get(*key).is_some())
}

fn exec_name(index: usize) -> String {
    format!("{OURS}exec:{index}")
}

fn data_name(label: &str) -> String {
    format!("{OURS}data:{label}")
}

fn watch_name(label: &str) -> String {
    format!("{OURS}watch:{label}")
}

impl Advanced {
    /// The bodies of `POST /api/breakpoints`, with what each one is for (in
    /// a refusal).
    fn requests(&self) -> Vec<(String, Value)> {
        let mut requests = Vec::new();
        for (index, entry) in self.exec.iter().enumerate() {
            let address = entry
                .get("instructionReference")
                .and_then(Value::as_str)
                .and_then(crate::protocol::parse_address_reference)
                .unwrap_or(0);
            let mut body = json!({
                "kind": "exec",
                "addr": address,
                // A log point reports and goes on.
                "run": if entry.get("logMessage").is_some() { "watch" } else { "stop" },
                "name": exec_name(index)
            });
            copy_options(entry, &mut body);
            requests.push((format!("breakpoint at 0x{address:04X}"), body));
        }
        for entry in &self.data {
            let label = entry.get("label").and_then(Value::as_str).unwrap_or_default();
            let mut body = json!({
                "kind": "mem",
                "access": entry.get("access").and_then(Value::as_str).unwrap_or("w"),
                "addr": entry.get("address").and_then(Value::as_u64).unwrap_or(0),
                "run": "stop",
                "name": data_name(label)
            });
            copy_options(entry, &mut body);
            requests.push((format!("data breakpoint on {label}"), body));
        }
        for entry in &self.watches {
            let label = entry.get("label").and_then(Value::as_str).unwrap_or_default();
            let flag = |key: &str| entry.get(key).and_then(Value::as_bool).unwrap_or(false);
            let access = match (flag("read"), flag("write")) {
                (true, true) => "rw",
                (true, false) => "r",
                _ => "w"
            };
            requests.push((
                format!("watch on {label}"),
                json!({
                    "kind": "mem",
                    "access": access,
                    "addr": entry.get("address").and_then(Value::as_u64).unwrap_or(0),
                    "run": "watch",
                    "name": watch_name(label)
                })
            ));
        }
        requests
    }

    /// Replace the breakpoints armed here by the current set. Returns the
    /// refusals (a condition bemu does not understand, say), described.
    pub(crate) fn push(&self, endpoint: &str) -> std::io::Result<Vec<String>> {
        // Ours out, highest index first: each removal renumbers the ones
        // after it.
        let listed = perform(endpoint, &Call::get("/api/breakpoints"))?;
        let listed: Value = serde_json::from_str(&listed).unwrap_or(Value::Null);
        let mut ours: Vec<u64> = listed
            .get("advanced")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter(|b| b.get("name").and_then(Value::as_str).is_some_and(|n| n.starts_with(OURS)))
                    .filter_map(|b| b.get("index").and_then(Value::as_u64))
                    .collect()
            })
            .unwrap_or_default();
        ours.sort_unstable_by(|a, b| b.cmp(a));
        for index in ours {
            perform(endpoint, &Call::delete("/api/breakpoints").query("index", index))?;
        }
        let mut refused = Vec::new();
        for (what, body) in self.requests() {
            let answer = perform(endpoint, &Call::post("/api/breakpoints").body(body.to_string()))?;
            let answer: Value = serde_json::from_str(&answer).unwrap_or(Value::Null);
            if let Some(error) = answer.get("error").and_then(Value::as_str) {
                refused.push(format!("bemu refused the {what}: {error}"));
            }
        }
        Ok(refused)
    }

    /// The Debug Console line for a `watch` event (a watch or a log point
    /// reporting), or `None` for a breakpoint this session did not arm.
    pub(crate) fn report(&self, event: &Value) -> Option<String> {
        let name = event.get("name").and_then(Value::as_str)?;
        if let Some(index) = name.strip_prefix(&format!("{OURS}exec:")) {
            let entry = self.exec.get(index.parse::<usize>().ok()?)?;
            let message = entry.get("logMessage").and_then(Value::as_str)?;
            return Some(format!("{}\n", interpolate(message, event)));
        }
        let label = name.strip_prefix(&format!("{OURS}watch:"))?;
        Some(format!("{label}: {} (PC {})\n", describe_access(event)?, hex16(event.get("pc"))))
    }

    /// The `stopped` reason and description for a stop on a breakpoint
    /// armed here: a data breakpoint, told as one, with the access.
    pub(crate) fn stop(&self, event: &Value) -> Option<(&'static str, String)> {
        let name = event.get("name").and_then(Value::as_str)?;
        let label = name.strip_prefix(&format!("{OURS}data:"))?;
        Some(("data breakpoint", format!("{label}: {}", describe_access(event)?)))
    }
}

fn copy_options(from: &Value, to: &mut Value) {
    for key in ["condition", "after"] {
        if let Some(value) = from.get(key) {
            to[key] = value.clone();
        }
    }
}

/// `write of #41 at #C000`.
fn describe_access(event: &Value) -> Option<String> {
    let access = event.get("access")?;
    let kind = access.get("type").and_then(Value::as_str).unwrap_or("access");
    let value = access.get("value").and_then(Value::as_u64).unwrap_or(0);
    Some(format!("{kind} of #{value:02X} at {}", hex16(access.get("addr"))))
}

/// A 16-bit value as bemu sends it (`"0x4002"`) or a number, as `#4002`.
fn hex16(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => {
            let digits = text.trim_start_matches("0x").trim_start_matches("0X");
            format!("#{}", digits.to_ascii_uppercase())
        },
        Some(Value::Number(number)) => format!("#{:04X}", number.as_u64().unwrap_or(0)),
        _ => "?".to_owned()
    }
}

/// A log message with its `{register}` parts replaced by the registers the
/// event carries (`{A}`, `{hl}`, `{PC}`); anything else in braces is kept.
fn interpolate(message: &str, event: &Value) -> String {
    let mut out = String::new();
    let mut rest = message;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}')
        else {
            rest = &rest[open..];
            break;
        };
        let name = &rest[open + 1..open + close];
        match event.get(name.trim().to_ascii_lowercase()) {
            Some(Value::Number(number)) => out.push_str(&format!("#{:02X}", number.as_u64().unwrap_or(0))),
            value @ Some(Value::String(_)) => out.push_str(&hex16(value)),
            _ => out.push_str(&rest[open..=open + close])
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> Advanced {
        Advanced {
            exec: vec![
                json!({ "instructionReference": "0x4002", "condition": "A == 3", "after": 1 }),
                json!({ "instructionReference": "0x4004", "logMessage": "A={A} HL={hl} at {PC} {nope}" }),
            ],
            data: vec![json!({ "label": "screen", "address": 0xC000, "access": "rw", "condition": "value == 2" })],
            watches: vec![json!({ "label": "count", "address": 0x8000, "read": false, "write": true })]
        }
    }

    #[test]
    fn each_kind_becomes_its_breakpoint() {
        let bodies: Vec<Value> = set().requests().into_iter().map(|(_, body)| body).collect();
        assert_eq!(bodies, vec![
            json!({ "kind": "exec", "addr": 0x4002, "run": "stop", "name": "cpclib:exec:0", "condition": "A == 3", "after": 1 }),
            json!({ "kind": "exec", "addr": 0x4004, "run": "watch", "name": "cpclib:exec:1" }),
            json!({ "kind": "mem", "access": "rw", "addr": 0xC000, "run": "stop", "name": "cpclib:data:screen", "condition": "value == 2" }),
            json!({ "kind": "mem", "access": "w", "addr": 0x8000, "run": "watch", "name": "cpclib:watch:count" }),
        ]);
    }

    #[test]
    fn plain_entries_stay_on_the_address_list() {
        assert!(!is_advanced(&json!({ "instructionReference": "0x4000" })));
        assert!(is_advanced(&json!({ "instructionReference": "0x4000", "after": 2 })));
    }

    #[test]
    fn reports_and_stops_are_described() {
        let set = set();
        let log = json!({ "name": "cpclib:exec:1", "a": 42, "hl": "0xc000", "pc": "0x4004" });
        assert_eq!(set.report(&log).unwrap(), "A=#2A HL=#C000 at #4004 {nope}\n");
        let access = json!({ "type": "write", "addr": "0x8000", "value": 7 });
        let watch = json!({ "name": "cpclib:watch:count", "pc": "0x4010", "access": access });
        assert_eq!(set.report(&watch).unwrap(), "count: write of #07 at #8000 (PC #4010)\n");
        assert_eq!(set.report(&json!({ "name": "screen" })), None, "not ours: a snapshot's");

        let stop = json!({ "name": "cpclib:data:screen", "access": { "type": "read", "addr": "0xc000", "value": 2 } });
        assert_eq!(set.stop(&stop).unwrap(), ("data breakpoint", "screen: read of #02 at #C000".to_owned()));
        assert_eq!(set.stop(&json!({ "name": "cpclib:exec:0" })), None, "a breakpoint stop");
    }
}

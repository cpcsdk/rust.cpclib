//! The breakpoint manager: breakpoints made by hand, as ACE's breakpoint
//! window makes them - execution, memory or I/O port, with masks, an access
//! direction, a value, a condition, a run mode (stop or watch), a pass count
//! and a name - rather than from a source line.
//!
//! Applied whole by an emulator that takes `cpclib/setManagerBreakpoints`
//! (bemu); elsewhere a plain execution one still becomes an address
//! breakpoint and the others are listed as not armed.

use serde_json::{Value, json};

/// Execution, memory or I/O port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerKind {
    Exec,
    Memory,
    Io
}

impl ManagerKind {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "exec" | "execution" => Some(Self::Exec),
            "mem" | "memory" => Some(Self::Memory),
            "io" | "port" | "ports" => Some(Self::Io),
            _ => None
        }
    }

    /// As bemu's `/api/breakpoints` names it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Exec => "exec",
            Self::Memory => "mem",
            Self::Io => "io"
        }
    }
}

/// One breakpoint of the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerBreakpoint {
    pub id: u32,
    pub kind: ManagerKind,
    pub address: u32,
    pub mask: u16,
    /// Bytes covered from `address` (memory).
    pub size: u16,
    /// `r`, `w` or `rw`.
    pub access: String,
    pub value: u8,
    pub value_mask: u8,
    /// In ACE's condition language.
    pub condition: Option<String>,
    /// Reported without stopping rather than stopping.
    pub watch: bool,
    /// Passes let through before it acts (ACE's "Pas").
    pub step: u32,
    pub name: Option<String>
}

impl ManagerBreakpoint {
    /// From the manager's form: `kind`, `address` (a number, or a label
    /// `resolve` knows), and optionally `mask`, `size`, `access`, `value`,
    /// `valmask`, `condition`, `mode` (stop|watch), `step`, `name`.
    pub fn from_form(form: &Value, id: u32, resolve: impl Fn(&str) -> Option<u32>) -> Result<Self, String> {
        let text = |key: &str| form.get(key).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty());
        let number = |key: &str, max: u64| -> Result<Option<u64>, String> {
            match form.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Number(n)) => n.as_u64().filter(|n| *n <= max).map(Some).ok_or_else(|| format!("{key} must be 0 to {max}")),
                Some(Value::String(t)) if t.trim().is_empty() => Ok(None),
                Some(Value::String(t)) => crate::session::parse_number(t.trim())
                    .map(u64::from)
                    .filter(|n| *n <= max)
                    .map(Some)
                    .ok_or_else(|| format!("{key}: \"{t}\" is not a number from 0 to {max} (&, # or 0x for hex)")),
                Some(_) => Err(format!("{key} must be a number"))
            }
        };
        let kind = text("kind").and_then(ManagerKind::parse).ok_or("kind must be exec, mem or io")?;
        let typed = text("address").ok_or("an address (or a label) is needed")?;
        let address = crate::session::parse_number(typed)
            .or_else(|| resolve(typed))
            .filter(|a| *a <= 0xffff)
            .ok_or_else(|| format!("\"{typed}\" is neither an address nor a label of the program"))?;
        let access = match text("access").unwrap_or("rw") {
            "r" | "read" => "r",
            "w" | "write" => "w",
            "rw" | "readwrite" | "both" => "rw",
            other => return Err(format!("access \"{other}\": r, w or rw"))
        };
        let watch = match text("mode").unwrap_or("stop") {
            "stop" => false,
            "watch" => true,
            other => return Err(format!("mode \"{other}\": stop or watch"))
        };
        Ok(Self {
            id,
            kind,
            address,
            mask: number("mask", 0xffff)?.unwrap_or(0xffff) as u16,
            size: number("size", 0xffff)?.unwrap_or(1).max(1) as u16,
            access: access.to_owned(),
            value: number("value", 0xff)?.unwrap_or(0) as u8,
            value_mask: number("valmask", 0xff)?.unwrap_or(0) as u8,
            condition: text("condition").map(str::to_owned),
            watch,
            step: number("step", u32::MAX as u64)?.unwrap_or(0) as u32,
            name: text("name").map(str::to_owned)
        })
    }

    /// Whether an emulator without advanced breakpoints can still arm it:
    /// an execution breakpoint that only stops on an address.
    pub fn is_plain_exec(&self) -> bool {
        self.kind == ManagerKind::Exec && self.condition.is_none() && !self.watch && self.step == 0 && self.mask == 0xffff && self.size == 1
    }

    /// What the peer posts to bemu's `/api/breakpoints`, with its `id` and
    /// `label` (see `advanced_breakpoints`).
    pub fn body(&self, label: &str) -> Value {
        let mut body = json!({
            "id": self.id,
            "label": label,
            "kind": self.kind.name(),
            "access": self.access,
            "run": if self.watch { "watch" } else { "stop" },
            "addr": self.address,
            "mask": self.mask,
            "size": self.size,
            "value": self.value,
            "valmask": self.value_mask
        });
        if let Some(condition) = &self.condition {
            body["condition"] = json!(condition);
        }
        if self.step > 0 {
            body["after"] = json!(self.step);
        }
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(name: &str) -> Option<u32> {
        (name == "BIOS_READ_SECTOR").then_some(0xc03c)
    }

    #[test]
    fn the_form_as_ace_fills_it() {
        let form = json!({ "kind": "exec", "address": "BIOS_READ_SECTOR", "condition": "BC=7 AND FlagC", "name": "TrackDone" });
        let brk = ManagerBreakpoint::from_form(&form, 1, resolve).unwrap();
        assert_eq!((brk.kind, brk.address, brk.watch, brk.step), (ManagerKind::Exec, 0xc03c, false, 0));
        assert_eq!(brk.body("TrackDone")["condition"], "BC=7 AND FlagC");
        assert!(!brk.is_plain_exec());

        let form = json!({ "kind": "io", "address": "&FB01", "mask": "&FF81", "access": "rw", "value": "&00", "valmask": "#00", "step": "2" });
        let brk = ManagerBreakpoint::from_form(&form, 2, resolve).unwrap();
        assert_eq!((brk.address, brk.mask, brk.access.as_str(), brk.step), (0xfb01, 0xff81, "rw", 2));
        assert_eq!(brk.body("&FB01")["after"], 2);

        let plain = ManagerBreakpoint::from_form(&json!({ "kind": "exec", "address": "0x0506" }), 3, resolve).unwrap();
        assert!(plain.is_plain_exec());
    }

    #[test]
    fn a_bad_form_says_what_is_wrong() {
        let error = |form: Value| ManagerBreakpoint::from_form(&form, 1, resolve).unwrap_err();
        assert!(error(json!({ "kind": "exec", "address": "nowhere" })).contains("label"));
        assert!(error(json!({ "kind": "rom", "address": "0" })).contains("kind"));
        assert!(error(json!({ "kind": "io", "address": "0", "mask": "&1FFFF" })).contains("mask"));
        assert!(error(json!({ "kind": "mem", "address": "0", "mode": "jump" })).contains("mode"));
    }
}

//! The types of bemu's debug API (`--debug-server`): what a request takes and a reply gives.
//!
//! Part of the cpclib workspace because both sides use it: bemu, which builds against cpclib,
//! serves it (and imports it as `bemu_api`), and `cpclib-dap` is a client of it.
//!
//! The server parses its requests into these types and builds its replies from them, and its
//! OpenAPI description (`GET /api/doc?format=openapi`) is generated from their JSON schemas: what
//! is documented is what is parsed. A client (a debugger) can use the same types.
//!
//! Conventions shared with AMSpiriT's API, which bemu serves:
//! - an integer is a JSON number or a string, decimal or hexadecimal with `0x` ([`Int`]);
//! - a JSON body's field may be nested at any depth (`{"a":1}` and `{"x":{"a":1}}` both carry
//!   `a`): the server lifts them before parsing;
//! - an error answers 400 with an [`ErrorReply`].

use std::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// What the server offers beyond the AMSpiriT API, listed in [`Ping::features`].
pub const FEATURES: &[&str] = &[
    // `/api/step?mode=over|out|frame`
    "step_modes",
    // `/api/run_to`
    "run_to",
    // `/api/z80_bp?add=1`, `DELETE /api/z80_bp`
    "z80_bp_edit",
    // a `reason` and the `cmd_seq` in the stop and pause events
    "stop_reason",
    // `POST /api/z80` takes `HL`, `BC2`...
    "register_pairs",
    // a raster breakpoint's `name`, and its `index` in its stop event
    "raster_names",
    // `GET /api/doc?format=openapi` generated from these types
    "openapi",
];

// ------------------------------------------------------------------ values

/// An integer, as a JSON number or as a string: decimal, or hexadecimal with `0x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Int(pub i64);

impl Int {
    /// `text` as the API reads an integer: decimal, or hexadecimal with `0x`.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(hex) => i64::from_str_radix(hex, 16).ok(),
            None => text.parse().ok(),
        }
        .map(Int)
    }

    /// The value, if it is within `min..=max`.
    pub fn within(self, min: i64, max: i64) -> Option<i64> {
        (min..=max).contains(&self.0).then_some(self.0)
    }
}

impl From<i64> for Int {
    fn from(value: i64) -> Self {
        Int(value)
    }
}

impl Serialize for Int {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.0)
    }
}

impl<'de> Deserialize<'de> for Int {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = Int;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an integer: a number, or a string (decimal, or hexadecimal with 0x)")
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Int, E> {
                Ok(Int(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Int, E> {
                i64::try_from(v).map(Int).map_err(|_| E::custom("integer too large"))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Int, E> {
                if v.fract() == 0.0 && v.abs() < 9.0e15 {
                    Ok(Int(v as i64))
                } else {
                    Err(E::invalid_type(serde::de::Unexpected::Float(v), &self))
                }
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Int, E> {
                Int::parse(v).ok_or_else(|| E::invalid_value(serde::de::Unexpected::Str(v), &self))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl JsonSchema for Int {
    fn schema_name() -> Cow<'static, str> {
        "Int".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "An integer: a JSON number, or a string, decimal or hexadecimal with 0x.",
            "oneOf": [
                { "type": "integer" },
                { "type": "string", "pattern": "^\\s*(0[xX][0-9a-fA-F]+|-?[0-9]+)\\s*$" }
            ]
        })
    }
}

// ------------------------------------------------------------------ replies shared by many

/// An error: 400 for a request the server refuses, 404 for an unknown path, 405 for a method a
/// path does not take, 503 when the emulation did not answer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorReply {
    /// What is wrong.
    pub error: String,
    /// The field or parameter at fault, when one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// A command accepted. Its effects happen as the emulation runs: the events they cause carry its
/// `cmd_seq`, and `GET /api/ping`'s `emu.applied_cmd_seq` reaches it once it is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommandAck {
    /// Always true.
    pub ok: bool,
    /// The command's number, counting from 1.
    pub cmd_seq: u64,
}

impl CommandAck {
    pub fn new(cmd_seq: u64) -> Self {
        CommandAck { ok: true, cmd_seq }
    }
}

// ------------------------------------------------------------------ GET /api/ping

/// `GET /api/ping`: the server is there, and the emulator's state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Ping {
    /// Always true.
    pub ok: bool,
    pub emu: EmuState,
    /// `bemu`: which emulator serves this AMSpiriT-compatible API.
    pub frontend: String,
    /// bemu's version.
    pub version: String,
    /// What it offers beyond the AMSpiriT API (see [`FEATURES`]); none from a bemu older than
    /// the list.
    #[serde(default)]
    pub features: Vec<String>,
}

/// The emulator, as `/api/ping` and `/api/state` describe it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EmuState {
    /// The model: 464, 664, 6128, 464plus, 6128plus, gx4000.
    pub cpc_model: String,
    /// The CRTC type, 0 to 4.
    pub crtc_type: u8,
    /// Whether the machine is paused.
    pub paused: bool,
    /// Frames emulated.
    pub frames: u64,
    /// Frames per second, measured over the last second.
    pub fps: f64,
    /// NOPs (microseconds) emulated.
    pub ticks: u64,
    /// The last command applied (see [`CommandAck::cmd_seq`]).
    pub applied_cmd_seq: u64,
    /// The number of memory changes made through the API.
    pub ram_apply_seq: u64,
    /// Whether text is being typed (`/api/keytype`).
    pub autotyping: bool,
    /// Characters left to type.
    pub autotype_remaining: u64,
}

// ------------------------------------------------------------------ /api/z80

/// `GET /api/z80`: the Z80's registers. The alternate set is `A2`..`L2`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Z80Registers {
    pub PC: u16,
    pub SP: u16,
    pub IX: u16,
    pub IY: u16,
    pub A: u8,
    pub F: u8,
    pub B: u8,
    pub C: u8,
    pub D: u8,
    pub E: u8,
    pub H: u8,
    pub L: u8,
    pub A2: u8,
    pub F2: u8,
    pub B2: u8,
    pub C2: u8,
    pub D2: u8,
    pub E2: u8,
    pub H2: u8,
    pub L2: u8,
    pub I: u8,
    pub R: u8,
    /// 0 or 1.
    pub IFF1: u8,
    /// 0 or 1.
    pub IFF2: u8,
    /// The interrupt mode, 0 to 2.
    pub IM: u8,
    /// NOPs emulated.
    pub ticks: u64,
}

/// `POST /api/z80`: the registers to set, by name; any left out are kept. All are checked
/// before any is changed. The pairs (`AF`..`HL`, `AF2`..`HL2`) set their two halves.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SetZ80 {
    /// 0 to 0xFFFF, like `SP`, `IX`, `IY` and the pairs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub PC: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub SP: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub IX: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub IY: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub AF: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub BC: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub DE: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub HL: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub AF2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub BC2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub DE2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub HL2: Option<Int>,
    /// 0 to 0xFF, like every 8-bit register.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub A: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub F: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub B: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub C: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub D: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub E: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub H: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub L: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub A2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub F2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub B2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub C2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub D2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub E2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub H2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub L2: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub I: Option<Int>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub R: Option<Int>,
    /// 0 or 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub IFF1: Option<Int>,
    /// 0 or 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub IFF2: Option<Int>,
    /// The interrupt mode, 0 to 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub IM: Option<Int>,
}

impl SetZ80 {
    /// Set the register `name` (as the API names it: `PC`, `HL`, `A2`, `IFF1`..., in any case)
    /// to `value`; false for a name that is none of them.
    pub fn set(&mut self, name: &str, value: impl Into<Int>) -> bool {
        let field = match name.to_ascii_uppercase().as_str() {
            "PC" => &mut self.PC,
            "SP" => &mut self.SP,
            "IX" => &mut self.IX,
            "IY" => &mut self.IY,
            "AF" => &mut self.AF,
            "BC" => &mut self.BC,
            "DE" => &mut self.DE,
            "HL" => &mut self.HL,
            "AF2" => &mut self.AF2,
            "BC2" => &mut self.BC2,
            "DE2" => &mut self.DE2,
            "HL2" => &mut self.HL2,
            "A" => &mut self.A,
            "F" => &mut self.F,
            "B" => &mut self.B,
            "C" => &mut self.C,
            "D" => &mut self.D,
            "E" => &mut self.E,
            "H" => &mut self.H,
            "L" => &mut self.L,
            "A2" => &mut self.A2,
            "F2" => &mut self.F2,
            "B2" => &mut self.B2,
            "C2" => &mut self.C2,
            "D2" => &mut self.D2,
            "E2" => &mut self.E2,
            "H2" => &mut self.H2,
            "L2" => &mut self.L2,
            "I" => &mut self.I,
            "R" => &mut self.R,
            "IFF1" => &mut self.IFF1,
            "IFF2" => &mut self.IFF2,
            "IM" => &mut self.IM,
            _ => return false,
        };
        *field = Some(value.into());
        true
    }
}

// ------------------------------------------------------------------ POST /api/step

/// How far `POST /api/step` runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StepMode {
    /// One instruction (a `z80_bp` event, reason `step`).
    #[default]
    In,
    /// The instruction, a `CALL`, `RST`, `DJNZ` or block loop run whole (reason `run_to`).
    Over,
    /// Until the routine returns: the stack above where it is (reason `step_out`).
    Out,
    /// To the end of the frame (a `pause` event).
    Frame,
}

impl StepMode {
    /// As the query names it.
    pub fn as_str(self) -> &'static str {
        match self {
            StepMode::In => "in",
            StepMode::Over => "over",
            StepMode::Out => "out",
            StepMode::Frame => "frame",
        }
    }
}

/// `POST /api/step`'s query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StepQuery {
    /// How far to run: `in` (default), `over`, `out` or `frame`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<StepMode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_int_is_a_number_or_a_string() {
        let parse = |json: &str| serde_json::from_str::<Int>(json).map(|i| i.0).ok();
        assert_eq!(parse("16384"), Some(16384));
        assert_eq!(parse(r#""0x4000""#), Some(0x4000));
        assert_eq!(parse(r#"" 12 ""#), Some(12));
        assert_eq!(parse("2.0"), Some(2));
        assert_eq!(parse("2.5"), None);
        assert_eq!(parse(r#""&4000""#), None);
        assert_eq!(serde_json::to_string(&Int(7)).unwrap(), "7");
    }

    #[test]
    fn a_step_mode_is_its_name() {
        let query: StepQuery = serde_json::from_str(r#"{"mode":"over"}"#).unwrap();
        assert_eq!(query.mode, Some(StepMode::Over));
        assert!(serde_json::from_str::<StepQuery>(r#"{"mode":"sideways"}"#).is_err());
    }

    #[test]
    fn a_register_is_set_by_its_name() {
        let mut set = SetZ80::default();
        assert!(set.set("hl2", 0x1234));
        assert!(set.set("IFF1", 1));
        assert!(!set.set("Q", 1));
        assert_eq!(serde_json::to_string(&set).unwrap(), r#"{"HL2":4660,"IFF1":1}"#);
    }

    #[test]
    fn a_step_mode_is_named_as_serde_names_it() {
        for mode in [StepMode::In, StepMode::Over, StepMode::Out, StepMode::Frame] {
            assert_eq!(serde_json::to_value(mode).unwrap(), mode.as_str());
        }
    }

    #[test]
    fn an_older_ping_has_no_features() {
        let ping: Ping = serde_json::from_str(
            r#"{"ok":true,"frontend":"bemu","version":"0.1.1","emu":{"cpc_model":"6128","crtc_type":0,
               "paused":true,"frames":0,"fps":0.0,"ticks":0,"applied_cmd_seq":0,"ram_apply_seq":0,
               "autotyping":false,"autotype_remaining":0}}"#,
        )
        .unwrap();
        assert!(ping.features.is_empty());
    }

    #[test]
    fn set_z80_takes_registers_by_name() {
        let set: SetZ80 = serde_json::from_str(r#"{"HL":"0x1234","A":7,"unknown":1}"#).unwrap();
        assert_eq!(set.HL, Some(Int(0x1234)));
        assert_eq!(set.A, Some(Int(7)));
        assert_eq!(set.PC, None);
    }
}

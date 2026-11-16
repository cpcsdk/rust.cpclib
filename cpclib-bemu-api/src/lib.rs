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
    // `/api/ram`'s `at` (RAM banks, ROMs, the cartridge, the ASIC's registers) and `paging`,
    // `/api/memmap`'s `paging`
    "memory_spaces",
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

// ------------------------------------------------------------------ more values

/// A yes or no in a query: `1` or `0` (also `true` or `false`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Flag(pub bool);

impl Serialize for Flag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(if self.0 { "1" } else { "0" })
    }
}

impl<'de> Deserialize<'de> for Flag {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = Flag;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("0 or 1")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Flag, E> {
                Ok(Flag(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Flag, E> {
                match v {
                    0 => Ok(Flag(false)),
                    1 => Ok(Flag(true)),
                    _ => Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &self)),
                }
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Flag, E> {
                match u64::try_from(v) {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => Err(E::invalid_value(serde::de::Unexpected::Signed(v), &self)),
                }
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Flag, E> {
                match v.trim() {
                    "1" | "true" => Ok(Flag(true)),
                    "0" | "false" => Ok(Flag(false)),
                    _ => Err(E::invalid_value(serde::de::Unexpected::Str(v), &self)),
                }
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl JsonSchema for Flag {
    fn schema_name() -> Cow<'static, str> {
        "Flag".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "Yes or no: 1 or 0 (true or false are taken too).",
            "type": "string",
            "enum": ["0", "1"]
        })
    }
}

/// A CPU address or a symbol (`label`, `label+n`): a number, or a string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddressOrSymbol(pub String);

impl Serialize for AddressOrSymbol {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AddressOrSymbol {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = AddressOrSymbol;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an address or a symbol")
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(AddressOrSymbol(v.to_string()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(AddressOrSymbol(v.to_string()))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(AddressOrSymbol(v.to_owned()))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl JsonSchema for AddressOrSymbol {
    fn schema_name() -> Cow<'static, str> {
        "AddressOrSymbol".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A CPU address (a number, or decimal or 0x hex text) or a symbol (label, label+n).",
            "oneOf": [{ "type": "integer" }, { "type": "string" }]
        })
    }
}

/// A name or a list of names (counters).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Names(pub Vec<String>);

impl Serialize for Names {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Names {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany {
            One(String),
            Many(Vec<String>),
        }
        match OneOrMany::deserialize(deserializer).map_err(|_| {
            serde::de::Error::custom("expected a counter name or a list of them")
        })? {
            OneOrMany::One(name) => Ok(Names(vec![name])),
            OneOrMany::Many(names) => Ok(Names(names)),
        }
    }
}

impl JsonSchema for Names {
    fn schema_name() -> Cow<'static, str> {
        "Names".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A name, or a list of names.",
            "oneOf": [{ "type": "string" }, { "type": "array", "items": { "type": "string" } }]
        })
    }
}

/// An enum named by a word, in any case, some with other spellings too.
macro_rules! named {
    (
        $(#[$meta:meta])*
        $name:ident: $what:literal {
            $($(#[$vmeta:meta])* $variant:ident = $canon:literal $(| $alias:literal)*),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant),+
        }

        impl $name {
            /// The names, as the API writes them.
            pub const NAMES: &'static [&'static str] = &[$($canon),+];

            /// As the API writes it.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $canon),+
                }
            }

            /// `text`, in any case, one of [`Self::NAMES`] or another spelling.
            pub fn parse(text: &str) -> Option<Self> {
                match text.trim().to_ascii_lowercase().as_str() {
                    $($canon $(| $alias)* => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                Self::parse(&text).ok_or_else(|| {
                    serde::de::Error::custom(format!(
                        "unknown {} \"{text}\": expected one of {}",
                        $what,
                        Self::NAMES.join(", ")
                    ))
                })
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                stringify!($name).into()
            }

            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({ "type": "string", "enum": Self::NAMES })
            }
        }
    };
}

// ------------------------------------------------------------------ commands, settings

/// A command accepted that changed memory or `PC`: `seq` counts those changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MemoryAck {
    /// Always true.
    pub ok: bool,
    /// The memory changes made through the API so far (`emu.ram_apply_seq`).
    pub seq: u64,
    /// The command's number.
    pub cmd_seq: u64,
}

/// `POST /api/exec`: set `PC`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SetPc {
    /// The new `PC` (required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Int>,
}

/// `GET /api/config`: the emulator's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    /// The CRTC type, 0 to 4.
    pub crtc_type: u8,
    pub paused: bool,
    /// Instructions kept for `/api/history`.
    pub history_size: u64,
}

/// `POST /api/config`: pause or resume, reset, size the history; at least one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SetConfig {
    /// Pause (true) or resume (false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    /// The reset button: RAM kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub do_soft_reset: Option<bool>,
    /// Power off and on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub do_hard_reset: Option<bool>,
    /// Instructions kept for `/api/history`, 1 to 100000; empties it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_size: Option<Int>,
}

/// `POST /api/keytype`: type text on the CPC keyboard, a key at a time.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KeyType {
    /// The characters (required); `\r` or `\n` is RETURN.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// `POST /api/run_to`: `addr`, or `line` (and `nop`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunTo {
    /// Where to run to: a CPU address or a symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<AddressOrSymbol>,
    /// In place of `addr`: the line of the frame, from the VSYNC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<Int>,
    /// With `line`: the NOP in it, 0 to 63 (default 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nop: Option<Int>,
}

// ------------------------------------------------------------------ memory spaces, pagings

/// A memory a [`Location`] is in, apart from the CPU's address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Space {
    /// A 16K RAM bank: 0-3 the main 64K (`MB0`-`MB3`), 4 and up the extension's 64K pages
    /// (`P0B0`-`P0B3` are 4-7, `P1B0` is 8...).
    Ram(u16),
    /// The lower ROM: the firmware (on a CPC+, the cartridge page RMR2 selects).
    Lower,
    /// An upper ROM slot (on a CPC+, the cartridge page its number selects; an empty slot reads
    /// as BASIC's, as on the CPC).
    Upper(u8),
    /// A page of a CPC+'s cartridge.
    Cartridge(u8),
    /// A CPC+'s ASIC register page (`&4000`-`&7FFF` when paged): offset 0 is `&4000`.
    Asic,
}

impl Space {
    /// As a [`Location`] writes it, before the `:` (`B04`, `LOWER`, `UPPER07`, `CART01`, `ASIC`).
    pub fn name(self) -> String {
        match self {
            Space::Ram(bank) => format!("B{bank:02X}"),
            Space::Lower => "LOWER".to_owned(),
            Space::Upper(slot) => format!("UPPER{slot:02X}"),
            Space::Cartridge(page) => format!("CART{page:02X}"),
            Space::Asic => "ASIC".to_owned(),
        }
    }

    /// `text`, as [`Self::name`] writes it, in any case; a RAM bank also by the name the window
    /// gives it (`MB0`-`MB3`, `P1B2`).
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_uppercase();
        let hex = |digits: &str| (!digits.is_empty()).then(|| u16::from_str_radix(digits, 16).ok()).flatten();
        if text == "LOWER" {
            return Some(Space::Lower);
        }
        if text == "ASIC" {
            return Some(Space::Asic);
        }
        if let Some(slot) = text.strip_prefix("UPPER") {
            return hex(slot).and_then(|slot| u8::try_from(slot).ok()).map(Space::Upper);
        }
        if let Some(page) = text.strip_prefix("CART") {
            return hex(page).and_then(|page| u8::try_from(page).ok()).map(Space::Cartridge);
        }
        if let Some(block) = text.strip_prefix("MB") {
            return block.parse::<u16>().ok().filter(|b| *b < 4).map(Space::Ram);
        }
        if let Some((page, block)) = text.strip_prefix('P').and_then(|rest| rest.split_once('B')) {
            let (page, block) = (page.parse::<u16>().ok()?, block.parse::<u16>().ok()?);
            return (block < 4).then(|| Space::Ram(4 + page * 4 + block));
        }
        text.strip_prefix('B').and_then(hex).map(Space::Ram)
    }

    /// Whether the CPU can write it (the RAM; not a ROM, the cartridge or the registers).
    pub fn writable(self) -> bool {
        matches!(self, Space::Ram(_))
    }
}

/// Where a byte is: a CPU address (through a [`Paging`]), or an offset in a [`Space`].
///
/// Written `0x4000` (or `&4000`, `#4000`, `16384`) for a CPU address, `SPACE:hhhh` (a hex
/// offset) for the others: `B04:0010` (RAM bank 4; also `P0B0:0010`), `LOWER:0038`,
/// `UPPER07:0000`, `CART01:0100`, `ASIC:0000`. The same names as the breakpoints' locations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Location {
    Cpu(u16),
    In {
        space: Space,
        /// In the space: 0-&3FFF, but for a RAM bank, where it may run on into the next banks.
        offset: u32,
    },
}

impl Location {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some((space, offset)) = text.split_once(':') {
            let space = Space::parse(space)?;
            let offset = parse_hex_address(offset)?;
            let limit = if matches!(space, Space::Ram(_)) { 0xFFFF } else { 0x3FFF };
            return (offset <= limit).then_some(Location::In { space, offset });
        }
        parse_address(text)
            .and_then(|address| u16::try_from(address).ok())
            .map(Location::Cpu)
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Location::Cpu(address) => write!(f, "0x{address:04X}"),
            Location::In { space, offset } => write!(f, "{}:{offset:04X}", space.name()),
        }
    }
}

/// A hex offset (`0010`, `0x10`, `&10`, `#10`).
fn parse_hex_address(text: &str) -> Option<u32> {
    let text = text.trim();
    let digits = ["0x", "0X", "&", "#", "$"]
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix))
        .unwrap_or(text);
    (!digits.is_empty()).then(|| u32::from_str_radix(digits, 16).ok()).flatten()
}

/// An address: decimal, or hex with `0x`, `&`, `#` or `$`.
fn parse_address(text: &str) -> Option<u32> {
    let text = text.trim();
    match ["0x", "0X", "&", "#", "$"].iter().find_map(|prefix| text.strip_prefix(prefix)) {
        Some(digits) => u32::from_str_radix(digits, 16).ok(),
        None => text.parse().ok(),
    }
}

macro_rules! text_type {
    ($name:ident, $what:literal, $schema:literal) => {
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                $name::parse(&text).ok_or_else(|| {
                    serde::de::Error::custom(format!(concat!("\"{}\" is not ", $what), text))
                })
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                stringify!($name).into()
            }

            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({ "type": "string", "description": $schema })
            }
        }
    };
}

text_type!(
    Location,
    "a location (0x4000, B04:0010, LOWER:0038, UPPER07:0000, CART01:0100, ASIC:0000)",
    "Where a byte is: a CPU address (0x4000, &4000, #4000 or decimal), read through a paging; or \
     SPACE:hhhh, a hex offset in a space: Bnn (RAM bank nn, hex; also MB0-MB3, PxBy), LOWER (the \
     lower ROM), UPPERnn (upper ROM slot nn, hex), CARTnn (CPC+ cartridge page nn, hex), ASIC (the \
     CPC+ register page, 0 is &4000)."
);

/// How the CPU's addresses are read: as the CPU reads them now, or through another paging,
/// ACE's way. Each part left out is as the machine has it now.
///
/// Written as comma-separated parts, in any case: `cpu` (or nothing): the CPU's paging now;
/// `C0`-`C7` the RAM configuration, `&C0`-`&FF` the Gate Array's value (the configuration and the
/// page), `Pn` the extension's 64K page (decimal); `L` or `-L` the lower ROM paged or not, `U` or
/// `-U` the upper ROM, `Un` upper ROM number n (paged, decimal); on a CPC+ `LPn` the lower ROM's
/// cartridge page, `LBn` the block it is in (0-2: &0000, &4000, &8000), `R` or `-R` the ASIC's
/// registers at &4000; `W` what a write reaches (the RAM, under the ROMs too). `C4,P1,-L`,
/// `U7`, `W`. The old `4` and `4:2` stand for `C4` and `C4,P2`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Paging {
    pub ram_config: Option<u8>,
    pub page: Option<u8>,
    pub lower_rom: Option<bool>,
    pub upper_rom: Option<bool>,
    pub upper_number: Option<u8>,
    pub lower_page: Option<u8>,
    pub lower_block: Option<u8>,
    pub registers: Option<bool>,
    /// What a write reaches, not what a read gives.
    pub writes: bool,
}

impl Paging {
    /// The CPU's paging now: nothing changed.
    pub fn is_cpu(&self) -> bool {
        *self == Paging::default()
    }

    /// Only the RAM's configuration and page are chosen (what an emulator that pages nothing
    /// else can follow).
    pub fn ram_only(&self) -> bool {
        Paging {
            ram_config: None,
            page: None,
            ..*self
        }
        .is_cpu()
    }

    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let mut paging = Paging::default();
        if text.is_empty() || text.eq_ignore_ascii_case("cpu") || text == "_" {
            return Some(paging);
        }
        // the debugger's old forms: a configuration, or configuration:page
        if let Some((config, page)) = text.split_once(':') {
            paging.ram_config = Some(config.parse::<u8>().ok().filter(|c| *c < 8)?);
            paging.page = Some(page.parse().ok()?);
            return Some(paging);
        }
        if let Ok(config) = text.parse::<u8>() {
            paging.ram_config = Some(config).filter(|c| *c < 8);
            return paging.ram_config.map(|_| paging);
        }
        for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let upper = part.to_ascii_uppercase();
            let number = |digits: &str| digits.parse::<u8>().ok();
            match upper.as_str() {
                "L" => paging.lower_rom = Some(true),
                "-L" => paging.lower_rom = Some(false),
                "U" => paging.upper_rom = Some(true),
                "-U" => paging.upper_rom = Some(false),
                "R" => paging.registers = Some(true),
                "-R" => paging.registers = Some(false),
                "W" => paging.writes = true,
                _ if upper.starts_with('&') || upper.starts_with("0X") => {
                    let value = u8::try_from(parse_address(&upper)?).ok().filter(|v| *v >= 0xC0)?;
                    paging.ram_config = Some(value & 7);
                    paging.page = Some((value >> 3) & 7);
                }
                _ if upper.starts_with("LP") => paging.lower_page = Some(number(&upper[2..]).filter(|p| *p < 8)?),
                _ if upper.starts_with("LB") => paging.lower_block = Some(number(&upper[2..]).filter(|b| *b < 3)?),
                _ if upper.starts_with('C') => paging.ram_config = Some(number(&upper[1..]).filter(|c| *c < 8)?),
                _ if upper.starts_with('P') => paging.page = Some(number(&upper[1..])?),
                _ if upper.starts_with('U') => {
                    paging.upper_rom = Some(true);
                    paging.upper_number = Some(number(&upper[1..])?);
                }
                _ => return None,
            }
        }
        Some(paging)
    }
}

impl std::fmt::Display for Paging {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(config) = self.ram_config {
            parts.push(format!("C{config}"));
        }
        if let Some(page) = self.page {
            parts.push(format!("P{page}"));
        }
        match self.lower_rom {
            Some(true) => parts.push("L".to_owned()),
            Some(false) => parts.push("-L".to_owned()),
            None => {}
        }
        if let Some(page) = self.lower_page {
            parts.push(format!("LP{page}"));
        }
        if let Some(block) = self.lower_block {
            parts.push(format!("LB{block}"));
        }
        match (self.upper_rom, self.upper_number) {
            (Some(true) | None, Some(number)) => parts.push(format!("U{number}")),
            (Some(true), None) => parts.push("U".to_owned()),
            // not paged, its number does not matter
            (Some(false), _) => parts.push("-U".to_owned()),
            (None, None) => {}
        }
        match self.registers {
            Some(true) => parts.push("R".to_owned()),
            Some(false) => parts.push("-R".to_owned()),
            None => {}
        }
        if self.writes {
            parts.push("W".to_owned());
        }
        if parts.is_empty() {
            f.write_str("cpu")
        } else {
            f.write_str(&parts.join(","))
        }
    }
}

text_type!(
    Paging,
    "a paging (cpu, or C0-C7, &C0-&FF, Pn, L, -L, U, -U, Un, LPn, LBn, R, -R, W, comma-separated)",
    "How the CPU's addresses are read, each part left out as the machine has it now: cpu (the \
     default) or comma-separated parts: C0-C7 (the RAM configuration), &C0-&FF (the Gate Array's \
     value: configuration and page), Pn (the extension's 64K page), L/-L (lower ROM paged or not), \
     U/-U (upper ROM), Un (upper ROM number n, paged); CPC+: LPn (the lower ROM's cartridge page), \
     LBn (its block, 0-2), R/-R (the ASIC registers at &4000); W (what a write reaches: the RAM, \
     under the ROMs too). E.g. C4,P1,-L or U7."
);

// ------------------------------------------------------------------ memory

named! {
    /// What `GET /api/ram` reads.
    RamView: "view" {
        /// The RAM banks, whatever is mapped.
        Raw = "raw",
        /// What the CPU reads now, ROMs included.
        Cpu = "cpu",
    }
}

/// `GET /api/ram`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReadRam {
    /// The first byte (default 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Int>,
    /// The number of bytes, 1 to 65536 (default 256).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len: Option<Int>,
    /// With `view=raw`: the 16K RAM bank `addr` is counted from (default 0); offsets carry into
    /// the next banks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank: Option<Int>,
    /// `raw` (default) or `cpu`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<RamView>,
    /// In place of `addr`, `bank` and `view`: where to read from, a CPU address (through
    /// `paging`) or an offset in a RAM bank, a ROM, a cartridge page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Location>,
    /// How a CPU address is read (default the CPU's paging now): with `at`, or `view=cpu`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paging: Option<Paging>,
}

/// `GET /api/ram`: the bytes, as hex.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Ram {
    pub addr: u32,
    pub len: u32,
    /// Two lower case hex digits a byte.
    pub hex: String,
    /// `cpu` when read as the CPU does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<RamView>,
    /// With `at`: where the bytes were read from, as `at` names it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Location>,
    /// The paging a CPU address was read through, when not the CPU's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paging: Option<Paging>,
}

/// `POST /api/ram`: write bytes to RAM, and/or jump.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WriteRam {
    /// Where to write: an offset in `bank` (required with `data`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Int>,
    /// The 16K RAM bank `addr` is counted from (default 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank: Option<Int>,
    /// The bytes, as hex (spaces and `:` ignored).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Set `PC` afterwards (default false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec: Option<bool>,
    /// The `PC` `exec` sets (default `addr`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<Int>,
    /// In place of `addr` and `bank`: where to write, a CPU address (the RAM a write there
    /// reaches, through `paging`) or an offset in a RAM bank. A ROM, a cartridge, the ASIC's
    /// registers are not written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Location>,
    /// How a CPU address `at` is written (default the CPU's paging now).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paging: Option<Paging>,
}

/// `GET /api/memmap`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MemMapQuery {
    /// The paging to describe (default the CPU's now).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paging: Option<Paging>,
}

/// `GET /api/memmap`: what each 16K region of the CPU maps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MemMap {
    pub regions: Vec<MemRegion>,
    /// The Gate Array's ROM and mode register.
    pub rmr: u8,
    /// The RAM configuration (`MMR & 7`).
    pub ram_mode: u8,
    /// The 64K page of the extension (`(MMR >> 3) & 7`).
    pub ram_page: u8,
    pub mmr: u8,
    /// The 16K RAM banks there are.
    pub ram_banks: u32,
}

/// One 16K region of the CPU's view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MemRegion {
    pub base: u16,
    /// `0000`, `4000`, `8000`, `C000`.
    pub name: String,
    /// A ROM is read here.
    pub rom: bool,
    /// The ROM read: 255 the lower ROM, else the upper ROM's slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rom_bank: Option<u16>,
    /// The RAM bank read, when it is RAM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ram_bank: Option<u16>,
    /// The RAM bank is the extension's (4 and above).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<bool>,
    /// The RAM bank written, ROM or not.
    pub write_bank: u16,
    /// What is read here, as a [`Location`]'s space names it (`B04`, `LOWER`, `UPPER07`,
    /// `CART01`, `ASIC`); none from a bemu older than it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// `GET /api/symbols`: one symbol.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Symbol {
    pub name: String,
    pub address: u16,
}

/// `GET /api/history`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryQuery {
    /// The last `n` only, 1 to 100000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<Int>,
}

/// One instruction run, in `GET /api/history` (oldest first).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryEntry {
    pub pc: u16,
    /// Its bytes as fetched.
    pub hex: String,
    /// Its bytes as in memory now.
    pub current_hex: String,
    /// The symbol it is at (`label` or `label+n`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

// ------------------------------------------------------------------ breakpoints

/// `POST /api/z80_bp`'s and `DELETE /api/z80_bp`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExecBreakpointsQuery {
    /// `POST`: 1 adds these to the others (default 0: replaces them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Flag>,
}

/// An execution breakpoint as resolved: the entry given and where it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResolvedBreakpoint {
    pub entry: String,
    /// `Bnn:hhhh` (RAM bank, offset), `LOWER:hhhh` or `UPPERnn:hhhh`.
    pub location: String,
}

/// `POST /api/z80_bp`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExecBreakpointsSet {
    pub ok: bool,
    pub cmd_seq: u64,
    pub resolved: Vec<ResolvedBreakpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// `DELETE /api/z80_bp`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExecBreakpointsRemoved {
    pub ok: bool,
    pub cmd_seq: u64,
    pub removed: Vec<ResolvedBreakpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

named! {
    /// What an advanced breakpoint watches.
    BreakpointKind: "kind" {
        /// An instruction executed.
        Exec = "exec",
        /// A memory read or write.
        Mem = "mem" | "memory",
        /// An I/O port read or write.
        Io = "io",
    }
}

named! {
    /// The accesses an advanced breakpoint watches.
    BreakpointAccess: "access" {
        Read = "r" | "read",
        Write = "w" | "write",
        ReadWrite = "rw",
    }
}

named! {
    /// What an advanced breakpoint does on a hit.
    BreakpointRun: "run" {
        /// Stop (a `z80_bp` event).
        Stop = "stop",
        /// Report it (a `watch` event), not stopping.
        Watch = "watch",
        /// Only act on its counters.
        Count = "count",
    }
}

named! {
    /// What a breakpoint does to a counter, or a counter command.
    CounterAction: "action" {
        Create = "create",
        Reset = "reset",
        Start = "start",
        Stop = "stop",
    }
}

/// `POST /api/breakpoints`: an advanced breakpoint, by its fields or as the text of a REMU
/// `acebreak` entry (`remu`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NewBreakpoint {
    /// The breakpoint as the text of a REMU `acebreak` entry, in place of the fields below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remu: Option<String>,
    /// `exec` (default), `mem` or `io`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<BreakpointKind>,
    /// `r`, `w` or `rw` (default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<BreakpointAccess>,
    /// `stop` (default), `watch` or `count`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<BreakpointRun>,
    /// The address or port (required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Int>,
    /// The bits of the address compared (default 0xFFFF).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<Int>,
    /// The bytes or ports covered from `addr` (default 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Int>,
    /// The byte read or written to match, under `valmask`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Int>,
    /// The bits of `value` compared (default 0: any value).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valmask: Option<Int>,
    /// Shown in its events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// ACE's condition language (`A = &12 AND NOT FlagZ`, `LineCount > 100`, `CrtcR12 = &30`...)
    /// or C operators (`A == #12 && peek(HL) <> 0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// The counters started on a hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<Names>,
    /// The counters stopped on a hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<Names>,
    /// The counters reset on a hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<Names>,
    /// The hits ignored first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Int>,
}

/// `POST /api/breakpoints`: the breakpoint added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BreakpointAdded {
    pub ok: bool,
    /// Its index in `GET /api/breakpoints`'s `advanced`.
    pub index: u64,
    pub cmd_seq: u64,
}

/// `DELETE /api/breakpoints`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RemoveBreakpoint {
    /// The one to remove (the others are renumbered); none: all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Int>,
}

/// `GET /api/breakpoints`: the execution and advanced breakpoints.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Breakpoints {
    /// The execution breakpoints' locations (`Bnn:hhhh`...).
    pub z80_bp: Vec<String>,
    pub advanced: Vec<AdvancedBreakpoint>,
}

/// An advanced breakpoint, as armed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AdvancedBreakpoint {
    pub index: u64,
    pub kind: BreakpointKind,
    pub access: BreakpointAccess,
    pub run: BreakpointRun,
    pub addr: u16,
    pub mask: u16,
    pub size: u16,
    pub value: u8,
    pub valmask: u8,
    pub name: Option<String>,
    /// Its condition's source.
    pub condition: Option<String>,
    pub counters: Vec<CounterLink>,
    /// The hits ignored first.
    pub after: u64,
    /// The hits so far.
    pub hits: u64,
}

/// What a breakpoint does to a counter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CounterLink {
    pub action: CounterAction,
    pub name: String,
}

/// `POST /api/raster_bp`'s query: a position, `x`/`y`, `line`/`nop` or `frame_nop`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SetRasterBreakpoint {
    /// Beam ticks (1/16 µs) in the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Int>,
    /// The raster line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Int>,
    /// 1: `x`/`y` relative to the visible picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Flag>,
    /// The NOP in the line, 0 to 63; alone, in every line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nop: Option<Int>,
    /// The line of the frame, from the VSYNC; alone, at its start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<Int>,
    /// The NOP of the frame, 0 to 19967.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_nop: Option<Int>,
    /// 1: keep the others (default 0: replace them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Flag>,
    /// 0: clear them all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable: Option<Flag>,
    /// Shown in its stop event, with its index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A beam breakpoint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RasterBreakpoint {
    pub index: u64,
    /// Beam ticks in the line.
    pub x: u64,
    /// The NOP in the line.
    pub nop: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The line (none: every line).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_nop: Option<u64>,
}

/// `GET /api/raster_bp`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RasterBreakpoints {
    pub breakpoints: Vec<RasterBreakpoint>,
}

/// `POST /api/raster_bp`: the beam breakpoints now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RasterBreakpointsSet {
    pub ok: bool,
    pub cmd_seq: u64,
    pub breakpoints: Vec<RasterBreakpoint>,
}

// ------------------------------------------------------------------ counters

/// A counter of the NOPs run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Counter {
    pub name: String,
    pub nops: u64,
    pub running: bool,
}

/// `GET /api/counters`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Counters {
    pub counters: Vec<Counter>,
}

/// `POST /api/counters` and `DELETE /api/counters`: the counters now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CountersChanged {
    pub ok: bool,
    pub counters: Vec<Counter>,
}

/// `POST /api/counters`: create, reset, start or stop a counter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CounterCommand {
    /// The counter (required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `create` (default), `reset`, `start` or `stop`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<CounterAction>,
    /// On `create`: start counting now (default true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,
}

/// `DELETE /api/counters`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RemoveCounter {
    /// The one to delete; none: all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

// ------------------------------------------------------------------ the chips, the beam

/// `GET /api/state`: everything at once.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct State {
    pub z80: Z80Registers,
    pub ga: GateArray,
    pub crtc: Crtc,
    pub psg: Psg,
    pub fdc: Fdc,
    /// Null without a monitor.
    pub beam: Option<BeamState>,
    pub emu: EmuState,
}

/// `GET /api/crtc`: the registers, the selected one and the counters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Crtc {
    /// The CRTC type, 0 to 4.
    #[serde(rename = "type")]
    pub crtc_type: u8,
    /// R0 to R13, as read back.
    pub regs: Vec<u8>,
    pub selected_reg: u8,
    /// The horizontal counter (C0).
    pub hcc: u8,
    /// The character row counter (C4).
    pub vcc: u8,
    /// The raster counter (C9).
    pub vlc: u8,
    /// The VSYNC counter.
    pub vsc: u8,
    /// The vertical total adjust counter (R5).
    pub vtac: u8,
    /// The memory address output.
    pub ma: u16,
    /// The beam's line, with a monitor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rasterline: Option<u64>,
}

/// `GET /api/ga`: the Gate Array.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GateArray {
    /// The screen mode, 0 to 3.
    pub mode: u8,
    /// The pen selected.
    pub pen: u8,
    /// The border's hardware colour number.
    pub border_idx: u8,
    /// The border as 0xRRGGBB.
    pub border_rgb: u32,
    /// The 16 inks' hardware colour numbers.
    pub ink_idx: Vec<u8>,
    /// The 16 inks as 0xRRGGBB.
    pub ink_rgb: Vec<u32>,
    pub rmr: u8,
    pub ram_mode: u8,
    pub ram_page: u8,
    /// The 17 inks' (border last) firmware numbers.
    pub ink_firmware: Vec<u8>,
    pub lower_rom: bool,
    pub upper_rom: bool,
    /// The interrupt counter.
    pub r52: u8,
}

/// `GET /api/psg`: the PSG, decoded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Psg {
    pub period_a: u16,
    pub period_b: u16,
    pub period_c: u16,
    pub vol_a: u8,
    pub vol_b: u8,
    pub vol_c: u8,
    pub mixer: u8,
    pub noise: u8,
    pub env_period: u16,
    pub env_shape: u8,
    /// R0 to R15 as they are.
    pub regs: Vec<u8>,
    /// The register selected, if any.
    pub selected_reg: Option<u8>,
}

/// `GET /api/fdc`: the FDC.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Fdc {
    /// The main status register.
    pub msr: u8,
    pub sr0: u8,
    pub sr1: u8,
    pub sr2: u8,
    pub motor: bool,
    pub phase: String,
    pub command: String,
    pub drives: Vec<Drive>,
}

/// One drive of the FDC.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Drive {
    pub connected: bool,
    /// `3` (one head: the side read is the side up) or `3.5` (double sided).
    pub kind: String,
    /// A disc is in.
    pub disk: bool,
    /// The side up, `A` or `B`.
    pub side: String,
    pub track: u8,
}

/// `POST /api/drive`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DriveCommand {
    /// 0 (A, default) or 1 (B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive: Option<Int>,
    /// `3`, `3.5` or `gotek`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// 1 turns the disc of a 3" drive over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flip: Option<Flag>,
}

/// The beam, in the 1024x350 raster and in NOPs; where the visible picture is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BeamState {
    /// Ticks of 1/16 µs in the line.
    pub x: i64,
    /// The line.
    pub y: i64,
    /// The NOP in the line.
    pub nop: u64,
    pub line: u64,
    /// The NOP of the frame.
    pub frame_nop: u64,
    /// The visible picture in the raster: its origin and size.
    pub cx: i64,
    pub cy: i64,
    pub cw: u64,
    pub ch: u64,
}

/// `GET /api/beam`: [`BeamState`], and the frame's size in NOPs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BeamPosition {
    #[serde(flatten)]
    pub beam: BeamState,
    pub nops_per_line: u64,
    pub nops_per_frame: u64,
}

/// `GET /api/beam`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BeamQuery {
    /// 1: `x` and `y` relative to the visible picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Flag>,
}

// ------------------------------------------------------------------ media

named! {
    /// What a loaded snapshot does to the CRTC.
    MediaCrtc: "crtc" {
        Keep = "keep",
        Sna = "sna",
    }
}

/// `POST /api/media`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MediaQuery {
    /// The file's name: needed for a `.bin`/`.amsdos` (an AMSDOS header is used when there is
    /// one; else `name@LOAD[@ENTRY].bin`), an archive's (`games.zip#side b.dsk`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The drive for a disc: 0 (A, default) or 1 (B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive: Option<Int>,
    /// `keep` or `sna` (accepted; the CRTC is kept).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crtc: Option<MediaCrtc>,
}

/// `POST /api/media`: what was loaded.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MediaLoaded {
    pub ok: bool,
    /// `sna`, `dsk`, `cdt` or `bin`.
    pub media: String,
    pub cmd_seq: u64,
    /// A snapshot: the breakpoints it set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breakpoints: Option<u64>,
    /// A snapshot: the symbols it brought.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbols: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
    /// A binary: where it was loaded, its length and its entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<u16>,
}

/// `GET /api/screenshot`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ScreenshotQuery {
    /// 1: the visible 768x272 picture (default); 0: the whole 1024x350 raster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Flag>,
    /// 1: the frame being drawn (default while paused); 0: the last complete one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<Flag>,
    /// With `live=1`: 1 the drawn part alone (default), 0 drawn over the previous frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full: Option<Flag>,
}

// ------------------------------------------------------------------ BASIC

/// `POST /api/basic`'s query; the body is the program, as typed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicLoad {
    /// 1: reset first; loaded once the firmware has booted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<Flag>,
    /// 1: RUN it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<Flag>,
}

/// `GET /api/basic_state`: where the program and its variables are, what is run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicState {
    /// 10 (1.0) or 11 (1.1).
    pub basic_ver: u8,
    pub prog_start: u16,
    pub txttop: u16,
    pub vartop: u16,
    pub arrend: u16,
    pub prog_size: u16,
    pub var_size: u16,
    pub chain_heads_addr: u16,
    /// The line run, 0xFFFF in direct mode.
    pub cur_linenum: u16,
    /// The statement run (an `addr` of `/api/basic_listing`).
    pub stmt_addr: u16,
}

/// `GET /api/basic_listing`: the program as lines of statements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicListing {
    pub lines: Vec<BasicLine>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicLine {
    /// Its length field.
    pub addr: u16,
    pub num: u16,
    pub stmts: Vec<BasicStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicStatement {
    /// The byte before it, as `stmt_addr` names it while it runs.
    pub addr: u16,
    /// The `:` or end of line after it.
    pub end: u16,
    /// Preceded by a `:`.
    pub colon: bool,
    pub text: String,
    /// The variables it names, as written.
    pub vars: Vec<String>,
}

named! {
    /// How far `POST /api/basic_step` runs.
    BasicStepMode: "mode" {
        /// The next statement.
        Statement = "statement" | "stmt",
        /// The next line.
        Line = "line",
    }
}

/// `POST /api/basic_step`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicStep {
    /// `statement` (default) or `line`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<BasicStepMode>,
}

/// `GET /api/basic_bp`: the BASIC line breakpoints.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicBreakpoints {
    pub lines: Vec<u16>,
}

/// `POST /api/basic_bp`: the BASIC line breakpoints now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicBreakpointsSet {
    pub ok: bool,
    pub lines: Vec<u16>,
    pub cmd_seq: u64,
}

/// `POST /api/basic_runto`'s query: `line` or `addr`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicRunTo {
    /// The line number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<Int>,
    /// A statement's address, as `/api/basic_listing` gives them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Int>,
}

// ------------------------------------------------------------------ the doc, the events

named! {
    /// What `GET /api/doc` answers.
    DocFormat: "format" {
        /// The endpoints, or one.
        Json = "json",
        /// The whole API as OpenAPI 3.1.
        Openapi = "openapi",
        /// A page to read.
        Html = "html",
    }
}

/// `GET /api/doc`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocQuery {
    /// One endpoint in detail: its path, with or without `/api/` (`ram`, `/api/ram`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// `json` (default), `openapi` or `html`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<DocFormat>,
}

/// `GET /api/events`'s query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventsQuery {
    /// Comma-separated event names to receive (default all).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topics: Option<String>,
}

/// The registers an event carries (`pc`, `sp`, `ix`, `iy` as `0xHHHH`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventRegisters {
    pub pc: String,
    pub sp: String,
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub ix: String,
    pub iy: String,
}

/// The beam where an event happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventBeam {
    pub x: u64,
    pub y: u64,
    pub nop: u64,
    pub line: u64,
    pub frame_nop: u64,
}

/// The memory or I/O access a breakpoint was hit by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventAccess {
    /// `read` or `write`.
    #[serde(rename = "type")]
    pub access_type: String,
    /// `0xHHHH`.
    pub addr: String,
    pub value: u8,
}

named! {
    /// Why the machine stopped.
    StopReason: "reason" {
        /// An execution, memory or I/O breakpoint.
        Breakpoint = "breakpoint",
        Raster = "raster",
        Step = "step",
        StepOut = "step_out",
        RunTo = "run_to",
        RunToBeam = "run_to_beam",
    }
}

/// The `z80_bp` event: the machine stopped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StopEvent {
    #[serde(flatten)]
    pub registers: EventRegisters,
    pub reason: StopReason,
    /// What stopped it: `exec` (an execution breakpoint), `mem`, `io` (an advanced one),
    /// `raster`, `step`, `step_out`, `run_to`, `run_to_beam`.
    pub kind: String,
    /// The `cmd_seq` of the last command: the one that let the machine run.
    pub cmd_seq: u64,
    /// NOPs emulated.
    pub ticks: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beam: Option<EventBeam>,
    /// The breakpoint's index (advanced or raster).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// An advanced breakpoint's hits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<EventAccess>,
    /// The counters' NOPs, by name, when there are counters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counters: Option<std::collections::BTreeMap<String, u64>>,
}

/// The `watch` event: a `watch` breakpoint was hit, the machine running on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WatchEvent {
    #[serde(flatten)]
    pub registers: EventRegisters,
    /// `exec`, `mem` or `io`.
    pub kind: String,
    pub index: u64,
    pub hits: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<EventAccess>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counters: Option<std::collections::BTreeMap<String, u64>>,
}

/// The `frame` event, every 10 frames while running.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FrameEvent {
    #[serde(flatten)]
    pub registers: EventRegisters,
    pub fps: f64,
    pub frame: u64,
    pub paused: bool,
}

/// The `pause` event: the machine paused or resumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PauseEvent {
    pub paused: bool,
    /// The `cmd_seq` of the last command.
    pub cmd_seq: u64,
}

/// The `reset` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResetEvent {
    /// Power off and on (else the reset button).
    pub hard: bool,
}

named! {
    /// Why a BASIC program stopped.
    BasicStopReason: "reason" {
        Breakpoint = "breakpoint",
        Step = "step",
        RunTo = "runto",
    }
}

/// The `basic_bp` event: a BASIC program stopped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicStopEvent {
    pub line: u16,
    /// The statement, `0xHHHH`.
    pub addr: String,
    pub reason: BasicStopReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counters: Option<std::collections::BTreeMap<String, u64>>,
}

/// The `basic_error` event: a program asked for before the machine was ready failed to load.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BasicErrorEvent {
    pub error: String,
}

/// The events of `GET /api/events`, by name, with the type of their data.
pub const EVENTS: &[(&str, &str)] = &[
    ("z80_bp", "StopEvent"),
    ("watch", "WatchEvent"),
    ("frame", "FrameEvent"),
    ("pause", "PauseEvent"),
    ("reset", "ResetEvent"),
    ("basic_bp", "BasicStopEvent"),
    ("basic_error", "BasicErrorEvent"),
];

/// The events' JSON schemas, by type name (see [`EVENTS`]).
pub fn event_schemas(generator: &mut SchemaGenerator) -> Vec<(&'static str, Schema)> {
    vec![
        ("StopEvent", generator.subschema_for::<StopEvent>()),
        ("WatchEvent", generator.subschema_for::<WatchEvent>()),
        ("FrameEvent", generator.subschema_for::<FrameEvent>()),
        ("PauseEvent", generator.subschema_for::<PauseEvent>()),
        ("ResetEvent", generator.subschema_for::<ResetEvent>()),
        ("BasicStopEvent", generator.subschema_for::<BasicStopEvent>()),
        ("BasicErrorEvent", generator.subschema_for::<BasicErrorEvent>()),
    ]
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
    fn a_flag_is_0_or_1() {
        let parse = |json: &str| serde_json::from_str::<Flag>(json).map(|f| f.0).ok();
        assert_eq!(parse(r#""1""#), Some(true));
        assert_eq!(parse(r#""0""#), Some(false));
        assert_eq!(parse("true"), Some(true));
        assert_eq!(parse(r#""2""#), None);
        let negative = serde_json::from_str::<Flag>("-1").unwrap_err().to_string();
        assert!(negative.contains("-1"), "{negative}");
    }

    #[test]
    fn a_name_takes_any_case_and_its_other_spellings() {
        assert_eq!(BreakpointKind::parse("MEMORY"), Some(BreakpointKind::Mem));
        assert_eq!(BreakpointAccess::parse(" Read "), Some(BreakpointAccess::Read));
        let refused = serde_json::from_str::<BreakpointKind>(r#""nope""#).unwrap_err().to_string();
        assert!(refused.contains("expected one of exec, mem, io"), "{refused}");
        assert_eq!(serde_json::to_string(&BreakpointKind::Mem).unwrap(), r#""mem""#);
    }

    #[test]
    fn every_event_has_its_schema() {
        let mut generator = SchemaGenerator::default();
        let schemas: Vec<&str> = event_schemas(&mut generator).into_iter().map(|(name, _)| name).collect();
        for (event, schema) in EVENTS {
            assert!(schemas.contains(schema), "{event}: {schema}");
        }
        assert_eq!(schemas.len(), EVENTS.len());
    }

    #[test]
    fn a_location_is_a_cpu_address_or_an_offset_in_a_space() {
        let parse = |text: &str| Location::parse(text);
        assert_eq!(parse("0x4000"), Some(Location::Cpu(0x4000)));
        assert_eq!(parse("&C000"), Some(Location::Cpu(0xC000)));
        assert_eq!(parse("16384"), Some(Location::Cpu(16384)));
        assert_eq!(parse("B04:0010"), Some(Location::In { space: Space::Ram(4), offset: 0x10 }));
        assert_eq!(parse("p1b2:0"), Some(Location::In { space: Space::Ram(10), offset: 0 }));
        assert_eq!(parse("MB3:3fff"), Some(Location::In { space: Space::Ram(3), offset: 0x3FFF }));
        assert_eq!(parse("upper07:C0"), Some(Location::In { space: Space::Upper(7), offset: 0xC0 }));
        assert_eq!(parse("CART1F:0100"), Some(Location::In { space: Space::Cartridge(0x1F), offset: 0x100 }));
        assert_eq!(parse("LOWER:0038"), Some(Location::In { space: Space::Lower, offset: 0x38 }));
        assert_eq!(parse("ASIC:0000"), Some(Location::In { space: Space::Asic, offset: 0 }));
        // a RAM bank's offset runs on into the next banks; a ROM's does not
        assert!(parse("B00:8000").is_some());
        assert_eq!(parse("UPPER07:4000"), None);
        for nonsense in ["", "X1:0", "B:0", "UPPER:0", "0x10000", "B04:zz"] {
            assert_eq!(parse(nonsense), None, "{nonsense}");
        }
        for text in ["0x4000", "B04:0010", "LOWER:0038", "UPPER07:0000", "CART01:0100", "ASIC:0000"] {
            assert_eq!(parse(text).unwrap().to_string(), text);
        }
        assert!(Space::Ram(0).writable() && !Space::Upper(0).writable());
    }

    #[test]
    fn a_paging_is_written_in_parts() {
        let parse = |text: &str| Paging::parse(text);
        assert!(parse("cpu").unwrap().is_cpu() && parse("").unwrap().is_cpu() && parse("_").unwrap().is_cpu());
        let paging = parse("C4,P1,-L,U7,W").unwrap();
        assert_eq!(
            paging,
            Paging {
                ram_config: Some(4),
                page: Some(1),
                lower_rom: Some(false),
                upper_rom: Some(true),
                upper_number: Some(7),
                writes: true,
                ..Paging::default()
            }
        );
        assert_eq!(paging.to_string(), "C4,P1,-L,U7,W");
        // the Gate Array's value: the configuration and the page
        assert_eq!(parse("&CA").unwrap(), Paging { ram_config: Some(2), page: Some(1), ..Paging::default() });
        assert_eq!(parse("lp2,lb1,r").unwrap().to_string(), "LP2,LB1,R");
        // the debugger's old forms
        assert_eq!(parse("4").unwrap(), Paging { ram_config: Some(4), ..Paging::default() });
        assert_eq!(parse("4:2").unwrap().to_string(), "C4,P2");
        assert!(parse("C4").unwrap().ram_only() && !parse("C4,-L").unwrap().ram_only());
        for nonsense in ["C8", "&BF", "LB3", "LP8", "X", "9", "4:x", "U256"] {
            assert_eq!(parse(nonsense), None, "{nonsense}");
        }
        assert_eq!(serde_json::to_string(&parse("C4,U7").unwrap()).unwrap(), r#""C4,U7""#);
        let refused = serde_json::from_str::<Paging>(r#""C9""#).unwrap_err().to_string();
        assert!(refused.contains("is not a paging"), "{refused}");
    }

    #[test]
    fn set_z80_takes_registers_by_name() {
        let set: SetZ80 = serde_json::from_str(r#"{"HL":"0x1234","A":7,"unknown":1}"#).unwrap();
        assert_eq!(set.HL, Some(Int(0x1234)));
        assert_eq!(set.A, Some(Int(7)));
        assert_eq!(set.PC, None);
    }
}

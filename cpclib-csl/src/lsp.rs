//! Instruction/argument metadata for editor tooling (`cpclib-lsp`'s CSL
//! completion) - kept here, next to the parser it describes, so
//! keyword/argument suggestions can never drift from what `csl_parser`
//! actually accepts. Mirrors the same "single source of truth in the
//! origin crate" convention `cpclib-asm`'s and `cpclib-bndbuild`'s own
//! `lsp` modules already follow.

use crate::csl::{CpcModel, CrtcModel, CslVersion, GateArrayModel};

/// The shape of arguments one CSL instruction keyword expects - one
/// variant per distinct case `csl_parser::parse_instruction`'s per-keyword
/// parsers actually implement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgShape {
    /// No arguments at all (`tape_play`/`tape_stop`/`tape_rewind`,
    /// `wait_vsyncoffon`, `wait_ssm0000`).
    None,
    /// `<major>.<minor>` (`csl_version`).
    Version,
    /// Optional `soft|S|hard|H`, defaulting to hard (`reset`).
    ResetType,
    /// One of `CrtcModel`'s tokens, required (`crtc_select`).
    CrtcModel,
    /// One of `GateArrayModel`'s tokens, required (`gate_array`, v1.1).
    GateArrayModel,
    /// One of `CpcModel`'s tokens, required (`cpc_model`, v1.1).
    CpcModel,
    /// One of `MemoryExpansion`'s tokens, required (`memory_exp`, v1.1).
    MemoryExpansion,
    /// `<U|L|C|M> <0-255> '<path>'` (`rom_config`, v1.1).
    RomConfig,
    /// A single quoted path - every `*_dir`/`*_load`/`*_insert`
    /// (tape/snapshot/screenshot/rom_dir/key_from_file/csl_load)
    /// instruction that takes nothing else.
    QuotedPath,
    /// Optional `A|B` drive, then a quoted path (`disk_insert`).
    DiskInsert,
    /// One required number, plus up to two further optional ones
    /// (`key_delay <press_delay> [delay_after_key] [delay_after_cr]`).
    KeyDelay,
    /// Quoted key-output text, with `\(KEY)` special-key escapes
    /// (`key_output`).
    KeyboardOutput,
    /// 10 comma-separated byte values, a raw keyboard-matrix write
    /// (`keyboard_write`, v1.2).
    KeyboardWrite,
    /// A single required number (`wait`).
    Duration,
    /// One optional number, defaulting to 1 (`wait_driveonoff`).
    OptionalCount,
    /// An optional bare word `vsync` (`screenshot`/`snapshot`).
    OptionalVsyncFlag,
    /// One of `1`, `2`, `3` (`snapshot_version`).
    SnapshotVersion
}

/// One CSL instruction keyword, as an editor would want to offer it: its
/// name, the CSL version it requires, and what argument(s) follow it.
#[derive(Debug, Clone, Copy)]
pub struct InstructionSpec {
    pub name: &'static str,
    pub min_version: CslVersion,
    pub args: ArgShape
}

const V1_0: CslVersion = CslVersion::new(1, 0);
const V1_1: CslVersion = CslVersion::new(1, 1);
const V1_2: CslVersion = CslVersion::new(1, 2);

/// Every CSL instruction keyword `csl_parser::parse_instruction` accepts,
/// in the same order that function tries them.
///
/// `cls_load` - the misspelled compatibility alias
/// `csl_parser::parse_csl_load` also accepts, "kept for compatibility with
/// wrong shaker files" per that parser's own comment - is deliberately not
/// listed: a real script should be steered toward the correctly-spelled
/// `csl_load`, never offered the typo.
pub const INSTRUCTION_SPECS: &[InstructionSpec] = &[
    InstructionSpec {
        name: "csl_version",
        min_version: V1_0,
        args: ArgShape::Version
    },
    InstructionSpec {
        name: "reset",
        min_version: V1_0,
        args: ArgShape::ResetType
    },
    InstructionSpec {
        name: "crtc_select",
        min_version: V1_0,
        args: ArgShape::CrtcModel
    },
    InstructionSpec {
        name: "gate_array",
        min_version: V1_1,
        args: ArgShape::GateArrayModel
    },
    InstructionSpec {
        name: "cpc_model",
        min_version: V1_1,
        args: ArgShape::CpcModel
    },
    InstructionSpec {
        name: "memory_exp",
        min_version: V1_1,
        args: ArgShape::MemoryExpansion
    },
    InstructionSpec {
        name: "rom_dir",
        min_version: V1_1,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "rom_config",
        min_version: V1_1,
        args: ArgShape::RomConfig
    },
    InstructionSpec {
        name: "disk_insert",
        min_version: V1_0,
        args: ArgShape::DiskInsert
    },
    InstructionSpec {
        name: "disk_dir",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "tape_insert",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "tape_dir",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "tape_play",
        min_version: V1_0,
        args: ArgShape::None
    },
    InstructionSpec {
        name: "tape_stop",
        min_version: V1_0,
        args: ArgShape::None
    },
    InstructionSpec {
        name: "tape_rewind",
        min_version: V1_0,
        args: ArgShape::None
    },
    InstructionSpec {
        name: "snapshot_load",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "snapshot_dir",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "key_delay",
        min_version: V1_0,
        args: ArgShape::KeyDelay
    },
    InstructionSpec {
        name: "key_output",
        min_version: V1_0,
        args: ArgShape::KeyboardOutput
    },
    InstructionSpec {
        name: "key_from_file",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "keyboard_write",
        min_version: V1_2,
        args: ArgShape::KeyboardWrite
    },
    InstructionSpec {
        name: "wait",
        min_version: V1_0,
        args: ArgShape::Duration
    },
    InstructionSpec {
        name: "wait_driveonoff",
        min_version: V1_0,
        args: ArgShape::OptionalCount
    },
    InstructionSpec {
        name: "wait_vsyncoffon",
        min_version: V1_0,
        args: ArgShape::None
    },
    InstructionSpec {
        name: "wait_ssm0000",
        min_version: V1_0,
        args: ArgShape::None
    },
    InstructionSpec {
        name: "screenshot_name",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "screenshot_dir",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "screenshot",
        min_version: V1_0,
        args: ArgShape::OptionalVsyncFlag
    },
    InstructionSpec {
        name: "snapshot_name",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
    InstructionSpec {
        name: "snapshot",
        min_version: V1_0,
        args: ArgShape::OptionalVsyncFlag
    },
    InstructionSpec {
        name: "snapshot_version",
        min_version: V1_0,
        args: ArgShape::SnapshotVersion
    },
    InstructionSpec {
        name: "csl_load",
        min_version: V1_0,
        args: ArgShape::QuotedPath
    },
];

/// `CrtcModel`'s tokens, in declaration order - matches
/// `csl_parser::parse_crtc_model`'s accepted alternatives exactly (derived
/// from `Display`, which that parser's own alternatives were written to
/// agree with - `CrtcModel` has no `MemoryExpansion`-style bug: its
/// `Display` output already equals what the parser accepts, since the two
/// were kept trivially in sync as plain digit/letter tokens throughout).
pub fn crtc_model_tokens() -> impl Iterator<Item = String> {
    [
        CrtcModel::Type0,
        CrtcModel::Type1,
        CrtcModel::Type1A,
        CrtcModel::Type1B,
        CrtcModel::Type2,
        CrtcModel::Type3,
        CrtcModel::Type4
    ]
    .into_iter()
    .map(|m| m.to_string())
}

/// `GateArrayModel`'s tokens - see `crtc_model_tokens`'s doc comment.
pub fn gate_array_model_tokens() -> impl Iterator<Item = String> {
    [
        GateArrayModel::Model40007,
        GateArrayModel::Model40008,
        GateArrayModel::Model40010
    ]
    .into_iter()
    .map(|m| m.to_string())
}

/// `CpcModel`'s tokens - see `crtc_model_tokens`'s doc comment.
pub fn cpc_model_tokens() -> impl Iterator<Item = String> {
    [
        CpcModel::Cpc464,
        CpcModel::Cpc664,
        CpcModel::Cpc6128,
        CpcModel::Cpc6128Plus,
        CpcModel::Cpc464Plus,
        CpcModel::GX4000
    ]
    .into_iter()
    .map(|m| m.to_string())
}

/// `MemoryExpansion`'s tokens - **not** sourced from `Display` (which used
/// to disagree with the parser here - see
/// `every_memory_expansion_display_form_reparses_to_the_same_variant` in
/// `csl.rs`'s own tests for the regression this fixed); listed directly in
/// the order `csl_parser::parse_memory_expansion` tries them, "0".."4".
pub fn memory_expansion_tokens() -> impl Iterator<Item = &'static str> {
    ["0", "1", "2", "3", "4"].into_iter()
}

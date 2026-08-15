//! Feature configuration for the LSP: `cpclib-lsp.toml`, one section per
//! language (asm/basic/bndbuild). Deliberately a separate file from
//! `cpclib-asmfmt`'s own `basm-fmt.toml` (formatting is a fully independent
//! concern already handled elsewhere, see `server/backend.rs`'s
//! `textDocument/formatting` handler) - this one is about which diagnostics
//! fire and how they're matched/severity-escalated, not code style.
//!
//! Every field's default reproduces today's exact behavior: a missing,
//! empty, or partially-filled config file changes nothing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE_NAME: &str = "cpclib-lsp.toml";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct LspConfig {
    pub asm: AsmConfig,
    pub basic: BasicConfig,
    pub bndbuild: BndbuildConfig,
    pub dap: DapConfig
}

/// Debug-adapter settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DapConfig {
    /// Where to write a transcript of the whole debug conversation.
    ///
    /// Empty means no log. Diagnosing a debug session from the outside is
    /// otherwise guesswork - a pane that fails to appear says nothing about
    /// which message caused it - so this exists to turn a symptom into
    /// evidence. Relative paths are resolved against the project root.
    pub log: String
}

impl Default for DapConfig {
    fn default() -> Self {
        Self {
            log: String::new()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AsmConfig {
    /// Case-sensitive symbol matching for goto-definition/references, and
    /// forwarded to `cpclib_asm::AssemblingOptions::set_case_sensitive` for
    /// real dry-run assembling - one config value, two consumers, no drift.
    /// Matches basm's own real (case-sensitive) default.
    pub case_sensitive: bool,
    /// Escalate every WARNING-severity diagnostic in an assembly document
    /// to ERROR. A different, editor-specific mechanism from `basm
    /// --Werror` (which fails the whole build) - see `warnings_as_errors`
    /// in `server/backend.rs::compute_diagnostics`.
    pub warnings_as_errors: bool,
    pub warnings: AsmWarningClasses,
    /// Show firmware routine/constant documentation (Action/Entry/Exit/Notes,
    /// extracted from `cpclib-asm`'s embedded `inner://firmware/*.asm`
    /// assets) on hover - both for the symbolic name (`TXT_OUTPUT`) and its
    /// resolved numeric address (`&BB5A`). See `common::firmware_docs`.
    pub firmware_docs: bool,
    /// Which `cpclib-asmoptim` built-in rule set the peephole-optimizer
    /// diagnostic, quickfix, and "Fix All" CodeLens all match against - see
    /// `PeepholeGoal`'s own doc comment.
    pub peephole_goal: PeepholeGoal,
    /// Fade the branches of an `IF`/`ELSEIF`/`ELSE` whose condition can be
    /// decided at assembly time, the way an editor fades an inactive `#if`
    /// block in C.
    ///
    /// Published as `Unnecessary`-tagged `HINT` diagnostics, so they colour
    /// the code without appearing in the Problems panel and without being
    /// touched by `warnings_as_errors` (which only escalates WARNINGs).
    pub inactive_code: bool,
    /// Inlay hints naming what a closing directive belongs to - the `ENDIF`
    /// of *which* `IF`, and so on.
    pub inlay_hints: bool,
    /// CodeLens buttons above assembly code: "▶ Run" for each rule of an
    /// embedded `#!bndbuild` block, and the peephole optimizer's "⚡ Fix All"
    /// summary.
    pub code_lens: bool,
    /// The directive an editor breakpoint writes into the source.
    ///
    /// Toggling the gutter's red dot inserts this in front of the line's first
    /// instruction, followed by basm's statement separator; clearing the
    /// breakpoint takes it back out. Only the word is configurable - spelling
    /// is a house style, and `BREAKPOINT`/`breakpoint` are equally valid basm.
    ///
    /// Removal recognises the directive through the parsed AST rather than by
    /// matching this text, so changing it does not strand breakpoints already
    /// written in a different case.
    pub breakpoint_directive: String,
    /// The project's entry file, relative to the project root - the one a real
    /// build assembles (`src/sna.asm` in a typical demo).
    ///
    /// Address-aware analysis (`jp2jr`) needs the addresses a real build
    /// produces, and a file that is only ever `include`d does not produce them
    /// on its own: assembled alone it has no memory map, and the constants
    /// that decide which conditional blocks exist live in the entry.
    ///
    /// Left empty, the entry is discovered by following the include graph back
    /// to whichever `RUN`-bearing file reaches the open document. Set this only
    /// when that is ambiguous - a shared file included by several programs has
    /// genuinely different addresses in each, so the LSP refuses to guess and
    /// simply reports no address-aware suggestions until told which to use.
    #[serde(default)]
    pub entry: Option<String>
}

impl Default for AsmConfig {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            warnings_as_errors: false,
            warnings: AsmWarningClasses::default(),
            inactive_code: true,
            inlay_hints: true,
            code_lens: true,
            breakpoint_directive: default_breakpoint_directive(),
            firmware_docs: true,
            peephole_goal: PeepholeGoal::default(),
            entry: None
        }
    }
}

/// Mirrors `cpclib_asmoptim::OptimizationGoal` (which has no `serde` support
/// of its own - this crate is the only one so far that needs to deserialize
/// one) - `Neutral` (the default) only matches rules every goal agrees are
/// improvements; `Size`/`Speed` add extra rules that can actively disagree
/// with each other (e.g. `jp` vs `jr`), see `builtin_rules`'s own doc
/// comment in `cpclib-asmoptim`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PeepholeGoal {
    #[default]
    Neutral,
    Size,
    Speed
}

impl From<PeepholeGoal> for cpclib_asmoptim::OptimizationGoal {
    fn from(goal: PeepholeGoal) -> Self {
        match goal {
            PeepholeGoal::Neutral => Self::Neutral,
            PeepholeGoal::Size => Self::Size,
            PeepholeGoal::Speed => Self::Speed
        }
    }
}

/// `fake_instructions`/`redundant_accumulator_prefix`/`override_memory`/
/// `overflow` each map directly to a `cpclib_asm::WarningCategory` -
/// forwarded to real `ParserOptions`/`AssemblingOptions` before parsing/
/// assembling (filtered at the source, exactly like `basm --disable-warning`
/// would), never filtered after the fact from already-collected diagnostics.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AsmWarningClasses {
    pub fake_instructions: bool,
    pub redundant_accumulator_prefix: bool,
    pub override_memory: bool,
    pub overflow: bool,
    /// LSP-only - `cpclib_asm::unused_bindings::find_unused_bindings` isn't
    /// part of the `env.warnings()` pipeline at all today (`basm` itself
    /// never calls it), so this one has no assembler-level counterpart to
    /// forward to.
    pub unused_bindings: bool,
    /// LSP-only, same as `unused_bindings` above - `cpclib-asmoptim`'s
    /// matching engine is a separate, advisory-only pass (see that crate's
    /// own doc comment) that never runs as part of a real `basm` assemble
    /// and never changes what gets assembled. Suggests improvable Z80
    /// instruction sequences (e.g. `ld a,0` where `xor a` would do) using
    /// real, community-vetted rules from `mdlz80optimizer`'s pattern format.
    ///
    /// **Off by default**, unlike every other warning class here. Deciding
    /// whether a `jp` reaches as a `jr` needs the addresses a real build
    /// produces, which means assembling the whole project - 37s cold for a
    /// demo the size of `birthtro`. That is not a price to pay unprompted, on
    /// a keystroke, for advice. Turn it on to have the suggestions appear as
    /// you type; leave it off and ask for them explicitly with
    /// `cpclib.analyzePeephole` (VS Code: *CPClib: Find Peephole
    /// Optimizations in File / Selection / Workspace*), which reports for the
    /// chosen document exactly as if this were on, until
    /// `cpclib.clearPeephole`.
    pub peephole_optimizer: bool,
    /// LSP-only. A label written where the self-modifying-code idiom wants
    /// `equ $-1` - `ld a, 0 : .counter` instead of
    /// `ld a, 0 : .counter equ $-1`. Both assemble; only one names the byte
    /// the patch is meant to reach.
    pub smc_label_without_equ: bool
}

impl AsmWarningClasses {
    /// The warning categories to switch off in the *parser*.
    ///
    /// `OverrideMemory`/`Overflow` never apply here - they are only knowable
    /// once something is actually assembled.
    pub fn disabled_parser_categories(
        &self
    ) -> enumflags2::BitFlags<cpclib_asm::WarningCategory> {
        use cpclib_asm::WarningCategory;
        let mut disabled = enumflags2::BitFlags::empty();
        if !self.fake_instructions {
            disabled.insert(WarningCategory::FakeInstruction);
        }
        if !self.redundant_accumulator_prefix {
            disabled.insert(WarningCategory::RedundantAccumulatorPrefix);
        }
        disabled
    }

    /// The same, for real assembling - all four categories, since the two
    /// parser ones are passed again here as a backstop.
    pub fn disabled_assembling_categories(
        &self
    ) -> enumflags2::BitFlags<cpclib_asm::WarningCategory> {
        use cpclib_asm::WarningCategory;
        let mut disabled = self.disabled_parser_categories();
        if !self.override_memory {
            disabled.insert(WarningCategory::OverrideMemory);
        }
        if !self.overflow {
            disabled.insert(WarningCategory::Overflow);
        }
        disabled
    }
}

impl Default for AsmWarningClasses {
    fn default() -> Self {
        Self {
            fake_instructions: true,
            redundant_accumulator_prefix: true,
            override_memory: true,
            overflow: true,
            unused_bindings: true,
            peephole_optimizer: false,
            smc_label_without_equ: true
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BasicConfig {
    pub warnings_as_errors: bool,
    pub warnings: BasicWarningClasses,
    /// The "▶ Run in emulator" CodeLens at the top of a `.bas` file.
    pub code_lens: bool,
    /// Inlay hints in BASIC listings.
    pub inlay_hints: bool,
    /// Which emulator `cpclib.runBasic` (the "▶ Run in emulator" CodeLens on
    /// `.bas` files) launches. Must be one of
    /// `cpclib_bndbuild::pipeline::basic_run::SUPPORTED_AUTO_RUN_EMULATORS` (kept in
    /// sync manually: `ace`, `winape`, `cpcemupower`, `caprice`,
    /// `emulator1984`, `amspirit`) - every other backend either silently
    /// ignores auto-run or (SugarBoxV2) panics, so an unsupported value is
    /// rejected with a clear error at run time rather than attempted.
    pub run_emulator: String,
    /// Same firmware documentation-on-hover feature as `AsmConfig::
    /// firmware_docs`, independently toggleable - BASIC's own `CALL &BB18`
    /// is just as common a way to invoke a firmware routine as asm's `call
    /// &BB5A`. See `common::firmware_docs`.
    pub firmware_docs: bool
}

impl Default for BasicConfig {
    fn default() -> Self {
        Self {
            warnings_as_errors: false,
            warnings: BasicWarningClasses::default(),
            code_lens: true,
            inlay_hints: true,
            run_emulator: "ace".to_string(),
            firmware_docs: true
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BasicWarningClasses {
    pub undefined_line: bool,
    pub catart_no_op: bool
}

impl Default for BasicWarningClasses {
    fn default() -> Self {
        Self {
            undefined_line: true,
            catart_no_op: true
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BndbuildConfig {
    pub warnings_as_errors: bool,
    pub warnings: BndbuildWarningClasses,
    /// The "▶ Run" CodeLens on each rule and each individual task command.
    pub code_lens: bool
}

impl Default for BndbuildConfig {
    // Written out rather than derived: a `bool` feature switch that derives
    // its default is off, which for a feature that ships enabled is a silent
    // regression the type system will not catch.
    fn default() -> Self {
        Self {
            warnings_as_errors: false,
            warnings: BndbuildWarningClasses::default(),
            code_lens: true
        }
    }
}

/// basm accepts either case; this is the one written by default.
fn default_breakpoint_directive() -> String {
    "BREAKPOINT".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BndbuildWarningClasses {
    pub missing_build_structure: bool,
    pub missing_dependency: bool
}

impl Default for BndbuildWarningClasses {
    fn default() -> Self {
        Self {
            missing_build_structure: true,
            missing_dependency: true
        }
    }
}

/// Result of trying to load `cpclib-lsp.toml`: an *always-usable* config
/// (missing/unreadable/malformed all fall back to `LspConfig::default()` -
/// a bad config file must never prevent the LSP from starting) plus an
/// optional human-readable error the caller should surface to the editor
/// (`initialize()` turns this into `window/showMessage`) - unlike
/// `cpclib_asmfmt::config::load_config`, which silently swallows a parse
/// error into its own default with no caller-visible signal at all, that's
/// not acceptable here: a typo in a hand-edited config silently reverting
/// to defaults with zero feedback is confusing to debug.
pub struct LoadedConfig {
    pub config: LspConfig,
    pub error: Option<String>
}

/// Search `workspace_root` itself (not further up, unlike
/// `cpclib_asmfmt::config`'s `cwd`-based walk-to-filesystem-root - a
/// long-running, editor-launched server's `cwd` is meaningless, but its
/// workspace root is the actually correct anchor) for `cpclib-lsp.toml`,
/// then fall back to the XDG-style global config location, mirroring
/// `cpclib_asmfmt::config`'s own fallback convention.
/// Locate `cpclib-lsp.toml`.
///
/// Searched **upwards** from `workspace_root`, then in the user's config
/// directory. The ancestor walk matters more than it looks: a project is often
/// opened at a subdirectory (`birthtro/src`) while the configuration sits at
/// its root, and a tool that only checks the one directory it was handed finds
/// nothing and says nothing - the setting appears to be ignored.
pub fn find_config_file(workspace_root: Option<&Path>) -> Option<PathBuf> {
    if let Some(root) = workspace_root {
        let mut directory = Some(root);
        while let Some(current) = directory {
            let path = current.join(CONFIG_FILE_NAME);
            if path.is_file() {
                return Some(path);
            }
            directory = current.parent();
        }
    }
    let config_base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|_| std::env::var("APPDATA").map(PathBuf::from))
        .ok()?;
    let path = config_base.join("cpclib-lsp").join(CONFIG_FILE_NAME);
    if path.is_file() { Some(path) } else { None }
}

pub fn load_config(workspace_root: Option<&Path>) -> LoadedConfig {
    let Some(path) = find_config_file(workspace_root)
    else {
        return LoadedConfig {
            config: LspConfig::default(),
            error: None
        };
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return LoadedConfig {
                config: LspConfig::default(),
                error: Some(format!("cannot read {}: {e}", path.display()))
            };
        }
    };
    match toml::from_str(&content) {
        Ok(config) => {
            LoadedConfig {
                config,
                error: None
            }
        },
        Err(e) => {
            LoadedConfig {
                config: LspConfig::default(),
                error: Some(format!("invalid config in {}: {e}", path.display()))
            }
        },
    }
}

/// Merges any field present in the canonical schema (`EXAMPLE_CONFIG_TOML`)
/// but missing from `existing_toml` into it, leaving every already-present
/// key - its value *and* its comments - completely untouched. Returns the
/// merged document text plus the dotted path of every field that was added
/// (empty if the config was already fully up to date). Used by `cpclib-lsp
/// --update-config` to bring an older hand-edited config up to date with
/// newly-added schema fields without a destructive rewrite (a plain
/// `toml::from_str`/`to_string` round-trip would silently drop the user's
/// own comments and reformat the whole file).
pub fn merge_missing_config_fields(existing_toml: &str) -> Result<(String, Vec<String>), String> {
    let mut existing = existing_toml
        .parse::<toml_edit::Document>()
        .map_err(|e| e.to_string())?;
    let template = EXAMPLE_CONFIG_TOML
        .parse::<toml_edit::Document>()
        .map_err(|e| e.to_string())?;

    let mut added = Vec::new();
    merge_missing_into(existing.as_table_mut(), template.as_table(), "", &mut added);

    Ok((existing.to_string(), added))
}

fn merge_missing_into(
    existing: &mut toml_edit::Table,
    template: &toml_edit::Table,
    prefix: &str,
    added: &mut Vec<String>
) {
    for (key, template_item) in template.iter() {
        let path = if prefix.is_empty() {
            key.to_string()
        }
        else {
            format!("{prefix}.{key}")
        };
        if !existing.contains_key(key) {
            existing.insert(key, template_item.clone());
            added.push(path);
        }
        else if let (Some(existing_table), Some(template_table)) = (
            existing.get_mut(key).and_then(|i| i.as_table_mut()),
            template_item.as_table()
        ) {
            merge_missing_into(existing_table, template_table, &path, added);
        }
    }
}

/// Hand-written, fully-commented template (not derived via `toml::to_string`,
/// which can't preserve comments/grouping) - written by `cpclib-lsp
/// --init-config`. Kept in sync with `LspConfig`'s real schema by the
/// `example_config_toml_matches_the_real_schema` test below.
pub const EXAMPLE_CONFIG_TOML: &str = r#"# cpclib-lsp configuration file.
# Every field's default reproduces the LSP's exact out-of-the-box behavior -
# delete any section/field you don't want to override.

[asm]
# Case-sensitive symbol matching for goto-definition/references, and for
# real dry-run assembling (matches basm's own real default).
case_sensitive = true
# Escalate every warning in an assembly document to an error.
warnings_as_errors = false
# Show firmware routine/constant documentation on hover, both for the
# symbolic name (TXT_OUTPUT) and its resolved numeric address (&BB5A).
firmware_docs = true
# Fade the branches of an IF/ELSEIF/ELSE whose condition can be decided at
# assembly time, the way an inactive #if block is faded in C. Published as
# hints, so they never reach the Problems panel.
inactive_code = true
# Inlay hints naming what a closing directive belongs to - the ENDIF of
# *which* IF, and so on.
inlay_hints = true
# CodeLens buttons above assembly code: "▶ Run" for each rule of an embedded
# #!bndbuild block, and the peephole optimizer's "⚡ Fix All" summary.
code_lens = true
# The directive an editor breakpoint writes in front of the line's first
# instruction (basm's statement separator is added after it). Removing the
# breakpoint takes it back out - recognised through the parse, so changing
# this does not strand breakpoints already written in another case.
breakpoint_directive = "BREAKPOINT"
# Which built-in peephole-optimization rule set to match against: "neutral"
# (only rules every goal agrees are improvements), "size", or "speed" - the
# latter two add extra rules that can actively disagree with each other
# (e.g. turning jp into jr, or jr into jp), so only one is ever active.
peephole_goal = "neutral"

[asm.warnings]
# ADD/ADC/SBC/CP/SUB/AND/OR/XOR with an explicit "A," prefix that isn't
# required (e.g. "cp a, c" instead of "cp c").
redundant_accumulator_prefix = true
# A basm shorthand assembled from several real opcodes (e.g. "add de, bc").
fake_instructions = true
# A byte/word write that overlaps a previously-written address.
override_memory = true
# A value that doesn't fit the 8/16-bit slot it's being written into.
overflow = true
# A declared-but-never-referenced MACRO/FUNCTION parameter or REPEAT/
# ITERATE/FOR loop counter (LSP-only diagnostic, no basm CLI equivalent).
unused_bindings = true
# An improvable Z80 instruction sequence (e.g. "ld a,0" where "xor a" would
# do), using real community-vetted rules (LSP-only advisory, no basm CLI
# equivalent - never changes what actually gets assembled).
# Off by default: answering needs a whole-project assemble. Ask on demand
# with the "CPClib: Find Peephole Optimizations" commands instead.
peephole_optimizer = false
# A label written where the self-modifying-code idiom wants "equ $-1":
# "ld a, 0 : .counter" names the address *after* the instruction, so a patch
# through it overwrites the next opcode instead of the operand.
smc_label_without_equ = true

[basic]
warnings_as_errors = false
# The "▶ Run in emulator" CodeLens at the top of a .bas file.
code_lens = true
# Inlay hints in BASIC listings.
inlay_hints = true
# Emulator launched by the "▶ Run in emulator" CodeLens on .bas files.
# Supported (the only backends that honor auto-RUN): ace, winape,
# cpcemupower, caprice, emulator1984, amspirit.
run_emulator = "ace"
# Same firmware documentation-on-hover feature as asm.firmware_docs,
# independently toggleable - covers e.g. "CALL &BB18" in BASIC.
firmware_docs = true

[basic.warnings]
# A GOTO/GOSUB/etc. target line that doesn't exist in the program.
undefined_line = true
# A CatArt-only warning: CURSOR/SYMBOL are valid BASIC but no-ops in CatArt.
catart_no_op = true

[dap]
# Write a transcript of the whole debug-adapter conversation to this file.
# Empty means no log. Relative paths are resolved against the project root.
# Useful when a debug session misbehaves: the log shows every message in both
# directions, which turns "the panes are empty" into something diagnosable.
log = ""

[bndbuild]
warnings_as_errors = false
# The "▶ Run" CodeLens on each rule, and on each individual task command.
code_lens = true

[bndbuild.warnings]
# The build file has neither a "targets:" nor a "tasks:" section.
missing_build_structure = true
# A dependency that's neither an existing file nor a target this file builds.
missing_dependency = true
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_toml_matches_the_real_schema() {
        let config: LspConfig = toml::from_str(EXAMPLE_CONFIG_TOML)
            .expect("EXAMPLE_CONFIG_TOML must parse against the real LspConfig schema");
        assert!(config.asm.case_sensitive);
        assert!(config.asm.warnings.unused_bindings);
    }

    /// Collect every `section.field` path a TOML value contains.
    fn key_paths(value: &toml::Value, prefix: &str, out: &mut Vec<String>) {
        if let Some(table) = value.as_table() {
            for (key, item) in table {
                let path = if prefix.is_empty() {
                    key.clone()
                }
                else {
                    format!("{prefix}.{key}")
                };
                if item.is_table() {
                    key_paths(item, &path, out);
                }
                else {
                    out.push(path);
                }
            }
        }
    }

    /// Every setting the server understands must appear in the template.
    ///
    /// Parsing the template is not enough to know this: every struct here is
    /// `#[serde(default)]`, so a field left out of the template still parses -
    /// it just silently becomes undiscoverable, which is how a documented
    /// config drifts from a real one. Comparing key *paths* against the
    /// serialized defaults is what actually catches the omission.
    #[test]
    fn every_setting_appears_in_the_example_config() {
        let template: toml::Value = toml::from_str(EXAMPLE_CONFIG_TOML).unwrap();
        let schema = toml::Value::try_from(LspConfig::default()).unwrap();

        let (mut wanted, mut present) = (Vec::new(), Vec::new());
        key_paths(&schema, "", &mut wanted);
        key_paths(&template, "", &mut present);

        let missing: Vec<&String> = wanted.iter().filter(|k| !present.contains(k)).collect();
        assert!(
            missing.is_empty(),
            "these settings exist but are not in EXAMPLE_CONFIG_TOML: {missing:?}"
        );

        // And nothing in the template that the server would ignore.
        let unknown: Vec<&String> = present.iter().filter(|k| !wanted.contains(k)).collect();
        assert!(
            unknown.is_empty(),
            "EXAMPLE_CONFIG_TOML documents settings that do not exist: {unknown:?}"
        );
    }

    #[test]
    fn missing_config_file_yields_defaults_with_no_error() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let loaded = load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.error.is_none());
        assert!(loaded.config.asm.case_sensitive);
        assert!(loaded.config.asm.warnings.fake_instructions);
    }

    /// A project opened at a subdirectory still finds the configuration at its
    /// root - the case that made a configured setting look ignored.
    #[test]
    fn the_config_is_found_from_a_subdirectory() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(CONFIG_FILE_NAME),
            "[asm]\ncase_sensitive = false\n"
        )
        .unwrap();
        let deep = tmp.path().join("src").join("effects");
        std::fs::create_dir_all(&deep).unwrap();

        let found = find_config_file(Some(deep.as_std_path())).expect("found from below");
        assert_eq!(found, tmp.path().join(CONFIG_FILE_NAME).as_std_path());

        let loaded = load_config(Some(deep.as_std_path()));
        assert!(!loaded.config.asm.case_sensitive, "and it is really loaded");
    }

    #[test]
    fn config_file_found_directly_in_the_workspace_root() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(CONFIG_FILE_NAME),
            "[asm]\ncase_sensitive = false\n"
        )
        .unwrap();
        let loaded = load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        assert!(!loaded.config.asm.case_sensitive);
        // Every other field stays at its own default.
        assert!(loaded.config.asm.warnings.fake_instructions);
        assert!(loaded.config.basic.warnings.undefined_line);
    }

    #[test]
    fn a_partial_warnings_table_overrides_only_the_given_field() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(CONFIG_FILE_NAME),
            "[asm.warnings]\noverride_memory = false\n"
        )
        .unwrap();
        let loaded = load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        assert!(!loaded.config.asm.warnings.override_memory);
        assert!(loaded.config.asm.warnings.overflow);
        assert!(loaded.config.asm.warnings.fake_instructions);
        assert!(loaded.config.asm.warnings.redundant_accumulator_prefix);
        assert!(loaded.config.asm.warnings.unused_bindings);
    }

    #[test]
    fn basic_config_defaults_run_emulator_to_ace() {
        assert_eq!(BasicConfig::default().run_emulator, "ace");
    }

    #[test]
    fn a_configured_run_emulator_round_trips_through_load_config() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(CONFIG_FILE_NAME),
            "[basic]\nrun_emulator = \"winape\"\n"
        )
        .unwrap();
        let loaded = load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        assert_eq!(loaded.config.basic.run_emulator, "winape");
    }

    #[test]
    fn peephole_goal_defaults_to_neutral_and_is_configurable_via_toml() {
        assert_eq!(AsmConfig::default().peephole_goal, PeepholeGoal::Neutral);

        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(CONFIG_FILE_NAME),
            "[asm]\npeephole_goal = \"size\"\n"
        )
        .unwrap();
        let loaded = load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        assert_eq!(loaded.config.asm.peephole_goal, PeepholeGoal::Size);
    }

    #[test]
    fn every_peephole_goal_maps_to_the_matching_optimization_goal() {
        use cpclib_asmoptim::OptimizationGoal;
        assert_eq!(
            OptimizationGoal::from(PeepholeGoal::Neutral),
            OptimizationGoal::Neutral
        );
        assert_eq!(OptimizationGoal::from(PeepholeGoal::Size), OptimizationGoal::Size);
        assert_eq!(OptimizationGoal::from(PeepholeGoal::Speed), OptimizationGoal::Speed);
    }

    #[test]
    fn malformed_toml_falls_back_to_defaults_and_reports_the_error() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(CONFIG_FILE_NAME),
            "this is not valid toml [[["
        )
        .unwrap();
        let loaded = load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.config.asm.case_sensitive);
        let err = loaded.error.expect("expected a reported parse error");
        assert!(err.contains(CONFIG_FILE_NAME), "{err}");
    }

    #[test]
    fn merge_adds_every_missing_field_and_preserves_existing_values_and_comments() {
        let existing = "# my own note about this setting\n[asm]\ncase_sensitive = false\n";
        let (merged, added) = merge_missing_config_fields(existing).unwrap();

        // Existing value and its comment survive untouched.
        assert!(
            merged.contains("# my own note about this setting"),
            "{merged}"
        );
        assert!(merged.contains("case_sensitive = false"), "{merged}");
        assert!(
            !added.contains(&"asm.case_sensitive".to_string()),
            "{added:?}"
        );

        // Missing fields within the already-present [asm] table are added
        // individually; entirely-missing top-level sections ([basic],
        // [bndbuild]) are added wholesale as a single unit, template
        // comments and all - not recursed into field-by-field, since there
        // was nothing to merge against yet.
        assert!(
            added.contains(&"asm.firmware_docs".to_string()),
            "{added:?}"
        );
        assert!(added.contains(&"basic".to_string()), "{added:?}");
        assert!(added.contains(&"bndbuild".to_string()), "{added:?}");

        // The merged document is itself a valid, fully-populated config.
        let config: LspConfig = toml::from_str(&merged).unwrap();
        assert!(config.asm.firmware_docs);
        assert!(!config.asm.case_sensitive);
    }

    #[test]
    fn merging_an_already_up_to_date_config_is_a_no_op() {
        let (_, added) = merge_missing_config_fields(EXAMPLE_CONFIG_TOML).unwrap();
        assert!(added.is_empty(), "{added:?}");
    }

    #[test]
    fn merge_rejects_malformed_existing_toml() {
        assert!(merge_missing_config_fields("this is not valid toml [[[").is_err());
    }
}

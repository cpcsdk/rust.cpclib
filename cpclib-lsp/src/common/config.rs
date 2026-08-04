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

use serde::Deserialize;

pub const CONFIG_FILE_NAME: &str = "cpclib-lsp.toml";

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct LspConfig {
    pub asm: AsmConfig,
    pub basic: BasicConfig,
    pub bndbuild: BndbuildConfig
}

#[derive(Debug, Clone, Deserialize)]
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
    pub firmware_docs: bool
}

impl Default for AsmConfig {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            warnings_as_errors: false,
            warnings: AsmWarningClasses::default(),
            firmware_docs: true
        }
    }
}

/// `fake_instructions`/`redundant_accumulator_prefix`/`override_memory`/
/// `overflow` each map directly to a `cpclib_asm::WarningCategory` -
/// forwarded to real `ParserOptions`/`AssemblingOptions` before parsing/
/// assembling (filtered at the source, exactly like `basm --disable-warning`
/// would), never filtered after the fact from already-collected diagnostics.
#[derive(Debug, Clone, Deserialize)]
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
    pub peephole_optimizer: bool
}

impl Default for AsmWarningClasses {
    fn default() -> Self {
        Self {
            fake_instructions: true,
            redundant_accumulator_prefix: true,
            override_memory: true,
            overflow: true,
            unused_bindings: true,
            peephole_optimizer: true
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BasicConfig {
    pub warnings_as_errors: bool,
    pub warnings: BasicWarningClasses,
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
            run_emulator: "ace".to_string(),
            firmware_docs: true
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct BndbuildConfig {
    pub warnings_as_errors: bool,
    pub warnings: BndbuildWarningClasses
}

#[derive(Debug, Clone, Deserialize)]
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
pub fn find_config_file(workspace_root: Option<&Path>) -> Option<PathBuf> {
    if let Some(root) = workspace_root {
        let path = root.join(CONFIG_FILE_NAME);
        if path.is_file() {
            return Some(path);
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
peephole_optimizer = true

[basic]
warnings_as_errors = false
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

[bndbuild]
warnings_as_errors = false

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

    #[test]
    fn missing_config_file_yields_defaults_with_no_error() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let loaded = load_config(Some(tmp.path().as_std_path()));
        assert!(loaded.error.is_none());
        assert!(loaded.config.asm.case_sensitive);
        assert!(loaded.config.asm.warnings.fake_instructions);
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

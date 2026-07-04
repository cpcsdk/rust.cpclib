//! LSP support data - build tasks and keywords
//!
//! This module provides constants for Language Server Protocol implementations
//! to offer code completion, hover information, and validation for build files.

use crate::task::*;

/// A rule-level YAML key: one canonical name plus its accepted aliases,
/// derived directly from the `#[serde(alias = ...)]` annotations on `Rule`.
#[derive(Debug, Clone)]
pub struct RuleKey {
    /// All accepted names: first entry is the canonical form, the rest are aliases.
    pub names: &'static [&'static str],
    pub description: &'static str,
    pub required: bool
}

/// Rule-level YAML keys, kept in sync with the serde aliases on the `Rule` struct.
pub const RULE_KEYS: &[RuleKey] = &[
    RuleKey {
        names: &["targets", "tgt", "target", "build"],
        description: "Target file(s) this rule produces. Required.",
        required: true
    },
    RuleKey {
        names: &["dependencies", "dep", "dependency", "requires"],
        description: "Files or targets that must be up to date before building this rule.",
        required: false
    },
    RuleKey {
        names: &["tasks", "cmd", "command", "launch", "run"],
        description: "List of tasks to execute in order to build the target(s).",
        required: false
    },
    RuleKey {
        names: &["help"],
        description: "Help text shown when listing available targets (`bndbuild --list`).",
        required: false
    },
    RuleKey {
        names: &["phony"],
        description: "If true, the target is always rebuilt regardless of file timestamps (like `.PHONY` in make).",
        required: false
    },
    RuleKey {
        names: &["constraint"],
        description: "Condition under which this rule applies (e.g. OS or environment constraint).",
        required: false
    }
];

/// Task type definition with description
#[derive(Debug, Clone)]
pub struct TaskType {
    pub names: &'static [&'static str],
    pub description: &'static str,
    pub example: &'static str
}

/// All available task types with their metadata
pub const TASK_TYPES: &[TaskType] = &[
    // Assemblers
    TaskType {
        names: BASM_CMDS,
        description: "Assemble Z80 source code using basm (built-in assembler)",
        example: "basm [OPTIONS] [INPUT]"
    },
    TaskType {
        names: ORGAMS_CMDS,
        description: "Assemble using Orgams assembler syntax",
        example: "orgams [OPTIONS] --src <SRC>"
    },
    TaskType {
        names: RASM_CMDS,
        description: "Assemble using Rasm assembler",
        example: "rasm <inputfile> [options]"
    },
    TaskType {
        names: SJASMPLUS_CMDS,
        description: "Assemble using SjASMPlus assembler",
        example: "sjasmplus [options] sourcefile(s)"
    },
    TaskType {
        names: VASM_CMDS,
        description: "Assemble using vasm assembler",
        example: "vasm [options] filesm"
    },
    TaskType {
        names: VLINK_CMDS,
        description: "Link object files using vlink linker",
        example: "TODO"
    },
    // Disassemblers
    TaskType {
        names: BDASM_CMDS,
        description: "Disassemble Z80 binary code",
        example: "bdasm [OPTIONS] <INPUT>"
    },
    TaskType {
        names: DISARK_CMDS,
        description: "Disassemble using Disark",
        example: ""
    },
    TaskType {
        names: UZ80_CMDS,
        description: "Disassemble using uz80",
        example: ""
    },
    // Documentation
    TaskType {
        names: BASMDOC_CMDS,
        description: "Generate documentation from basm source",
        example: "basmdoc [OPTIONS] --output <OUTPUT> <INPUT>..."
    },
    // Emulators
    TaskType {
        names: ACE_CMDS,
        description: "Run in ACE emulator",
        example: ""
    },
    TaskType {
        names: CPCEC_CMDS,
        description: "Run in CPCEC emulator",
        example: ""
    },
    TaskType {
        names: SUGARBOX_CMDS,
        description: "Run in Sugarbox emulator",
        example: ""
    },
    TaskType {
        names: WINAPE_CMDS,
        description: "Run in WinAPE emulator",
        example: ""
    },
    TaskType {
        names: RETROVM_CMDS,
        description: "Run in RetroVirtual Machine emulator",
        example: ""
    },
    TaskType {
        names: AMSPIRIT_CMDS,
        description: "Run in AmSpirit emulator",
        example: ""
    },
    TaskType {
        names: CPCEMU_CMDS,
        description: "Run in CPCEmu emulator",
        example: ""
    },
    TaskType {
        names: CPCEMUPOWER_CMDS,
        description: "Run in CPCEmuPower emulator",
        example: ""
    },
    TaskType {
        names: CAPRICEFOREVER_CMDS,
        description: "Run in Caprice Forever emulator",
        example: ""
    },
    TaskType {
        names: CADENCE_CMDS,
        description: "Run in Cadence emulator",
        example: ""
    },
    TaskType {
        names: EMULATOR_1984_CMDS,
        description: "Run in 1984 emulator",
        example: ""
    },
    // Disk and snapshot operations
    TaskType {
        names: DISC_CMDS,
        description: "Create or modify DSK disk images",
        example: "disc_manager <DSK_FILE> [COMMAND]"
    },
    TaskType {
        names: SNA_CMDS,
        description: "Create or modify SNA snapshot files",
        example: "createSnapshot [OPTIONS] -- <OUTPUT>"
    },
    TaskType {
        names: CATALOG_CMDS,
        description: "List disk catalog",
        example: "catalog [INPUT_FILE] [COMMAND]"
    },
    TaskType {
        names: IMPDISC_CMDS,
        description: "ImpDisk disk operations",
        example: ""
    },
    TaskType {
        names: HXCFE_CMDS,
        description: "HxC Floppy Emulator file operations",
        example: "hxcfe [OPTIONS]"
    },
    TaskType {
        names: RTZX_CMDS,
        description: "Convert tape files to/from TZX format",
        example: ""
    },
    TaskType {
        names: TWO_CDT_CMDS,
        description: "Convert files to CDT tape format",
        example: ""
    },
    // Image conversion
    TaskType {
        names: IMG2CPC_CMDS,
        description: "Convert images to Amstrad CPC format",
        example: "img2cpc [OPTIONS] <SOURCE> [COMMAND]"
    },
    TaskType {
        names: CPC2IMG_CMDS,
        description: "Convert Amstrad CPC images to standard formats",
        example: "- cpc2img:\n    args: input.scr -o output.png"
    },
    TaskType {
        names: MARTINE_CMDS,
        description: "Convert images using Martine",
        example: "- martine:\n    args: -in input.png"
    },
    TaskType {
        names: GRAFX2_CMDS,
        description: "Open/edit images in GrafX2",
        example: "- grafx2:\n    args: image.png"
    },
    TaskType {
        names: CONVGENERIC_CMDS,
        description: "Generic image conversion",
        example: "- convgeneric:\n    args: input.png"
    },
    TaskType {
        names: HIDEUR_CMDS,
        description: "HiDeur image compression",
        example: "- hideur:\n    args: input.scr"
    },
    // Audio tools
    TaskType {
        names: AT_CMDS,
        description: "Play or convert Arkos Tracker files",
        example: "- at:\n    args: music.akt"
    },
    TaskType {
        names: AYT_CMDS,
        description: "Process AY sound files",
        example: "- ayt:\n    args: sound.ay"
    },
    TaskType {
        names: CHIPNSFX_CMDS,
        description: "ChipNSFX sound effects",
        example: "- chipnsfx:\n    args: sound.nsf"
    },
    TaskType {
        names: HSPC_CMDS,
        description: "HSP compiler for music",
        example: "- hspc:\n    args: music.hsp"
    },
    TaskType {
        names: MINY_CMDS,
        description: "Minimise AY music files",
        example: "- miny:\n    args: music.ay"
    },
    #[cfg(feature = "fap")]
    TaskType {
        names: FAP_CMDS,
        description: "Convert AY music to FAP format",
        example: "- fap:\n    args: music.ay"
    },
    TaskType {
        names: SONG2AKM_CMDS,
        description: "Convert song to AKM format",
        example: "- SongToAkm:\n    args: input.song"
    },
    // File operations
    TaskType {
        names: CP_CMDS,
        description: "Copy files",
        example: "- cp:\n    args: source.bin dest.bin"
    },
    TaskType {
        names: MV_CMDS,
        description: "Move or rename files",
        example: "- mv:\n    args: old.bin new.bin"
    },
    TaskType {
        names: RM_CMDS,
        description: "Remove files",
        example: "- rm:\n    args: temp.bin"
    },
    TaskType {
        names: MKDIR_CMDS,
        description: "Create directory",
        example: "- mkdir:\n    args: build"
    },
    TaskType {
        names: ARCHIVE_CMDS,
        description: "Create or extract archives",
        example: "- archive:\n    args: create bundle.zip"
    },
    // Other tools
    TaskType {
        names: ECHO_CMDS,
        description: "Print a message",
        example: "- echo:\n    args: Building project..."
    },
    TaskType {
        names: EXTERN_CMDS,
        description: "Run external command",
        example: "- extern:\n    args: my-script.sh"
    },
    TaskType {
        names: XFER_CMDS,
        description: "Transfer files to real hardware (CPC WiFi, M4)",
        example: "- xfer:\n    args: upload demo.sna"
    },
    TaskType {
        names: CPR_CMDS,
        description: "Create or modify CPR cartridge files",
        example: "- cpr:\n    args: create game.cpr"
    },
    TaskType {
        names: CSL_CMDS,
        description: "Process CSL files",
        example: "- csl:\n    args: input.csl"
    },
    TaskType {
        names: BNDBUILD_CMDS,
        description: "Run another bndbuild project",
        example: "- bndbuild:\n    args: -t release"
    },
    TaskType {
        names: LOCOMOTIVE_CMDS,
        description: "Process Locomotive BASIC files",
        example: "- locomotive:\n    args: program.bas"
    },
    TaskType {
        names: EMUCTRL_CMDS,
        description: "Control emulator remotely",
        example: "- emuctrl:\n    args: reset"
    },
    TaskType {
        names: Z80PROFILER_CMDS,
        description: "Profile and analyze Z80 code execution",
        example: "- Z80Profiler:\n    args: program.sna"
    }
];

/// Build file top-level keywords
pub const BUILD_KEYWORDS: &[(&str, &str)] = &[
    (
        "targets",
        "Define build targets with dependencies and tasks"
    ),
    ("tasks", "List of tasks to execute for this target"),
    (
        "deps",
        "Dependencies - other targets that must be built first"
    ),
    ("args", "Arguments or configuration for the task"),
    ("env", "Environment variables for the build"),
    ("default", "Default target to build"),
    ("includes", "Include other build files")
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::ALL_APPLICATIONS;

    #[test]
    fn test_task_types_not_empty() {
        assert!(!TASK_TYPES.is_empty());
    }

    #[test]
    fn test_all_tasks_have_names() {
        for task in TASK_TYPES {
            assert!(!task.names.is_empty());
            assert!(!task.description.is_empty());
        }
    }

    #[test]
    fn test_keywords_not_empty() {
        assert!(!BUILD_KEYWORDS.is_empty());
    }

    /// Critical test: Ensures TASK_TYPES covers ALL commands from ALL_APPLICATIONS.
    /// If this fails, it means a task was added to ALL_APPLICATIONS but not to TASK_TYPES.
    /// This prevents desynchronization between the build system and LSP.
    #[test]
    fn test_all_applications_covered_in_task_types() {
        // Collect all command names from ALL_APPLICATIONS
        let mut all_commands = HashSet::new();
        for (cmds, _clearable) in ALL_APPLICATIONS {
            for cmd in *cmds {
                all_commands.insert(*cmd);
            }
        }

        // Collect all command names from TASK_TYPES
        let mut task_type_commands = HashSet::new();
        for task in TASK_TYPES {
            for name in task.names {
                task_type_commands.insert(*name);
            }
        }

        // Find commands in ALL_APPLICATIONS but not in TASK_TYPES
        let missing: Vec<_> = all_commands.difference(&task_type_commands).collect();

        // Find commands in TASK_TYPES but not in ALL_APPLICATIONS (shouldn't happen)
        let extra: Vec<_> = task_type_commands.difference(&all_commands).collect();

        if !missing.is_empty() {
            panic!(
                "TASK_TYPES is missing the following commands that exist in ALL_APPLICATIONS: {:?}\n\
                 Please add TaskType entries for these commands in cpclib-bndbuild/src/lsp.rs",
                missing
            );
        }

        if !extra.is_empty() {
            panic!(
                "TASK_TYPES contains commands that don't exist in ALL_APPLICATIONS: {:?}\n\
                 This shouldn't happen - verify the command names are correct.",
                extra
            );
        }

        assert_eq!(
            all_commands.len(),
            task_type_commands.len(),
            "TASK_TYPES and ALL_APPLICATIONS have different numbers of unique commands"
        );
    }
}

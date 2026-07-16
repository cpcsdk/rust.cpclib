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
    /// CLI usage synopsis of the underlying tool, e.g. `"basm [OPTIONS] [INPUT]"`.
    pub synopsis: &'static str,
    /// A real `cmd:` value taken from an actual build file, verbatim (may reference
    /// Jinja variables or `$<`/`$@` make-style substitutions as found in the source).
    /// Empty when no real-world usage of this task type was found.
    pub example: &'static str
}

/// All available task types with their metadata
pub const TASK_TYPES: &[TaskType] = &[
    // Assemblers
    TaskType {
        names: BASM_CMDS,
        description: "Assemble Z80 source code using basm (built-in assembler)",
        synopsis: "basm [OPTIONS] [INPUT]",
        example: "basm main.asm --snapshot -o main.sna"
    },
    TaskType {
        names: ORGAMS_CMDS,
        description: "Assemble using Orgams assembler syntax",
        synopsis: "orgams [OPTIONS] --src <SRC>",
        example: "orgams --from orgams --src DATA3.O --dst DATA3.BIN"
    },
    TaskType {
        names: RASM_CMDS,
        description: "Assemble using Rasm assembler",
        synopsis: "rasm <inputfile> [options]",
        example: "rasm show.asm -oi show.sna -map"
    },
    TaskType {
        names: SJASMPLUS_CMDS,
        description: "Assemble using SjASMPlus assembler",
        synopsis: "sjasmplus [options] sourcefile(s)",
        example: ""
    },
    TaskType {
        names: VASM_CMDS,
        description: "Assemble using vasm assembler",
        synopsis: "vasm [options] filesm",
        example: ""
    },
    TaskType {
        names: VLINK_CMDS,
        description: "Link object files using vlink linker",
        synopsis: "TODO",
        example: ""
    },
    // Disassemblers
    TaskType {
        names: BDASM_CMDS,
        description: "Disassemble Z80 binary code",
        synopsis: "bdasm [OPTIONS] <INPUT>",
        example: ""
    },
    TaskType {
        names: DISARK_CMDS,
        description: "Disassemble using Disark",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: UZ80_CMDS,
        description: "Disassemble using uz80",
        synopsis: "TODO",
        example: "uz80 -q CHIPNSFZ.S80 -o$@"
    },
    // Documentation
    TaskType {
        names: BASMDOC_CMDS,
        description: "Generate documentation from basm source",
        synopsis: "basmdoc [OPTIONS] --output <OUTPUT> <INPUT>...",
        example: "basmdoc --wildcards src/demosystem/*.asm -o $@"
    },
    // Emulators
    TaskType {
        names: ACE_CMDS,
        description: "Run in ACE emulator",
        synopsis: "TODO",
        example: "ace show.sna"
    },
    TaskType {
        names: CPCEC_CMDS,
        description: "Run in CPCEC emulator",
        synopsis: "TODO",
        example: "cpcec ../../../cpclib/tests/dsk/harley.dsk"
    },
    TaskType {
        names: SUGARBOX_CMDS,
        description: "Run in Sugarbox emulator",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: WINAPE_CMDS,
        description: "Run in WinAPE emulator",
        synopsis: "TODO",
        example: "winape ../../../cpclib/tests/dsk/harley.dsk /A:-CED-.exe"
    },
    TaskType {
        names: RETROVM_CMDS,
        description: "Run in RetroVirtual Machine emulator",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: AMSPIRIT_CMDS,
        description: "Run in AmSpirit emulator",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: CPCEMU_CMDS,
        description: "Run in CPCEmu emulator",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: CPCEMUPOWER_CMDS,
        description: "Run in CPCEmuPower emulator",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: CAPRICEFOREVER_CMDS,
        description: "Run in Caprice Forever emulator",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: CADENCE_CMDS,
        description: "Run in Cadence emulator",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: EMULATOR_1984_CMDS,
        description: "Run in 1984 emulator",
        synopsis: "TODO",
        example: ""
    },
    // Disk and snapshot operations
    TaskType {
        names: DISC_CMDS,
        description: "Create or modify DSK disk images",
        synopsis: "disc_manager <DSK_FILE> [COMMAND]",
        example: "- dsk hello2.dsk format --format data\n- dsk hello2.dsk add HELLO2.BIN"
    },
    TaskType {
        names: SNA_CMDS,
        description: "Create or modify SNA snapshot files",
        synopsis: "createSnapshot [OPTIONS] -- <OUTPUT>",
        example: ""
    },
    TaskType {
        names: CATALOG_CMDS,
        description: "List disk catalog",
        synopsis: "catalog [INPUT_FILE] [COMMAND]",
        example: "catalog {{CATART}} build --output test_cata.DSK"
    },
    TaskType {
        names: IMPDISC_CMDS,
        description: "ImpDisk disk operations",
        synopsis: "TODO",
        example: "impdsk -dsk SHOW.DSK -get -amsdosfile SHOW.SCR"
    },
    TaskType {
        names: HXCFE_CMDS,
        description: "HxC Floppy Emulator file operations",
        synopsis: "hxcfe [OPTIONS]",
        example: "hxcfe -i $< -o $@ --conv HFE"
    },
    TaskType {
        names: RTZX_CMDS,
        description: "Convert tape files to/from TZX format",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: TWO_CDT_CMDS,
        description: "Convert files to CDT tape format",
        synopsis: "TODO",
        example: "- 2cdt -n -r BLIGHT.BAS BLIGHT.BAS $@\n- 2cdt -r BLIGHT.001 BLIGHT.001 $@"
    },
    // Image conversion
    TaskType {
        names: IMG2CPC_CMDS,
        description: "Convert images to Amstrad CPC format",
        synopsis: "img2cpc [OPTIONS] <SOURCE> [COMMAND]",
        example: "img2cpc --mode 1 --overscan $< scr -o mum1.scr --palette mum1.pal"
    },
    TaskType {
        names: CPC2IMG_CMDS,
        description: "Convert Amstrad CPC images to standard formats",
        synopsis: "TODO",
        example: "cpc2img $< $@ --mode 1 --pen0 0 --pen1 26 screen"
    },
    TaskType {
        names: MARTINE_CMDS,
        description: "Convert images using Martine",
        synopsis: "TODO",
        example: "martine -in martine-logo.png -mode 1 -noheader -out martine.scr"
    },
    TaskType {
        names: GRAFX2_CMDS,
        description: "Open/edit images in GrafX2",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: CONVGENERIC_CMDS,
        description: "Generic image conversion",
        synopsis: "TODO",
        example: "convgeneric simpleFonte.png -m 1 -size 12x16 -c 81 -flat -g"
    },
    TaskType {
        names: HIDEUR_CMDS,
        description: "HiDeur image compression",
        synopsis: "hideur [OPTIONS] <INPUT>",
        example: ""
    },
    // Audio tools
    TaskType {
        names: AT_CMDS,
        description: "Play or convert Arkos Tracker files",
        synopsis: "TODO",
        example: "at3"
    },
    TaskType {
        names: AYT_CMDS,
        description: "Process AY sound files",
        synopsis: "TODO",
        example: "ayt --verbose --target CPC \"{{ym_file}}\""
    },
    TaskType {
        names: CHIPNSFX_CMDS,
        description: "ChipNSFX sound effects",
        synopsis: "TODO",
        example: "chipnsfx WINGSOD5.CHP chipnsfz.mus -lchip_song_ -t"
    },
    TaskType {
        names: HSPC_CMDS,
        description: "HSP compiler for music",
        synopsis: "TODO",
        example: ""
    },
    TaskType {
        names: MINY_CMDS,
        description: "Minimise AY music files",
        synopsis: "TODO",
        example: "miny pack \"{{ym_file}}\" \"{{miny_file}}\""
    },
    #[cfg(feature = "fap")]
    TaskType {
        names: FAP_CMDS,
        description: "Convert AY music to FAP format",
        synopsis: "TODO",
        example: "fap wireshark.ym wireshark.fap -2"
    },
    TaskType {
        names: SONG2AKM_CMDS,
        description: "Convert song to AKM format",
        synopsis: "TODO",
        example: "SongToAkm 'Targhan - Crtc - End part.aks' 'Targhan - Crtc - End part.akm'"
    },
    // File operations
    TaskType {
        names: CP_CMDS,
        description: "Copy files",
        synopsis: "cp <FILES> <FILES>...",
        example: "cp invitro.dsk build/invitro.dsk"
    },
    TaskType {
        names: MV_CMDS,
        description: "Move or rename files",
        synopsis: "mv <FILES> <FILES>...",
        example: "mv MAIN.800 MAIN_CRTC{{crtc}}.800"
    },
    TaskType {
        names: RM_CMDS,
        description: "Remove files",
        synopsis: "rm <FILES>...",
        example: "-rm HELLO2.BIN"
    },
    TaskType {
        names: MKDIR_CMDS,
        description: "Create directory",
        synopsis: "mkdir [OPTIONS] <DIRECTORIES>...",
        example: "mkdir --ignore albi"
    },
    TaskType {
        names: ARCHIVE_CMDS,
        description: "Create or extract archives",
        synopsis: "archive [COMMAND]",
        example: "archive create -o HBL.zip dist/*"
    },
    // Other tools
    TaskType {
        names: ECHO_CMDS,
        description: "Print a message",
        synopsis: "TODO",
        example: "echo watched.txt as been modified"
    },
    TaskType {
        names: EXTERN_CMDS,
        description: "Run external command",
        synopsis: "extern command",
        example: "extern vlc \"$<\""
    },
    TaskType {
        names: XFER_CMDS,
        description: "Transfer files to real hardware (CPC WiFi, M4)",
        synopsis: "xfer [CPCADDR] [COMMAND]",
        example: "xfer 192.168.1.27 -y zic.sna"
    },
    TaskType {
        names: CPR_CMDS,
        description: "Create or modify CPR cartridge files",
        synopsis: "cpr [OPTIONS] --cpr1 <INPUT>",
        example: ""
    },
    TaskType {
        names: CSL_CMDS,
        description: "Process CSL files",
        synopsis: "csl [OPTIONS] <FILE>",
        example: ""
    },
    TaskType {
        names: BNDBUILD_CMDS,
        description: "Run another bndbuild project",
        synopsis: "bndbuilder [OPTIONS] [TARGET]...",
        example: "bndbuild -f mdr_intro MDRINTRO.8000"
    },
    TaskType {
        names: LOCOMOTIVE_CMDS,
        description: "Process Locomotive BASIC files",
        synopsis: "locomotive [COMMAND]",
        example: "locomotive encode --input {{CATART}} --output albi/CATART.BAS --header"
    },
    TaskType {
        names: EMUCTRL_CMDS,
        description: "Control emulator remotely",
        synopsis: "cpc [OPTIONS] <COMMAND>",
        example: "cpc --emulator=ace --snapshot=zic.sna run"
    },
    TaskType {
        names: Z80PROFILER_CMDS,
        description: "Profile and analyze Z80 code execution",
        synopsis: "TODO",
        example: ""
    }
];

static TARGET_HELP: &str = "Targets - define the list of targets built by the rules";
static DEPS_HELP: &str = "Dependencies - define the list of dependencies for the rules";
static TASKS_HELP: &str = "Tasks - define the list of tasks to execute for the rules";

/// Build file top-level keywords
pub const BUILD_KEYWORDS: &[(&str, &str)] = &[
    ("target", TARGET_HELP),
    ("tgt", TARGET_HELP),
    ("build", TARGET_HELP),

    ("dep", DEPS_HELP),
    ("dependency", DEPS_HELP),
    ("requires", DEPS_HELP),

    ("cmd", TASKS_HELP),
    ("command", TASKS_HELP),
    ("launch", TASKS_HELP),
    ("run", TASKS_HELP),

    ("help", "Help text for the rule"),
    ("phony", "If true, the target is always rebuilt regardless of file timestamps"),
    ("constraint", "Condition under which this rule applies (e.g. OS or environment constraint)"),

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

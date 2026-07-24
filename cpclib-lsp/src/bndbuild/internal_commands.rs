//! Registry mapping "internal" bndbuild task commands (ones implemented in this
//! workspace with `clap`, as opposed to delegated third-party binaries like rasm,
//! sjasmplus, ace, etc.) to their real `clap::Command` definition.
//!
//! This lets the LSP drive argument/flag completion straight from each tool's
//! actual CLI definition (via `clap_complete::engine::complete`) instead of the
//! hand-maintained `synopsis`/`example` strings in `cpclib_bndbuild::lsp::TASK_TYPES`.
//!
//! Every internal runner already builds its `clap::Command` via
//! `RunnerWithClap::get_clap_command()` with `.no_binary_name(true)` set, except
//! `EmulatorFacadeRunner` (backs the `cpc`/`emu`/`emuctrl`/`emucontrol` aliases) —
//! `normalized()` below papers over that one exception so callers never special-case it.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use cpclib_bndbuild::runners::assembler::{BasmRunner, OrgamsRunner};
use cpclib_bndbuild::runners::disassembler::BdasmRunner;
use cpclib_bndbuild::runners::disc::{CatalogRunner, DiscManagerRunner};
use cpclib_bndbuild::runners::fs::cp::CpRunner;
use cpclib_bndbuild::runners::fs::mkdir::MkdirRunner;
use cpclib_bndbuild::runners::fs::mv::MvRunner;
use cpclib_bndbuild::runners::fs::rm::RmRunner;
use cpclib_bndbuild::runners::xfer::XferRunner;
use cpclib_bndbuild::runners::{
    archive, asmfmt, basmdoc, bndbuild, cpc2img, cprcli, crunch, csl, fade, hideur, hxcfe, img2cpc,
    locomotive, snapshot
};
use cpclib_common::clap::Command;
use cpclib_common::event::CapturingObserver;
use cpclib_runner::emucontrol::EmulatorFacadeRunner;
use cpclib_runner::runner::RunnerWithClap;

/// Normalize a runner's `Command` so callers never have to special-case whether
/// the underlying runner already called `.no_binary_name(true)`.
fn normalized(cmd: Command) -> Command {
    if cmd.is_no_binary_name_set() {
        cmd
    }
    else {
        cmd.no_binary_name(true)
    }
}

type CommandBuilder = fn() -> Command;

/// Canonical name (matches `TASK_TYPES[].names[0]`) -> fresh-`Command` builder.
static INTERNAL_COMMANDS: LazyLock<HashMap<&'static str, CommandBuilder>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, CommandBuilder> = HashMap::new();

    m.insert("basm", || {
        normalized(
            BasmRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("orgams", || {
        normalized(
            OrgamsRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("dsk", || {
        normalized(
            DiscManagerRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("catalog", || {
        normalized(
            CatalogRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("bdasm", || {
        normalized(
            BdasmRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("basmdoc", || {
        normalized(
            basmdoc::BasmDocRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("cp", || {
        normalized(
            CpRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("mv", || {
        normalized(
            MvRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("rm", || {
        normalized(
            RmRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("mkdir", || {
        normalized(
            MkdirRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("archive", || {
        normalized(
            archive::ArchiveRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("xfer", || {
        normalized(
            XferRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("cpr", || {
        normalized(
            cprcli::CprCliRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("csl", || {
        normalized(
            csl::CslRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("crunch", || {
        normalized(
            crunch::CrunchRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("bndbuild", || {
        normalized(
            bndbuild::BndBuildRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("locomotive", || {
        normalized(
            locomotive::LocomotiveRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("img2cpc", || {
        normalized(
            img2cpc::ImgToCpcRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("cpc2img", || {
        normalized(
            cpc2img::CpcToImgRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("fade", || {
        normalized(
            fade::FadeRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("hideur", || {
        normalized(
            hideur::HideurRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("hxcfe", || {
        normalized(
            hxcfe::HxcfeRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("sna", || {
        normalized(
            snapshot::SnapshotRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    m.insert("asmfmt", || {
        normalized(
            asmfmt::AsmFmtRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });
    // Only entry whose Command isn't `no_binary_name(true)` by default.
    m.insert("cpc", || {
        normalized(
            EmulatorFacadeRunner::<CapturingObserver>::default()
                .get_clap_command()
                .clone()
        )
    });

    m
});

/// Built-`Command` cache, keyed by canonical name — building a runner's
/// `Command` (constructing a fresh runner via `::default()` and walking its
/// full `clap::Command` tree of args/subcommands via `get_clap_command()`)
/// isn't free, and `get_command_for` used to redo it from scratch on every
/// single call — every hover/completion request touching a `cmd:` line,
/// i.e. essentially every keystroke there. Mirrors `delegated_help.rs`'s
/// `HELP_CACHE` for the analogous problem one file over. Callers still get
/// an owned `Command` (`clap_complete::engine::complete` needs `&mut`), so
/// each call still pays one `Command::clone()` — what's avoided is
/// reconstructing the runner and its whole arg tree from scratch.
static COMMAND_CACHE: LazyLock<Mutex<HashMap<String, Command>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Returns a normalized `Command` for `name` if it's a known internal
/// command (built once and cached — see `COMMAND_CACHE`). Returns `None`
/// for delegated commands, unknown words, and feature-gated-out commands
/// (e.g. `rtzx`, not compiled into this crate).
///
/// `name` should be the *canonical* name (`TASK_TYPES[].names[0]`), not an
/// arbitrary alias — callers are expected to resolve aliases via `TASK_TYPES`
/// first (hover/completion code already does this).
pub fn get_command_for(name: &str) -> Option<Command> {
    let mut cache = COMMAND_CACHE.lock().unwrap();
    if let Some(cmd) = cache.get(name) {
        return Some(cmd.clone());
    }
    let cmd = INTERNAL_COMMANDS.get(name)?();
    cache.insert(name.to_string(), cmd.clone());
    Some(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basm_is_a_known_internal_command() {
        let cmd = get_command_for("basm");
        assert!(cmd.is_some(), "basm should be a known internal command");
    }

    #[test]
    fn basm_offers_snapshot_flag_completion() {
        let mut cmd = get_command_for("basm").unwrap();
        let candidates = clap_complete::engine::complete(&mut cmd, vec!["--sna".into()], 0, None)
            .expect("completion should not error");
        let values: Vec<String> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .collect();
        assert!(
            values.iter().any(|v| v == "--snapshot"),
            "expected --snapshot among {values:?}"
        );
    }

    #[test]
    fn delegated_and_unknown_commands_are_not_internal() {
        assert!(
            get_command_for("rasm").is_none(),
            "rasm is a delegated command"
        );
        assert!(get_command_for("nonexistent").is_none());
    }

    #[test]
    fn get_command_for_populates_and_reuses_the_command_cache() {
        // Use a name not touched by other tests running concurrently in
        // this same process, so the shared static cache's pre-existing
        // state can't make this test flaky.
        {
            let mut cache = COMMAND_CACHE.lock().unwrap();
            cache.remove("mkdir");
        }
        assert!(!COMMAND_CACHE.lock().unwrap().contains_key("mkdir"));

        let first = get_command_for("mkdir").expect("mkdir is a known internal command");
        assert!(
            COMMAND_CACHE.lock().unwrap().contains_key("mkdir"),
            "get_command_for should populate the cache on first call"
        );

        let second = get_command_for("mkdir").unwrap();
        // Both come from (or were just inserted into) the same cache entry
        // - same name, same set of top-level args either way.
        assert_eq!(
            first.get_arguments().count(),
            second.get_arguments().count()
        );
    }

    #[test]
    fn emuctrl_command_is_normalized() {
        let cmd = get_command_for("cpc").expect("cpc (emuctrl) should be a known command");
        assert!(
            cmd.is_no_binary_name_set(),
            "emuctrl command should be normalized"
        );
    }
}

//! Launches a `.asm` document in a CPC emulator - the "▶ Run in emulator"
//! CodeLens / `cpclib.runAssembly` command.
//!
//! The build is **exactly** the one `F5` performs: `assemble_for_debug` from
//! `cpclib-dap`, with the same parse options, the same `-D` definitions from the
//! project's build rules, and the same snapshot. Running and debugging differ
//! in what happens afterwards, not in what is built, and keeping one function
//! for the build is what stops the two drifting into "it runs but it does not
//! debug".
//!
//! What differs is the emulator: the debugger needs the one that speaks the
//! Debug Adapter Protocol, while running works with whichever the project
//! names (`[asm] run_emulator`, `ace` by default).

use std::sync::Arc;

use cpclib_project::config::AsmConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::bndbuild::command::{OutputLine, StreamingObserver};
use crate::common::document::Document;

/// What `cpclib.runAssembly` did.
pub struct AssemblyRunOutcome {
    pub success: bool,
    pub message: String
}

fn failure(message: impl Into<String>) -> AssemblyRunOutcome {
    AssemblyRunOutcome {
        success: false,
        message: message.into()
    }
}

pub fn run_document_in_emulator(
    document: &Document,
    config: &AsmConfig,
    tx: UnboundedSender<OutputLine>
) -> AssemblyRunOutcome {
    let Ok(entry) = document.uri.to_file_path()
    else {
        return failure("this document has no path on disc to assemble");
    };

    // The same build F5 performs.
    let built = match cpclib_dap::launch::assemble_for_debug(&entry, config) {
        Ok(built) => built,
        Err(problem) => return failure(format!("assembling failed: {problem}"))
    };

    // The emulator takes a file, so the snapshot has to land on disc - unlike
    // the debug path, which serves it over loopback from memory. Named after
    // the source rather than the process so a second run replaces it instead of
    // littering the temporary directory.
    let stem = entry
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "program".to_string());
    let snapshot = std::env::temp_dir().join(format!("cpclib-run-{stem}.sna"));
    if let Err(problem) = std::fs::write(&snapshot, &built.snapshot) {
        return failure(format!("cannot write {}: {problem}", snapshot.display()));
    }

    let Some(snapshot) = cpclib_common::camino::Utf8PathBuf::from_path_buf(snapshot.clone()).ok()
    else {
        return failure("the temporary directory is not utf-8");
    };

    let observer = Arc::new(StreamingObserver::new(tx));
    match cpclib_bndbuild::pipeline::launch_emulator_with_snapshot(
        &snapshot,
        &config.run_emulator,
        &observer
    ) {
        Ok(()) => {
            AssemblyRunOutcome {
                success: true,
                message: format!("Running {stem} in {}.", config.run_emulator)
            }
        },
        Err(problem) => {
            failure(format!(
                "could not launch {}: {problem}",
                config.run_emulator
            ))
        },
    }
}

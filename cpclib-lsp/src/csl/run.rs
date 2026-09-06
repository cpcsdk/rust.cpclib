//! Launches a `.csl` document in a CPC emulator (the "▶ Run in emulator"
//! code lens / `cpclib.runCsl` command) by delegating to
//! `cpclib_bndbuild::pipeline::csl_run`, reusing the same
//! `StreamingObserver` the "▶ Run" bndbuild lens already streams task
//! output through.

use std::sync::Arc;

use camino::Utf8PathBuf;
use cpclib_bndbuild::pipeline::csl_run::{CslRunOutcome, run_csl_in_emulator};
use tokio::sync::mpsc::UnboundedSender;

use crate::bndbuild::command::{OutputLine, StreamingObserver};
use crate::common::config::CslConfig;
use crate::common::document::Document;

pub fn run_document_in_emulator(
    document: &Document,
    config: &CslConfig,
    tx: UnboundedSender<OutputLine>
) -> CslRunOutcome {
    let observer = Arc::new(StreamingObserver::new(tx));
    // The script's own relative disk_insert/snapshot_load/etc. paths mean
    // "next to me" - see `run_csl_in_emulator`'s own doc comment for why
    // this directory (not the OS temp dir the source text would otherwise
    // land in) is what makes that resolve correctly. `None` for an
    // unsaved buffer with no on-disk location.
    let source_dir = document
        .uri
        .to_file_path()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .and_then(|d| Utf8PathBuf::from_path_buf(d).ok());
    run_csl_in_emulator(
        &document.text(),
        source_dir.as_deref(),
        &config.run_emulator,
        &observer
    )
}

//! Launches a `.bas` document in a CPC emulator (the "▶ Run in emulator"
//! code lens / `cpclib.runBasic` command) by delegating to
//! `cpclib_bndbuild::pipeline::basic_run`, reusing the same `StreamingObserver` the
//! "▶ Run" bndbuild lens already streams task output through.

use std::sync::Arc;

use cpclib_bndbuild::pipeline::basic_run::{BasicRunOutcome, run_basic_in_emulator};
use tokio::sync::mpsc::UnboundedSender;

use crate::bndbuild::command::{OutputLine, StreamingObserver};
use crate::common::config::BasicConfig;
use crate::common::document::Document;

pub fn run_document_in_emulator(
    document: &Document,
    config: &BasicConfig,
    tx: UnboundedSender<OutputLine>
) -> BasicRunOutcome {
    let name_hint = document
        .uri
        .to_file_path()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let observer = Arc::new(StreamingObserver::new(tx));
    run_basic_in_emulator(
        &document.text(),
        &name_hint,
        &config.run_emulator,
        &observer
    )
}

use std::sync::mpsc;

use cpclib_bndbuild::cpclib_common::event::EventObserver;
use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserver};

use crate::ratatui_event::RatatuiEvent;

// ─── Channel message ──────────────────────────────────────────────────────────

#[derive(Debug)]
pub(crate) enum RatatuiMessage {
    NewEvent(RatatuiEvent),
    Stdout(String),
    Stderr(String),
}

// ─── Observer ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub(crate) struct BndBuilderRatatuiObserver {
    tx: mpsc::Sender<RatatuiMessage>,
}

impl BndBuilderRatatuiObserver {
    pub(crate) fn new(tx: mpsc::Sender<RatatuiMessage>) -> Self {
        Self { tx }
    }
}

impl EventObserver for BndBuilderRatatuiObserver {
    fn emit_stdout(&self, s: &str) {
        let _ = self.tx.send(RatatuiMessage::Stdout(s.to_owned()));
    }

    fn emit_stderr(&self, s: &str) {
        let _ = self.tx.send(RatatuiMessage::Stderr(s.to_owned()));
    }
}

impl BndBuilderObserver for BndBuilderRatatuiObserver {
    fn update(&self, event: BndBuilderEvent) {
        let _ = self.tx.send(RatatuiMessage::NewEvent(event.into()));
    }
}

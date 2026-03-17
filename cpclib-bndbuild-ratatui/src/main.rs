mod ratatui_event;
mod model;
mod observer;
mod terminal;
mod widgets;
mod app;

use std::sync::mpsc;

use app::BndBuilderRatatui;
use model::BuildPhase;
use observer::{BndBuilderRatatuiObserver, RatatuiMessage};
use terminal::{init_terminal, restore_terminal};
use cpclib_bndbuild::app::BndBuilderApp;
use cpclib_bndbuild::event::{BndBuilderObserved, BndBuilderObserverRc};

fn main() {
    let app = match BndBuilderApp::new() {
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        },
        Ok(None) => return, // help / version already printed
        Ok(Some(a)) => a,
    };

    let (tx, rx) = mpsc::channel::<RatatuiMessage>();

    let mut cmd = match app.command() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        },
    };
    // Drop app so its Arc<observers> clone is released, allowing add_observer to
    // get exclusive mutable access via Arc::get_mut.
    drop(app);

    if !cmd.is_build() {
        // For non-build commands (list, show, dot, ...) bypass the TUI.
        cmd.clear_observers();
        cmd.add_observer(BndBuilderObserverRc::new_default());
        if let Err(e) = cmd.execute() {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // Replace any existing observers with the ratatui channel observer.
    cmd.clear_observers();
    cmd.add_observer(BndBuilderObserverRc::new(BndBuilderRatatuiObserver::new(tx)));

    let term = match init_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to initialise terminal: {e}");
            std::process::exit(1);
        },
    };

    let mut state = BndBuilderRatatui {
        command:         Some(cmd),
        rx,
        rules:           Vec::new(),
        orphans:         Vec::new(),
        phase:           BuildPhase::default(),
        scroll:          None,
        exit:            false,
        confirm_quit:    false,
        pending_aliases: std::collections::HashMap::new(),
        selected_rule:   None,
        build_error:     None,
    };

    let result = state.run(term);
    restore_terminal().unwrap();

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

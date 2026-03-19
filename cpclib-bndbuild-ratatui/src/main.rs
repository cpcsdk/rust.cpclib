mod ratatui_event;
mod model;
mod observer;
mod profile;
mod terminal;
mod timing;
mod widgets;
mod app;

use std::sync::mpsc;

use app::BndBuilderRatatui;
use model::BuildPhase;
use observer::{BndBuilderRatatuiObserver, RatatuiMessage};
use terminal::{init_terminal, restore_terminal};
use timing::TimingCache;
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

    // Capture the top-level build file path before we drop `app` so we can
    // seed `current_build_file` in the TUI state.  This ensures that rule
    // timings for different projects (different -f paths) are stored under
    // distinct keys even when both are invoked from the same working directory.
    let outer_build_file: Option<String> = app.build_file().map(|s| s.to_owned());
    // Resolve the profile path to absolute NOW, before app.command() may change the
    // working directory via decode_from_reader / set_current_dir. That ensures
    // a relative path like "profile.html" points to the user's launch directory.
    let profile_output: Option<String> = app.profile_output().map(|s| {
        let p = std::path::Path::new(s);
        if p.is_absolute() {
            s.to_owned()
        } else {
            std::env::current_dir()
                .ok()
                .map(|d| d.join(p).to_string_lossy().into_owned())
                .unwrap_or_else(|| s.to_owned())
        }
    });

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
        command:             Some(cmd),
        rx,
        rules:               Vec::new(),
        orphans:             Vec::new(),
        phase:               BuildPhase::default(),
        scroll:              None,
        exit:                false,
        confirm_quit:        false,
        pending_aliases:     std::collections::HashMap::new(),
        selected_rule:       None,
        build_error:         None,
        build_started:       None,
        build_duration:      None,
        current_build_file:  outer_build_file,
        build_nesting_depth: 0,
        estimated_finish:    None,
        collapse_uptodate:   false,
        show_help:           false,
        profile_msg:         None,
        profile_output,
        timing_cache: TimingCache::load(
            &std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        ),
    };

    let result = state.run(term);
    restore_terminal().unwrap();

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

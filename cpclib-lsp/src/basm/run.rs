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

/// Which program to build for a document.
pub enum EntryChoice {
    /// Build this file.
    Program(std::path::PathBuf),
    /// Several programs include the document and their answers differ; only
    /// the user can say which is meant.
    Ask(Vec<std::path::PathBuf>)
}

/// The program that a document belongs to.
///
/// A file carrying its own `RUN` is its own program. A file something else
/// includes belongs to that; the include graph is walked to find which. With
/// more than one candidate the choice is the user's - guessing would build and
/// launch a program they did not ask for.
pub fn resolve_entry(document: &std::path::Path) -> EntryChoice {
    match cpclib_project::entry::entry_of(document, None) {
        cpclib_project::entry::Entry::Project(entry) => EntryChoice::Program(entry),
        cpclib_project::entry::Entry::Standalone => EntryChoice::Program(document.to_path_buf()),
        cpclib_project::entry::Entry::Unknown => {
            // `Unknown` covers both "nothing reaches it" and "several do". The
            // roots are what distinguishes them, and the user needs the list
            // either way.
            //
            // `_or_own_dir`, not plain `project_root`: a document with no
            // `.git`/`Makefile`/etc. anywhere above it (a scratch file, an
            // example folder with several related files but no such marker)
            // made this return `None`, which skipped `scan_workspace`
            // entirely - not "found no other candidates", but "never looked"
            // - so a file genuinely `include`d by a sibling in the same
            // directory was silently treated as its own standalone program
            // instead. The fallback (the document's own directory) is enough
            // for `scan_workspace` to at least see files sitting right there.
            let roots = cpclib_project::root::project_root_or_own_dir(document)
                .map(|root| {
                    let workspace = cpclib_project::entry::scan_workspace(&root);
                    cpclib_project::entry::graph_of(&workspace)
                        .run_roots()
                        .to_vec()
                })
                .unwrap_or_default();
            match roots.as_slice() {
                [] => EntryChoice::Program(document.to_path_buf()),
                [only] => EntryChoice::Program(only.clone()),
                _ => EntryChoice::Ask(roots)
            }
        }
    }
}

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
    let Ok(opened) = document.uri.to_file_path()
    else {
        return failure("this document has no path on disc to assemble");
    };

    // The file you are looking at is usually not the program.
    //
    // `events.asm` is included by something that is; assembling it on its own
    // gets an object with no entry point and none of its dependencies. The
    // include graph already knows who reaches it, so the program to build is
    // the root that does - and when several do, that is a question for the
    // user rather than a coin toss.
    let entry = match resolve_entry(&opened) {
        EntryChoice::Program(path) => path,
        EntryChoice::Ask(candidates) => {
            return failure(format!(
                "{} is included by more than one program ({}). \
                 Open the one you mean and run it, or name it with `[asm] entry` \
                 in cpclib-lsp.toml.",
                opened.display(),
                candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
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
    if let Err(problem) = fs_err::write(&snapshot, &built.snapshot) {
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

#[cfg(test)]
mod entry_resolution_tests {
    use super::*;

    fn project(files: &[(&str, &str)]) -> camino_tempfile::Utf8TempDir {
        let dir = camino_tempfile::tempdir().unwrap();
        for (name, text) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, text).unwrap();
        }
        // A project root is what makes the workspace scan - and therefore the
        // include graph - possible.
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    /// The file you are looking at is usually not the program.
    ///
    /// `events.asm` is included by `sna.asm`, which carries the `RUN`.
    /// Assembling `events.asm` on its own gets an object with no entry point
    /// and none of its dependencies.
    #[test]
    fn an_included_file_resolves_to_the_program_that_includes_it() {
        let dir = project(&[
            (
                "sna.asm",
                "\torg 0x8000\n\trun $\n\tinclude \"events.asm\"\n"
            ),
            ("events.asm", "\tnop\n")
        ]);

        let choice = resolve_entry(dir.path().join("events.asm").as_std_path());
        match choice {
            EntryChoice::Program(entry) => {
                assert_eq!(entry.file_name().unwrap(), "sna.asm", "{entry:?}");
            },
            EntryChoice::Ask(candidates) => panic!("one answer expected: {candidates:?}")
        }
    }

    /// A file that is its own program is built as itself.
    #[test]
    fn a_standalone_program_is_its_own_entry() {
        let dir = project(&[("hello.asm", "\torg 0x8000\n\trun $\n\tnop\n")]);
        let path = dir.path().join("hello.asm");

        match resolve_entry(path.as_std_path()) {
            EntryChoice::Program(entry) => assert_eq!(entry.file_name().unwrap(), "hello.asm"),
            EntryChoice::Ask(candidates) => panic!("{candidates:?}")
        }
    }

    /// Two programs reach it: which one is meant is genuinely the user's to
    /// say, and guessing would build and launch one they did not ask for.
    #[test]
    fn a_file_two_programs_include_is_a_question_for_the_user() {
        let dir = project(&[
            (
                "one.asm",
                "\torg 0x8000\n\trun $\n\tinclude \"shared.asm\"\n"
            ),
            (
                "two.asm",
                "\torg 0x9000\n\trun $\n\tinclude \"shared.asm\"\n"
            ),
            ("shared.asm", "\tnop\n")
        ]);

        match resolve_entry(dir.path().join("shared.asm").as_std_path()) {
            EntryChoice::Ask(candidates) => {
                assert_eq!(candidates.len(), 2, "{candidates:?}");
                let names: Vec<String> = candidates
                    .iter()
                    .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                    .collect();
                assert!(names.contains(&"one.asm".to_string()), "{names:?}");
                assert!(names.contains(&"two.asm".to_string()), "{names:?}");
            },
            EntryChoice::Program(entry) => panic!("should have asked: {entry:?}")
        }
    }
}

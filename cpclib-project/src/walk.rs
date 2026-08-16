//! One way to walk a workspace, shared by everything that needs to.
//!
//! Four features here scanned the file tree independently - the include graph,
//! symbol search, bndbuild's `{% include %}` graph, and the peephole
//! optimizer's entry resolution - each with its own copy of a "directories not
//! worth descending into" list. They now all come through here.
//!
//! Two things changed with the move from `walkdir` to [`ignore`]:
//!
//! * **`.gitignore` is respected.** Generated assembly, build output and
//!   anything else the project has deliberately excluded from version control
//!   stops being read at all. That is right for build output and would be
//!   wrong for a checked-out-but-ignored source; the tradeoff is worth it
//!   because the former is common and the latter is not, and because reading
//!   generated `.asm` was actively harmful - a generated file with its own
//!   `RUN` could be picked as a project's entry point.
//! * **The walk is parallel.** `ignore`'s `build_parallel` spreads the `stat`
//!   and directory reads across threads, which is where the speed-up for large
//!   projects comes from.
//!
//! Results are sorted by path before being handed back, so a parallel walk
//! stays as reproducible as the sequential one it replaces.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Directories never worth descending into: VCS metadata and build output can
/// be huge and are never where hand-written sources live.
///
/// This stays alongside `.gitignore` handling rather than being replaced by
/// it, because a project need not have git at all - and `target/` is worth
/// skipping either way.
fn is_ignored_dir(name: &str) -> bool {
    matches!(name, ".git" | ".hg" | ".svn" | "target" | "node_modules")
}

fn builder(root: &Path) -> ignore::WalkBuilder {
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        // `.gitignore` applies whether or not this is a git checkout: the LSP
        // is regularly pointed at an exported or vendored copy of a project,
        // and the file still says what is generated.
        .require_git(false)
        // Hidden files are *not* skipped - dotted directories worth skipping
        // are named explicitly below, and a hidden source file is still a
        // source file.
        .hidden(false)
        .filter_entry(|entry| {
            !entry.file_type().is_some_and(|t| t.is_dir())
                || !entry.file_name().to_str().is_some_and(is_ignored_dir)
        });
    builder
}

/// Every file under `root`, walked in parallel and returned in path order.
///
/// Directories are not included: every caller here wants files.
pub fn files_under(root: &Path) -> Vec<ignore::DirEntry> {
    let found = Mutex::new(Vec::new());
    builder(root).build_parallel().run(|| {
        Box::new(|entry| {
            if let Ok(entry) = entry
                && entry.file_type().is_some_and(|t| t.is_file())
            {
                found.lock().unwrap().push(entry);
            }
            ignore::WalkState::Continue
        })
    });

    let mut found = found.into_inner().unwrap();
    found.sort_by(|a, b| a.path().cmp(b.path()));
    found
}

/// [`files_under`] across several roots, de-duplicated by path - workspace
/// roots can nest.
pub fn files_under_all(roots: &[PathBuf]) -> Vec<ignore::DirEntry> {
    let mut found: Vec<ignore::DirEntry> = roots.iter().flat_map(|r| files_under(r)).collect();
    found.sort_by(|a, b| a.path().cmp(b.path()));
    found.dedup_by(|a, b| a.path() == b.path());
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, text).unwrap();
    }

    fn names(root: &Path) -> Vec<String> {
        files_under(root)
            .iter()
            .map(|e| {
                e.path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    #[test]
    fn a_gitignored_source_is_not_walked_even_without_a_git_directory() {
        let dir = camino_tempfile::tempdir().unwrap();
        let root = dir.path().as_std_path();
        write(&root.join(".gitignore"), "generated/\n*.gen.asm\n");
        write(&root.join("main.asm"), "  ret\n");
        write(&root.join("sprite.gen.asm"), "  ret\n");
        write(&root.join("generated/tiles.asm"), "  ret\n");

        let found = names(root);
        assert!(found.contains(&"main.asm".to_owned()));
        assert!(
            !found.iter().any(|n| n.ends_with(".gen.asm")),
            "a gitignored file must not be walked: {found:?}"
        );
        assert!(
            !found.iter().any(|n| n.starts_with("generated/")),
            "a gitignored directory must not be descended into: {found:?}"
        );
    }

    #[test]
    fn build_output_and_vcs_metadata_are_skipped_without_any_ignore_file() {
        let dir = camino_tempfile::tempdir().unwrap();
        let root = dir.path().as_std_path();
        write(&root.join("main.asm"), "  ret\n");
        write(&root.join("target/debug/junk.asm"), "  ret\n");
        write(&root.join(".git/config"), "");
        write(&root.join("node_modules/pkg/index.asm"), "  ret\n");

        assert_eq!(names(root), vec!["main.asm".to_owned()]);
    }

    #[test]
    fn results_come_back_in_path_order_despite_the_parallel_walk() {
        let dir = camino_tempfile::tempdir().unwrap();
        let root = dir.path().as_std_path();
        for name in ["c.asm", "a.asm", "b.asm"] {
            write(&root.join(name), "  ret\n");
        }
        assert_eq!(
            names(root),
            vec!["a.asm".to_owned(), "b.asm".to_owned(), "c.asm".to_owned()]
        );
    }
}

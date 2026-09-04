//! Where a project starts, and how a file finds the files it includes.
//!
//! Four features used to walk up the ancestor tree looking for the same
//! markers, in three near-identical loops that had drifted apart in their edge
//! cases (one returned `None` at the filesystem root, another the document's
//! own directory, a third stopped one level short). They are one walk now, and
//! the differences that mattered are named in the function that wants them.

use std::path::{Path, PathBuf};

/// Directories whose presence means "this is the top of a project".
///
/// Deliberately not `.asm`-specific: a CPC project is usually a git checkout
/// with a `Makefile` or a `build.bnd`, and the marker that actually holds is
/// the version control directory.
pub const PROJECT_ROOT_MARKERS: &[&str] = &[
    ".git",
    ".hg",
    "Cargo.toml",
    "Cargo.lock",
    "Makefile",
    "makefile"
];

pub const INCLUDE_DIRECTIVES: &[&str] = &["INCLUDE", "INCBIN", "BINCLUDE"];

/// Whether `dir` looks like the top of a project.
pub fn is_project_root(dir: &Path) -> bool {
    PROJECT_ROOT_MARKERS.iter().any(|m| dir.join(m).exists())
}

/// The directories to search from `file`, nearest first, up to and including
/// the project root (or the filesystem root if there is no marker anywhere).
///
/// This is the one walk; everything else below is a question asked of it.
pub fn ancestor_directories(file: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let Some(mut dir) = file.parent().map(Path::to_path_buf)
    else {
        return dirs;
    };
    loop {
        let at_root = is_project_root(&dir);
        dirs.push(dir.clone());
        if at_root {
            break;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break
        }
    }
    dirs
}

/// The nearest ancestor of `file` that looks like a project root, or `None`
/// when there is no marker anywhere above it.
///
/// `None` means "this file is not in a project I can identify" - which callers
/// resolving an entry point must treat as unknown rather than guessing.
pub fn project_root(file: &Path) -> Option<PathBuf> {
    let mut dir = file.parent()?.to_path_buf();
    loop {
        if is_project_root(&dir) {
            return Some(dir);
        }
        {
            let parent = dir.parent()?;
            dir = parent.to_path_buf()
        }
    }
}

/// [`project_root`], falling back to the file's own directory.
///
/// The right answer for a *search base* - a lone `.asm` outside any project
/// still has a directory worth searching - and the wrong one for entry
/// resolution, which is why the two are separate functions rather than one
/// with a flag.
pub fn project_root_or_own_dir(file: &Path) -> Option<PathBuf> {
    let own_dir = file.parent()?.to_path_buf();
    Some(project_root(file).unwrap_or(own_dir))
}

/// Resolve `filename` as `from_file` would: try each ancestor directory as a
/// base, stopping at the project root.
pub fn resolve_include_path(filename: &str, from_file: &Path) -> Option<PathBuf> {
    ancestor_directories(from_file)
        .into_iter()
        .map(|dir| dir.join(filename))
        .find(|candidate| candidate.exists())
}

/// Every filename referenced by an `INCLUDE`/`INCBIN`/`BINCLUDE` directive in
/// `text`, in document order.
///
/// A best-effort text scan, line-anchored so a whole file is read in one pass.
/// It is used to build the include graph, where missing an edge costs a worse
/// answer rather than a wrong one.
pub fn extract_include_filenames(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let upper = trimmed.to_uppercase();

        let Some(directive) = INCLUDE_DIRECTIVES.iter().find(|d| {
            upper == **d
                || upper.starts_with(&format!("{d} "))
                || upper.starts_with(&format!("{d}\t"))
        })
        else {
            continue;
        };

        let after = &trimmed[directive.len()..];
        let Some(q1) = after.find('"')
        else {
            continue;
        };
        let Some(q2_rel) = after[q1 + 1..].find('"')
        else {
            continue;
        };
        out.push(after[q1 + 1..q1 + 1 + q2_rel].to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three walks used to disagree here; this pins what each answers.
    #[test]
    fn a_file_outside_any_project_has_a_search_base_but_no_root() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let file = tmp.path().join("lone.asm");
        std::fs::write(&file, "").unwrap();
        let file = file.as_std_path();

        // No marker anywhere above a temp dir.
        assert_eq!(project_root(file), None, "entry resolution must not guess");
        assert_eq!(
            project_root_or_own_dir(file).as_deref(),
            file.parent(),
            "a search base falls back to the file's own directory"
        );
    }

    #[test]
    fn the_walk_stops_at_the_marker() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        let deep = root.join("src").join("effects");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::create_dir(root.join(".git")).unwrap();
        let file = deep.join("a.asm");
        std::fs::write(&file, "").unwrap();

        assert!(is_project_root(root.as_std_path()));
        assert_eq!(
            project_root(file.as_std_path()).unwrap(),
            std::fs::canonicalize(root.as_std_path()).unwrap_or(root.as_std_path().to_path_buf())
        );

        let dirs = ancestor_directories(file.as_std_path());
        assert_eq!(dirs.first().unwrap(), deep.as_std_path());
        assert_eq!(
            dirs.last().unwrap(),
            root.as_std_path(),
            "the root is included, and nothing above it"
        );
    }

    #[test]
    fn an_include_resolves_against_the_nearest_ancestor_that_has_it() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("shared.asm"), "").unwrap();
        let from = root.join("src").join("main.asm");
        std::fs::write(&from, "").unwrap();

        assert_eq!(
            resolve_include_path("shared.asm", from.as_std_path()).unwrap(),
            root.join("shared.asm").as_std_path()
        );
        assert!(resolve_include_path("absent.asm", from.as_std_path()).is_none());
    }

    #[test]
    fn include_filenames_are_scanned_in_order() {
        let text = "\tinclude \"a.asm\"\n\tld a,0\n\tINCBIN \"b.bin\"\n\t; include \"c.asm\"\n";
        assert_eq!(extract_include_filenames(text), vec!["a.asm", "b.bin"]);
    }
}

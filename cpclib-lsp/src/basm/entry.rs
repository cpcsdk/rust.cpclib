//! Which file is *this* file's program?
//!
//! Address-aware analysis (`jp2jr`, via `reachableByJr`) needs the addresses a
//! real build produces, and most files do not produce them on their own. In
//! `birthtro`, `src/demo_code.asm` is `include`d by `src/sna.asm`, which is
//! where the constants, the `range 0x0300, ...` memory map and the `run`
//! directive live. Assembled alone, `demo_code.asm` is a different, shorter
//! program at a different base - and a `jp` measured against it reported 127
//! bytes where the real build measured 146.
//!
//! So the question is not "which file has a `RUN`" - `birthtro` has fourteen,
//! `src/sna.asm` plus thirteen standalone test programs under `tests/`. It is
//! **which `RUN`-bearing file reaches this one through the include graph**.
//!
//! Everything here is best-effort and fails toward [`Entry::Unknown`], whose
//! only consequence is that address-aware rules stay quiet - the same
//! fail-closed answer the engine already gives when it has no resolver at all.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::Url;

use cpclib_tokens::symbols::SymbolsTableTrait;

use super::definition::{extract_include_filenames, resolve_include_path};

/// What to assemble in order to get real addresses for a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Entry {
    /// Assemble this other file; the document is somewhere inside its include
    /// graph.
    Project(PathBuf),
    /// The document is its own program - it carries a `RUN` and nothing
    /// includes it. Assembling it directly is correct, which is what the LSP
    /// already did for every file.
    Standalone,
    /// No usable answer: nothing reaches it, or *several* programs do and
    /// their addresses genuinely differ. Callers must not use addresses.
    Unknown
}

/// Does this text define the program's entry point?
///
/// A line-anchored text scan, deliberately matching how
/// [`extract_include_filenames`] works: parsing every assembly file in a
/// workspace to answer this would cost far more than the question is worth,
/// and both directions of error are safe - a missed `RUN` loses coverage, a
/// spurious one is filtered out again by the reachability check below.
fn declares_run(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        let mut words = trimmed.split_whitespace();
        words
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case("run"))
            && words.next().is_some()
    })
}

/// The project root for `doc_uri`: the highest ancestor directory still inside
/// the project, found by the same markers `resolve_include_path` stops at.
fn project_root(doc_uri: &Url) -> Option<PathBuf> {
    let path = doc_uri.to_file_path().ok()?;
    let mut dir = path.parent()?.to_path_buf();
    loop {
        if super::definition::is_project_root(&dir) {
            return Some(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return None
        }
    }
}

/// Every `.asm` file under `root`, with its text.
fn workspace_sources(root: &Path) -> Vec<(PathBuf, String)> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "asm"))
        .filter_map(|e| {
            let path = std::fs::canonicalize(e.path()).ok()?;
            let text = std::fs::read_to_string(&path).ok()?;
            Some((path, text))
        })
        .collect()
}

/// Resolve the entry for `doc_uri`.
///
/// `configured` is `[asm] entry` from `cpclib-lsp.toml`, taken as read when
/// set - it exists precisely so a user can settle the ambiguous case below.
pub(super) fn entry_for(doc_uri: &Url, configured: Option<&str>) -> Entry {
    let Some(root) = project_root(doc_uri)
    else {
        return Entry::Unknown;
    };
    let Ok(document) = doc_uri
        .to_file_path()
        .and_then(|p| std::fs::canonicalize(p).map_err(|_| ()))
    else {
        return Entry::Unknown;
    };

    if let Some(configured) = configured {
        let path = root.join(configured);
        return match std::fs::canonicalize(&path) {
            Ok(path) if path == document => Entry::Standalone,
            Ok(path) => Entry::Project(path),
            // A configured entry that does not exist is a mistake worth not
            // papering over with a guess.
            Err(_) => Entry::Unknown
        };
    }

    let sources = workspace_sources(&root);

    // Who includes whom. A file's own directory drives include resolution, so
    // each edge is resolved from the *including* file.
    let mut includes: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for (path, text) in &sources {
        let Ok(uri) = Url::from_file_path(path)
        else {
            continue;
        };
        let targets = extract_include_filenames(text)
            .iter()
            .filter_map(|name| resolve_include_path(name, &uri))
            .filter_map(|p| std::fs::canonicalize(p).ok())
            .collect();
        includes.insert(path.clone(), targets);
    }

    // Which RUN-bearing files reach the document.
    let mut roots: Vec<PathBuf> = sources
        .iter()
        .filter(|(_, text)| declares_run(text))
        .filter(|(path, _)| reaches(path, &document, &includes))
        .map(|(path, _)| path.clone())
        .collect();
    roots.sort();
    roots.dedup();

    match roots.as_slice() {
        [only] if *only == document => Entry::Standalone,
        [only] => Entry::Project(only.clone()),
        // Several programs include this file, and its addresses differ in each
        // of them. There is no right one to pick, so pick none - `[asm] entry`
        // is how a user resolves it deliberately.
        _ => Entry::Unknown
    }
}

/// Whether `from` reaches `target` through the include graph, itself included.
fn reaches(from: &Path, target: &Path, includes: &HashMap<PathBuf, Vec<PathBuf>>) -> bool {
    let mut seen: HashSet<&Path> = HashSet::new();
    let mut stack = vec![from];
    while let Some(current) = stack.pop() {
        if current == target {
            return true;
        }
        if !seen.insert(current) {
            continue;
        }
        if let Some(next) = includes.get(current) {
            stack.extend(next.iter().map(|p| p.as_path()));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        std::fs::canonicalize(&path).unwrap()
    }

    /// A marker so `project_root` stops here rather than walking out into the
    /// real filesystem.
    fn project(dir: &Path) {
        std::fs::create_dir_all(dir.join(".git")).unwrap();
    }

    /// The reported case in miniature: the document is only ever included, and
    /// the file including it is the one carrying `RUN`.
    #[test]
    fn an_included_file_resolves_to_the_run_bearing_file_that_includes_it() {
        let tmp = camino_tempfile::tempdir().unwrap();
        project(tmp.path().as_std_path());
        let sna = write(
            tmp.path().as_std_path(),
            "sna.asm",
            "    run demo_start\n    include \"demo_code.asm\"\n"
        );
        let code = write(tmp.path().as_std_path(), "demo_code.asm", "demo_start\n    ret\n");

        let uri = Url::from_file_path(&code).unwrap();
        assert_eq!(entry_for(&uri, None), Entry::Project(sna));
    }

    /// A standalone test program is its own entry, so nothing that worked
    /// before regresses.
    #[test]
    fn a_file_carrying_its_own_run_is_standalone() {
        let tmp = camino_tempfile::tempdir().unwrap();
        project(tmp.path().as_std_path());
        let test = write(tmp.path().as_std_path(), "test1.asm", "    run start\nstart\n    ret\n");

        let uri = Url::from_file_path(&test).unwrap();
        assert_eq!(entry_for(&uri, None), Entry::Standalone);
    }

    /// The case that must refuse: a shared library reached from two programs
    /// has different addresses in each, and guessing one would be wrong half
    /// the time.
    #[test]
    fn a_file_reachable_from_two_programs_has_no_single_entry() {
        let tmp = camino_tempfile::tempdir().unwrap();
        project(tmp.path().as_std_path());
        write(
            tmp.path().as_std_path(),
            "one.asm",
            "    run start\n    include \"shared.asm\"\n"
        );
        write(
            tmp.path().as_std_path(),
            "two.asm",
            "    run start\n    include \"shared.asm\"\n"
        );
        let shared = write(tmp.path().as_std_path(), "shared.asm", "start\n    ret\n");

        let uri = Url::from_file_path(&shared).unwrap();
        assert_eq!(entry_for(&uri, None), Entry::Unknown);
    }

    /// ...and the configured entry is how a user settles it.
    #[test]
    fn a_configured_entry_wins_over_the_search() {
        let tmp = camino_tempfile::tempdir().unwrap();
        project(tmp.path().as_std_path());
        let one = write(
            tmp.path().as_std_path(),
            "one.asm",
            "    run start\n    include \"shared.asm\"\n"
        );
        write(
            tmp.path().as_std_path(),
            "two.asm",
            "    run start\n    include \"shared.asm\"\n"
        );
        let shared = write(tmp.path().as_std_path(), "shared.asm", "start\n    ret\n");

        let uri = Url::from_file_path(&shared).unwrap();
        assert_eq!(entry_for(&uri, Some("one.asm")), Entry::Project(one));
    }

    /// A file nothing includes and which declares no program of its own.
    #[test]
    fn an_orphan_file_has_no_entry() {
        let tmp = camino_tempfile::tempdir().unwrap();
        project(tmp.path().as_std_path());
        let orphan = write(tmp.path().as_std_path(), "orphan.asm", "    nop\n");

        let uri = Url::from_file_path(&orphan).unwrap();
        assert_eq!(entry_for(&uri, None), Entry::Unknown);
    }

    #[test]
    fn run_is_recognised_whatever_its_case_and_spacing() {
        assert!(declares_run("    run demo_start\n"));
        assert!(declares_run("\tRUN start\n"));
        assert!(!declares_run("    run\n"), "a bare RUN names no entry");
        assert!(!declares_run("    running_total equ 3\n"));
        assert!(!declares_run("    ret\n"));
    }
}

/// Addresses for `document`, obtained by assembling whatever program actually
/// contains it.
///
/// `None` means "no trustworthy addresses", which every caller must treat as
/// "make no address-aware suggestion". That covers an ambiguous or missing
/// entry, an entry that will not assemble, and - importantly - a document
/// whose buffer no longer matches the file on disk.
pub(super) struct ProjectAddresses {
    pub env: std::sync::Arc<cpclib_asm::assembler::Env>,
    /// The document's own canonical path, which is the key its tokens are
    /// recorded under in `env`.
    pub document: PathBuf
}

/// The newest modification time across every source that could affect the
/// project assemble.
///
/// Cheap enough to compute on every request (a `stat` per file) and it changes
/// exactly when a rebuild would place code differently - which is what makes
/// caching the assemble safe rather than merely fast.
pub(super) fn sources_fingerprint(root: &Path) -> u128 {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().is_some_and(|ext| {
                ext == "asm" || ext == "bnd" || ext == "build"
            })
        })
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .filter_map(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .max()
        .unwrap_or(0)
}

/// The project root for a path, exposed so callers can fingerprint it.
pub(super) fn root_of(doc_uri: &Url) -> Option<PathBuf> {
    project_root(doc_uri)
}

/// Assemble `entry` and hand back addresses usable for `document`.
///
/// The staleness check is the load-bearing part. Addresses are recorded
/// against *byte offsets in the file as assembled*; if the editor buffer has
/// unsaved changes those offsets have shifted, and every answer would be
/// confidently wrong. Being quiet while the user types is the right trade.
pub(super) fn assemble_entry(
    entry: &Path,
    case_sensitive: bool,
    disabled: enumflags2::BitFlags<cpclib_asm::WarningCategory>
) -> Option<cpclib_asm::assembler::Env> {
    let entry_text = std::fs::read_to_string(entry).ok()?;
    let entry_uri = Url::from_file_path(entry).ok()?;

    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    parse.set_disabled_warning_categories(disabled);
    for dir in super::definition::ancestor_search_directories(&entry_uri) {
        let _ = parse.add_search_path(dir);
    }
    let builder = parse
        .clone()
        .context_builder()
        .set_current_filename(entry.to_str()?);
    let listing = cpclib_asm::parser::parse_z80_with_context_builder(&entry_text, builder).ok()?;

    let mut assemble = cpclib_asm::AssemblingOptions::default();
    assemble.set_dry_run(true);
    assemble.set_case_sensitive(case_sensitive);
    assemble.set_record_token_addresses(true);
    for category in disabled.iter() {
        assemble.disable_warning_category(category);
    }

    // Assembling the entry is still not *building* it. A project passes
    // symbols on the command line (`basm ... -DMUSIC_CFG=\"...\"`), and
    // `sna.asm` reaches `include MUSIC_CFG` - which resolves to nothing at
    // all without them, so the assemble fails and no address is trustworthy.
    // Read them from the build rule rather than asking the user to restate
    // them somewhere else, so they cannot drift apart. See `build_defs`.
    let definitions = super::build_defs::definitions_for_entry(entry);
    for (name, value) in &definitions.values {
        let value = match value.parse::<i32>() {
            Ok(number) => cpclib_tokens::ExprResult::from(number),
            Err(_) => cpclib_tokens::ExprResult::String(value.as_str().into())
        };
        let _ = assemble.symbols_mut().assign_symbol_to_value(name.as_str(), value);
    }
    let options = cpclib_asm::EnvOptions::new(
        parse,
        assemble,
        std::sync::Arc::new(cpclib_common::event::DiscardObserver)
    );

    // Only a *complete* assemble is usable: a half-laid-out program's
    // addresses describe something that never existed.
    Some(
        cpclib_asm::assembler::visit_tokens_all_passes_with_options(&listing, options)
            .ok()?
            .1
    )
}

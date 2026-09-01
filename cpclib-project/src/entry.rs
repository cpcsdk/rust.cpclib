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

use cpclib_tokens::ListingElement;
use cpclib_tokens::symbols::SymbolsTableTrait;

use crate::root::{extract_include_filenames, resolve_include_path};

/// What to assemble in order to get real addresses for a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
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
/// Asked of the parser, not of the raw lines. A line scan gets this wrong in
/// both directions: basm allows several instructions per line, so `nop : run
/// start` declares an entry that no line-anchored scan sees, and a `RUN`
/// inside a block comment is one that it wrongly believes.
///
/// Parsing every assembly file in a workspace would cost far more than the
/// question is worth, so a substring prefilter guards it: a file whose text
/// does not contain `run` at all cannot hold a `RUN` directive, which makes
/// the prefilter incapable of a false negative while keeping the common case
/// a scan rather than a parse.
fn declares_run(text: &str) -> bool {
    if !contains_run_word(text) {
        return false;
    }
    let Ok(listing) = cpclib_asm::parser::parse_z80_str(text)
    else {
        // A file that does not parse standalone is common enough - fragments
        // meant to be `include`d lean on macros their includer defines. Fall
        // back to the line scan this used to be, which is wrong in the ways
        // described above but never worse than what was here before.
        return text.lines().any(|line| {
            let mut words = line.split_whitespace();
            words.next().is_some_and(|w| w.eq_ignore_ascii_case("run")) && words.next().is_some()
        });
    };
    cpclib_asm::flatten::flatten_for_analysis(listing.iter()).any(|t| t.is_run())
}

/// Case-insensitive `run` substring, without allocating a lowercased copy of
/// the file.
fn contains_run_word(text: &str) -> bool {
    text.as_bytes()
        .windows(3)
        .any(|w| w.eq_ignore_ascii_case(b"run"))
}

/// The project root for `doc_uri`: the highest ancestor directory still inside
/// the project, found by the same markers `resolve_include_path` stops at.
fn project_root(document: &Path) -> Option<PathBuf> {
    let mut dir = document.parent()?.to_path_buf();
    loop {
        if crate::root::is_project_root(&dir) {
            return Some(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return None
        }
    }
}

/// Everything one traversal of a project needs to answer both questions asked
/// of it: *which file is the entry* (needs the sources) and *has anything
/// changed* (needs their timestamps).
///
/// One walk, because these were two - `entry_for` reading every `.asm` and
/// `sources_fingerprint` stat-ing them all again, from four call sites, on
/// every request.
pub struct Workspace {
    pub sources: Vec<(PathBuf, String)>,
    /// Newest modification time across every source that could affect a
    /// build - `.asm` and the build files themselves. Changes exactly when a
    /// rebuild would lay code out differently.
    pub fingerprint: u128
}

/// Every file under `root` a rebuild would read, paired with whether it is an
/// assembly source. `.bnd`/`.build` count for the fingerprint - changing a
/// build rule changes `-D` values, which changes addresses - but are never
/// sources.
fn build_affecting_files(root: &Path) -> impl Iterator<Item = (ignore::DirEntry, bool)> {
    crate::walk::files_under(root)
        .into_iter()
        .filter_map(|entry| {
            let extension = entry.path().extension().and_then(|e| e.to_str())?;
            let is_source = extension == "asm";
            (is_source || extension == "bnd" || extension == "build").then_some((entry, is_source))
        })
}

fn newest_mtime(entry: &ignore::DirEntry) -> u128 {
    entry
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// How long a computed fingerprint is trusted before `fingerprint_of` walks
/// the tree again for the same root - see [`FINGERPRINT_CACHE`]'s own doc
/// comment for why this exists at all.
const FINGERPRINT_CACHE_TTL: std::time::Duration = std::time::Duration::from_millis(300);

/// `fingerprint_of`'s own memo, keyed by root: `(when it was computed, the
/// value)`.
///
/// `fingerprint_of` is the cache *key* for `env_cache`/`address_source_cache`
/// (see `cpclib-lsp`'s `workspace_fingerprint_of`) - which means it now runs
/// on every diagnostics/hover/peephole computation for every open document,
/// not just on a genuine cache miss, since you have to compute the
/// fingerprint before you can even tell whether it changed. Real, measured
/// consequence: opening a workspace-restore burst of N previously-open tabs
/// (VS Code sends every `didOpen` within milliseconds of each other) fires N
/// full recursive walks of the *same, unchanged* project root back to back,
/// for a value that provably has not changed between them. Each walk alone
/// is cheap; N of them queued through `tower-lsp`'s own concurrency limit is
/// not. A short-lived memo collapses a burst like that into one real walk -
/// long enough to matter for a startup burst (milliseconds apart), short
/// enough that a genuine edit to an included file is picked up within a
/// fraction of a second, not stale for any duration a person would notice.
static FINGERPRINT_CACHE: std::sync::OnceLock<dashmap::DashMap<PathBuf, (std::time::Instant, u128)>> =
    std::sync::OnceLock::new();

/// The fingerprint alone, without reading a single file.
///
/// This is the cache *key* for everything below it, so it has to be the cheap
/// half of the walk: `stat` per candidate, no `read_to_string`. When it matches
/// what a previous answer was computed under, nothing else runs at all.
pub fn fingerprint_of(root: &Path) -> u128 {
    let cache = FINGERPRINT_CACHE.get_or_init(dashmap::DashMap::new);
    if let Some(cached) = cache.get(root)
        && cached.0.elapsed() < FINGERPRINT_CACHE_TTL
    {
        return cached.1;
    }
    let value = build_affecting_files(root)
        .map(|(entry, _)| newest_mtime(&entry))
        .max()
        .unwrap_or(0);
    cache.insert(root.to_path_buf(), (std::time::Instant::now(), value));
    value
}

pub fn scan_workspace(root: &Path) -> Workspace {
    let mut sources = Vec::new();
    let mut fingerprint = 0u128;

    for (entry, is_source) in build_affecting_files(root) {
        fingerprint = fingerprint.max(newest_mtime(&entry));

        if is_source
            && let Ok(path) = std::fs::canonicalize(entry.path())
            && let Ok(text) = std::fs::read_to_string(&path)
        {
            sources.push((path, text));
        }
    }

    Workspace {
        sources,
        fingerprint
    }
}

/// [`entry_for`] with a workspace scan of its own, for callers that ask once
/// and have no fingerprint to reuse.
pub fn entry_of(document: &Path, configured: Option<&str>) -> Entry {
    match project_root(document) {
        Some(root) => entry_for(document, configured, &scan_workspace(&root)),
        None => Entry::Unknown
    }
}

/// Resolve the entry for `doc_uri`.
///
/// `configured` is `[asm] entry` from `cpclib-lsp.toml`, taken as read when
/// set - it exists precisely so a user can settle the ambiguous case below.
pub fn entry_for(document: &Path, configured: Option<&str>, workspace: &Workspace) -> Entry {
    let Some(root) = project_root(document)
    else {
        return Entry::Unknown;
    };
    entry_in_graph(document, configured, &root, &graph_of(workspace))
}

/// Everything resolving an entry needs that does **not** depend on which
/// document is asking: who includes whom, and which files carry `RUN`.
///
/// Split out because it was being rebuilt per document, and it is the
/// expensive half by a wide margin - one graph reads every source in the
/// project and parses each one that might hold a `RUN`. Answering for N
/// documents that way is N times the work of answering for one, which turned
/// a workspace-wide scan quadratic. Held by
/// `AssemblyAnalyzer::project_graph_cached` against the project fingerprint,
/// so it is built once per change instead.
pub struct ProjectGraph {
    /// Edges resolved from the *including* file, since a file's own directory
    /// drives include resolution.
    includes: HashMap<PathBuf, Vec<PathBuf>>,
    /// The files that declare the program's entry point.
    run_roots: Vec<PathBuf>
}

impl ProjectGraph {
    /// The files that declare a `RUN`, i.e. the candidate entry points.
    ///
    /// Exposed so a caller that has no particular document in hand - a debug
    /// session started from a build rule, say - can still find the program the
    /// project builds.
    pub fn run_roots(&self) -> &[PathBuf] {
        &self.run_roots
    }

    /// The single entry point of this project, when there is exactly one.
    ///
    /// `None` when the project declares none, or several: with more than one
    /// `RUN` the answer genuinely depends on which program is meant, and
    /// guessing would silently debug the wrong one. `[asm] entry` in
    /// `cpclib-lsp.toml` settles it.
    pub fn sole_run_root(&self) -> Option<&Path> {
        match self.run_roots.as_slice() {
            [only] => Some(only.as_path()),
            _ => None
        }
    }
}

pub fn graph_of(workspace: &Workspace) -> ProjectGraph {
    let sources = &workspace.sources;

    let mut includes: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for (path, text) in sources {
        let targets = extract_include_filenames(text)
            .iter()
            .filter_map(|name| resolve_include_path(name, path))
            .filter_map(|p| std::fs::canonicalize(p).ok())
            .collect();
        includes.insert(path.clone(), targets);
    }

    let mut run_roots: Vec<PathBuf> = sources
        .iter()
        .filter(|(_, text)| declares_run(text))
        .map(|(path, _)| path.clone())
        .collect();
    run_roots.sort();
    run_roots.dedup();

    ProjectGraph {
        includes,
        run_roots
    }
}

/// Which program `doc_uri` belongs to, given an already-built [`ProjectGraph`].
///
/// Reachability over a graph that is already in hand - cheap enough to run per
/// document, which is the whole point of the split.
pub fn entry_in_graph(
    document: &Path,
    configured: Option<&str>,
    root: &Path,
    graph: &ProjectGraph
) -> Entry {
    let Ok(document) = std::fs::canonicalize(document)
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

    // Which RUN-bearing files reach the document.
    let roots: Vec<&PathBuf> = graph
        .run_roots
        .iter()
        .filter(|path| reaches(path, &document, &graph.includes))
        .collect();

    match roots.as_slice() {
        [only] if ***only == *document => Entry::Standalone,
        [only] => Entry::Project((*only).clone()),
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
        let code = write(
            tmp.path().as_std_path(),
            "demo_code.asm",
            "demo_start\n    ret\n"
        );

        let uri = code.clone();
        assert_eq!(entry_of(&uri, None), Entry::Project(sna));
    }

    /// A standalone test program is its own entry, so nothing that worked
    /// before regresses.
    #[test]
    fn a_file_carrying_its_own_run_is_standalone() {
        let tmp = camino_tempfile::tempdir().unwrap();
        project(tmp.path().as_std_path());
        let test = write(
            tmp.path().as_std_path(),
            "test1.asm",
            "    run start\nstart\n    ret\n"
        );

        let uri = test.clone();
        assert_eq!(entry_of(&uri, None), Entry::Standalone);
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

        let uri = shared.clone();
        assert_eq!(entry_of(&uri, None), Entry::Unknown);
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

        let uri = shared.clone();
        assert_eq!(entry_of(&uri, Some("one.asm")), Entry::Project(one));
    }

    /// A file nothing includes and which declares no program of its own.
    #[test]
    fn an_orphan_file_has_no_entry() {
        let tmp = camino_tempfile::tempdir().unwrap();
        project(tmp.path().as_std_path());
        let orphan = write(tmp.path().as_std_path(), "orphan.asm", "    nop\n");

        let uri = orphan.clone();
        assert_eq!(entry_of(&uri, None), Entry::Unknown);
    }

    #[test]
    fn run_is_recognised_whatever_its_case_and_spacing() {
        assert!(declares_run("    run demo_start\n"));
        assert!(declares_run("\tRUN start\n"));
        assert!(!declares_run("    run\n"), "a bare RUN names no entry");
        assert!(!declares_run("    running_total equ 3\n"));
        assert!(!declares_run("    ret\n"));

        // Neither of these is answerable by a line scan, which is why this
        // goes through the parser.
        assert!(
            declares_run("    nop : run start\n"),
            "basm allows several instructions per line"
        );
        assert!(
            !declares_run("    ;; run start\n"),
            "a commented-out RUN declares nothing"
        );
        assert!(
            !declares_run("/*\n    run start\n*/\n    ret\n"),
            "a RUN inside a block comment declares nothing"
        );

        // The prefilter is a substring test on purpose: it must never say
        // "no RUN here" about a file that has one.
        assert!(contains_run_word("    nop : RuN start"));
        assert!(!contains_run_word("    ld a, 1 : ret"));
    }

    /// Regression coverage for `fingerprint_of`'s short-lived memo: a burst
    /// of calls for the same root within `FINGERPRINT_CACHE_TTL` must not
    /// each re-walk the tree - repeated calls right after one another return
    /// the same value even though the file changed *between* them, proving
    /// the second call was served from the cache rather than recomputed.
    /// Once the TTL has actually elapsed, a real change is picked up -
    /// proving this is a short debounce, not a permanent staleness bug like
    /// the one this same fingerprint mechanism exists to prevent elsewhere.
    #[test]
    fn fingerprint_of_coalesces_a_burst_of_calls_for_the_same_unchanged_root() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let root = tmp.path().as_std_path();
        write(root, "main.asm", "ret\n");

        let first = fingerprint_of(root);

        // A change to the tree, immediately after - a real edit landing
        // inside the memo's own TTL window, exactly like 45 near-simultaneous
        // `did_open`s for a workspace-restore burst would.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        let changed = write(root, "second.asm", "nop\n");
        std::fs::File::open(&changed)
            .unwrap()
            .set_modified(later)
            .unwrap();

        let second = fingerprint_of(root);
        assert_eq!(
            first, second,
            "a call within the memo's TTL must not see the change yet"
        );

        std::thread::sleep(FINGERPRINT_CACHE_TTL + std::time::Duration::from_millis(50));
        let third = fingerprint_of(root);
        assert_ne!(
            first, third,
            "once the memo has actually expired, the real change must be picked up"
        );
    }
}

/// Addresses for `document`, obtained by assembling whatever program actually
/// contains it.
///
/// `None` means "no trustworthy addresses", which every caller must treat as
/// "make no address-aware suggestion". That covers an ambiguous or missing
/// entry, an entry that will not assemble, and - importantly - a document
/// whose buffer no longer matches the file on disk.
pub struct ProjectAddresses {
    pub env: std::sync::Arc<cpclib_asm::assembler::Env>,
    /// The document's own canonical path, which is the key its tokens are
    /// recorded under in `env`.
    pub document: PathBuf
}

/// The project root for a path, exposed so callers can fingerprint it.
pub fn root_of(document: &Path) -> Option<PathBuf> {
    project_root(document)
}

/// Assemble `entry` and hand back addresses usable for `document`.
///
/// The staleness check is the load-bearing part. Addresses are recorded
/// against *byte offsets in the file as assembled*; if the editor buffer has
/// unsaved changes those offsets have shifted, and every answer would be
/// confidently wrong. Being quiet while the user types is the right trade.
pub fn assemble_entry(
    entry: &Path,
    case_sensitive: bool,
    disabled: enumflags2::BitFlags<cpclib_asm::WarningCategory>
) -> Option<cpclib_asm::assembler::Env> {
    let entry_text = std::fs::read_to_string(entry).ok()?;

    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    parse.set_disabled_warning_categories(disabled);
    for dir in crate::root::ancestor_directories(entry) {
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
    let definitions = crate::build_defs::definitions_for_entry(entry);
    for (name, value) in &definitions.values {
        let value = match value.parse::<i32>() {
            Ok(number) => cpclib_tokens::ExprResult::from(number),
            Err(_) => cpclib_tokens::ExprResult::String(value.as_str().into())
        };
        let _ = assemble
            .symbols_mut()
            .assign_symbol_to_value(name.as_str(), value);
    }
    let options = cpclib_asm::EnvOptions::new(
        parse,
        assemble,
        std::sync::Arc::new(cpclib_bndbuild::cpclib_common::event::DiscardObserver)
    );

    // Only a *complete* assemble is usable: a half-laid-out program's
    // addresses describe something that never existed.
    Some(
        cpclib_asm::assembler::visit_tokens_all_passes_with_options(&listing, options)
            .ok()?
            .1
    )
}

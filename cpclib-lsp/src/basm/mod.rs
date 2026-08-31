//! LSP support for Z80 assembly in basm syntax (`.asm`/`.s`/`.z80` files).
//!
//! Each feature lives in its own file; they all extend the same
//! `AssemblyAnalyzer` type through separate `impl` blocks.
//!
//! Assembly sources can embed Locomotive BASIC blocks (`LOCOMOTIVE`
//! directive); those are detected here (`embedded_basic`) but analyzed by
//! the `locomotive` module — a one-directional `basm -> locomotive`
//! dependency. Similarly, a `.asm` file's comments can embed a `#!bndbuild`
//! rule; that's detected here (`embedded_bndbuild`) but executed by the
//! `bndbuild` module — a one-directional `basm -> bndbuild` dependency.

use std::sync::{Arc, RwLock};

use cpclib_asm::assembler::Env;
use cpclib_asm::parser::obtained::LocatedListing;
use dashmap::DashMap;
use tower_lsp::lsp_types::{Range, Url};

use crate::common::config::AsmConfig;

pub mod autocomplete;
pub mod call_hierarchy;
pub mod color;
pub mod command;
pub mod cycles;
pub mod definition;
pub mod diagnostics;
// Moved to `cpclib-project`, shared with the debug adapter: "which program
// does this file belong to, and how is it built" is not an LSP question.
// The LSP converts its `Url`s to paths at each call site.
use cpclib_project::{build_defs, entry};
pub(crate) mod breakpoint;
pub mod disassemble;
pub mod embedded_basic;
pub mod embedded_bndbuild;
pub mod expand;
pub mod format;
pub mod hover;
pub mod includes;
pub mod inlay_hints;
mod lint_smc_label;
pub mod overflow;
pub mod parse;
pub mod peephole;
pub mod refactor;
pub mod registers;
pub mod remove_parameter;
pub mod run;
pub mod semantic_tokens;
pub mod semantic_tokens_ast;
pub mod stabilize;
pub mod symbols;
pub mod timing;
pub mod token;

pub(crate) use token::semantic_tokens_legend;

/// Analyzer for Z80 assembly files using basm syntax.
///
/// Feature implementations are spread across this module's files, one
/// `impl AssemblyAnalyzer` block per concern.
///
/// Holds a parse-result cache keyed by document URI (see `parse.rs`'s
/// `parse_document`). `LocatedListing` is a self-referential type (built
/// with `ouroboros`) whose `ParserContext` field deliberately forbids
/// `Clone` - but that's a "don't accidentally deep-copy this" guard
/// (`ParserContext::clone()` panics because a derived clone would wrongly
/// duplicate a lazily-computed line/column lookup table instead of
/// resetting it, see `clone_with_state`), not a thread-safety one:
/// `LocatedListing` is verified `Send + Sync` (confirmed via a compile-time
/// `fn assert_send_sync<T: Send + Sync>()` probe), so caching it behind an
/// `Arc` - parsed once, shared by reference, never cloned - is exactly as
/// safe as it is for BASIC's fully-owned `LocatedBasicProgram`.
pub struct AssemblyAnalyzer {
    parse_cache: DashMap<Url, (i32, Result<Arc<LocatedListing>, Arc<LocatedListing>>)>,
    /// Cache for `expand::dry_run_env`'s result (a *real, full multi-pass
    /// assemble* of the whole document) - only used by features that
    /// genuinely need one (cross-file macro/`FUNCTION`/`STRUCT` lookup,
    /// real assembler warnings).
    /// `((document version, workspace fingerprint), env, whether the
    /// assemble actually finished)`.
    ///
    /// The completeness flag matters because a *failed* assemble still yields a
    /// usable partial `Env` - good enough for hover and `EQU` values, and
    /// actively wrong for anything address-shaped. See
    /// `expand::dry_run_env_cached_checked`.
    ///
    /// The fingerprint has to be in the key, not just the version - same
    /// reason as `address_source_cache` below: `dry_run_env` follows
    /// `include`s at any depth, so an edit to an *included* file changes
    /// this document's real assemble without touching its own version.
    /// Without it, diagnostics/semantic-tokens/macro-hover silently served
    /// stale results after editing an included file in another buffer.
    env_cache: DashMap<Url, ((i32, u128), Arc<Env>, bool)>,
    /// Cache for `expand::local_symbols_env`'s result (a lightweight,
    /// non-assembling local `EQU`/`SET` resolution) - what most hover
    /// value-substitution needs actually use, since `dry_run_env`'s real
    /// assemble only produces a correct result from a project's actual
    /// root/entry file (any other file either errors out or gets an
    /// incomplete symbol table) and is needlessly expensive for what's
    /// usually just "what number did this `EQU` resolve to" - recomputing
    /// it on every hover request made hovering visibly slow once
    /// register-value tracking made *some* form of this fire on almost
    /// every hover (any register name), not just the relatively rare
    /// "hovering an instruction mnemonic" case. A separate map from
    /// `env_cache` since it holds a different `Env` for the same
    /// document/version.
    local_env_cache: DashMap<Url, (i32, Arc<Env>)>,
    /// Assembled project `Env`s, keyed by entry file.
    ///
    /// Assembling a whole demo is expensive - 37s for `birthtro` - so this is
    /// not optional. The stored fingerprint is the newest modification time
    /// across the project's sources, which changes exactly when a rebuild
    /// would produce different addresses, and costs a `stat` per file to
    /// compute rather than a full assemble.
    /// Shared with the debug adapter (`cpclib-project`): one assembled
    /// project per fingerprint, however many features ask for it.
    projects: Arc<cpclib_project::cache::ProjectCache>,
    /// Where each document's real addresses come from, keyed by
    /// `(document version, workspace fingerprint)`.
    ///
    /// Resolving this walks the workspace reading every source, and four
    /// separate entry points ask for it during one editor interaction.
    address_source_cache: DashMap<Url, ((i32, u128), Arc<super::basm::peephole::AddressSource>)>,
    /// The include graph and `RUN`-bearing files of each project root, keyed
    /// by the project fingerprint.
    ///
    /// Keyed by *root*, not by document: every document in a project shares
    /// one graph, and building it reads and parses every source. Without this
    /// a workspace-wide scan rebuilds it once per file.
    /// Documents the user has explicitly asked to have analysed for peephole
    /// optimizations, and the range they asked about (`None` = the whole
    /// file).
    ///
    /// The automatic pass is off by default because it costs a full project
    /// assemble, so this is the other way in: an entry here makes the
    /// diagnostic and the Fix All lens behave exactly as if the warning class
    /// were enabled, for this document only. It is *sticky* on purpose -
    /// having asked once, the user keeps getting answers as they edit,
    /// instead of having to re-ask after every keystroke.
    peephole_requested: DashMap<Url, Option<Range>>,
    /// Cache for `autocomplete::collect_symbols`'s result (labels/`EQU`/
    /// `ASSIGN`/macro/module/section names, extracted by walking a
    /// document's full flattened token listing) - same `(version, Arc<T>)`
    /// shape as `env_cache`/`local_env_cache`. Completion needs this for
    /// every *other* open Assembly document too (cross-file label
    /// completion), and unlike the current document (whose version bumps on
    /// every keystroke, so this cache never hits for it while actively
    /// typing there), those other documents' versions typically stay stable
    /// while typing elsewhere - so this turns their full-listing walk from
    /// "redone on every completion keystroke" into "redone only when that
    /// other document itself actually changes".
    symbols_cache: DashMap<Url, (i32, Arc<Vec<(String, String)>>)>,
    /// Loaded once at `initialize()` (see `common::config`) - defaults to
    /// today's exact behavior until/unless a real `cpclib-lsp.toml` is
    /// found. `RwLock<Arc<_>>` rather than a plain field so callers can grab
    /// a cheap, self-contained snapshot (`config()`) without holding a lock
    /// across a whole request.
    config: RwLock<Arc<AsmConfig>>
}

impl AssemblyAnalyzer {
    pub fn new() -> Self {
        Self {
            parse_cache: DashMap::new(),
            env_cache: DashMap::new(),
            local_env_cache: DashMap::new(),
            projects: Arc::new(cpclib_project::cache::ProjectCache::new()),
            address_source_cache: DashMap::new(),
            peephole_requested: DashMap::new(),
            symbols_cache: DashMap::new(),
            config: RwLock::new(Arc::new(AsmConfig::default()))
        }
    }

    /// Drop `uri`'s cached parse/env(s), if any - called on
    /// `textDocument/didClose` so a closed document's cache entries don't
    /// linger indefinitely.
    pub fn evict(&self, uri: &Url) {
        self.parse_cache.remove(uri);
        self.env_cache.remove(uri);
        self.local_env_cache.remove(uri);
        self.symbols_cache.remove(uri);
        self.address_source_cache.remove(uri);
        self.peephole_requested.remove(uri);
        // The project cache deliberately survives: it is keyed by entry
        // path, not by document, and the project outlives any one editor tab.
    }

    /// Ask for peephole analysis of `uri`, optionally narrowed to `scope`.
    ///
    /// The scope narrows *what is reported*, never what is analysed: a
    /// suggestion inside a selection is only safe because of what surrounds
    /// it, so the whole document (and its project) is always examined.
    pub fn request_peephole(&self, uri: &Url, scope: Option<Range>) {
        self.peephole_requested.insert(uri.clone(), scope);
    }

    /// Stop reporting peephole optimizations for `uri` - the undo for
    /// [`Self::request_peephole`]. `None` clears every document at once.
    pub fn clear_peephole_request(&self, uri: Option<&Url>) {
        match uri {
            Some(uri) => {
                self.peephole_requested.remove(uri);
            },
            None => self.peephole_requested.clear()
        }
    }

    /// Should peephole matches be reported for `uri` at all - because the
    /// warning class is on, or because the user asked for this document?
    pub(super) fn peephole_wanted(&self, uri: &Url) -> bool {
        self.config().warnings.peephole_optimizer || self.peephole_requested.contains_key(uri)
    }

    /// The range an explicit request narrowed itself to, if any.
    pub(super) fn peephole_scope(&self, uri: &Url) -> Option<Range> {
        self.peephole_requested.get(uri).and_then(|e| *e.value())
    }

    pub fn set_config(&self, config: AsmConfig) {
        *self.config.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(config);
    }

    pub fn config(&self) -> Arc<AsmConfig> {
        self.config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Default for AssemblyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace fingerprint for `uri`'s project root - a `stat` per candidate
/// source, no reads. Shared by every cache whose result depends on more
/// than just `uri`'s own document version, because it follows `include`s:
/// `env_cache` (`expand::dry_run_env_cached_checked`) and
/// `address_source_cache` (`peephole::peephole_addresses`) both need this
/// in their key, since an edit to an *included* file changes their result
/// without touching the including document's own version. `0` when `uri`
/// isn't a file path or has no discoverable project root (e.g. an unsaved
/// buffer) - same fallback both call sites already used before this was
/// shared.
pub(super) fn workspace_fingerprint_of(uri: &Url) -> u128 {
    uri.to_file_path()
        .ok()
        .as_deref()
        .and_then(cpclib_project::entry::root_of)
        .map(|root| cpclib_project::entry::fingerprint_of(&root))
        .unwrap_or(0)
}

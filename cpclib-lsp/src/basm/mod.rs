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
use tower_lsp::lsp_types::Url;

use crate::common::config::AsmConfig;

pub mod autocomplete;
pub mod call_hierarchy;
pub mod color;
pub mod command;
pub mod cycles;
pub mod definition;
pub mod diagnostics;
pub mod disassemble;
pub mod embedded_basic;
pub mod embedded_bndbuild;
pub mod expand;
pub mod format;
pub mod hover;
pub mod includes;
pub mod overflow;
pub mod parse;
pub mod refactor;
pub mod registers;
pub mod remove_parameter;
pub mod semantic_tokens;
pub mod semantic_tokens_ast;
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
    /// real assembler warnings). Same `(version, Arc<T>)` shape as
    /// `parse_cache`.
    env_cache: DashMap<Url, (i32, Arc<Env>)>,
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

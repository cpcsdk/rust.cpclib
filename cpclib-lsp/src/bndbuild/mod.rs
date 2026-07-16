//! LSP support for bndbuild build files (`bndbuild.yml` / `bnd.build` /
//! `build.bnd`): YAML rules + Jinja templating + task command lines.
//!
//! Each feature lives in its own file; they all extend the same
//! `BuildFileAnalyzer` type through separate `impl` blocks.

pub mod autocomplete;
pub mod definition;
pub mod delegated_help;
pub mod diagnostics;
pub mod hover;
pub mod internal_commands;
pub mod semantic_tokens;
pub mod sourcemap;
pub mod symbols;
pub mod token;

/// Analyzer for build files (YAML with Jinja templates).
///
/// Feature implementations are spread across this module's files, one
/// `impl BuildFileAnalyzer` block per concern.
pub struct BuildFileAnalyzer {}

impl BuildFileAnalyzer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BuildFileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

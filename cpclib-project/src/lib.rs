//! What a source file belongs to.
//!
//! One question family, asked by more than one tool: *given a source file,
//! which program does it belong to, how is that program built, and where does
//! each line end up?*
//!
//! `cpclib-lsp` has always answered it - to resolve a `goto definition` across
//! includes, to know which entry point to assemble for address-aware analysis,
//! and to recover the `-D` symbols a build passes to `basm`. A debug adapter
//! asks exactly the same questions, for exactly the same reasons, so the
//! answers live here rather than twice.
//!
//! Deliberately editor-agnostic: nothing here knows about LSP, DAP, UTF-16
//! column arithmetic or incremental document sync. Callers convert at their own
//! boundary and pass plain paths.

pub mod build_defs;
pub mod cache;
pub mod config;
pub mod embedded_build;
pub mod entry;
pub mod jinja;
pub mod root;
pub mod srcmap;
pub mod walk;

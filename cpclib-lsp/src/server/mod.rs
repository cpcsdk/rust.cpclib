//! LSP server plumbing: the `tower_lsp` backend, capability registration and
//! per-document-type dispatch to the language modules.

pub mod backend;

pub use backend::CpcLspBackend;

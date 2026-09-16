// `pub`, not `mod`: an external consumer driving code intelligence
// directly (e.g. an MCP server) needs `AssemblyAnalyzer` and its per-feature
// return types (`basm::cycles::SelectionCycleCount`, etc.), same as
// `bndbuild` below already is for the same reason.
pub mod basm;
pub mod bndbuild;
pub mod common;
mod csl;
mod fileformat;
mod locomotive;
mod server;

pub use common::config;
pub use server::CpcLspBackend;

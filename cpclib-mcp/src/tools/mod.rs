//! One module per tool domain. Each holds its `#[tool]`-annotated thin
//! wrapper methods on [`crate::McpServer`] (its own `#[tool_router]` block,
//! combined with every other domain's in [`crate::McpServer::new`]) plus
//! the actual business-logic functions they delegate to - kept as plain
//! functions (not `McpServer` methods) so they can be unit-tested directly,
//! with no MCP transport involved.

pub mod asm;
pub mod basic;
pub mod basmopt;
pub mod behavior;
pub mod build;
pub mod common;
pub mod crunch;
pub mod disc;
pub mod emulator;
pub mod lsp;
pub mod project;
pub mod render;
pub mod reorder;
pub mod sandbox;
pub mod sizemap;
pub mod sna;
pub mod variants;

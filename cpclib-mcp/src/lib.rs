//! `cpclib-mcp`: an MCP (Model Context Protocol) server exposing this
//! workspace's Amstrad CPC democoding capabilities to AI agents - static
//! analysis (assemble/diagnostics, NOP-count timing, peephole-optimization
//! suggestions, hover/goto-definition), screen rendering, live emulator
//! control, and build/disc/snapshot/BASIC convenience wrappers.
//!
//! stdio transport only (what Claude Code and other MCP hosts actually
//! spawn) - see `main.rs`. Every tool's business logic lives in
//! `tools::<domain>` as a plain, directly-unit-testable function; the
//! `#[tool]`-annotated methods here are thin wrappers that parse input,
//! call into it, and normalize the result/error through [`error::ToolError`].

pub mod error;
pub mod session;
pub mod tools;

use std::sync::Arc;
use std::time::Duration;

use cpclib_lsp::basm::AssemblyAnalyzer;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, tool_handler};

use crate::session::SessionManager;

/// How long an emulator session may sit untouched before the background
/// sweep force-closes it.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The MCP server's own state: one long-lived [`AssemblyAnalyzer`] (so its
/// whole-project assemble/include-graph caches carry over between tool
/// calls, the entire point of holding one instead of building one per
/// call), the emulator [`SessionManager`], and the combined tool router
/// (every domain module's own router, summed in [`Self::with_sessions`]).
pub struct McpServer {
    analyzer: Arc<AssemblyAnalyzer>,
    sessions: Arc<SessionManager>,
    tool_router: ToolRouter<Self>
}

impl McpServer {
    /// Build a server around an already-constructed [`SessionManager`] -
    /// what `main.rs` uses, so it can keep its own `Arc` to force-close
    /// every surviving session on shutdown after `McpServer` itself (and
    /// the `rmcp` service holding it) has been consumed by `.serve(...)`.
    pub fn with_sessions(sessions: Arc<SessionManager>) -> Self {
        Self {
            analyzer: Arc::new(AssemblyAnalyzer::new()),
            sessions,
            tool_router: Self::asm_router()
                + Self::lsp_router()
                + Self::basmopt_router()
                + Self::render_router()
                + Self::emulator_router()
                + Self::build_router()
                + Self::disc_router()
                + Self::sna_router()
                + Self::basic_router()
                + Self::crunch_router()
                + Self::reorder_router()
        }
    }

    /// Convenience constructor for tests/callers that don't need to force-
    /// close sessions afterward themselves.
    pub fn new() -> Self {
        let sessions = SessionManager::new(DEFAULT_IDLE_TIMEOUT);
        sessions.spawn_idle_sweep();
        Self::with_sessions(sessions)
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Amstrad CPC democoding tools: assemble/diagnose basm sources, count Z80 NOP \
             timing, suggest/apply peephole optimizations, hover/goto-definition, render CPC \
             screen memory to PNG, drive a live emulator session, run bndbuild targets, and \
             inspect/edit discs, snapshots, and Locomotive BASIC programs. Tools marked \
             MUTATING write to disk or a live session; everything else is read-only."
        )
    }
}

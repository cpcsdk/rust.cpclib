//! `rmcp` stdio transport wiring. `tracing` goes to stderr only - stdout is
//! reserved for MCP protocol frames (any stray print to stdout corrupts the
//! stream a real MCP host is parsing).

use std::sync::Arc;

use cpclib_mcp::session::SessionManager;
use cpclib_mcp::{DEFAULT_IDLE_TIMEOUT, McpServer};
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let sessions = SessionManager::new(DEFAULT_IDLE_TIMEOUT);
    sessions.spawn_idle_sweep();

    let server = McpServer::with_sessions(Arc::clone(&sessions));
    tracing::info!("cpclib-mcp starting on stdio");
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    // Force-close every surviving emulator session on shutdown - `server`
    // (and its own `Arc<SessionManager>`) was consumed by `.serve(...)`
    // above, so this relies on the separate `Arc` kept here.
    sessions.close_all().await;

    Ok(())
}

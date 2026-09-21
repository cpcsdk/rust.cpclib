//! `rmcp` stdio transport wiring. `tracing` goes to stderr only - stdout is
//! reserved for MCP protocol frames (any stray print to stdout corrupts the
//! stream a real MCP host is parsing).

use std::sync::Arc;

use cpclib_mcp::session::SessionManager;
use cpclib_mcp::{DEFAULT_IDLE_TIMEOUT, McpServer};
use rmcp::ServiceExt;

/// Gives the MCP transport a private copy of the original stdout, then
/// points file descriptor 1 at stderr.
///
/// Stdout is reserved for protocol frames, but native code this server runs
/// in-process does not know that: the vendored Shrinkler and pucrunch
/// crunchers `printf` progress straight to fd 1 (confirmed live - a build
/// crunching with Shrinkler put 1.8 MB of progress bars on the stream and no
/// response ever parsed). Redirecting fd 1 only *during* such a call cannot
/// work (a concurrent call's real response would be corrupted); doing it once
/// at startup, with the protocol on its own descriptor, has no such race.
///
/// Implemented for Unix (`dup`/`dup2`) and Windows (see the `windows` version
/// below - written without access to a Windows machine, so it is untested;
/// if native output ever reaches the protocol stream there, start there).
/// Elsewhere this returns `None` and the stock stdio transport is used.
#[cfg(unix)]
fn private_protocol_writer() -> Option<tokio::fs::File> {
    use std::os::fd::FromRawFd;

    // SAFETY: plain fd duplication at process start, before any other thread
    // touches stdout; the duplicate is owned exclusively by the returned File.
    unsafe {
        let protocol_fd = libc::dup(1);
        if protocol_fd < 0 || libc::dup2(2, 1) < 0 {
            return None;
        }
        Some(tokio::fs::File::from_std(std::fs::File::from_raw_fd(protocol_fd)))
    }
}

/// Windows counterpart of the Unix version above. Two layers of "stdout"
/// exist and both must be redirected:
///
/// - the Win32 standard handle (`GetStdHandle`), used by Rust's own I/O and
///   any code calling `WriteFile` on it;
/// - the C runtime's descriptor 1, which is what the vendored C/C++
///   crunchers' `printf` writes to. It was bound to the original handle at
///   process start, so `SetStdHandle` alone does not move it - `_dup2` does
///   (and the CRT also updates the Win32 standard handle when descriptor 1
///   is redirected).
///
/// The protocol gets its own duplicate of the original stdout handle, taken
/// *before* either redirect.
#[cfg(windows)]
fn private_protocol_writer() -> Option<tokio::fs::File> {
    use std::os::windows::io::FromRawHandle;

    use windows_sys::Win32::Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // The C runtime's descriptor duplication. Linked from the CRT that
    // Rust's std already depends on (MSVC UCRT or MinGW msvcrt).
    unsafe extern "C" {
        fn _dup2(source: i32, target: i32) -> i32;
    }

    // SAFETY: Win32/CRT calls at process start; every handle passed is either
    // a standard handle just queried or the duplicate created below, whose
    // ownership moves into the returned File.
    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        let stderr = GetStdHandle(STD_ERROR_HANDLE);
        let unusable = |h: HANDLE| h.is_null() || h == INVALID_HANDLE_VALUE;
        if unusable(stdout) || unusable(stderr) {
            return None;
        }

        // Protocol channel: an independent duplicate of the real stdout.
        let mut protocol: HANDLE = std::ptr::null_mut();
        let process = GetCurrentProcess();
        if DuplicateHandle(process, stdout, process, &mut protocol, 0, 0, DUPLICATE_SAME_ACCESS) == 0
            || unusable(protocol)
        {
            return None;
        }

        // Point C `printf` (descriptor 1) at stderr, then make the Win32
        // standard output handle agree (the CRT normally does this itself;
        // doing it explicitly is harmless and covers a CRT that does not).
        if _dup2(2, 1) < 0 {
            return None;
        }
        SetStdHandle(STD_OUTPUT_HANDLE, stderr);

        Some(tokio::fs::File::from_std(std::fs::File::from_raw_handle(protocol as _)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    // Before anything can print: see `private_protocol_writer`.
    #[cfg(any(unix, windows))]
    let protocol_writer = private_protocol_writer();

    let sessions = SessionManager::new(DEFAULT_IDLE_TIMEOUT);
    sessions.spawn_idle_sweep();

    let server = McpServer::with_sessions(Arc::clone(&sessions));
    tracing::info!("cpclib-mcp starting on stdio");
    #[cfg(any(unix, windows))]
    let service = match protocol_writer {
        Some(writer) => server.serve((tokio::io::stdin(), writer)).await?,
        None => server.serve(rmcp::transport::stdio()).await?
    };
    #[cfg(not(any(unix, windows)))]
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    // Force-close every surviving emulator session on shutdown - `server`
    // (and its own `Arc<SessionManager>`) was consumed by `.serve(...)`
    // above, so this relies on the separate `Arc` kept here.
    sessions.close_all().await;

    Ok(())
}

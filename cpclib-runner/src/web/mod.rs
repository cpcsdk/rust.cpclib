//! Installing and serving a web-based emulator.
//!
//! Downloading a tool and *being a web application* are unrelated concerns:
//! the existing `DelegateApplicationDescription` already knows how to fetch,
//! unpack and cache, and none of that cares whether what comes out is an
//! executable or a directory of files to serve. So this adds only the part
//! that is genuinely different - what "launching" means.
//!
//! This module serves the plain, unmodified emulator and a generic framed
//! channel to it (`web::server`); it knows nothing about any particular
//! debug protocol. Patching the served page so a specific debugger can reach
//! its in-page engine is a separate concern, owned by whichever debugger
//! integration needs it.

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};

pub mod js1984;
pub mod robot_bridge;
pub mod server;
pub use server::{
    ServerHandle, decode_content_length_messages, encode_content_length_message, serve, serve_on
};

/// How an application is started once it is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchKind {
    /// Spawn a process, the assumption everything else in this crate makes.
    Native,
    /// Serve a directory and open a page in a browser or an editor tab.
    Web
}

/// An application that is served rather than spawned.
pub trait WebApplication {
    /// The directory whose files are served.
    fn web_root(&self) -> Utf8PathBuf;
    /// The document to open inside it.
    fn entry_document(&self) -> &'static str {
        "index.html"
    }
}

/// The MIME type to serve a file with.
///
/// Written out rather than pulled from a crate because exactly one entry here
/// is load-bearing: upstream requires `.wasm` to arrive as `application/wasm`,
/// and getting it wrong produces a blank page with no error worth reading.
pub fn mime_for(path: &Utf8Path) -> &'static str {
    match path.extension() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("png") => "image/png",
        Some("json") => "application/json",
        Some("sna") => "application/octet-stream",
        _ => "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one MIME type that actually matters.
    #[test]
    fn wasm_is_served_as_wasm() {
        assert_eq!(mime_for(Utf8Path::new("6128.wasm")), "application/wasm");
        assert_eq!(
            mime_for(Utf8Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            mime_for(Utf8Path::new("app.js")),
            "text/javascript; charset=utf-8"
        );
    }
}

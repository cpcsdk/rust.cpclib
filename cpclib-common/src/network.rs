//! Network utilities for HTTP downloads
//!
//! This module provides common HTTP download functionality used across cpclib crates.
//! It requires the "network" feature flag to be enabled.

use std::io::{Cursor, Read};

// Re-export for downstream crates
pub use http;
pub use ureq;

/// Download an HTTP resource with proper configuration for large files.
///
/// This function handles the ureq 3.x body size limit issue by setting a 1GB limit
/// for large downloads (the default is 10MB which is too small for emulators, tools, etc.).
///
/// # Arguments
/// * `url` - The URL to download from
///
/// # Returns
/// A boxed Read trait object containing the downloaded content
///
/// # Example
/// ```no_run
/// # use cpclib_common::network::download;
/// let content = download("https://example.com/file.zip")?;
/// # Ok::<_, String>(())
/// ```
pub fn download(url: &str) -> Result<Box<dyn Read + Send + Sync>, String> {
    let mut response = download_response(url)?;

    // Read body into bytes with large limit (ureq 3.x API - default is 10MB)
    // Set to 1GB for large downloads like emulators and tools
    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024 * 1024) // 1GB limit for large downloads
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    Ok(Box::new(Cursor::new(bytes)))
}

/// Download an HTTP resource and return the response for manual body handling.
///
/// This is useful when you need more control over the response, such as accessing
/// headers or streaming the body differently.
///
/// # Arguments
/// * `url` - The URL to download from
///
/// # Returns
/// An http::Response<ureq::Body> for manual processing
pub fn download_response(url: &str) -> Result<http::Response<ureq::Body>, String> {
    ureq::get(url)
        .header("Cache-Control", "max-age=1")
        .header("From", "krusty.benediction@gmail.com")
        .header("User-Agent", "cpclib")
        .call()
        .map_err(|e| e.to_string())
}

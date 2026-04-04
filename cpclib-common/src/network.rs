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

/// Upload a file using multipart/form-data encoding.
///
/// # Arguments
/// * `url` - The URL to upload to
/// * `field_name` - The name of the form field (e.g., "upfile")
/// * `file_path` - Path to the file to upload
/// * `remote_filename` - The filename to use in the multipart form (can be different from local filename)
///
/// # Returns
/// Ok(()) on success, Err with description on failure
///
/// # Example
/// ```no_run
/// # use cpclib_common::network::upload_file_multipart;
/// upload_file_multipart(
///     "http://example.com/upload",
///     "file",
///     "/path/to/local.txt",
///     "remote.txt"
/// )?;
/// # Ok::<_, String>(())
/// ```
pub fn upload_file_multipart(
    url: &str,
    field_name: &str,
    file_path: &str,
    remote_filename: &str
) -> Result<(), String> {
    use std::fs::File;
    use std::io::BufReader;

    // Verify file exists before proceeding
    let _file = File::open(file_path)
        .map_err(|e| format!("Failed to open file {}: {}", file_path, e))?;

    // Build multipart form manually
    let boundary = format!("----WebKitFormBoundary{:016x}", rand::random::<u64>());
    let content_type = format!("multipart/form-data; boundary={}", boundary);

    // Create the multipart body
    let mut body = Vec::new();

    // Add file part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            field_name, remote_filename
        )
        .as_bytes()
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");

    // Read file content
    use std::io::Read as _;
    let mut file_content = Vec::new();
    BufReader::new(File::open(file_path).unwrap())
        .read_to_end(&mut file_content)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    body.extend_from_slice(&file_content);

    // End of multipart
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    // Send the request
    ureq::post(url)
        .header("Content-Type", &content_type)
        .header("User-Agent", "cpclib")
        .send(body)
        .map_err(|e| format!("Upload failed: {}", e))?;

    Ok(())
}

/// Make a simple GET request and read the response body into a string.
///
/// # Arguments
/// * `url` - The URL to request
///
/// # Returns
/// The response body as a String
pub fn get_to_string(url: &str) -> Result<String, String> {
    let mut response = ureq::get(url)
        .header("User-Agent", "cpclib")
        .call()
        .map_err(|e| e.to_string())?;

    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024 * 100) // 100MB limit
        .read_to_vec()
        .map_err(|e| e.to_string())?;

    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8 in response: {}", e))
}

/// URL-encode a string for use in query parameters.
///
/// # Arguments
/// * `input` - The string to encode
///
/// # Returns
/// URL-encoded string
pub fn url_encode(input: &str) -> String {
    urlencoding::encode(input).into_owned()
}

//! Asset management for templates and external resources

use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/templates/"]
#[include = "*.jinja"]
#[include = "*.js"]
#[include = "*.css"]
pub struct Templates;

const HIGHLIGHTJS_URL: &str =
    "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js";
const HIGHLIGHTJS_CSS_URL: &str =
    "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/atom-one-dark.min.css";

/// Get cache directory for basmdoc assets
fn get_cache_dir() -> std::path::PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cpclib-basmdoc");

    fs_err::create_dir_all(&cache_dir).ok();
    cache_dir
}

/// Download a URL and cache it, or return cached content
fn download_or_cache(url: &str, filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cache_file = get_cache_dir().join(filename);

    // Check if cached file exists
    if cache_file.exists() {
        return Ok(fs_err::read_to_string(&cache_file)?);
    }

    // Download the file
    let response = ureq::get(url).call()?;
    let content = response.into_string()?;

    // Cache it
    fs_err::write(&cache_file, &content).ok();

    Ok(content)
}

// Get highlight.js content (download once, then cache)
pub fn get_highlightjs() -> String {
    download_or_cache(HIGHLIGHTJS_URL, "highlight.min.js").unwrap_or_else(|e| {
        eprintln!(
            "Warning: Failed to download highlight.js: {}. Syntax highlighting will be disabled.",
            e
        );
        String::new()
    })
}

// Get atom-one-dark CSS content (download once, then cache)
pub fn get_highlightjs_css() -> String {
    download_or_cache(HIGHLIGHTJS_CSS_URL, "atom-one-dark.min.css").unwrap_or_else(|e| {
        eprintln!(
            "Warning: Failed to download highlight.js CSS: {}. Styling will be limited.",
            e
        );
        String::new()
    })
}

// Get documentation.js content from embedded templates
pub fn get_documentation_js() -> String {
    Templates::get("documentation.js")
        .map(|file| String::from_utf8_lossy(file.data.as_ref()).to_string())
        .unwrap_or_else(|| {
            eprintln!("Warning: Failed to load documentation.js");
            String::new()
        })
}

// Get documentation.css content from embedded templates
pub fn get_documentation_css() -> String {
    Templates::get("documentation.css")
        .map(|file| String::from_utf8_lossy(file.data.as_ref()).to_string())
        .unwrap_or_else(|| {
            eprintln!("Warning: Failed to load documentation.css");
            String::new()
        })
}

/// Encode binary data to ASCII85 (custom implementation without delimiters)
/// This matches our JavaScript decoder exactly
fn encode_ascii85(data: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        // Get up to 4 bytes
        let mut tuple: u32 = 0;
        let mut count = 0;

        for j in 0..4 {
            if i + j < data.len() {
                tuple = (tuple << 8) | (data[i + j] as u32);
                count += 1;
            }
            else {
                tuple <<= 8;
            }
        }

        // Special case: four null bytes -> 'z'
        if count == 4 && tuple == 0 {
            result.push('z');
            i += 4;
            continue;
        }

        // Convert to 5 ASCII85 digits
        let mut digits = [0u8; 5];
        let mut temp = tuple;
        for k in (0..5).rev() {
            digits[k] = (temp % 85) as u8;
            temp /= 85;
        }

        // Output the appropriate number of characters
        let output_count = if count == 4 { 5 } else { count + 1 };
        for k in 0..output_count {
            result.push((digits[k] + 33) as char);
        }

        i += count;
    }

    result
}

/// Compress a string using gzip and encode as ASCII85
/// ASCII85 is ~6-7% more efficient than base64 (80% vs 75% efficiency)
/// This reduces the size of code blocks that are initially collapsed
pub fn compress_string(input: &str) -> Result<String, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(input.as_bytes())?;
    let compressed = encoder.finish()?;
    Ok(encode_ascii85(&compressed))
}

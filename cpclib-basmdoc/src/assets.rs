//! Asset management for templates and external resources

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/templates/"]
#[include = "*.jinja"]
#[include = "*.js"]
#[include = "*.css"]
pub struct Templates;

const HIGHLIGHTJS_URL: &str = "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js";
const HIGHLIGHTJS_CSS_URL: &str = "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/atom-one-dark.min.css";

/// Get cache directory for basmdoc assets
fn get_cache_dir() -> std::path::PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cpclib-basmdoc");
    
    std::fs::create_dir_all(&cache_dir).ok();
    cache_dir
}

/// Download a URL and cache it, or return cached content
fn download_or_cache(url: &str, filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cache_file = get_cache_dir().join(filename);
    
    // Check if cached file exists
    if cache_file.exists() {
        return Ok(std::fs::read_to_string(&cache_file)?);
    }
    
    // Download the file
    let response = ureq::get(url).call()?;
    let content = response.into_string()?;
    
    // Cache it
    std::fs::write(&cache_file, &content).ok();
    
    Ok(content)
}

// Get highlight.js content (download once, then cache)
pub fn get_highlightjs() -> String {
    download_or_cache(HIGHLIGHTJS_URL, "highlight.min.js")
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to download highlight.js: {}. Syntax highlighting will be disabled.", e);
            String::new()
        })
}

// Get atom-one-dark CSS content (download once, then cache)
pub fn get_highlightjs_css() -> String {
    download_or_cache(HIGHLIGHTJS_CSS_URL, "atom-one-dark.min.css")
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to download highlight.js CSS: {}. Styling will be limited.", e);
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

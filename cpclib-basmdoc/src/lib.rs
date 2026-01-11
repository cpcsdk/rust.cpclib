#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    /// Helper: create a temp file with given content, return (dir, file_path, content)
    fn create_temp_source_file(content: &str) -> (tempfile::TempDir, String, String) {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("test.asm");
        let mut file = File::create(&file_path).expect("create file");
        file.write_all(content.as_bytes()).expect("write");
        (dir, file_path.to_string_lossy().to_string(), content.to_string())
    }

    #[test]
    fn test_source_preserved_for_file_without_documentation() {
        // 1. Create a file with no documentation comments
        let source_code = "LD A,42\nNOP\nRET\n";
        let (_dir, file_path, expected_source) = create_temp_source_file(source_code);
        let display_name = "test.asm";

        // 2. Generate DocumentationPage for this file
        let page = DocumentationPage::for_file(&file_path, display_name, true).expect("parse");

        // 3. Check that a Source item exists in page.content and contains the correct code
        let source_item = page.content.iter().find(|it| it.item.is_source()).expect("Source item present");
        match &source_item.item {
            DocumentedItem::Source(src) => {
                assert_eq!(src, &expected_source, "Source content must match original");
            },
            _ => panic!("Not a Source variant"),
        }

        // 4. Merge with itself (simulate multi-file merge)
        let merged = DocumentationPage::merge(vec![page.clone()]);
        let merged_source_item = merged.content.iter().find(|it| it.item.is_source()).expect("Merged Source item present");
        match &merged_source_item.item {
            DocumentedItem::Source(src) => {
                assert_eq!(src, &expected_source, "Merged Source content must match original");
            },
            _ => panic!("Not a Source variant (merged)"),
        }

        // 5. Generate HTML and check that the source code appears in the output
        let html = merged.to_html();
        println!("\n--- GENERATED HTML ---\n{}\n--- END HTML ---\n", html);
        // The source will be syntax-highlighted, so check for key elements
        assert!(html.contains("LD") && html.contains("42") && html.contains("NOP") && html.contains("RET"), 
                "HTML output must contain the source code (with syntax highlighting)");
        // Also check that it's not showing the fallback message
        assert!(!html.contains("(Source not available)"), "HTML output must not show 'Source not available'");
    }
}
impl ItemDocumentation {
        pub fn macro_source(&self) -> String {
            match &self.item {
                DocumentedItem::Macro { content, .. } => content.clone(),
                _ => String::new(),
            }
        }

        pub fn function_source(&self) -> String {
            match &self.item {
                DocumentedItem::Function { content, .. } => content.clone(),
                _ => String::new(),
            }
        }
    pub fn item_long_summary(&self) -> String {
        match &self.item {
            DocumentedItem::Label(label) => format!("Label: {}", label),
            DocumentedItem::Equ(name, value) => format!("Equ: {} = {}", name, value),
            DocumentedItem::Macro { name, arguments, .. } => format!("Macro: {}({})", name, arguments.join(", ")),
            DocumentedItem::Function { name, arguments, .. } => format!("Function: {}({})", name, arguments.join(", ")),
            DocumentedItem::File(fname) => format!("File: {}", fname),
            DocumentedItem::Source(_) => "Source".to_string(),
            DocumentedItem::SyntaxError(_) => "Syntax Error".to_string(),
        }
    }

    pub fn item_short_summary(&self) -> String {
        match &self.item {
            DocumentedItem::Label(label) => label.clone(),
            DocumentedItem::Equ(name, _) => name.clone(),
            DocumentedItem::Macro { name, .. } => name.clone(),
            DocumentedItem::Function { name, .. } => name.clone(),
            DocumentedItem::File(fname) => fname.clone(),
            DocumentedItem::Source(_) => "Source".to_string(),
            DocumentedItem::SyntaxError(_) => "Syntax Error".to_string(),
        }
    }

    pub fn to_markdown(&self) -> String {
        // This is a placeholder; you can implement markdown rendering as needed
        format!("# {}\n{}", self.item_long_summary(), self.doc)
    }
}
pub mod cmdline;

use std::collections::HashMap;
use std::sync::Arc;
use std::borrow::Cow;

use dashmap::DashMap;
use indicatif::{ProgressBar, ProgressStyle};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

use lazy_static::lazy_static;

// Polyfill for par_iter when rayon is disabled - makes it behave like regular iter
#[cfg(not(feature = "rayon"))]
trait ParallelIteratorShim<'a, T: 'a> {
    type Iter: Iterator<Item = &'a T>;
    fn par_iter(&'a self) -> Self::Iter;
}

#[cfg(not(feature = "rayon"))]
impl<'a, T: 'a> ParallelIteratorShim<'a, T> for [T] {
    type Iter = std::slice::Iter<'a, T>;
    fn par_iter(&'a self) -> Self::Iter {
        self.iter()
    }
}

#[cfg(not(feature = "rayon"))]
impl<'a, T: 'a> ParallelIteratorShim<'a, T> for Vec<T> {
    type Iter = std::slice::Iter<'a, T>;
    fn par_iter(&'a self) -> Self::Iter {
        self.iter()
    }
}

use cpclib_asm::{ListingElement, MayHaveSpan, parse_z80_str, LocatedListing};
use minijinja::value::Object;
use minijinja::{Environment, ErrorKind, Value, context};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};

// Include syntax highlighting keywords generated by build.rs
include!(concat!(env!("OUT_DIR"), "/syntax_keywords.rs"));

#[derive(Embed)]
#[folder = "src/templates/"]
#[include = "*.jinja"]
#[include = "*.js"]
#[include = "*.css"]
struct Templates;

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
fn get_highlightjs() -> String {
    download_or_cache(HIGHLIGHTJS_URL, "highlight.min.js")
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to download highlight.js: {}. Syntax highlighting will be disabled.", e);
            String::new()
        })
}

// Get atom-one-dark CSS content (download once, then cache)
fn get_highlightjs_css() -> String {
    download_or_cache(HIGHLIGHTJS_CSS_URL, "atom-one-dark.min.css")
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to download highlight.js CSS: {}. Styling will be limited.", e);
            String::new()
        })
}

// Get documentation.js content from embedded templates
fn get_documentation_js() -> String {
    Templates::get("documentation.js")
        .map(|file| String::from_utf8_lossy(file.data.as_ref()).to_string())
        .unwrap_or_else(|| {
            eprintln!("Warning: Failed to load documentation.js");
            String::new()
        })
}

// Get documentation.css content from embedded templates
fn get_documentation_css() -> String {
    Templates::get("documentation.css")
        .map(|file| String::from_utf8_lossy(file.data.as_ref()).to_string())
        .unwrap_or_else(|| {
            eprintln!("Warning: Failed to load documentation.css");
            String::new()
        })
}

const GLOBAL_DOCUMENTATION_START: &str = ";;;";
const LOCAL_DOCUMENTATION_START: &str = ";;";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MetaDocumentation {
    Author(String),
    Date(String),
    Since(String)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DocumentedItem {
    File(String),
    Label(String),
    Equ(String, String),
    Macro { name: String, arguments: Vec<String>, content: String },
    Function { name: String, arguments: Vec<String>, content: String },
    Source(String), // Whole source of a file
    SyntaxError(String) // Parse error message
}

impl DocumentedItem {
    pub fn is_label(&self) -> bool {
        matches!(self, DocumentedItem::Label(_))
    }

    pub fn is_equ(&self) -> bool {
        matches!(self, DocumentedItem::Equ(_, _))
    }

    pub fn is_macro(&self) -> bool {
        matches!(self, DocumentedItem::Macro { .. })
    }

    pub fn is_function(&self) -> bool {
        matches!(self, DocumentedItem::Function { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self, DocumentedItem::File(_))
    }

    pub fn is_source(&self) -> bool {
        matches!(self, DocumentedItem::Source(_))
    }

    pub fn is_syntax_error(&self) -> bool {
        matches!(self, DocumentedItem::SyntaxError(_))
    }

    pub fn item_key(&self, fname: &str) -> String {
        match self {
            DocumentedItem::Label(l) => format!("label_{}", l),
            DocumentedItem::Equ(l, _) => format!("equ_{}", l),
            DocumentedItem::Macro { name, .. } => format!("macro_{}", name),
            DocumentedItem::Function { name, .. } => format!("function_{}", name),
            DocumentedItem::File(f) => format!("file_{}", f.replace(['/', '\\', '.'], "_")),
            DocumentedItem::Source(_) => format!("source_{}", fname.replace(['/', '\\', '.'], "_")),
            DocumentedItem::SyntaxError(_) => format!("syntax_error_{}", fname.replace(['/', '\\', '.'], "_")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolReference {
    #[serde(serialize_with = "serialize_arc_str", deserialize_with = "deserialize_arc_str")]
    pub source_file: Arc<str>,
    pub line_number: usize,
    #[serde(serialize_with = "serialize_cow_str", deserialize_with = "deserialize_cow_str")]
    pub context: Cow<'static, str>, // surrounding code for context
    #[serde(serialize_with = "serialize_arc_str", deserialize_with = "deserialize_arc_str")]
    pub highlighted_context: Arc<str> // syntax-highlighted context (Arc to avoid clones)
}

// Helper functions for Arc<str> serialization
fn serialize_arc_str<S>(arc: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(arc)
}

fn deserialize_arc_str<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(|s| Arc::from(s.as_str()))
}

// Helper functions for Cow<str> serialization
fn serialize_cow_str<S>(cow: &Cow<'static, str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(cow)
}

fn deserialize_cow_str<'de, D>(deserializer: D) -> Result<Cow<'static, str>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(|s| Cow::Owned(s))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemDocumentation {
    item: DocumentedItem,
    doc: String, // TODO use MetaDocumentation
    source_file: String,
    display_source_file: String,
    line_number: usize, // 1-indexed line number where the symbol is defined
    references: Vec<SymbolReference>,
    linked_source: Option<String> // Source code with symbols linked to their documentation
}

impl Object for ItemDocumentation {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str() {
            Some("doc") => Some(Value::from(&self.doc as &str)),
            Some("summary") => Some(Value::from(self.item_long_summary())),
            Some("short_summary") => Some(Value::from(self.item_short_summary())),
            Some("key") => Some(Value::from(self.item.item_key(&self.source_file))),
            Some("source_file") => Some(Value::from(&self.source_file as &str)),
            Some("display_source_file") => Some(Value::from(&self.display_source_file as &str)),
            Some("line_number") => Some(Value::from(self.line_number)),
            Some("references") => {
                let refs: Vec<Value> = self.references.iter().map(|r| Value::from_serialize(r)).collect();
                Some(Value::from(refs))
            },
            Some("has_references") => Some(Value::from(!self.references.is_empty())),
            Some("is_source") => Some(Value::from(self.item.is_source())),
            Some("is_syntax_error") => Some(Value::from(self.item.is_syntax_error())),
            Some("syntax_error_message") => {
                match &self.item {
                    DocumentedItem::SyntaxError(msg) => Some(Value::from(msg as &str)),
                    _ => None
                }
            },
            Some("linked_source") => {
                self.linked_source.as_ref().map(|s| Value::from(s as &str))
            },
            Some("source") => {
                // Expose source for full-file items and for macros/functions.
                match &self.item {
                    DocumentedItem::Source(src) => Some(Value::from(src as &str)),
                    DocumentedItem::Macro { .. } => {
                        if let Some(ls) = &self.linked_source {
                            Some(Value::from(ls as &str))
                        } else {
                            Some(Value::from(self.macro_source()))
                        }
                    }
                    DocumentedItem::Function { .. } => {
                        if let Some(ls) = &self.linked_source {
                            Some(Value::from(ls as &str))
                        } else {
                            Some(Value::from(self.function_source()))
                        }
                    }
                    _ => None
                }
            }
            None => None,
            Some(_) => None,
        }
    }


}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentationPage {
    fname: String,
    content: Vec<ItemDocumentation>,
    all_files: Vec<String>
}

impl Object for DocumentationPage {
    fn get_value(self: &Arc<Self>, name: &Value) -> Option<minijinja::value::Value> {
        match name.as_str() {
            Some("file_name") => Some(Value::from(self.fname.clone())),
            Some("file_list") => {
                let files = self.file_list()
                    .into_iter()
                    .map(Value::from)
                    .collect::<Vec<_>>();
                Some(Value::from(files))
            },
            Some("labels") => {
                let mut labels = self
                    .label_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                labels.sort_by(|a, b| a.item_short_summary().cmp(&b.item_short_summary()));
                let labels = labels.into_iter().map(Value::from_object).collect::<Vec<_>>();
                let labels = Value::from_object(labels);
                Some(labels)
            },
            Some("macros") => {
                let mut macros = self
                    .macro_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                macros.sort_by(|a, b| a.item_short_summary().cmp(&b.item_short_summary()));
                let macros = macros.into_iter().map(Value::from_object).collect::<Vec<_>>();
                let macros = Value::from_object(macros);
                Some(macros)
            },
            Some("equs") => {
                let mut equs = self
                    .equ_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                equs.sort_by(|a, b| a.item_short_summary().cmp(&b.item_short_summary()));
                let equs = equs.into_iter().map(Value::from_object).collect::<Vec<_>>();
                let equs = Value::from_object(equs);
                Some(equs)
            },
            Some("functions") => {
                let mut functions = self
                    .function_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                functions.sort_by(|a, b| a.item_short_summary().cmp(&b.item_short_summary()));
                let functions = functions.into_iter().map(Value::from_object).collect::<Vec<_>>();
                let functions = Value::from_object(functions);
                Some(functions)
            },
            Some("files") => {
                // Include the full source item (DocumentedItem::Source) as the first
                // entry for each file, followed by the merged file-level documentation.
                // Ensure we still present source blocks even when there are no file-level
                // documentation comments (merged_files() may be empty).
                let merged = self.merged_files();
                let mut files_vec: Vec<minijinja::value::Value> = Vec::new();

                if merged.is_empty() {
                    // No merged file docs; enumerate unique source files from content
                    let mut file_names: Vec<String> = self.content.iter().map(|it| it.source_file.clone()).collect();
                    file_names.sort();
                    file_names.dedup();

                    for fname in file_names {
                        // If a Source item exists for this file, push it first.
                        // Match either exact path or suffix (handles absolute vs workspace-relative).
                        let source_item_opt = self.content.iter().find(|it| {
                            it.item.is_source() && (it.source_file == fname || it.source_file.ends_with(&fname))
                        });

                        if let Some(source_item) = source_item_opt {
                            files_vec.push(Value::from_object(source_item.clone()));
                        } else {
                            unreachable!("No explicit source item found for file: {}. This is an impossible case", fname);
                            // No explicit source item found; try reading file content.
                            // If reading the provided name fails (likely because it's a
                            // workspace-relative path while content stores absolute
                            // paths), attempt to locate the absolute path from
                            // existing content entries and read that instead.
                            let mut code = std::fs::read_to_string(&fname).unwrap_or_default();
                            if code.is_empty() {
                                if let Some(it) = self.content.iter().find(|it| it.source_file.ends_with(&fname)) {
                                    code = std::fs::read_to_string(&it.source_file).unwrap_or_default();
                                }
                            }

                            files_vec.push(Value::from_object(ItemDocumentation {
                                item: DocumentedItem::Source(code),
                                doc: String::new(),
                                source_file: fname.clone(),
                                display_source_file: fname.clone(),
                                line_number: 0,
                                references: Vec::new(),
                                linked_source: None
                            }));
                        }
                    }
                } else {
                    for mf in merged.into_iter() {
                        // Find the corresponding Source item (if any) in the original content
                        // Match source item by exact path or by suffix to handle
                        // absolute vs workspace-relative path differences.
                        let source_item_opt = self.content.iter().find(|it| {
                            it.item.is_source() && (it.source_file == mf.source_file || it.source_file.ends_with(&mf.source_file))
                        });

                        if let Some(source_item) = source_item_opt {
                            files_vec.push(Value::from_object(source_item.clone()));
                        } else {
                            unreachable!("No explicit source item found for merged file: {}. This is an impossible case, we are not supposed to read files at this point", mf.source_file);
                            // No source item found for this merged file; try reading
                            // the merge-provided path first, then fallback to any
                            // matching absolute path found in content.
                            let mut code = std::fs::read_to_string(&mf.source_file).unwrap_or_default();
                            if code.is_empty() {
                                if let Some(it) = self.content.iter().find(|it| it.source_file.ends_with(&mf.source_file)) {
                                    code = std::fs::read_to_string(&it.source_file).unwrap_or_default();
                                }
                            }

                            files_vec.push(Value::from_object(ItemDocumentation {
                                item: DocumentedItem::Source(code),
                                doc: String::new(),
                                source_file: mf.source_file.clone(),
                                display_source_file: mf.source_file.clone(),
                                line_number: 0,
                                references: Vec::new(),
                                linked_source: None
                            }));
                        }
                        files_vec.push(Value::from_object(mf));
                    }
                }

                let files = Value::from_object(files_vec);
                Some(files)
            },
            Some("symbol_index") => {
                let index = self.symbol_index();
                let index_data: Vec<_> = index.into_iter().map(|(letter, items)| {
                    let items_vec: Vec<_> = items.into_iter()
                        .map(|item| Value::from_object(item.clone()))
                        .collect();
                    minijinja::value::Value::from_serialize(&(letter.to_string(), items_vec))
                }).collect();
                Some(Value::from(index_data))
            },
            _ => None
        }
    }

    fn call_method(
        self: &std::sync::Arc<Self>,
        _state: &minijinja::State<'_, '_>,
        method: &str,
        _args: &[minijinja::Value]
    ) -> Result<minijinja::Value, minijinja::Error> {
        match method {
            "has_labels" => Ok(Value::from(self.has_labels())),
            "has_macros" => Ok(Value::from(self.has_macros())),
            "has_equ" => Ok(Value::from(self.has_equ())),
            "has_functions" => Ok(Value::from(self.has_functions())),
            "has_files" => Ok(Value::from(self.has_files())),

            _ => {
                Err(minijinja::Error::new(
                    ErrorKind::UnknownMethod,
                    format!("Unknown method '{}'", method)
                ))
            },
        }
    }
}
impl DocumentationPage {
    // TODO handle errors
    pub fn for_file(fname: &str, display_name: &str, include_undocumented: bool) -> Result<Self, String> {
        let code = std::fs::read_to_string(fname)
            .map_err(|e| format!("Unable to read {} file. {}", fname, e))?;
        
        // Parse the source code, but continue even if there's a parse error
        let parse_result = parse_z80_str(&code);
        let (tokens, parse_error) = match parse_result {
            Ok(tokens) => (tokens, None),
            Err(e) => {
                // Create empty token list by parsing empty string
                let empty_tokens = parse_z80_str("").unwrap_or_else(|_| unreachable!("Empty string should always parse"));
                (empty_tokens, Some(format!("Parse error: {}", e)))
            }
        };
        
        let doc = aggregate_documentation_on_tokens(&tokens, include_undocumented);
        let mut page = build_documentation_page_from_aggregates(fname, doc);
        
        // Add syntax error item if parsing failed
        if let Some(error_msg) = parse_error {
            page.content.insert(0, ItemDocumentation {
                item: DocumentedItem::SyntaxError(error_msg),
                doc: String::new(),
                source_file: fname.to_string(),
                display_source_file: fname.to_string(),
                line_number: 0,
                references: Vec::new(),
                linked_source: None
            });
        }
        
        // Always add the source code
        page.content.insert(0, ItemDocumentation {
            item: DocumentedItem::Source(code.clone()),
            doc: String::new(),
            source_file: fname.to_string(),
            display_source_file: fname.to_string(),
            line_number: 0,
            references: Vec::new(),
            linked_source: None
        });
        // Set the display name for all items in this page so templates and
        // client-side filtering consistently use workspace-relative paths.
        for it in &mut page.content {
            it.display_source_file = display_name.to_string();
        }
        page.all_files = vec![display_name.to_string()];
        
        // Populate cross-references with symbol links
        let symbols = page.all_symbols();
        page = populate_cross_references(page, &tokens, &symbols);
        
        // Link symbols in source code
        page = page.link_source_symbols();
        
        Ok(page)
    }

    /// Parse file without populating cross-references (for later batch processing)
    /// Returns both the documentation page and the parsed tokens
    pub fn for_file_without_refs(fname: &str, display_name: &str, include_undocumented: bool) -> Result<(Self, LocatedListing), String> {
        let code = std::fs::read_to_string(fname)
            .map_err(|e| format!("Unable to read {} file. {}", fname, e))?;
        
        // Parse the source code, but continue even if there's a parse error
        let parse_result = parse_z80_str(&code);
        let (tokens, parse_error) = match parse_result {
            Ok(tokens) => (tokens, None),
            Err(e) => {
                // Create empty token list by parsing empty string
                let empty_tokens = parse_z80_str("").unwrap_or_else(|_| unreachable!("Empty string should always parse"));
                (empty_tokens, Some(format!("Parse error: {}", e)))
            }
        };
        
        let doc = aggregate_documentation_on_tokens(&tokens, include_undocumented);
        let mut page = build_documentation_page_from_aggregates(fname, doc);
        
        // Add syntax error item if parsing failed
        if let Some(error_msg) = parse_error {
            page.content.insert(0, ItemDocumentation {
                item: DocumentedItem::SyntaxError(error_msg),
                doc: String::new(),
                source_file: fname.to_string(),
                display_source_file: fname.to_string(),
                line_number: 0,
                references: Vec::new(),
                linked_source: None
            });
        }
        
        // Always add the source code
        page.content.insert(0, ItemDocumentation {
            item: DocumentedItem::Source(code.clone()),
            doc: String::new(),
            source_file: fname.to_string(),
            display_source_file: fname.to_string(),
            line_number: 0,
            references: Vec::new(),
            linked_source: None
        });
        // For the without-refs variant we also set the display name so later
        // processing that renders templates can rely on `display_source_file`.
        for it in &mut page.content {
            it.display_source_file = display_name.to_string();
        }
        page.all_files = vec![display_name.to_string()];
        
        Ok((page, tokens))
    }

    /// Merge multiple documentation pages into a single page
    pub fn merge(pages: Vec<Self>) -> Self {
        if pages.is_empty() {
            return Self {
                fname: String::from("Empty documentation"),
                content: Vec::new(),
                all_files: Vec::new()
            };
        }

        if pages.len() == 1 {
            return pages.into_iter().next().unwrap();
        }

        let fnames = pages.iter()
            .map(|p| p.fname.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let mut all_files: Vec<String> = pages.iter()
            .flat_map(|p| p.all_files.iter().cloned())
            .collect();
        all_files.sort();
        all_files.dedup();

        let mut content: Vec<ItemDocumentation> = pages.into_iter()
            .flat_map(|p| p.content)
            .collect();

        // Normalize all source_file and display_source_file fields in content to match canonical all_files entries
        let all_files_clone = all_files.clone();
        for it in &mut content {
            // If an all_files entry matches the end of the source_file, prefer it
            if let Some(matching) = all_files_clone.iter().find(|f| it.source_file.ends_with(f.as_str())) {
                it.display_source_file = matching.clone();
                it.source_file = matching.clone();
                continue;
            }
            // Fallback: match by basename
            if let Some(src_basename) = std::path::Path::new(&it.source_file).file_name().and_then(|s| s.to_str()) {
                if let Some(matching) = all_files_clone.iter().find(|f| std::path::Path::new(f).file_name().and_then(|s| s.to_str()) == Some(src_basename)) {
                    it.display_source_file = matching.clone();
                    it.source_file = matching.clone();
                }
            }
        }

        // Guarantee a Source item for every file in all_files
        for fname in &all_files {
            // Enforce: every file in all_files must have an in-memory Source item, never fallback to disk
            let existing = content.iter().find(|it| {
                it.item.is_source() &&
                (it.source_file == *fname ||
                 it.source_file.ends_with(fname) ||
                 fname.ends_with(&it.source_file))
            });
            if let Some(existing) = existing {
                // Already present, do nothing (or could move to front if needed)
            } else {
                panic!("INVARIANT VIOLATION: No in-memory Source item for file '{fname}'. This must never happen. Check earlier code for lost or mismatched Source items.");
            }
        }

        let mut merged = Self {
            fname: fnames,
            content,
            all_files
        };

        // Link symbols in source code
        merged = merged.link_source_symbols();

        // Canonicalize display_source_file values so templates and client-side
        // filtering use the same workspace-relative names where possible.
        // Prefer any name from `all_files` that is a suffix of the item's source_file,
        // otherwise try matching by basename.
        let all_files_clone = merged.all_files.clone();
        for it in &mut merged.content {
            // If an all_files entry matches the end of the source_file, prefer it
            if let Some(matching) = all_files_clone.iter().find(|f| it.source_file.ends_with(f.as_str())) {
                it.display_source_file = matching.clone();
                continue;
            }

            // Fallback: match by basename
            if let Some(src_basename) = std::path::Path::new(&it.source_file).file_name().and_then(|s| s.to_str()) {
                if let Some(matching) = all_files_clone.iter().find(|f| std::path::Path::new(f).file_name().and_then(|s| s.to_str()) == Some(src_basename)) {
                    it.display_source_file = matching.clone();
                }
            }
        }

        merged
    }

    /// Link symbols in source code for all macros and functions
    pub fn link_source_symbols(mut self) -> Self {
        let symbols = self.all_symbols();
        
        for item in &mut self.content {
            if item.item.is_macro() || item.item.is_function() {
                let source = if item.item.is_macro() {
                    item.macro_source()
                } else {
                    item.function_source()
                };
                if !source.is_empty() {
                    item.linked_source = Some(link_symbols_in_source(&source, &symbols));
                }
            }
        }
        
        self
    }
    
    /// Populate cross-references for all symbols from all files
    /// This should be called after merging pages to get cross-file references
    pub fn populate_all_cross_references(mut self, all_tokens: &[(String, LocatedListing)]) -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        spinner.set_message("Preparing symbol analysis...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        // Collect and sort symbol names once (OPTIMIZATION: sort once, not per macro)
        let mut symbol_names: Vec<String> = self.content.iter()
            .filter(|item| !item.item.is_file())
            .map(|item| item.item_short_summary())
            .collect();
        // Sort by length (longest first) to avoid partial matches in regex
        symbol_names.sort_by(|a: &String, b: &String| b.len().cmp(&a.len()));
        
        // Get all symbols for linking
        let all_symbols = self.all_symbols();
        
        // Create shared cache for ALL operations (MAJOR OPTIMIZATION: shared across all parallel threads)
        let highlight_cache = DashMap::new();
        
        spinner.finish_with_message("Preparation complete");
        
        // Create progress bar for file processing
        let pb = ProgressBar::new(all_tokens.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        pb.set_message("Analyzing files");
        
        // Collect all references from all files
        // Only use parallelization if workload is large enough to offset overhead (threshold: 10 files)
        let file_refs: Vec<HashMap<String, Vec<SymbolReference>>> = if all_tokens.len() > 10 {
            all_tokens.par_iter()
                .map(|(source_file, tokens)| {
                    let source_file_arc: Arc<str> = Arc::from(source_file.as_str());
                    let result = collect_cross_references(tokens, source_file_arc, &all_symbols, &highlight_cache);
                    pb.inc(1);
                    result
                })
                .collect()
        } else {
            all_tokens.iter()
                .map(|(source_file, tokens)| {
                    let source_file_arc: Arc<str> = Arc::from(source_file.as_str());
                    let result = collect_cross_references(tokens, source_file_arc, &all_symbols, &highlight_cache);
                    pb.inc(1);
                    result
                })
                .collect()
        };
        
        pb.finish_with_message("File analysis complete");
        
        // Merge all references
        let mut all_refs: HashMap<String, Vec<SymbolReference>> = HashMap::new();
        for refs in file_refs {
            for (symbol, mut symbol_refs) in refs {
                all_refs.entry(symbol)
                    .or_insert_with(Vec::new)
                    .append(&mut symbol_refs);
            }
        }
        
        // Also search for symbols inside macro and function content
        let macro_func_items: Vec<_> = self.content.iter()
            .filter(|item| item.item.is_macro() || item.item.is_function())
            .collect();
        
        // Create progress bar for macro/function processing
        let pb_macro = ProgressBar::new(macro_func_items.len() as u64);
        pb_macro.set_style(ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} items ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        pb_macro.set_message("Analyzing macros/functions");
        
        // Only parallelize if workload is large enough (threshold: 20 macros/functions)
        let macro_refs: Vec<HashMap<String, Vec<SymbolReference>>> = if macro_func_items.len() > 20 {
            macro_func_items.par_iter()
                .map(|item| {
                    let content = if item.item.is_macro() {
                        item.macro_source()
                    } else {
                        item.function_source()
                    };
                    
                    // Exclude the current item's name from the search to avoid self-references
                    let current_name = item.item_short_summary();
                    let filtered_symbols: Vec<&str> = symbol_names.iter()
                        .filter(|name| name.as_str() != current_name.as_str())
                        .map(|s| s.as_str())
                        .collect();
                    
                    let base_line = item.line_number;
                    let source_file_arc: Arc<str> = Arc::from(item.source_file.as_str());
                    let result = collect_references_in_content(&content, &filtered_symbols, source_file_arc, base_line, &all_symbols, &highlight_cache);
                    pb_macro.inc(1);
                    result
                })
                .collect()
        } else {
            macro_func_items.iter()
                .map(|item| {
                    let content = if item.item.is_macro() {
                        item.macro_source()
                    } else {
                        item.function_source()
                    };
                    
                    // Exclude the current item's name from the search to avoid self-references
                    let current_name = item.item_short_summary();
                    let filtered_symbols: Vec<&str> = symbol_names.iter()
                        .filter(|name| name.as_str() != current_name.as_str())
                        .map(|s| s.as_str())
                        .collect();
                    
                    let base_line = item.line_number;
                    let source_file_arc: Arc<str> = Arc::from(item.source_file.as_str());
                    let result = collect_references_in_content(&content, &filtered_symbols, source_file_arc, base_line, &all_symbols, &highlight_cache);
                    pb_macro.inc(1);
                    result
                })
                .collect()
        };
        
        pb_macro.finish_with_message("Macro/function analysis complete");
        
        // Merge macro/function references
        for refs in macro_refs {
            for (symbol, mut symbol_refs) in refs {
                all_refs.entry(symbol)
                    .or_insert_with(Vec::new)
                    .append(&mut symbol_refs);
            }
        }
        
        // Match references to documented items
        for item in &mut self.content {
            let symbol_name = item.item_short_summary();
            
            if let Some(refs) = all_refs.get(&symbol_name) {
                // Replace existing references with complete set from all files
                item.references = refs.clone();
            }
        }
        
        // Link symbols in source code
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        spinner.set_message("Linking symbols in source code...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        self = self.link_source_symbols();
        
        spinner.finish_with_message("Symbol linking complete");
        
        self
    }

    pub fn label_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.item.is_label())
    }

    pub fn macro_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.item.is_macro())
    }

    pub fn equ_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.item.is_equ())
    }

    pub fn function_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.item.is_function())
    }

    pub fn file_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.item.is_file())
    }

    pub fn syntax_error_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.item.is_syntax_error())
    }

    /// Get file documentation items merged by source file
    /// Multiple file-level doc comments from the same file are combined into one item
    pub fn merged_files(&self) -> Vec<ItemDocumentation> {
        let mut file_docs: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        
        // Group documentation by source file
        for item in self.file_iter() {
            file_docs
                .entry(item.source_file.clone())
                .or_insert_with(Vec::new)
                .push(item.doc.clone());
        }
        
        // Create merged ItemDocumentation for each file
        let mut merged: Vec<ItemDocumentation> = file_docs
            .into_iter()
            .map(|(source_file, docs)| {
                ItemDocumentation {
                    item: DocumentedItem::File(source_file.clone()),
                    doc: docs.join("\n\n"),
                    source_file: source_file.clone(),
                    display_source_file: source_file.clone(),
                    line_number: 0, // File-level documentation doesn't have a specific line
                    references: Vec::new(),
                    linked_source: None
                }
            })
            .collect();
        
        // Sort by source file name
        merged.sort_by(|a, b| a.source_file.cmp(&b.source_file));
        merged
    }

    /// Get all symbols (labels, macros, functions, equs) for cross-referencing
    pub fn all_symbols(&self) -> Vec<(String, String)> {
        let mut symbols = Vec::new();
        for item in &self.content {
            if !item.item.is_file() && !item.item.is_source() {
                symbols.push((item.item_short_summary(), item.item.item_key(&item.source_file)));
            }
        }
        symbols
    }

    /// Get alphabetically grouped symbols for index page
    pub fn symbol_index(&self) -> Vec<(char, Vec<&ItemDocumentation>)> {
        use std::collections::BTreeMap;
        
        let mut index: BTreeMap<char, Vec<&ItemDocumentation>> = BTreeMap::new();
        
        for item in &self.content {
                if !item.item.is_file() && !item.item.is_source() {
                let name = item.item_short_summary();
                if let Some(first_char) = name.chars().next() {
                    let key = first_char.to_ascii_uppercase();
                    index.entry(key).or_insert_with(Vec::new).push(item);
                }
            }
        }
        
        index.into_iter().collect()
    }

    pub fn has_labels(&self) -> bool {
        self.label_iter().next().is_some()
    }

    pub fn has_macros(&self) -> bool {
        self.macro_iter().next().is_some()
    }

    pub fn has_equ(&self) -> bool {
        self.equ_iter().next().is_some()
    }

    pub fn has_functions(&self) -> bool {
        self.function_iter().next().is_some()
    }

    pub fn has_files(&self) -> bool {
        // Consider a page to "have files" if there are either file-level
        // documentation items or a Source item (whole-file source) present.
        self.file_iter().next().is_some() || self.content.iter().any(|item| item.item.is_source())
    }

    pub fn has_documentation(&self) -> bool {
        self.content.iter().filter(|d| !d.item.is_source()).next().is_some()
    }

    /// Get a sorted list of unique source files (workspace-relative, normalized)
    pub fn file_list(&self) -> &[String] {
        &self.all_files
    }

    /// Normalize a file path to workspace-relative, using '/' separators
    fn normalize_path(path: &str) -> String {
        let p = std::path::Path::new(path);
        // If absolute, strip to file name (simulate workspace-relative)
        if p.is_absolute() {
            if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                return fname.replace('\\', "/");
            }
        }
        path.replace('\\', "/")
    }

    /// Return a string that encode the documentation page in markdown
    pub fn to_markdown(&self) -> String {
        let page = Value::from_object(self.clone());

        let mut env = Environment::new();
        const TMPL_NAME: &str = "markdown_documentation.jinja";
        let tmpl_src = Templates::get(TMPL_NAME).expect("Template not found").data;
        let tmpl_src = std::str::from_utf8(tmpl_src.as_ref()).unwrap();
        env.add_template(TMPL_NAME, tmpl_src).unwrap();

        let tmpl = env.get_template("markdown_documentation.jinja").unwrap();
        tmpl.render(context! {
            page
        })
        .unwrap()
    }

    /// Return a string that encode the documentation page in HTML
    pub fn to_html(&self) -> String {
        let page_obj = self;
        let _merged = self.merged_files();
        let mut files_vec: Vec<minijinja::value::Value> = Vec::new();
        let mut file_source_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        // Use all_files directly - they should already be normalized workspace-relative paths
        for fname in &self.all_files {
            // Find Source item by matching either exact path or normalized path
            let source_item_opt = self.content.iter().find(|it| {
                it.item.is_source() && 
                (&it.source_file == fname || 
                 &it.display_source_file == fname ||
                 it.source_file.ends_with(fname) ||
                 it.display_source_file.ends_with(fname))
            });
            
            let source_code: String;
            
            if let Some(source_item) = source_item_opt {
                // Extract source code from the Source item
                if let DocumentedItem::Source(ref code) = source_item.item {
                    source_code = code.clone();
                } else {
                    source_code = String::new();
                }
            } else {
                // This should never happen - all files should have Source items
                eprintln!("WARNING: No Source item found for file: {}", fname);
                source_code = String::new();
            }

            // Store in the direct mapping using the filename from all_files
            file_source_map.insert(fname.clone(), source_code);
        }

        let files = Value::from_object(files_vec);
        // Add a filter that applies syntax highlighting and symbol linking
        // This uses the Rust-side highlighter and linker to produce HTML
        // with <span class="hljs-..."> classes and <a class="symbol-link"> anchors.
        let mut env = Environment::new();
        let symbols_for_highlight = std::sync::Arc::new(self.all_symbols());
        let symbols_clone = symbols_for_highlight.clone();
        env.add_filter("highlight_and_link", move |value: String| -> Result<String, minijinja::Error> {
            let out = link_symbols_in_source(&value, &symbols_clone);
            Ok(out)
        });
        
        const TMPL_NAME: &str = "html_documentation.jinja";
        let tmpl_src = Templates::get(TMPL_NAME).expect("Template not found").data;
        let tmpl_src = std::str::from_utf8(tmpl_src.as_ref()).unwrap();
        env.add_template(TMPL_NAME, tmpl_src).unwrap();

        // Add embedded assets as globals
        env.add_global("highlightjs", get_highlightjs());
        env.add_global("highlightjs_css", get_highlightjs_css());
        env.add_global("documentation_js", get_documentation_js());
        env.add_global("documentation_css", get_documentation_css());
        // Add syntax highlighting keywords
        env.add_global("syntax_instructions", SYNTAX_INSTRUCTIONS);
        env.add_global("syntax_directives", SYNTAX_DIRECTIVES);

        let tmpl = env.get_template("html_documentation.jinja").unwrap();
        let file_list = page_obj.file_list();
        // Build sidebar lists from iterators, always as lists
        let labels: Vec<Value> = self.label_iter()
            .map(|item| Value::from_object(item.clone()))
            .collect();
        let macros: Vec<Value> = self.macro_iter()
            .map(|item| Value::from_object(item.clone()))
            .collect();
        let functions: Vec<Value> = self.function_iter()
            .map(|item| Value::from_object(item.clone()))
            .collect();
        let equs: Vec<Value> = self.equ_iter()
            .map(|item| Value::from_object(item.clone()))
            .collect();
        let syntax_errors: Vec<Value> = self.syntax_error_iter()
            .map(|item| Value::from_object(item.clone()))
            .collect();
        tmpl.render(context! {
            page => Value::from_object(page_obj.clone()),
            file_list => file_list,
            labels => labels,
            macros => macros,
            functions => functions,
            equs => equs,
            files => files,
            file_source_map => file_source_map,
            syntax_errors => syntax_errors,
        })
        .unwrap()
    }
}

#[inline]
fn is_any_documentation<T: ListingElement>(token: &T) -> bool {
    token.is_comment() && token.comment().starts_with(LOCAL_DOCUMENTATION_START)
}

#[inline]
fn is_global_documentation<T: ListingElement>(token: &T) -> bool {
    token.is_comment() && token.comment().starts_with(GLOBAL_DOCUMENTATION_START)
}

#[inline]
fn is_local_documentation<T: ListingElement>(token: &T) -> bool {
    token.is_comment()
        && token.comment().starts_with(LOCAL_DOCUMENTATION_START)
        && !token.comment().starts_with(GLOBAL_DOCUMENTATION_START)
}

fn is_documentable<T: ListingElement + ToString>(token: &T) -> bool {
    documentation_type(token, None).is_some()
}

fn documentation_type<T: ListingElement + ToString>(token: &T, last_global_label: Option<&str>) -> Option<DocumentedItem> {
    if token.is_label() {
        let label = token.label_symbol().to_string();
        // Handle local labels (starting with ".")
        if label.starts_with('.') {
            // If we have a parent global label, prepend it
            if let Some(parent) = last_global_label {
                Some(DocumentedItem::Label(format!("{}{}", parent, label)))
            } else {
                // No parent label, return the local label as-is (will be filtered later)
                Some(DocumentedItem::Label(label))
            }
        } else {
            // Regular global label
            Some(DocumentedItem::Label(label))
        }
    }
    else if token.is_equ() {
        Some(DocumentedItem::Equ(
            token.equ_symbol().to_string(),
            token.equ_value().to_string()
        ))
    }
    else if token.is_function_definition() {
        Some(DocumentedItem::Function {
            name: token.function_definition_name().to_string(),
            arguments: token
                .function_definition_params()
                .iter()
                .map(|a| a.to_string())
                .collect(),
            content: token.to_string()
        })
    }
    else if token.is_macro_definition() {
        Some(DocumentedItem::Macro {
            name: token.macro_definition_name().to_string(),
            arguments: token
                .macro_definition_arguments()
                .iter()
                .map(|a| a.to_string())
                .collect(),
            content: token.to_string() /*token.macro_definition_code().to_string()*/
        })
    }
    else {
        None
    }
}

pub fn build_documentation_page_from_aggregates<T: ListingElement + ToString>(
    fname: &str,
    agg: Vec<(String, Option<&T>, Option<String>, usize)>
) -> DocumentationPage {
    // Note: Source item is added by the calling function (for_file or for_file_without_refs)
    // to avoid duplicate Source items and ensure proper path handling
    let mut content: Vec<ItemDocumentation> = Vec::new();

    // Add all documentation items
    content.extend(
        agg.into_iter()
            .filter_map(|(doc, t, last_global_label, line_number)| {
                if let Some(t) = t {
                    documentation_type(t, last_global_label.as_deref()).map(|item| {
                        ItemDocumentation {
                            item,
                            doc,
                                    source_file: fname.to_string(),
                                    display_source_file: fname.to_string(),
                            line_number,
                            references: Vec::new(),
                            linked_source: None // Will be populated later
                        }
                    })
                }
                else {
                    Some(ItemDocumentation {
                        item: DocumentedItem::File(fname.to_string()),
                        doc,
                                source_file: fname.to_string(),
                                display_source_file: fname.to_string(),
                        line_number, // Use provided line number (typically 0 for file-level docs)
                        references: Vec::new(),
                        linked_source: None
                    })
                }
            })
    );

    DocumentationPage {
        fname: fname.to_string(),
        content,
        all_files: Vec::new() // Will be populated by for_file
    }
}

/// Aggregate the comments when there are considered to be documentation and associate them to the required token if any
/// Local labels (starting with ".") are resolved using the tracked parent global label
/// Returns: (doc_string, token_ref, parent_label, line_number)
pub fn aggregate_documentation_on_tokens<T: ListingElement + ToString + MayHaveSpan>(
    tokens: &[T],
    include_undocumented: bool
) -> Vec<(String, Option<&T>, Option<String>, usize)> {
    #[derive(PartialEq, Debug, Default, Clone, Copy)]
    enum CommentKind {
        #[default]
        Unspecified,
        Global,
        Local
    }

    #[derive(Default)]
    struct CommentInConstruction {
        kind: CommentKind,
        content: String
    }

    impl CommentInConstruction {
        fn consume(&mut self) -> String {
            self.kind = CommentKind::Unspecified;
            let comment = self.content.clone();
            self.content.clear();
            comment
        }

        fn clear(&mut self) {
            let _ = self.consume();
        }

        fn kind(&self) -> CommentKind {
            self.kind
        }

        fn set_kind(&mut self, kind: CommentKind) {
            self.kind = kind;
        }

        fn is_local(&self) -> bool {
            self.kind() == CommentKind::Local
        }

        fn is_global(&self) -> bool {
            self.kind() == CommentKind::Global
        }

        fn is_unspecified(&self) -> bool {
            self.kind() == CommentKind::Unspecified
        }

        fn add_comment(&mut self, comment: &str) {
            if !self.content.is_empty() {
                self.content += "\n";
            }

            // remove the ; that encode the documentation
            let comment = if self.is_global() {
                &comment[3..]
            }
            else {
                debug_assert!(self.is_local());
                &comment[2..]
            };

            // remove very first space if any
            let comment = if let Some(' ') = comment.chars().next() {
                &comment[1..]
            }
            else {
                comment
            };
            self.content += comment;
        }
    }

    let mut doc = Vec::new();

    let mut in_process_comment = CommentInConstruction::default();
    let mut last_global_label: Option<String> = None;

    for token in tokens {
        let (current_is_doc, current_is_documentable) = if is_global_documentation(token) {
            if in_process_comment.is_local() {
                // here, this is an error, there was a local comment and it is replaced by a global one
                // so, we lost it
                in_process_comment.clear();
            }
            in_process_comment.set_kind(CommentKind::Global);
            (true, false)
        }
        else if is_local_documentation(token) {
            if in_process_comment.is_global() {
                // here we can release the global comment
                doc.push((in_process_comment.consume(), None, None, 0));
            }
            in_process_comment.set_kind(CommentKind::Local);
            (true, false)
        }
        else {
            (false, is_documentable(token))
        };

        // Track the last global label for local label resolution
        if token.is_label() {
            let label = token.label_symbol().to_string();
            if !label.starts_with('.') {
                last_global_label = Some(label);
            }
        }

        if current_is_doc {
            // we update the documentation
            in_process_comment.add_comment(token.comment());
        }
        else if current_is_documentable {
            // Skip local labels without a parent
            let is_local_label = token.is_label() && token.label_symbol().starts_with('.');
            let should_skip = is_local_label && last_global_label.is_none();
            
            if !should_skip {
                let line_number = token.possible_span().map(|s| s.location_line()).unwrap_or(0) as usize;
                if !in_process_comment.is_unspecified() {
                    // we comment an item if any
                    let documented = if in_process_comment.is_global() {
                        // for a global comment, we do not care of that
                        None
                    }
                    else {
                        // but we do for a local comment
                        Some(token)
                    };
                    doc.push((in_process_comment.consume(), documented, last_global_label.clone(), line_number));
                }
                else if include_undocumented && is_documentable(token) {
                    // Include all undocumented items (labels, equs, macros, functions) if flag is set
                    doc.push((String::new(), Some(token), last_global_label.clone(), line_number));
                }
                else {
                    // we add no comment, so we do nothing
                }
            }
        }
        else {
            // this is not a doc or a documentable, so we can eventually treat a global
            if in_process_comment.is_global() {
                // For file-level documentation, line_number is 0 (or could be 1)
                doc.push((in_process_comment.consume(), None, None, 0));
            }
            else if in_process_comment.is_local() {
                // comment is lost as there is nothing else to comment
                in_process_comment.clear();
            }
        }
    }

    // The last comment can only be a global comment
    if in_process_comment.is_global() {
        doc.push((in_process_comment.consume(), None, None, 0));
    }

    doc
}

/// Apply syntax highlighting to Z80/BASM code using highlight.js compatible CSS classes
fn highlight_z80_syntax(source: &str) -> String {
    lazy_static! {
        // Build regex patterns from cpclib_asm constants
        static ref INSTRUCTIONS_REGEX: regex::Regex = {
            // SAFETY: INSTRUCTIONS constants are guaranteed to be valid UTF-8 by cpclib_asm
            let instructions = cpclib_asm::parser::instructions::INSTRUCTIONS
                .iter()
                .map(|s| unsafe { std::str::from_utf8_unchecked(s) });
            let pattern = format!(r"(?i)\b({})\b", instructions.collect::<Vec<_>>().join("|"));
            regex::Regex::new(&pattern).unwrap()
        };
        
        static ref DIRECTIVES_REGEX: regex::Regex = {
            use cpclib_asm::parser::parser::*;
            // SAFETY: Directive constants are guaranteed to be valid UTF-8 by cpclib_asm
            let directives = STAND_ALONE_DIRECTIVE.iter()
                .chain(START_DIRECTIVE.iter())
                .chain(END_DIRECTIVE.iter())
                .map(|s| unsafe { std::str::from_utf8_unchecked(s) });
            
            let pattern = format!(r"(?i)\b({})\b", directives.collect::<Vec<_>>().join("|"));
            regex::Regex::new(&pattern).unwrap()
        };
        
        static ref REGISTERS_REGEX: regex::Regex = {
            regex::Regex::new(
                r"(?i)\b(af|bc|de|hl|ix|iy|ixh|ixl|iyh|iyl|a|b|c|d|e|h|l|i|r|sp|pc|f)\b"
            ).unwrap()
        };
        
        static ref MACRO_PARAMS_REGEX: regex::Regex = {
            regex::Regex::new(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}").unwrap()
        };
        
        static ref NUMBERS_REGEX: regex::Regex = {
            regex::Regex::new(
                r"\b0x[0-9a-fA-F]+|\$[0-9a-fA-F]+|#[0-9a-fA-F]+|&[0-9a-fA-F]+|0b[01]+|%[01]+|\b[0-9]+\b"
            ).unwrap()
        };
        
        static ref STRINGS_REGEX: regex::Regex = {
            regex::Regex::new(r#""[^"]*"|'[^']*'"#).unwrap()
        };
        
        static ref COMMENTS_REGEX: regex::Regex = {
            // Match single-line comments per-line (multiline mode)
            regex::Regex::new(r"(?m);.*$").unwrap()
        };
        
        static ref MULTILINE_COMMENT_REGEX: regex::Regex = {
            // Use (?s) flag to make . match newlines, or use [\s\S] to match any character
            regex::Regex::new(r"(?s)/\*.*?\*/").unwrap()
        };
    }
    
    // Collect all matches with their positions, types, and matched text
    let mut matches: Vec<(usize, usize, &str)> = Vec::new();
    
    // Find multiline comments first (highest priority - they can span lines)
    for cap in MULTILINE_COMMENT_REGEX.find_iter(source) {
        matches.push((cap.start(), cap.end(), "comment"));
    }
    
    // Find single-line comments
    for cap in COMMENTS_REGEX.find_iter(source) {
        matches.push((cap.start(), cap.end(), "comment"));
    }
    
    // Sort matches by start position
    matches.sort_by_key(|m| m.0);
    
    // Merge overlapping comment ranges to find non-comment regions
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    for (start, end, _) in &matches {
        if let Some(last) = comment_ranges.last_mut() {
            if *start <= last.1 {
                // Overlapping or adjacent - merge
                last.1 = (*end).max(last.1);
                continue;
            }
        }
        comment_ranges.push((*start, *end));
    }
    
    // Find non-comment ranges (code regions)
    let mut code_ranges: Vec<(usize, usize)> = Vec::new();
    let mut last_end = 0;
    for (start, end) in &comment_ranges {
        if *start > last_end {
            code_ranges.push((last_end, *start));
        }
        last_end = *end;
    }
    if last_end < source.len() {
        code_ranges.push((last_end, source.len()));
    }
    
    // Find other syntax elements in code regions only
    let mut code_matches: Vec<(usize, usize, &str)> = Vec::new();
    
    for (code_start, code_end) in code_ranges {
        let code_part = &source[code_start..code_end];
        
        for cap in STRINGS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "string"));
        }
        for cap in NUMBERS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "number"));
        }
        for cap in MACRO_PARAMS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "meta"));
        }
        for cap in DIRECTIVES_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "built_in"));
        }
        for cap in INSTRUCTIONS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "keyword"));
        }
        for cap in REGISTERS_REGEX.find_iter(code_part) {
            code_matches.push((cap.start() + code_start, cap.end() + code_start, "variable"));
        }
    }
    
    // Merge all matches and sort by position
    matches.extend(code_matches);
    matches.sort_by_key(|m| m.0);
    
    // Build the result, applying highlighting and avoiding overlaps
    let mut result = String::new();
    let mut last_end = 0;
    
    for (start, end, class) in matches {
        if start >= last_end {
            // Add un-highlighted text before this match
            result.push_str(&source[last_end..start]);
            // Add highlighted match
            result.push_str(&format!("<span class=\"hljs-{}\">{}</span>", class, &source[start..end]));
            last_end = end;
        }
    }
    
    // Add any remaining un-highlighted text
    result.push_str(&source[last_end..]);
    
    result
}

/// Link symbols in source code to their documentation anchors
/// Applies after syntax highlighting to preserve both features
fn link_symbols_in_source(source: &str, symbols: &[(String, String)]) -> String {
    // First apply syntax highlighting
    let highlighted = highlight_z80_syntax(source);
    
    // Pre-compile all regexes once (HUGE performance improvement!)
    let mut symbol_regexes: Vec<(regex::Regex, String, String)> = Vec::new();
    for (name, key) in symbols {
        let pattern = format!(r"\b{}\b", regex::escape(name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            symbol_regexes.push((re, name.clone(), key.clone()));
        }
    }
    
    // Sort by length (longest first) to avoid partial matches
    symbol_regexes.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    
    // Then add symbol links - process text segments only
    let mut result = String::new();
    let mut chars = highlighted.chars().peekable();
    let mut in_tag = false;
    let mut in_anchor = false;
    let mut current_text = String::new();
    
    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Process accumulated text before tag
            if !in_tag && !in_anchor && !current_text.is_empty() {
                // Apply ALL symbol replacements to this text segment
                let mut text = current_text.clone();
                for (re, name, key) in &symbol_regexes {
                    let replacement = format!("<a href=\"#{}\" class=\"symbol-link\">{}</a>", key, name);
                    text = re.replace_all(&text, replacement.as_str()).to_string();
                }
                result.push_str(&text);
            } else {
                result.push_str(&current_text);
            }
            current_text.clear();
            
            in_tag = true;
            result.push(ch);
            
            // Check if this is an anchor tag
            let ahead: String = chars.clone().take(2).collect();
            if ahead == "a " || ahead == "a>" {
                in_anchor = true;
            } else if ahead == "/a" {
                in_anchor = false;
            }
        } else if ch == '>' && in_tag {
            in_tag = false;
            result.push(ch);
        } else if in_tag {
            result.push(ch);
        } else {
            current_text.push(ch);
        }
    }
    
    // Process remaining text
    if !in_tag && !in_anchor && !current_text.is_empty() {
        let mut text = current_text;
        for (re, name, key) in &symbol_regexes {
            let replacement = format!("<a href=\"#{}\" class=\"symbol-link\">{}</a>", key, name);
            text = re.replace_all(&text, replacement.as_str()).to_string();
        }
        result.push_str(&text);
    } else {
        result.push_str(&current_text);
    }
    
    result
}

/// Replace a symbol with a link in plain text (not inside HTML tags)
fn replace_symbol_in_text(text: &str, symbol: &str, key: &str) -> String {
    let pattern = format!(r"\b{}\b", regex::escape(symbol));
    if let Ok(re) = regex::Regex::new(&pattern) {
        let replacement = format!("<a href=\"#{}\" class=\"symbol-link\">{}</a>", key, symbol);
        re.replace_all(text, replacement.as_str()).to_string()
    } else {
        text.to_string()
    }
}

/// Extract symbols used in a token's expressions
fn extract_symbols_from_token<T: ListingElement + std::fmt::Display>(token: &T) -> Vec<String> {
    // Skip comments and label definitions (they're not references)
    if token.is_comment() || token.is_label() || token.is_macro_definition() || token.is_function_definition() {
        return Vec::new();
    }
    
    // Use the proper symbols() method from ListingElement
    token.symbols().into_iter().collect()
}

/// Collect symbol references inside macro/function content
/// Since we don't have parsed tokens for macro content, we search for symbol names manually
fn collect_references_in_content(
    content: &str,
    symbol_names: &[&str],
    source_file: Arc<str>,
    base_line: usize,
    all_symbols: &[(String, String)],
    highlight_cache: &DashMap<String, Arc<str>> // Shared concurrent cache
) -> HashMap<String, Vec<SymbolReference>> {
    let mut references: HashMap<String, Vec<SymbolReference>> = HashMap::new();
    
    // Pre-compile all regexes once (MAJOR OPTIMIZATION)
    let symbol_regexes: Vec<(&str, regex::Regex)> = symbol_names.iter()
        .filter_map(|&symbol| {
            let pattern = format!(r"\b{}\b", regex::escape(symbol));
            regex::Regex::new(&pattern).ok().map(|re| (symbol, re))
        })
        .collect();
    
    for (line_offset, line) in content.lines().enumerate() {
        let line_number = base_line + line_offset;
        
        // Use Cow to avoid allocation when no truncation needed
        let context: Cow<'static, str> = if line.chars().count() > 100 {
            let truncated: String = line.chars().take(100).collect();
            Cow::Owned(format!("{}...", truncated))
        } else {
            Cow::Owned(line.to_string())
        };
        
        // Search for each symbol in the line using pre-compiled regexes
        for (symbol, re) in &symbol_regexes {
            if re.is_match(line) {
                // Use shared cache to avoid re-highlighting the same context
                let context_key = context.to_string();
                let highlighted = highlight_cache
                    .entry(context_key)
                    .or_insert_with(|| Arc::from(link_symbols_in_source(&context, all_symbols).as_str()))
                    .clone();
                
                references
                    .entry(symbol.to_string())
                    .or_insert_with(Vec::new)
                    .push(SymbolReference {
                        source_file: Arc::clone(&source_file),
                        line_number,
                        context: context.clone(),
                        highlighted_context: highlighted
                    });
            }
        }
    }
    
    references
}

/// Collect cross-references by analyzing which symbols are used in which locations
fn collect_cross_references<T: ListingElement + std::fmt::Display>(
    tokens: &[T],
    source_file: Arc<str>,
    all_symbols: &[(String, String)],
    highlight_cache: &DashMap<String, Arc<str>> // Shared concurrent cache
) -> HashMap<String, Vec<SymbolReference>> {
    let mut references: HashMap<String, Vec<SymbolReference>> = HashMap::new();
    
    for (line_num, token) in tokens.iter().enumerate() {
        let symbols = extract_symbols_from_token(token);
        let token_str = token.to_string();
        
        // Use Cow to avoid allocation when no truncation needed
        let context: Cow<'static, str> = if token_str.chars().count() > 100 {
            let truncated: String = token_str.chars().take(100).collect();
            Cow::Owned(format!("{}...", truncated))
        } else {
            Cow::Owned(token_str)
        };
        
        for symbol in symbols {
            // Use shared cache to avoid re-highlighting the same context
            let context_key = context.to_string();
            let highlighted = highlight_cache
                .entry(context_key)
                .or_insert_with(|| Arc::from(link_symbols_in_source(&context, all_symbols).as_str()))
                .clone();
            
            references
                .entry(symbol)
                .or_insert_with(Vec::new)
                .push(SymbolReference {
                    source_file: Arc::clone(&source_file),
                    line_number: line_num + 1, // 1-indexed for display
                    context: context.clone(),
                    highlighted_context: highlighted
                });
        }
    }
    
    references
}

/// Populate cross-references in documentation page
fn populate_cross_references<T: ListingElement + std::fmt::Display>(mut page: DocumentationPage, tokens: &[T], all_symbols: &[(String, String)]) -> DocumentationPage {
    // Create shared cache for this operation
    let highlight_cache = DashMap::new();
    
    // Collect all references from tokens
    let source_file_arc: Arc<str> = Arc::from(page.fname.as_str());
    let all_refs = collect_cross_references(tokens, source_file_arc, all_symbols, &highlight_cache);
    
    // Match references to documented items
    for item in &mut page.content {
        let symbol_name = item.item_short_summary();
        
        if let Some(refs) = all_refs.get(&symbol_name) {
            item.references.extend(refs.clone());
        }
    }
    
    page
}

#[cfg(test)]
mod test {
    use cpclib_asm::Token;

    use crate::{aggregate_documentation_on_tokens, is_any_documentation, link_symbols_in_source};

    #[test]
    fn test_is_documentation() {
        assert!(!is_any_documentation(&Token::Comment(
            "; any comment".into()
        )));
        assert!(is_any_documentation(&Token::Comment(
            ";; any comment".into()
        )));
        assert!(is_any_documentation(&Token::Comment(
            ";;; any comment".into()
        )));
    }

    #[test]
    fn test_aggregate_global_documentation() {
        let tokens = [
            Token::Comment(";;; This file is commented, not the function!".into()),
            Token::Label("my_function".into())
        ];
        let doc = aggregate_documentation_on_tokens(&tokens, false);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, "This file is commented, not the function!");
        assert!(doc[0].1.is_none());
    }

    #[test]
    fn test_aggregate_global_documentation_followed_by_comment() {
        let tokens = [
            Token::Comment(";;; The aim of this file is to do stuffs.".into()),
            Token::Comment(";;; And this comment is a top file comment.".into()),
            Token::Comment("; This is not a documentation, just a comment".into())
        ];
        let doc = aggregate_documentation_on_tokens(&tokens, false);
        assert_eq!(doc.len(), 1);
        assert_eq!(
            &doc[0].0,
            "The aim of this file is to do stuffs.\nAnd this comment is a top file comment."
        );
        assert!(doc[0].1.is_none());
    }

    #[test]
    fn test_aggregate_label_comment() {
        let tokens = [
            Token::Comment(";; This function does something".into()),
            Token::Label("my_function".into())
        ];
        let doc = aggregate_documentation_on_tokens(&tokens, false);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, "This function does something");
        assert!(doc[0].1.is_some());
    }

    #[test]
    fn test_aggregate_label_merged_comment() {
        let tokens = [
            Token::Comment(";; This function does something ...".into()),
            Token::Comment(";; ... on two lines".into()),
            Token::Label("my_function".into())
        ];
        let doc = aggregate_documentation_on_tokens(&tokens, false);
        assert_eq!(doc.len(), 1);
        assert_eq!(
            &doc[0].0,
            "This function does something ...\n... on two lines"
        );
        assert!(doc[0].1.is_some());
    }

    #[test]
    fn test_aggregate_macro_comment() {
        let tokens = [
            Token::Comment(";; This macro does something".into()),
            Token::Macro {
                name: "macro_name".into(),
                params: Vec::new(),
                content: "".into(),
                tokenized_content: Default::default(),
                flavor: cpclib_asm::AssemblerFlavor::Basm
            }
        ];
        let doc = aggregate_documentation_on_tokens(&tokens, false);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, "This macro does something");
        assert!(doc[0].1.is_some());
    }

    #[test]
    fn test_link_symbols_in_source() {
        let symbols = vec![
            ("my_label".to_string(), "label_my_label".to_string()),
            ("other_func".to_string(), "function_other_func".to_string()),
        ];
        
        let source = "    ld hl, my_label\n    call other_func\n    ret";
        let linked = link_symbols_in_source(source, &symbols);
        
        println!("Generated output:\n{}", linked);
        
        // Verify that symbols are wrapped in links
        assert!(linked.contains("<a href=\"#label_my_label\" class=\"symbol-link\">my_label</a>"));
        assert!(linked.contains("<a href=\"#function_other_func\" class=\"symbol-link\">other_func</a>"));
        // Verify that instructions have syntax highlighting
        assert!(linked.contains("hljs-keyword"));
        // Verify that registers have syntax highlighting
        assert!(linked.contains("hljs-variable"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_instructions() {
        use crate::highlight_z80_syntax;
        
        let source = "ld a, 10\ncall my_func\nret";
        let highlighted = highlight_z80_syntax(source);
        
        // Should contain highlighted instructions
        assert!(highlighted.contains("<span class=\"hljs-keyword\">ld</span>"));
        assert!(highlighted.contains("<span class=\"hljs-keyword\">call</span>"));
        assert!(highlighted.contains("<span class=\"hljs-keyword\">ret</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_directives() {
        use crate::highlight_z80_syntax;
        
        let source = "macro TEST\n    org $4000\nendm";
        let highlighted = highlight_z80_syntax(source);
        
        // Should contain highlighted directives
        assert!(highlighted.contains("<span class=\"hljs-built_in\">macro</span>"));
        assert!(highlighted.contains("<span class=\"hljs-built_in\">org</span>"));
        assert!(highlighted.contains("<span class=\"hljs-built_in\">endm</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_registers() {
        use crate::highlight_z80_syntax;
        
        let source = "ld hl, bc\npush af";
        let highlighted = highlight_z80_syntax(source);
        
        // Should contain highlighted registers
        assert!(highlighted.contains("<span class=\"hljs-variable\">hl</span>"));
        assert!(highlighted.contains("<span class=\"hljs-variable\">bc</span>"));
        assert!(highlighted.contains("<span class=\"hljs-variable\">af</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_numbers() {
        use crate::highlight_z80_syntax;
        
        let source = "db $FF, #1234, 0x10, %10101010, 42";
        let highlighted = highlight_z80_syntax(source);
        
        // Should contain highlighted numbers
        assert!(highlighted.contains("<span class=\"hljs-number\">$FF</span>"));
        assert!(highlighted.contains("<span class=\"hljs-number\">#1234</span>"));
        assert!(highlighted.contains("<span class=\"hljs-number\">0x10</span>"));
        assert!(highlighted.contains("<span class=\"hljs-number\">%10101010</span>"));
        assert!(highlighted.contains("<span class=\"hljs-number\">42</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_strings() {
        use crate::highlight_z80_syntax;
        
        let source = r#"db "Hello", 'World'"#;
        let highlighted = highlight_z80_syntax(source);
        
        // Should contain highlighted strings
        assert!(highlighted.contains("<span class=\"hljs-string\">\"Hello\"</span>"));
        assert!(highlighted.contains("<span class=\"hljs-string\">'World'</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_single_line_comment() {
        use crate::highlight_z80_syntax;
        
        let source = "ld a, 10 ; load 10 into A";
        let highlighted = highlight_z80_syntax(source);
        
        // Should contain highlighted comment
        assert!(highlighted.contains("<span class=\"hljs-comment\">"));
        assert!(highlighted.contains("; load 10 into A"));
        // Instruction before comment should still be highlighted
        assert!(highlighted.contains("<span class=\"hljs-keyword\">ld</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_multiline_comment() {
        use crate::highlight_z80_syntax;
        
        let source = "ld a, 10 /* comment \n*/ call func";
        let highlighted = dbg!(highlight_z80_syntax(source));
        
        // Should contain highlighted multiline comment
        assert!(highlighted.contains("<span class=\"hljs-comment\">/* comment \n*/</span>"));
        // Instructions around comment should still be highlighted
        assert!(highlighted.contains("<span class=\"hljs-keyword\">ld</span>"));
        assert!(highlighted.contains("<span class=\"hljs-keyword\">call</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_case_insensitive() {
        use crate::highlight_z80_syntax;
        
        let source = "LD a, 10\nCALL func\nRET";
        let highlighted = highlight_z80_syntax(source);
        
        // Should highlight regardless of case
        assert!(highlighted.contains("<span class=\"hljs-keyword\">LD</span>"));
        assert!(highlighted.contains("<span class=\"hljs-keyword\">CALL</span>"));
        assert!(highlighted.contains("<span class=\"hljs-keyword\">RET</span>"));
    }
    
    #[test]
    fn test_highlight_z80_syntax_macro_parameters() {
        use crate::highlight_z80_syntax;
        
        let source = "ld a, {param1}\nadd {value}\nld hl, {address}";
        let highlighted = highlight_z80_syntax(source);
        
        // Macro parameters should be highlighted
        assert!(highlighted.contains("<span class=\"hljs-meta\">{param1}</span>"));
        assert!(highlighted.contains("<span class=\"hljs-meta\">{value}</span>"));
        assert!(highlighted.contains("<span class=\"hljs-meta\">{address}</span>"));
        // Instructions should still be highlighted
        assert!(highlighted.contains("<span class=\"hljs-keyword\">ld</span>"));
        assert!(highlighted.contains("<span class=\"hljs-keyword\">add</span>"));
    }
}

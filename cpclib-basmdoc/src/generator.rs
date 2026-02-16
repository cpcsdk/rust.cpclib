//! Documentation generator orchestration with builder pattern
//!
//! This module provides the `BasmDocGenerator` struct with a builder pattern
//! to orchestrate the complete documentation generation workflow:
//! 1. File discovery (with wildcard/directory support)
//! 2. Documentation parsing
//! 3. Cross-reference collection  
//! 4. Rendering to HTML or Markdown

use std::path::{Path, PathBuf};

use indicatif::{ProgressBar, ProgressStyle};

use crate::{DocumentationPage, UndocumentedConfig};

/// Builder for configuring and generating documentation
#[derive(Debug, Clone)]
pub struct BasmDocGenerator {
    /// Input files or patterns to process
    inputs: Vec<String>,
    /// Whether to expand wildcards in input patterns
    enable_wildcards: bool,
    /// Configuration for including undocumented items
    undocumented_config: UndocumentedConfig,
    /// Optional title for the documentation
    title: Option<String>,
    /// Whether to show progress indicators
    show_progress: bool,
    /// Whether to minify HTML output
    minify_html: bool
}

impl Default for BasmDocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl BasmDocGenerator {
    /// Create a new documentation generator with default settings
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            enable_wildcards: false,
            undocumented_config: UndocumentedConfig::default(),
            title: None,
            show_progress: true,
            minify_html: true
        }
    }

    /// Add input files or directories to process
    pub fn add_inputs<I, S>(mut self, inputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>
    {
        self.inputs.extend(inputs.into_iter().map(|s| s.into()));
        self
    }

    /// Enable wildcard expansion on input patterns
    pub fn with_wildcards(mut self, enable: bool) -> Self {
        self.enable_wildcards = enable;
        self
    }

    /// Configure which undocumented items to include
    pub fn with_undocumented_config(mut self, config: UndocumentedConfig) -> Self {
        self.undocumented_config = config;
        self
    }

    /// Set the title for the documentation
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Control whether to show progress indicators
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Enable or disable HTML minification
    pub fn with_minify(mut self, minify: bool) -> Self {
        self.minify_html = minify;
        self
    }

    /// Resolve input patterns to concrete file paths
    ///
    /// This handles:
    /// - Wildcard expansion (if enabled)
    /// - Directory traversal (recursively finds .asm files)
    /// - Direct file paths
    pub fn resolve_inputs(&self) -> Result<Vec<String>, String> {
        if self.inputs.is_empty() {
            return Err("No input files specified".to_string());
        }

        let resolved = if self.enable_wildcards {
            self.resolve_wildcards()?
        }
        else {
            self.resolve_files_and_dirs()?
        };

        if resolved.is_empty() {
            return Err("No assembly files found for the given inputs".to_string());
        }

        Ok(resolved)
    }

    /// Resolve inputs using wildcard patterns
    fn resolve_wildcards(&self) -> Result<Vec<String>, String> {
        use cpclib_common::itertools::Itertools;

        let expanded: Vec<String> = self
            .inputs
            .iter()
            .flat_map(|input| {
                glob::glob(input)
                    .map_err(|e| format!("Invalid wildcard pattern {}: {}", input, e))
                    .and_then(|paths| {
                        paths
                            .map(|res| res.map(|p| p.to_string_lossy().to_string()))
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| {
                                format!("Error reading files for pattern {}: {}", input, e)
                            })
                    })
            })
            .flatten()
            .collect_vec();

        if expanded.is_empty() {
            return Err("No input files found for the given patterns.".to_string());
        }

        Ok(expanded)
    }

    /// Resolve inputs by traversing files and directories
    fn resolve_files_and_dirs(&self) -> Result<Vec<String>, String> {
        let mut resolved = Vec::new();

        let spinner = if self.show_progress {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap()
            );
            pb.set_message("Searching for assembly files...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        }
        else {
            None
        };

        for input in &self.inputs {
            let path = Path::new(input);

            if path.is_dir() {
                resolved.extend(Self::find_asm_files(path)?);
            }
            else if path.is_file() {
                resolved.push(input.to_string());
            }
            else {
                if let Some(s) = &spinner {
                    s.finish_and_clear();
                }
                return Err(format!(
                    "Input '{}' is neither a file nor a directory",
                    input
                ));
            }
        }

        if let Some(s) = spinner {
            s.finish_with_message(format!("Found {} assembly files", resolved.len()));
        }

        Ok(resolved)
    }

    /// Recursively find all .asm files in a directory
    fn find_asm_files(path: &Path) -> Result<Vec<String>, String> {
        let mut asm_files = Vec::new();

        let entries = fs_err::read_dir(path)
            .map_err(|e| format!("Failed to read directory {}: {}", path.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                asm_files.extend(Self::find_asm_files(&entry_path)?);
            }
            else if entry_path.is_file() {
                if let Some(ext) = entry_path.extension() {
                    if ext.eq_ignore_ascii_case("asm") {
                        asm_files.push(entry_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(asm_files)
    }

    /// Calculate the longest common prefix for display names
    pub fn calculate_common_prefix(paths: &[String]) -> String {
        if paths.is_empty() {
            return String::new();
        }

        if paths.len() == 1 {
            // For a single file, use its parent directory as prefix
            let path = Path::new(&paths[0]);
            let parent = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            // Normalize path separator to forward slash and strip leading slash
            // if the path is not considered absolute by the OS
            let normalized = parent.replace('\\', "/");
            if !path.is_absolute() && normalized.starts_with('/') {
                return normalized.trim_start_matches('/').to_string();
            }
            return normalized;
        }

        // Check if all paths are absolute
        let all_absolute = paths.iter().all(|p| Path::new(p).is_absolute());

        // Store the prefix component (for Windows drive letters)
        let prefix_component = if all_absolute {
            Path::new(&paths[0]).components().next().and_then(|c| {
                if let std::path::Component::Prefix(prefix) = c {
                    Some(prefix.as_os_str().to_string_lossy().to_string())
                }
                else {
                    None
                }
            })
        }
        else {
            None
        };

        // Convert all paths to components for comparison
        let path_components: Vec<Vec<&str>> = paths
            .iter()
            .map(|p| {
                Path::new(p)
                    .components()
                    .filter_map(|c| {
                        if let std::path::Component::Normal(s) = c {
                            s.to_str()
                        }
                        else {
                            None
                        }
                    })
                    .collect()
            })
            .collect();

        // Find the minimum number of components
        let min_len = path_components.iter().map(|v| v.len()).min().unwrap_or(0);

        // Find common prefix components
        let mut common = Vec::new();
        for i in 0..min_len {
            let first = path_components[0][i];
            if path_components
                .iter()
                .all(|components| components[i] == first)
            {
                common.push(first);
            }
            else {
                break;
            }
        }

        // Don't include the filename itself in the prefix
        if !common.is_empty() && common.len() == min_len {
            common.pop();
        }

        if common.is_empty() {
            String::new()
        }
        else {
            let prefix = common.join("/");
            // Add Windows drive letter or Unix root slash
            if let Some(win_prefix) = prefix_component {
                format!("{}/{}", win_prefix, prefix)
            }
            else if all_absolute {
                format!("/{}", prefix)
            }
            else {
                prefix
            }
        }
    }

    /// Remove the common prefix from a path to create a display name
    pub fn remove_prefix(path: &str, prefix: &str) -> String {
        if prefix.is_empty() {
            return path.to_string();
        }

        let path = path.replace("\\", "/");

        // Handle both absolute and relative paths
        let prefix_patterns = vec![
            format!("/{}/", prefix), // /prefix/
            format!("{}/", prefix),  // prefix/
        ];

        for pattern in &prefix_patterns {
            if let Some(stripped) = path.strip_prefix(pattern) {
                return stripped.to_string();
            }
        }

        // If path equals prefix, return just the filename
        if path.ends_with(&format!("/{}", prefix)) || path == prefix {
            return Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&path)
                .to_string();
        }

        path
    }

    /// Generate merged HTML documentation from all inputs
    ///
    /// This is the main entry point for HTML generation. It:
    /// 1. Resolves input files
    /// 2. Parses all files (collecting both pages and tokens)
    /// 3. Merges pages into a single documentation page
    /// 4. Populates cross-references across all files
    /// 5. Renders to HTML
    pub fn generate_html(&self) -> Result<String, String> {
        let inputs = self.resolve_inputs()?;
        let common_prefix = Self::calculate_common_prefix(&inputs);

        // Parse all files with progress tracking
        let pb_parse = if self.show_progress {
            let pb = ProgressBar::new(inputs.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                    .unwrap()
                    .progress_chars("#>-")
            );
            pb.set_message("Parsing assembly files");
            Some(pb)
        }
        else {
            None
        };

        let pages_and_tokens: Result<Vec<_>, String> = inputs
            .into_iter()
            .map(|input| {
                let display_name = Self::remove_prefix(&input, &common_prefix);
                let result = DocumentationPage::for_file_without_refs(
                    &input,
                    &display_name,
                    self.undocumented_config
                )
                .map(|(page, tokens)| (page, display_name, tokens));

                if let Some(pb) = &pb_parse {
                    pb.inc(1);
                }

                result
            })
            .collect();

        if let Some(pb) = pb_parse {
            pb.finish_with_message("Parsing complete");
        }

        let pages_and_tokens = pages_and_tokens?;

        // Separate pages and tokens
        let pages: Vec<_> = pages_and_tokens
            .iter()
            .map(|(page, ..)| page.clone())
            .collect();
        let all_tokens: Vec<(String, _)> = pages_and_tokens
            .into_iter()
            .map(|(_, display_name, tokens)| (display_name, tokens))
            .collect();

        // Merge pages
        let spinner_merge = if self.show_progress {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap()
            );
            pb.set_message("Merging documentation pages...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        }
        else {
            None
        };

        let merged_page = DocumentationPage::merge(pages);

        if let Some(s) = spinner_merge {
            s.finish_with_message("Merge complete");
        }

        // Populate cross-references
        let merged_page = merged_page.populate_all_cross_references(&all_tokens);

        // Generate HTML
        let spinner = if self.show_progress {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap()
            );
            pb.set_message("Generating HTML documentation...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        }
        else {
            None
        };

        let html = merged_page.to_html(self.title.as_deref());

        if let Some(s) = spinner {
            s.finish_with_message("HTML generation complete");
        }

        Ok(html)
    }

    /// Generate merged Markdown documentation from all inputs
    ///
    /// Simpler than HTML generation - just parses files and merges markdown output
    pub fn generate_markdown(&self) -> Result<String, String> {
        let inputs = self.resolve_inputs()?;
        let common_prefix = Self::calculate_common_prefix(&inputs);

        let pb = if self.show_progress {
            let pb = ProgressBar::new(inputs.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                    .unwrap()
                    .progress_chars("#>-")
            );
            pb.set_message("Generating markdown documentation");
            Some(pb)
        }
        else {
            None
        };

        let docs: Vec<String> = inputs
            .into_iter()
            .map(|input| {
                let display_name = Self::remove_prefix(&input, &common_prefix);
                let result =
                    DocumentationPage::for_file(&input, &display_name, self.undocumented_config)
                        .map(|page| page.to_markdown())
                        .unwrap_or_else(|e| {
                            format!("**Error generating documentation:**\n\n```\n{}\n```\n", e)
                        });

                if let Some(pb) = &pb {
                    pb.inc(1);
                }

                result
            })
            .collect();

        if let Some(pb) = pb {
            pb.finish_with_message("Markdown generation complete");
        }

        Ok(docs.join("\n\n---\n\n"))
    }

    /// Save documentation to a file, choosing format based on extension
    ///
    /// Supported formats:
    /// - `.html` / `.htm` - HTML output
    /// - `.md` - Markdown output
    /// - `.pdf` - PDF output (requires pandoc)
    pub fn save_to_file<P: AsRef<Path>>(&self, output_path: P) -> Result<(), String> {
        let output_path = output_path.as_ref();

        let ext = output_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| {
                "Output file must have an extension (.html, .md, or .pdf)".to_string()
            })?;

        let is_md = ext.eq_ignore_ascii_case("md");
        let is_html = ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm");
        let is_pdf = ext.eq_ignore_ascii_case("pdf");

        if is_html {
            let html = self.generate_html().map_err(|e| {
                format!(
                    "<p><strong>Error generating documentation:</strong></p><pre>{}</pre>",
                    e
                )
            })?;

            // Minify HTML if requested
            let final_html = if self.minify_html {
                self.minify_html_content(&html)?
            }
            else {
                html
            };

            fs_err::write(output_path, final_html)
                .map_err(|e| format!("Unable to write {} file. {}", output_path.display(), e))?;
        }
        else if is_md {
            let md = self.generate_markdown()?;

            fs_err::write(output_path, md)
                .map_err(|e| format!("Unable to write {} file. {}", output_path.display(), e))?;
        }
        else if is_pdf {
            // Generate markdown first, then convert to PDF using pandoc
            let md = self.generate_markdown()?;

            let md_path = output_path.with_extension("md");
            fs_err::write(&md_path, md)
                .map_err(|e| format!("Unable to write temporary markdown file. {}", e))?;

            let mut pandoc = pandoc::new();
            pandoc.add_input(&md_path);
            pandoc.set_output(pandoc::OutputKind::File(output_path.into()));
            pandoc.add_option(pandoc::PandocOption::Standalone);
            pandoc.add_option(pandoc::PandocOption::TableOfContents);
            pandoc
                .execute()
                .map_err(|e| format!("Pandoc error: {}", e))?;
        }
        else {
            return Err("Output file must have .md, .html, or .pdf extension".to_string());
        }

        Ok(())
    }

    /// Minify HTML content to reduce file size
    #[cfg(feature = "minify")]
    fn minify_html_content(&self, html: &str) -> Result<String, String> {
        let cfg = minify_html::Cfg {
            keep_closing_tags: true,
            keep_html_and_head_opening_tags: true,
            keep_comments: false,
            minify_css: true,
            minify_js: true,
            minify_doctype: true,
            remove_bangs: false,
            remove_processing_instructions: false,
            keep_input_type_text_attr: false,
            keep_ssi_comments: false,
            preserve_brace_template_syntax: false,
            preserve_chevron_percent_template_syntax: false,
            allow_noncompliant_unquoted_attribute_values: false,
            allow_optimal_entities: true,
            allow_removing_spaces_between_attributes: true
        };

        let minified = minify_html::minify(html.as_bytes(), &cfg);
        String::from_utf8(minified)
            .map_err(|e| format!("Failed to convert minified HTML to UTF-8: {}", e))
    }

    /// No-op when minify feature is disabled
    #[cfg(not(feature = "minify"))]
    fn minify_html_content(&self, html: &str) -> Result<String, String> {
        Ok(html.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_prefix_single_file() {
        #[cfg(unix)]
        let paths = vec!["/home/user/project/src/main.asm".to_string()];
        #[cfg(not(unix))]
        let paths = vec!["C:\\Users\\user\\project\\src\\main.asm".to_string()];

        let prefix = BasmDocGenerator::calculate_common_prefix(&paths);

        #[cfg(unix)]
        assert_eq!(prefix, "/home/user/project/src");
        #[cfg(not(unix))]
        assert_eq!(prefix, "C:/Users/user/project/src");
    }

    #[test]
    fn test_common_prefix_multiple_files() {
        #[cfg(unix)]
        let paths = vec![
            "/home/user/project/src/main.asm".to_string(),
            "/home/user/project/src/util.asm".to_string(),
            "/home/user/project/src/lib/helper.asm".to_string(),
        ];
        #[cfg(not(unix))]
        let paths = vec![
            "C:\\Users\\user\\project\\src\\main.asm".to_string(),
            "C:\\Users\\user\\project\\src\\util.asm".to_string(),
            "C:\\Users\\user\\project\\src\\lib\\helper.asm".to_string(),
        ];

        let prefix = BasmDocGenerator::calculate_common_prefix(&paths);

        #[cfg(unix)]
        assert_eq!(prefix, "/home/user/project/src");
        #[cfg(not(unix))]
        assert_eq!(prefix, "C:/Users/user/project/src");
    }

    #[test]
    fn test_remove_prefix() {
        let path = "/home/user/project/src/main.asm";
        let prefix = "/home/user/project/src";
        let result = BasmDocGenerator::remove_prefix(path, prefix);
        assert_eq!(result, "main.asm");
    }

    #[test]
    fn test_builder_pattern() {
        let generator = BasmDocGenerator::new()
            .add_inputs(vec!["test.asm"])
            .with_wildcards(true)
            .with_undocumented_config(UndocumentedConfig::all())
            .with_title("Test Documentation")
            .with_progress(false);

        assert_eq!(generator.inputs.len(), 1);
        assert!(generator.enable_wildcards);
        assert!(generator.undocumented_config.macros);
        assert_eq!(generator.title, Some("Test Documentation".to_string()));
        assert!(!generator.show_progress);
    }
}

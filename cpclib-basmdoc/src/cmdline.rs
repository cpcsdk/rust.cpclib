use cpclib_common::clap;
use cpclib_common::itertools::Itertools;
use indicatif::{ProgressBar, ProgressStyle};

use crate::DocumentationPage;

pub fn build_args_parser() -> clap::Command {
    clap::Command::new("basmdoc")
        .about("Generates assembly documentation in markdown format")
        .arg(
            clap::Arg::new("wildcards")
                .help("Enable wildcard expansion on input files")
                .short('w')
                .long("wildcards")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("input")
                .help("Input assembly file(s) or folder(s) (recursively searches for .asm files in folders)")
                .required(true)
                .num_args(1..)
        )
        .arg(
            clap::Arg::new("output")
                .help("Output markdown file")
                .short('o')
                .long("output")
                .required(true)
        )
        .arg(
            clap::Arg::new("undocumented")
                .help("Include all undocumented symbols (macros, functions, labels, equs)")
                .short('u')
                .long("undocumented")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-macros")
                .help("Include undocumented macros")
                .long("undocumented-macros")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-functions")
                .help("Include undocumented functions")
                .long("undocumented-functions")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-labels")
                .help("Include undocumented labels")
                .long("undocumented-labels")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("undocumented-equs")
                .help("Include undocumented equs")
                .long("undocumented-equs")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            clap::Arg::new("title")
                .help("Output title")
                .short('t')
                .long("title")
                .required(false)
        )
}

/// Find the longest common prefix of all paths (up to directory boundary)
fn longest_common_prefix(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    
    if paths.len() == 1 {
        // For a single file, use its parent directory as prefix
        let path = std::path::Path::new(&paths[0]);
        return path.parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
    }
    
    // Convert all paths to components for comparison
    let path_components: Vec<Vec<&str>> = paths
        .iter()
        .map(|p| {
            std::path::Path::new(p)
                .components()
                .filter_map(|c| {
                    if let std::path::Component::Normal(s) = c {
                        s.to_str()
                    } else {
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
        if path_components.iter().all(|components| components[i] == first) {
            common.push(first);
        } else {
            break;
        }
    }
    
    // Don't include the filename itself in the prefix
    if !common.is_empty() && common.len() == min_len {
        common.pop();
    }
    
    if common.is_empty() {
        String::new()
    } else {
        common.join("/")
    }
}

/// Remove the common prefix from a path
fn remove_prefix(path: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return path.to_string();
    }
    
    let path = path.replace("\\", "/");
    
    // Handle both absolute and relative paths
    let prefix_patterns = vec![
        format!("/{}/", prefix),  // /prefix/
        format!("{}/", prefix),   // prefix/
    ];
    
    for pattern in &prefix_patterns {
        if let Some(stripped) = path.strip_prefix(pattern) {
            return stripped.to_string();
        }
    }
    
    // If path equals prefix, return just the filename
    if path.ends_with(&format!("/{}", prefix)) || path == prefix {
        return std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&path)
            .to_string();
    }
    
    path
}

/// Recursively find all .asm files in a directory
fn find_asm_files(path: &std::path::Path) -> Result<Vec<String>, String> {
    let mut asm_files = Vec::new();
    
    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory {}: {}", path.display(), e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        
        if path.is_dir() {
            // Recursively search subdirectories
            asm_files.extend(find_asm_files(&path)?);
        } else if path.is_file() {
            // Check if file has .asm extension
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("asm") {
                    asm_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    Ok(asm_files)
}

pub fn handle_matches(matches: &clap::ArgMatches, cmd: &clap::Command) -> Result<(), String> {
    let inputs = matches.get_many::<String>("input").expect("required");
    let output = matches.get_one::<String>("output").expect("required");

    let inputs = if matches.get_flag("wildcards") {
        let expanded_inputs = inputs
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

        if expanded_inputs.is_empty() {
            return Err("No input files found for the given patterns.".to_string());
        }

        expanded_inputs
    }
    else {
        // Handle both files and directories
        let mut expanded_inputs = Vec::new();
        
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        spinner.set_message("Searching for assembly files...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        for input in inputs {
            let path = std::path::Path::new(input);
            
            if path.is_dir() {
                // It's a directory - find all .asm files recursively
                expanded_inputs.extend(find_asm_files(path)?);
            } else if path.is_file() {
                // It's a file - add it directly
                expanded_inputs.push(input.to_string());
            } else {
                spinner.finish_and_clear();
                return Err(format!("Input '{}' is neither a file nor a directory", input));
            }
        }
        
        spinner.finish_with_message(format!("Found {} assembly files", expanded_inputs.len()));
        
        if expanded_inputs.is_empty() {
            return Err("No .asm files found in the provided directories".to_string());
        }
        
        expanded_inputs
    };
    let inputs = inputs.into_iter();

    let output = std::path::Path::new(output);
    
    // Build undocumented config from flags
    let include_undocumented = if matches.get_flag("undocumented") {
        crate::UndocumentedConfig::all()
    } else {
        crate::UndocumentedConfig {
            macros: matches.get_flag("undocumented-macros"),
            functions: matches.get_flag("undocumented-functions"),
            labels: matches.get_flag("undocumented-labels"),
            equs: matches.get_flag("undocumented-equs"),
        }
    };

    // Calculate common prefix for all input files
    let input_vec: Vec<String> = inputs.collect();
    let common_prefix = longest_common_prefix(&input_vec);
    let inputs = input_vec.into_iter();

    if let Some(ext) = output.extension() {
        let is_md = ext.eq_ignore_ascii_case("md");
        let is_html = ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm");

        if is_html {
            // Generate HTML directly using minijinja - merge all pages into one
            // Parse all files and collect both pages and tokens
            let input_vec: Vec<_> = inputs.collect();
            
            let pb_parse = ProgressBar::new(input_vec.len() as u64);
            pb_parse.set_style(ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            pb_parse.set_message("Parsing assembly files");
            
            let pages_and_tokens: Result<Vec<_>, String> = input_vec.into_iter()
                .map(|input| {
                    let display_name = remove_prefix(&input, &common_prefix);
                    let result = DocumentationPage::for_file_without_refs(&input, &display_name, include_undocumented)
                        .map(|(page, tokens)| (page, display_name, tokens));
                    pb_parse.inc(1);
                    result
                })
                .collect();
            
            pb_parse.finish_with_message("Parsing complete");
            
            let html = match pages_and_tokens {
                Ok(pages_and_tokens) => {
                    // Separate pages and tokens with their source file names
                    let pages: Vec<_> = pages_and_tokens.iter()
                        .map(|(page, _, _)| page.clone())
                        .collect();
                    let all_tokens: Vec<(String, _)> = pages_and_tokens.into_iter()
                        .map(|(_, display_name, tokens)| (display_name, tokens))
                        .collect();
                    
                    // Merge pages and then populate cross-references from ALL files
                    let spinner_merge = ProgressBar::new_spinner();
                    spinner_merge.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.cyan} {msg}")
                            .unwrap()
                    );
                    spinner_merge.set_message("Merging documentation pages...");
                    spinner_merge.enable_steady_tick(std::time::Duration::from_millis(100));
                    
                    let merged_page = DocumentationPage::merge(pages);
                    spinner_merge.finish_with_message("Merge complete");
                    
                    let merged_page = merged_page.populate_all_cross_references(&all_tokens);
                    
                    let spinner = ProgressBar::new_spinner();
                    spinner.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.cyan} {msg}")
                            .unwrap()
                    );
                    spinner.set_message("Generating HTML documentation...");
                    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
                    
                    let html = merged_page.to_html();
                    spinner.finish_with_message("HTML generation complete");
                    html
                },
                Err(e) => format!("<p><strong>Error generating documentation:</strong></p><pre>{}</pre>", e)
            };
            
            // Save the HTML file directly
            std::fs::write(output, html)
                .map_err(|e| format!("Unable to write {} file. {e}", output.display()))?;
        }
        else {
            // Generate markdown (for .md or PDF)
            let mut docs = inputs
                .map(|input| {
                    let display_name = remove_prefix(&input, &common_prefix);
                    DocumentationPage::for_file(&input, &display_name, include_undocumented)
                })
                .map(|page| page.map(|page| page.to_markdown()))
                .map(|res| {
                    match res {
                        Ok(md) => md,
                        Err(e) => format!("**Error generating documentation:**\n\n```\n{}\n```\n", e)
                    }
                });
            let md = docs.join("\n\n---\n\n");

            let md_fname = if is_md {
                output.to_owned()
            }
            else {
                output.with_extension(".md") // TODO create a temp file instead?
            };

            // save the markdown file
            std::fs::write(&md_fname, md)
                .map_err(|e| format!("Unable to write {} file. {e}", md_fname.display()))?;

            // export to the final output if needed (PDF)
            if !is_md {
                let mut pandoc = pandoc::new();
                pandoc.add_input(&md_fname);
                pandoc.set_output(pandoc::OutputKind::File(output.into()));
                pandoc.add_option(pandoc::PandocOption::Standalone);
                pandoc.add_option(pandoc::PandocOption::TableOfContents);
                pandoc
                    .execute()
                    .map_err(|e| format!("Pandoc error: {}", e))?;
            }
        }
    }
    else {
        return Err("Output file must have .md, .html, or PDF extension".to_string());
    }

    Ok(())
}

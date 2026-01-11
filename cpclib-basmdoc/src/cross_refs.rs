//! Cross-reference collection module for tracking symbol usage across source files.
//!
//! This module analyzes token streams to identify where symbols (labels, functions, macros)
//! are referenced, creating a map of symbol usage with highlighted context.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use cpclib_asm::ListingElement;
use dashmap::DashMap;

use crate::models::SymbolReference;
use crate::syntax;

/// Extract symbols used in a token's expressions
pub(crate) fn extract_symbols_from_token<T: ListingElement + std::fmt::Display>(token: &T) -> Vec<String> {
    // Skip comments and label definitions (they're not references)
    if token.is_comment() || token.is_label() || token.is_macro_definition() || token.is_function_definition() {
        return Vec::new();
    }
    
    // Use the proper symbols() method from ListingElement
    token.symbols().into_iter().collect()
}

/// Collect symbol references inside macro/function content
/// Since we don't have parsed tokens for macro content, we search for symbol names manually
pub(crate) fn collect_references_in_content(
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
                    .or_insert_with(|| Arc::from(syntax::link_symbols_in_source(&context, all_symbols).as_str()))
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
pub(crate) fn collect_cross_references<T: ListingElement + std::fmt::Display>(
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
                .or_insert_with(|| Arc::from(syntax::link_symbols_in_source(&context, all_symbols).as_str()))
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

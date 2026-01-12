//! Parsing module for documentation extraction from assembly source code.
//!
//! This module handles the parsing of Z80/BASM source code to extract
//! documentation comments and their associated items (labels, functions, macros, etc.).

use cpclib_asm::{ListingElement, MayHaveSpan};

use crate::models::{DocumentedItem, ItemDocumentation, UndocumentedConfig};
use crate::DocumentationPage;

/// Marker for global documentation comments (file-level or section-level)
const GLOBAL_DOCUMENTATION_START: &str = ";;;";

/// Marker for local documentation comments (item-level)
const LOCAL_DOCUMENTATION_START: &str = ";;";

/// Recursively collect all tokens including those inside conditional branches
///
/// This function expands IF/ELSE/ENDIF structures to extract documentation
/// from all conditional branches, not just the active one at parse time.
///
/// # Arguments
/// * `tokens` - The token stream to expand
/// * `result` - Vector to accumulate all tokens (including nested conditional content)
fn collect_all_tokens_including_conditionals<'a, T: ListingElement>(
    tokens: &'a [T],
    result: &mut Vec<&'a T>
) {
    for token in tokens {
        result.push(token);
        
        // If this is a conditional token, recursively process all branches
        if token.is_if() {
            let nb_tests = token.if_nb_tests();
            
            // Process each condition branch
            for idx in 0..nb_tests {
                let (_condition, branch_tokens) = token.if_test(idx);
                collect_all_tokens_including_conditionals(branch_tokens, result);
            }
            
            // Process the else branch if it exists
            if let Some(else_tokens) = token.if_else() {
                collect_all_tokens_including_conditionals(else_tokens, result);
            }
        }
        // Also handle other nested structures
        else if token.is_repeat() {
            let listing = token.repeat_listing();
            collect_all_tokens_including_conditionals(listing, result);
        }
        else if token.is_for() {
            let listing = token.for_listing();
            collect_all_tokens_including_conditionals(listing, result);
        }
        else if token.is_while() {
            let listing = token.while_listing();
            collect_all_tokens_including_conditionals(listing, result);
        }
        else if token.is_iterate() {
            let listing = token.iterate_listing();
            collect_all_tokens_including_conditionals(listing, result);
        }
        else if token.is_macro_definition() {
            // Note: Macro content is typically stored as a string, not as parsed tokens
            // So we don't recurse into macro bodies
        }
        else if token.is_function_definition() {
            // we don't recurse into function bodies either
        }
        else if token.is_module() {
            let listing = token.module_listing();
            collect_all_tokens_including_conditionals(listing, result);
        }
    }
}
fn is_any_documentation<T: ListingElement>(token: &T) -> bool {
    token.is_comment() && token.comment().starts_with(LOCAL_DOCUMENTATION_START)
}

/// Check if a token is a global documentation comment (;;;)
fn is_global_documentation<T: ListingElement>(token: &T) -> bool {
    is_any_documentation(token) && token.comment().starts_with(GLOBAL_DOCUMENTATION_START)
}

/// Check if a token is a local documentation comment (;; but not ;;;)
fn is_local_documentation<T: ListingElement>(token: &T) -> bool {
    is_any_documentation(token) && !is_global_documentation(token)
}

/// Check if a token can be documented (is it a label, equ, function, or macro?)
fn is_documentable<T: ListingElement + ToString>(token: &T) -> bool {
    documentation_type(token, None).is_some()
}

/// Determine the type of documentable item
fn documentation_type<T: ListingElement + ToString>(
    token: &T,
    last_global_label: Option<&str>
) -> Option<DocumentedItem> {
    if token.is_label() {
        let mut label = token.label_symbol().to_string();
        
        // Handle local labels (starting with .)
        if label.starts_with('.') {
            if let Some(parent) = last_global_label {
                label = format!("{}{}", parent, label);
            } else {
                // Local label without a parent is invalid
                return None;
            }
        }
        
        Some(DocumentedItem::Label(label))
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
            content: token.to_string()
        })
    }
    else {
        None
    }
}

/// Build a documentation page from aggregated documentation items
///
/// # Arguments
/// * `fname` - The source file name
/// * `agg` - Aggregated documentation tuples: (doc_string, token_ref, parent_label, line_number)
///
/// # Returns
/// A DocumentationPage with all items properly linked
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

/// Aggregate documentation comments and associate them with their documented items
///
/// This function processes a token stream and:
/// - Collects consecutive documentation comments (;; or ;;;)
/// - Associates them with the next documentable token (label, equ, function, macro)
/// - Resolves local labels (starting with ".") using the tracked parent global label
/// - Optionally includes undocumented items based on configuration
/// - **Recursively processes all conditional branches** (IF/ELSE/ENDIF) to extract
///   documentation from commented-out code paths
///
/// # Arguments
/// * `tokens` - The token stream from the assembler
/// * `include_undocumented` - Configuration for which undocumented items to include
///
/// # Returns
/// A vector of tuples: (doc_string, token_ref, parent_label, line_number)
pub fn aggregate_documentation_on_tokens<T: ListingElement + ToString + MayHaveSpan>(
    tokens: &[T],
    include_undocumented: UndocumentedConfig
) -> Vec<(String, Option<&T>, Option<String>, usize)> {
    // First, expand all tokens including those in conditional branches
    let mut expanded_tokens = Vec::new();
    collect_all_tokens_including_conditionals(tokens, &mut expanded_tokens);
    
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

    // Process the expanded token list (includes all conditional branches)
    for token in expanded_tokens {
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
                else if is_documentable(token) {
                    // Check if this specific item type should be included when undocumented
                    if let Some(item_type) = documentation_type(token, last_global_label.as_deref()) {
                        if include_undocumented.should_include(&item_type) {
                            doc.push((String::new(), Some(token), last_global_label.clone(), line_number));
                        }
                    }
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

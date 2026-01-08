pub mod cmdline;

use std::sync::Arc;

use cpclib_asm::{ListingElement, parse_z80_str};
use cpclib_common::itertools::Itertools;
use minijinja::value::Object;
use minijinja::{Environment, ErrorKind, Value, context};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};

#[derive(Embed)]
#[folder = "src/templates/"]
#[include = "*.jinja"]
struct Templates;

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
    Function { name: String, arguments: Vec<String>, content: String }
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

    pub fn item_key(&self) -> String {
        match self {
            DocumentedItem::Label(l) => {
                format!("label_{}", l)
            },

            DocumentedItem::Equ(l, _) => {
                format!("equ_{}", l)
            },

            DocumentedItem::Macro { name, .. } => {
                format!("macro_{}", name)
            },

            DocumentedItem::Function { name, .. } => {
                format!("function_{}", name)
            },

            _ => String::from("unknown_item")
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemDocumentation {
    item: DocumentedItem,
    doc: String // TODO use MetaDocumentation
}

impl Object for ItemDocumentation {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str() {
            Some("doc") => Some(Value::from(self.doc.clone())),
            Some("summary") => Some(Value::from(self.item_long_summary())),
            Some("short_summary") => Some(Value::from(self.item_short_summary())),
            Some("key") => Some(Value::from(self.item.item_key())),
            Some("source") => {
                if self.is_macro() {
                    Some(Value::from(self.macro_source()))
                } else if self.is_function() {
                    Some(Value::from(self.function_source()))
                } else {
                    Some(Value::from(String::new()))
                }
            },
            _ => None
        }
    }
}

impl ItemDocumentation {
    /// Get the source code of a macro, or empty string for other items
    pub fn macro_source(&self) -> String {
        match &self.item {
            DocumentedItem::Macro { content, .. } => content.clone(),
            _ => String::new()
        }
    }

    /// Get the source code of a function, or empty string for other items
    pub fn function_source(&self) -> String {
        match &self.item {
            DocumentedItem::Function { content, .. } => content.clone(),
            _ => String::new()
        }
    }

    delegate::delegate! {
        to self.item {
            pub fn is_label(&self) -> bool;
            pub fn is_equ(&self) -> bool;
            pub fn is_macro(&self) -> bool;
            pub fn is_function(&self) -> bool;

            pub fn item_key(&self) -> String;
        }
    }

    pub fn item_long_summary(&self) -> String {
        match &self.item {
            DocumentedItem::Label(l) => l.to_string(),

            DocumentedItem::Equ(l, v) => {
                format!("{l} EQU {v}")
            },

            DocumentedItem::Macro { name, arguments, .. } => {
                let args = arguments.join(",");
                format!("MACRO {name}({args})")
            },

            DocumentedItem::Function { name, arguments, .. } => {
                let args = arguments.join(",");
                format!("FUNCTION {name}({args})")
            },

            _ => String::from("Unknown item")
        }
    }

    pub fn item_short_summary(&self) -> String {
        match &self.item {
            DocumentedItem::Label(l) => l.to_string(),

            DocumentedItem::Equ(l, v) => {
                l.clone()
            },

            DocumentedItem::Macro { name, .. } => {
                name.clone()
            },

            DocumentedItem::Function { name, .. } => {
                name.clone()
            },

            _ => String::from("Unknown item")
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::default();

        match &self.item {
            DocumentedItem::Label(l) => {
                md += &format!("## {l} (label) \n\n");
            },

            DocumentedItem::Equ(l, v) => {
                md += &format!("## {l} EQU {v} \n\n");
            },

            DocumentedItem::Macro { name, arguments, .. } => {
                let args = arguments.join(",");
                md += &format!("## MACRO {name}({args}) \n\n");
            },

            DocumentedItem::Function { name, arguments, .. } => {
                let args = arguments.join(",");
                md += &format!("## FUNCTION {name}({args}) \n\n");
            },

            _ => {
                // currently ignored
            }
        }

        md += &self.doc;
        md += "\n";

        md
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentationPage {
    fname: String,
    content: Vec<ItemDocumentation>
}

impl Object for DocumentationPage {
    fn get_value(self: &Arc<Self>, name: &Value) -> Option<minijinja::value::Value> {
        match name.as_str() {
            Some("file_name") => Some(Value::from(self.fname.clone())),
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
    pub fn for_file(fname: &str, include_undocumented: bool) -> Result<Self, String> {
        let code = std::fs::read_to_string(fname)
            .map_err(|e| format!("Unable to read {} file. {}", fname, e))?;
        let tokens = parse_z80_str(&code).map_err(|e| format!("Unable to read source. {}", e))?;
        let doc = aggregate_documentation_on_tokens(&tokens, include_undocumented);

        Ok(build_documentation_page_from_aggregates(fname, doc))
    }

    /// Merge multiple documentation pages into a single page
    pub fn merge(pages: Vec<Self>) -> Self {
        if pages.is_empty() {
            return Self {
                fname: String::from("Empty documentation"),
                content: Vec::new()
            };
        }

        if pages.len() == 1 {
            return pages.into_iter().next().unwrap();
        }

        let fnames = pages.iter()
            .map(|p| p.fname.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        
        let content = pages.into_iter()
            .flat_map(|p| p.content)
            .collect();

        Self {
            fname: fnames,
            content
        }
    }

    pub fn label_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.is_label())
    }

    pub fn macro_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.is_macro())
    }

    pub fn equ_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.is_equ())
    }

    pub fn function_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter().filter(|item| item.is_function())
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
        let page = Value::from_object(self.clone());

        let mut env = Environment::new();
        
        // Add a custom filter to convert markdown to HTML
        env.add_filter("markdown_to_html", |value: String| -> Result<String, minijinja::Error> {
            use pulldown_cmark::{Parser, Options, html};
            
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_FOOTNOTES);
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_TASKLISTS);
            
            let parser = Parser::new_ext(&value, options);
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            
            Ok(html_output)
        });
        
        const TMPL_NAME: &str = "html_documentation.jinja";
        let tmpl_src = Templates::get(TMPL_NAME).expect("Template not found").data;
        let tmpl_src = std::str::from_utf8(tmpl_src.as_ref()).unwrap();
        env.add_template(TMPL_NAME, tmpl_src).unwrap();

        let tmpl = env.get_template("html_documentation.jinja").unwrap();
        tmpl.render(context! {
            page
        })
        .unwrap()
    }
}

#[inline]
pub fn is_any_documentation<T: ListingElement>(token: &T) -> bool {
    token.is_comment() && token.comment().starts_with(LOCAL_DOCUMENTATION_START)
}

#[inline]
pub fn is_global_documentation<T: ListingElement>(token: &T) -> bool {
    token.is_comment() && token.comment().starts_with(GLOBAL_DOCUMENTATION_START)
}

#[inline]
pub fn is_local_documentation<T: ListingElement>(token: &T) -> bool {
    token.is_comment()
        && token.comment().starts_with(LOCAL_DOCUMENTATION_START)
        && !token.comment().starts_with(GLOBAL_DOCUMENTATION_START)
}

pub fn is_documentable<T: ListingElement + ToString>(token: &T) -> bool {
    documentation_type(token, None).is_some()
}

pub fn documentation_type<T: ListingElement + ToString>(token: &T, last_global_label: Option<&str>) -> Option<DocumentedItem> {
    if token.is_label() {
        let label = token.label_symbol().to_string();
        // Handle local labels (starting with ".")
        if label.starts_with('.') {
            // If we have a parent global label, prepend it
            if let Some(parent) = last_global_label {
                Some(DocumentedItem::Label(format!("{}{}", parent, label)))
            } else {
                // No parent label, skip this local label
                None
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
    agg: Vec<(String, Option<&T>, Option<String>)>
) -> DocumentationPage {
    let content = agg
        .into_iter()
        .filter_map(|(doc, t, last_global_label)| {
            if let Some(t) = t {
                // Try to create a documented item, passing the parent label context
                documentation_type(t, last_global_label.as_deref()).map(|item| {
                    ItemDocumentation {
                        item,
                        doc
                    }
                })
            }
            else {
                Some(ItemDocumentation {
                    item: DocumentedItem::File(fname.to_string()),
                    doc
                })
            }
        })
        .collect();

    DocumentationPage {
        fname: fname.to_string(),
        content
    }
}

/// Aggregate the comments when there are considered to be documentation and associate them to the required token if any
/// Also tracks the last global label to handle local labels (starting with ".")
pub fn aggregate_documentation_on_tokens<T: ListingElement + ToString>(
    tokens: &[T],
    include_undocumented: bool
) -> Vec<(String, Option<&T>, Option<String>)> {
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
                doc.push((in_process_comment.consume(), None, last_global_label.clone()));
            }
            in_process_comment.set_kind(CommentKind::Local);
            (true, false)
        }
        else {
            (false, is_documentable(token))
        };

        if current_is_doc {
            // we update the documentation
            in_process_comment.add_comment(token.comment());
        }
        else if current_is_documentable {
            // Track the last global label for local label resolution
            if token.is_label() {
                let label = token.label_symbol().to_string();
                if !label.starts_with('.') {
                    last_global_label = Some(label);
                }
            }
            
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
                doc.push((in_process_comment.consume(), documented, last_global_label.clone()));
            }
            else if include_undocumented && (token.is_macro_definition() || token.is_function_definition()) {
                // Include undocumented macros and functions if flag is set
                doc.push((String::new(), Some(token), last_global_label.clone()));
            }
            else {
                // we add no comment, so we do nothing
            }
        }
        else {
            // this is not a doc or a documentable, so we can eventually treat a global
            if in_process_comment.is_global() {
                doc.push((in_process_comment.consume(), None, last_global_label.clone()));
            }
            else if in_process_comment.is_local() {
                // comment is lost as there is nothing else to comment
                in_process_comment.clear();
            }
        }
    }

    // The last comment can only be a global comment
    if in_process_comment.is_global() {
        doc.push((in_process_comment.consume(), None, last_global_label.clone()));
    }

    doc
}

#[cfg(test)]
mod test {
    use cpclib_asm::Token;

    use crate::{aggregate_documentation_on_tokens, is_any_documentation};

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
}

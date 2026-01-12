//! Data models for documentation generation

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

/// A documented item (symbol, file, macro, function, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentedItem {
    File(String),
    Label(String),
    Equ(String, String),
    Macro { name: String, arguments: Vec<String>, content: String },
    Function { name: String, arguments: Vec<String>, content: String },
    Source(String),
    SyntaxError(String)
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

/// Configuration for including undocumented symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndocumentedConfig {
    pub macros: bool,
    pub functions: bool,
    pub labels: bool,
    pub equs: bool,
}

impl UndocumentedConfig {
    pub fn all() -> Self { Self { macros: true, functions: true, labels: true, equs: true } }
    pub fn none() -> Self { Self { macros: false, functions: false, labels: false, equs: false } }
    pub fn should_include(&self, item: &DocumentedItem) -> bool {
        match item {
            DocumentedItem::Macro { .. } => self.macros,
            DocumentedItem::Function { .. } => self.functions,
            DocumentedItem::Label { .. } => self.labels,
            DocumentedItem::Equ { .. } => self.equs,
            _ => true,
        }
    }
}

impl Default for UndocumentedConfig {
    fn default() -> Self { Self::none() }
}

/// A reference to where a symbol is used
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolReference {
    #[serde(serialize_with = "serialize_arc_str", deserialize_with = "deserialize_arc_str")]
    pub source_file: Arc<str>,
    pub line_number: usize,
    #[serde(serialize_with = "serialize_cow_str", deserialize_with = "deserialize_cow_str")]
    pub context: Cow<'static, str>,
    #[serde(serialize_with = "serialize_arc_str", deserialize_with = "deserialize_arc_str")]
    pub highlighted_context: Arc<str>
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

/// Metadata for documentation (author, date, etc.)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MetaDocumentation {
    Author(String),
    Date(String),
    Since(String)
}

/// Documentation for a single item
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemDocumentation {
    pub(crate) item: DocumentedItem,
    pub(crate) doc: String,
    pub(crate) source_file: String,
    pub(crate) display_source_file: String,
    pub(crate) line_number: usize,
    pub(crate) references: Vec<SymbolReference>,
    pub(crate) linked_source: Option<String>
}

impl ItemDocumentation {
    pub fn new(
        item: DocumentedItem,
        doc: String,
        source_file: String,
        display_source_file: String,
        line_number: usize,
    ) -> Self {
        Self {
            item,
            doc,
            source_file,
            display_source_file,
            line_number,
            references: Vec::new(),
            linked_source: None
        }
    }

    pub fn item(&self) -> &DocumentedItem {
        &self.item
    }

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
            DocumentedItem::Label(label) => format!("{}", label),
            DocumentedItem::Equ(name, value) => format!("{} EQU {}", name, value),
            DocumentedItem::Macro { name, arguments, .. } => format!("{}({})", name, arguments.join(", ")),
            DocumentedItem::Function { name, arguments, .. } => format!("{}({})", name, arguments.join(", ")),
            DocumentedItem::File(fname) => format!("{}", fname),
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

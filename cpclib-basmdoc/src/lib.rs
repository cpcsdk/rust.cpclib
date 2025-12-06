pub mod cmdline;


use std::sync::Arc;

use cpclib_asm::{ListingElement, parse_z80_str};
use minijinja::{Environment, ErrorKind, Value, context, value::Object};
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
    Macro(String, Vec<String>),
    Function
}

impl DocumentedItem {
    pub fn is_label(&self) -> bool {
        matches!(self, DocumentedItem::Label(_))
    }

    pub fn is_equ(&self) -> bool {
        matches!(self, DocumentedItem::Equ(_, _))
    }

    pub fn is_macro(&self) -> bool {
        matches!(self, DocumentedItem::Macro(_, _))
    }

    pub fn is_function(&self) -> bool {
        matches!(self, DocumentedItem::Function)
    }


    pub fn item_key(&self) -> String {
        match self {
            DocumentedItem::Label(l) => {
                format!("label_{}", l.to_string())
            },

            DocumentedItem::Equ(l, _) => {
                format!("equ_{}", l.to_string())
            },

            DocumentedItem::Macro(n, _) => {
                format!("macro_{}", n.to_string())
            },

            _ => {
                String::from("unknown_item")
            }
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
            Some("summary") => Some(Value::from(self.item_summary())),
            Some("key") => Some(Value::from(self.item.item_key())),
            _ => None
        }
    }
}

impl ItemDocumentation {

    delegate::delegate! {
        to self.item {
            pub fn is_label(&self) -> bool;
            pub fn is_equ(&self) -> bool;
            pub fn is_macro(&self) -> bool;
            pub fn is_function(&self) -> bool;

            pub fn item_key(&self) -> String;
        }
    }



    pub fn item_summary(&self) -> String {
        match &self.item {
            DocumentedItem::Label(l) => {
                l.to_string()
            },

            DocumentedItem::Equ(l, v) => {
                format!("{l} EQU {v}")
            },

            DocumentedItem::Macro(n, args) => {
                let args = args.join(",");
                format!("MACRO {n}({args})")
            },

            _ => {
                String::from("Unknown item")
            }
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

            DocumentedItem::Macro(n, args) => {
                let args = args.join(",");
                md += &format!("## MACRO {n}({args}) \n\n");
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
                let labels =self.label_iter()
                    .cloned()
                    .map(|item| Value::from_object(item))
                    .collect::<Vec<_>>();
                let labels = Value::from_object(dbg!(labels));
                Some(labels)
            },
            Some("macros") => {
                let macros =self.macro_iter()
                    .cloned()
                    .map(|item| Value::from_object(item))
                    .collect::<Vec<_>>();
                let macros = Value::from_object(dbg!(macros));
                Some(macros)
            },
            Some("equs") => {
                let equs =self.equ_iter()
                    .cloned()
                    .map(|item| Value::from_object(item))
                    .collect::<Vec<_>>();
                let equs = Value::from_object(dbg!(equs));
                Some(equs)
            },
            _ => None
        }
    }

    fn call_method(
            self: &std::sync::Arc<Self>,
            state: &minijinja::State<'_, '_>,
            method: &str,
            args: &[minijinja::Value],
        ) -> Result<minijinja::Value, minijinja::Error> {
        match method {
            "has_labels" => Ok(Value::from(self.has_labels())),
            "has_macros" => Ok(Value::from(self.has_macros())),
            "has_equ" => Ok(Value::from(self.has_equ())),

            _ => Err(minijinja::Error::new(
                ErrorKind::UnknownMethod,
                format!("Unknown method '{}'", method),
            )),
        }
    }
}
impl DocumentationPage {
    // TODO handle errors
    pub fn for_file(fname: &str) -> Self {
        let code = std::fs::read_to_string(fname).unwrap();
        let tokens = parse_z80_str(&code).unwrap();
        let doc = dbg!(aggregate_documentation_on_tokens(&tokens));
        build_documentation_page_from_aggregates(fname, doc)
    }

    pub fn label_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter()
            .filter(|item| item.is_label())
    }

    pub fn macro_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter()
            .filter(|item| item.is_macro())
    }

    pub fn equ_iter(&self) -> impl Iterator<Item = &ItemDocumentation> {
        self.content.iter()
            .filter(|item| item.is_equ())
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

    /// Return a string that encode the documentation page in markdown
    pub fn to_markdown(&self) -> String {
        let page = Value::from_object(self.clone());

        let mut env = Environment::new();
        const TMPL_NAME : &str = "markdown_documentation.jinja";
        let tmpl_src = Templates::get(TMPL_NAME)
            .expect("Template not found")
            .data;
        let tmpl_src = std::str::from_utf8(tmpl_src.as_ref()).unwrap();
        env.add_template(TMPL_NAME, tmpl_src).unwrap();
        
        let tmpl = env.get_template("markdown_documentation.jinja").unwrap();
        tmpl.render(context! {
            page
        }).unwrap()
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

pub fn is_documentable<T: ListingElement>(token: &T) -> bool {
    documentation_type(token).is_some()
}

pub fn documentation_type<T: ListingElement>(token: &T) -> Option<DocumentedItem> {
    if token.is_label() {
        Some(DocumentedItem::Label(token.label_symbol().to_string()))
    }
    else if token.is_equ() {
        Some(DocumentedItem::Equ(
            token.equ_symbol().to_string(),
            token.equ_value().to_string()
        ))
    }
    else if token.is_function_definition() {
        Some(DocumentedItem::Function)
    }
    else if token.is_macro_definition() {
        Some(DocumentedItem::Macro(
            token.macro_definition_name().to_string(),
            token
                .macro_definition_arguments()
                .iter()
                .map(|a| a.to_string())
                .collect()
        ))
    }
    else {
        None
    }
}

pub fn build_documentation_page_from_aggregates<T: ListingElement>(
    fname: &str,
    agg: Vec<(String, Option<&T>)>
) -> DocumentationPage {
    let content = agg
        .into_iter()
        .map(|(doc, t)| {
            if let Some(t) = t {
                ItemDocumentation {
                    item: documentation_type(t).unwrap(),
                    doc
                }
            }
            else {
                ItemDocumentation {
                    item: DocumentedItem::File(fname.to_string()),
                    doc
                }
            }
        })
        .collect();

    DocumentationPage {
        fname: fname.to_string(),
        content
    }
}

/// Aggregate the comments when there are considered to be documentation and associate them to the required token if any
pub fn aggregate_documentation_on_tokens<T: ListingElement>(
    tokens: &[T]
) -> Vec<(String, Option<&T>)> {
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
    };

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
                doc.push((in_process_comment.consume(), None));
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
                doc.push((in_process_comment.consume(), documented));
            }
            else {
                // we add no comment, so we do nothing
            }
        }
        else {
            // this is not a doc or a documentable, so we can eventually treat a global
            if in_process_comment.is_global() {
                doc.push((in_process_comment.consume(), None));
            }
            else if in_process_comment.is_local() {
                // comment is lost as there is nothing else to comment
                in_process_comment.clear();
            }
        }
    }

    // The last comment can only be a global comment
    if in_process_comment.is_global() {
        doc.push((in_process_comment.consume(), None));
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
        let doc = aggregate_documentation_on_tokens(&tokens);
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
        let doc = aggregate_documentation_on_tokens(&tokens);
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
        let doc = aggregate_documentation_on_tokens(&tokens);
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
        let doc = aggregate_documentation_on_tokens(&tokens);
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
                flavor: cpclib_asm::AssemblerFlavor::Basm
            }
        ];
        let doc = aggregate_documentation_on_tokens(&tokens);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, "This macro does something");
        assert!(doc[0].1.is_some());
    }
}

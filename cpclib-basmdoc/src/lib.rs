
use cpclib_asm::ListingElement;

const GLOBAL_DOCUMENTATION_START : &str = ";;;";
const LOCAL_DOCUMENTATION_START : &str = ";;";

pub enum MetaDocumentation {
    Author(String),
    Date(String),
    Since(String)
}


pub enum DocumentedItem {
    File,
    Label,
    Macro,
    Function
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
    token.is_comment() && token.comment().starts_with(LOCAL_DOCUMENTATION_START) && !token.comment().starts_with(GLOBAL_DOCUMENTATION_START)
}

pub fn is_documentable<T: ListingElement>(token: &T) -> bool {
    documentation_type(token).is_some()
}

pub fn documentation_type<T: ListingElement>(token: &T) -> Option<DocumentedItem> {
    if token.is_label() {
        Some(DocumentedItem::Label)
    } 
    else if token.is_function_definition() {
        Some(DocumentedItem::Function)
    }
    else if token.is_macro_definition() {
        Some(DocumentedItem::Macro)
    }
    else {
        None
    }
}

pub fn aggregate_comments_on_tokens<T: ListingElement>(tokens: &[T]) -> Vec<(String, Option<&T>)>{

    #[derive(PartialEq)]
    enum CommentKind {
        Unspecified,
        Global,
        Local
    }


    let mut doc = Vec::new();

    let mut current_comment = String::new();
    let mut current_kind = CommentKind::Unspecified;

    for token in tokens {
        let is_comment = if is_global_documentation(token) {
            if current_kind == CommentKind::Local {
                // here, this is an error, there was a local comment and it is replaced by a global one
                // so, we lost it
                current_comment.clear();
            }
            current_kind = CommentKind::Global;
            true
        } else if is_local_documentation(token) {
            if current_kind == CommentKind::Global {
                // here we can release the global comment
                doc.push((current_comment.clone(), None));
                current_comment.clear();
            }
            current_kind = CommentKind::Local;
            true
        } else {
            false
        };


        if is_comment {
            if !current_comment.is_empty() {
                current_comment += "\n";
            }
            current_comment += token.comment()
        } else if is_documentable(token){
            let added = if current_kind == CommentKind::Global {
                None
            } else {
                Some(token)
            };
            doc.push((current_comment.clone(), added));
            current_comment.clear();
        } else {
            // former comment was not commenting something and is ignored
            current_comment.clear();
        }
    }


    doc
}


#[cfg(test)]
mod test {
    use cpclib_asm::Token;

    use crate::{aggregate_comments_on_tokens, is_any_documentation};

    #[test]
    fn test_is_documentation() {
        assert!(!is_any_documentation(&Token::Comment("; any comment".into())));
        assert!(is_any_documentation(&Token::Comment(";; any comment".into())));
        assert!(is_any_documentation(&Token::Comment(";;; any comment".into())));
    }


    #[test]
    fn test_aggregate_global_comment() {
        let tokens = [  
            Token::Comment(";;; This file is commented, not the function!".into()),
            Token::Label("my_function".into())
        ];
        let doc = aggregate_comments_on_tokens(&tokens);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, ";;; This file is commented, not the function!");
        assert!(doc[0].1.is_none());
        
    }


    #[test]
    fn test_aggregate_label_comment() {
        let tokens = [  
            Token::Comment(";; This function does something".into()),
            Token::Label("my_function".into())
        ];
        let doc = aggregate_comments_on_tokens(&tokens);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, ";; This function does something");
        assert!(doc[0].1.is_some());
    }


    #[test]
    fn test_aggregate_label_merged_comment() {
        let tokens = [  
            Token::Comment(";; This function does something ...".into()),
            Token::Comment(";; ... on two lines".into()),
            Token::Label("my_function".into())
        ];
        let doc = aggregate_comments_on_tokens(&tokens);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, ";; This function does something ...\n;; ... on two lines");
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
                flavor: cpclib_asm::AssemblerFlavor::Basm,
            }
        ];
        let doc = aggregate_comments_on_tokens(&tokens);
        assert_eq!(doc.len(), 1);
        assert_eq!(&doc[0].0, ";; This macro does something");
        assert!(doc[0].1.is_some());
    }
}
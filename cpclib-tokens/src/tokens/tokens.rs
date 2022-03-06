use crate::tokens::expression::*;
use crate::tokens::instructions::*;
use crate::tokens::listing::*;

impl ListingElement for Token {
    type MacroParam = MacroParam;
    type TestKind = TestKind;
    type Expr = Expr;



    fn macro_call_name(&self) -> &str {
        match self {
            Token::MacroCall(name, _) => name.as_str(),
            _ => panic!()
        }
    }

    fn macro_call_arguments(&self) -> &[Self::MacroParam] {
        match self {
            Token::MacroCall(_, args) => args,
            _ => panic!()
        }
    }


    fn is_if(&self) -> bool {
       match self {
           Token::If(..) => true,
           _ => false
       }
    }

    fn if_nb_tests(&self) -> usize {
        match self {
            Self::If(tests, ..) => tests.len(),
            _ => panic!()
        }
    }

    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]) {
        match self {
            Self::If(tests, ..) => {
                let data = &tests[idx];
                (&data.0, &data.1)
            },
            _ => panic!()
        }
    }

    fn if_else(&self) -> Option<&[Self]> {
        match self {
            Self::If(_, r#else) => r#else.as_ref().map(|l| l.as_slice()),
            _ => panic!()
        }
    }

    fn is_include(&self) -> bool {
        match self {
            Token::Include(..) => true,
            _ => false
        }
    }

    fn is_incbin(&self) -> bool {
        match self {
            Token::Incbin{..} => true,
            _ => false
        }
    }


    fn incbin_fname(&self) -> &str {
        match self{
            Self::Incbin{fname, ..} => fname.as_ref(),
            _ => unreachable!()
        }
    }

    fn incbin_offset(&self) -> Option<&Self::Expr> {
        match self{
            Self::Incbin{offset, ..} => offset.as_ref(),
            _ => unreachable!()
        }
    }

    fn incbin_length(&self) -> Option<&Self::Expr> {
        match self{
            Self::Incbin{length, ..} => length.as_ref(),
            _ => unreachable!()
        }
    }

    fn incbin_transformation(&self) -> &BinaryTransformation {
        match self{
            Self::Incbin{transformation, ..} => transformation,
            _ => unreachable!()
        }
    }

    fn include_fname(&self) -> &str {
        match self{
            Self::Include(fname, _, _) => fname.as_ref(),
            _ => unreachable!()
        }
    }

}

/// Standard listing is a specific implementation
pub type Listing = BaseListing<Token>;

// Set of methods that do not have additional dependencies
impl Listing {
    /// Add a new label to the listing
    pub fn add_label(&mut self, label: &str) {
        self.listing_mut().push(Token::Label(label.into()));
    }

    /// Add a new comment to the listing
    pub fn add_comment(&mut self, comment: &str) {
        self.listing_mut()
            .push(Token::Comment(String::from(comment)));
    }

    /// Add a list of bytes to the listing
    pub fn add_bytes(&mut self, bytes: &[u8]) {
        let exp = bytes
            .iter()
            .map(|pu8| Expr::Value(i32::from(*pu8)))
            .collect::<Vec<_>>();
        let tok = Token::Defb(exp);
        self.push(tok);
    }

    // Macro can have labels like @stuff.
    // They must be replaced by unique values to be sure they can be called several times
    // pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
    // self.iter_mut()
    // .for_each(|e| e.fix_local_macro_labels_with_seed(seed));
    //
    //     dbg!(&self);
    // }
}

impl From<&[u8]> for Listing {
    fn from(src: &[u8]) -> Listing {
        let mut new = Listing::default();
        new.add_bytes(src);
        new
    }
}

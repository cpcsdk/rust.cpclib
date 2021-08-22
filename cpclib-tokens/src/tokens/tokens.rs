use crate::tokens::instructions::*;
use crate::tokens::listing::*;

use crate::tokens::expression::*;

impl ListingElement for Token {}

/// Standard listing is a specific implementation
pub type Listing = BaseListing<Token>;

// Set of methods that do not have additional dependencies
impl Listing {
    /// Add a new label to the listing
    pub fn add_label(&mut self, label: &str) {
        self.listing_mut().push(Token::Label(String::from(label)));
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
    /*
    pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
        self.iter_mut()
            .for_each(|e| e.fix_local_macro_labels_with_seed(seed));

        //     dbg!(&self);
    }
    */
}

impl From<&[u8]> for Listing {
    fn from(src: &[u8]) -> Listing {
        let mut new = Listing::default();
        new.add_bytes(src);
        new
    }
}

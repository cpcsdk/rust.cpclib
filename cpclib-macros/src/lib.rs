use proc_macro::TokenStream;
use cpclib_asm::preamble::*;
use syn::{parse_quote};
use quote::ToTokens;

#[proc_macro]
/// Parse an assembly code and produce the appropriate Listing.
/// In fact parsing is done 2 times: once during compilation to check the validity.
/// Once during execution to really do it. 
/// TODO find a way to generate the stream of tokens during the compilation
pub fn parse_assembly(item: TokenStream) -> TokenStream {
    let (str_listing, _listing) = get_listing(item);

    let tokens = quote::quote!{
        {
            Listing::from_str(#str_listing).unwrap()
        }
    };

    tokens.into()
}


/// Generte the bytes of asssembled data
#[proc_macro]
pub fn assemble(item: TokenStream) -> TokenStream
{
    let (_str_listing, listing) = get_listing(item);
    let bytes = listing.to_bytes()
        .unwrap_or_else(|e| panic!("Unable to assemble the probided code. {}", e));

        /*
    let tokens = quote::quote!{
        #bytes
    };

    tokens.into()
    */

    let mut tokens = proc_macro2::TokenStream::default();
    proc_macro2::Literal::byte_string(&bytes).to_tokens(&mut tokens);
    tokens.into()
}

/// Generate the listing needed for the various macros
fn get_listing(item: TokenStream) -> (String, Listing) {
    let str_listing = item.to_string();

    // A string is provided (Note this is the only way ATM)
    // We need to remove the " ... "
    let str_listing = if str_listing.starts_with('"') {
        &str_listing[1..str_listing.len()-1]
    }
    else {
        &str_listing
    };

    // Here we check if the code is valid
    (
        str_listing.to_owned(),
        Listing::from_str(&str_listing)
            .unwrap_or_else(|e| panic!("Unable to parse the provided code. {}", e))
    )
}
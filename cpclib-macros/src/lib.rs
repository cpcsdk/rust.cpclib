use proc_macro::TokenStream;
use cpclib_asm::preamble::*;
use syn::{parse_quote};

#[proc_macro]
/// Parse an assembly code and produce the appropriate Listing.
/// In fact parsing is done 2 times: once during compilation to check the validity.
/// Once during execution to really do it. 
/// TODO find a way to generate the stream of tokens during the compilation
pub fn parse_assembly(item: TokenStream) -> TokenStream {
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
    let listing: Listing = Listing::from_str(&str_listing)
                                .unwrap_or_else(|e| panic!("Unable to parse the provided code. {}", e));

    let tokens = quote::quote!{
        {
            Listing::from_str(#str_listing).unwrap()
        }
    };

    tokens.into()
}



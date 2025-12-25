use cpclib_asm::preamble::*;
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::{Result, parse_macro_input};
mod tokens;

/// Structure that contains the input of the macro.
/// Will be updated once we'll have additional parameters
struct AssemblyMacroInput {
    /// Code provided by the user of the macro
    code: String
}

mod kw {
    syn::custom_keyword!(fname);
}

/// Obtain the z80 code from:
/// - the direct string if any
/// - a file if "fname:" is provided
impl Parse for AssemblyMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::fname) {
            input.parse::<kw::fname>()?;
            input.parse::<syn::Token![:]>()?;
            let fname = (input.parse::<syn::LitStr>()?).value();
            let content = std::fs::read_to_string(&fname).map_err(|e| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("Unable to load {fname}.\n{e}")
                )
            })?;

            Ok(AssemblyMacroInput { code: content })
        }
        else if lookahead.peek(syn::LitStr) {
            Ok(AssemblyMacroInput {
                code: (input.parse::<syn::LitStr>()?).value()
            })
        }
        else {
            Err(lookahead.error())
        }
    }
}

#[proc_macro]
/// Parse an assembly code and produce the appropriate Listing while compiling the rust code.
/// No more parsing is done at execution.
/// input can be:
/// - a string literal
/// - a path
pub fn parse_z80(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as AssemblyMacroInput);
    let listing = get_listing(input);

    match listing {
        Ok(listing) => {
            use tokens::*;
            let mut stream = proc_macro2::TokenStream::new();
            listing.to_tokens(&mut stream);
            stream.into()
        },
        Err(e) => {
            panic!("[ERROR] {e:?}");
        }
    }
}

fn get_listing(
    input: AssemblyMacroInput
) -> std::result::Result<Listing, Box<cpclib_asm::error::AssemblerError>> {
    Listing::from_str(&input.code)
}

/// Generte the bytes of asssembled data
#[proc_macro]
pub fn assemble(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as AssemblyMacroInput);
    let listing = get_listing(input);

    match listing {
        Ok(listing) => {
            match listing.to_bytes() {
                Ok(ref bytes) => {
                    let mut tokens = proc_macro2::TokenStream::default();
                    proc_macro2::Literal::byte_string(bytes).to_tokens(&mut tokens);
                    tokens.into()
                },

                Err(e) => {
                    panic!("Unable to assemble the provided code. {e:?}");
                }
            }
        },
        Err(e) => {
            panic!("[ERROR] {e:?}");
        }
    }
}

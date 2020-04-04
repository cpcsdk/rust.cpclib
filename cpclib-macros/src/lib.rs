use proc_macro::TokenStream;
use cpclib_asm::preamble::*;
use quote::ToTokens;
use syn::{parse_macro_input, Result, token, Error};
use syn::parse::Parse;
use syn::parse::ParseStream;

/// Structure that contains the input f the macro.
/// Will be updated once we'll have additional parameters
struct AssemblyMacroInput {
    /// Code provided by the user of the macro
    code: String,
    /// Generated tokens
    tokens: Listing
}

/// XXX We cannot use this code because it remove the whitespaces :(
/// Whitespaces are of uttermost importance
impl Parse for AssemblyMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let str_listing = input.to_string();

        // A string is provided.
        // We need to remove the " ... "
        let str_listing = if str_listing.starts_with('"') {
            &str_listing[1..str_listing.len()-1]
        }
        // tokens are provided as is, there is nothing to do
        else {
            &str_listing
        }.to_owned();

        // Check if the tokens are valid
        let tokens = Listing::from_str(&str_listing)
                        .or_else(|e| return Err( 
                            input.error(format!("Unable to parse the provided code. {}", e)
                            )
                        )
                    )?;

        Ok(AssemblyMacroInput {
            code: str_listing,
            tokens
        })
    
    }
}

#[proc_macro]
/// Parse an assembly code and produce the appropriate Listing.
/// In fact parsing is done 2 times: once during compilation to check the validity.
/// Once during execution to really do it. 
/// TODO find a way to generate the stream of tokens during the compilation
pub fn parse_assembly(item: TokenStream) -> TokenStream {
    let input: syn::LitStr = parse_macro_input!(item);

    let str_listing = input.value();

    // A string is provided.
    // We need to remove the " ... "
    let code = if str_listing.starts_with('"') {
        (&str_listing[1..str_listing.len()-1]).to_owned()
    }
    // tokens are provided as is, there it is necessary to add a space
    // TODO check if it is a valid instruction first
    else {
        format!(" {}", str_listing)
    };

    // Check if the tokens are valid and raise an error if not
    let tokens = Listing::from_str(&code);
    if tokens.is_err() {
        eprintln!("{}", tokens.err().unwrap());
        return TokenStream::new();
    }
    let tokens = tokens.ok().unwrap();


    (quote::quote!{
                {
                    Listing::from_str(#code).unwrap()
                }

    }).into()
}


/// Generte the bytes of asssembled data
#[proc_macro]
pub fn assemble(item: TokenStream) -> TokenStream
{
    match get_listing(item) {
        Ok(AssemblyMacroInput {
            code,
            tokens
        }) => {

            match tokens.to_bytes() {
                Ok(ref bytes) => {
                    let mut tokens = proc_macro2::TokenStream::default();
                    proc_macro2::Literal::byte_string(&bytes).to_tokens(&mut tokens);
                    return tokens.into();
                },

                Err(e) => {
                    eprintln!("Unable to assemble the provided code. {}", e);
                    return TokenStream::new();
                }
            }
        },
        Err(e) => {
            eprintln!("Error while parsing ASM code. {}", e);
            return TokenStream::new();
        }
    }
}

/// Generate the listing needed for the various macros
fn get_listing(item: TokenStream) -> std::result::Result<AssemblyMacroInput, AssemblerError> {





    let str_listing = item.to_string();

    // A string is provided.
    // We need to remove the " ... "
    let str_listing = if str_listing.starts_with('"') {
        (&str_listing[1..str_listing.len()-1]).to_owned()
    }
    // tokens are provided as is, there it is necessary to add a space
    // TODO check if it is a valid instruction first
    else {
        format!(" {}", str_listing)
    };

    // Check if the tokens are valid
    let tokens = Listing::from_str(&str_listing)?;

    Ok(AssemblyMacroInput {
        code: str_listing,
        tokens
    })
}
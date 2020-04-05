use proc_macro::TokenStream;
use cpclib_asm::preamble::*;
use quote::ToTokens;
use syn::{parse_macro_input, Result, token, Error};
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::Parser;
mod tokens;


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
/// Parse an assembly code and produce the appropriate Listing while compiling the rust code.
/// No more parsing is done at execution.
/// input can be:
/// - a string literal
/// - a path
pub fn parse_z80(input: TokenStream) -> TokenStream {
    eprintln!("{:?}", input);

    let code = {
        let tokens = input.clone();
        let code: Result<syn::LitStr> = syn::parse(tokens);
        match code {
            Ok(code) => code.value(),
            Err(_) => {
                unimplemented!();
            }
        }
    };

    // Check if the tokens are valid and raise an error if not
    let listing = Listing::from_str(&code);
    if listing.is_err() {
        eprintln!("{}", listing.err().unwrap());
        return TokenStream::new();
    }
    let listing = listing.ok().unwrap();


    use tokens::*;
    let mut stream = proc_macro2::TokenStream::new();
    listing.to_tokens(&mut stream);
    stream.into()

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
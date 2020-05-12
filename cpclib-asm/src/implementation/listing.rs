use cpclib_tokens::tokens::*;
use cpclib_tokens::symbols::*;

use crate::parser;
use crate::error::*;
use std::fmt;

use crate::implementation::tokens::*;
use crate::implementation::expression::*;

use std::iter::FromIterator;



/// Additional methods for the listings
pub trait ListingExt {
    fn add_code<S: AsRef<str> + core::fmt::Display>(
        &mut self,
        code: S,
    ) -> Result<(), AssemblerError> ;

    /// Assemble the listing (without context) and returns the bytes 
    fn to_bytes(&self) -> Result<Vec<u8>, AssemblerError>;

          /// Compute the size of the listing.
    /// The listing has a size only if its tokens has a size
    fn number_of_bytes(&self) -> Result<usize, AssemblerError>;

    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    fn estimated_duration(&self) -> Result<usize, String>;

    fn save<P: AsRef<std::path::Path>>(&self, path: P) -> ::std::io::Result<()>;

    fn to_string(&self) -> String;

        /// Generate a string that contains also the bytes
        fn to_enhanced_string(&self) -> String;

}

impl ListingExt for Listing {
        /// Add additional tokens, that need to be parsed from a string, to the listing
        fn add_code<S: AsRef<str> + core::fmt::Display>(
            &mut self,
            code: S,
        ) -> Result<(), AssemblerError> {
            parser::parse_z80_str(code.as_ref())
                .map_err(|e| AssemblerError::SyntaxError {
                    error: format!("{:?}", e),
                })
                .map(|(_res, local_tokens)| {
                    self.listing_mut().extend_from_slice(&local_tokens);
                })
        }



    /// Compute the size of the listing when assembling it.
    /// 
    fn number_of_bytes(&self) -> Result<usize, AssemblerError> {
        Ok(self.to_bytes()?.len())
    }

    fn to_bytes(&self) -> Result<Vec<u8>, AssemblerError> {
        let options = crate::AssemblingOptions::default();
        let env = crate::assembler::visit_tokens_all_passes_with_options(&self.listing(), &options)?;
        Ok(env.produced_bytes())
    }



    
    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    fn estimated_duration(&self) -> Result<usize, String> {
        if let Some(duration) = self.duration() {
            Ok(duration)
        } else {
            let mut duration = 0;
            for token in self.listing().iter() {
                duration += token.estimated_duration()?;
            }
            Ok(duration)
        }
    }

        /// Save the listing on disc in a string version
    fn save<P: AsRef<std::path::Path>>(&self, path: P) -> ::std::io::Result<()> {
            use std::fs::File;
            use std::io::prelude::*;
    
            // Open a file in write-only mode, returns `io::Result<File>`
            let mut file = File::create(path.as_ref())?;
            file.write_all(self.to_string().as_bytes())?;
    
            Ok(())
        }
    

        fn to_string(&self) -> String {
            PrintableListing::from(self).to_string()
        }

        fn to_enhanced_string(&self) -> String {
            let mut res = String::new();
            let mut current_address: Option<u32> = None;
    
            for instruction in self.listing() {

                match current_address.as_ref() {
                    Some(address) => {res+= &format!("{:4x}", address);},
                    None => {res+= "???? ";}
                }

                match instruction.to_bytes() {
                    Ok(bytes) => {
                        for i in 0..4 {
                            if bytes.len() > i {
                                res += &format!("{:2X} ", bytes[i]);
                            }
                            else {
                                res += "   ";
                            }
                        }
                    },
                    _ => {
                        // BUG need to better manage interpretation to never achieve such error
                        res += "?? ?? ?? ?? ";
                    }
                }

                res += &instruction.to_string();
                res += "\n";
            }
    
            res
    
        }
    
}

/// Workaround to display a Lisitng as we cannot implement display there....
pub struct PrintableListing<'a>(&'a Listing);
impl<'a> From<&'a Listing> for PrintableListing<'a> {
    fn from(src: &'a Listing) -> PrintableListing<'a> {
        PrintableListing(src)
    }
}
impl<'a> fmt::Display for PrintableListing<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for token in self.0.listing().iter() {
            match token {
                Token::Label(_) | Token::Equ(_, _) | Token::Comment(_) => (),
                _ => {
                    write!(f, "\t")?;
                }
            }
            //write!(f, "{} ; {:?} {:?} nops {:?} bytes\n", token, token, token.estimated_duration(), token.number_of_bytes())?;
            writeln!(f, "{}", token)?;
        }

        Ok(())
    }
}




/// Generate a listing from a string
pub trait ListingFromStr {
    fn from_str(s: &str) -> Result<Listing, AssemblerError> ;
}

impl ListingFromStr for Listing {
    fn from_str(s: &str) -> Result<Listing, AssemblerError> {
        crate::parser::parse_str(s)
    } 
}

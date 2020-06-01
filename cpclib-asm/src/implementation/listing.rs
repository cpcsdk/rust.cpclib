use cpclib_tokens::tokens::*;
use cpclib_tokens::symbols::*;

use crate::parser;
use crate::error::*;
use std::fmt;

use crate::implementation::tokens::*;
use crate::implementation::expression::*;

use std::iter::FromIterator;

use crate::AssemblingOptions;



/// Additional methods for the listings
pub trait ListingExt {
    fn add_code<S: AsRef<str> + core::fmt::Display>(
        &mut self,
        code: S,
    ) -> Result<(), AssemblerError> ;

    /// Assemble the listing (without context) and returns the bytes 
    fn to_bytes(&self) -> Result<Vec<u8>, AssemblerError>;
    fn to_bytes_with_options(&self, option: &AssemblingOptions) -> Result<Vec<u8>, AssemblerError>;

          /// Compute the size of the listing.
    /// The listing has a size only if its tokens has a size
    fn number_of_bytes(&self) -> Result<usize, AssemblerError>;

    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    fn estimated_duration(&self) -> Result<usize, String>;

    fn save<P: AsRef<std::path::Path>>(&self, path: P) -> ::std::io::Result<()>;

    fn to_string(&self) -> String;

        /// Generate a string that contains also the bytes
        /// panic even for simple corner cases
        fn to_enhanced_string(&self) -> String;



    /// Modify the listing to inject labels at the given addresses
    fn inject_labels(&mut self, labels: &[(u16, &str)]);
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
        self.to_bytes_with_options(&options)
    }

    fn to_bytes_with_options(&self, options: &AssemblingOptions) -> Result<Vec<u8>, AssemblerError> {
        let env = crate::assembler::visit_tokens_all_passes_with_options(
            &self.listing(), 
            options)?;
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
            // TODO - allow assembling module to generate this listing by itself. This way no need to implement it properly a second time

            let mut res = String::new();
            let mut current_address: Option<u16> = None;
    
            let mut options = AssemblingOptions::default();

            for instruction in self.listing() {

                if let Token::Org(address, _) = instruction {
                    current_address = Some(address.eval().unwrap() as u16);
                }

                match current_address.as_ref() {
                    Some(address) => {
                        res+= &format!("{:4x} ", address);
                        options.symbols_mut().set_current_address(*address);

                    },
                    None => {res+= "???? ";}
                }

                
                match instruction.to_bytes_with_options(&options) {
                    Ok(bytes) => {
                        for i in 0..4 {
                            if bytes.len() > i {
                                res += &format!("{:2X} ", bytes[i]);
                            }
                            else {
                                res += "   ";
                            }
                        }
                        if current_address.is_some() {
                            current_address = Some(bytes.len() as u16 + current_address.unwrap());
                        }
                    },
                    Err(err) => {
                      //  panic!("{:?} {:?}", err, options);
                        // BUG need to better manage interpretation to never achieve such error
                        res += "?? ?? ?? ?? ";
                        current_address = None;
                    }
                }

                if !instruction.is_label() {
                    res += "\t";
                }
                res += &instruction.to_string();
                res += "\n";
            }
    
            res
    
        }


        /// Panic if Org is not one of the first instructions
        fn inject_labels(&mut self, sorted_labels: &[(u16, &str)]) {
            use cpclib_tokens::builder::label;
            use cpclib_tokens::builder::equ;

            let mut current_address: Option<u16> = None;
            let mut current_idx = 0;
            let mut nb_labels_added = 0;

            while current_idx < self.len() && nb_labels_added < sorted_labels.len(){
                let current_instruction = &self.listing()[current_idx];;

                current_idx += 1;

                let next_address = if let Token::Org(address, _) = current_instruction {
                    current_address = Some(address.eval().unwrap() as u16);
                    current_address.clone()
                }
                else {
                    let nb_bytes = current_instruction.number_of_bytes().unwrap();
                    match current_address {
                        Some(address) => Some(address + nb_bytes as u16),
                        None => {
                            if nb_bytes != 0 {
                                panic!("Unable to run if assembling address is unknown")
                            }
                            else {
                                None
                            }
                        }
                    }
                };
            

            let (expected, new_label) = sorted_labels[nb_labels_added];
            match (current_address, next_address) {

                (Some(current), Some(next)) if current == expected => {
                    self.listing_mut().insert(
                        current_idx, 
                        label(new_label)
                    );
                    nb_labels_added += 1;
                }
                (Some(current), Some(next)) if next < expected  => {
                    self.listing_mut().insert(
                        current_idx,
                        equ(new_label, expected) // TODO check if realtive address is better or not
                    );
                    nb_labels_added += 1;   
                }
                (_, _) => {
                    current_idx += 1;
                }
            }
        }
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

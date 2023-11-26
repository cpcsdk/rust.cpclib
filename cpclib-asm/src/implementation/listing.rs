use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;

use cpclib_tokens::symbols::PhysicalAddress;
use cpclib_tokens::tokens::*;

use crate::error::*;
use crate::implementation::expression::*;
use crate::implementation::tokens::*;
use crate::preamble::parse_z80_str;
use crate::EnvOptions;

/// Additional methods for the listings
pub trait ListingExt {
    fn add_code<S: AsRef<str> + core::fmt::Display>(
        &mut self,
        code: S
    ) -> Result<(), AssemblerError>;

    /// Assemble the listing (without context) and returns the bytes
    fn to_bytes(&self) -> Result<Vec<u8>, AssemblerError> {
        let options = EnvOptions::default();
        self.to_bytes_with_options(options)
    }

    fn to_bytes_with_options(&self, option: EnvOptions) -> Result<Vec<u8>, AssemblerError>;

    /// Compute the size of the listing.
    /// The listing has a size only if its tokens has a size
    fn number_of_bytes(&self) -> Result<usize, AssemblerError> {
        Ok(self.to_bytes()?.len())
    }

    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    fn estimated_duration(&self) -> Result<usize, AssemblerError>;
    /// Save the listing on disc in a string version
    fn save<P: AsRef<std::path::Path>>(&self, path: P) -> ::std::io::Result<()> {
        use std::fs::File;
        use std::io::prelude::*;

        // Open a file in write-only mode, returns `io::Result<File>`
        let mut file = File::create(path.as_ref())?;
        file.write_all(self.to_string().as_bytes())?;

        Ok(())
    }

    fn to_string(&self) -> String;

    /// Generate a string that contains also the bytes
    /// panic even for simple corner cases
    fn to_enhanced_string(&self) -> String;

    /// Modify the listing to inject labels at the given addresses
    fn inject_labels<S: Borrow<str>>(&mut self, labels: HashMap<u16, S>);
}

impl ListingExt for Listing {
    /// Add additional tokens, that need to be parsed from a string, to the listing
    fn add_code<S: AsRef<str> + core::fmt::Display>(
        &mut self,
        code: S
    ) -> Result<(), AssemblerError> {
        parse_z80_str(code.as_ref().trim_end()).map(|local_tokens| {
            let mut local_tokens = local_tokens.as_listing().to_vec();
            self.listing_mut().append(&mut local_tokens);
        })
    }

    fn to_bytes_with_options(&self, options: EnvOptions) -> Result<Vec<u8>, AssemblerError> {
        let (_, env) =
            crate::assembler::visit_tokens_all_passes_with_options(&self.listing(), options)
                .map_err(|(_, _, e)| AssemblerError::AlreadyRenderedError(e.to_string()))?;
        Ok(env.produced_bytes())
    }

    /// Get the execution duration.
    /// If field `duration` is set, returns it. Otherwise, compute it
    fn estimated_duration(&self) -> Result<usize, AssemblerError> {
        if let Some(duration) = self.duration() {
            Ok(duration)
        }
        else {
            let mut duration = 0;
            for token in self.listing().iter() {
                duration += token.estimated_duration()?;
            }
            Ok(duration)
        }
    }

    fn to_string(&self) -> String {
        PrintableListing::from(self).to_string()
    }

    fn to_enhanced_string(&self) -> String {
        todo!()
    }

    /// Panic if Org is not one of the first instructions
    fn inject_labels<S: Borrow<str>>(&mut self, mut labels: HashMap<u16,S>) {
        use cpclib_tokens::builder::{equ, label};

        let mut current_address: Option<u16> = None;
        let mut current_idx = 0;
        let mut nb_labels_added = 0;

        // inject labels at the appropriate address if any
        while current_idx < self.len() && ! labels.is_empty() {
            if let Some(current_address) = &current_address {
                if let Some(new_label) = labels.remove(current_address) {
                    self.listing_mut().insert(current_idx, label(new_label.borrow()));
                    nb_labels_added += 1;
                }
            }

            let current_instruction = &self.listing()[current_idx];

            let next_address = if let Token::Org { val1: address, .. } = current_instruction {
                current_address = Some(address.eval().unwrap().int().unwrap() as u16);
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
                    },
                }
            };

            current_idx += 1;
            current_address = next_address;
        }

        // inject all the remaining ones
        for (next_address, next_label) in labels.into_iter() {
            self.listing_mut().insert(
                0,
                equ(
                    next_label.borrow(),
                    next_address
                )
            );
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
                Token::Label(_) | Token::Equ { .. } | Token::Comment(_) => (),
                _ => {
                    write!(f, "\t")?;
                }
            }
            // write!(f, "{} ; {:?} {:?} nops {:?} bytes\n", token, token, token.estimated_duration(), token.number_of_bytes())?;
            writeln!(f, "{}", token)?;
        }

        Ok(())
    }
}

/// Generate a listing from a string
pub trait ListingFromStr {
    fn from_str(s: &str) -> Result<Listing, AssemblerError>;
}

impl ListingFromStr for Listing {
    fn from_str(s: &str) -> Result<Listing, AssemblerError> {
        crate::parser::parse_z80_str(s).map(|ll| ll.as_listing())
    }
}

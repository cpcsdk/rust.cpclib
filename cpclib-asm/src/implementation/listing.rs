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
    fn inject_labels(&mut self, labels: &[(u16, &str)]);
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
        // TODO - allow assembling module to generate this listing by itself. This way no need to implement it properly a second time

        let mut res = String::new();
        let mut current_address: Option<u16> = None;

        let mut options = EnvOptions::default();

        for instruction in self.listing() {
            if let Token::Org{val1:address,..} = instruction {
                current_address = Some(address.eval().unwrap().int().unwrap() as u16);
            }

            match current_address.as_ref() {
                Some(address) => {
                    res += &format!("{:4x} ", address);
                    options
                        .symbols_mut()
                        .set_current_address(PhysicalAddress::new(*address, 0xC0));
                }
                None => {
                    res += "???? ";
                }
            }

            let remaining = match instruction.to_bytes_with_options(options.clone()) {
                Ok(bytes) => {
                    for i in 0..4 {
                        if bytes.len() > i {
                            res += &format!("{:2X} ", bytes[i]);
                        }
                        else {
                            res += "   ";
                        }
                    }

                    res += "  ";
                    for i in 0..4 {
                        if bytes.len() > i {
                            let mut c: char = bytes[i] as char;
                            if !c.is_ascii_graphic() {
                                c = '.';
                            }
                            res += &format!("{} ", c);
                        }
                        else {
                            res += " ";
                        }
                    }

                    if current_address.is_some() {
                        current_address = Some(bytes.len() as u16 + current_address.unwrap());
                    }

                    if bytes.len() > 4 {
                        bytes[4..].to_vec()
                    }
                    else {
                        Vec::new()
                    }
                }
                Err(err) => {
                    panic!("{:?} {:?} {:?}", instruction, err, options);
                    // BUG need to better manage interpretation to never achieve such error
                    res += "?? ?? ?? ?? ";
                    current_address = None;
                    Vec::new()
                }
            };

            if !instruction.is_label() {
                res += "\t";
            }
            res += &instruction.to_string();
            res += "\n";

            if !remaining.is_empty() {
                let mut idx = 0;
                while idx < remaining.len() {
                    res += "     ";
                    for i in 0..4 {
                        if remaining.len() > (i + idx) {
                            res += &format!("{:2X} ", remaining[i + idx]);
                        }
                        else {
                            res += "   ";
                        }
                    }

                    res += "  ";
                    for i in 0..4 {
                        if remaining.len() > (i + idx) {
                            let mut c: char = remaining[i + idx] as char;
                            if !c.is_ascii_graphic() {
                                c = '.';
                            }
                            res += &format!("{} ", c);
                        }
                        else {
                            res += " ";
                        }
                    }

                    res += "\n";
                    idx += 4;
                }
            }
        }

        res
    }

    /// Panic if Org is not one of the first instructions
    fn inject_labels(&mut self, sorted_labels: &[(u16, &str)]) {
        use cpclib_tokens::builder::{equ, label};

        let mut current_address: Option<u16> = None;
        let mut current_idx = 0;
        let mut nb_labels_added = 0;

        while current_idx < self.len() && nb_labels_added < sorted_labels.len() {
            let current_instruction = &self.listing()[current_idx];

            let next_address = if let Token::Org{val1:address, ..} = current_instruction {
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
                    }
                }
            };

            let (expected, new_label) = sorted_labels[nb_labels_added];

            match (current_address, next_address) {
                (Some(current), Some(next)) => {
                    if current == expected {
                        self.listing_mut().insert(current_idx, label(new_label));
                        nb_labels_added += 1;
                    }
                    else if current < expected && next > expected {
                        self.listing_mut().insert(
                            current_idx,
                            equ(new_label, expected), // TODO check if realtive address is better or not
                        );
                        nb_labels_added += 1;
                    }
                    else {
                        current_idx += 1;
                    }
                }
                (..) => {
                    current_idx += 1;
                }
            }

            current_address = next_address;
        }

        while nb_labels_added < sorted_labels.len() {
            panic!("{} remaining", sorted_labels.len() - nb_labels_added);
            self.listing_mut().insert(
                0,
                equ(
                    sorted_labels[nb_labels_added].1,
                    sorted_labels[nb_labels_added].0
                )
            );
            nb_labels_added += 1;
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
                Token::Label(_) | Token::Equ{..} | Token::Comment(_) => (),
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

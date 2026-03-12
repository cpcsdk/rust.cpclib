use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::RangeInclusive;

use clap::Parser;
use cpclib_asm::{Listing, ListingExt, defb_elements, org};
use cpclib_common::camino::Utf8PathBuf;
use cpclib_disc::amsdos::AmsdosHeader;

mod error;
mod analysis;
mod control_file;
mod parser;

use error::{BdAsmError, Result};
use analysis::{collect_addresses_from_expressions, inject_labels_into_expressions};
use control_file::{ControlFile, save_control_file};
use parser::{parse_u16_value, parse_value_or_label, parse_data_bloc_string, load_control_file};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Maximum number of elements in a single DB directive for readability
const MAX_DB_ELEMENTS: usize = 8;

/// Data bloc specification with string values (before label resolution)
#[derive(Debug, Clone)]
pub enum DataBlocString {
    /// START-LENGTH syntax: (start_str, length_str)
    Sized(String, String),
    /// START..END syntax: (start_str, end_str) - exclusive end
    Range(String, String),
    /// START..=END syntax: (start_str, end_str) - inclusive end
    InclusiveRange(String, String),
}

impl std::fmt::Display for DataBlocString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataBlocString::Sized(start, length) => {
                write!(f, "{}-{}", start, length)
            },
            DataBlocString::Range(start, end) => {
                write!(f, "{}..{}", start, end)
            },
            DataBlocString::InclusiveRange(start, end) => {
                write!(f, "{}..={}", start, end)
            },
        }
    }
}

/// Data bloc specification with resolved u16 values
#[derive(Debug, Clone)]
pub enum DataBloc {
    /// START-LENGTH: (start, length)
    Sized(u16, u16),
    /// START..END: (start, end) - exclusive end
    Range(u16, u16),
    /// START..=END: (start, end) - inclusive end
    InclusiveRange(u16, u16),
}

impl std::str::FromStr for DataBlocString {
    type Err = BdAsmError;

    fn from_str(spec: &str) -> Result<Self> {
        let bytes = spec.as_bytes();
        let mut input: &[u8] = bytes;
        
        parse_data_bloc_string(&mut input)
            .map_err(|e| BdAsmError::InvalidDataBloc(
                format!("Invalid data bloc format '{}': {}", spec, e)
            ))
    }
}

impl DataBlocString {
    /// Convert this DataBlocString to a DataBloc by resolving labels
    pub fn to_data_bloc(&self, labels: &HashMap<u16, Cow<str>>) -> Result<DataBloc> {
        match self {
            DataBlocString::Sized(start_str, length_str) => {
                let start = parse_value_or_label(start_str, labels)
                    .map_err(|e| BdAsmError::InvalidDataBloc(format!("Invalid start: {}", e)))?;
                let length = parse_value_or_label(length_str, labels)
                    .map_err(|e| BdAsmError::InvalidDataBloc(format!("Invalid length: {}", e)))?;
                Ok(DataBloc::Sized(start, length))
            }
            DataBlocString::Range(start_str, end_str) => {
                let start = parse_value_or_label(start_str, labels)
                    .map_err(|e| BdAsmError::InvalidDataBloc(format!("Invalid start: {}", e)))?;
                let end = parse_value_or_label(end_str, labels)
                    .map_err(|e| BdAsmError::InvalidDataBloc(format!("Invalid end: {}", e)))?;
                Ok(DataBloc::Range(start, end))
            }
            DataBlocString::InclusiveRange(start_str, end_str) => {
                let start = parse_value_or_label(start_str, labels)
                    .map_err(|e| BdAsmError::InvalidDataBloc(format!("Invalid start: {}", e)))?;
                let end = parse_value_or_label(end_str, labels)
                    .map_err(|e| BdAsmError::InvalidDataBloc(format!("Invalid end: {}", e)))?;
                Ok(DataBloc::InclusiveRange(start, end))
            }
        }
    }
}

impl DataBloc {
    /// Convert DataBloc to RangeInclusive<u16>
    pub fn to_range_inclusive(&self) -> std::result::Result<RangeInclusive<u16>, BdAsmError> {
        match self {
            DataBloc::Sized(start, length) => {
                if *length == 0 {
                    return Err(BdAsmError::InvalidDataBloc(
                        "Length must be at least 1".to_string()
                    ));
                }
                let end = start.checked_add(length - 1)
                    .ok_or_else(|| BdAsmError::InvalidDataBloc(
                        format!("Data bloc range overflow: {} + {}", start, length - 1)
                    ))?;
                Ok(*start..=end)
            }
            DataBloc::Range(start, end) => {
                if start >= end {
                    return Err(BdAsmError::InvalidDataBloc(
                        format!("Invalid range: start ({}) must be less than end ({})", start, end)
                    ));
                }
                // END is exclusive, so inclusive range is start..=(end-1)
                Ok(*start..=(end - 1))
            }
            DataBloc::InclusiveRange(start, end) => {
                if start > end {
                    return Err(BdAsmError::InvalidDataBloc(
                        format!("Invalid range: start ({}) must be less than or equal to end ({})", start, end)
                    ));
                }
                Ok(*start..=*end)
            }
        }
    }

    /// Resolve a DataBlocString to a DataBloc using the label map
    /// Delegates to DataBlocString::to_data_bloc
    pub fn from_string(spec: &DataBlocString, labels: &HashMap<u16, Cow<str>>) -> Result<Self> {
        spec.to_data_bloc(labels)
    }
}

/// Environment for disassembly containing all resolved configuration
#[derive(Debug)]
struct BdAsmEnv {
    origin: Option<u16>,
    address2label: HashMap<u16, String>,
    label2address: HashMap<String, u16>,
    blocs: Vec<DataBloc>,
}

impl BdAsmEnv {
    /// Create a BdAsmEnv from a ControlFile
    /// Two-pass approach: first collect all labels, then resolve data blocs
    fn from_control_file(control: &ControlFile) -> Result<Self> {
        let mut origin = None;
        let mut address2label = HashMap::new();
        let mut label2address = HashMap::new();
        let mut data_bloc_specs = Vec::new();
        
        // First pass: collect origin and all labels
        for directive in &control.directives {
            match directive {
                control_file::ControlDirective::Origin(addr) => {
                    origin = Some(*addr);
                }
                control_file::ControlDirective::Label { name, address } => {
                    address2label.insert(*address, name.clone());
                    label2address.insert(name.clone(), *address);
                }
                control_file::ControlDirective::DataBloc(spec) => {
                    // Store for later resolution
                    data_bloc_specs.push(spec);
                }
                _ => {} // Skip other directives (e.g., Skip is handled separately)
            }
        }
        
        // Second pass: resolve data blocs now that all labels are collected
        let label_map_cow: HashMap<u16, Cow<str>> = address2label
            .iter()
            .map(|(addr, name)| (*addr, Cow::Borrowed(name.as_str())))
            .collect();
        
        let mut blocs = Vec::new();
        for spec in data_bloc_specs {
            let bloc = spec.to_data_bloc(&label_map_cow)?;
            blocs.push(bloc);
        }
        
        Ok(BdAsmEnv {
            origin,
            address2label,
            label2address,
            blocs,
        })
    }
    
    /// Calculate the valid address range for label generation (origin to origin + binary size)
    fn valid_range(&self, binary_size: usize) -> Option<RangeInclusive<u16>> {
        self.origin.map(|start| {
            let end = start.saturating_add(binary_size as u16);
            start..=end
        })
    }
    
    /// Convert data bloc addresses to file offsets by subtracting origin
    /// Returns the data blocs as file offsets, with a warning if origin is not set
    fn data_blocs_offsets(&self) -> Result<Vec<RangeInclusive<u16>>> {
        let mut data_blocs: Vec<RangeInclusive<u16>> = Vec::new();
        for bloc in &self.blocs {
            data_blocs.push(bloc.to_range_inclusive()?);
        }
        
        if let Some(origin_value) = self.origin {
            Ok(data_blocs
                .iter()
                .map(|range| {
                    let start_offset = range.start().saturating_sub(origin_value);
                    let end_offset = range.end().saturating_sub(origin_value);
                    start_offset..=end_offset
                })
                .collect())
        } else {
            // No origin, use addresses as-is (assume they are offsets)
            if !data_blocs.is_empty() {
                eprintln!("; Warning: --data specified without --origin; treating addresses as file offsets");
            }
            Ok(data_blocs)
        }
    }
    
    /// Create a listing from the input bytes, handling data blocs
    fn create_listing(&self, input_bytes: &[u8]) -> Result<Listing> {
        // Convert data bloc addresses to file offsets
        let data_blocs_offsets = self.data_blocs_offsets()?;
        
        // Retrieve the listing
        let mut listing: Listing = if !data_blocs_offsets.is_empty() {
            let mut data_blocs_mut = data_blocs_offsets.clone();
            data_blocs_mut.sort_by_key(|range| *range.start());

            // Make the listing for each of the blocs
            let mut listings: Vec<Listing> = Vec::new();
            let mut current_idx: usize = 0;
            while !data_blocs_mut.is_empty() {
                let bloc_range = data_blocs_mut
                    .first()
                    .ok_or_else(|| BdAsmError::InvalidDataBloc("Empty blocs list".to_string()))?
                    .clone();
                let bloc_idx = *bloc_range.start() as usize;
                let bloc_end = (*bloc_range.end() as usize).min(input_bytes.len() - 1);

                if current_idx < bloc_idx {
                    listings.push(cpclib_asm::disass::disassemble(
                        &input_bytes[current_idx..bloc_idx],
                    ));
                    current_idx = bloc_idx;
                } else {
                    assert_eq!(current_idx, bloc_idx);
                    listings.push(defb_elements_chunked(&input_bytes[bloc_idx..=bloc_end]));
                    data_blocs_mut.remove(0);
                    current_idx = bloc_end + 1;
                }
            }
            if current_idx < input_bytes.len() {
                listings.push(cpclib_asm::disass::disassemble(&input_bytes[current_idx..]));
            }

            // Merge the blocs
            listings
                .into_iter()
                .fold(Listing::new(), |mut lst, current| {
                    lst.inject_listing(&current);
                    lst
                })
        } else {
            // No blocs, easy disassembling
            cpclib_asm::disass::disassemble(input_bytes)
        };
        
        // Add origin to the listing
        if let Some(origin) = self.origin {
            listing.insert(0, org(origin));
        }
        
        Ok(listing)
    }
    
    /// Inject labels into the listing
    /// Collects addresses from expressions and injects all labels
    fn inject_labels(&mut self, listing: &mut Listing, binary_size: usize) -> Result<()> {
        // Calculate the valid address range for label generation
        let valid_range = self.valid_range(binary_size);

        // Get extra labels from expressions (only within valid range)
        for address in collect_addresses_from_expressions(listing, valid_range)? {
            self.address2label
                .entry(address)
                .or_insert(format!("label_{address:.4x}"));
            self.label2address
                .entry(format!("label_{address:.4x}"))
                .or_insert(address);
        }
        
        // Convert to Cow<str> for compatibility with listing.inject_labels
        let labels_cow: HashMap<u16, Cow<str>> = self
            .address2label
            .iter()
            .map(|(addr, name)| (*addr, Cow::Borrowed(name.as_str())))
            .collect();
        
        listing.inject_labels(labels_cow.clone());
        inject_labels_into_expressions(listing)?;
        
        Ok(())
    }
}

/// Generate defb directives from a slice of bytes, chunking into multiple directives
/// if the data is longer than MAX_DB_ELEMENTS for better readability
fn defb_elements_chunked(bytes: &[u8]) -> Listing {
    let mut listing = Listing::new();
    
    for chunk in bytes.chunks(MAX_DB_ELEMENTS) {
        listing.push(defb_elements(chunk));
    }
    
    listing
}

/// Parse a data bloc specification supporting two syntaxes:
/// - START-LENGTH: where START is address and LENGTH is size
/// - START..END: where both are addresses (END is exclusive)
/// Each component can be a numeric value or a label name
/// Benediction disassembler
#[derive(Parser, Debug)]
#[command(name = "bdasm")]
#[command(version = built_info::PKG_VERSION)]
#[command(author = "Krusty/Benediction")]
#[command(about = "Benediction disassembler", long_about = None)]
pub struct BdAsmCli {
    /// Input binary file to disassemble
    pub input: Utf8PathBuf,

    /// Disassembling origin (supports hex, decimal, binary, octal)
    #[arg(short = 'o', long, value_parser = parse_u16_value)]
    pub origin: Option<u16>,

    /// Data bloc specification. Three syntaxes supported:
    /// - START-LENGTH: address and size (e.g., 0x1211-7 or message-7)
    /// - START..END: address range with exclusive end (e.g., 0x1211..0x1222 or message..new_line)
    /// - START..=END: address range with inclusive end (e.g., 0x1211..=0x1218 or message..=end_label)
    /// Addresses are in assembly space. START, LENGTH, and END can be numeric values or label names.
    #[arg(short = 'd', long = "data")]
    pub data_bloc: Vec<DataBlocString>,

    /// Set a label at the given address. Format LABEL=ADDRESS
    #[arg(short = 'l', long = "label")]
    pub label: Vec<String>,

    /// Skip the first <SKIP> bytes (supports hex, decimal, binary, octal)
    #[arg(short = 's', long = "skip", value_parser = parse_u16_value)]
    pub skip: Option<u16>,

    /// Output a simple listing that only contains the opcodes
    #[arg(short = 'c', long = "compressed")]
    pub compress: bool,

    /// Output file (if not specified, output to stdout)
    #[arg(short = 'O', long = "output")]
    pub output: Option<Utf8PathBuf>,

    /// Save control file with disassembly directives
    #[arg(long = "save-control")]
    pub save_control: Option<Utf8PathBuf>,

    /// Load control file with disassembly directives
    #[arg(long = "control")]
    pub control: Option<Utf8PathBuf>,

    /// Automatically detect CPC strings in the binary
    #[arg(long = "detect-cpc-strings")]
    pub detect_cpc_strings: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

pub fn process(cli: &BdAsmCli) -> Result<()> {
    // Extract fields we'll need after shadowing cli
    let input_filename = cli.input.clone();
    let output_file = cli.output.clone();
    let save_control_path = cli.save_control.clone();

    // build the control file merging CLI arguments and loaded control file (if any)
    let control_file = {
        let mut control_file = if let Some(ref control_path) = cli.control {
            load_control_file(control_path)?
        } else {
            ControlFile::default()
        };
        
        // Step 2: Merge CLI arguments into control file
        control_file.merge_cli(cli);
        control_file
    };

    let cli = (); // We won't use CLI directly anymore, all info is now in control_file
    
    // Get skip bytes value for later use
    let skip_bytes = control_file.get_skip();
    
    // Get the bytes to disassemble
    let input_bytes = std::fs::read(input_filename)?;
    
    // Check if there is an amsdos header and remove it if any
    let (input_bytes, amsdos_load) = if input_bytes.len() > 128 {
        let header = AmsdosHeader::from_buffer(&input_bytes);
        if header.is_checksum_valid() {
            println!("Amsdos header detected and removed");
            (&input_bytes[128..], Some(header.loading_address()))
        } else {
            (input_bytes.as_ref(), None)
        }
    } else {
        (input_bytes.as_ref(), None)
    };

    // Check if first bytes need to be removed
    let input_bytes = if skip_bytes > 0 {
        eprintln!("; Skip {skip_bytes} bytes");
        &input_bytes[skip_bytes..]
    } else {
        input_bytes
    };

    // Disassemble
    eprintln!("; 0x{:x} bytes to disassemble", input_bytes.len());

    // Convert control file to BdAsmEnv
    let mut env = BdAsmEnv::from_control_file(&control_file)?;
    
    // Override origin with amsdos header if not set
    if env.origin.is_none() {
        env.origin = amsdos_load;
    }

    // Create the listing and inject labels
    let mut listing = env.create_listing(input_bytes)?;
    env.inject_labels(&mut listing, input_bytes.len())?;

    // Generate output
    let output_content = listing.to_string();

    // Write output to file or stdout
    if let Some(ref output_file) = output_file {
        std::fs::write(output_file, output_content)?;
    } else {
        print!("{}", output_content);
    }

    // Save control file if requested
    if let Some(ref control_path) = save_control_path {
        let mut control = ControlFile {
            directives: Vec::new(),
        };

        // Add origin if present
        if let Some(origin) = env.origin {
            control
                .directives
                .push(control_file::ControlDirective::Origin(origin));
        }

        // Add skip directive if present
        if skip_bytes > 0 {
            control
                .directives
                .push(control_file::ControlDirective::Skip(skip_bytes));
        }

        // Add labels
        for (address, label) in env.address2label.iter() {
            control
                .directives
                .push(control_file::ControlDirective::Label {
                    name: label.to_string(),
                    address: *address,
                });
        }

        save_control_file(control_path, &control)?;
    }

    Ok(())
}

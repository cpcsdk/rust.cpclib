#![feature(let_chains)]
#![feature(const_mut_refs)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use clap::{Arg, ArgAction, Command};
use cpclib_asm::preamble::*;
use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::clap;
use cpclib_common::smol_str::SmolStr;
use cpclib_common::winnow::error::ParseError;
use cpclib_common::winnow::Parser;
use cpclib_disc::amsdos::AmsdosHeader;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

static DESC_BEFORE: &str = const_format::formatc!(
    "Profile {} compiled: {}",
    built_info::PROFILE,
    built_info::BUILT_TIME_UTC
);

/// Several expressions can refere to addresses
/// TODO move it in a public library
/// TODO refactor with inject_labels_into_expressions to share common patterns
fn collect_addresses_from_expressions(listing: &Listing) -> Vec<u16> {
    let mut labels: Vec<u16> = Default::default();

    let mut current_address: Option<u16> = None;
    for current_instruction in listing.iter() {
        if let Token::OpCode(Mnemonic::Djnz, Some(DataAccess::Expression(e)), ..)
        | Token::OpCode(Mnemonic::Jr, _, Some(DataAccess::Expression(e)), _) =
            current_instruction
        {
            let address = if let Expr::Label(l) = e
                && l == "$"
            {
                current_address.unwrap() // address before instruction
            }
            else {
                let delta = e.eval().unwrap().int().unwrap() + 2;
                (*current_address.as_ref().unwrap() as i32 + delta) as _
            };
            labels.push(address);
        }
        else if let Token::OpCode(Mnemonic::Ld, Some(DataAccess::Memory(e)), ..)
        | Token::OpCode(Mnemonic::Ld, _, Some(DataAccess::Memory(e)), _) =
            current_instruction
        {
            let address = e.eval().unwrap().int().unwrap();
            labels.push(address as u16);
        }

        let next_address = if let Token::Org { val1: address, .. } = current_instruction {
            current_address = Some(address.eval().unwrap().int().unwrap() as u16);
            current_address
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

        current_address = next_address;
    }

    labels
}

/// TODO refactor with collect_addresses_from_expressions to share common patterns
fn inject_labels_into_expressions(listing: &mut Listing) {
    let (_bytes, table) = cpclib_asm::assemble_tokens_with_options(listing, Default::default())
        .expect("Impossible to assemble the listing, there is an error somewhere");

    let address_to_label = {
        let mut address_to_label = HashMap::<u16, &str>::default();
        for (s, v) in table.expression_symbol() {
            if s.value() == "$" || s.value() == "$$" {
                continue;
            }

            match v {
                Value::Expr(expr) => {
                    if expr.is_int() {
                        address_to_label.insert(v.integer().unwrap() as u16, s.value());
                    }
                },
                Value::String(_) => {},
                Value::Address(a) => {
                    address_to_label.insert(a.address(), s.value());
                },
                Value::Macro(_) => todo!(),
                Value::Struct(_) => todo!(),
                Value::Counter(_) => todo!()
            }
        }
        address_to_label
    };

    let update_expr_address = move |e: &mut Expr, value: u16| {
        if let Some(label) = address_to_label.get(&value) {
            *e = Expr::Label(SmolStr::from(*label));
        }
    };

    let mut current_address: Option<u16> = None;
    for current_instruction in listing.iter_mut() {
        let next_address = if let Token::Org { val1: address, .. } = current_instruction {
            current_address = Some(address.eval().unwrap().int().unwrap() as u16);
            current_address
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

        if let Token::OpCode(Mnemonic::Djnz, Some(DataAccess::Expression(e)), ..)
        | Token::OpCode(Mnemonic::Jr, _, Some(DataAccess::Expression(e)), _) =
            current_instruction
        {
            let address = if let Expr::Label(l) = e
                && l == "$"
            {
                current_address.unwrap() // address before instruction
            }
            else {
                let delta = e.eval().unwrap().int().unwrap() + 2;
                (*current_address.as_ref().unwrap() as i32 + delta) as _
            };

            update_expr_address(e, address);
        }
        else if let Token::OpCode(Mnemonic::Ld, Some(DataAccess::Memory(e)), ..)
        | Token::OpCode(Mnemonic::Ld, _, Some(DataAccess::Memory(e)), _) =
            current_instruction
        {
            let address = e.eval().unwrap().int().unwrap() as u16;
            update_expr_address(e, address);
        }

        current_address = next_address;
    }
}

fn main() {
    let matches = Command::new("bdasm")
					.version(built_info::PKG_VERSION)
					.author("Krusty/Benediction")
					.about("Benediction disassembler")
					.before_help(DESC_BEFORE)
					.arg(
						Arg::new("INPUT")
							.help("Input binary file to disassemble.")
							.action(ArgAction::Set)
                            .value_parser(clap::value_parser!(Utf8PathBuf))
							.required(true)
					)
					.arg(
						Arg::new("ORIGIN")
							.help("Disassembling origin")
							.short('o')
							.long("origin")
							.action(ArgAction::Set)
							.required(false)

					)
					.arg(
						Arg::new("DATA_BLOC")
						.help("Relative position that contains data for a given size. Format: RELATIVE_START(in hexadecimal)-SIZE(in decimal)")
						.short('d')
						.long("data")
                        .action(ArgAction::Append)
					)
					.arg(
						Arg::new("LABEL")
						.help("Set a label at the given address. Format LABEL=ADDRESS")
						.short('l')
						.long("label")
						.action(ArgAction::Append)
					)
                    .arg(
                        Arg::new("SKIP")
                        .help("Skip the first <SKIP> bytes")
                        .short('s')
                        .long("SKIP")
                        .action(ArgAction::Set)
                        .value_parser(clap::value_parser!(usize))
                    )
                    .arg(
                        Arg::new("COMPRESS")
                        .help("Output a simple listing that only contains the opcodes")
                        .short('c')
                        .long("compressed")
                        .action(ArgAction::SetTrue)
                    )
					.get_matches();

    // Get the bytes to disassemble
    let input_filename: &Utf8PathBuf = matches.get_one("INPUT").unwrap();
    let mut input_bytes = Vec::new();
    let mut file = File::open(input_filename).expect("Unable to open file");
    file.read_to_end(&mut input_bytes)
        .expect("Unable to read file");

    // check if there is an amsdos header and remove it if any
    let (input_bytes, amsdos_load) = if input_bytes.len() > 128 {
        let header = AmsdosHeader::from_buffer(&input_bytes);
        if header.is_checksum_valid() {
            println!("Amsdos header detected and removed");
            (&input_bytes[128..], Some(header.loading_address()))
        }
        else {
            (input_bytes.as_ref(), None)
        }
    }
    else {
        (input_bytes.as_ref(), None)
    };

    // check if first bytes need to be removed
    let input_bytes = if let Some(skip) = matches.get_one::<usize>("SKIP") {
        eprintln!("; Skip {} bytes", skip);
        &input_bytes[*skip..]
    }
    else {
        input_bytes
    };

    // Disassemble
    eprintln!("; 0x{:x} bytes to disassemble", input_bytes.len());

    // Retreive the listing
    // TODO move that in its own function
    let mut listing: Listing = if matches.contains_id("DATA_BLOC") {
        // retreive the blocs and parse them
        let mut blocs = matches
            .get_many::<String>("DATA_BLOC")
            .unwrap()
            .map(|bloc| {
                let split = bloc.split('-').collect::<Vec<_>>();
                let start = usize::from_str_radix(split[0], 16).unwrap();
                let length = match usize::from_str_radix(split[1], 10) {
                    Ok(l) => Some(l),
                    Err(_) => None
                };
                (start, length)
            })
            .collect::<Vec<_>>();
        blocs.sort();

        // make the listing for each of the blocs
        let mut listings: Vec<Listing> = Vec::new();
        let mut current_idx = 0;
        while !blocs.is_empty() {
            let &(bloc_idx, bloc_length) = blocs.first().unwrap();
            if current_idx < bloc_idx {
                listings.push(cpclib_asm::disass::disassemble(
                    &input_bytes[current_idx..(bloc_idx - current_idx)]
                ));
                current_idx = bloc_idx;
            }
            else {
                assert_eq!(current_idx, bloc_idx);
                listings.push(
                    defb_elements(match bloc_length {
                        Some(l) => &input_bytes[current_idx..(current_idx + l)],
                        None => &input_bytes[current_idx..]
                    })
                    .into()
                );
                blocs.remove(0);
                current_idx += match bloc_length {
                    Some(l) => l,
                    None => input_bytes.len() - current_idx
                };
            }
        }
        if current_idx < input_bytes.len() {
            listings.push(cpclib_asm::disass::disassemble(&input_bytes[current_idx..]));
        }

        // merge the blocs
        listings
            .into_iter()
            .fold(Listing::new(), |mut lst, current| {
                lst.inject_listing(&current);
                lst
            })
    }
    else {
        // no blocs
        cpclib_asm::disass::disassemble(input_bytes)
    };

    // add origin if any
    if let Some(address) = matches.get_one::<String>("ORIGIN") {
        let address = address.as_bytes();
        let origin: Result<u32, ParseError<_, ()>> = cpclib_common::parse_value.parse(address);
        let origin = origin.expect("Unable to parse origin") as u16;
        listing.insert(0, org(origin));
    }
    else if let Some(origin) = amsdos_load {
        listing.insert(0, org(origin));
    }

    // add labels
    let mut labels = if let Some(labels) = matches.get_many::<String>("LABEL") {
        labels
            .map(|label| {
                let split = label.split('=').collect::<Vec<_>>();
                assert_eq!(2, split.len());
                let label = split[0];
                let address = split[1].as_bytes();
                let address: Result<u32, ParseError<_, ()>> =
                    cpclib_common::parse_value.parse(address);
                let address = address.expect("Unable to parse label value") as u16;
                (address, Cow::Borrowed(label))
            })
            .collect::<HashMap<u16, Cow<str>>>()
    }
    else {
        Default::default()
    };

    // get extra labels
    for address in collect_addresses_from_expressions(&listing) {
        let entry = labels.entry(address);
        entry.or_insert(Cow::Owned(format!("label_{:.4x}", address)));
    }
    listing.inject_labels(labels);
    inject_labels_into_expressions(&mut listing);

    if matches.get_flag("COMPRESS") {
        println!("{}", listing.to_string());
    }
    else {
        let mut options = EnvOptions::default();
        options.write_listing_output(std::io::stdout());
        cpclib_asm::assemble_with_options(&listing.to_string(), options);
    }
}

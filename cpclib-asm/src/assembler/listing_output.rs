use std::{fmt::Debug, io::Write, ops::Index};

use cpclib_tokens::Token;
use cpclib_common::itertools::Itertools;
use cpclib_common::nom::ExtendInto;
use cpclib_common::smallvec::SmallVec;
use std::rc::Rc;
use std::ops::Deref;
use crate::preamble::LocatedToken;

/// Generate an output listing.
/// Can be useful to detect issues
pub struct ListingOutput {
	/// Writer that will contains the listing/
	/// The listing is produced line by line and not token per token
	writer: Box<dyn Write>,
	/// Filename of the current line
	current_fname: Option<String>,
	activated: bool,

	/// Bytes collected at the current line
	current_line_bytes: SmallVec<[u8; 4]>,
	/// Complete source
	current_source: Option<Rc<String>>,
	/// Line number and line content. 
	current_line: Option<(u32, String)>, // clone view of the line XXX avoid this clone
	current_first_address: u32

}	

impl Debug for ListingOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl ListingOutput {

	/// Build a new ListingOutput that will write everyting in writter
	pub fn new<W:'static + Write>(writer: W) -> Self {
		Self {
			writer: Box::new(writer),
			current_fname: None,
			activated: false,
			current_line_bytes: Default::default(),
			current_line: None,
			current_source: None,
			current_first_address: 0
		}
	}


	fn bytes_per_line(&self) -> usize {
		8
	}

	/// Check if the token is for the same source
	fn token_is_on_same_source(&self, token: &LocatedToken) -> bool {
		match &self.current_source {
			Some(current_source) => {
				std::ptr::eq(
					token.context().0.deref().as_ptr(), 
					current_source.as_str().as_ptr()
				)
			},
			None => false
		}
	}

	/// Check if the token is for the same line than the previous token
	fn token_is_on_same_line(&self, token: &LocatedToken) -> bool {
		match &self.current_line {
			Some( (current_location, current_line)) => {
				self.token_is_on_same_source(token) &&
					*current_location == token.span().location_line()
			},
			None => false
		}
	}

	/// Add a token for the current line
	pub fn add_token(&mut self, token: &LocatedToken, bytes: &[u8], address: u32) {
		if ! self.activated {return;}

		if !self.token_is_on_same_line(token) {
			self.process_current_line(); // request a display

			// replace the objects of interest
			self.current_source = Some(token.context().0.clone());
			let current_line  = unsafe{
				std::str::from_utf8_unchecked(
					token.span().get_line_beginning()
				)
			}.to_owned();
			// TODO manage differently for macros and so on
			//let current_line = current_line.split("\n").next().unwrap_or(current_line);
			self.current_line = Some((
				token.span().location_line(),
				current_line
			));
			self.current_first_address = address;
			self.manage_fname(token);
		}

		self.current_line_bytes.extend_from_slice(bytes);


	}

	pub fn process_current_line(&mut self) {

		// retrieve the line
		let (line_number, line) = match &self.current_line {
			Some((idx, line)) => (idx, line),
			None => return
		};

		// build the iterators over the line representation of source code and data
		let mut line_representation = line.split("\n");
		let data_representation = self.current_line_bytes.iter()
				.chunks(self.bytes_per_line())
				.into_iter()
				.map(|c| {
					c.map(|b| format!("{:02X}", b))
					.join(" ")
				}).collect_vec();
		let mut data_representation = data_representation.iter();

		// TODO manage missing end of files/blocks if needed

		// draw all line
		let mut idx = 0;
		loop {

			let current_inner_line = line_representation.next();
			let current_inner_data = data_representation.next();

			if current_inner_data.is_none() && current_inner_line.is_none() {
				break;
			}

			let loc_representation = if false /*(data_representation.is_empty() && !self.current_address_is_value) || idx!=0 */{
				"    ".to_owned()
			} else {
				format!("{:04X}", self.current_first_address)
			};

			

			let line_nb_representation = if current_inner_line.is_none() {
				"    ".to_owned()
			} else {
				format!("{:4}", line_number+idx)
			};

			writeln!(
				self.writer,
				"{} {} {:bytes_width$} {} ",
				line_nb_representation,
				loc_representation,
				current_inner_data.unwrap_or(&"".to_owned()),
				current_inner_line.unwrap_or(""),
				
				bytes_width = self.bytes_per_line()*3
			).unwrap();
		
			idx += 1;
		}

				

		// cleanup all the fields of the current line
		self.current_line = None;
		self.current_source = None;
		self.current_line_bytes.clear();
	}


	pub fn finish(&mut self) {
	
	}


	/// Print filename if needed
	pub fn manage_fname(&mut self, token: &LocatedToken) -> Option<String> {

	//	dbg!(token);

		let ctx = &token.span().extra.1;
		let fname = ctx.current_filename.as_ref()
			.map(|p| p.as_os_str().to_str().unwrap().to_string())
			.or_else(||{
				ctx.context_name.clone()
			});

		match fname {
			Some(fname) => {
				let print = match self.current_fname.as_ref() {
					Some(current_fname) => {
						*current_fname != fname
					},
					None => true
				};
	
				if print {
					self.current_fname = Some(fname.clone());
					 Some(format!("Context: {}", fname))
				} else {
					None
				}
			},
			None => None
		}


	}

	pub fn on(&mut self) {
		self.activated = true;
	}

	pub fn off(&mut self) {
		self.finish();
		self.activated = false;
	}

}
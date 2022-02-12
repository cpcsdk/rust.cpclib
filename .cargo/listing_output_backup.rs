use std::{fmt::Debug, io::Write, ops::Index};

use cpclib_tokens::Token;
use cpclib_common::itertools::Itertools;
use cpclib_common::nom::ExtendInto;
use std::rc::Rc;
use std::ops::Deref;
use crate::preamble::LocatedToken;

/// Generate an output listing.
/// Can be useful to detect issues
pub struct ListingOutput {
	/// Writer that will contains the listing/
	/// The listing is produced line by line and not token per token
	writer: Box<dyn Write>,

	// the complete source
	current_source: Option<Rc<String>>,
	/// The line that will be printed when all the tokens will be injected (ptr, len, line number in source)
	/// XXX address is not static at all but corresponds to the life of the current source
	current_line: Option<(u32, &'static str)>,
	/// The data generated for the current line
	current_data: Vec<u8>,
	/// The adress of the first token of the line
	current_first_address: u32,
	current_address_is_value: bool,
	/// The name of the file containing the token
	current_fname: Option<String>,

	/// Set to true when listing is properly handled
	activated: bool
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
			current_source: None,
			current_fname: None,
			current_line: None,
			current_data: Vec::new(),
			current_first_address: 0,
			current_address_is_value: false,
			activated: true
		}
	}


	/// Print the data for the current line
	fn process_current_line(&mut self) {
		// rebuild the string
		let (line_representation, current_line) = self.current_line.take().unwrap();
		//let line_representation = String::from_utf8(unsafe{std::slice::from_raw_parts(ptr, len).to_vec()}).unwrap_or("BUG".to_owned());
		let mut line_representation = line_representation.split("\n");
		// TODO include the other lines for macros and so on

		// Split the bytes in several lines if any
		let data_representation = self.current_data.iter()
											.chunks(self.bytes_per_line())
											.into_iter()
											.map(|c| {
												c.map(|b| format!("{:02X}", b))
												.join(" ")
											}).collect_vec();
		let mut data_representation = data_representation.iter();


		static mut FIRST_TOKEN_PROCESSED: bool = false;
		unsafe{ // safe as soon as we have no parallel assembling
			if ! FIRST_TOKEN_PROCESSED {
				let source_ptr = self.current_source.as_ref().unwrap().deref().as_ptr();
				let missing_len = ptr.offset_from(source_ptr.deref()).abs();

				if missing_len > 0 {
					let missing_content = std::slice::from_raw_parts(source_ptr, missing_len as _);
					let missing_content = String::from_utf8_lossy(
						missing_content);
					let missing_content = &missing_content[..missing_content.len()-1] ; // remove last \n

					for (idx, line) in missing_content.split('\n').enumerate() {
						writeln!(
							self.writer,
							"{:4} {} {:bytes_width$} {}",
							idx+1,
							"    ",
							"",
							line,
							bytes_width = self.bytes_per_line()*3
						).unwrap();
					}

				}

				FIRST_TOKEN_PROCESSED = true;
			}
		};

		// draw all line
		let mut idx = 0;
		loop {

			let current_line = line_representation.next();
			let current_data = data_representation.next();

			if current_data.is_none() && current_line.is_none() {
				break;
			}

//			dbg!(self.current_address_is_value, self.current_first_address, data_representation.is_empty());

			let loc_representation = if false /*(data_representation.is_empty() && !self.current_address_is_value) || idx!=0 */{
				"    ".to_owned()
			} else {
				format!("{:04X}", self.current_first_address)
			};

			

			let line_nb_representation = if current_line.is_none() {
				"    ".to_owned()
			} else {
				format!("{:4}", line_number+idx)
			};

			writeln!(
				self.writer,
				"{} {} {:bytes_width$} {} ",
				line_nb_representation,
				loc_representation,
				current_data.unwrap_or(&"".to_owned()),
				current_line.unwrap_or(""),
				
				bytes_width = self.bytes_per_line()*3
			).unwrap();
		
			idx += 1;
		}


		self.current_data.clear();

	}

	fn bytes_per_line(&self) -> usize {
		8
	}

	/// Add a token for the current line
	pub fn add_token(&mut self, token: &LocatedToken, bytes: &[u8], address: u32) {
		if ! self.activated {return;}

		let fname_handling = self.manage_fname(token);
		let source = Rc::clone(&token.span().extra.0);



		// pointer slice on the line
		let token_line = token.span().get_line_beginning(); // in the same space than the source
		let token_line = token_line.split(|c| *c == '\n' as u8).next().unwrap_or(token_line);
		let token_line = unsafe{std::str::from_utf8_unchecked(token_line)};


		let token_line_number = token.span().location_line(); 

		if self.current_line.is_none() {
			// first call, we add the info
			self.current_source = Some(source);
			self.current_line = Some((token_line_number, token_line));
			self.current_data.extend_from_slice(bytes);
			self.current_first_address = address;
			self.current_address_is_value = if let LocatedToken::Standard{
				token: Token::Equ(_, _),
				..
			} = token {true} else {false};
		}
		else {
			let current_line_number = self.current_line.as_ref().unwrap().0;
			if current_line_number == token_line_number && 
				source.as_ref().as_ptr() == self.current_source.as_ref().unwrap().as_ptr() {
				// still the same line of the same source, just collect the bytes
				self.current_data.extend_from_slice(bytes);
			}
			else {
				// need to purge the previously collected data
				// enlarge if needed the first line

				// display previous
				self.process_current_line();

				// add new token
				self.add_token(token, bytes, address); // avoid copy paste of similar code
			}
		}


		if let Some(line) = fname_handling {
			writeln!(self.writer, "{}", line).unwrap();
		}
	}

	pub fn finish(&mut self) {
		if self.current_line.is_none() {return;}

		let source = self.current_source.as_ref().unwrap();
		 
		// ensure we display the end of the file
		let new_line_length = unsafe{source.len() - (source.as_ptr().offset_from(self.current_line.as_ref().unwrap().0).abs() as usize)};
		self.current_line.as_mut().unwrap().1 = new_line_length as _;
		self.process_current_line()
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
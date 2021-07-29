use std::{fmt::Debug, io::Write};

use itertools::Itertools;
use lazy_static::__Deref;
use nom::ExtendInto;


use crate::preamble::LocatedToken;

/// Generate an output listing.
/// Can be useful to detect issues
pub struct ListingOutput {
	/// Writer that will contains the listing/
	/// The listing is produced line by line and not token per token
	writer: Box<dyn Write>,

	// the complete source
	current_source: Option<(*const u8, usize)>,
	/// The line that will be printed when all the tokens will be injected
	current_line: Option<(*const u8, usize)>,
	/// The data generated for the current line
	current_data: Vec<u8>,
	/// The adress of the first token of the line
	current_first_address: u32,

	current_fname: Option<String>
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
			current_first_address: 0
		}
	}


	/// Print the data for the current line
	fn process_current_line(&mut self) {
		// rebuild the string
		let (ptr, len) = self.current_line.take().unwrap();
		let line_representation = String::from_utf8_lossy(unsafe{std::slice::from_raw_parts(ptr, len)});
		let mut line_representation = line_representation.split("\n")
			.map(|l| l.trim_end_matches("\n"));
		// TODO include the other lines for macros and so on

		// Split the bytes in several lines if any
		let data_representation = self.current_data.iter()
											.chunks(self.bytes_per_line())
											.into_iter()
											.map(|c| {
												c.map(|b| format!("{:X}", b))
												.join(" ")
											}).collect_vec();
		let mut data_representation = data_representation.iter();



		// draw all line
		let mut first = true;
		loop {

			let loc_representation = if data_representation.is_empty() || !first{
				"    ".to_owned()
			} else {
				format!("{:04X}", self.current_first_address)
			};

			let current_line = line_representation.next();
			let current_data = data_representation.next();

			if current_data.is_none() && current_line.is_none() {
				break;
			}


			writeln!(
				self.writer,
				"{} {:bytes_width$} {}",
				loc_representation,
				current_data.unwrap_or(&"".to_owned()),
				current_line.unwrap_or(""),
				bytes_width = self.bytes_per_line()*3
			).unwrap();
		
			first = false;


		}


		self.current_data.clear();

	}

	fn bytes_per_line(&self) -> usize {
		8
	}

	/// Add a token for the current line
	pub fn add_token(&mut self, token: &LocatedToken, bytes: &[u8], address: u32) {


		self.manage_fname(token);



		// pointer slice on the line
		let token_line = token.span().get_line_beginning();
		let token_line_len = token_line.len();
		let token_line = token_line.as_ptr();
		let token_line_desc = (token_line, token_line_len);

		// pointer slice on the code
		let source = &token.span().extra.0.as_str();
		let source_len = source.len();
		let source = source.as_ptr();
		let source_desc = (source, source_len);

		if self.current_line.is_none() {
			// first call, we add the info
			self.current_source = Some(source_desc);
			self.current_line = Some(token_line_desc);
			self.current_data.extend_from_slice(bytes);
			self.current_first_address = address;
		}
		else {
			let (current_line, _) = *self.current_line.as_ref().unwrap();

			if std::ptr::eq(current_line, token_line) {
				// still the same line
				self.current_data.extend_from_slice(bytes);
			}
			else {
				// enlarge if needed the first line
				if source_desc == *self.current_source.as_ref().unwrap() {
					// update the texte length to allow multilines directives to be properly handled
					let new_line_length = unsafe{token_line_desc.0.offset_from( self.current_line.as_ref().unwrap().0)};


					self.current_line.as_mut().unwrap().1 = new_line_length as _;


				}


				// display previous
				self.process_current_line();

				// add new token
				self.add_token(token, bytes, address); // avoid copy paste of similar code
			}
		}
	}

	pub fn finish(&mut self) {
		// ensure we display the end of the file
		let new_line_length = unsafe{self.current_source.as_ref().unwrap().1 - (self.current_source.as_ref().unwrap().0.offset_from(self.current_line.as_ref().unwrap().0).abs() as usize)};
		self.current_line.as_mut().unwrap().1 = new_line_length as _;
		self.process_current_line()
	}


	/// Print filename if needed
	pub fn manage_fname(&mut self, token: &LocatedToken) {


		let ctx = &token.span().extra.1;
		let mut fname = ctx.current_filename.as_ref()
			.map(|p| p.as_os_str().to_str().unwrap().to_string());

		fname.or_else(||{
			ctx.context_name.clone()
		})
		.and_then(|fname| {

			let print = match self.current_fname.as_ref() {
				Some(current_fname) => {
					*current_fname != fname
				},
				None => true
			};

			if print {
				writeln!(self.writer, "Context: {}", fname).unwrap();
				self.current_fname = Some(fname);
			}

			Some(())
		});

		
	}

}
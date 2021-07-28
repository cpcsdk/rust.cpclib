use std::{fmt::Debug, io::Write};

use itertools::Itertools;


use crate::preamble::LocatedToken;

/// Generate an output listing.
/// Can be useful to detect issues
pub struct ListingOutput {
	/// Writer that will contains the listing/
	/// The listing is produced line by line and not token per token
	writer: Box<dyn Write>,

	/// The line that will be printed when all the tokens will be injected
	current_line: Option<(*const u8, usize)>,
	/// The data generated for the current line
	current_data: Vec<u8>,
	/// The adress of the first token of the line
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
			current_line: None,
			current_data: Vec::new(),
			current_first_address: 0
		}
	}


	/// Print the data for the current line
	fn process_current_line(&mut self) {
		// rebuild the string
		let (ptr, len) = self.current_line.take().unwrap();
		let line_representation = String::from_utf8_lossy(unsafe{std::slice::from_raw_parts(ptr, len)}).to_string();
		// TODO include the other lines for macros and so on

		// Split the bytes in several lines if any
		let data_representation = self.current_data.iter()
											.chunks(self.bytes_per_line())
											.into_iter()
											.map(|c| {
												c.map(|b| format!("{:X}", b))
												.join(" ")
											})
											.collect_vec();
		self.current_data.clear();

		// draw the first line
		writeln!(
			self.writer,
			"{:04X} {:bytes_width$} {}",
			self.current_first_address,
			data_representation.get(0).unwrap_or(&"".to_owned()),
			line_representation,
			bytes_width = self.bytes_per_line()*3
		).unwrap();

		for i in 1..data_representation.len() {
			writeln!(
				self.writer,
				"{:04X} {:bytes_width$}",
				self.current_first_address,
				data_representation.get(i).unwrap(),
				bytes_width = self.bytes_per_line()*3
			).unwrap();
		

		}
	}

	fn bytes_per_line(&self) -> usize {
		8
	}

	/// Add a token for the current line
	pub fn add_token(&mut self, token: &LocatedToken, bytes: &[u8], address: u32) {
		let token_line = token.span().get_line_beginning();
		let token_line_size = token_line.len();
		let token_line = token_line.as_ptr();

		if self.current_line.is_none() {
			// first call, we add the info
			self.current_line = Some((token_line, token_line_size));
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
				// new line to handle
				self.process_current_line();
				self.add_token(token, bytes, address); // avoid copy paste of similar code
			}
		}
	}

	pub fn finish(&mut self) {
		self.process_current_line()
	}

}
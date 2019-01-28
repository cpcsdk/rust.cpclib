pub mod tokens;
pub mod parser;

use tokens::BasicToken;
use parser::parse_basic_program;
use std::collections::HashMap;


/// Basic line of code representation
#[derive(Debug, Clone)]
pub struct BasicLine {
	line_number: u16,
	tokens: Vec<tokens::BasicToken>
}

impl BasicLine {
	pub fn line_number(&self) -> u16 {
		self.line_number
	}

	/// Produces the end line
	pub fn end_line() -> BasicLine {
		BasicLine {
			line_number: 0,
			tokens: Vec::new()
		}
	}

	/// Create a line with its content
	pub fn new(line_number: u16, tokens: &[tokens::BasicToken]) -> BasicLine {
		BasicLine {
			line_number,
			tokens: tokens.to_vec()
		}
	}

	/// Returns the encoded line.
	/// - 2 bytes for data length
	/// - 2 bytes for line number
	/// - n bytes for tokens
	/// - 1 bytes for end of line marker
	pub fn as_bytes(&self) -> Vec<u8> {
		let tokens = self.tokens.iter()
			.flat_map(|t|{t.as_bytes()})
			.collect::<Vec<u8>>();
		let size = tokens.len() + 2 + 2 + 1;

		let mut content = vec![
			(size%256) as u8,
			(size/256) as u8,
			(self.line_number%256) as u8,
			(self.line_number/256) as u8
		];
		content.extend_from_slice(&tokens);
		content.push(0);

		content
	}
}

pub struct BasicProgram {
	lines: HashMap<u16, BasicLine>
}

impl BasicProgram {

	/// Create the program from a list of lines
	pub fn new(mut lines: Vec<BasicLine>) -> BasicProgram {
		let mut prog = BasicProgram {
			lines: Default::default()
		};

		for line in lines.drain(..) {
			prog.add_line(line);
		}

		prog
	}

	/// Create the program from a code to parse
	pub fn parse<S:AsRef<str>>(code: S) -> Result<BasicProgram, String> {
		match parse_basic_program(code.as_ref().into()) {
			Ok((res, prog)) => {
				if res.len() != 0 {
					Err(
						format!("Basic content has not been totally parsed: {}", res)
					)
				}
				else {
					Ok(prog)
				}
            },
            Err(e) => {
				Err(format!("Error while parsing the Basic content: {}", e))
            }
		}
	}

	/// Add a line to the program. If a line with the same number already exists, it is replaced
	pub fn add_line(&mut self, line: BasicLine) {
		self.lines.insert(
			line.line_number(),
			line
		);
	}


	pub fn as_bytes(&self) -> Vec<u8> {
		unimplemented!("[ERROR] we have to add the latest line");
		self.lines.iter()
			.flat_map(|(k,v)|{
				v.as_bytes()
			})
			.collect::<Vec<u8>>()
	}
}


#[cfg(test)]
mod test {
 	use crate::basic::*;

	#[test]
	fn test_parse() {
		let code = "10 call &0: call &0\n";
		BasicProgram::parse(code).expect("Unable to produce basic tokens");

		let code = "10 call &0: call &0";
		BasicProgram::parse(code).expect("Unable to produce basic tokens");


	}
}
/// All the stuff to parse z80 code.
pub mod parser;

/// Definition of the tokens.
pub mod tokens;

/// Production of the bytecodes from the tokens.
pub mod assembler;

/// Utility functions to manually create tokens.
pub mod builder;


/// Assemble a piece of code and returns the associated list of bytes
pub fn assemble(code: &str) -> Result<Vec<u8>, String> {

	let tokens = match parser::parse_str(code.into()) {
			Err(e) => return Err(e),
			Ok(tokens) => tokens
	};
	
	let env = match assembler::visit_tokens(&tokens) {
		Err(e) => return Err(e),
		Ok(env) => env
	};

	Ok(env.produced_bytes())
}

#[cfg(test)]
mod test_super {
	use super::*;

	#[test]
	fn simple_test_assemble() {
		let code = "
		org 0
		db 1, 2
		db 3, 4
		";

		let bytes = assemble(code)
					.unwrap_or_else(|e|panic!("Unable to assemble {}: {}", code, e));
		assert_eq!(bytes.len(), 4);
		assert_eq!(bytes, vec![1, 2, 3, 4]);
	}



	#[test]
	fn located_test_assemble() {
		let code = "
		org 0x100
		db 1, 2
		db 3, 4
		";

		let bytes = assemble(code)
					.unwrap_or_else(|e|panic!("Unable to assemble {}: {}", code, e));
		assert_eq!(bytes, vec![1, 2, 3, 4]);
	}
}
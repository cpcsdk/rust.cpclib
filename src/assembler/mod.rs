/// All the stuff to parse z80 code.
pub mod parser;

/// Definition of the tokens.
pub mod tokens;

/// Production of the bytecodes from the tokens.
pub mod assembler;

/// Utility functions to manually create tokens.
pub mod builder;


#[derive(Debug, Fail)]
pub enum AssemblerError {
    #[fail(display = "Assembling bug: {}", msg)]
    BugInAssembler {
        msg: String,
    },
    #[fail(display = "Parser bug: {}", error)]
    BugInParser {
        error: String,
    },

	// TODO add more information
    #[fail(display = "Syntax error: {}", error)]
    SyntaxError {
        error: String
	},

	// TODO add more information
    #[fail(display = "Assembling error: {}", msg)]
    AssemblingError {
        msg: String
    },
    
	// TODO remove this case and dispatch it everywhere else
	#[fail(display = "To be sorted error: {}", msg)]
	GenericError {
		msg: String
	},

	#[fail(display = "Unknown symbol: {}", symbol)]
	UnknownSymbol {
		symbol: String
	},

	#[fail(display = "Current assembling address is unknown.")]
	UnknownAssemblingAddress,

	#[fail(display = "Unable to resolve expression {}.", expression)]
	ExpressionUnresolvable {
		expression: crate::assembler::tokens::expression::Expr
	}
}


impl From<String> for AssemblerError {
    fn from(msg: String) -> AssemblerError {
        AssemblerError::GenericError{
            msg
        }
    }
}

impl From<&String> for AssemblerError {
    fn from(msg: &String) -> AssemblerError {
        AssemblerError::GenericError{
            msg: msg.to_string()
        }
    }
}



/// Assemble a piece of code and returns the associated list of bytes
pub fn assemble(code: &str) -> Result<Vec<u8>, AssemblerError> {

	let tokens = parser::parse_str(code.into())?;
	let env = assembler::visit_tokens_all_passes(&tokens)?;

	Ok(env.produced_bytes())
}

pub fn assemble_and_table(code: &str) -> Result< (Vec<u8>, assembler::SymbolsTable), AssemblerError > {

	let tokens = parser::parse_str(code.into())?;
	let env = assembler::visit_tokens_all_passes(&tokens)?;

	Ok((
		env.produced_bytes(),
		env.symbols().clone()
	))
}

pub fn assemble_with_table(code: &str, table: &assembler::SymbolsTable) -> Result< (Vec<u8>, assembler::SymbolsTable), AssemblerError> {

	let tokens = parser::parse_str(code.into())?;
	let env = assembler::visit_tokens_all_passes_with_table(
		&tokens, 
		table)?;

	Ok((
		env.produced_bytes(),
		env.symbols().clone()
	))
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
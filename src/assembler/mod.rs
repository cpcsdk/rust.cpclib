/// All the stuff to parse z80 code.
pub mod parser;

/// Definition of the tokens.
pub mod tokens;

/// Production of the bytecodes from the tokens.
pub mod assembler;

/// Utility functions to manually create tokens.
pub mod builder;
use crate::assembler::parser::ParserContext;

use crate::basic::BasicError;
use failure::Fail;

#[derive(Debug, Fail)]
#[allow(missing_docs)]
pub enum AssemblerError {
    #[fail(display = "Assembling bug: {}", msg)]
    BugInAssembler { msg: String },
    #[fail(display = "Parser bug: {}. Context: {:?}", error, context)]
    BugInParser {
        error: String,
        context: ParserContext,
    },

    // TODO add more information
    #[fail(display = "Syntax error: {}", error)]
    SyntaxError { error: String },

    #[fail(display = "Basic error: {}", error)]
    BasicError { error: String },

    // TODO add more information
    #[fail(display = "Assembling error: {}", msg)]
    AssemblingError { msg: String },

    // TODO remove this case and dispatch it everywhere else
    #[fail(display = "To be sorted error: {}", msg)]
    GenericError { msg: String },

    #[fail(display = "Assertion failed -- {}: {}", test, msg)]
    AssertionFailed { test: String, msg: String },

    #[fail(display = "Symbol `{}`already present on the symbol table", symbol)]
    SymbolAlreadyExists { symbol: String },

    #[fail(display = "Unknown symbol: {}. Closest one is: {:?}", symbol, closest)]
    UnknownSymbol {
        symbol: String,
        closest: Option<String>,
    },

    #[fail(display = "IO error: {}", msg)]
    IOError { msg: String },

    #[fail(display = "Current assembling address is unknown.")]
    UnknownAssemblingAddress,

    #[fail(display = "Unable to resolve expression {}.", expression)]
    ExpressionUnresolvable {
        expression: crate::assembler::tokens::expression::Expr,
    },
}

impl From<String> for AssemblerError {
    fn from(msg: String) -> AssemblerError {
        AssemblerError::GenericError { msg }
    }
}

impl From<&String> for AssemblerError {
    fn from(msg: &String) -> AssemblerError {
        AssemblerError::GenericError {
            msg: msg.to_string(),
        }
    }
}

impl From<BasicError> for AssemblerError {
    fn from(msg: BasicError) -> AssemblerError {
        AssemblerError::BasicError {
            error: msg.to_string(),
        }
    }
}

/// Configuration of the assembler. By default the assembler is case sensitive and has no symbol
#[derive(Clone, Debug)]
pub struct AssemblingOptions {
    /// Set to true to consider that the assembler pay attention to the case of the labels
    case_sensitive: bool,
    /// Contains some symbols that could be used during assembling
    symbols: assembler::SymbolsTable,
}

impl Default for AssemblingOptions {
    fn default() -> AssemblingOptions {
        AssemblingOptions {
            case_sensitive: true,
            symbols: Default::default(),
        }
    }
}

#[allow(missing_docs)]
impl AssemblingOptions {
    pub fn new_case_sensitive() -> Self {
        Self::default()
    }

    pub fn new_case_insensitive() -> Self {
        let mut options = Self::new_case_sensitive();
        options.case_sensitive = false;
        options
    }

    /// Creation an option object with the given symbol table
    pub fn new_with_table(symbols: &assembler::SymbolsTable) -> Self {
        let mut options = Self::default();
        options.set_symbols(symbols);
        options
    }

    /// Specify if the assembler must be case sensitive or not
    pub fn set_case_sensitive(&mut self, val: bool) -> &mut Self {
        self.case_sensitive = val;
        self
    }

    /// Specify a symbol table to copy
    pub fn set_symbols(&mut self, val: &assembler::SymbolsTable) -> &mut Self {
        self.symbols = val.clone();
        self
    }

    pub fn symbols(&self) -> &assembler::SymbolsTable {
        &self.symbols
    }

    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }
}

/// Assemble a piece of code and returns the associated list of bytes
pub fn assemble(code: &str) -> Result<Vec<u8>, AssemblerError> {
    assemble_and_table(code).map(|(b, _)| b)
}

#[allow(missing_docs)]
#[deprecated(note = "use assemble_with_options instead.")]
pub fn assemble_and_table(
    code: &str,
) -> Result<(Vec<u8>, assembler::SymbolsTable), AssemblerError> {
    let tokens = parser::parse_str(code.into())?;
    let options = AssemblingOptions::default();
    let env = assembler::visit_tokens_all_passes_with_options(&tokens, &options)?;

    Ok((env.produced_bytes(), env.symbols().as_ref().clone()))
}

#[allow(missing_docs)]
#[deprecated(note = "use assemble_with_options instead.")]
pub fn assemble_with_table(
    code: &str,
    table: &assembler::SymbolsTable,
) -> Result<(Vec<u8>, assembler::SymbolsTable), AssemblerError> {
    let tokens = parser::parse_str(code.into())?;
    let options = AssemblingOptions::new_with_table(table);
    let env = assembler::visit_tokens_all_passes_with_options(&tokens, &options)?;

    Ok((env.produced_bytes(), env.symbols().as_ref().clone()))
}

#[allow(missing_docs)]
pub fn assemble_with_options(
    code: &str,
    options: &AssemblingOptions,
) -> Result<(Vec<u8>, assembler::SymbolsTable), AssemblerError> {
    let tokens = parser::parse_str(code.into())?;
    let env = assembler::visit_tokens_all_passes_with_options(&tokens, &options)?;

    Ok((env.produced_bytes(), env.symbols().as_ref().clone()))
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

        let bytes = assemble(code).unwrap_or_else(|e| panic!("Unable to assemble {}: {}", code, e));
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

        let bytes = assemble(code).unwrap_or_else(|e| panic!("Unable to assemble {}: {}", code, e));
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn case_verification() {
        let code = "
		ld hl, TruC
Truc
		";

        let options = AssemblingOptions::new_case_sensitive();
        println!("{:?}", assemble_with_options(code, &options));
        assert!(assemble_with_options(code, &options).is_err());

        let options = AssemblingOptions::new_case_insensitive();
        println!("{:?}", assemble_with_options(code, &options));
        assert!(assemble_with_options(code, &options).is_ok());
    }
}

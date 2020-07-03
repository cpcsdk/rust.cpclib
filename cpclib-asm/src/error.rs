
use cpclib_basic::BasicError;
use cpclib_tokens::tokens;
use crate::parser::ParserContext;
use cpclib_tokens::symbols::SymbolError;

use cpclib_disc::amsdos::AmsdosError;
use cpclib_disc::amsdos::AmsdosFile;

use failure::Fail;

#[derive(Debug, Fail)]
#[allow(missing_docs)]
pub enum AssemblerError {

    #[fail(display = "Several errors arised: {:?}", errors)]
    MultipleErrors{
        errors: Vec<AssemblerError>
    },

    #[fail(display = "{} cannot be empty.", 0)]
    EmptyBinaryFile(String),

    #[fail(display = "Amsdos error: {}", error)]
    AmsdosError {error: AmsdosError},

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
    
    #[fail(display = "Invalid argument: {}", msg)]
    InvalidArgument {msg: String},

    // TODO remove this case and dispatch it everywhere else
    #[fail(display = "To be sorted error: {}", msg)]
    GenericError { msg: String },

    #[fail(display = "Assertion failed -- {} [{}]: {}", test, guidance, msg)]
    AssertionFailed { test: String, msg: String, guidance: String },

    #[fail(display = "Symbol `{}` already present on the symbol table", symbol)]
    SymbolAlreadyExists { symbol: String },
    
    #[fail(display = "There is no macro named `{}`. Closest one is: {:?}", symbol, closest)]    
    UnknownMacro { symbol: String,  closest: Option<String>},

    #[fail(display = "Error when applying macro {}. {}", name, root)]
    MacroError{
        name: String,
        root: Box<AssemblerError>
    },

    #[fail(display = "Macro `{}` expect {} arguments; {} are provided.", symbol, nb_arguments, nb_paramers)]
    WrongNumberOfParameters{
        symbol: String,
        nb_paramers: usize,
        nb_arguments: usize
    },

    #[fail(display = "Unknown symbol: {}. Closest one is: {:?}", symbol, closest)]
    UnknownSymbol {
        symbol: String,
        closest: Option<String>,
    },

    #[fail(display = "Symbol {} is not a {}", symbol, isnot)]
    WrongSymbolType {
        symbol: String,
        isnot: String
    },

    #[fail(display = "IO error: {}", msg)]
    IOError { msg: String },

    #[fail(display = "Current assembling address is unknown.")]
    UnknownAssemblingAddress,

    #[fail(display = "Unable to resolve expression {}.", expression)]
    ExpressionUnresolvable {
        expression: tokens::Expr,
    },
}

impl From<String> for AssemblerError {
    fn from(msg: String) -> Self {
        AssemblerError::GenericError { msg }
    }
}

impl From<&String> for AssemblerError {
    fn from(msg: &String) -> Self {
        AssemblerError::GenericError {
            msg: msg.to_string(),
        }
    }
}

impl From<BasicError> for AssemblerError {
    fn from(msg: BasicError) -> Self {
        AssemblerError::BasicError {
            error: msg.to_string(),
        }
    }
}

/// TODO generate a real error
impl From<SymbolError> for AssemblerError {
    fn from(err: SymbolError) -> Self {
        AssemblerError::GenericError {
            msg: "Unknown assembling address".to_string(),
        }
    }
}


impl From<AmsdosError> for AssemblerError {
    fn from(err: AmsdosError) -> Self {
        AssemblerError::AmsdosError {
            error: err,
        }
    }
}
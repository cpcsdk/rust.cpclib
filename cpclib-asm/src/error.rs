use std::collections::HashMap;
use std::fmt::Display;

use crate::assembler::AssemblingPass;
use crate::parser::ParserContext;
use crate::Z80Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use cpclib_basic::BasicError;
use cpclib_disc::amsdos::AmsdosError;
use cpclib_tokens::symbols::SymbolError;
use cpclib_tokens::tokens;
use failure::Fail;
use itertools::Itertools;
use nom::error::VerboseError;

#[derive(Debug)]
#[allow(missing_docs)]
pub enum AssemblerError {
    //#[fail(display = "Several errors arised: {:?}", errors)]
    MultipleErrors {
        errors: Vec<AssemblerError>,
    },

    //#[fail(display = "{} cannot be empty.", 0)]
    EmptyBinaryFile(String),

    //#[fail(display = "Amsdos error: {}", error)]
    AmsdosError {
        error: AmsdosError,
    },

    //#[fail(display = "Assembling bug: {}", msg)]
    BugInAssembler {
        msg: String,
    },

    //#[fail(display = "Parser bug: {}. Context: {:?}", error, context)]
    BugInParser {
        error: String,
        context: ParserContext,
    },

    // TODO add more information
    //#[fail(display = "Syntax error:\n{}", error)]
    SyntaxError {
        error: VerboseError<Z80Span>,
    },

    IncludedFileError {
        span: Z80Span,
        error: Box<AssemblerError>,
    },

    //#[fail(display = "Basic error: {}", error)]
    BasicError {
        error: String,
    },

    // TODO add more information
    // #[fail(display = "Assembling error: {}", msg)]
    AssemblingError {
        msg: String,
    },

    // #[fail(display = "Invalid argument: {}", msg)]
    InvalidArgument {
        msg: String,
    },

    // TODO remove this case and dispatch it everywhere else
    // #[fail(display = "To be sorted error: {}", msg)]
    GenericError {
        msg: String,
    },

    //  #[fail(display = "Assertion failed -- {} [{}]: {}", test, guidance, msg)]
    AssertionFailed {
        test: String,
        msg: String,
        guidance: String,
    },

    //  #[fail(display = "Symbol `{}` already present on the symbol table", symbol)]
    SymbolAlreadyExists {
        symbol: String,
    },

    //    #[fail(
    //        display = "There is no macro named `{}`. Closest one is: {:?}",
    //        symbol, closest
    //    )]
    UnknownMacro {
        symbol: String,
        closest: Option<String>,
    },

    //    #[fail(display = "Error when applying macro {}. {}", name, root)]
    MacroError {
        name: String,
        root: Box<AssemblerError>,
    },

    //   #[fail(
    //       display = "Macro `{}` expect {} arguments; {} are provided.",
    //       symbol, nb_arguments, nb_paramers
    //   )]
    WrongNumberOfParameters {
        symbol: String,
        nb_paramers: usize,
        nb_arguments: usize,
    },

    //  #[fail(display = "Unknown symbol: {}. Closest one is: {:?}", symbol, closest)]
    UnknownSymbol {
        symbol: String,
        closest: Option<String>,
    },

    //   #[fail(display = "Symbol {} is not a {}", symbol, isnot)]
    WrongSymbolType {
        symbol: String,
        isnot: String,
    },

    //   #[fail(display = "IO error: {}", msg)]
    IOError {
        msg: String,
    },

    //  #[fail(display = "Current assembling address is unknown.")]
    UnknownAssemblingAddress,

    //  #[fail(display = "Unable to resolve expression {}.", expression)]
    ExpressionUnresolvable {
        expression: tokens::Expr,
    },

    RelativeAddressUncomputable {
        address: i32,
        pass: AssemblingPass,
        error: Box<AssemblerError>,
    },
}

impl From<VerboseError<Z80Span>> for AssemblerError {
    fn from(err: VerboseError<Z80Span>) -> Self {
        AssemblerError::SyntaxError { error: err }
    }
}

impl From<std::io::Error> for AssemblerError {
    fn from(err: std::io::Error) -> Self {
        AssemblerError::IOError {
            msg: err.to_string(),
        }
    }
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
    fn from(_err: SymbolError) -> Self {
        AssemblerError::GenericError {
            msg: "Unknown assembling address".to_string(),
        }
    }
}

impl From<AmsdosError> for AssemblerError {
    fn from(err: AmsdosError) -> Self {
        AssemblerError::AmsdosError { error: err }
    }
}

pub(crate) const LD_WRONG_SOURCE: &'static str = "LD: error in the source";
pub(crate) const LD_WRONG_DESTINATION: &'static str = "LD: error in the destination";

pub(crate) const JP_WRONG_PARAM: &'static str = "JP: error in the destination";
pub(crate) const JR_WRONG_PARAM: &'static str = "JR: error in the destination";
pub(crate) const CALL_WRONG_PARAM: &'static str = "CALL: error in the destination";

pub(crate) const SNASET_WRONG_LABEL: &'static str = "SNASET: error in the option naming";
pub(crate) const SNASET_MISSING_COMMA: &'static str = "SNASET: missing comma";

impl Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use codespan_reporting::files::SimpleFiles;
        use codespan_reporting::term;
        use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, Buffer};
        use nom::error::ErrorKind;
        use nom::error::VerboseErrorKind;
        use std::ops::Deref;
        use std::ops::DerefMut;
        use std::rc::Rc;

        match self {
            Self::SyntaxError { error } => {
                let mut source_files = SimpleFiles::new();
                let mut fname_to_id = std::collections::BTreeMap::new();

                let str = error
                    .errors
                    .iter()
                    .filter(|e| match e.1 {
                        VerboseErrorKind::Context(ctx) => !ctx.starts_with("[DBG]"),
                        VerboseErrorKind::Nom(ErrorKind::Eof) => true,
                        _ => false,
                    })
                    .map(|e| match e.1 {
                        VerboseErrorKind::Context(_) | VerboseErrorKind::Nom(ErrorKind::Eof) => {
                            // Get the real are build the context
                            let ctx = match e.1 {
                                VerboseErrorKind::Context(ctx) => ctx,
                                VerboseErrorKind::Nom(ErrorKind::Eof) => "Unknown error",
                                _ => unreachable!(),
                            };

                            let span = &e.0;

                            // Add filename to database if needed
                            let filename = Box::new(
                                e.0.extra
                                    .1
                                    .current_filename
                                    .as_ref()
                                    .map(|p| p.as_os_str().to_str().unwrap().to_owned())
                                    .unwrap_or_else(|| "no file".to_owned()),
                            );
                            let source = e.0.extra.0.as_ref();
                            let file_id = match fname_to_id.get(filename.deref()) {
                                Some(&id) => id,
                                None => {
                                    let id = source_files.add(filename.deref().to_owned(), source);
                                    fname_to_id.insert(filename.deref().to_owned(), id);
                                    id
                                }
                            };

                            let sample_range = std::ops::Range {
                                start: span.location_offset(),
                                end: guess_error_end(
                                    source_files.get(file_id).unwrap().source(),
                                    span.location_offset(),
                                    ctx,
                                ),
                            };
                            let mut diagnostic = Diagnostic::error()
                                .with_message("Syntax error")
                                .with_labels(vec![Label::new(
                                    codespan_reporting::diagnostic::LabelStyle::Primary,
                                    file_id,
                                    sample_range,
                                )
                                .with_message(ctx)]);

                            if let Some(notes) = get_additional_notes(ctx) {
                                diagnostic = diagnostic.with_notes(notes);
                            }

                            let mut writer = Buffer::ansi();
                            let config = codespan_reporting::term::Config::default();
                            term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();
                            
                            std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
                        }

                        _ => unreachable!(),
                    })
                    .join("\n");
                write!(f, "{}", str)
            }

            AssemblerError::IncludedFileError { span, error } => {
 

                let filename = Box::new(
                    span.extra
                        .1
                        .current_filename
                        .as_ref()
                        .map(|p| p.as_os_str().to_str().unwrap().to_owned())
                        .unwrap_or_else(|| "no file".to_owned()),
                );
                let source = span.extra.0.as_ref();

                let mut source_files = SimpleFiles::new();
                let file = source_files.add(filename, source);

                let sample_range = std::ops::Range {
                    start: span.location_offset(),
                    end: guess_error_end(
                        source_files.get(file).unwrap().source(),
                        span.location_offset(),
                        JP_WRONG_PARAM, // fake value
                    ),
                };

                let mut diagnostic = Diagnostic::error()
                    .with_message("Error in imported file")
                    .with_labels(vec![Label::new(
                        codespan_reporting::diagnostic::LabelStyle::Primary,
                        file,
                        sample_range,
                    )]);

                let mut writer = Buffer::ansi();
                let config = codespan_reporting::term::Config::default();
                term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();
                
                write!(f, "{}", std::str::from_utf8(writer.as_slice()).unwrap())?;
                error.fmt(f)
            }

            _ => unimplemented!("{:?}", self),
        }
    }
}

/// The parser is unable to provide the end of the error.
/// This function tries to get it
fn guess_error_end(code: &str, offset: usize, ctx: &str) -> usize {
    enum EndKind {
        CommaOrEnd,
        End,
    }

    impl EndKind {
        fn guess(&self, code: &str, mut offset: usize) -> usize {
            match self {
                EndKind::End => {
                    for current in code[offset..].chars() {
                        if current == ':'
                            || current == '\n'
                            || current == ':'
                            || offset == code.len() - 1
                        {
                            break;
                        }
                        offset += 1;
                    }
                    offset
                }

                EndKind::CommaOrEnd => {
                    for current in code[offset..].chars() {
                        if current == ','
                            || current == ':'
                            || current == '\n'
                            || current == ':'
                            || offset == code.len() - 1
                        {
                            break;
                        }
                        offset += 1;
                    }
                    offset
                }
                _ => unimplemented!(),
            }
        }
    }
    lazy_static::lazy_static! {
        static ref GUESSER_LUT: HashMap<&'static str, EndKind> = {
            let mut hash = HashMap::new();

            hash.insert(LD_WRONG_DESTINATION, EndKind::CommaOrEnd);

            hash
        };
    }

    let guesser = GUESSER_LUT.get(ctx).unwrap_or(&EndKind::End);

    let mut end = guesser.guess(code, offset);
    // remove whitespace from selection
    for previous in code[offset..end].chars().rev() {
        previous;
        if previous.is_whitespace() {
            end -= 1;
        } else {
            break;
        }
    }
    end
}

fn get_additional_notes(ctx: &str) -> Option<Vec<String>> {
    // phf is not currently usable
    lazy_static::lazy_static! {
        static ref NOTES_LUT: HashMap<&'static str, Vec<String>> = {
            let mut hash = HashMap::new();

            hash.insert(LD_WRONG_DESTINATION, vec![
                "Possible destinations are:".to_owned(),
                " - 16 bits registers: AF, HL, BC, DE, IX, IY".to_owned(),
                " - 8 bits registers: A, B, C, D, E, H, L, IXH, IXL, IYH, IYL, I".to_owned(),
                " - addresses: (address), (hl), (de), (bc), (IX+delta), (IY+delta)".to_owned()
            ]);

            hash
        };
    }

    NOTES_LUT.get(ctx).cloned()
}

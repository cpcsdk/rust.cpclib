use std::collections::HashMap;
use std::fmt::Display;

use crate::PhysicalAddress;
use crate::assembler::AssemblingPass;
use crate::parser::ParserContext;
use crate::Z80Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use cpclib_basic::BasicError;
use cpclib_disc::amsdos::AmsdosError;
use cpclib_tokens::symbols::Symbol;
use cpclib_tokens::symbols::SymbolError;
use cpclib_tokens::tokens;
use itertools::Itertools;
use nom::error::VerboseError;
use nom::error::ErrorKind;
use nom::error::VerboseErrorKind;
use std::ops::Deref;

use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, Buffer};

#[derive(Debug, Clone)]
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
        error: BasicError,
    },

    DisassemblerError {
        msg: String
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

    CounterAlreadyExists {
        symbol: String,
    },

    IncoherentCode {
        msg: String
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

    InvalidSymbol(String),
    
    //   #[fail(display = "Symbol {} is not a {}", symbol, isnot)]
    WrongSymbolType {
        symbol: String,
        isnot: String,
    },

    // TODO add symbol type
    AlreadyDefinedSymbol {
        symbol: String,
        kind: String
    },

    //   #[fail(display = "IO error: {}", msg)]
    IOError {
        msg: String,
    },

    //  #[fail(display = "Current assembling address is unknown.")]
    UnknownAssemblingAddress,
    ReadOnlySymbol(Symbol),
    RunAlreadySpecified,
    NoActiveCounter,
    NoDataToCrunch,

    OutputExceedsLimits(usize),
    OutputProtected{
        area: std::ops::RangeInclusive<u16>,
        address: u16
    },
    OverrideMemory(PhysicalAddress, usize),

    //  #[fail(display = "Unable to resolve expression {}.", expression)]
    ExpressionUnresolvable {
        expression: tokens::Expr,
    },

    ExpressionError {
        msg: String
    },

    RelativeAddressUncomputable {
        address: i32,
        pass: AssemblingPass,
        error: Box<AssemblerError>,
    },

    /// Several errors has been generated without span information.
    /// RelocatedError allows them to be approximately located
    RelocatedError {
       error: Box<AssemblerError>,
       span: Z80Span
    },
    RelocatedWarning {
        error: Box<AssemblerError>,
        span: Z80Span
     },

    RepeatIssue {
        error: Box<AssemblerError>,
        span: Option<Z80Span>,
        repetition: i32
    },

    MMRError {
        value: i32
    }

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


impl From<BasicError> for AssemblerError {
    fn from(msg: BasicError) -> Self {
        AssemblerError::BasicError {
            error: msg,
        }
    }
}


impl From<SymbolError> for AssemblerError {
    fn from(err: SymbolError) -> Self {
        match err {
            SymbolError::UnknownAssemblingAddress => AssemblerError::UnknownAssemblingAddress,
            SymbolError::CannotModify(symb) => AssemblerError::ReadOnlySymbol(symb),
            SymbolError::WrongSymbol(err) => AssemblerError::InvalidSymbol(err)
        }
    }
}

impl From<AmsdosError> for AssemblerError {
    fn from(err: AmsdosError) -> Self {
        AssemblerError::AmsdosError { error: err }
    }
}


impl AssemblerError {
    /// Returns true only for errors already located
    pub fn is_located(&self) -> bool {
        match self {
            AssemblerError::RelocatedError{..} => true,
            AssemblerError::RelocatedWarning{..} => true,
            _ => false
        }
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

        match self {
            AssemblerError::SyntaxError { error } => {
                let mut source_files = SimpleFiles::new();
                let mut fname_to_id = std::collections::BTreeMap::new();

                let str = error
                    .errors
                    .iter()
                    .filter(|e| match e.1 {
                        VerboseErrorKind::Context(ctx) => !ctx.starts_with("[DBG]"),
                      //  VerboseErrorKind::Nom(ErrorKind::Eof) => true,
                        _ => true,
                    })
                    .map(|e| match e.1 {
                        VerboseErrorKind::Context(_) | 
                        VerboseErrorKind::Nom(_) |
                        VerboseErrorKind::Char(_)=> {
                            // Get the real are build the context
                            let ctx: std::borrow::Cow<str> = match e.1 {
                                VerboseErrorKind::Context(ctx) => ctx.into(),
                                VerboseErrorKind::Nom(_) => "Unknown error".into(),
                                VerboseErrorKind::Char(c) => format!("Error with char '{}'", c).into(), 
                                _ => unreachable!(),
                            };
                            let ctx = ctx.deref();

                            let span = &e.0;

                            // Add filename to database if needed
                            let filename = 
                                e.0.extra
                                    .1
                                    .current_filename
                                    .as_ref()
                                    .map(|p| p.as_os_str().to_str().unwrap().to_owned());
                            
                            let filename = filename.unwrap_or_else(||{
                                e.0.extra.1.context_name.as_ref().cloned().unwrap_or_else(|| {
                                     "no file".to_owned()
                                })
                            });
                            let filename = Box::new(filename);
                            
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

                        _ => unreachable!("{:?}", e.1),
                    })
                    .join("\n");
                write!(f, "{}", str)
            }

            AssemblerError::IncludedFileError { span, error } => {
                match error.as_ref() {
                    AssemblerError::IOError{msg} => {
                        let msg =  build_simple_error_message_with_message(
                            "Error for imported file", 
                            msg,
                            span);
                        write!(f, "{}",msg)
                    }
                    _ => {
                        let msg =  build_simple_error_message("Error in imported file", span, true);
                        write!(f, "{}",msg)?;
                        error.fmt(f)
                    }
                }

            }

            AssemblerError::OverrideMemory(address, count) => {
                write!(f, "Override {} bytes at 0x{:x} (0x{:x} in page {})", *count, address.address, address.offset_in_page(), address.page)
            }
            AssemblerError::DisassemblerError{msg} => write!(f, "Disassembler error: {}", msg),
            AssemblerError::ExpressionError{msg} => write!(f, "Expression error: {}", msg),
            AssemblerError::CounterAlreadyExists{symbol} => write!(f, "A counter named `{}` already exists", symbol),
            AssemblerError::SymbolAlreadyExists{symbol} => write!(f, "A symbol named `{}` already exists", symbol),
            AssemblerError::IncoherentCode{msg} => write!(f, "Incoherent code: {}", msg),
            AssemblerError::NoActiveCounter => write!(f, "No active counter"),
            AssemblerError::OutputExceedsLimits(limit) => write!(f, "Output exceeds limits of 0x{:X}", limit),
            AssemblerError::RunAlreadySpecified => write!(f, "RUN has already been specified"),
            AssemblerError::AlreadyDefinedSymbol{symbol, kind} => write!(f, "Symbol \"{}\" already defined as a {}", symbol, kind),

            AssemblerError::MultipleErrors { errors } => {
                for e in errors.iter() {
                    writeln!(f, "{}", e)?;
                }
                Ok(())
            },

            AssemblerError::UnknownSymbol { symbol, closest } => write!(f, "Unknown symbol: {}. Closest one is: {:?}", symbol, closest),

            AssemblerError::EmptyBinaryFile(_) => todo!(),
            AssemblerError::AmsdosError { error } => {
                todo!()
            },
            AssemblerError::BugInAssembler { msg } => write!(f, "BUG in assembler: {}", msg),
            AssemblerError::BugInParser { error, context } => todo!(),

            AssemblerError::BasicError { error } => todo!(),
            AssemblerError::AssemblingError { msg } => todo!(),
            AssemblerError::InvalidArgument { msg } => write!(f, "Invalid argument: {}", msg),
            AssemblerError::AssertionFailed { test, msg, guidance } => write!(f, "Assert error: {} {} {}", test, msg, guidance),

            AssemblerError::UnknownMacro { symbol, closest } => {
                write!(f, "MACRO {} does not exist. Try {}" , symbol, closest.as_ref().unwrap_or(&"".to_owned()))
            },

            AssemblerError::WrongNumberOfParameters { symbol, nb_paramers, nb_arguments } => todo!(),
            AssemblerError::MacroError { name, root }  => {
                write!(f, "Error in macro call: {}\n{}", name, root)
            },
            AssemblerError::WrongSymbolType { symbol, isnot } => todo!(),
            AssemblerError::IOError { msg } => {
                write!(f, "IO Error: {}", msg)
            },
            AssemblerError::UnknownAssemblingAddress => todo!(),
            AssemblerError::ExpressionUnresolvable { expression } => todo!(),
            AssemblerError::ExpressionError { msg } => todo!(),
            AssemblerError::RelativeAddressUncomputable { address, pass, error } => todo!(),

            // By construction contains only error with no span information
            AssemblerError::RelocatedError { error, span } => {

                // Relocated error format may vary among errors
                match error.deref() {
                    AssemblerError::UnknownSymbol { symbol, closest } => {
                        let msg =  match closest {
                            Some(closest) => {
                                build_simple_error_message_with_notes(
                                    &format!("Unknown symbol: {}", symbol),
                                    vec![format!("Closest one is: {}", closest)],
                                    span
                                )
                            },
                            None => {
                                  build_simple_error_message(
                                &format!("Unknown symbol: {}", symbol),
                                span,
                                true
                              )
                            }
                        };
        
                        write!(f, "{}",msg)
                    },

                    AssemblerError::UnknownMacro { symbol, closest } => {
                        let msg =  match closest {
                            Some(closest) => {
                                build_simple_error_message_with_notes(
                                    &format!("Unknown macro: {}", symbol),
                                    vec![format!("Closest one is: {}", closest)],
                                    span
                                )
                            },
                            None => {
                                build_simple_error_message(
                                &format!("Unknown macro: {}", symbol),
                                span,
                                true
                            )
                            }
                        };
        
                        write!(f, "{}",msg)
                    },


                    AssemblerError::MacroError { name, root } => {
                        let msg =  build_simple_error_message(&format!("Error in macro call: {}", name), span, true);
                        write!(f, "{}\n{}",msg,root)
                    },

                    AssemblerError::OutputProtected { area, address } => {
                        let msg = build_simple_error_message_with_message(
                            "Forbidden output", 
                            &format!("Tentative to write in 0x{:X} in a protected area [0x{:X}:0x{:X}]",
                            address, area.start(), area.end()), 
                            span);
                            write!(f, "{}",msg)
                    }

                    _ => {
                        let msg =  build_simple_error_message(&format!("{}", error), span, true);
                        write!(f, "{}",msg)
                    }
                }

            },
            AssemblerError::ReadOnlySymbol(symb) => write!(f, "{} cannot be modified", symb.value()),

            AssemblerError::RepeatIssue { error, span, repetition } => {
                if span.is_some(){
                    let msg =  build_simple_error_message(&format!("REPEAT: error in loop {}", repetition), span.as_ref().unwrap(), true);
                    write!(f, "{}\n{}",msg, error)
                } else {
                    write!(f, "Repeat issue\n{}", error)
                }
            
            },

            AssemblerError::OutputProtected { area, address } => {
                write!(
                    f,
                    "Tentative to write in 0x{:X} in a protected area [0x{:X}:0x{:X}]",
                    address, area.start(), area.end()
                )
            },
            AssemblerError::InvalidSymbol(msg) => {
                write!(f, "Invalid symbol {}", msg)
            },
            AssemblerError::NoDataToCrunch => {
                write!(f, "There is no bytes to crunch")
            }
            AssemblerError::MMRError { value } => {
                write!(f, "{} is invalid. We expect values from 0xC0 to 0xc7.", value)            
            }
            AssemblerError::RelocatedWarning { error, span } => {
                let msg =  build_simple_error_message(&format!("{}", error), span, false);
                write!(f, "{}",msg)
            },
           
        }
    }
}


fn build_simple_error_message_with_message(title: &str,message: &str,  span: &Z80Span) -> String {
    let filename = build_filename(span);
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
        .with_message(title)
        .with_labels(vec![Label::new(
            codespan_reporting::diagnostic::LabelStyle::Primary,
            file,
            sample_range,
        ).with_message(message)]);

    let mut writer = Buffer::ansi();
    let config = codespan_reporting::term::Config::default();
    term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
}


fn build_simple_error_message(title: &str, span: &Z80Span, is_error: bool) -> String {
    let filename = build_filename(span);
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

    let mut diagnostic = if is_error {
        Diagnostic::error()
    } else {
        Diagnostic::warning()
    };
    diagnostic = diagnostic
        .with_message(title)
        .with_labels(vec![Label::new(
            codespan_reporting::diagnostic::LabelStyle::Primary,
            file,
            sample_range,
        )]);

    let mut writer = Buffer::ansi();
    let config = codespan_reporting::term::Config::default();
    term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
}


fn build_filename(span: &Z80Span) -> Box<String> {
    let fname = &span.extra.1.current_filename;
    let context = &span.extra.1.context_name;

    let name = fname.as_ref()
        .map(|p|{
            p.as_os_str().to_str().unwrap()
        }).unwrap_or_else(||{
            context.as_ref().map(|s| s.as_ref())
            .unwrap_or_else(||{
                "no file specified"
            })
    });

    Box::new(name.to_owned())
}

fn build_simple_error_message_with_notes(title: &str, notes: Vec<String>, span: &Z80Span) -> String {
    let filename = build_filename(span);
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
        .with_message(title)
        .with_labels(vec![Label::new(
            codespan_reporting::diagnostic::LabelStyle::Primary,
            file,
            sample_range,
        )])
        .with_notes(notes);

    let mut writer = Buffer::ansi();
    let config = codespan_reporting::term::Config::default();
    term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
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
                            || current == ';'
                            || offset == code.len()
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
                            || current == ';'
                            || offset == code.len()
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

use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;

use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::Buffer;
use codespan_reporting::term::{self, Chars, Config, DisplayStyle};
use cpclib_basic::BasicError;
use cpclib_common::itertools::Itertools;
use cpclib_common::nom::error::{VerboseError, VerboseErrorKind};
use cpclib_common::smol_str::SmolStr;
use cpclib_disc::amsdos::AmsdosError;
use cpclib_sna::SnapshotError;
use cpclib_tokens::symbols::{Symbol, SymbolError};
use cpclib_tokens::{tokens, ExpressionTypeError, Oper};

use crate::assembler::AssemblingPass;
use crate::parser::ParserContext;
use crate::{PhysicalAddress, Z80Span};

#[derive(Debug, Clone)]
pub enum ExpressionError {
    LeftError(Oper, Box<AssemblerError>),
    RightError(Oper, Box<AssemblerError>),
    LeftAndRightError(Oper, Box<AssemblerError>, Box<AssemblerError>),
    OwnError(Box<AssemblerError>),
    InvalidSize(usize, usize) // expected index
}

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum AssemblerError {
    //#[fail(display = "Several errors arised: {:?}", errors)]
    MultipleErrors {
        errors: Vec<AssemblerError>
    },

    //#[fail(display = "{} cannot be empty.", 0)]
    EmptyBinaryFile(String),

    //#[fail(display = "Amsdos error: {}", error)]
    AmsdosError {
        error: AmsdosError
    },

    //#[fail(display = "Assembling bug: {}", msg)]
    BugInAssembler {
        msg: String
    },

    //#[fail(display = "Parser bug: {}. Context: {:?}", error, context)]
    BugInParser {
        error: String,
        context: ParserContext
    },

    // TODO add more information
    //#[fail(display = "Syntax error:\n{}", error)]
    SyntaxError {
        error: VerboseError<Z80Span>
    },

    IncludedFileError {
        span: Z80Span,
        error: Box<AssemblerError>
    },

    //#[fail(display = "Basic error: {}", error)]
    BasicError {
        error: BasicError
    },

    DisassemblerError {
        msg: String
    },

    // TODO add more information
    // #[fail(display = "Assembling error: {}", msg)]
    AssemblingError {
        msg: String
    },

    // #[fail(display = "Invalid argument: {}", msg)]
    InvalidArgument {
        msg: String
    },

    Fail {
        msg: String
    },

    //  #[fail(display = "Assertion failed -- {} [{}]: {}", test, guidance, msg)]
    AssertionFailed {
        test: String,
        msg: String,
        guidance: String
    },

    //  #[fail(display = "Symbol `{}` already present on the symbol table", symbol)]
    SymbolAlreadyExists {
        symbol: String
    },

    CounterAlreadyExists {
        symbol: String
    },

    IncoherentCode {
        msg: String
    },

    //    #[fail(
    //        display = "There is no macro named `{}`. Closest one is: {:?}",
    //        symbol, closest
    //    )]
    UnknownMacro {
        symbol: SmolStr,
        closest: Option<SmolStr>
    },

    //    #[fail(display = "Error when applying macro {}. {}", name, root)]
    MacroError {
        name: SmolStr,
        root: Box<AssemblerError>
    },

    //   #[fail(
    //       display = "Macro `{}` expect {} arguments; {} are provided.",
    //       symbol, nb_arguments, nb_paramers
    //   )]
    WrongNumberOfParameters {
        symbol: String,
        nb_paramers: usize,
        nb_arguments: usize
    },

    //  #[fail(display = "Unknown symbol: {}. Closest one is: {:?}", symbol, closest)]
    UnknownSymbol {
        symbol: SmolStr,
        closest: Option<SmolStr>
    },

    InvalidSymbol(SmolStr),

    //   #[fail(display = "Symbol {} is not a {}", symbol, isnot)]
    WrongSymbolType {
        symbol: SmolStr,
        isnot: SmolStr
    },

    // TODO add symbol type
    AlreadyDefinedSymbol {
        symbol: SmolStr,
        kind: SmolStr
    },

    //   #[fail(display = "IO error: {}", msg)]
    IOError {
        msg: String
    },

    //  #[fail(display = "Current assembling address is unknown.")]
    UnknownAssemblingAddress,
    ReadOnlySymbol(Symbol),
    RunAlreadySpecified,
    NoActiveCounter,
    NoDataToCrunch,
    NotAllowed,

    OutputExceedsLimits(PhysicalAddress, usize),
    OutputAlreadyExceedsLimits(usize),
    OutputProtected {
        area: std::ops::RangeInclusive<u16>,
        address: u16
    },
    OverrideMemory(PhysicalAddress, usize),

    //  #[fail(display = "Unable to resolve expression {}.", expression)]
    ExpressionUnresolvable {
        expression: tokens::Expr
    },

    ExpressionError(ExpressionError),

    RelativeAddressUncomputable {
        address: i32,
        pass: AssemblingPass,
        error: Box<AssemblerError>
    },

    CrunchedSectionError {
        error: Box<AssemblerError>
    },

    /// Several errors has been generated without span information.
    /// RelocatedError allows them to be approximately located
    RelocatedError {
        error: Box<AssemblerError>,
        span: Z80Span
    },
    RelocatedWarning {
        warning: Box<AssemblerError>,
        span: Z80Span
    },
    RelocatedInfo {
        info: Box<AssemblerError>,
        span: Z80Span
    },

    RepeatIssue {
        error: Box<AssemblerError>,
        span: Option<Z80Span>,
        repetition: i32
    },

    WhileIssue {
        error: Box<AssemblerError>,
        span: Option<Z80Span>
    },

    MMRError {
        value: i32
    },

    SnapshotError {
        error: SnapshotError
    },

    FunctionWithoutReturn(String),
    FunctionWithEmptyBody(String),
    FunctionUnknown(String),
    FunctionWithWrongNumberOfArguments(String, usize, usize),
    FunctionError(String, Box<AssemblerError>),

    ExpressionTypeError(ExpressionTypeError)
}

impl From<ExpressionTypeError> for AssemblerError {
    fn from(e: ExpressionTypeError) -> Self {
        Self::ExpressionTypeError(e)
    }
}

impl From<&ExpressionTypeError> for AssemblerError {
    fn from(e: &ExpressionTypeError) -> Self {
        Self::ExpressionTypeError(e.clone())
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
            msg: err.to_string()
        }
    }
}

impl From<BasicError> for AssemblerError {
    fn from(msg: BasicError) -> Self {
        AssemblerError::BasicError { error: msg }
    }
}

impl From<SnapshotError> for AssemblerError {
    fn from(msg: SnapshotError) -> Self {
        AssemblerError::SnapshotError { error: msg }
    }
}

impl From<SymbolError> for AssemblerError {
    fn from(err: SymbolError) -> Self {
        match err {
            SymbolError::UnknownAssemblingAddress => AssemblerError::UnknownAssemblingAddress,
            SymbolError::CannotModify(symb) => AssemblerError::ReadOnlySymbol(symb),
            SymbolError::WrongSymbol(err) => AssemblerError::InvalidSymbol(err.value().into()),
            SymbolError::NoNamespaceActive => {
                AssemblerError::AssemblingError {
                    msg: "There is no namespace active".to_owned()
                }
            }
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
            AssemblerError::RelocatedError{..} |
            AssemblerError::RelocatedWarning{..} /*|
            AssemblerError::SyntaxError{..} */ => true, // we need to exclude syntax error to add the location from the assembler when using macros
            _ => false
        }
    }

    pub fn is_override_memory(&self) -> bool {
        match self {
            AssemblerError::OverrideMemory(..) => true,
            AssemblerError::RelocatedError { error, .. }
            | AssemblerError::RelocatedWarning { warning: error, .. } => error.is_override_memory(),
            _ => false
        }
    }

    pub fn locate(self, span: Z80Span) -> Self {
        if self.is_located() {
            self
        }
        else {
            AssemblerError::RelocatedError {
                span: span,
                error: Box::new(self)
            }
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
        self.format(f, true)
    }
}

impl AssemblerError {
    pub fn format(&self, f: &mut std::fmt::Formatter<'_>, complete: bool) -> std::fmt::Result {
        match self {
            AssemblerError::SyntaxError { error } => {
                let mut source_files = SimpleFiles::new();
                let mut fname_to_id = std::collections::BTreeMap::new();

                let str = error
                    .errors
                    .iter()
                    .filter(|e| {
                        match e.1 {
                            VerboseErrorKind::Context(ctx) => !ctx.starts_with("[DBG]"),
                            //  VerboseErrorKind::Nom(ErrorKind::Eof) => true,
                            _ => true
                        }
                    })
                    .map(|e| {
                        match e.1 {
                            VerboseErrorKind::Context(_)
                            | VerboseErrorKind::Nom(_)
                            | VerboseErrorKind::Char(_) => {
                                // Get the real are build the context
                                let ctx: std::borrow::Cow<str> = match e.1 {
                                    VerboseErrorKind::Context(ctx) => ctx.into(),
                                    VerboseErrorKind::Nom(_) => "Unknown error".into(),
                                    VerboseErrorKind::Char(c) => {
                                        format!("Error with char '{}'", c).into()
                                    }
                                    _ => unreachable!()
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

                                let filename = filename.unwrap_or_else(|| {
                                    e.0.extra
                                        .1
                                        .context_name
                                        .as_ref()
                                        .cloned()
                                        .unwrap_or_else(|| "no file".to_owned())
                                });
                                let filename = Box::new(filename);

                                let source = e.0.extra.0.as_ref();
                                let file_id = match fname_to_id.get(filename.deref()) {
                                    Some(&id) => id,
                                    None => {
                                        let id =
                                            source_files.add(filename.deref().to_owned(), source);
                                        fname_to_id.insert(filename.deref().to_owned(), id);
                                        id
                                    }
                                };

                                let sample_range = std::ops::Range {
                                    start: span.location_offset(),
                                    end: guess_error_end(
                                        source_files.get(file_id).unwrap().source(),
                                        span.location_offset(),
                                        ctx
                                    )
                                };
                                let mut diagnostic = Diagnostic::error()
                                    .with_message("Syntax error")
                                    .with_labels(vec![Label::new(
                                        codespan_reporting::diagnostic::LabelStyle::Primary,
                                        file_id,
                                        sample_range
                                    )
                                    .with_message(ctx)]);

                                if let Some(notes) = get_additional_notes(ctx) {
                                    diagnostic = diagnostic.with_notes(notes);
                                }

                                let mut writer = buffer();
                                let config = config();
                                term::emit(&mut writer, &config, &source_files, &diagnostic)
                                    .unwrap();

                                std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
                            }

                            _ => unreachable!("{:?}", e.1)
                        }
                    })
                    .join("\n");
                write!(f, "{}", str)
            }

            AssemblerError::IncludedFileError { span, error } => {
                match error.as_ref() {
                    AssemblerError::IOError { msg } => {
                        let msg = build_simple_error_message_with_message(
                            "Error for imported file",
                            msg,
                            span
                        );
                        write!(f, "{}", msg)
                    }
                    _ => {
                        let msg = build_simple_error_message(
                            "Error in imported file",
                            span,
                            Severity::Error
                        );
                        write!(f, "{}", msg)?;
                        error.fmt(f)
                    }
                }
            }

            AssemblerError::OverrideMemory(address, count) => {
                write!(
                    f,
                    "Override {} bytes at 0x{:X} (0x{:X} in page {})",
                    *count,
                    address.address(),
                    address.offset_in_page(),
                    address.page()
                )
            }
            AssemblerError::DisassemblerError { msg } => write!(f, "Disassembler error: {}", msg),

            AssemblerError::ExpressionError(e) => {
                let msg = match e {
                    ExpressionError::LeftError(oper, error) => {
                        format!("on left operand of {}: {}.", oper, error)
                    }
                    ExpressionError::RightError(oper, error) => {
                        format!("on right operand of {}: {}.", oper, error)
                    }
                    ExpressionError::LeftAndRightError(oper, error1, error2) => {
                        format!(
                            "on left and right operand of {}: {} / {}",
                            oper, error1, error2
                        )
                    }
                    ExpressionError::OwnError(error) => {
                        format!("{}", error)
                    }
                    ExpressionError::InvalidSize(expected, index) => {
                        format!("{} index incompatible with size {}", index, index)
                    }
                };
                write!(f, "Expression error {}", msg)
            }
            AssemblerError::CounterAlreadyExists { symbol } => {
                write!(f, "A counter named `{}` already exists", symbol)
            }
            AssemblerError::SymbolAlreadyExists { symbol } => {
                write!(f, "A symbol named `{}` already exists", symbol)
            }
            AssemblerError::IncoherentCode { msg } => write!(f, "Incoherent code: {}", msg),
            AssemblerError::NoActiveCounter => write!(f, "No active counter"),
            AssemblerError::OutputExceedsLimits(address, limit) => {
                write!(
                    f,
                    "Code  at 0x{:X} (0x{:X} in page {}) exceeds limits of 0x{:X}",
                    address.address(),
                    address.offset_in_page(),
                    address.page(),
                    limit
                )
            }
            AssemblerError::OutputAlreadyExceedsLimits(limit) => {
                write!(f, "Code  already exceeds limits of 0x{:X}", limit)
            }
            AssemblerError::RunAlreadySpecified => write!(f, "RUN has already been specified"),
            AssemblerError::AlreadyDefinedSymbol { symbol, kind } => {
                write!(f, "Symbol \"{}\" already defined as a {}", symbol, kind)
            }

            AssemblerError::MultipleErrors { errors } => {
                for e in errors.iter() {
                    writeln!(f, "{}", e)?;
                }
                Ok(())
            }

            AssemblerError::UnknownSymbol { symbol, closest } => {
                write!(
                    f,
                    "Unknown symbol: {}. Closest one is: {:?}",
                    symbol, closest
                )
            }

            AssemblerError::ExpressionTypeError(e) => write!(f, "{}", e),

            AssemblerError::EmptyBinaryFile(_) => todo!(),
            AssemblerError::AmsdosError { error: _ } => {
                todo!()
            }
            AssemblerError::BugInAssembler { msg } => write!(f, "BUG in assembler: {}", msg),
            AssemblerError::BugInParser {
                error: _,
                context: _
            } => todo!(),

            AssemblerError::BasicError { error } => write!(f, "{}", error.to_string()),
            AssemblerError::AssemblingError { msg } => write!(f, "{}", msg),
            AssemblerError::InvalidArgument { msg } => write!(f, "Invalid argument: {}", msg),
            AssemblerError::AssertionFailed {
                test,
                msg,
                guidance
            } => {
                write!(f, "Assert error: {} {} {}", test, msg, guidance)
            }

            AssemblerError::UnknownMacro { symbol, closest } => {
                write!(
                    f,
                    "MACRO {} does not exist. Try {}",
                    symbol,
                    closest.as_ref().unwrap_or(&SmolStr::new_inline(""))
                )
            }
            AssemblerError::FunctionWithoutReturn(name) => {
                write!(f, "Function {} has no RETURN directive", name)
            }
            AssemblerError::FunctionWithEmptyBody(name) => {
                write!(f, "Function {} has no body", name)
            }
            AssemblerError::FunctionUnknown(name) => {
                write!(f, "Function {} unknown", name)
            }
            AssemblerError::FunctionError(name, e) => {
                write!(f, "Function {} error: {}", name, e)
            }
            AssemblerError::FunctionWithWrongNumberOfArguments(name, expected, received) => {
                write!(
                    f,
                    "Function {} called with {} parameters instead of {}",
                    name, received, expected
                )
            }
            AssemblerError::WrongNumberOfParameters {
                symbol: _,
                nb_paramers: _,
                nb_arguments: _
            } => todo!(),
            AssemblerError::MacroError { name, root } => {
                write!(f, "Error in macro call: {}\n{}", name, root)
            }
            AssemblerError::WrongSymbolType {
                symbol: s,
                isnot: n
            } => {
                write!(f, "Wrong symbol type: {} is not {}", s, n)
            }
            AssemblerError::IOError { msg } => {
                write!(f, "IO Error: {}", msg)
            }
            AssemblerError::UnknownAssemblingAddress => todo!(),
            AssemblerError::ExpressionUnresolvable { expression: _ } => todo!(),
            AssemblerError::RelativeAddressUncomputable {
                address: _,
                pass: _,
                error
            } => {
                write!(f, "Unable to compute relative address {}", error)
            }

            // By construction contains only error with no span information
            AssemblerError::RelocatedError { error, span } => {
                if complete {
                    // Relocated error format may vary among errors
                    match error.deref() {
                        AssemblerError::RelocatedError { error, span: _ } => {
                            write!(f, "{}", error)
                        }

                        AssemblerError::UnknownSymbol { symbol, closest } => {
                            let msg = match closest {
                                Some(closest) => {
                                    build_simple_error_message_with_notes(
                                        &format!("Unknown symbol: {}", symbol),
                                        vec![format!("Closest one is: {}", closest)],
                                        span
                                    )
                                }
                                None => {
                                    build_simple_error_message(
                                        &format!("Unknown symbol: {}", symbol),
                                        span,
                                        Severity::Error
                                    )
                                }
                            };

                            write!(f, "{}", msg)
                        }

                        AssemblerError::UnknownMacro { symbol, closest } => {
                            let msg = match closest {
                                Some(closest) => {
                                    build_simple_error_message_with_notes(
                                        &format!("Unknown macro: {}", symbol),
                                        vec![format!("Closest one is: {}", closest)],
                                        span
                                    )
                                }
                                None => {
                                    build_simple_error_message(
                                        &format!("Unknown macro: {}", symbol),
                                        span,
                                        Severity::Error
                                    )
                                }
                            };

                            write!(f, "{}", msg)
                        }

                        AssemblerError::MacroError { name, root } => {
                            let msg = build_simple_error_message(
                                &format!("Error in macro call: {}", name),
                                span,
                                Severity::Error
                            );
                            write!(f, "{}\n{}", msg, root)
                        }

                        AssemblerError::BasicError { error } => {
                            let msg =
                                build_simple_error_message("BASIC error", span, Severity::Error);
                            write!(f, "{}\n{}", msg, error)
                        }

                        AssemblerError::OutputProtected { area, address } => {
                            let msg = build_simple_error_message_with_message(
                                "Forbidden output",
                                &format!(
                                "Tentative to write in 0x{:X} in a protected area [0x{:X}:0x{:X}]",
                                address,
                                area.start(),
                                area.end()
                            ),
                                span
                            );
                            write!(f, "{}", msg)
                        }

                        AssemblerError::CrunchedSectionError { error } => {
                            let msg = build_simple_error_message(
                                "Impossible to crunch section",
                                span,
                                Severity::Error
                            );
                            write!(f, "{}", msg)?;
                            write!(f, "{}", error)
                        }

                        _ => {
                            let msg = build_simple_error_message(
                                &format!("{}", error),
                                span,
                                Severity::Error
                            );
                            write!(f, "{}", msg)
                        }
                    }
                }
                else {
                    write!(f, "{}", error)
                }
            }
            AssemblerError::ReadOnlySymbol(symb) => {
                write!(f, "{} cannot be modified", symb.value())
            }

            AssemblerError::RepeatIssue {
                error,
                span,
                repetition
            } => {
                if span.is_some() {
                    let msg = build_simple_error_message(
                        &format!("REPEAT: error in loop {}", repetition),
                        span.as_ref().unwrap(),
                        Severity::Error
                    );
                    write!(f, "{}\n{}", msg, error)
                }
                else {
                    write!(f, "Repeat issue\n{}", error)
                }
            }

            AssemblerError::WhileIssue { error, span } => {
                if span.is_some() {
                    let msg = build_simple_error_message(
                        &format!("WHILE: error in loop"),
                        span.as_ref().unwrap(),
                        Severity::Error
                    );
                    write!(f, "{}\n{}", msg, error)
                }
                else {
                    write!(f, "WHILE issue\n{}", error)
                }
            }

            AssemblerError::OutputProtected { area, address } => {
                write!(
                    f,
                    "Tentative to write in 0x{:X} in a protected area [0x{:X}:0x{:X}]",
                    address,
                    area.start(),
                    area.end()
                )
            }
            AssemblerError::InvalidSymbol(msg) => {
                write!(f, "Invalid symbol {}", msg)
            }
            AssemblerError::NoDataToCrunch => {
                write!(f, "There is no bytes to crunch")
            }
            AssemblerError::MMRError { value } => {
                write!(
                    f,
                    "{} is invalid. We expect values from 0xC0 to 0xc7.",
                    value
                )
            }
            AssemblerError::RelocatedWarning { warning, span } => {
                let msg =
                    build_simple_error_message(&format!("{}", warning), span, Severity::Warning);
                write!(f, "{}", msg)
            }
            AssemblerError::RelocatedInfo { info, span } => {
                let msg = build_simple_error_message(&format!("{}", info), span, Severity::Note);
                write!(f, "{}", msg)
            }
            AssemblerError::SnapshotError { error } => write!(f, "Snapshot error. {:#?}", error),
            AssemblerError::CrunchedSectionError { error } => {
                write!(f, "Error when crunching code {}", error)
            }
            AssemblerError::NotAllowed => write!(f, "Instruction not allowed in this context."),
            AssemblerError::Fail { msg } => write!(f, "FAIL: {}", msg)
        }
    }
}

fn build_simple_error_message_with_message(title: &str, message: &str, span: &Z80Span) -> String {
    let filename = build_filename(span);
    let source = span.extra.0.as_ref();

    let mut source_files = SimpleFiles::new();
    let file = source_files.add(filename, source);

    let sample_range = std::ops::Range {
        start: span.location_offset(),
        end: guess_error_end(
            source_files.get(file).unwrap().source(),
            span.location_offset(),
            JP_WRONG_PARAM // fake value
        )
    };

    let diagnostic = Diagnostic::error()
        .with_message(title)
        .with_labels(vec![Label::new(
            codespan_reporting::diagnostic::LabelStyle::Primary,
            file,
            sample_range
        )
        .with_message(message)]);

    let mut writer = buffer();
    let config = config();
    term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
}

pub fn build_simple_error_message(title: &str, span: &Z80Span, severity: Severity) -> String {
    let filename = build_filename(span);
    let source = span.extra.0.as_ref();

    let mut source_files = SimpleFiles::new();
    let file = source_files.add(filename, source);

    let sample_range = std::ops::Range {
        start: span.location_offset(),
        end: guess_error_end(
            source_files.get(file).unwrap().source(),
            span.location_offset(),
            JP_WRONG_PARAM // fake value
        )
    };

    let diagnostic = Diagnostic::new(severity)
        .with_message(title)
        .with_labels(vec![Label::new(
            codespan_reporting::diagnostic::LabelStyle::Primary,
            file,
            sample_range
        )]);

    let mut writer = buffer();
    let mut config = config();
    if severity == Severity::Note {
        config.display_style = DisplayStyle::Short;
    }
    term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
}

fn build_filename(span: &Z80Span) -> Box<String> {
    let fname = &span.extra.1.current_filename;
    let context = &span.extra.1.context_name;

    let name = fname
        .as_ref()
        .map(|p| p.as_os_str().to_str().unwrap())
        .unwrap_or_else(|| {
            context
                .as_ref()
                .map(|s| s.as_ref())
                .unwrap_or_else(|| "no file specified")
        });

    Box::new(name.to_owned())
}

fn build_simple_error_message_with_notes(
    title: &str,
    notes: Vec<String>,
    span: &Z80Span
) -> String {
    let filename = build_filename(span);
    let source = span.extra.0.as_ref();

    let mut source_files = SimpleFiles::new();
    let file = source_files.add(filename, source);

    let sample_range = std::ops::Range {
        start: span.location_offset(),
        end: guess_error_end(
            source_files.get(file).unwrap().source(),
            span.location_offset(),
            JP_WRONG_PARAM // fake value
        )
    };

    let diagnostic = Diagnostic::error()
        .with_message(title)
        .with_labels(vec![Label::new(
            codespan_reporting::diagnostic::LabelStyle::Primary,
            file,
            sample_range
        )])
        .with_notes(notes);

    let mut writer = buffer();
    let config = config();
    term::emit(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
}

/// The parser is unable to provide the end of the error.
/// This function tries to get it
fn guess_error_end(code: &str, offset: usize, ctx: &str) -> usize {
    enum EndKind {
        CommaOrEnd,
        End
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
                _ => unimplemented!()
            }
        }
    }
    cpclib_common::lazy_static::lazy_static! {
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
        if previous.is_whitespace() {
            end -= 1;
        }
        else {
            break;
        }
    }
    end
}

fn get_additional_notes(ctx: &str) -> Option<Vec<String>> {
    // phf is not currently usable
    cpclib_common::lazy_static::lazy_static! {
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

fn buffer() -> Buffer {
    if cfg!(feature = "nocolor") {
        println!("no color");
        Buffer::no_color()
    }
    else {
        Buffer::ansi()
    }
}

fn config() -> codespan_reporting::term::Config {
    if cfg!(feature = "nocolor") {
        let mut conf = codespan_reporting::term::Config::default();
        conf.chars = Chars::ascii();
        conf
    }
    else {
        codespan_reporting::term::Config::default()
    }
}

pub struct SimplerAssemblerError<'e>(pub(crate) &'e AssemblerError);

impl<'e> Display for SimplerAssemblerError<'e> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.format(f, false)
    }
}

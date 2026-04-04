impl From<AmsdosError> for Box<AssemblerError> {
    fn from(e: AmsdosError) -> Self {
        Box::new(AssemblerError::AssemblingError {
            msg: format!("Amsdos error: {e:?}")
        })
    }
}
impl From<SnapshotError> for Box<AssemblerError> {
    fn from(e: SnapshotError) -> Self {
        Box::new(AssemblerError::AssemblingError {
            msg: format!("Snapshot error: {e:?}")
        })
    }
}

impl From<BasicError> for Box<AssemblerError> {
    fn from(e: BasicError) -> Self {
        Box::new(AssemblerError::AssemblingError {
            msg: format!("Basic error: {e:?}")
        })
    }
}
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use std::ops::Deref;
use std::sync::LazyLock;

use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::Buffer;
use codespan_reporting::term::{self, Chars, DisplayStyle};
use cpclib_basic::BasicError;
use cpclib_common::itertools::Itertools;
use cpclib_common::smol_str::SmolStr;
use cpclib_disc::amsdos::AmsdosError;
use cpclib_sna::SnapshotError;
use cpclib_tokens::symbols::{PhysicalAddress, SourceLocation, Symbol, SymbolError};
use cpclib_tokens::{BinaryOperation, ExpressionTypeError, tokens};

use crate::Z80Span;
use crate::assembler::AssemblingPass;
use crate::parser::ParserContext;
use crate::preamble::{LocatedListing, SourceString, Z80ParserError, Z80ParserErrorKind};

/// When a span's filename / context_name has the form
/// `{path}:{def_line}:{def_col} > MACRO {name}:`, `complete_source()`
/// contains only the macro body (its line 1 = macro line 1, not the
/// absolute file line).  Codespan therefore computes macro-local line
/// numbers.
///
/// This function returns how many blank lines need to be prepended to
/// the source so that codespan produces absolute line numbers directly.
fn macro_line_offset(filename: &str) -> usize {
    if let Some(macro_pos) = filename.find(" > MACRO ") {
        // filename is "{path}:{def_line}:{def_col} > MACRO {name}:"
        let before = &filename[..macro_pos];
        if let Some(col_colon) = before.rfind(':') {
            let without_col = &before[..col_colon];
            if let Some(line_colon) = without_col.rfind(':') {
                let line_str = &without_col[line_colon + 1..];
                if let Ok(def_line) = line_str.parse::<usize>() {
                    return def_line.saturating_sub(1);
                }
            }
        }
    }
    0
}

/// Adjust `source` and `offset` for a span that lives inside a macro body.
/// See [`macro_line_offset`] for details.
fn adjust_macro_source(filename: &str, source: &str, offset: usize) -> (String, usize) {
    let adj = macro_line_offset(filename);
    if adj > 0 {
        (format!("{}{}", "\n".repeat(adj), source), offset + adj)
    }
    else {
        (source.to_owned(), offset)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionError {
    LeftError(BinaryOperation, Box<AssemblerError>),
    RightError(BinaryOperation, Box<AssemblerError>),
    LeftAndRightError(BinaryOperation, Box<AssemblerError>, Box<AssemblerError>),
    OwnError(Box<AssemblerError>),
    InvalidSize(usize, usize) // expected index
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum AssemblerError {
    /// Dirty trick to not play with memory
    AlreadyRenderedError(String),

    /// The maximum number of passes has been reached
    MaximumNumberOfPassesReached(usize),

    /// Parse of a located listing failed, but the error is in fact stored within the located listing object...
    LocatedListingError(std::sync::Arc<LocatedListing>),

    //#[fail(display = "Several errors arised: {:?}", errors)]
    MultipleErrors {
        errors: Vec<Box<AssemblerError>>
    },

    //#[fail(display = "{} cannot be empty.", 0)]
    EmptyBinaryFile(String),

    //#[fail(display = "Amsdos error: {}", error)]
    AmsdosError {
        error: AmsdosError
    },

    //#[fail(display = "Assembling bug: {}", msg)]
    BugInAssembler {
        file: &'static str,
        line: u32,
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
        error: Z80ParserError
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
        location: Option<SourceLocation>,
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
        kind: SmolStr,
        here: Option<SourceLocation>
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

    ForIssue {
        error: Box<AssemblerError>,
        span: Option<Z80Span>
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

    IfIssue {
        error: Box<AssemblerError>,
        /// The span covering the full conditional block (from `if` to `endif`)
        span: Z80Span
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
    FunctionWithWrongNumberOfArguments(String, either::Either<usize, &'static [usize]>, usize),
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

impl From<Z80ParserError> for AssemblerError {
    fn from(err: Z80ParserError) -> Self {
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

impl From<Box<AssemblerError>> for AssemblerError {
    fn from(err: Box<AssemblerError>) -> Self {
        // Unbox and return the inner error
        *err
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
            },
        }
    }
}

impl From<ExpressionTypeError> for Box<AssemblerError> {
    fn from(e: ExpressionTypeError) -> Self {
        Box::new(AssemblerError::ExpressionTypeError(e))
    }
}

impl From<SymbolError> for Box<AssemblerError> {
    fn from(err: SymbolError) -> Self {
        Box::new(AssemblerError::from(err))
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
            // AlreadyRenderedError already has correct location baked into its string;
            // re-wrapping it with an outer span (e.g. the IF token's line) would
            // produce a misleading second location in the error output.
            AssemblerError::AlreadyRenderedError(_) => true,
            // IfIssue already carries the full IF block span; no need to re-wrap
            AssemblerError::IfIssue { .. } => true,
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
                span,
                error: Box::new(self)
            }
        }
    }

    pub fn locate_warning(self, span: Z80Span) -> Self {
        if self.is_located() {
            self
        }
        else {
            AssemblerError::RelocatedWarning {
                span,
                warning: Box::new(self)
            }
        }
    }
}

#[allow(unused)]
pub(crate) const LD_WRONG_SOURCE: &str = "LD: error in the source";
pub(crate) const LD_WRONG_DESTINATION: &str = "LD: error in the destination";

pub(crate) const JP_WRONG_PARAM: &str = "JP: error in the destination";
#[allow(unused)]
pub(crate) const JR_WRONG_PARAM: &str = "JR: error in the destination";
#[allow(unused)]
pub(crate) const CALL_WRONG_PARAM: &str = "CALL: error in the destination";

pub(crate) const SNASET_WRONG_LABEL: &str = "SNASET: error in the option naming";
pub(crate) const SNASET_MISSING_COMMA: &str = "SNASET: missing comma";

impl Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f, true)
    }
}

impl AssemblerError {
    pub fn is_already_rendered(&self) -> bool {
        match self {
            AssemblerError::AlreadyRenderedError(_) => true,
            _ => false
        }
    }

    pub fn render(self) -> Self {
        match &self {
            Self::AlreadyRenderedError(_) => self,
            _ => Self::AlreadyRenderedError(self.to_string())
        }
    }

    pub fn format(&self, f: &mut std::fmt::Formatter<'_>, complete: bool) -> std::fmt::Result {
        match self {
            AssemblerError::SyntaxError { error } => {
                let mut source_files = SimpleFiles::new();
                let mut fname_to_id = BTreeMap::new();

                let errors = error.errors();
                // Collect offsets that already have a meaningful (non-Winnow) entry so we
                // can suppress redundant "Unknown error" labels at those positions.
                let offsets_with_label: HashSet<usize> = errors
                    .iter()
                    .filter(|e| !matches!(e.1, Z80ParserErrorKind::Winnow))
                    .map(|e| Z80Span::from(*e.0).offset_from_start())
                    .collect();
                let str = errors
                    .iter()
                    .filter(|e| {
                        match e.1 {
                            Z80ParserErrorKind::Winnow => {
                                !offsets_with_label
                                    .contains(&Z80Span::from(*e.0).offset_from_start())
                            },
                            kind => !kind.is_dbg()
                        }
                    })
                    .map(|e| {
                        let ctx = e.1.display_label();

                        let filename =
                            e.0.state
                                .filename()
                                .map(|p| p.as_os_str().to_str().unwrap())
                                .unwrap_or_else(|| e.0.state.context_name().unwrap_or("no file"))
                                .to_owned();

                        let line_adj = macro_line_offset(&filename);
                        let raw_offset = Z80Span::from(*e.0).offset_from_start();
                        let file_id = match fname_to_id.get(&filename) {
                            Some(&id) => id,
                            None => {
                                let (adj_source, _) = adjust_macro_source(
                                    &filename,
                                    e.0.state.complete_source(),
                                    raw_offset
                                );
                                let id = source_files.add(filename.clone(), adj_source);
                                fname_to_id.insert(filename.clone(), id);
                                id
                            }
                        };

                        let offset = raw_offset + line_adj;
                        let end = if let Z80ParserErrorKind::ContextWithEnd { end_offset, .. } = e.1
                        {
                            end_offset + line_adj
                        }
                        else {
                            guess_error_end(
                                source_files.get(file_id).unwrap().source(),
                                offset,
                                &ctx
                            )
                        };

                        let mut diagnostic = Diagnostic::error()
                            .with_message("Syntax error")
                            .with_labels(vec![
                                Label::new(
                                    codespan_reporting::diagnostic::LabelStyle::Primary,
                                    file_id,
                                    offset..end
                                )
                                .with_message(&ctx),
                            ]);

                        if let Some(notes) = get_additional_notes(&ctx) {
                            diagnostic = diagnostic.with_notes(notes);
                        }

                        let mut writer = buffer();
                        let config = config();
                        term::emit_to_write_style(&mut writer, &config, &source_files, &diagnostic)
                            .unwrap();

                        std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
                    })
                    .unique()
                    .join("\n");
                write!(f, "{str}")
            },

            AssemblerError::IncludedFileError { span, error } => {
                match error.as_ref() {
                    AssemblerError::IOError { msg } => {
                        let msg = build_simple_error_message_with_message(
                            "Error for imported file",
                            msg,
                            span
                        );
                        write!(f, "{msg}")
                    },
                    _ => {
                        let msg = build_simple_error_message(
                            "Error in imported file",
                            span,
                            Severity::Error
                        );
                        write!(f, "{msg}")?;
                        error.fmt(f)
                    }
                }
            },

            AssemblerError::OverrideMemory(address, count) => {
                write!(f, "Override {} bytes at {}", *count, address)
            },
            AssemblerError::DisassemblerError { msg } => write!(f, "Disassembler error: {msg}"),

            AssemblerError::ExpressionError(e) => {
                let msg = match e {
                    ExpressionError::LeftError(oper, error) => {
                        format!("on left operand of {oper}: {error}.")
                    },
                    ExpressionError::RightError(oper, error) => {
                        format!("on right operand of {oper}: {error}.")
                    },
                    ExpressionError::LeftAndRightError(oper, error1, error2) => {
                        format!("on left and right operand of {oper}: {error1} / {error2}")
                    },
                    ExpressionError::OwnError(error) => {
                        format!("{error}")
                    },
                    ExpressionError::InvalidSize(expected, index) => {
                        format!("{index} index incompatible with size {expected}")
                    }
                };
                write!(f, "Expression error {msg}")
            },
            AssemblerError::CounterAlreadyExists { symbol } => {
                write!(f, "A counter named `{symbol}` already exists")
            },
            AssemblerError::SymbolAlreadyExists { symbol } => {
                write!(f, "A symbol named `{symbol}` already exists")
            },
            AssemblerError::IncoherentCode { msg } => write!(f, "Incoherent code: {msg}"),
            AssemblerError::NoActiveCounter => write!(f, "No active counter"),
            AssemblerError::OutputExceedsLimits(address, limit) => {
                write!(f, "Code at {address} exceeds limits of 0x{limit:X}")
            },
            AssemblerError::OutputAlreadyExceedsLimits(limit) => {
                write!(f, "Code  already exceeds limits of 0x{limit:X}")
            },
            AssemblerError::RunAlreadySpecified => write!(f, "RUN has already been specified"),
            AssemblerError::AlreadyDefinedSymbol { symbol, kind, here } => {
                if let Some(here) = here {
                    write!(
                        f,
                        "Symbol \"{symbol}\" already defined as a {kind} in {here}"
                    )
                }
                else {
                    write!(f, "Symbol \"{symbol}\" already defined as a {kind}")
                }
            },

            AssemblerError::MultipleErrors { errors } => {
                for e in errors.iter().map(|e| e.to_string()).unique() {
                    writeln!(f, "{e}")?;
                }
                Ok(())
            },

            AssemblerError::UnknownSymbol { symbol, closest } => {
                write!(
                    f,
                    "Unknown symbol: {}.{}",
                    symbol,
                    closest
                        .as_ref()
                        .map(|v| format!(" Closest one is: `{v}`"))
                        .unwrap_or_default()
                )
            },

            AssemblerError::MaximumNumberOfPassesReached(count) => {
                write!(f, "Maximum number of passes reached ({count})")
            },

            AssemblerError::ExpressionTypeError(e) => write!(f, "{e}"),

            AssemblerError::EmptyBinaryFile(name) => {
                write!(f, "Binary file is empty: {name}")
            },
            AssemblerError::AmsdosError { error: e } => {
                write!(f, "AMSDOS error: {e}")
            },
            AssemblerError::BugInAssembler { file, line, msg } => {
                write!(f, "BUG in assembler {file}:{line} {msg}")
            },
            AssemblerError::BugInParser { error, context } => {
                write!(f, "Parser bug: {error} (context: {context:?})")
            },

            AssemblerError::BasicError { error } => write!(f, "{error}"),
            AssemblerError::AssemblingError { msg } => write!(f, "{msg}"),
            AssemblerError::InvalidArgument { msg } => write!(f, "Invalid argument: {msg}"),
            AssemblerError::AssertionFailed {
                test,
                msg,
                guidance
            } => {
                write!(f, "Assert error: {test}\n{msg}\n{guidance}")
            },

            AssemblerError::UnknownMacro { symbol, closest } => {
                write!(
                    f,
                    "MACRO {} does not exist. Try {}",
                    symbol,
                    closest.as_ref().unwrap_or(&SmolStr::new_inline(""))
                )
            },
            AssemblerError::FunctionWithoutReturn(name) => {
                write!(f, "Function {name} has no RETURN directive")
            },
            AssemblerError::FunctionWithEmptyBody(name) => {
                write!(f, "Function {name} has no body")
            },
            AssemblerError::FunctionUnknown(name) => {
                write!(f, "Function {name} unknown")
            },
            AssemblerError::FunctionError(name, e) => {
                write!(f, "Function {name} error: {e}")
            },
            AssemblerError::FunctionWithWrongNumberOfArguments(name, expected, received) => {
                let expected = match expected {
                    either::Either::Left(s) => format!("{s}"),
                    either::Either::Right(s) => format!("one among {s:?}")
                };
                write!(
                    f,
                    "Function {name} called with {received} parameters instead of {expected}"
                )
            },
            AssemblerError::WrongNumberOfParameters {
                symbol,
                nb_paramers,
                nb_arguments
            } => {
                write!(
                    f,
                    "Macro `{symbol}` expects {nb_arguments} argument(s), {nb_paramers} provided"
                )
            },
            AssemblerError::MacroError {
                name,
                location,
                root
            } => {
                if let Some(location) = location {
                    write!(
                        f,
                        "Error in macro call {name} (defined in {location})\n{root}"
                    )
                }
                else {
                    write!(f, "Error in macro call: {name}\n{root}")
                }
            },
            AssemblerError::WrongSymbolType {
                symbol: s,
                isnot: n
            } => {
                write!(f, "Wrong symbol type: {s} is not {n}")
            },
            AssemblerError::IOError { msg } => {
                write!(f, "IO Error: {msg}")
            },
            AssemblerError::UnknownAssemblingAddress => {
                write!(f, "Current assembling address is unknown")
            },
            AssemblerError::ExpressionUnresolvable { expression } => {
                write!(f, "Unable to resolve expression: {expression}")
            },
            AssemblerError::RelativeAddressUncomputable {
                address: _,
                pass: _,
                error
            } => {
                write!(f, "Unable to compute relative address {error}")
            },

            // By construction contains only error with no span information
            AssemblerError::RelocatedError { error, span } => {
                if complete {
                    // Relocated error format may vary among errors
                    match error.deref() {
                        AssemblerError::RelocatedError { error, span: _ } => {
                            write!(f, "{error}")
                        },

                        AssemblerError::UnknownSymbol { symbol, closest } => {
                            let msg = match closest {
                                Some(closest) => {
                                    build_simple_error_message_with_notes(
                                        &format!("Unknown symbol: {symbol}"),
                                        vec![format!("Closest one is: {}", closest)],
                                        span
                                    )
                                },
                                None => {
                                    build_simple_error_message(
                                        &format!("Unknown symbol: {symbol}"),
                                        span,
                                        Severity::Error
                                    )
                                },
                            };

                            write!(f, "{msg}")
                        },

                        AssemblerError::UnknownMacro { symbol, closest } => {
                            let msg = match closest {
                                Some(closest) => {
                                    build_simple_error_message_with_notes(
                                        &format!("Unknown macro: {symbol}"),
                                        vec![format!("Closest one is: {}", closest)],
                                        span
                                    )
                                },
                                None => {
                                    build_simple_error_message(
                                        &format!("Unknown macro: {symbol}"),
                                        span,
                                        Severity::Error
                                    )
                                },
                            };

                            write!(f, "{msg}")
                        },

                        AssemblerError::MacroError {
                            name,
                            location,
                            root
                        } => {
                            let msg = if let Some(location) = location {
                                format!("Error in macro call {name} (defined in {location})")
                            }
                            else {
                                format!("Error in macro call {name}")
                            };

                            let msg = build_simple_error_message(&msg, span, Severity::Error);

                            // The inner `root` error's spans carry the macro body as their
                            // complete_source(), so build_simple_error_message (called
                            // recursively) will now use adjust_macro_source to prefix the
                            // source with the right number of blank lines, making codespan
                            // produce absolute file line numbers directly.
                            write!(f, "{msg}\n{root}")
                        },

                        AssemblerError::BasicError { error } => {
                            let msg =
                                build_simple_error_message("BASIC error", span, Severity::Error);
                            write!(f, "{msg}\n{error}")
                        },

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
                            write!(f, "{msg}")
                        },

                        AssemblerError::CrunchedSectionError { error } => {
                            let msg = build_simple_error_message(
                                "Impossible to crunch section",
                                span,
                                Severity::Error
                            );
                            write!(f, "{msg}")?;
                            write!(f, "{error}")
                        },

                        _ => {
                            let msg = build_simple_error_message(
                                &format!("{error}"),
                                span,
                                Severity::Error
                            );
                            write!(f, "{msg}")
                        }
                    }
                }
                else {
                    write!(f, "{error}")
                }
            },
            AssemblerError::ReadOnlySymbol(symb) => {
                write!(f, "{} cannot be modified", symb.value())
            },

            AssemblerError::RepeatIssue {
                error,
                span,
                repetition
            } => {
                if let Some(span) = span {
                    let msg = build_simple_error_message_with_notes(
                        &format!("REPEAT: error in loop {repetition}"),
                        vec![error.to_string()],
                        span
                    );
                    write!(f, "{msg}")
                }
                else {
                    write!(f, "Repeat issue\n{error}")
                }
            },

            AssemblerError::ForIssue { error, span } => {
                if let Some(span) = span {
                    let msg = build_simple_error_message_with_notes(
                        "FOR: error in loop",
                        vec![error.to_string()],
                        span
                    );
                    write!(f, "{msg}")
                }
                else {
                    write!(f, "FOR issue\n{error}")
                }
            },

            AssemblerError::WhileIssue { error, span } => {
                if let Some(span) = span {
                    let msg = build_simple_error_message_with_notes(
                        "WHILE: error in loop",
                        vec![error.to_string()],
                        span
                    );
                    write!(f, "{msg}")
                }
                else {
                    write!(f, "WHILE issue\n{error}")
                }
            },

            AssemblerError::IfIssue { error, span } => {
                let msg = build_simple_error_message_with_notes(
                    "IF: error in conditional block",
                    vec![error.to_string()],
                    span
                );
                write!(f, "{msg}")
            },

            AssemblerError::OutputProtected { area, address } => {
                write!(
                    f,
                    "Tentative to write in 0x{:X} in a protected area [0x{:X}:0x{:X}]",
                    address,
                    area.start(),
                    area.end()
                )
            },
            AssemblerError::InvalidSymbol(msg) => {
                write!(f, "Invalid symbol \"{msg}\"")
            },
            AssemblerError::NoDataToCrunch => {
                write!(f, "There is no bytes to crunch")
            },
            AssemblerError::MMRError { value } => {
                write!(f, "{value} is invalid. We expect values from 0xC0 to 0xc7.")
            },
            AssemblerError::RelocatedWarning { warning, span } => {
                let msg =
                    build_simple_error_message(&format!("{warning}"), span, Severity::Warning);
                write!(f, "{msg}")
            },
            AssemblerError::RelocatedInfo { info, span } => {
                let msg = build_simple_error_message(&format!("{info}"), span, Severity::Note);
                write!(f, "{msg}")
            },
            AssemblerError::SnapshotError { error } => write!(f, "Snapshot error. {error:#?}"),
            AssemblerError::CrunchedSectionError { error } => {
                write!(f, "Error when crunching code {error}")
            },
            AssemblerError::NotAllowed => write!(f, "Instruction not allowed in this context."),
            AssemblerError::Fail { msg } => write!(f, "FAIL: {msg}"),
            AssemblerError::LocatedListingError(arc) => {
                write!(f, "{}", arc.as_ref().cpclib_error_unchecked())
            },
            AssemblerError::AlreadyRenderedError(e) => write!(f, "{e}")
        }
    }
}

fn build_simple_error_message_with_message(title: &str, message: &str, span: &Z80Span) -> String {
    let filename = build_filename(span);
    let (source, offset) = adjust_macro_source(
        filename.as_ref(),
        span.state.complete_source(),
        span.offset_from_start()
    );

    let mut source_files = SimpleFiles::new();
    let file = source_files.add(filename, source);

    let span_len = span.as_str().len();
    let end = if span_len > 1 {
        offset + span_len
    }
    else {
        guess_error_end(
            source_files.get(file).unwrap().source(),
            offset,
            JP_WRONG_PARAM // fake value
        )
    };
    let sample_range = std::ops::Range { start: offset, end };

    let diagnostic = Diagnostic::error().with_message(title).with_labels(vec![
        Label::new(
            codespan_reporting::diagnostic::LabelStyle::Primary,
            file,
            sample_range
        )
        .with_message(message),
    ]);

    let mut writer = buffer();
    let config = config();
    term::emit_to_write_style(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
}

#[inline]
pub fn build_simple_error_message(title: &str, span: &Z80Span, severity: Severity) -> String {
    let filename = build_filename(span);
    let (source, offset) = adjust_macro_source(
        filename.as_ref(),
        span.state.complete_source(),
        span.offset_from_start()
    );

    let mut source_files = SimpleFiles::new();
    let file = source_files.add(filename, source);

    // TODO do it in a cleaner way. Here it is an ugly path !!!
    let end = if title.starts_with("Override ") {
        // XXX Handle the case of memory overriding that can use lots of instructions
        span.as_str().chars().count() + offset + 1
    }
    else {
        let span_len = span.as_str().len();
        if span_len > 1 {
            // Use the span's actual byte extent so multi-line blocks (IF/REPEAT/…)
            // are highlighted from their opening keyword to their closing directive.
            offset + span_len
        }
        else {
            guess_error_end(
                source_files.get(file).unwrap().source(),
                offset,
                JP_WRONG_PARAM // fake value
            )
        }
    };

    let sample_range = std::ops::Range { start: offset, end };

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
    term::emit_to_write_style(&mut writer, &config, &source_files, &diagnostic).unwrap();

    std::str::from_utf8(writer.as_slice()).unwrap().to_owned()
}

#[inline]
pub fn build_filename(span: &Z80Span) -> Box<String> {
    Box::new(span.filename().to_owned())
}

fn build_simple_error_message_with_notes(
    title: &str,
    notes: Vec<String>,
    span: &Z80Span
) -> String {
    let filename = build_filename(span);
    let (source, offset) = adjust_macro_source(
        filename.as_ref(),
        span.state.complete_source(),
        span.offset_from_start()
    );

    let mut source_files = SimpleFiles::new();
    let file = source_files.add(filename, source);

    let span_len = span.as_str().len();
    let end = if span_len > 1 {
        offset + span_len
    }
    else {
        guess_error_end(
            source_files.get(file).unwrap().source(),
            offset,
            JP_WRONG_PARAM // fake value
        )
    };
    let sample_range = std::ops::Range { start: offset, end };

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
    term::emit_to_write_style(&mut writer, &config, &source_files, &diagnostic).unwrap();

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
                            || current == ';'
                            || offset == code.len()
                        {
                            break;
                        }
                        offset += 1;
                    }
                    offset
                },

                EndKind::CommaOrEnd => {
                    for current in code[offset..].chars() {
                        if current == ','
                            || current == ':'
                            || current == '\n'
                            || current == ';'
                            || offset == code.len()
                        {
                            break;
                        }
                        offset += 1;
                    }
                    offset
                }
            }
        }
    }
    static GUESSER_LUT: LazyLock<HashMap<&'static str, EndKind>> = LazyLock::new(|| {
        let mut hash = HashMap::new();

        hash.insert(LD_WRONG_DESTINATION, EndKind::CommaOrEnd);

        hash
    });

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

    static NOTES_LUT: LazyLock<HashMap<&'static str, Vec<String>>> = LazyLock::new(|| {
        let mut hash = HashMap::new();

        hash.insert(
            LD_WRONG_DESTINATION,
            vec![
                "Possible destinations are:".to_owned(),
                " - 16 bits registers: AF, HL, BC, DE, IX, IY".to_owned(),
                " - 8 bits registers: A, B, C, D, E, H, L, IXH, IXL, IYH, IYL, I".to_owned(),
                " - addresses: (address), (hl), (de), (bc), (IX+delta), (IY+delta)".to_owned(),
            ]
        );

        hash
    });

    NOTES_LUT.get(ctx).cloned()
}

fn buffer() -> Buffer {
    if cfg!(feature = "colored_errors") {
        Buffer::ansi()
    }
    else {
        Buffer::no_color()
    }
}

fn config() -> codespan_reporting::term::Config {
    if cfg!(feature = "colored_errors") {
        codespan_reporting::term::Config::default()
    }
    else {
        let mut conf = codespan_reporting::term::Config::default();
        conf.chars = Chars::ascii();
        conf
    }
}

pub struct SimplerAssemblerError<'e>(pub(crate) &'e AssemblerError);

impl Display for SimplerAssemblerError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.format(f, false)
    }
}

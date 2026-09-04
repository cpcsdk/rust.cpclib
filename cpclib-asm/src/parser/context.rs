use std::borrow::{Borrow, Cow};
use std::collections::HashSet;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{LazyLock, RwLock};

use fs_err::PathExt;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::winnow::BStr;
use cpclib_tokens::symbols::{SymbolFor, SymbolsTableTrait, Value};
use cpclib_tokens::{AssemblerFlavor, ListingElement, Token};
use either::Either;
use enumflags2::BitFlags;
use regex::Regex;

use super::line_col::LineColLookup;
use super::obtained::LocatedTokenInner;
use super::source::Z80Span;
use crate::LocatedToken;
use crate::assembler::Env;
use crate::error::{AssemblerError, WarningCategory};

/// State to limit the parsing abilities depending on the parsing context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsingState {
    /// Parse of a standard Z80 code
    Standard,
    /// Parse of the content of a function
    FunctionLimited,
    /// Parse of the content of a struct
    StructLimited,
    /// Forbid directives
    GeneratedLimited, // TODO rename
    /// Parse of a symbols file
    SymbolsLimited
}

pub trait ParsingStateVerified {
    fn is_accepted(&self, state: &ParsingState) -> bool;
}

impl ParsingStateVerified for LocatedToken {
    fn is_accepted(&self, state: &ParsingState) -> bool {
        self.deref().is_accepted(state)
    }
}

macro_rules! parsing_state_verified_inner {
    () => {
        fn is_accepted(&self, state: &ParsingState) -> bool {
            match state {
                ParsingState::GeneratedLimited => !self.is_directive(),
                ParsingState::Standard => {
                    match self {
                        Self::Return(..) => false,
                        _ => true
                    }
                },
                ParsingState::FunctionLimited => {
                    match self {
                        Self::Equ { .. } | Self::Let(..) => true,
                        Self::If { .. }
                        | Self::Repeat { .. }
                        | Self::Break
                        | Self::Switch { .. }
                        | Self::Iterate { .. } => true,
                        Self::Return(_) => true,
                        Self::Assert(..) | Self::Print(_) | Self::Fail(_) | Self::Comment(_) => {
                            true
                        },
                        _ => false
                    }
                },
                ParsingState::StructLimited => {
                    match self {
                        Self::Defb(..) | Self::Defw(..) | Self::Str(..) | Self::MacroCall(..) => {
                            true
                        },
                        _ => false
                    }
                },
                ParsingState::SymbolsLimited => {
                    match self {
                        Self::Equ { .. } | Self::Let(..) | Self::Comment(_) => true,
                        _ => false
                    }
                },
            }
        }
    };
}

impl ParsingStateVerified for LocatedTokenInner {
    parsing_state_verified_inner!();
}

impl ParsingStateVerified for Token {
    parsing_state_verified_inner!();
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParserOptions {
    /// Search path to find files
    pub search_path: Vec<Utf8PathBuf>,
    /// When activated, the parser also read and parse the include-like directives (deactivated by default)
    pub read_referenced_files: bool,
    pub show_progress: bool,
    /// Set to true when directives must start by a dot
    pub dotted_directive: bool,
    pub assembler_flavor: AssemblerFlavor,
    /// When set, directives that print at *parse* time (`PRINT_PARSE`) are
    /// suppressed instead of writing to the real process stdout. Unlike
    /// `AssemblingOptions::dry_run`, this has nothing to do with assembling
    /// or an `Env` — it exists because `PRINT_PARSE` runs before any `Env`
    /// is created, so it can't be gated at that level; a caller such as an
    /// LSP server (whose real stdout carries JSON-RPC protocol traffic, not
    /// build output) must suppress it here instead.
    pub quiet: bool,
    /// Same reasoning as `quiet`: `FakeInstruction`/`RedundantAccumulatorPrefix`
    /// warnings are detected *in the parser* (`wrap_optional_accumulator_warning`
    /// in `parser/instructions.rs`), which constructs a real `WarningWrapper`
    /// AST node before any `Env`/`AssemblingOptions` exists - gating them only
    /// at `Env::add_warning` would leave the token permanently `is_warning()
    /// == true` even when "disabled". `OverrideMemory`/`Overflow` bits are
    /// never checked from the parser side (harmless if set) - those two are
    /// only ever known at real assemble time, gated via
    /// `AssemblingOptions::disabled_warning_categories` instead.
    pub disabled_warning_categories: BitFlags<WarningCategory>
}

impl Default for ParserOptions {
    fn default() -> Self {
        ParserOptions {
            search_path: Default::default(),
            read_referenced_files: true,
            dotted_directive: false,
            show_progress: false,
            assembler_flavor: AssemblerFlavor::Basm,
            quiet: false,
            disabled_warning_categories: BitFlags::empty()
        }
    }
}

impl ParserOptions {
    pub fn context_builder(self) -> ParserContextBuilder {
        ParserContextBuilder {
            options: self,
            current_filename: None,
            context_name: None,
            state: ParsingState::Standard,
            expansion_columns: None
        }
    }
}

pub struct ParserContextBuilder {
    options: ParserOptions,
    current_filename: Option<Utf8PathBuf>,
    context_name: Option<String>,
    state: ParsingState,
    expansion_columns: Option<ExpansionColumnMap>
}

impl Default for ParserContextBuilder {
    fn default() -> Self {
        ParserOptions::default().context_builder()
    }
}

impl From<ParserContext> for ParserContextBuilder {
    fn from(ctx: ParserContext) -> Self {
        Self {
            state: ctx.state,
            current_filename: ctx.current_filename,
            context_name: ctx.context_name,
            options: ctx.options,
            expansion_columns: ctx.expansion_columns
        }
    }
}

impl ParserContextBuilder {
    pub fn current_filename(&self) -> Option<&Utf8Path> {
        self.current_filename.as_ref().map(|p| p.as_path())
    }

    pub fn context_name(&self) -> Option<&str> {
        self.context_name.as_deref()
    }

    pub fn set_current_filename<S: Into<Utf8PathBuf>>(mut self, fname: S) -> ParserContextBuilder {
        self.current_filename = Some(fname.into());
        self
    }

    pub fn remove_filename(mut self) -> Self {
        self.current_filename.take();
        self
    }

    pub fn set_context_name<S: Into<String>>(mut self, name: S) -> ParserContextBuilder {
        self.context_name = Some(name.into());
        self
    }

    pub fn set_state(mut self, state: ParsingState) -> Self {
        self.state = state;
        self
    }

    /// Record where the text about to be parsed came from - see
    /// [`ExpansionColumnMap`].
    pub fn set_expansion_columns(mut self, columns: ExpansionColumnMap) -> Self {
        self.expansion_columns = Some(columns);
        self
    }

    /// See [`ParserOptions::quiet`].
    pub fn set_quiet(mut self, quiet: bool) -> Self {
        self.options.set_quiet(quiet);
        self
    }

    /// See [`ParserOptions::disabled_warning_categories`].
    pub fn set_disabled_warning_categories(
        mut self,
        categories: BitFlags<WarningCategory>
    ) -> Self {
        self.options.set_disabled_warning_categories(categories);
        self
    }

    pub fn set_options(mut self, options: ParserOptions) -> Self {
        self.options = options;
        self
    }

    /// Build a ParserContext for the given source code
    #[inline]
    pub fn build(self, code: &str) -> ParserContext {
        let code: &'static str = unsafe { std::mem::transmute(code) };
        let str: &'static BStr = unsafe { std::mem::transmute(BStr::new(code)) };
        ParserContext {
            options: self.options,
            current_filename: self.current_filename,
            context_name: self.context_name,
            state: self.state,
            source: str,
            line_col_lut: Default::default(),
            expansion_columns: self.expansion_columns
        }
    }
}

impl ParserOptions {
    pub fn set_read_referenced_files(&mut self, tag: bool) {
        self.read_referenced_files = tag;
    }

    pub fn set_dotted_directives(&mut self, tag: bool) {
        self.dotted_directive = tag;
    }

    /// See [`ParserOptions::quiet`].
    pub fn set_quiet(&mut self, tag: bool) {
        self.quiet = tag;
    }

    /// See [`ParserOptions::disabled_warning_categories`].
    pub fn set_disabled_warning_categories(&mut self, categories: BitFlags<WarningCategory>) {
        self.disabled_warning_categories = categories;
    }

    /// Add a search path and ensure it is ABSOLUTE
    /// Method crashes if the search path does not exist
    pub fn add_search_path<P: Into<PathBuf>>(&mut self, path: P) -> Result<(), AssemblerError> {
        let path = path.into();

        if path.is_dir() {
            #[cfg(not(target_arch = "wasm32"))]
            let path = path.fs_err_canonicalize().unwrap();

            // manual fix for for windows. No idea why
            let path = path.to_str().unwrap();
            const PREFIX: &str = "\\\\?\\";
            let path = if let Some(stripped) = path.strip_prefix(PREFIX) {
                stripped.to_string()
            }
            else {
                path.to_string()
            };

            // Really add
            self.search_path.push(path.into());
            Ok(())
        }
        else {
            Err(AssemblerError::IOError {
                msg: format!(
                    "{} is not a path and cannot be added in the search path",
                    path.to_str().unwrap()
                )
            })
        }
    }

    /// Add the folder that contains the given file. Ignore if there are issues with the filename
    pub fn add_search_path_from_file<P: Into<PathBuf>>(
        &mut self,
        file: P
    ) -> Result<(), AssemblerError> {
        let file = file.into();
        let path = file.fs_err_canonicalize();

        match path {
            Ok(path) => {
                let path = path.parent().unwrap().to_owned();
                self.add_search_path(path)
            },

            Err(err) => {
                Err(AssemblerError::IOError {
                    msg: format!(
                        "Unable to add search path for {}. {}",
                        file.to_str().unwrap(),
                        err
                    )
                })
            },
        }
    }

    /// Return the real path name that correspond to the requested file.
    /// Do it in a case insensitive way (for compatibility reasons)
    pub fn get_path_for(
        &self,
        fname: &str,
        env: Option<&Env>
    ) -> Result<Utf8PathBuf, either::Either<AssemblerError, Vec<String>>> {
        use globset::*;
        let mut does_not_exists = Vec::new();
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{+[^\}]+\}+").unwrap());

        let re = RE.deref();
        // Make the expansion in the filename
        let fname: Cow<str> = if let Some(env) = env {
            let mut fname = fname.to_owned();

            let mut replace = HashSet::new();
            for cap in re.captures_iter(&fname) {
                if cap[0] != fname {
                    replace.insert(cap[0].to_owned());
                }
            }

            // make the replacement
            for model in replace.iter() {
                let local_symbol = &model[1..model.len() - 1]; // remove {}
                let local_value = match env
                    .symbols()
                    .any_value(local_symbol)
                    .map(|vl| vl.map(|vl| vl.value()))
                {
                    Ok(Some(Value::String(s))) => s.to_string(),
                    Ok(Some(Value::Expr(e))) => e.to_string(),
                    Ok(Some(Value::Counter(e))) => e.to_string(),
                    Ok(Some(unkn)) => {
                        unimplemented!("{:?}", unkn)
                    },
                    Ok(None) => {
                        return Err(Either::Left(AssemblerError::UnknownSymbol {
                            symbol: model.into(),
                            closest: env
                                .symbols()
                                .closest_symbol(model, SymbolFor::Any)
                                .unwrap()
                                .map(|s| s.into())
                        }));
                    },
                    Err(e) => return Err(Either::Left(e.into()))
                };
                fname = fname.replace(model, &local_value);
            }
            Cow::Owned(fname)
        }
        else {
            Cow::Borrowed(fname)
        };

        let fname: &str = fname.borrow();

        // early exit if the fname goes in an embedding file
        if fname.starts_with("inner://") {
            return Ok(Utf8Path::new(fname).into());
        }

        let fname = Utf8Path::new(fname);

        // check if file exists
        if fname.is_file() {
            return Ok(fname.into());
        }
        does_not_exists.push(fname.as_str().to_owned());

        // otherwhise, try with the current directory of the environment
        if let Some(env) = env.as_ref()
            && let Some(search) = env.get_current_working_directory()
        {
            let current_path = search.join(fname);
            if current_path.is_file() {
                return Ok(current_path);
            }
            else {
                does_not_exists.push(current_path.to_string());
            }
        }

        // otherwhise try with the folder set up at the beginning
        {
            // loop over all possibilities
            for search in &self.search_path {
                assert!(Utf8Path::new(&search).is_dir());
                let current_path = search.join(fname);

                if current_path.is_file() {
                    return Ok(current_path);
                }
                else {
                    let glob = GlobBuilder::new(current_path.as_path().as_str())
                        .case_insensitive(true)
                        .literal_separator(true)
                        .build()
                        .map_err(|e| {
                            Either::Left(AssemblerError::AssemblingError {
                                msg: format!("Error while searching the file. {e}")
                            })
                        })?;
                    let matcher = glob.compile_matcher();

                    for entry in fs_err::read_dir(search).unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        if matcher.is_match(&path) {
                            return Ok(path.try_into().unwrap());
                        }
                    }

                    does_not_exists.push(current_path.as_str().to_owned());
                }
            }
        }

        // No file found
        Err(Either::Right(does_not_exists))
    }

    pub fn set_flavor(&mut self, flavor: AssemblerFlavor) -> &mut Self {
        self.assembler_flavor = flavor;
        self
    }

    #[inline(always)]
    pub fn is_orgams(&self) -> bool {
        self.assembler_flavor == AssemblerFlavor::Orgams
    }
}
/// Where each piece of an expanded macro body came from, well enough to put a
/// column back where the user wrote it.
///
/// A macro body is substituted textually and re-parsed as a source of its own,
/// so a span inside it carries columns of the *expanded* text: `({addr1})`
/// becomes `(0xc000)`, and every instruction after it on that line has moved.
/// Line numbers survive - substitution inserts no newlines - which is why they
/// could be corrected by a simple shift; columns cannot, and a debugger handed
/// them selects the wrong instruction, or one past the end of the line and so
/// nothing at all.
///
/// One entry per piece of the body, in order, is enough to undo that: literal
/// text runs in step with the body it was copied from, and a substituted
/// argument stands, whole, at the placeholder it replaced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpansionColumnMap {
    pieces: Vec<ExpansionPiece>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExpansionPiece {
    /// Byte offset where this piece starts in the expanded text.
    expanded: u32,
    /// Byte offset where it starts in the body it was expanded from.
    source: u32,
    /// Whether the two advance together. True for body text copied verbatim;
    /// false for a substituted argument, whose every byte maps back to the
    /// single placeholder it replaced.
    verbatim: bool
}

impl ExpansionColumnMap {
    /// Note that the expanded text reaches `expanded` just as the body reaches
    /// `source`. Called in expansion order, so the entries stay sorted.
    pub fn push_piece(&mut self, expanded: usize, source: usize, verbatim: bool) {
        self.pieces.push(ExpansionPiece {
            expanded: expanded as u32,
            source: source as u32,
            verbatim
        });
    }

    /// Where `expanded` sits in the body, and whether the piece holding it runs
    /// in step with the body.
    fn source_offset(&self, expanded: usize) -> Option<(usize, bool)> {
        let expanded = expanded as u32;
        let index = self
            .pieces
            .partition_point(|piece| piece.expanded <= expanded)
            .checked_sub(1)?;
        let piece = self.pieces[index];
        let offset = if piece.verbatim {
            piece.source + (expanded - piece.expanded)
        }
        else {
            piece.source
        };
        Some((offset as usize, piece.verbatim))
    }

    /// The columns, on the body's own line, of a token that the expanded text
    /// reports at `column` of the line starting `column - 1` bytes before
    /// `offset`, and that is `width` bytes long.
    ///
    /// `None` when the answer cannot be trusted - which is the useful outcome,
    /// because the caller then records no columns rather than wrong ones. The
    /// only way in is a substituted argument that carried a newline: the line
    /// the token is on then began inside an argument value, and nothing about
    /// it can be attributed to the body.
    pub fn source_columns(&self, offset: usize, column: usize, width: usize) -> Option<(u16, u16)> {
        let line_start = offset.checked_sub(column.checked_sub(1)?)?;
        let (line_start, verbatim) = self.source_offset(line_start)?;
        if !verbatim {
            return None;
        }
        let (start, _) = self.source_offset(offset)?;
        let (end, _) = self.source_offset(offset + width)?;
        let start = start.checked_sub(line_start)? + 1;
        let end = end.checked_sub(line_start)? + 1;
        if end < start {
            return None;
        }
        Some((u16::try_from(start).ok()?, u16::try_from(end).ok()?))
    }
}

/// Context information that can guide the parser
/// TODO add assembling flags
#[derive(Debug)]
pub struct ParserContext {
    /// Limitation on the kind of intruction to parse.
    /// The current state is at the end (it is modified when in a struct)
    pub state: ParsingState,
    /// Filename that is currently parsed
    pub current_filename: Option<Utf8PathBuf>,
    /// Current context (mainly when playing with macros)
    pub context_name: Option<String>,
    pub options: ParserOptions,
    /// Full source code of the parsing state
    pub source: &'static BStr,
    pub line_col_lut: RwLock<Option<LineColLookup<'static>>>,
    /// Set when `source` is a macro expansion rather than text the user wrote -
    /// see [`ExpansionColumnMap`].
    pub expansion_columns: Option<ExpansionColumnMap>
}

impl Eq for ParserContext {}

impl PartialEq for ParserContext {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.current_filename == other.current_filename
            && self.context_name == other.context_name
            && self.source == other.source
            && self.options == other.options
    }
}

impl Clone for ParserContext {
    fn clone(&self) -> Self {
        panic!("ParserContext::clone() should never be called - this is intentional design");
    }
}

// impl Default for ParserContext {
// fn default() -> Self {
// ParserContext {
// current_filename: None,
// context_name: None,
// search_path: Default::default(),
// read_referenced_files: true,
// parse_warning: Default::default(),
// state: ParsingState::Standard,
// dotted_directive: false,
// source: &NO_CODE,
// show_progress: false
// }
// }
// }

impl ParserContext {
    pub fn clone_with_state(&self, state: ParsingState) -> Self {
        Self {
            current_filename: self.current_filename.clone(),
            context_name: self.context_name.clone(),
            source: self.source,
            options: self.options.clone(),
            line_col_lut: Default::default(), // no need to duplicate the structure
            state,
            expansion_columns: self.expansion_columns.clone()
        }
    }
}

#[allow(missing_docs)]
impl ParserContext {
    #[inline]
    pub fn context_name(&self) -> Option<&str> {
        self.context_name.as_deref()
    }

    /// Whether `source` is a macro or struct body re-parsed as a source of its
    /// own rather than text the user wrote in a file.
    ///
    /// The context name is what says so - `main.asm:289:5 > MACRO DRAW:` - and
    /// it is set nowhere else in that shape.
    #[inline]
    pub fn is_expansion(&self) -> bool {
        self.context_name
            .as_deref()
            .is_some_and(crate::assembler::listing_output::source_map::is_expansion_context)
    }

    /// Where the text of this expansion came from, when that is known.
    #[inline]
    pub fn expansion_columns(&self) -> Option<&ExpansionColumnMap> {
        self.expansion_columns.as_ref()
    }

    #[inline]
    pub fn filename(&self) -> Option<&Utf8Path> {
        self.current_filename.as_ref().map(|p| p.as_path())
    }

    //#[deprecated(note="Totally unsafe. Every test should be modified to not use it")]
    #[inline]
    pub fn build_span<S: ?Sized + AsRef<[u8]>>(&self, src: &S) -> Z80Span {
        Z80Span::new_extra(src, self)
    }

    /// Specify the path that contains the code
    #[inline]
    pub fn set_current_filename<P: Into<Utf8PathBuf>>(&mut self, file: P) {
        let file = file.into();
        self.current_filename = Some(
            file.canonicalize()
                .map(|p| Utf8PathBuf::from_path_buf(p).unwrap())
                .unwrap_or(file)
        )
    }

    #[inline]
    pub fn remove_filename(&mut self) {
        self.current_filename = None;
    }

    #[inline]
    pub fn set_context_name(&mut self, name: &str) {
        self.context_name = Some(name.to_owned());
    }

    #[inline]
    pub fn complete_source(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.source.deref()) }
    }

    #[inline(always)]
    pub fn options(&self) -> &ParserOptions {
        &self.options
    }

    #[inline]
    pub fn state(&self) -> &ParsingState {
        &self.state
    }

    #[inline]
    pub fn relative_line_and_column(&self, offset: usize) -> (usize, usize) {
        if self.line_col_lut.read().unwrap().is_none() {
            let src: &'static str = unsafe { std::str::from_utf8_unchecked(self.source.deref()) };

            self.line_col_lut
                .write()
                .unwrap()
                .replace(LineColLookup::new(src));
        }

        self.line_col_lut
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .get(offset)
    }
}
// pub(crate) static DEFAULT_CTX: ParserContext = ParserContext {
// context_name: None,
// current_filename: None,
// read_referenced_files: false,
// search_path: Vec::new(),
// parse_warning: Default::default()
// };

#[cfg(test)]
mod test_super {
    use super::*;

    #[test]
    fn test_function_state() {
        assert!(Token::Return(0.into()).is_accepted(&ParsingState::FunctionLimited));
    }
    #[test]

    fn test_normal_state() {
        assert!(!Token::Return(0.into()).is_accepted(&ParsingState::Standard));
    }
}

#[cfg(test)]
mod expansion_column_tests {
    use super::*;

    /// `\tld hl, ({a}) : nop` expanded with `a` = `0x1234`, as a map: literal
    /// up to the placeholder, the argument, then the rest.
    fn map() -> ExpansionColumnMap {
        let mut map = ExpansionColumnMap::default();
        map.push_piece(0, 0, true); // "\tld hl, ("
        map.push_piece(9, 9, false); // "0x1234" standing in for "{a}"
        map.push_piece(15, 12, true); // ") : nop"
        map.push_piece(22, 19, true); // the end of both texts
        map
    }

    /// The whole point: the second instruction is reported where it is
    /// *written*, not where the substitution pushed it.
    #[test]
    fn a_column_after_a_substitution_comes_back_to_the_body() {
        // "nop" starts at offset 19 of the expansion, column 20 of its line.
        assert_eq!(map().source_columns(19, 20, 3), Some((17, 20)));
        // ...and the instruction before it is untouched, though it *contains*
        // the substitution: it ends where "({a})" ends, not where "(0x1234)"
        // does.
        assert_eq!(map().source_columns(1, 2, 14), Some((2, 13)));
    }

    /// Anything inside the substituted text answers with the placeholder it
    /// replaced - there is no column in the body for the middle of `0x1234`.
    #[test]
    fn a_column_inside_a_substitution_is_the_placeholder() {
        assert_eq!(map().source_columns(11, 12, 0), Some((10, 10)));
    }

    /// An argument carrying a newline moves the *lines* as well, and then
    /// nothing on the line can be attributed to the body. Saying so is what
    /// makes the caller record no columns rather than wrong ones.
    #[test]
    fn a_line_beginning_inside_an_argument_has_no_answer() {
        let mut map = ExpansionColumnMap::default();
        map.push_piece(0, 0, true);
        map.push_piece(4, 4, false); // an argument spelled over two lines
        map.push_piece(20, 7, true);
        // Offset 12 is on a line that began at 10, inside the argument.
        assert_eq!(map.source_columns(12, 3, 2), None);
    }
}

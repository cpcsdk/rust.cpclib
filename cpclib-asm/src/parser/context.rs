use std::borrow::{Borrow, Cow};
use std::collections::HashSet;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{LazyLock, RwLock};

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::winnow::BStr;
use either::Either;
use regex::Regex;

use super::line_col::LineColLookup;
use crate::error::AssemblerError;
use crate::preamble::*;
use crate::LocatedToken;

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
    pub assembler_flavor: AssemblerFlavor
}

impl Default for ParserOptions {
    fn default() -> Self {
        ParserOptions {
            search_path: Default::default(),
            read_referenced_files: true,
            dotted_directive: false,
            show_progress: false,
            assembler_flavor: AssemblerFlavor::Basm
        }
    }
}

impl ParserOptions {
    pub fn context_builder(self) -> ParserContextBuilder {
        ParserContextBuilder {
            options: self,
            current_filename: None,
            context_name: None,
            state: ParsingState::Standard
        }
    }
}

pub struct ParserContextBuilder {
    options: ParserOptions,
    current_filename: Option<Utf8PathBuf>,
    context_name: Option<String>,
    state: ParsingState
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
            options: ctx.options
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
            line_col_lut: Default::default()
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

    /// Add a search path and ensure it is ABSOLUTE
    /// Method crashes if the search path does not exist
    pub fn add_search_path<P: Into<PathBuf>>(&mut self, path: P) -> Result<(), AssemblerError> {
        let path = path.into();

        if path.is_dir() {
            #[cfg(not(target_arch ="wasm32"))]
            let path = path.canonicalize().unwrap();

            // manual fix for for windows. No idea why
            let path = path.to_str().unwrap();
            const PREFIX: &str = "\\\\?\\";
            let path = if path.starts_with(PREFIX) {
                path[PREFIX.len()..].to_string()
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
        let path = file.canonicalize();

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
                let local_value = match env.symbols().value(local_symbol) {
                    Ok(Some(Value::String(s))) => s.to_string(),
                    Ok(Some(Value::Expr(e))) => e.to_string(),
                    Ok(Some(Value::Counter(e))) => e.to_string(),
                    Ok(Some(unkn)) => {
                        unimplemented!("{:?}", unkn)
                    },
                    Ok(None) => {
                        return Err(Either::Left(AssemblerError::UnknownSymbol {
                            symbol: model.into(),
                            closest: env.symbols().closest_symbol(model, SymbolFor::Any).unwrap()
                        }))
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
        if let Some(env) = env.as_ref() {
            if let Some(search) = env.get_current_working_directory() {
                let current_path = search.join(fname);
                if current_path.is_file() {
                    return Ok(current_path.try_into().unwrap());
                }
                else {
                    does_not_exists.push(current_path.to_string());
                }
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
                        .unwrap();
                    let matcher = glob.compile_matcher();

                    for entry in std::fs::read_dir(search).unwrap() {
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
    pub line_col_lut: RwLock<Option<LineColLookup<'static>>>
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
        panic!();

        Self {
            current_filename: self.current_filename.clone(),
            context_name: self.context_name.clone(),
            state: self.state,
            source: self.source,
            options: self.options.clone(),
            line_col_lut: RwLock::default() /* no need to copy paste the datastructure if it is never used */
        }
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
            state
        }
    }
}

#[allow(missing_docs)]
impl ParserContext {
    #[inline]
    pub fn context_name(&self) -> Option<&str> {
        self.context_name.as_deref()
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
        unsafe { std::mem::transmute(self.source.deref()) }
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
            let src: &'static str = unsafe { std::mem::transmute(self.source.deref()) };

            self.line_col_lut
                .write()
                .unwrap()
                .replace(LineColLookup::new(src));
        }

        let res = self
            .line_col_lut
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .get(offset);

        res
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

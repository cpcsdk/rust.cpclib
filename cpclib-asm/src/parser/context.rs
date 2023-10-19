use std::borrow::{Borrow, Cow};
use std::collections::HashSet;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use cpclib_common::lazy_static;
use either::Either;
use regex::Regex;

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

macro_rules!  parsing_state_verified_inner {
 () => {
    fn is_accepted(&self, state: &ParsingState) -> bool {
        match state {
            ParsingState::GeneratedLimited => !self.is_directive(),
            ParsingState::Standard => {
                match self {
                    Self::Return(..) => false,
                    _ => true
                }
            }
            ParsingState::FunctionLimited => {
                match self {
                    Self::Equ{..} | Self::Let(..) => true,
                    Self::If { .. }
                    | Self::Repeat { .. }
                    | Self::Break
                    | Self::Switch { .. }
                    | Self::Iterate { .. } => true,
                    Self::Return(_) => true,
                    Self::Assert(..) | Self::Print(_) | Self::Fail(_) | Self::Comment(_) => {
                        true
                    }
                    _ => false
                }
            }
            ParsingState::StructLimited => {
                match self {
                    Self::Defb(..) |
                    Self::Defw(..) |
                    Self::Str(..) |
                    Self::MacroCall(..) => true,
                    _ => false
                }
            },
            ParsingState::SymbolsLimited => {
                match self {
                    Self::Equ{..} | Self::Let(..) | Self::Comment(_) => true,
                    _ => false
                }
            }
        }
    }
 }
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
    pub search_path: Vec<PathBuf>,
    /// When activated, the parser also read and parse the include-like directives (deactivated by default)
    pub read_referenced_files: bool,
    pub show_progress: bool,
    /// Set to true when directives must start by a dot
    pub dotted_directive: bool
}

impl Default for ParserOptions {
    fn default() -> Self {
        ParserOptions {
            search_path: Default::default(),
            read_referenced_files: true,
            dotted_directive: false,
            show_progress: false
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
    current_filename: Option<PathBuf>,
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
    pub fn current_filename(&self) -> Option<&Path> {
        self.current_filename.as_ref().map(|p| p.as_path())
    }

    pub fn context_name(&self) -> Option<&str> {
        self.context_name.as_ref().map(|s| s.as_str())
    }

    pub fn set_current_filename<S: Into<PathBuf>>(mut self, fname: S) -> ParserContextBuilder {
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

    /// Build a ParserContext for the given source code
    pub fn build(self, code: &str) -> ParserContext {
        ParserContext {
            options: self.options,
            current_filename: self.current_filename,
            context_name: self.context_name,
            state: self.state,
            source: unsafe { &*(code as *const str) as &'static str }
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

        if std::path::Path::new(&path).is_dir() {
            let path = path.canonicalize().unwrap();

            // manual fix for for windows. No idea why
            let path = path.to_str().unwrap();
            const PREFIX: &'static str = "\\\\?\\";
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
                    path.to_str().unwrap().to_string()
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
            }

            Err(err) => {
                Err(AssemblerError::IOError {
                    msg: format!(
                        "Unable to add search path for {}. {}",
                        file.to_str().unwrap().to_string(),
                        err.to_string()
                    )
                })
            }
        }
    }

    /// Return the real path name that correspond to the requested file.
    /// Do it in a case insensitive way (for compatibility reasons)
    pub fn get_path_for(
        &self,
        fname: &str,
        env: Option<&Env>
    ) -> Result<PathBuf, either::Either<AssemblerError, Vec<String>>> {
        use globset::*;
        let mut does_not_exists = Vec::new();

        // Make the expansion in the filename
        let fname: Cow<str> = if let Some(env) = env {
            let mut fname = fname.to_owned();

            lazy_static::lazy_static! {
                static ref RE: Regex = Regex::new(r"\{+[^\}]+\}+").unwrap();
            }
            let mut replace = HashSet::new();
            for cap in RE.captures_iter(&fname) {
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
                    }
                    Ok(None) => {
                        return Err(Either::Left(AssemblerError::UnknownSymbol {
                            symbol: model.into(),
                            closest: env.symbols().closest_symbol(model, SymbolFor::Any).unwrap()
                        }))
                    }
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
            return Ok(std::path::Path::new(fname).into());
        }

        let fname = std::path::Path::new(fname);

        // We expect the file to exists if no search_path is provided
        if self.search_path.is_empty() {
            if fname.is_file() {
                return Ok(fname.into());
            }
            else {
                does_not_exists.push(fname.to_str().unwrap().to_owned());
            }
        }
        else {
            // loop over all possibilities
            for search in &self.search_path {
                assert!(std::path::Path::new(&search).is_dir());
                let current_path = search.join(fname);

                if current_path.is_file() {
                    return Ok(current_path);
                }
                else {
                    let glob =
                        GlobBuilder::new(current_path.as_path().display().to_string().as_str())
                            .case_insensitive(true)
                            .literal_separator(true)
                            .build()
                            .unwrap();
                    let matcher = glob.compile_matcher();

                    for entry in std::fs::read_dir(search).unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        if matcher.is_match(&path) {
                            return Ok(path);
                        }
                    }

                    does_not_exists.push(current_path.to_str().unwrap().to_owned());
                }
            }
        }

        // No file found
        return Err(Either::Right(does_not_exists));
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
    pub current_filename: Option<PathBuf>,
    /// Current context (mainly when playing with macros)
    pub context_name: Option<String>,
    pub options: ParserOptions,
    /// Full source code of the parsing state
    pub source: &'static str
}

impl Eq for ParserContext {}

impl PartialEq for ParserContext {
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
        Self {
            current_filename: self.current_filename.clone(),
            context_name: self.context_name.clone(),
            state: self.state.clone(),
            source: self.source,
            options: self.options.clone()
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
            state: state
        }
    }
}

#[allow(missing_docs)]
impl ParserContext {
    pub fn context_name(&self) -> Option<&str> {
        self.context_name.as_ref().map(|b| b.as_str())
    }

    pub fn filename(&self) -> Option<&Path> {
        self.current_filename.as_ref().map(|p| p.as_path())
    }

    //#[deprecated(note="Totally unsafe. Every test should be modified to not use it")]
    pub fn build_span<S: AsRef<str>>(&self, src: S) -> Z80Span {
        Z80Span::new_extra(src.as_ref(), self)
    }

    /// Specify the path that contains the code
    pub fn set_current_filename<P: Into<PathBuf>>(&mut self, file: P) {
        let file = file.into();
        self.current_filename = Some(file.canonicalize().unwrap_or(file))
    }

    pub fn remove_filename(&mut self) {
        self.current_filename = None;
    }

    pub fn set_context_name(&mut self, name: &str) {
        self.context_name = Some(name.to_owned());
    }

    pub fn complete_source(&self) -> &str {
        self.source
    }

    pub fn options(&self) -> &ParserOptions {
        &self.options
    }

    pub fn state(&self) -> &ParsingState {
        &self.state
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

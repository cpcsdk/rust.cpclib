use std::ops::Deref;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::error::AssemblerError;
use crate::preamble::*;
use crate::LocatedToken;

/// State to limit the parsing abilities depending on the parsing context
#[derive(Debug, Clone, Copy)]
pub enum ParsingState {
    Standard,
    FunctionLimited,
    StructLimited,
    GeneratedLimited
}

pub trait ParsingStateVerified {
    fn is_accepted(&self, state: &ParsingState) -> bool;
}

impl ParsingStateVerified for LocatedToken {
    fn is_accepted(&self, state: &ParsingState) -> bool {
        match state {
            ParsingState::GeneratedLimited => !self.is_directive(),
            ParsingState::Standard => {
                match self {
                    LocatedToken::Standard { token, span: _span } => token.is_accepted(state), /* because of return */
                    _ => true
                }
            }
            ParsingState::FunctionLimited => {
                match self {
                    LocatedToken::Standard { token, span: _span } => token.is_accepted(state),
                    LocatedToken::If { .. }
                    | LocatedToken::Repeat { .. }
                    | LocatedToken::Switch { .. }
                    | LocatedToken::Iterate { .. } => true,
                    _ => false
                }
            }
            ParsingState::StructLimited => todo!()
        }
    }
}

impl ParsingStateVerified for Token {
    fn is_accepted(&self, state: &ParsingState) -> bool {
        match state {
            ParsingState::GeneratedLimited => !self.is_directive(),

            ParsingState::Standard => {
                match self {
                    Token::Return(_) => false,
                    _ => true
                }
            }
            ParsingState::FunctionLimited => {
                match self {
                    Token::Equ(..) | Token::Let(..) => true,
                    Token::If { .. }
                    | Token::Repeat { .. }
                    | Token::Break
                    | Token::Switch { .. }
                    | Token::Iterate { .. } => true,
                    Token::Return(_) => true,
                    Token::Assert(..) | Token::Print(_) | Token::Fail(_) | Token::Comment(_) => {
                        true
                    }
                    _ => false
                }
            }
            ParsingState::StructLimited => todo!()
        }
    }
}

/// Context information that can guide the parser
/// TODO add assembling flags
#[derive(Debug)]
pub struct ParserContext {
    /// Limitation on the kind of intruction to parse
    pub state: ParsingState,
    /// Filename that is currently parsed
    pub current_filename: Option<PathBuf>,
    /// Current context (mainly when playing with macros)
    pub context_name: Option<String>,
    /// Search path to find files
    pub search_path: Vec<PathBuf>,
    /// When activated, the parser also read and parse the include-like directives (deactivated by default)
    pub read_referenced_files: bool,
    /// Set to true when directives must start by a dot
    pub dotted_directive: bool,
    /// indicate we are parsing a listing generating by a struct
    pub parse_warning: RwLock<Vec<AssemblerError>>,
    /// source code of the parsing state
    pub source: Option<&'static str>
}

impl Clone for ParserContext {
    fn clone(&self) -> Self {
        Self {
            current_filename: self.current_filename.clone(),
            context_name: self.context_name.clone(),
            search_path: self.search_path.clone(),
            read_referenced_files: self.read_referenced_files.clone(),
            parse_warning: self.parse_warning.write().unwrap().clone().into(),
            state: self.state.clone(),
            dotted_directive: self.dotted_directive.clone(),
            source: self.source.clone()
        }
    }
}

impl Default for ParserContext {
    fn default() -> Self {
        ParserContext {
            current_filename: None,
            context_name: None,
            search_path: Default::default(),
            read_referenced_files: true,
            parse_warning: Default::default(),
            state: ParsingState::Standard,
            dotted_directive: false,
            source: None
        }
    }
}

impl ParserContext {
    pub fn clone_with_state(&self, state: ParsingState) -> Self {
        Self {
            current_filename: self.current_filename.clone(),
            context_name: self.context_name.clone(),
            search_path: self.search_path.clone(),
            read_referenced_files: self.read_referenced_files.clone(),
            parse_warning: self.parse_warning.write().unwrap().clone().into(),
            dotted_directive: self.dotted_directive.clone(),
            source: self.source.clone(),
            state
        }
    }
}

#[allow(missing_docs)]
impl ParserContext {
    /*
    pub fn build_span<S: Into<String>>(&self, src: S) -> Z80Span {
        Z80Span::new_extra(src, self.clone())
    }
    */

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
            const prefix: &'static str = "\\\\?\\";
            let path = if path.starts_with(prefix) {
                path[prefix.len()..].to_string()
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
    pub fn get_path_for<P: Into<PathBuf>>(&self, fname: P) -> Result<PathBuf, Vec<String>> {
        use globset::*;
        let mut does_not_exists = Vec::new();

        let fname = fname.into();

        // We expect the file to exists if no search_path is provided
        if self.search_path.is_empty() {
            if fname.is_file() {
                return Ok(fname);
            }
            else {
                does_not_exists.push(fname.to_str().unwrap().to_owned());
            }
        }
        else {
            // loop over all possibilities
            for search in &self.search_path {
                assert!(std::path::Path::new(&search).is_dir());
                let current_path = search.join(fname.clone());

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
        return Err(does_not_exists);
    }

    pub fn add_warning(&self, warning: AssemblerError) {
        self.parse_warning.write().unwrap().push(warning)
    }

    pub fn warnings(&self) -> Vec<AssemblerError> {
        self.parse_warning.write().unwrap().deref().clone() // TODO investigate why I cannot return a reference
    }

    pub fn pop_warning(&self) -> Option<AssemblerError> {
        self.parse_warning.write().unwrap().pop() // TODO investigate why I cannot return a reference
    }

    pub fn complete_source(&self) -> &str {
        self.source.unwrap()
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

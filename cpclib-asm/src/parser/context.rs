use std::path::PathBuf;

use crate::error::AssemblerError;

use super::Z80Span;

/// Context information that can guide the parser
/// TODO add assembling flags
#[derive(Clone, Debug)]
pub struct ParserContext {
    /// Filename that is currently parsed
    pub current_filename: Option<PathBuf>,
    /// Current context (mainly when playing with macros)
    pub context_name: Option<String>,
    /// Search path to find files
    pub search_path: Vec<PathBuf>,
    /// When activated, the parser also read and parse the include-like directives (deactivated by default)
    pub read_referenced_files: bool,
}

impl Default for ParserContext {
    fn default() -> Self {
        ParserContext {
            current_filename: None,
            context_name: None,
            search_path: Default::default(),
            read_referenced_files: false,
        }
    }
}

#[allow(missing_docs)]
impl ParserContext {
    pub fn build_span<S: Into<String>>(&self, src: S) -> Z80Span {
        Z80Span::new_extra(src, self.clone())
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

    pub fn set_read_referenced_files(&mut self, tag: bool) {
        self.read_referenced_files = true;
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
            } else {
                path.to_string()
            };

            // Really add
            self.search_path.push(path.into());
            Ok(())
        } else {
            Err(AssemblerError::IOError {
                msg: format!(
                    "{} is not a path and cannot be added in the search path",
                    path.to_str().unwrap().to_string()
                ),
            })
        }
    }

    /// Add the folder that contains the given file. Ignore if there are issues with the filename
    pub fn add_search_path_from_file<P: Into<PathBuf>>(
        &mut self,
        file: P,
    ) -> Result<(), AssemblerError> {
        let file = file.into();
        let path = file.canonicalize();

        match path {
            Ok(path) => {
                let path = path.parent().unwrap().to_owned();
                self.add_search_path(path)
            }

            Err(err) => Err(AssemblerError::IOError {
                msg: format!(
                    "Unable to add search path for {}. {}",
                    file.to_str().unwrap().to_string(),
                    err.to_string()
                ),
            }),
        }
    }

    /// Return the real path name that correspond to the requested file
    pub fn get_path_for<P: Into<PathBuf>>(&self, fname: P) -> Result<PathBuf, Vec<String>> {
        let mut does_not_exists = Vec::new();
        let fname = fname.into();

        // We expect the file to exists if no search_path is provided
        if self.search_path.is_empty() {
            if fname.is_file() {
                return Ok(fname);
            } else {
                does_not_exists.push(fname.to_str().unwrap().to_owned());
            }
        } else {
            // loop over all possibilities
            for search in &self.search_path {
                assert!(std::path::Path::new(&search).is_dir());
                let current_path = search.join(fname.clone());

                if current_path.is_file() {
                    return Ok(current_path);
                } else {
                    does_not_exists.push(current_path.to_str().unwrap().to_owned())
                }
            }
        }

        // No file found
        return Err(does_not_exists);
    }
}

pub(crate) static DEFAULT_CTX: ParserContext = ParserContext {
    context_name: None,
    current_filename: None,
    read_referenced_files: false,
    search_path: Vec::new(),
};
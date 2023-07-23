use deps::Graph;
use std::{
    io::{BufReader, Read},
    path::Path,
};
use thiserror::Error;

pub mod deps;
pub mod executor;
pub mod parser;
pub mod runners;
pub mod task;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Error, Debug)]
pub enum BndBuilderError {
    #[error("Unable to access file {fname}: {error}.")]
    InputFileError {
        fname: String,
        error: std::io::Error,
    },
    #[error("Unable to setup working directory {fname}: {error}.")]
    WorkingDirectoryError {
        fname: String,
        error: std::io::Error,
    },
    #[error("Unable to deserialize rules {0}.")]
    ParseError(serde_yaml::Error),
    #[error("Unable to build the dependency graph {0}.")]
    DependencyError(String),
    #[error("Unable to build {fname}: {msg}.")]
    ExecuteError { fname: String, msg: String },
    #[error("Unable to build default target.\n{source}")]
    DefaultTargetError { source: Box<BndBuilderError> },
    #[error("The file does not contain a target.")]
    NoTargets,
}

self_cell::self_cell! {
    /// WARNING the BndBuilder changes the current working directory.
    /// This is probably a problematic behavior. Need to think about it later
    struct BndBuilderInner {
        owner: deps::Rules,
        #[covariant]
        dependent: Graph,
    }
}

pub struct BndBuilder {
    inner: BndBuilderInner,
}

impl BndBuilder {
    pub fn from_fname<P: AsRef<Path>>(fname: P) -> Result<Self, BndBuilderError> {
        let fname = fname.as_ref();
        let file = std::fs::File::open(fname).map_err(|e| BndBuilderError::InputFileError {
            fname: fname.display().to_string(),
            error: e,
        })?;

        let path = std::path::Path::new(fname).parent().unwrap();
        let working_directory = if path.is_dir() { Some(path) } else { None };

        let rdr = BufReader::new(file);
        Self::from_reader(rdr, working_directory)
    }

    pub fn from_reader<P: AsRef<Path>>(
        mut rdr: impl Read,
        working_directory: Option<P>,
    ) -> Result<Self, BndBuilderError> {
        if let Some(working_directory) = working_directory {
            let working_directory = working_directory.as_ref();
            std::env::set_current_dir(working_directory).map_err(|e| {
                BndBuilderError::WorkingDirectoryError {
                    fname: working_directory.display().to_string(),
                    error: e,
                }
            })?;
        }

        let rules: deps::Rules =
            serde_yaml::from_reader(&mut rdr).map_err(|e| BndBuilderError::ParseError(e))?;

        let inner = BndBuilderInner::try_new(rules, |rules| rules.to_deps())?;

        Ok(BndBuilder { inner })
    }

    /// Return the default target if any
    pub fn default_target(&self) -> Option<&Path> {
        self.inner.borrow_owner().default_target()
    }

    /// Execute the target after all its predecessors
    pub fn execute<P: AsRef<Path>>(&self, target: P) -> Result<(), BndBuilderError> {
        self.inner.borrow_dependent().execute(target)
    }
}

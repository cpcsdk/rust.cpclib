use std::collections::HashSet;
use std::io::{BufReader, Read};
use std::path::Path;

use cpclib_common::itertools::Itertools;

use crate::rules::{self, Graph, Rule};
use crate::BndBuilderError;

self_cell::self_cell! {
    /// WARNING the BndBuilder changes the current working directory.
    /// This is probably a problematic behavior. Need to think about it later
    struct BndBuilderInner {
        owner: rules::Rules,
        #[covariant]
        dependent: Graph,
    }
}

pub struct BndBuilder {
    inner: BndBuilderInner
}

impl BndBuilder {
    pub fn add_default_rule<S: AsRef<str>>(mut self, targets: &[S],
        dependencies: &[S], kind: &str) -> Self {

        let rule = Rule::new_default(targets, dependencies, kind);
        let mut rules = self.inner.into_owner();
        rules.add(rule);

        let inner = BndBuilderInner::try_new(rules, |rules| rules.to_deps()).unwrap();
        BndBuilder { inner }
    }


    pub fn from_fname<P: AsRef<Path>>(fname: P) -> Result<Self, BndBuilderError> {
        let fname = fname.as_ref();
        let file = std::fs::File::open(fname).map_err(|e| {
            BndBuilderError::InputFileError {
                fname: fname.display().to_string(),
                error: e
            }
        })?;

        let path = std::path::Path::new(fname).parent().unwrap();
        let working_directory = if path.is_dir() { Some(path) } else { None };

        let rdr = BufReader::new(file);
        Self::from_reader(rdr, working_directory)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let contents = self.inner.borrow_owner().to_string();
        std::fs::write(path, contents)
    }

    pub fn from_reader<P: AsRef<Path>>(
        mut rdr: impl Read,
        working_directory: Option<P>
    ) -> Result<Self, BndBuilderError> {
        if let Some(working_directory) = working_directory {
            let working_directory = working_directory.as_ref();
            println!(
                "> Set working directory to: {}",
                working_directory.display()
            );
            std::env::set_current_dir(working_directory).map_err(|e| {
                BndBuilderError::WorkingDirectoryError {
                    fname: working_directory.display().to_string(),
                    error: e
                }
            })?;
        }

        let rules: rules::Rules =
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

    pub fn outdated<P: AsRef<Path>>(&self, target: P) -> Result<bool, BndBuilderError> {
        self.inner.borrow_dependent().outdated(target, true)
    }

    pub fn get_layered_dependencies(&self) -> Vec<HashSet<&Path>> {
        self.inner.borrow_dependent().get_layered_dependencies()
    }

    pub fn get_layered_dependencies_for<'a, P: AsRef<Path>>(
        &'a self,
        p: &'a P
    ) -> Vec<HashSet<&'a Path>> {
        self.inner
            .borrow_dependent()
            .get_layered_dependencies_for(p)
    }

    pub fn get_rule<P: AsRef<Path>>(&self, tgt: P) -> Option<&Rule> {
        self.inner.borrow_owner().rule(tgt)
    }

    pub fn rules(&self) -> &[Rule] {
        self.inner.borrow_owner().rules()
    }

    pub fn targets<'a>(&'a self) -> Vec<&'a Path> {
        self.rules()
            .iter()
            .map(|r| r.targets())
            .flatten()
            .map(|p| p.as_path())
            .collect_vec()
    }
}

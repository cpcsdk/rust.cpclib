use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use cpclib_common::itertools::Itertools;
use serde::de::Visitor;
use serde::{self, Deserialize, Deserializer};
use topologic::AcyclicDependencyGraph;

use crate::constraints::{deserialize_constraint, Constraint};
use crate::executor::execute;
use crate::task::Task;
use crate::{expand_glob, BndBuilderError};

use super::{Graph, Rule};

#[derive(Deserialize)]
#[serde(transparent)]
pub struct Rules {
    rules: Vec<Rule>
}

impl Rules {
    pub fn new(rules: Vec<Rule>) -> Self {
        Rules { rules }
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

	pub fn rule_at(&self, at: usize) -> &Rule {
		&self.rules[at]
	}

    /// Get the rule for this target (of course None is returned for leaf files)
    pub fn rule<P: AsRef<Path>>(&self, tgt: P) -> Option<&Rule> {
        let tgt = tgt.as_ref();

        // remove current dir path if any
        let tgt = if tgt.starts_with(r"./") {
            tgt.strip_prefix(r"./").unwrap()
        } else if dbg!(tgt.to_str().unwrap().starts_with(r".\")) {
            Path::new(&tgt.to_str().unwrap()[2..])
        }else {
            tgt
        };

        self.rules
            .iter()
            .find(|r| r.targets().iter().any(|tgt2| tgt2 == &tgt))
    }

    pub fn default_target(&self) -> Option<&Path> {
        self.rules.get(0).map(|r| r.target(0))
    }

    // TODO implement a version with less copy
    pub fn to_deps(&self) -> Result<Graph, BndBuilderError> {
        let mut g = AcyclicDependencyGraph::<&Path>::new();
        let mut node2tracked_idx: BTreeMap<&Path, usize> = BTreeMap::new();

        for (idx, rule) in self.rules.iter().enumerate() {
            if !rule.is_enabled() {
                continue;
            }

            for p in rule.targets().iter() {
                let p: &Path = p.as_ref();
                // link the rule to the target
                if node2tracked_idx.contains_key(p) {
                    let other_rule_idx = node2tracked_idx.get(p).unwrap();
                    let other_rule = &self.rules[*other_rule_idx];
                    return Err(BndBuilderError::DependencyError(
                        format! {"{} has already a rule to build it:\n{:?}", p.display(), other_rule}
                    ));
                }
                else {
                    node2tracked_idx.insert(p, idx);
                }

                // link the target to the dependencies
                for p2 in rule.dependencies().iter() {
                    g.depend_on(p, p2).map_err(|_e| {
                        BndBuilderError::DependencyError(format!(
                            "{} and {}",
                            p.display(),
                            p2.display()
                        ))
                    })?;
                }
            }
        }

        Ok(Graph {
            node2tracked: node2tracked_idx,
            tracked: self,
            g
        })
    }
}

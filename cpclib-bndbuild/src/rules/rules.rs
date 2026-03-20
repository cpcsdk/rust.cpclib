use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use serde::{self, Deserialize};
use topologic::AcyclicDependencyGraph;

use super::{Graph, Rule};
use crate::BndBuilderError;

#[derive(Deserialize)]
#[serde(transparent)]
pub struct Rules {
    rules: Vec<Rule>
}

impl Display for Rules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rule in self.rules() {
            writeln!(f, "{rule}")?;
        }

        Ok(())
    }
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

    pub fn add(&mut self, rule: Rule) {
        self.rules.push(rule)
    }

    /// Get the rule for this target (of course None is returned for leaf files)
    /// We can have several versions depending on OS. In case of multiplicity returns only the appropriate one
    pub fn rule<P: AsRef<Utf8Path>>(&self, tgt: P) -> Option<&Rule> {
        let tgt = tgt.as_ref();

        // remove current dir path if any
        let tgt = if tgt.starts_with(r"./") {
            tgt.strip_prefix(r"./").unwrap()
        }
        else if tgt.as_str().starts_with(r".\") {
            Utf8Path::new(&tgt.as_str()[2..])
        }
        else {
            tgt
        };

        // when the rule is present several times, we only get the one of the appropriate for the filtering
        let mut rules = self
            .rules
            .iter()
            .filter(|r| r.targets().iter().any(|tgt2| tgt2 == tgt))
            .collect_vec();

        if rules.is_empty() {
            return None;
        }
        if rules.len() == 1 {
            rules.pop()
        }
        else {
            let indicies = rules.iter().positions(|r| r.is_enabled()).collect_vec();
            if indicies.is_empty() || indicies.len() > 1 {
                return rules.pop(); // return any one, we know it will fail or it is ambiguous
            }
            assert_eq!(1, indicies.len());
            Some(rules[indicies[0]]) // return the first that match
        }
    }

    pub fn default_target(&self) -> Option<&Utf8Path> {
        self.rules.first().map(|r| r.target(0))
    }

    // TODO implement a version with less copy
    pub fn to_deps(&self) -> Result<Graph<'_>, BndBuilderError> {
        let mut g = AcyclicDependencyGraph::<&Utf8Path>::new();
        let mut node2tracked_idx: BTreeMap<&Utf8Path, usize> = BTreeMap::new();

        for (idx, rule) in self.rules.iter().enumerate() {
            if !rule.is_enabled() {
                continue;
            }

            for p in rule.targets().iter() {
                let p: &Utf8Path = p.as_ref();
                // link the rule to the target
                if node2tracked_idx.contains_key(p) {
                    let other_rule_idx = node2tracked_idx.get(p).unwrap();
                    let other_rule = &self.rules[*other_rule_idx];
                    return Err(BndBuilderError::DependencyError(
                        format! {"{p} has already a rule to build it:\n{other_rule:?}"}
                    ));
                }
                else {
                    node2tracked_idx.insert(p, idx);
                }

                // link the target to the dependencies
                for p2 in rule.dependencies().iter() {
                    g.depend_on(p, p2)
                        .map_err(|_e| BndBuilderError::DependencyError(format!("{p} and {p2}")))?;
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

impl Deref for Rules {
    type Target = Vec<Rule>;

    fn deref(&self) -> &Self::Target {
        &self.rules
    }
}

impl DerefMut for Rules {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rules
    }
}

use std::collections::{BTreeMap, HashSet};

use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use topologic::AcyclicDependencyGraph;

use super::{Rule, Rules};
use crate::BndBuilderError;

#[derive(Clone)]
pub struct Graph<'r> {
    pub(crate) node2tracked: BTreeMap<&'r Utf8Path, usize>,
    pub(crate) tracked: &'r Rules,
    pub(crate) g: AcyclicDependencyGraph<&'r Utf8Path>
}

impl<'r> Graph<'r> {
    pub fn get_layered_dependencies(&self) -> Vec<HashSet<&Utf8Path>> {
        let mut res = self.g.get_forward_dependency_topological_layers();
        let orphans = self
            .tracked
            .rules()
            .iter() // get the nodes that are not in the graph because they have no dependencies
            .filter(|rule| rule.dependencies().is_empty())
            .flat_map(|r| r.targets())
            .map(|p| p.as_path())
            .collect::<HashSet<&Utf8Path>>();
        if !orphans.is_empty() {
            res.push(orphans);
        }
        res.reverse();
        res
    }

    pub fn get_layered_dependencies_for<P: AsRef<Utf8Path>>(
        &self,
        p: &'r P
    ) -> Vec<HashSet<&Utf8Path>> {
        let p: &Utf8Path = p.as_ref();
        let mut layers = self.g.get_forward_dependency_topological_layers();
        let mut nodes = self.g.get_forward_dependencies(&p);
        nodes.insert(p);

        // For each layer, keep only the nodes belonging the the dependency
        for layer in layers.iter_mut() {
            *layer = layer.intersection(&nodes).cloned().collect();
        }

        // remove empty layers if any
        layers.into_iter().filter(|l| !l.is_empty()).collect_vec()
    }

    pub fn show_dependencies<P: AsRef<Utf8Path>>(&self, p: P) {
        let layers = self.get_layered_dependencies_for(&p);

        for layer in layers {
            println!("{:?}", layer)
        }
    }

    pub fn outdated<P: AsRef<Utf8Path>>(
        &self,
        p: P,
        skip_rules_without_commands: bool
    ) -> Result<bool, BndBuilderError> {
        let p = p.as_ref();
        // a phony rule is always outdated
        if self.rule(p)?.is_phony() {
            return Ok(true);
        }

        let dependences = self.get_layered_dependencies_for(&p);
        let dependencies = dependences.into_iter().flatten().collect_vec();

        for p in dependencies.into_iter().rev() {
            let res = match self.rule(p) {
                Ok(r) => {
                    if skip_rules_without_commands {
                        if r.is_phony() {
                            false
                        }
                        else {
                            !r.is_up_to_date()
                        }
                    }
                    else {
                        !r.is_up_to_date()
                    }
                },

                Err(BndBuilderError::UnknownTarget(msg)) => {
                    if !p.exists() {
                        return Err(BndBuilderError::UnknownTarget(msg));
                    }
                    else {
                        false
                    }
                },
                _ => todo!()
            };
            if res {
                return Ok(true);
            }
        }
        return Ok(false);
        //  .unwrap_or(false) // ignore not existing rule. Should fail ?
    }

    #[inline]
    pub fn rule<P: AsRef<Utf8Path>>(&self, p: P) -> Result<&Rule, BndBuilderError> {
        let tgt = p.as_ref();

        let p = if tgt.starts_with(r"./") {
            tgt.strip_prefix(r"./").unwrap()
        }
        else if tgt.as_str().starts_with(r".\") {
            Utf8Path::new(&tgt.as_str()[2..])
        }
        else {
            tgt
        };

        self.node2tracked
            .get(p)
            .map(|idx| self.tracked.rule_at(*idx))
            .ok_or_else(|| /*todo!()*/   BndBuilderError::UnknownTarget(p.as_str().to_owned()))
    }

    #[inline]
    pub fn has_rule<P: AsRef<Utf8Path>>(&self, p: P) -> bool {
        let p = p.as_ref();
        self.node2tracked.contains_key(p)
    }
}

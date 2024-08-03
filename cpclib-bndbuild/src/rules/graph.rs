use std::collections::{BTreeMap, HashSet};

use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use topologic::AcyclicDependencyGraph;

use super::{Rule, Rules};
use crate::executor::execute;
use crate::BndBuilderError;

#[derive(Clone)]
pub struct Graph<'r> {
    pub(crate) node2tracked: BTreeMap<&'r Utf8Path, usize>,
    pub(crate) tracked: &'r Rules,
    pub(crate) g: AcyclicDependencyGraph<&'r Utf8Path>
}

#[derive(Default)]
struct ExecutionState {
    nb_deps: usize,
    task_count: usize
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
        let dependences = self.get_layered_dependencies_for(&p);
        let dependencies = dependences.into_iter().flatten().collect_vec();
        let res = dependencies.into_iter().rev().any(|p| {
            self.rule(p)
                .map(|r| {
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
                })
                .unwrap_or(false) // ignore not existing rule. Should fail ?
        });
        Ok(res)
    }

    pub fn rule<P: AsRef<Utf8Path>>(&self, p: P) -> Option<&Rule> {
        let p = p.as_ref();
        self.node2tracked
            .get(p)
            .map(|idx| self.tracked.rule_at(*idx))
    }

    pub fn execute<P: AsRef<Utf8Path>>(&self, p: P) -> Result<(), BndBuilderError> {
        let p = p.as_ref();
        println!("> Compute dependencies");

        let layers = self.get_layered_dependencies_for(&p);
        let mut state = ExecutionState {
            nb_deps: layers.iter().map(|l| l.len()).sum::<usize>(),
            task_count: 0
        };

        if state.nb_deps == 0 {
            if self.node2tracked.contains_key(p) {
                println!("> Execute task");
                state.nb_deps = 1;
                self.execute_rule(p, &mut state)?;
            }
            else {
                return Err(BndBuilderError::ExecuteError {
                    fname: p.to_string(),
                    msg: "no rule to build it".to_owned()
                });
            }
        }
        else {
            println!("> Execute tasks");
            for layer in layers.into_iter() {
                self.execute_layer(layer, &mut state)?;
            }
        }
        println!("> Done.");

        Ok(())
    }

    fn execute_layer(
        &self,
        layer: HashSet<&Utf8Path>,
        state: &mut ExecutionState
    ) -> Result<(), BndBuilderError> {
        layer
            .into_iter()
            .map(|p| self.execute_rule(&p, state))
            .collect::<Result<Vec<()>, BndBuilderError>>()?;
        Ok(())
    }

    fn execute_rule<P: AsRef<Utf8Path>>(
        &self,
        p: P,
        state: &mut ExecutionState
    ) -> Result<(), BndBuilderError> {
        let p = p.as_ref();
        state.task_count += 1;
        println!("[{}/{}] Handle {}", state.task_count, state.nb_deps, p);

        if let Some(&rule_idx) = self.node2tracked.get(p) {
            let rule = self.tracked.rule_at(rule_idx);

            if !rule.is_enabled() {
                return Err(BndBuilderError::DisabledTarget(p.to_string()));
            }

            let done = rule.is_up_to_date();
            if done {
                println!("\t{} is already up to date", p);
            }
            else {
                for task in rule.commands() {
                    execute(task).map_err(|e| {
                        BndBuilderError::ExecuteError {
                            fname: p.to_string(),
                            msg: e
                        }
                    })?;
                }
            }
        }
        else {
            if !p.exists() {
                return Err(BndBuilderError::ExecuteError {
                    fname: p.to_string(),
                    msg: "no rule to build it".to_owned()
                });
            }
            else {
                println!("\t{} is already up to date", p)
            }
        }

        Ok(())
    }
}

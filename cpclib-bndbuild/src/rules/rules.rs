use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::ops::Sub;

use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use dot_writer::{Attributes, DotWriter, Style};
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

        self.rules
            .iter()
            .find(|r| r.targets().iter().any(|tgt2| tgt2 == &tgt))
    }

    pub fn default_target(&self) -> Option<&Utf8Path> {
        self.rules.get(0).map(|r| r.target(0))
    }

    // TODO implement a version with less copy
    pub fn to_deps(&self) -> Result<Graph, BndBuilderError> {
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
                        format! {"{} has already a rule to build it:\n{:?}", p, other_rule}
                    ));
                }
                else {
                    node2tracked_idx.insert(p, idx);
                }

                // link the target to the dependencies
                for p2 in rule.dependencies().iter() {
                    g.depend_on(p, p2).map_err(|_e| {
                        BndBuilderError::DependencyError(format!("{} and {}", p, p2))
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

impl Rules {
    pub fn to_dot(&self) -> String {
        let mut output_bytes = Vec::new();
        {
            let mut writer = DotWriter::from(&mut output_bytes);
            writer.set_pretty_print(true);

            let mut digraph = writer.digraph();
            digraph
                .set_rank_direction(dot_writer::RankDirection::BottomTop)
                .node_attributes()
                .set_style(Style::Filled)
                .set_shape(dot_writer::Shape::Rectangle);

            let mut all_deps = HashSet::<String>::default();
            let mut all_tgts = HashSet::<String>::default();

            for rule in self.rules() {
                let deps = rule.dependencies();
                let tgts = rule.targets();
                let cmd = rule
                    .commands()
                    .iter()
                    .map(|cmd| cmd.to_string())
                    .join("\\l");

                let mut rule_id = if deps.is_empty() {
                    None
                }
                else {
                    let mut rule_node = digraph.node_auto();
                    if cmd.is_empty() {
                        rule_node.set("shape", "point", false);
                    }
                    else {
                        rule_node.set_label(&cmd);
                    }
                    Some(rule_node.id())
                };

                for dep in deps {
                    let dep = format!("\"{}\"", dep.to_string());
                    all_deps.insert(dep.clone());

                    rule_id
                        .as_mut()
                        .map(|rule_id| digraph.edge(&dep, rule_id.clone()));
                }

                for tgt in tgts {
                    let tgt = format!("\"{}\"", tgt.to_string());
                    all_tgts.insert(tgt.clone());

                    rule_id
                        .as_mut()
                        .map(|rule_id| digraph.edge(rule_id.clone(), &tgt));
                }
            }

            for tgt in all_tgts.iter() {
                digraph
                    .node_named(tgt)
                    .set_font_color(dot_writer::Color::Blue)
                    .set("style", "rounded", false);
            }

            for tgt in all_tgts.sub(&all_deps) {
                digraph
                    .node_named(&tgt)
                    .set_font_color(dot_writer::Color::Red)
                    .set("shape", "folder", false);
            }

            for dep in all_deps.sub(&all_tgts) {
                digraph
                    .node_named(&dep)
                    .set("fontcolor", "darkgreen", false)
                    .set("shape", "cylinder", false)
                    .set_style(Style::Rounded);
            }
        }

        String::from_utf8_lossy(&output_bytes).into_owned()
    }
}

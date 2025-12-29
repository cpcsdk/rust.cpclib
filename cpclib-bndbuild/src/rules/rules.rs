use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::ops::{Deref, DerefMut, Sub};

use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use dot_writer::{Attributes, DotWriter, Node, Scope, Style};
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

impl Rules {
    /// Generate a graphvis representation of the build
    /// If compressed is activated, the rules are not shown
    pub fn to_dot(&self, compressed: bool) -> String {
        let mut output_bytes = Vec::new();
        {
            let mut writer = DotWriter::from(&mut output_bytes);
            writer.set_pretty_print(true);

            let mut digraph = writer.digraph();
            digraph
                .set_rank_direction(dot_writer::RankDirection::BottomTop)
                .node_attributes()
                .set_style(Style::Filled)
                .set_font_size(12.0)
                .set_shape(dot_writer::Shape::Rectangle);

            let mut all_deps = HashSet::<String>::default();
            let mut all_tgts = HashSet::<String>::default();

            // loop over each rule
            for rule in self.rules() {
                let deps = rule.dependencies();
                let tgts = rule.targets();

                let cmd = rule
                    .commands()
                    .iter()
                    .map(|cmd| {
                        cmd.to_string()
                            .replace("\\", "\\\\")
                            .replace("\"", "\\\"")
                            .lines()
                            .map(|l| l.trim())
                            .join(" \\\\\\l  ")
                    })
                    .join(" \\l");
                let cmd = if cmd.contains("\\l") {
                    cmd + " \\l"
                }
                else {
                    cmd
                };

                let build_rule_node =
                    for<'a, 'b, 'c> |digraph: &'b mut Scope<'a, 'c>| -> Node<'b, 'c> {
                        let mut rule_node = digraph.node_auto();
                        if let Some(help) = rule.help() {
                            rule_node.set("tooltip", help, true);
                        }
                        rule_node
                    };
                let complete_rule_node = |cmd: &str, rule_node: &mut Node| {
                    debug_assert!(!cmd.is_empty());
                    if compressed {
                        rule_node.set("shape", "point", false);
                        rule_node.set("tooltip", cmd, true);
                    }
                    else {
                        rule_node.set_label(cmd);
                        rule_node.set_font("Courier New");
                        rule_node.set_font_size(12.);
                    }
                };

                let mut rule_id = if deps.is_empty() {
                    if cmd.is_empty() {
                        None
                    }
                    else {
                        let id = {
                            let mut rule_node = build_rule_node(&mut digraph);
                            complete_rule_node(&cmd, &mut rule_node);
                            rule_node.id()
                        };

                        // we want the tule node to be aligned with the other ones that depend on files
                        // so we add an extra hidden node
                        let fill_id = {
                            let mut fill_space_node = digraph.node_auto();
                            fill_space_node.set_style(Style::Invisible);
                            fill_space_node.id()
                        };

                        let e = digraph.edge(&fill_id, &id);
                        e.attributes().set_style(Style::Invisible);

                        Some(id)
                    }
                }
                else {
                    let mut rule_node = build_rule_node(&mut digraph);
                    if cmd.is_empty() {
                        rule_node.set("shape", "point", false);
                    }
                    else {
                        complete_rule_node(&cmd, &mut rule_node);
                    }

                    Some(rule_node.id())
                };

                for dep in deps {
                    let dep = format!("\"{dep}\"");
                    all_deps.insert(dep.clone());

                    if let Some(rule_id) = rule_id.as_mut() {
                        let _e = digraph.edge(&dep, rule_id.clone());
                    }
                }

                for tgt in tgts {
                    let tgt = format!("\"{tgt}\"");
                    all_tgts.insert(tgt.clone());

                    rule_id
                        .as_mut()
                        .map(|rule_id| digraph.edge(rule_id.clone(), &tgt));
                }
            }

            for tgt in all_tgts.iter() {
                digraph
                    .node_named(tgt)
                    .set(
                        "URL",
                        &format!("bndbuild://{}", tgt.replace("\"", "")),
                        true
                    )
                    .set_font_color(dot_writer::Color::Blue)
                    .set("style", "rounded", false);
            }

            for tgt in all_tgts.sub(&all_deps) {
                digraph
                    .node_named(&tgt)
                    .set_font_color(dot_writer::Color::Red)
                    .set("shape", "folder", false);
            }

            {
                let mut cluster = digraph.cluster();
                cluster.set_style(Style::Invisible);
                for dep in all_deps.sub(&all_tgts) {
                    cluster
                        .node_named(&dep)
                        .set("fontcolor", "darkgreen", false)
                        .set("shape", "cylinder", false)
                        .set_style(Style::Rounded);
                }
            }
        }

        String::from_utf8_lossy(&output_bytes).into_owned()
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

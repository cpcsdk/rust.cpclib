//! Graphviz / DOT output for build rules.
//!
//! This module provides two entry-points on [`Rules`]:
//! * [`Rules::to_dot`]       – single-file graph (legacy).
//! * [`Rules::to_dot_multi`] – multi-file graph with cross-file edges.

use std::collections::{HashMap, HashSet};
use std::ops::Sub;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use dot_writer::{Attributes, DotWriter, Node, Scope, Style};

use super::Rules;
use crate::env::create_template_env;
use crate::task::InnerTask;
use crate::EXPECTED_FILENAMES;

/// Parse a bndbuild argument string to extract `-f`/`--file` FILE and
/// any positional target arguments.  Returns `(file_path, targets)` when
/// the `-f` flag is present; `None` otherwise.
fn parse_bndbuild_file_ref(args: &str) -> Option<(String, Vec<String>)> {
    let tokens: Vec<String> = shlex::split(args)
        .unwrap_or_else(|| args.split_whitespace().map(str::to_owned).collect());
    let mut file: Option<String> = None;
    let mut targets: Vec<String> = Vec::new();
    let mut iter = tokens.into_iter();
    while let Some(tok) = iter.next() {
        match tok.as_str() {
            "-f" | "--file" => {
                if let Some(next) = iter.next() {
                    file = Some(next);
                }
            },
            s if !s.starts_with('-') => targets.push(s.to_owned()),
            _ => {}
        }
    }
    file.map(|f| (f, targets))
}

impl Rules {
    /// Generate a graphvis representation of the build.
    /// If `compressed` is `true`, the rules are not shown.
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

                        // we want the rule node to be aligned with the other ones that depend on files
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

    /// Generate a multi-file graphviz representation of the build.
    ///
    /// Each distinct build file is drawn in its own `subgraph cluster`.
    /// `bndbuild -f FILE` tasks are detected; their target nodes are linked
    /// back to the calling rule with a dashed purple edge so that cross-file
    /// dependencies are immediately visible.
    ///
    /// Falls back to [`Rules::to_dot`] when `source_file` is `None`.
    pub fn to_dot_multi(&self, source_file: Option<&Utf8Path>, compressed: bool) -> String {
        let source_file = match source_file {
            Some(f) => f,
            None => return self.to_dot(compressed)
        };

        // ----------------------------------------------------------------
        // Phase 1 – BFS collection of per-file rule data
        // ----------------------------------------------------------------

        // All data extracted into plain owned types so we never need Rules:Clone.
        struct PRule {
            targets: Vec<String>,
            deps: Vec<String>,
            cmd: String,
            help: Option<String>,
            /// (referenced_file_path, explicit_targets) from every BndBuild task
            bndbuild_refs: Vec<(String, Vec<String>)>
        }
        struct PFile {
            label: String,
            base_dir: Utf8PathBuf,
            rules: Vec<PRule>
        }

        fn make_prules(rules: &Rules) -> Vec<PRule> {
            rules.rules().iter().map(|rule| {
                let targets = rule.targets().iter().map(|t| t.to_string()).collect();
                let deps = rule.dependencies().iter().map(|d| d.to_string()).collect();
                let cmd = rule
                    .commands()
                    .iter()
                    .map(|c| {
                        c.to_string()
                            .replace("\\", "\\\\")
                            .replace("\"", "\\\"")
                            .lines()
                            .map(|l| l.trim())
                            .join(" \\\\\\l  ")
                    })
                    .join(" \\l");
                let cmd = if cmd.contains("\\l") { cmd + " \\l" } else { cmd };
                let help = rule.help().map(str::to_owned);
                let bndbuild_refs = rule
                    .commands()
                    .iter()
                    .filter_map(|task| {
                        if let InnerTask::BndBuild(args) = &task.inner {
                            parse_bndbuild_file_ref(args.args())
                        } else {
                            None
                        }
                    })
                    .collect();
                PRule { targets, deps, cmd, help, bndbuild_refs }
            }).collect()
        }

        // Resolve a path, converting directories to their contained build file,
        // then canonicalize to remove any `..` / `.` components so that two
        // relative routes to the same file (e.g. "a/../b/build.bnd" vs "b/build.bnd")
        // produce the same string key and are correctly deduplicated during BFS.
        let resolve_and_canon = |p: Utf8PathBuf| -> String {
            let resolved = if p.is_dir() {
                EXPECTED_FILENAMES
                    .iter()
                    .map(|f| p.join(f))
                    .find(|candidate| candidate.is_file())
                    .unwrap_or(p)
            } else {
                p
            };
            std::fs::canonicalize(&resolved)
                .ok()
                .and_then(|c| Utf8PathBuf::from_path_buf(c).ok())
                .map(|c| c.into_string())
                .unwrap_or_else(|| resolved.into_string())
        };

        // Resolve the root file to an absolute path string used as the BFS key.
        let source_abs: String = {
            let raw = if source_file.is_absolute() {
                Utf8PathBuf::from(source_file)
            } else {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
                    .map(|cwd| cwd.join(source_file))
                    .unwrap_or_else(|| Utf8PathBuf::from(source_file))
            };
            resolve_and_canon(raw)
        };

        let mut files: Vec<PFile> = Vec::new();
        let mut path_to_idx: HashMap<String, usize> = HashMap::new();
        // queue entries: (absolute_path_string, base_dir_for_resolving_relative_refs)
        let mut bfs_queue: Vec<(String, Utf8PathBuf)> = Vec::new();
        {
            let p = Utf8PathBuf::from(&source_abs);
            let base = p.parent().map(|d| d.to_owned()).unwrap_or_else(|| Utf8PathBuf::from("."));
            bfs_queue.push((source_abs.clone(), base));
        }

        let mut qi = 0;
        while qi < bfs_queue.len() {
            let (abs_path, base_dir) = bfs_queue[qi].clone();
            qi += 1;

            if path_to_idx.contains_key(&abs_path) {
                continue;
            }
            let file_idx = files.len();
            path_to_idx.insert(abs_path.clone(), file_idx);

            // Helper: compute a human-readable cluster label.
            // When the build file has a default name (bndbuild.yml, build.bnd, …)
            // use its parent directory name so that e.g. "linking/build.bnd"
            // shows as "linking" rather than "build.bnd" in every cluster.
            let make_label = |path: &str| -> String {
                let p = Utf8PathBuf::from(path);
                let fname = p.file_name().unwrap_or(path);
                if EXPECTED_FILENAMES.contains(&fname) {
                    p.parent()
                        .and_then(|d| d.file_name())
                        .map(str::to_owned)
                        .unwrap_or_else(|| fname.to_owned())
                } else {
                    fname.to_owned()
                }
            };

            let prules: Vec<PRule> = if file_idx == 0 {
                make_prules(self)
            } else {
                // Use the Jinja template engine (same as normal loading) so that
                // files using {%include%} / {{…}} are rendered before YAML parsing.
                let rendered_opt: Option<String> = (|| {
                    let path = Utf8PathBuf::from(&abs_path);
                    let parent = path.parent()?;
                    std::env::set_current_dir(parent).ok()?;
                    let content = std::fs::read_to_string(&path).ok()?;
                    let env = create_template_env(
                        Some(parent),
                        &[] as &[(&str, &str)]
                    );
                    env.render_str(&content, minijinja::context!()).ok()
                })();
                match rendered_opt.and_then(|content| serde_yaml::from_str::<Rules>(&content).ok()) {
                    Some(r) => make_prules(&r),
                    None => {
                        // File cannot be loaded: record an empty cluster and move on.
                        files.push(PFile {
                            label: make_label(&abs_path),
                            base_dir,
                            rules: Vec::new()
                        });
                        continue;
                    }
                }
            };

            // Enqueue all child files that haven't been seen yet.
            for pr in &prules {
                for (ref_file, _ref_tgts) in &pr.bndbuild_refs {
                    let raw_abs = if Utf8Path::new(ref_file).is_absolute() {
                        Utf8PathBuf::from(ref_file)
                    } else {
                        base_dir.join(ref_file)
                    };
                    let ref_abs = resolve_and_canon(raw_abs);
                    if !path_to_idx.contains_key(&ref_abs) {
                        let ref_path = Utf8PathBuf::from(&ref_abs);
                        let ref_base = ref_path
                            .parent()
                            .map(|d| d.to_owned())
                            .unwrap_or_else(|| Utf8PathBuf::from("."));
                        bfs_queue.push((ref_abs, ref_base));
                    }
                }
            }

            files.push(PFile { label: make_label(&abs_path), base_dir, rules: prules });
        }

        // ----------------------------------------------------------------
        // Phase 2 – Generate DOT
        // ----------------------------------------------------------------

        // rule_node_ids[file_idx][rule_idx] = Some(node_id_string) | None
        let mut rule_node_ids: Vec<Vec<Option<String>>> = Vec::new();

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

            for (file_idx, pfile) in files.iter().enumerate() {
                let mut ids_for_file: Vec<Option<String>> = Vec::new();
                let mut all_deps = HashSet::<String>::default();
                let mut all_tgts = HashSet::<String>::default();
                // Prefix applied to all named (target/dep) nodes in this file.
                // This avoids name collisions across files (graphviz IDs), but
                // the prefix is stripped for the human-readable label.
                let node_prefix = format!("f{}:", file_idx);
                let strip_prefix = |s: &str| -> String {
                    let inner = s.trim_matches('"');
                    inner.strip_prefix(&*node_prefix).unwrap_or(inner).to_owned()
                };

                {
                    let mut cluster = digraph.cluster();
                    cluster.set_label(&pfile.label);

                    for pr in pfile.rules.iter() {
                        let cmd = &pr.cmd;

                        if pr.deps.is_empty() && cmd.is_empty() {
                            ids_for_file.push(None);
                            continue;
                        }

                        if pr.deps.is_empty() {
                            // No incoming deps: create the rule node …
                            let rid_str: String = {
                                let mut rn = cluster.node_auto();
                                if compressed {
                                    rn.set("shape", "point", false);
                                    rn.set("tooltip", cmd, true);
                                } else {
                                    rn.set_label(cmd);
                                    rn.set_font("Courier New");
                                    rn.set_font_size(12.);
                                }
                                if let Some(help) = &pr.help {
                                    rn.set("tooltip", help, true);
                                }
                                rn.id().into()
                            };
                            // … plus an invisible fill node to keep the layout aligned.
                            let fid_str: String = {
                                let mut fn_node = cluster.node_auto();
                                fn_node.set_style(Style::Invisible);
                                fn_node.id().into()
                            };
                            {
                                let e = cluster.edge(&fid_str, &rid_str);
                                e.attributes().set_style(Style::Invisible);
                            }
                            for tgt in &pr.targets {
                                let tgt_node = format!("\"{}{}\"", node_prefix, tgt);
                                all_tgts.insert(tgt_node.clone());
                                cluster.edge(&rid_str, &tgt_node);

                            }
                            ids_for_file.push(Some(rid_str));
                        } else {
                            let rid_str: String = {
                                let mut rn = cluster.node_auto();
                                if cmd.is_empty() {
                                    rn.set("shape", "point", false);
                                } else if compressed {
                                    rn.set("shape", "point", false);
                                    rn.set("tooltip", cmd, true);
                                } else {
                                    rn.set_label(cmd);
                                    rn.set_font("Courier New");
                                    rn.set_font_size(12.);
                                }
                                if let Some(help) = &pr.help {
                                    rn.set("tooltip", help, true);
                                }
                                rn.id().into()
                            };
                            for dep in &pr.deps {
                                let dep_node = format!("\"{}{}\"", node_prefix, dep);
                                all_deps.insert(dep_node.clone());
                                cluster.edge(&dep_node, &rid_str);
                            }
                            for tgt in &pr.targets {
                                let tgt_node = format!("\"{}{}\"", node_prefix, tgt);
                                all_tgts.insert(tgt_node.clone());
                                cluster.edge(&rid_str, &tgt_node);

                            }
                            ids_for_file.push(Some(rid_str));
                        }
                    }

                    // Style all target nodes.
                    for tgt in all_tgts.iter() {
                        let label = strip_prefix(tgt);
                        cluster
                            .node_named(tgt)
                            .set(
                                "URL",
                                &format!("bndbuild://{}", tgt.replace("\"", "")),
                                true
                            )
                            .set_font_color(dot_writer::Color::Blue)
                            .set("style", "rounded", false)
                            .set_label(&label);
                    }
                    for tgt in all_tgts.sub(&all_deps) {
                        let label = strip_prefix(&tgt);
                        cluster
                            .node_named(&tgt)
                            .set_font_color(dot_writer::Color::Red)
                            .set("shape", "folder", false)
                            .set_label(&label);
                    }
                    // Leaf deps (no rule produces them) in an inner invisible cluster.
                    {
                        let mut leaf = cluster.cluster();
                        leaf.set_style(Style::Invisible);
                        for dep in all_deps.sub(&all_tgts) {
                            let label = strip_prefix(&dep);
                            leaf.node_named(&dep)
                                .set("fontcolor", "darkgreen", false)
                                .set("shape", "cylinder", false)
                                .set_style(Style::Rounded)
                                .set_label(&label);
                        }
                    }
                } // cluster dropped — digraph is usable again

                rule_node_ids.push(ids_for_file);
            }

            // ---- Cross-file edges (dashed, purple) ----
            for (file_idx, pfile) in files.iter().enumerate() {
                for (rule_idx, pr) in pfile.rules.iter().enumerate() {
                    let rule_id = match rule_node_ids
                        .get(file_idx)
                        .and_then(|v| v.get(rule_idx))
                        .and_then(|o| o.as_ref())
                    {
                        Some(id) => id,
                        None => continue
                    };
                    for (ref_file, ref_targets) in &pr.bndbuild_refs {
                        let raw_abs = if Utf8Path::new(ref_file).is_absolute() {
                            Utf8PathBuf::from(ref_file)
                        } else {
                            pfile.base_dir.join(ref_file)
                        };
                        let ref_abs = resolve_and_canon(raw_abs);
                        let ref_file_idx = match path_to_idx.get(&ref_abs) {
                            Some(&i) => i,
                            None => continue
                        };
                        let ref_pfile = &files[ref_file_idx];
                        // Determine which target names to look up in the referenced file.
                        let targets_to_link: Vec<String> = if ref_targets.is_empty() {
                            // No explicit target → use the first (default) target of that file.
                            ref_pfile
                                .rules
                                .first()
                                .and_then(|r| r.targets.first())
                                .cloned()
                                .into_iter()
                                .collect()
                        } else {
                            ref_targets.clone()
                        };
                        // For each target: link from the target node in the referenced file
                        // to the calling rule node in this file.
                        for target in &targets_to_link {
                            let src_node = format!("\"f{}:{}\"", ref_file_idx, target);
                            let e = digraph.edge(&src_node, rule_id);
                            e.attributes()
                                .set("style", "dashed", false)
                                .set("color", "purple", false);
                        }
                    }
                }
            }
        }

        String::from_utf8_lossy(&output_bytes).into_owned()
    }
}

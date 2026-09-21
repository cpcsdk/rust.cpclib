//! `measure_variants` - rebuild a project once per set of text edits and
//! report a real size metric for each, as a table a human can compare.
//!
//! The general form of what `compare_link_sizes` does for one selector
//! variable: any number of edits across any number of files per variant,
//! and a metric that is either the size of an output file (e.g. the final
//! `.RAN`/`.BIN`, the only number that matters for a size-limited intro)
//! or the `.sym`-derived linked size. This exists because ideas from the
//! static tools (peephole suggestions, reorderings, a different data
//! encoding, a feature switch) are only worth anything if the *real* build
//! agrees - crunchers such as Shrinkler quantise their output and do not
//! follow the proxy compressors this server can run in-process.

use std::collections::HashMap;

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};
use crate::tools::build::{
    ReportBuildInput, RestoreFileOnDrop, one_line_error, report_build, run_target_quiet
};

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct TextEdit {
    /// File to edit.
    pub path: String,
    /// Exact text to find. Must occur **exactly once** in the file - an
    /// ambiguous or missing match fails that variant instead of guessing.
    pub find: String,
    /// Replacement text.
    pub replace: String
}

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct Variant {
    /// Name shown in the table.
    pub name: String,
    /// Edits applied together, to a pristine copy of every file.
    pub edits: Vec<TextEdit>
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MeasureVariantsInput {
    /// Path to the build file (e.g. `build.bnd`/`bndbuild.yml`) or a
    /// directory containing one.
    pub bnd_path: String,
    /// Target to build; defaults to the build file's own default target.
    pub target: Option<String>,
    /// Metric option 1: size in bytes of this output file after the build
    /// (e.g. the final `GPT-LEC.RAN`). Give exactly one of `output_file` /
    /// `sym_path`.
    pub output_file: Option<String>,
    /// Metric option 2: a `.sym` file; the metric is `last - first` as in
    /// `report_build`.
    pub sym_path: Option<String>,
    /// The variants to try.
    pub variants: Vec<Variant>,
    /// Also build and measure the unmodified project first, as the row
    /// every other row is compared against. Default true.
    pub include_baseline: Option<bool>
}

#[derive(Debug, Clone)]
struct VariantRow {
    name: String,
    is_baseline: bool,
    metric: Option<i64>,
    duration_ms: Option<u128>,
    error: Option<String>
}

/// Applies `edits` to `originals` (path -> pristine content), returning the
/// new content of every touched file, or why it can't be done. Every `find`
/// must match exactly once.
fn apply_edits(
    originals: &HashMap<String, String>,
    edits: &[TextEdit]
) -> Result<HashMap<String, String>, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    for edit in edits {
        let current = out
            .get(&edit.path)
            .or_else(|| originals.get(&edit.path))
            .ok_or_else(|| format!("no such file loaded: {}", edit.path))?;
        if edit.find.is_empty() {
            return Err("an edit's `find` must not be empty".to_string());
        }
        match current.matches(edit.find.as_str()).count() {
            1 => {},
            0 => return Err(format!("`{}` not found in {}", edit.find, edit.path)),
            n => return Err(format!("`{}` found {n} times in {} (must be unique)", edit.find, edit.path))
        }
        let replaced = current.replacen(&edit.find, &edit.replace, 1);
        out.insert(edit.path.clone(), replaced);
    }
    Ok(out)
}

/// Markdown table, smallest metric first (rows must already be sorted):
/// rank, variant, metric, delta against the baseline, delta against the
/// best, build seconds. Failed variants are listed last with their error.
fn render_variants_table(rows: &[VariantRow]) -> String {
    let baseline = rows.iter().find(|r| r.is_baseline).and_then(|r| r.metric);
    let best = rows.iter().filter_map(|r| r.metric).min();
    let signed = |d: i64| if d > 0 { format!("+{d}") } else { d.to_string() };
    let mut out = String::from(
        "| # | Variant | Size | vs baseline | vs best | Build (s) |\n|--:|:--|--:|--:|--:|--:|\n"
    );
    let mut rank = 0;
    for r in rows {
        let name = if r.is_baseline {
            format!("{} (baseline)", r.name)
        }
        else {
            r.name.clone()
        };
        let secs = r.duration_ms.map_or("-".to_string(), |ms| format!("{:.1}", ms as f64 / 1000.0));
        match r.metric {
            Some(m) => {
                rank += 1;
                let vs_base = match (baseline, r.is_baseline) {
                    (_, true) => "-".to_string(),
                    (Some(b), false) => signed(m - b),
                    (None, false) => "-".to_string()
                };
                let vs_best = if Some(m) == best { "best".to_string() } else { signed(m - best.unwrap_or(m)) };
                out.push_str(&format!("| {rank} | {name} | {m} | {vs_base} | {vs_best} | {secs} |\n"));
            },
            None => {
                let reason = r.error.as_deref().map_or("no result".to_string(), one_line_error);
                out.push_str(&format!("| - | {name} | FAILED: {reason} | | | {secs} |\n"));
            }
        }
    }
    out
}

fn measure_once(input: &MeasureVariantsInput) -> Result<(i64, u128), ToolError> {
    match (&input.output_file, &input.sym_path) {
        (Some(file), None) => {
            let ms = run_target_quiet(&input.bnd_path, input.target.as_deref())?;
            let size = fs_err::metadata(file)
                .map_err(|e| ToolError::io(format!("build succeeded but cannot stat {file}: {e}")))?
                .len();
            Ok((size as i64, ms))
        },
        (None, Some(sym)) => {
            let report = report_build(ReportBuildInput {
                bnd_path: input.bnd_path.clone(),
                target: input.target.clone(),
                sym_path: sym.clone(),
                target_size: None,
                overhead_bytes: None,
                verbose: Some(false)
            })?;
            let size = report["total_linked_size"].as_i64().ok_or_else(|| {
                ToolError::io("the .sym file has no `first`/`last` labels to derive a size from")
            })?;
            Ok((size, report["duration_ms"].as_u64().unwrap_or(0) as u128))
        },
        _ => unreachable!("validated up front")
    }
}

pub(crate) fn measure_variants(input: MeasureVariantsInput) -> ToolResult {
    if input.output_file.is_some() == input.sym_path.is_some() {
        return Err(ToolError::invalid_input("give exactly one of `output_file` / `sym_path`"));
    }
    if input.variants.is_empty() {
        return Err(ToolError::invalid_input("`variants` must not be empty"));
    }

    // Pristine content of every file any variant touches, restored on every
    // exit path.
    let mut originals: HashMap<String, String> = HashMap::new();
    for edit in input.variants.iter().flat_map(|v| &v.edits) {
        if !originals.contains_key(&edit.path) {
            let text = fs_err::read_to_string(&edit.path)
                .map_err(|e| ToolError::io(format!("cannot read {}: {e}", edit.path)))?;
            originals.insert(edit.path.clone(), text);
        }
    }
    let _guards: Vec<RestoreFileOnDrop> = originals
        .iter()
        .map(|(path, text)| RestoreFileOnDrop {
            path: path.as_str(),
            original: text.clone()
        })
        .collect();

    let write_all = |contents: &HashMap<String, String>| -> Result<(), ToolError> {
        for (path, text) in contents {
            fs_err::write(path, text).map_err(|e| ToolError::io(format!("cannot write {path}: {e}")))?;
        }
        Ok(())
    };

    let mut rows: Vec<VariantRow> = Vec::new();
    let mut run = |name: &str, is_baseline: bool, edits: &[TextEdit]| -> Result<(), ToolError> {
        // Every variant starts from pristine files.
        write_all(&originals)?;
        let row = match apply_edits(&originals, edits) {
            Err(e) => {
                VariantRow {
                    name: name.to_string(),
                    is_baseline,
                    metric: None,
                    duration_ms: None,
                    error: Some(e)
                }
            },
            Ok(changed) => {
                write_all(&changed)?;
                match measure_once(&input) {
                    Ok((metric, ms)) => {
                        VariantRow {
                            name: name.to_string(),
                            is_baseline,
                            metric: Some(metric),
                            duration_ms: Some(ms),
                            error: None
                        }
                    },
                    Err(e) => {
                        VariantRow {
                            name: name.to_string(),
                            is_baseline,
                            metric: None,
                            duration_ms: None,
                            error: Some(e.message)
                        }
                    }
                }
            }
        };
        rows.push(row);
        Ok(())
    };

    if input.include_baseline.unwrap_or(true) {
        run("baseline", true, &[])?;
    }
    for variant in &input.variants {
        run(&variant.name, false, &variant.edits)?;
    }

    rows.sort_by_key(|r| r.metric.unwrap_or(i64::MAX));
    let table = render_variants_table(&rows);
    let baseline = rows.iter().find(|r| r.is_baseline).and_then(|r| r.metric);
    let results: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "variant": r.name,
                "baseline": r.is_baseline,
                "ok": r.metric.is_some(),
                "size": r.metric,
                "vs_baseline": r.metric.zip(baseline).map(|(m, b)| m - b),
                "duration_ms": r.duration_ms.map(|d| d as u64),
                "error": r.error
            })
        })
        .collect();
    Ok(json!({ "bnd_path": input.bnd_path, "table": table, "results": results }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = variants_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "MUTATING (transiently only - every touched file is always restored, \
                           even on error): rebuilds the real project once per variant (a named \
                           set of exact-text edits across any files) and measures a real size \
                           - the size of an output file such as the final .RAN/.BIN \
                           (`output_file`), or the .sym-derived linked size (`sym_path`) - \
                           returning a markdown `table` (rank, size, delta vs the unmodified \
                           baseline and vs the best, build time) to show a human as-is. Use it to \
                           check ideas from suggest_optimizations / search_reorderings / a \
                           feature switch against the real cruncher: proxy compressors do not \
                           predict quantised crunchers like Shrinkler. Each `find` must occur \
                           exactly once in its file, otherwise that variant fails (nothing is \
                           guessed). For a single cruncher-selection variable prefer \
                           compare_link_sizes.")]
    async fn measure_variants(
        &self,
        Parameters(input): Parameters<MeasureVariantsInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(measure_variants(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn originals() -> HashMap<String, String> {
        HashMap::from([
            ("a.asm".to_string(), "ld hl,1\nld hl,1\nxor 255\n".to_string()),
            ("b.asm".to_string(), "nop\n".to_string())
        ])
    }

    fn edit(path: &str, find: &str, replace: &str) -> TextEdit {
        TextEdit {
            path: path.to_string(),
            find: find.to_string(),
            replace: replace.to_string()
        }
    }

    #[test]
    fn edits_across_files_apply_together() {
        let out = apply_edits(&originals(), &[edit("a.asm", "xor 255", "cpl"), edit("b.asm", "nop", "halt")])
            .unwrap();
        assert_eq!(out["a.asm"], "ld hl,1\nld hl,1\ncpl\n");
        assert_eq!(out["b.asm"], "halt\n");
    }

    #[test]
    fn two_edits_to_one_file_compose() {
        let out = apply_edits(&originals(), &[edit("a.asm", "xor 255", "cpl"), edit("a.asm", "cpl", "neg")])
            .unwrap();
        assert_eq!(out["a.asm"], "ld hl,1\nld hl,1\nneg\n");
    }

    #[test]
    fn an_ambiguous_or_missing_find_fails_instead_of_guessing() {
        let err = apply_edits(&originals(), &[edit("a.asm", "ld hl,1", "nop")]).unwrap_err();
        assert!(err.contains("2 times"), "{err}");
        let err = apply_edits(&originals(), &[edit("a.asm", "absent", "x")]).unwrap_err();
        assert!(err.contains("not found"), "{err}");
        let err = apply_edits(&originals(), &[edit("a.asm", "", "x")]).unwrap_err();
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn the_metric_source_must_be_exactly_one() {
        let base = |output_file: Option<&str>, sym_path: Option<&str>| MeasureVariantsInput {
            bnd_path: "b".to_string(),
            target: None,
            output_file: output_file.map(String::from),
            sym_path: sym_path.map(String::from),
            variants: vec![Variant {
                name: "v".to_string(),
                edits: vec![]
            }],
            include_baseline: None
        };
        assert_eq!(measure_variants(base(None, None)).unwrap_err().kind, "invalid_input");
        assert_eq!(measure_variants(base(Some("o"), Some("s"))).unwrap_err().kind, "invalid_input");
    }

    #[test]
    fn the_table_compares_against_the_baseline_and_the_best() {
        let rows = vec![
            VariantRow { name: "cpl".into(), is_baseline: false, metric: Some(3964), duration_ms: Some(1200), error: None },
            VariantRow { name: "baseline".into(), is_baseline: true, metric: Some(3968), duration_ms: Some(1100), error: None },
            VariantRow { name: "worse".into(), is_baseline: false, metric: Some(3976), duration_ms: Some(1300), error: None },
            VariantRow { name: "broken".into(), is_baseline: false, metric: None, duration_ms: None, error: Some("boom | x".into()) },
        ];
        let table = render_variants_table(&rows);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines[2], "| 1 | cpl | 3964 | -4 | best | 1.2 |", "{table}");
        assert_eq!(lines[3], "| 2 | baseline (baseline) | 3968 | - | +4 | 1.1 |", "{table}");
        assert_eq!(lines[4], "| 3 | worse | 3976 | +8 | +12 | 1.3 |", "{table}");
        assert!(lines[5].contains("FAILED: boom / x"), "{table}");
    }
}

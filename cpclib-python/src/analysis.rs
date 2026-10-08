//! Static analysis of basm sources: diagnostics, NOP counts, peephole
//! optimisation - what the language server and the MCP server do for an editor,
//! for a script.
//!
//! ```python
//! from cpclib_python import analysis
//!
//! analysis.check(code="ld a,0\nld b,a\n")["ok"]
//! analysis.count_nops(code="ld a,1\ninc a\n", start_line=1, end_line=2)
//! analysis.suggest_optimizations("main.asm", goal="speed")["suggestions"]
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::hash::{Hash, Hasher};

use cpclib_basmopt::{OptimizationGoal, Options};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_lsp::basm::AssemblyAnalyzer;
use cpclib_lsp::common::document::Document;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use serde_json::{Value, json};
use tower_lsp::lsp_types::{DiagnosticSeverity, Position, Range, Url};

/// A JSON value as the matching Python objects (dicts, lists, ints, ...).
fn to_py(py: Python, value: &Value) -> PyResult<Py<PyAny>> {
    Ok(py
        .import("json")?
        .call_method1("loads", (value.to_string(),))?
        .unbind())
}

fn document(path: Option<&str>, code: Option<&str>) -> PyResult<Document> {
    let uri_of = |path: &str| {
        let path = std::path::Path::new(path);
        let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        });
        Url::from_file_path(&absolute).map_err(|_| {
            PyValueError::new_err(format!("not a usable file path: {}", path.display()))
        })
    };
    let (text, uri) = match (path, code) {
        (Some(p), Some(c)) => (c.to_string(), uri_of(p)?),
        (Some(p), None) => {
            let text = std::fs::read_to_string(p)
                .map_err(|e| PyRuntimeError::new_err(format!("cannot read {p}: {e}")))?;
            (text, uri_of(p)?)
        },
        (None, Some(c)) => {
            (
                c.to_string(),
                Url::parse("file:///untitled.asm").expect("static URI is valid")
            )
        },
        (None, None) => {
            return Err(PyValueError::new_err(
                "either `path` or `code` must be given"
            ));
        }
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    Ok(Document::new(
        uri,
        text,
        (hasher.finish() & 0x7FFF_FFFF) as i32
    ))
}

/// Assembles a source for real and returns `{"ok": bool, "diagnostics": [...]}`:
/// the parse errors (several, not just the first) and the assembler's warnings,
/// as the language server reports them.
///
/// Give `code`, a `path` or both (the code is then checked as if it were the
/// file's, so that its `include`s resolve).
#[pyfunction]
#[pyo3(signature = (path=None, code=None))]
fn check(py: Python, path: Option<&str>, code: Option<&str>) -> PyResult<Py<PyAny>> {
    let document = document(path, code)?;
    let diagnostics = py.detach(|| AssemblyAnalyzer::new().analyze(&document));
    let ok = diagnostics
        .iter()
        .all(|d| d.severity != Some(DiagnosticSeverity::ERROR));
    let value = json!({
        "ok": ok,
        "diagnostics": serde_json::to_value(&diagnostics)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
    });
    to_py(py, &value)
}

/// A control-flow-aware NOP count - the minimum and maximum cost over the real
/// paths, not a naive sum - for the lines `start_line` to `end_line` (1-based,
/// inclusive) of the source (`path` and/or `code`, as for `check`).
///
/// Returns a dict with `min_nops`, `max_nops`, ... as the language server's
/// cycle counter does.
#[pyfunction]
#[pyo3(signature = (start_line, end_line, path=None, code=None))]
fn count_nops(
    py: Python,
    start_line: u32,
    end_line: u32,
    path: Option<&str>,
    code: Option<&str>
) -> PyResult<Py<PyAny>> {
    if start_line == 0 || end_line == 0 {
        return Err(PyValueError::new_err("lines are 1-based"));
    }
    if end_line < start_line {
        return Err(PyValueError::new_err("end_line must be >= start_line"));
    }
    let document = document(path, code)?;
    // the end character: the last of the line, so that `end_line` stays inclusive
    let range = Range {
        start: Position {
            line: start_line - 1,
            character: 0
        },
        end: Position {
            line: end_line - 1,
            character: u32::MAX
        }
    };
    let summary = py
        .detach(|| AssemblyAnalyzer::new().cycle_count_for_selection(&document, range))
        .ok_or_else(|| {
            PyRuntimeError::new_err(
                "could not compute a NOP count for this range - the source may not parse, or the \
                 range contains no recognised instruction"
            )
        })?;
    let value =
        serde_json::to_value(summary).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    to_py(py, &value)
}

fn options(
    goal: Option<&str>,
    disabled_rules: Option<Vec<String>>,
    include_dirs: Option<Vec<String>>,
    defines: Option<Vec<String>>
) -> PyResult<Options> {
    let goal = match goal.map(str::to_ascii_lowercase).as_deref() {
        None => OptimizationGoal::default(),
        Some("neutral") => OptimizationGoal::Neutral,
        Some("size") => OptimizationGoal::Size,
        Some("speed") => OptimizationGoal::Speed,
        Some(other) => {
            return Err(PyValueError::new_err(format!(
                "unknown goal '{other}' - expected one of: neutral, size, speed"
            )));
        }
    };
    Ok(Options {
        goal,
        disabled_rules: disabled_rules.unwrap_or_default(),
        include_dirs: include_dirs
            .unwrap_or_default()
            .into_iter()
            .map(Utf8PathBuf::from)
            .collect(),
        defines: defines.unwrap_or_default(),
        ..Default::default()
    })
}

fn suggestion(s: &cpclib_basmopt::Suggestion) -> Value {
    json!({
        "line": s.line,
        "column": s.column,
        "rule_name": s.rule_name,
        "bulk_unsafe": s.bulk_unsafe,
        "message": s.message,
        "replacement": s.replacement,
        "reasons": s.reasons.iter()
            .map(|r| json!({"text": r.text, "line": r.line, "column": r.column}))
            .collect::<Vec<_>>()
    })
}

/// The peephole optimisations the rules find in the file `path`:
/// `{"suggestion_count", "suggestions": [{"line", "column", "rule_name",
/// "message", "replacement", "reasons", "bulk_unsafe"}], "assemble_warning"}`.
/// Nothing is changed.
///
/// - `goal`: `neutral` (default: only what wins on both), `size` or `speed`;
/// - `disabled_rules`: rules to skip, by name;
/// - `include_dirs`, `defines`: as for the assembler (`NAME` or `NAME=VALUE`);
/// - `include_project`: also analyse every file `path` includes, each against
///   the whole project's real addresses.
#[pyfunction]
#[pyo3(signature = (path, goal=None, disabled_rules=None, include_dirs=None, defines=None, include_project=false))]
fn suggest_optimizations(
    py: Python,
    path: &str,
    goal: Option<&str>,
    disabled_rules: Option<Vec<String>>,
    include_dirs: Option<Vec<String>>,
    defines: Option<Vec<String>>,
    include_project: bool
) -> PyResult<Py<PyAny>> {
    let options = options(goal, disabled_rules, include_dirs, defines)?;
    let file = Utf8Path::new(path);

    let value = if include_project {
        let outcomes = py
            .detach(|| cpclib_basmopt::analyze_project(file, &options))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let files: Vec<Value> = outcomes
            .iter()
            .filter(|(_, o)| !o.suggestions.is_empty() || o.assemble_warning.is_some())
            .map(|(path, o)| {
                json!({
                    "path": path.as_str(),
                    "suggestion_count": o.suggestions.len(),
                    "suggestions": o.suggestions.iter().map(suggestion).collect::<Vec<_>>(),
                    "assemble_warning": o.assemble_warning
                })
            })
            .collect();
        json!({
            "entry": path,
            "files_analyzed": outcomes.len(),
            "files_with_findings": files.len(),
            "suggestion_count": outcomes.iter().map(|(_, o)| o.suggestions.len()).sum::<usize>(),
            "files": files
        })
    }
    else {
        let outcome = py
            .detach(|| cpclib_basmopt::analyze_file(file, &options))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        json!({
            "suggestion_count": outcome.suggestions.len(),
            "suggestions": outcome.suggestions.iter().map(suggestion).collect::<Vec<_>>(),
            "assemble_warning": outcome.assemble_warning
        })
    };
    to_py(py, &value)
}

/// **Rewrites `path`** with every bulk-safe optimisation applied (options as
/// for `suggest_optimizations`). Returns what was done: `{"passes_run",
/// "total_applied", "remaining_skipped", "assemble_warning"}`.
#[pyfunction]
#[pyo3(signature = (path, goal=None, disabled_rules=None, include_dirs=None, defines=None))]
fn apply_optimizations_in_place(
    py: Python,
    path: &str,
    goal: Option<&str>,
    disabled_rules: Option<Vec<String>>,
    include_dirs: Option<Vec<String>>,
    defines: Option<Vec<String>>
) -> PyResult<Py<PyAny>> {
    let options = options(goal, disabled_rules, include_dirs, defines)?;
    let file = Utf8Path::new(path);
    let outcome = py
        .detach(|| cpclib_basmopt::apply_fixes_in_place(file, &options))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    std::fs::write(path, &outcome.final_source)
        .map_err(|e| PyRuntimeError::new_err(format!("cannot write {path}: {e}")))?;
    to_py(
        py,
        &json!({
            "passes_run": outcome.passes_run,
            "total_applied": outcome.total_applied,
            "remaining_skipped": outcome.remaining_skipped,
            "assemble_warning": outcome.assemble_warning
        })
    )
}

/// **Rewrites every `.asm` file under `project_dir`** (`.gitignore` is
/// respected) with its own bulk-safe optimisations that do not depend on real
/// addresses; the address-aware ones are counted, not applied. Returns
/// `{"files_touched", "files_with_errors", "total_applied",
/// "total_skipped_for_review", "total_address_aware_skipped"}`.
#[pyfunction]
#[pyo3(signature = (project_dir, goal=None, disabled_rules=None, include_dirs=None, defines=None))]
fn apply_optimizations_project_in_place(
    py: Python,
    project_dir: &str,
    goal: Option<&str>,
    disabled_rules: Option<Vec<String>>,
    include_dirs: Option<Vec<String>>,
    defines: Option<Vec<String>>
) -> PyResult<Py<PyAny>> {
    let options = options(goal, disabled_rules, include_dirs, defines)?;
    let dir = Utf8Path::new(project_dir);
    let outcome = py.detach(|| cpclib_basmopt::apply_fixes_in_place_project(dir, &options));
    to_py(
        py,
        &json!({
            "files_touched": outcome.files_touched,
            "files_with_errors": outcome.files_with_errors,
            "total_applied": outcome.total_applied,
            "total_skipped_for_review": outcome.total_skipped_for_review,
            "total_address_aware_skipped": outcome.total_address_aware_skipped
        })
    )
}

pub fn analysis(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(count_nops, m)?)?;
    m.add_function(wrap_pyfunction!(suggest_optimizations, m)?)?;
    m.add_function(wrap_pyfunction!(apply_optimizations_in_place, m)?)?;
    m.add_function(wrap_pyfunction!(apply_optimizations_project_in_place, m)?)?;
    Ok(())
}

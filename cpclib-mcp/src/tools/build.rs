//! `run_build`/`list_build_targets`/`outdated_targets` - thin wrappers over
//! `cpclib_bndbuild::builder::BndBuilder`.

use std::sync::{Arc, Mutex};

use camino::{Utf8Path, Utf8PathBuf};
use cpclib_bndbuild::BndBuilderError;
use cpclib_bndbuild::app::WatchState;
use cpclib_bndbuild::builder::BndBuilder;
use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc};
use cpclib_common::event::EventObserver;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};

/// Captures every `BndBuilderEvent`'s text into an ordered log, rather than
/// letting a build write to this process's own stdout (reserved for MCP
/// protocol frames). Cheap-clone: the observer instance handed to
/// `BndBuilder::add_observer` is a separate clone of the one this module
/// keeps, sharing the same underlying `Vec` so the log is still readable
/// after the observer itself has been moved into the builder.
#[derive(Debug, Clone, Default)]
struct LogObserver {
    lines: Arc<Mutex<Vec<String>>>
}

impl LogObserver {
    fn push(&self, line: impl Into<String>) {
        self.lines.lock().unwrap().push(line.into());
    }

    fn into_lines(self) -> Vec<String> {
        Arc::try_unwrap(self.lines)
            .map(|m| m.into_inner().unwrap())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone())
    }
}

impl EventObserver for LogObserver {
    fn emit_stdout(&self, s: &str) {
        self.push(s.trim_end());
    }

    fn emit_stderr(&self, s: &str) {
        self.push(format!("[stderr] {}", s.trim_end()));
    }
}

impl BndBuilderObserver for LogObserver {
    fn update(&self, event: BndBuilderEvent) {
        match event {
            BndBuilderEvent::StartRule { rule, nb, out_of } => {
                self.push(format!("[{nb}/{out_of}] building {rule}"));
            },
            BndBuilderEvent::StopRule(rule) => self.push(format!("done: {rule}")),
            BndBuilderEvent::SkippedRule(rule) => self.push(format!("up to date: {rule}")),
            BndBuilderEvent::FailedRule(rule) => self.push(format!("FAILED: {rule}")),
            BndBuilderEvent::TaskStdout(_, _, s) => self.push(s),
            BndBuilderEvent::TaskStderr(_, _, s) => self.push(format!("[stderr] {s}")),
            BndBuilderEvent::TaskIgnoredError(_, _, s) => {
                self.push(format!("[ignored error] {s}"));
            },
            BndBuilderEvent::Stdout(s) => self.push(s),
            BndBuilderEvent::Stderr(s) => self.push(format!("[stderr] {s}")),
            _ => {}
        }
    }
}

fn build_error(e: BndBuilderError) -> ToolError {
    ToolError::new(ToolErrorKind::Build, e.to_string())
}

fn open_builder(bnd_path: &str) -> Result<(Utf8PathBuf, BndBuilder), ToolError> {
    // `false` = do not force every nested `bndbuild` task to add `--serial`
    // (this crate's own "rayon" feature is enabled - see Cargo.toml - so
    // `from_path` takes this extra argument).
    BndBuilder::from_path(Utf8Path::new(bnd_path), false).map_err(build_error)
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunBuildInput {
    /// Path to the build file (e.g. `build.bnd`) or a directory containing
    /// one.
    pub bnd_path: String,
    /// Target to build; defaults to the build file's own default target.
    pub target: Option<String>
}

/// **MUTATING**: runs a build target (and its dependencies) via
/// `BndBuilder::execute`. Captures build output into a returned log rather
/// than writing to this process's own stdout.
pub(crate) fn run_build(input: RunBuildInput) -> ToolResult {
    let (resolved_path, mut builder) = open_builder(&input.bnd_path)?;
    let target = match &input.target {
        Some(t) => Utf8PathBuf::from(t),
        None => {
            builder
                .default_target()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| {
                    ToolError::new(
                        ToolErrorKind::Build,
                        "no target given and the build file declares no default target"
                    )
                })?
        }
    };

    let observer = LogObserver::default();
    builder.add_observer(BndBuilderObserverRc::new(observer.clone()));

    let start = std::time::Instant::now();
    let result = builder.execute(&target);
    let duration_ms = start.elapsed().as_millis();
    let log = observer.into_lines();

    match result {
        Ok(()) => {
            Ok(json!({
                "bnd_path": resolved_path.as_str(),
                "target": target.as_str(),
                "success": true,
                "duration_ms": duration_ms,
                "log": log
            }))
        },
        Err(e) => {
            Err(ToolError::with_details(
                ToolErrorKind::Build,
                e.to_string(),
                json!({
                    "bnd_path": resolved_path.as_str(),
                    "target": target.as_str(),
                    "duration_ms": duration_ms,
                    "log": log
                })
            ))
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BndPathInput {
    pub bnd_path: String
}

/// Every target the build file declares, with its direct dependencies.
/// Read-only.
pub(crate) fn list_build_targets(input: BndPathInput) -> ToolResult {
    let (resolved_path, builder) = open_builder(&input.bnd_path)?;
    let targets: Vec<Value> = builder
        .targets()
        .into_iter()
        .map(|t| {
            let dependencies: Vec<String> = builder
                .get_rule(t)
                .map(|r| r.dependencies().iter().map(|d| d.to_string()).collect())
                .unwrap_or_default();
            json!({ "target": t.as_str(), "dependencies": dependencies })
        })
        .collect();
    Ok(json!({
        "bnd_path": resolved_path.as_str(),
        "default_target": builder.default_target().map(|p| p.as_str()),
        "targets": targets
    }))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OutdatedInput {
    pub bnd_path: String,
    /// Target to check; when omitted, every declared target is checked.
    pub target: Option<String>
}

/// Whether target(s) are outdated relative to their dependencies (mtime-
/// based), without building anything. Read-only.
pub(crate) fn outdated_targets(input: OutdatedInput) -> ToolResult {
    let (resolved_path, builder) = open_builder(&input.bnd_path)?;
    let targets_to_check: Vec<Utf8PathBuf> = match &input.target {
        Some(t) => vec![Utf8PathBuf::from(t)],
        None => builder.targets().into_iter().map(|p| p.to_path_buf()).collect()
    };

    let mut results = Vec::new();
    for target in &targets_to_check {
        let outdated = builder
            .outdated(&WatchState::NoWatch, target)
            .map_err(build_error)?;
        results.push(json!({ "target": target.as_str(), "outdated": outdated }));
    }
    Ok(json!({ "bnd_path": resolved_path.as_str(), "results": results }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = build_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "MUTATING: runs a bndbuild target (and its dependencies). Returns a \
                           captured build log.")]
    async fn run_build(
        &self,
        Parameters(input): Parameters<RunBuildInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(run_build(input))
    }

    #[tool(description = "List every target a bndbuild file declares, with its direct \
                           dependencies. Read-only.")]
    async fn list_build_targets(
        &self,
        Parameters(input): Parameters<BndPathInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(list_build_targets(input))
    }

    #[tool(description = "Check whether bndbuild target(s) are outdated relative to their \
                           dependencies, without building anything. Read-only.")]
    async fn outdated_targets(
        &self,
        Parameters(input): Parameters<OutdatedInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(outdated_targets(input))
    }
}

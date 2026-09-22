//! `clone_project` - copy a project into a fresh scratch directory so it can
//! be built and edited freely without ever touching the original.
//!
//! Reading the source is the only thing done to it: nothing is created,
//! modified or deleted there. Modification times are preserved on purpose -
//! a build system decides what is out of date from them, and a naive copy
//! (all files stamped "now") makes it regenerate everything, including
//! steps that can take an hour (etchy's picture conversion).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::McpServer;
use crate::error::{ToolError, ToolResult};

/// Directories never copied: version control and build/dependency caches.
const SKIPPED_DIRS: &[&str] = &[".git", ".hg", ".svn", "target", "node_modules", ".claude"];

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CloneProjectInput {
    /// The project directory to copy (only ever read).
    pub source: String,
    /// Destination directory. Must not exist yet or be empty. Default: a
    /// new directory under the system temp directory.
    pub destination: Option<String>,
    /// Also copy version-control directories (`.git`, ...). Default false.
    pub include_vcs: Option<bool>
}

#[derive(Default)]
struct CopyStats {
    files: usize,
    bytes: u64,
    skipped_dirs: Vec<String>
}

fn copy_tree(from: &Path, to: &Path, include_vcs: bool, stats: &mut CopyStats) -> std::io::Result<()> {
    fs_err::create_dir_all(to)?;
    for entry in fs_err::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let kind = entry.file_type()?;
        let src = entry.path();
        let dst = to.join(&name);
        if kind.is_dir() {
            let name = name.to_string_lossy();
            let is_vcs = matches!(name.as_ref(), ".git" | ".hg" | ".svn");
            if SKIPPED_DIRS.contains(&name.as_ref()) && !(is_vcs && include_vcs) {
                stats.skipped_dirs.push(name.into_owned());
                continue;
            }
            copy_tree(&src, &dst, include_vcs, stats)?;
        }
        else if kind.is_file() {
            let modified: Option<SystemTime> = entry.metadata()?.modified().ok();
            let bytes = fs_err::copy(&src, &dst)?;
            stats.files += 1;
            stats.bytes += bytes;
            if let Some(t) = modified {
                // Best effort: a filesystem that cannot set times still
                // yields a usable copy.
                if let Ok(f) = fs_err::OpenOptions::new().write(true).open(&dst) {
                    let _ = f.set_modified(t);
                }
            }
        }
        // Symlinks and special files are deliberately not followed or copied.
    }
    Ok(())
}

/// A scratch copy of a project that mutating tools work in instead of the
/// real one, removed on drop.
///
/// Editing the real files and restoring them afterwards is not safe enough:
/// the restore does not run if the server is killed mid-build, and a build
/// step may write anywhere in the tree. Copying first makes the original
/// untouchable by construction - which is the rule for projects like etchy.
pub(crate) struct Sandbox {
    origin: PathBuf,
    root: PathBuf,
    keep: bool
}

impl Sandbox {
    pub(crate) fn create(origin: &Path, keep: bool) -> Result<Self, ToolError> {
        let cloned = clone_project(CloneProjectInput {
            source: origin.to_string_lossy().into_owned(),
            destination: None,
            include_vcs: Some(false)
        })?;
        let root = PathBuf::from(cloned["destination"].as_str().unwrap_or_default());
        let origin = fs_err::canonicalize(origin).map_err(|e| ToolError::io(e.to_string()))?;
        Ok(Self { origin, root, keep })
    }

    /// Where the copy lives, when it outlives the call.
    pub(crate) fn keep_path(&self) -> Option<String> {
        self.keep.then(|| self.root.display().to_string())
    }

    /// The copy of a path inside the original project. A path outside it is
    /// refused: it has no copy, so using it would touch the real thing.
    pub(crate) fn map(&self, path: &str) -> Result<String, ToolError> {
        let given = Path::new(path);
        let absolute = if given.is_absolute() {
            given.to_path_buf()
        }
        else {
            std::env::current_dir().map_err(|e| ToolError::io(e.to_string()))?.join(given)
        };
        // The file may not exist yet (an output): resolve its directory.
        let resolved = fs_err::canonicalize(&absolute).or_else(|_| {
            let name = absolute.file_name().ok_or_else(|| std::io::Error::other("no file name"))?;
            let dir = absolute.parent().ok_or_else(|| std::io::Error::other("no directory"))?;
            fs_err::canonicalize(dir).map(|d| d.join(name))
        });
        let relative = resolved
            .ok()
            .and_then(|r| r.strip_prefix(&self.origin).ok().map(Path::to_path_buf))
            .ok_or_else(|| {
                ToolError::invalid_input(format!(
                    "{path} is outside the project directory {} - pass `project_dir` to widen it, \
                     or `in_place: true` to work on the real files",
                    self.origin.display()
                ))
            })?;
        Ok(self.root.join(relative).to_string_lossy().into_owned())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs_err::remove_dir_all(&self.root);
        }
    }
}

/// The directory a build file lives in (or the directory itself).
pub(crate) fn project_dir_of(bnd_path: &str) -> PathBuf {
    let path = Path::new(bnd_path);
    let dir = if path.is_dir() { path } else { path.parent().unwrap_or(Path::new(".")) };
    if dir.as_os_str().is_empty() { PathBuf::from(".") } else { dir.to_path_buf() }
}

pub(crate) fn clone_project(input: CloneProjectInput) -> ToolResult {
    let source = PathBuf::from(&input.source);
    if !source.is_dir() {
        return Err(ToolError::invalid_input(format!("{} is not a directory", input.source)));
    }
    let source = fs_err::canonicalize(&source).map_err(|e| ToolError::io(e.to_string()))?;

    let destination = match &input.destination {
        Some(d) => PathBuf::from(d),
        None => {
            let name = source.file_name().map_or("project".into(), |n| n.to_string_lossy().into_owned());
            let stamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis());
            std::env::temp_dir().join(format!("cpclib-sandbox-{name}-{stamp}"))
        }
    };
    if destination.exists() && fs_err::read_dir(&destination).map_err(|e| ToolError::io(e.to_string()))?.next().is_some() {
        return Err(ToolError::invalid_input(format!(
            "{} already exists and is not empty - refusing to write into it",
            destination.display()
        )));
    }
    // Copying a directory into itself would recurse forever.
    let dest_abs = if destination.is_absolute() { destination.clone() } else { std::env::current_dir().map_err(|e| ToolError::io(e.to_string()))?.join(&destination) };
    if dest_abs.starts_with(&source) {
        return Err(ToolError::invalid_input("the destination must not be inside the source"));
    }

    let mut stats = CopyStats::default();
    copy_tree(&source, &destination, input.include_vcs.unwrap_or(false), &mut stats)
        .map_err(|e| ToolError::io(format!("copy failed: {e}")))?;

    Ok(json!({
        "source": source.display().to_string(),
        "destination": destination.display().to_string(),
        "files": stats.files,
        "bytes": stats.bytes,
        "skipped_directories": stats.skipped_dirs,
        "note": "modification times preserved; the source was only read"
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Value>, Json<Value>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = sandbox_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Copies a project directory into a fresh scratch directory (default: \
                           under the system temp dir) so it can be built and edited freely - \
                           the source is only read, never modified. Preserves file modification \
                           times so the build system does not regenerate everything (some \
                           projects have hour-long generation steps), skips .git/target/\
                           node_modules unless include_vcs. Refuses a non-empty destination or \
                           one inside the source. Use this before measure_variants or any edit \
                           on a project you must not change (current_projects, a release repo).")]
    async fn clone_project(
        &self,
        Parameters(input): Parameters<CloneProjectInput>
    ) -> Result<Json<Value>, Json<Value>> {
        ok_or_tool_error(clone_project(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(source: &Path, destination: Option<&Path>) -> CloneProjectInput {
        CloneProjectInput {
            source: source.display().to_string(),
            destination: destination.map(|d| d.display().to_string()),
            include_vcs: None
        }
    }

    #[test]
    fn a_project_is_copied_with_mtimes_and_without_vcs_or_target() {
        let dir = camino_tempfile::tempdir().unwrap();
        let src = dir.path().join("proj");
        fs_err::create_dir_all(src.join("src")).unwrap();
        fs_err::create_dir_all(src.join(".git")).unwrap();
        fs_err::create_dir_all(src.join("target")).unwrap();
        fs_err::write(src.join("src/a.asm"), " nop\n").unwrap();
        fs_err::write(src.join(".git/HEAD"), "ref").unwrap();
        fs_err::write(src.join("target/x.o"), "obj").unwrap();
        let old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        fs_err::OpenOptions::new().write(true).open(src.join("src/a.asm")).unwrap().set_modified(old).unwrap();

        let dest = dir.path().join("copy");
        let out = clone_project(input(src.as_std_path(), Some(dest.as_std_path()))).unwrap();

        assert_eq!(out["files"], 1);
        assert!(dest.join("src/a.asm").exists());
        assert!(!dest.join(".git").exists() && !dest.join("target").exists());
        let copied = fs_err::metadata(dest.join("src/a.asm")).unwrap().modified().unwrap();
        assert_eq!(copied, old, "mtime must be preserved");
        assert_eq!(fs_err::read_to_string(src.join("src/a.asm")).unwrap(), " nop\n", "source untouched");
    }

    #[test]
    fn a_non_empty_destination_or_one_inside_the_source_is_refused() {
        let dir = camino_tempfile::tempdir().unwrap();
        let src = dir.path().join("proj");
        fs_err::create_dir_all(&src).unwrap();
        fs_err::write(src.join("f"), "x").unwrap();
        let taken = dir.path().join("taken");
        fs_err::create_dir_all(&taken).unwrap();
        fs_err::write(taken.join("keep"), "y").unwrap();

        let err = clone_project(input(src.as_std_path(), Some(taken.as_std_path()))).unwrap_err();
        assert_eq!(err.kind, "invalid_input");
        assert!(taken.join("keep").exists());
        let err = clone_project(input(src.as_std_path(), Some(src.join("inner").as_std_path()))).unwrap_err();
        assert_eq!(err.kind, "invalid_input");
    }

    #[test]
    fn a_sandbox_maps_project_paths_refuses_outside_ones_and_cleans_up() {
        let dir = camino_tempfile::tempdir().unwrap();
        let project = dir.path().join("proj");
        fs_err::create_dir_all(project.join("src")).unwrap();
        fs_err::write(project.join("src/a.asm"), "nop").unwrap();
        let outside = dir.path().join("other.asm");
        fs_err::write(&outside, "nop").unwrap();

        let root;
        {
            let sandbox = Sandbox::create(project.as_std_path(), false).unwrap();
            let mapped = sandbox.map(project.join("src/a.asm").as_str()).unwrap();
            root = std::path::PathBuf::from(&mapped);
            assert!(!mapped.starts_with(project.as_str()), "{mapped}");
            assert_eq!(fs_err::read_to_string(&mapped).unwrap(), "nop");
            // a file that does not exist yet (an output) still maps
            assert!(sandbox.map(project.join("out.bin").as_str()).is_ok());
            assert_eq!(sandbox.map(outside.as_str()).unwrap_err().kind, "invalid_input");
            // editing the copy leaves the original alone
            fs_err::write(&mapped, "changed").unwrap();
            assert_eq!(fs_err::read_to_string(project.join("src/a.asm")).unwrap(), "nop");
        }
        assert!(!root.exists(), "the copy is removed on drop");
    }
}

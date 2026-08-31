//! Assembling a project once per change, not once per question.
//!
//! Assembling a real demo takes tens of seconds. Every feature that needs a
//! *real* address - a debugger placing a breakpoint, the optimizer deciding
//! whether a `jp` reaches as a `jr` - needs the same assembled `Env`, so it is
//! computed once and kept until the project actually changes.
//!
//! The key is a **fingerprint**: the newest modification time across the
//! project's sources. It changes exactly when a rebuild would lay the code out
//! differently, and costs one `stat` per file instead of an assemble.
//!
//! Note what this does *not* buy: the cache lives in a process, so a language
//! server and a debug adapter running side by side each keep their own. Only
//! the code is shared here, not the work.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cpclib_asm::assembler::Env;
use dashmap::DashMap;

use crate::config::AsmConfig;
use crate::entry::{self, ProjectGraph};

/// One project's assembled state, reused until the project changes.
#[derive(Default)]
pub struct ProjectCache {
    graph: DashMap<PathBuf, (u128, Arc<ProjectGraph>)>,
    env: DashMap<PathBuf, (u128, Arc<Env>)>
}

impl ProjectCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The project's include graph, rebuilt only when the project changed.
    ///
    /// Hands back the fingerprint alongside it so callers reuse the stat pass
    /// this already paid for rather than walking again.
    pub fn graph_for(&self, root: &Path) -> (u128, Arc<ProjectGraph>) {
        let fingerprint = entry::fingerprint_of(root);
        if let Some(cached) = self.graph.get(root)
            && cached.0 == fingerprint
        {
            return (fingerprint, cached.1.clone());
        }
        let graph = Arc::new(entry::graph_of(&entry::scan_workspace(root)));
        self.graph
            .insert(root.to_path_buf(), (fingerprint, graph.clone()));
        (fingerprint, graph)
    }

    /// The assembled `Env` for `entry`, cached against `fingerprint`.
    pub fn env_for(
        &self,
        entry_path: &Path,
        fingerprint: u128,
        config: &AsmConfig
    ) -> Option<Arc<Env>> {
        if let Some(cached) = self.env.get(entry_path)
            && cached.0 == fingerprint
        {
            return Some(cached.1.clone());
        }

        let disabled = config.warnings.disabled_assembling_categories();
        let env = Arc::new(entry::assemble_entry(
            entry_path,
            config.case_sensitive,
            disabled
        )?);
        self.env
            .insert(entry_path.to_path_buf(), (fingerprint, env.clone()));
        Some(env)
    }

    /// Drop everything. For a caller that knows the world changed underneath
    /// it in a way a fingerprint cannot see.
    pub fn clear(&self) {
        self.graph.clear();
        self.env.clear();
    }
}

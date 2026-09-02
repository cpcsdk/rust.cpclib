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
    ///
    /// Same `DashMap::entry`-based coalescing as `env_for` - see its own doc
    /// comment for why a `get`-then-`insert` pair can't close the race
    /// between two concurrent callers both missing on the same key.
    pub fn graph_for(&self, root: &Path) -> (u128, Arc<ProjectGraph>) {
        use dashmap::mapref::entry::Entry;

        let fingerprint = entry::fingerprint_of(root);
        match self.graph.entry(root.to_path_buf()) {
            Entry::Occupied(occ) if occ.get().0 == fingerprint => {
                (fingerprint, occ.get().1.clone())
            },
            Entry::Occupied(mut occ) => {
                let graph = Arc::new(entry::graph_of(&entry::scan_workspace(root)));
                occ.insert((fingerprint, graph.clone()));
                (fingerprint, graph)
            },
            Entry::Vacant(vac) => {
                let graph = Arc::new(entry::graph_of(&entry::scan_workspace(root)));
                vac.insert((fingerprint, graph.clone()));
                (fingerprint, graph)
            }
        }
    }

    /// The assembled `Env` for `entry`, cached against `fingerprint`.
    ///
    /// Uses `DashMap::entry` rather than a separate `get`-then-`insert` pair
    /// so a miss holds this key's shard lock across the whole assemble, not
    /// just the final `insert`: two callers racing for the same
    /// `(entry_path, fingerprint)` - e.g. two quick-fix requests fired for
    /// different cursor positions before the first one's assemble finishes -
    /// used to each independently pay this module's own documented "tens of
    /// seconds" cost on their own thread, competing with each other for CPU
    /// the whole time, instead of the second simply waiting for the first's
    /// answer. A `get`-then-`insert` pair can never close this window: the
    /// gap between the failed `get` and the eventual `insert` is exactly
    /// where a second caller's own `get` also misses.
    pub fn env_for(
        &self,
        entry_path: &Path,
        fingerprint: u128,
        config: &AsmConfig
    ) -> Option<Arc<Env>> {
        use dashmap::mapref::entry::Entry;

        let disabled = config.warnings.disabled_assembling_categories();
        match self.env.entry(entry_path.to_path_buf()) {
            Entry::Occupied(occ) if occ.get().0 == fingerprint => Some(occ.get().1.clone()),
            Entry::Occupied(mut occ) => {
                let env = Arc::new(entry::assemble_entry(
                    entry_path,
                    config.case_sensitive,
                    disabled
                )?);
                occ.insert((fingerprint, env.clone()));
                Some(env)
            },
            Entry::Vacant(vac) => {
                let env = Arc::new(entry::assemble_entry(
                    entry_path,
                    config.case_sensitive,
                    disabled
                )?);
                vac.insert((fingerprint, env.clone()));
                Some(env)
            }
        }
    }

    /// Drop everything. For a caller that knows the world changed underneath
    /// it in a way a fingerprint cannot see.
    pub fn clear(&self) {
        self.graph.clear();
        self.env.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_entry(dir: &camino_tempfile::Utf8TempDir, text: &str) -> PathBuf {
        let path = dir.path().join("main.asm");
        std::fs::write(&path, text).unwrap();
        path.into()
    }

    /// Regression test for the `DashMap::entry`-based rewrite of `env_for`:
    /// a miss must still compute and cache, a hit at the same fingerprint
    /// must reuse the cached `Arc` (not recompute), and a new fingerprint
    /// must recompute rather than serve the stale value.
    #[test]
    fn env_for_computes_on_miss_reuses_on_hit_and_recomputes_on_a_new_fingerprint() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let entry_path = write_entry(&tmp, "org 0x4000\n    ret\n");
        let cache = ProjectCache::new();
        let config = AsmConfig::default();

        let first = cache
            .env_for(&entry_path, 1, &config)
            .expect("a valid entry should assemble");
        let same_fingerprint = cache
            .env_for(&entry_path, 1, &config)
            .expect("cached lookup should still succeed");
        assert!(
            Arc::ptr_eq(&first, &same_fingerprint),
            "a hit at the same fingerprint must reuse the cached Arc, not recompute"
        );

        let new_fingerprint = cache
            .env_for(&entry_path, 2, &config)
            .expect("a fingerprint change should recompute, not fail");
        assert!(
            !Arc::ptr_eq(&first, &new_fingerprint),
            "a new fingerprint must recompute rather than serve the stale value"
        );
    }

    /// A genuinely unparseable entry stays a clean miss (`None`), not a
    /// panic or a poisoned cache entry - confirms the `?` inside the
    /// `Entry::Vacant`/`Entry::Occupied` match arms still short-circuits
    /// correctly now that it's nested inside `match` instead of the
    /// original flat `if`-then-`?` shape.
    #[test]
    fn env_for_returns_none_for_an_unparseable_entry_without_caching_it() {
        let tmp = camino_tempfile::tempdir().unwrap();
        let entry_path = write_entry(&tmp, "this is not valid z80 at all {{{\n");
        let cache = ProjectCache::new();
        let config = AsmConfig::default();

        assert!(cache.env_for(&entry_path, 1, &config).is_none());
        // A second call must retry (not have cached the failure) - same
        // observable behaviour as the original `get`-then-`insert` code,
        // which never inserted on a `?` short-circuit either.
        assert!(cache.env_for(&entry_path, 1, &config).is_none());
    }

    /// Same three behaviours as `env_for`'s own test, for `graph_for`.
    #[test]
    fn graph_for_computes_on_miss_reuses_on_hit_and_recomputes_on_a_new_root() {
        let tmp = camino_tempfile::tempdir().unwrap();
        write_entry(&tmp, "org 0x4000\n    ret\n");
        let cache = ProjectCache::new();

        let (fp1, first) = cache.graph_for(tmp.path().as_std_path());
        let (fp2, same_fingerprint) = cache.graph_for(tmp.path().as_std_path());
        assert_eq!(fp1, fp2);
        assert!(
            Arc::ptr_eq(&first, &same_fingerprint),
            "a hit at the same fingerprint must reuse the cached Arc, not recompute"
        );

        // Touching a source file moves the fingerprint forward, forcing a
        // real recompute - mirrors `fingerprint_of`'s own contract. Past its
        // own 300ms short-lived memo (`entry::fingerprint_of`'s
        // `FINGERPRINT_CACHE_TTL`), or this test would still observe the
        // pre-change fingerprint regardless of `graph_for`'s own logic.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::open(tmp.path().join("main.asm"))
            .unwrap()
            .set_modified(later)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(350));
        let (fp3, after_change) = cache.graph_for(tmp.path().as_std_path());
        assert_ne!(fp1, fp3);
        assert!(
            !Arc::ptr_eq(&first, &after_change),
            "a fingerprint change must recompute rather than serve the stale graph"
        );
    }
}

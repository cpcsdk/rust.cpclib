//! Build-time prediction cache.
//!
//! Historical run durations are persisted as a flat TSV file
//! (`.bndbuild_timings`) placed in the working directory where bndbuild was
//! launched.  Each line stores one sample:
//!
//! ```text
//! build_file_path TAB rule_name TAB task_name TAB duration_nanos
//! ```
//!
//! `task_name` is the empty string for rule-level records.
//! At most [`MAX_SAMPLES`] most-recent samples are kept per `(build_file,
//! rule, task)` key so the file stays bounded in size.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Maximum number of historical samples retained per key.
const MAX_SAMPLES: usize = 20;

/// Cache file name, placed in the bndbuild working directory.
pub(crate) const CACHE_FILENAME: &str = ".bndbuild_timings";

// ─── Key ──────────────────────────────────────────────────────────────────────

/// Unique address for one timing series.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub(crate) struct TimingKey {
    /// Absolute canonical path to the bndbuild file, or empty for the root build.
    pub(crate) build_file: String,
    pub(crate) rule: String,
    /// Empty string → rule-level sample; otherwise the task command string.
    pub(crate) task: String
}

// ─── Cache ────────────────────────────────────────────────────────────────────

/// In-memory timing cache with TSV-based load/save support.
pub(crate) struct TimingCache {
    /// Path to the on-disk cache file.
    path: PathBuf,
    /// Per-key sample list (insertion order, oldest first).
    entries: HashMap<TimingKey, Vec<Duration>>
}

impl TimingCache {
    // ── Construction ──────────────────────────────────────────────────────────

    /// Load (or create) the cache located at `{cwd}/.bndbuild_timings`.
    pub(crate) fn load(cwd: &Path) -> Self {
        let path = cwd.join(CACHE_FILENAME);
        let mut entries: HashMap<TimingKey, Vec<Duration>> = HashMap::new();

        if let Ok(file) = std::fs::File::open(&path) {
            for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
                let mut parts = line.splitn(4, '\t');
                let (Some(bf), Some(rule), Some(task), Some(ns_str)) =
                    (parts.next(), parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };
                let Ok(nanos) = ns_str.parse::<u64>()
                else {
                    continue;
                };

                let key = TimingKey {
                    build_file: bf.to_owned(),
                    rule: rule.to_owned(),
                    task: task.to_owned()
                };
                let v = entries.entry(key).or_default();
                // Only load up to MAX_SAMPLES (older entries are silently skipped).
                if v.len() < MAX_SAMPLES {
                    v.push(Duration::from_nanos(nanos));
                }
            }
        }

        Self { path, entries }
    }

    // ── Recording ─────────────────────────────────────────────────────────────

    /// Append a new duration sample for the given key, evicting the oldest
    /// entry once `MAX_SAMPLES` is exceeded.
    pub(crate) fn record(&mut self, build_file: &str, rule: &str, task: &str, duration: Duration) {
        let key = TimingKey {
            build_file: build_file.to_owned(),
            rule: rule.to_owned(),
            task: task.to_owned()
        };
        let v = self.entries.entry(key).or_default();
        if v.len() >= MAX_SAMPLES {
            v.remove(0);
        }
        v.push(duration);
    }

    // ── Estimation ────────────────────────────────────────────────────────────

    /// Return an exponentially-weighted moving average (EWMA) of historical
    /// durations for the given key, or `None` if no samples exist yet.
    ///
    /// α = 0.3 means recent samples are weighted more heavily, so the estimate
    /// adapts quickly when a target gets faster or slower after a code change.
    pub(crate) fn estimate(&self, build_file: &str, rule: &str, task: &str) -> Option<Duration> {
        let key = TimingKey {
            build_file: build_file.to_owned(),
            rule: rule.to_owned(),
            task: task.to_owned()
        };
        let v = self.entries.get(&key)?;
        if v.is_empty() {
            return None;
        }
        const ALPHA: f64 = 0.3;
        let mut ewma = v[0].as_secs_f64();
        for d in &v[1..] {
            ewma = (1.0 - ALPHA) * ewma + ALPHA * d.as_secs_f64();
        }
        Some(Duration::from_secs_f64(ewma))
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Write all in-memory entries back to the cache file, overwriting it.
    /// Silently ignores I/O errors (cache is best-effort).
    pub(crate) fn save(&self) -> std::io::Result<()> {
        let mut file = std::fs::File::create(&self.path)?;
        for (key, durations) in &self.entries {
            for d in durations {
                writeln!(
                    file,
                    "{}\t{}\t{}\t{}",
                    key.build_file,
                    key.rule,
                    key.task,
                    d.as_nanos(),
                )?;
            }
        }
        Ok(())
    }
}

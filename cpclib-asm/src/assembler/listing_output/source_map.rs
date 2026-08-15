//! Where each source line ended up, kept rather than printed.
//!
//! The listing already computes everything a debugger needs to map a line to
//! an address and back - which file, which line, what address, how many bytes
//! - and then formats it into text and forgets it. This collects the same
//! records instead, as a side channel: `--lst` and a source map are not
//! mutually exclusive, and asking for one must not change the other's output.
//!
//! Rows arrive in assembly order, so a `REPEAT` body or a macro called five
//! times naturally produces five rows for the same source line. That is
//! correct and load-bearing: each is a distinct address the line occupies.

use std::collections::HashMap;

/// One emitted run of bytes, and the source line it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMapRow {
    /// Index into [`RawSourceMap::files`].
    pub file: u16,
    /// 1-based line number, as the user sees it.
    pub line: u32,
    /// Logical address of the first byte.
    pub logical: u32,
    /// How many bytes this row emitted. Zero for a line that produced none
    /// (an `EQU`, a comment) - kept, because "this line exists but has no
    /// address" is a different answer from "unknown line".
    pub len: u16
}

/// The rows, plus the file table they index into.
#[derive(Debug, Clone, Default)]
pub struct RawSourceMap {
    pub files: Vec<String>,
    pub rows: Vec<SourceMapRow>
}

/// Accumulates rows during the listing pass.
///
/// Deliberately allocation-light: the listing pass runs **once**, on the last
/// pass, so by the time rows arrive the shape of the program is already known
/// and each row is a fixed-size record with an interned file id - no per-row
/// `String`, no per-row `Vec`.
#[derive(Debug, Default)]
pub struct SourceMapCollector {
    files: Vec<String>,
    indices: HashMap<String, u16>,
    rows: Vec<SourceMapRow>
}

impl SourceMapCollector {
    pub fn new() -> Self {
        Self {
            // A demo is thousands of lines, not tens; one growth from here is
            // cheaper than the dozen a default-capacity Vec would do.
            rows: Vec::with_capacity(8192),
            ..Default::default()
        }
    }

    /// Intern a source file name, returning its id.
    pub fn file_id(&mut self, name: &str) -> u16 {
        if let Some(id) = self.indices.get(name) {
            return *id;
        }
        let id = self.files.len() as u16;
        self.files.push(name.to_string());
        self.indices.insert(name.to_string(), id);
        id
    }

    /// Record one emitted run. Lines that emitted nothing are skipped by the
    /// caller rather than filtered here, so this stays a plain push.
    pub fn push(&mut self, file: u16, line: u32, logical: u32, len: u16) {
        self.rows.push(SourceMapRow {
            file,
            line,
            logical,
            len
        });
    }

    /// A copy of what has been collected so far, leaving the collector alone.
    pub fn snapshot(&self) -> RawSourceMap {
        RawSourceMap {
            files: self.files.clone(),
            rows: self.rows.clone()
        }
    }

    pub fn finish(self) -> RawSourceMap {
        RawSourceMap {
            files: self.files,
            rows: self.rows
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

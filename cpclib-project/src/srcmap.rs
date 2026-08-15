//! Source lines to Z80 addresses, and back.
//!
//! A debugger asks two questions and they are not the same question:
//!
//! * *the user clicked line 42 of `main.asm` - where do I put the breakpoint?*
//! * *the program stopped at `&4021` - which line is that?*
//!
//! Both are answered from the rows the assembler collected during its listing
//! pass ([`cpclib_asm::assembler::listing_output::RawSourceMap`]), which carry
//! a length as well as a start. The length is what makes the reverse direction
//! honest: an address inside an instruction resolves to that instruction, and
//! an address in no row at all resolves to nothing - so the editor shows
//! disassembly instead of highlighting a line that has nothing to do with it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cpclib_asm::assembler::listing_output::RawSourceMap;
use serde::{Deserialize, Serialize};

/// How far forward a breakpoint may slide to find a line that emitted code.
///
/// Users put breakpoints on comments and blank lines constantly; sliding to
/// the next real instruction is what every debugger does. The cap stops a
/// breakpoint at the end of a file from silently landing in the next routine.
const MAX_BREAKPOINT_SLIDE: u32 = 64;

/// A resolved source position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    /// 1-based, as the user sees it.
    pub line: u32
}

/// Where a breakpoint actually went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakpointPlacement {
    pub address: u32,
    /// The line it ended up on, which may be later than the one asked for.
    pub line: u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Span {
    start: u32,
    end: u32,
    file: u16,
    line: u32
}

/// Bidirectional line/address mapping for one assembled program.
///
/// `serde`-serialisable on purpose: a language server that has already
/// assembled a project can hand this to a debug adapter instead of making it
/// pay for the assemble again.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceMap {
    /// Label -> address, so a debugger can answer "what is at `animation_state`"
    /// without re-assembling. Kept beside the line map because both come from
    /// the same build and are worthless if they come from different ones.
    symbols: HashMap<String, u32>,
    files: Vec<PathBuf>,
    /// `(file, line) -> every address that line occupies`, in assembly order.
    /// A macro body called five times has five.
    forward: HashMap<(u16, u32), Vec<u32>>,
    /// Sorted by `start`, for binary search.
    spans: Vec<Span>
}

impl SourceMap {
    /// Build the queryable form from what the assembler collected.
    pub fn from_raw(raw: &RawSourceMap) -> Self {
        let files: Vec<PathBuf> = raw.files.iter().map(PathBuf::from).collect();

        let mut forward: HashMap<(u16, u32), Vec<u32>> = HashMap::new();
        let mut spans = Vec::with_capacity(raw.rows.len());
        for row in &raw.rows {
            // Only rows that actually emitted bytes go in. A line that emitted
            // none does not *occupy* its address - the next line does - so
            // recording it would let a breakpoint on a comment claim to be set
            // while the emulator stops somewhere else entirely.
            if row.len == 0 {
                continue;
            }
            forward
                .entry((row.file, row.line))
                .or_default()
                .push(row.logical);
            spans.push(Span {
                start: row.logical,
                end: row.logical + row.len as u32,
                file: row.file,
                line: row.line
            });
        }
        spans.sort_unstable_by_key(|s| (s.start, s.end));

        Self {
            symbols: HashMap::new(),
            files,
            forward,
            spans
        }
    }

    /// Resolve the recorded file names against the program they came from.
    ///
    /// The assembler records a file by the name it was reached by, which for an
    /// `include` is relative. An editor cannot open a relative path - it asks
    /// the adapter for the contents instead, and an emulator has no idea what a
    /// source file is. So every path is made absolute here, once, using the
    /// same ancestor search `include` itself uses.
    pub fn resolved_against(mut self, entry: &Path) -> Self {
        self.files = self
            .files
            .into_iter()
            .map(|file| {
                if file.is_absolute() {
                    return file;
                }
                let name = file.to_string_lossy().to_string();
                crate::root::resolve_include_path(&name, entry)
                    .or_else(|| entry.parent().map(|dir| dir.join(&file)))
                    .and_then(|candidate| std::fs::canonicalize(&candidate).ok())
                    .unwrap_or(file)
            })
            .collect();
        self
    }

    /// Record the program's labels, so they can be watched by name.
    pub fn with_symbols(mut self, symbols: HashMap<String, u32>) -> Self {
        self.symbols = symbols;
        self
    }

    /// The address a label stands for, matched case-insensitively as a
    /// fallback - basm is case-sensitive by default but a user typing a watch
    /// expression is not thinking about that.
    pub fn address_of_symbol(&self, name: &str) -> Option<u32> {
        if let Some(address) = self.symbols.get(name) {
            return Some(*address);
        }
        let wanted = name.to_ascii_uppercase();
        self.symbols
            .iter()
            .find(|(known, _)| known.to_ascii_uppercase() == wanted)
            .map(|(_, address)| *address)
    }

    /// Every known label, for completion and for listing what can be watched.
    pub fn symbols(&self) -> impl Iterator<Item = (&str, u32)> {
        self.symbols.iter().map(|(name, address)| (name.as_str(), *address))
    }

    fn file_id(&self, file: &Path) -> Option<u16> {
        // Compare canonically where we can: the assembler records the path it
        // was given, the editor sends whatever the user opened.
        let wanted = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        self.files
            .iter()
            .position(|known| {
                let known_canonical =
                    std::fs::canonicalize(known).unwrap_or_else(|_| known.clone());
                known_canonical == wanted || known == file
            })
            .map(|i| i as u16)
    }

    /// Every address the code on `line` of `file` occupies.
    ///
    /// Empty for a line that emitted nothing - a comment, an `EQU`, a label on
    /// its own.
    pub fn addresses_at(&self, file: &Path, line: u32) -> &[u32] {
        self.file_id(file)
            .and_then(|id| self.forward.get(&(id, line)))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Where a breakpoint on `line` of `file` should go.
    ///
    /// The lowest address the line occupies - a line is entered at its start,
    /// whatever else it later becomes. If the line emitted nothing, slide
    /// forward to the next one that did, and report which line that was so the
    /// editor can move the dot to where the breakpoint really is.
    pub fn breakpoint_at(&self, file: &Path, line: u32) -> Option<BreakpointPlacement> {
        let id = self.file_id(file)?;
        for candidate in line..line.saturating_add(MAX_BREAKPOINT_SLIDE) {
            if let Some(addresses) = self.forward.get(&(id, candidate))
                && let Some(address) = addresses.iter().copied().min()
            {
                return Some(BreakpointPlacement {
                    address,
                    line: candidate
                });
            }
        }
        None
    }

    /// Which source line an address belongs to, or `None` when it belongs to
    /// none - which is a real answer, not a failure.
    pub fn location_at(&self, address: u32) -> Option<SourceLocation> {
        // The last span starting at or before `address`, then a containment
        // check: sorted starts make this a binary search, and the containment
        // check is what stops a stray address being attributed to whatever
        // happened to be nearest.
        let index = self.spans.partition_point(|s| s.start <= address);
        let span = self.spans[..index]
            .iter()
            .rev()
            .find(|s| address >= s.start && address < s.end)?;
        Some(SourceLocation {
            file: self.files.get(span.file as usize)?.clone(),
            line: span.line
        })
    }

    /// The files this program was built from.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};

    use super::*;

    fn map(rows: &[(u16, u32, u32, u16)]) -> SourceMap {
        SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".to_string(), "inc.asm".to_string()],
            rows: rows
                .iter()
                .map(|&(file, line, logical, len)| {
                    SourceMapRow {
                        file,
                        line,
                        logical,
                        len
                    }
                })
                .collect()
        })
    }

    #[test]
    fn a_line_maps_to_its_address_and_back() {
        let m = map(&[(0, 10, 0x4000, 3), (0, 11, 0x4003, 1)]);
        assert_eq!(m.addresses_at(Path::new("main.asm"), 10), &[0x4000]);
        assert_eq!(
            m.location_at(0x4000).unwrap(),
            SourceLocation {
                file: PathBuf::from("main.asm"),
                line: 10
            }
        );
    }

    /// The length is the point: a PC in the middle of a 3-byte instruction is
    /// still that instruction.
    #[test]
    fn an_address_inside_an_instruction_resolves_to_it() {
        let m = map(&[(0, 10, 0x4000, 3), (0, 11, 0x4003, 1)]);
        assert_eq!(m.location_at(0x4001).unwrap().line, 10);
        assert_eq!(m.location_at(0x4002).unwrap().line, 10);
        assert_eq!(m.location_at(0x4003).unwrap().line, 11);
    }

    /// An address belonging to nothing is unknown, not "the nearest line".
    #[test]
    fn an_address_in_no_row_is_unknown() {
        let m = map(&[(0, 10, 0x4000, 3)]);
        assert!(m.location_at(0x3fff).is_none());
        assert!(m.location_at(0x4003).is_none(), "one past the end");
        assert!(m.location_at(0xc000).is_none());
    }

    /// A macro body or a REPEAT emits the same line several times; each is a
    /// real address, and a breakpoint takes the first.
    #[test]
    fn a_line_emitted_several_times_keeps_every_address() {
        let m = map(&[(0, 7, 0x5000, 2), (0, 7, 0x5002, 2), (0, 7, 0x5004, 2)]);
        assert_eq!(
            m.addresses_at(Path::new("main.asm"), 7),
            &[0x5000, 0x5002, 0x5004]
        );
        assert_eq!(
            m.breakpoint_at(Path::new("main.asm"), 7).unwrap().address,
            0x5000
        );
        // ...and each repetition still maps back to that one line.
        assert_eq!(m.location_at(0x5003).unwrap().line, 7);
    }

    /// A breakpoint on a comment slides to the next line with code, and says
    /// where it went.
    #[test]
    fn a_breakpoint_slides_to_the_next_line_with_code() {
        let m = map(&[(0, 10, 0x4000, 0), (0, 11, 0x4000, 0), (0, 12, 0x4000, 3)]);
        let placed = m.breakpoint_at(Path::new("main.asm"), 10).unwrap();
        assert_eq!(placed.address, 0x4000);
        assert_eq!(placed.line, 12, "the dot moves to where the code is");
    }

    /// Nothing to slide to.
    #[test]
    fn a_breakpoint_past_all_code_is_refused() {
        let m = map(&[(0, 10, 0x4000, 3)]);
        assert!(m.breakpoint_at(Path::new("main.asm"), 900).is_none());
    }

    /// Several files, and the right one wins.
    #[test]
    fn files_do_not_bleed_into_each_other() {
        let m = map(&[(0, 5, 0x4000, 2), (1, 5, 0x6000, 2)]);
        assert_eq!(m.addresses_at(Path::new("main.asm"), 5), &[0x4000]);
        assert_eq!(m.addresses_at(Path::new("inc.asm"), 5), &[0x6000]);
        assert_eq!(
            m.location_at(0x6001).unwrap().file,
            PathBuf::from("inc.asm")
        );
    }

    /// An unknown file is not silently attributed to a known one.
    #[test]
    fn an_unknown_file_has_no_addresses() {
        let m = map(&[(0, 5, 0x4000, 2)]);
        assert!(m.addresses_at(Path::new("elsewhere.asm"), 5).is_empty());
        assert!(m.breakpoint_at(Path::new("elsewhere.asm"), 5).is_none());
    }

    /// Rows arrive in assembly order, not address order - an `org` moving
    /// backwards must not break the reverse lookup.
    #[test]
    fn an_org_moving_backwards_still_resolves() {
        let m = map(&[(0, 10, 0x8000, 2), (0, 20, 0x4000, 2)]);
        assert_eq!(m.location_at(0x8001).unwrap().line, 10);
        assert_eq!(m.location_at(0x4001).unwrap().line, 20);
    }

    /// A long run (an `incbin`) covers every address inside it.
    #[test]
    fn a_long_run_covers_its_whole_extent() {
        let m = map(&[(0, 30, 0x2000, 1024)]);
        assert_eq!(m.location_at(0x2000).unwrap().line, 30);
        assert_eq!(m.location_at(0x23ff).unwrap().line, 30);
        assert!(m.location_at(0x2400).is_none());
    }
}

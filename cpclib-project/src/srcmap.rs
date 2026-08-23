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
//!
//! A CPC with extra memory asks a third question the other two hide: *which*
//! `&4000`? Code assembled into page 5 and code assembled into the base 64K
//! share every logical address they use, so a row records the page it was
//! assembled for as well. When two pages claim one address and nothing has
//! said which is currently banked in, the honest answer is to say so rather
//! than to pick the first one and highlight a line from the wrong bank.

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
    pub line: u32,
    /// 1-based columns the instruction occupies on that line.
    ///
    /// A line is often several instructions - `ld a,l : inc a : ld (.p),a` is
    /// three - so "line 42" alone puts the cursor at the start of all three.
    /// These let the editor highlight the one actually executing.
    #[serde(default)]
    pub column: u32,
    #[serde(default)]
    pub column_end: u32,
    /// Whether the assembler recorded this row as data (`db`/`defs`/`defw`/
    /// `incbin`/a string) rather than an instruction - see
    /// `cpclib_asm::SourceMapRow::is_data`.
    #[serde(default)]
    pub is_data: bool,
    /// How many bytes this row's span covers - `span.end - span.start`, an
    /// exact 1:1 reconstruction since `from_raw` builds one `Span` per
    /// `SourceMapRow`.
    ///
    /// Not `line_extent_at`: that fixpoints across every row sharing a line,
    /// which would fuse a line mixing a code token and a data token
    /// (`nop: db 1,2,3`) into one extent - exactly what `is_data` needs kept
    /// apart, per-row.
    #[serde(default)]
    pub len: u32
}

/// Where a breakpoint actually went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakpointPlacement {
    pub address: u32,
    /// The line it ended up on, which may be later than the one asked for.
    pub line: u32,
    /// The page that line was assembled into.
    pub page: u8
}

/// What a logical address resolves to.
///
/// Distinguishes "no source here" from "source here, but in more than one
/// bank" - two answers a plain `Option` would flatten into the same `None`,
/// and only one of them is worth telling the user about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressResolution {
    /// No row covers it: firmware, data, or a stray address.
    Unknown,
    Line(SourceLocation),
    /// Several banks hold code at this logical address. Which one is running
    /// depends on the banking state, which the emulator does not report.
    Ambiguous {
        pages: Vec<u8>,
        /// What each page would say, so a caller that *does* know the banking
        /// can still resolve it.
        candidates: Vec<(u8, SourceLocation)>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Span {
    start: u32,
    end: u32,
    file: u16,
    line: u32,
    page: u8,
    column: u16,
    column_end: u16,
    is_data: bool
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
    /// Of those, the ones that are a *place in the program* rather than a
    /// number that happens to equal one.
    ///
    /// A label has an address; `equ`/`=` defines a value, and a value equal to
    /// 0x8F2A is not the routine at 0x8F2A. Both belong in `symbols` - you can
    /// watch either - but only the first should ever name a call frame.
    #[serde(default)]
    address_symbols: std::collections::HashSet<String>,
    files: Vec<PathBuf>,
    /// `(file, line) -> every address that line occupies`, in assembly order.
    /// A macro body called five times has five.
    forward: HashMap<(u16, u32), Vec<u32>>,
    /// Sorted by `start`, for binary search.
    spans: Vec<Span>,
    /// Set when at least one logical address is claimed by more than one page,
    /// computed once at build time: it is asked on every stop and answered in
    /// a launch notice, and recomputing it would mean walking every span.
    #[serde(default)]
    banked_ambiguity: bool
}

/// Whether `s` ends with `suffix`, ignoring ASCII case - without allocating a
/// lowercased copy of `s` the way `s.to_ascii_lowercase().ends_with(...)`
/// would, on a check made for every candidate name at every stop. Compared
/// char by char from the end rather than byte-sliced, so a label with a
/// multi-byte character near the end cannot panic on a mid-character split.
fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
    let mut chars = s.chars().rev();
    suffix
        .chars()
        .rev()
        .all(|want| chars.next().is_some_and(|have| have.eq_ignore_ascii_case(&want)))
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
                line: row.line,
                page: row.page,
                column: row.column,
                column_end: row.column_end,
                is_data: row.is_data
            });
        }
        spans.sort_unstable_by_key(|s| (s.start, s.end, s.page));

        // Do any two pages emit at the same logical address? Sorted spans make
        // this one linear sweep over neighbours rather than a search per
        // address.
        let banked_ambiguity = spans
            .windows(2)
            .any(|pair| pair[0].page != pair[1].page && pair[1].start < pair[0].end);

        Self {
            symbols: HashMap::new(),
            address_symbols: std::collections::HashSet::new(),
            files,
            forward,
            spans,
            banked_ambiguity
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

    /// Which of the symbols are real addresses - see `address_symbols`.
    pub fn with_address_symbols(mut self, addresses: std::collections::HashSet<String>) -> Self {
        self.address_symbols = addresses;
        self
    }

    /// The address a label stands for, matched case-insensitively as a
    /// fallback - basm is case-sensitive by default but a user typing a watch
    /// expression is not thinking about that.
    pub fn address_of_symbol(&self, name: &str) -> Option<u32> {
        if let Some(address) = self.symbols.get(name) {
            return Some(*address);
        }
        // No allocation on either side: this is the common path for a
        // mistyped watch expression, checked against every symbol in the
        // program before giving up.
        self.symbols
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map(|(_, address)| *address)
    }

    /// The label standing exactly at `address`, if there is one.
    ///
    /// A linear scan: the reverse question is asked a few dozen times per stop
    /// against a table of a few thousand, which is nothing, and an index would
    /// have to be kept in step with `serde` round trips for no gain.
    /// Every label at `address`, best guess first.
    ///
    /// More than one is the normal case, not an oddity: the end of a table is
    /// the start of the routine after it. When the caller has evidence of
    /// which one is meant - the text of the `call` that jumped there, say - it
    /// should use this and decide for itself; `symbol_at` is the answer for
    /// when there is no such evidence.
    pub fn symbols_at(&self, address: u32) -> Vec<&str> {
        let mut found: Vec<&str> = self
            .symbols
            .iter()
            .filter(|(_, at)| **at == address)
            .map(|(name, _)| name.as_str())
            .collect();
        found.sort_by_key(|name| self.preference(name));
        found
    }

    /// How good a name this is for an address, lower being better.
    ///
    /// Called on every candidate at every stop, so the tie-break is a
    /// borrowed `&str` rather than an owned copy of the name - `min_by_key`/
    /// `sort_by_key` below need only `Ord`, which `&str` gives them for free.
    fn preference<'a>(&self, name: &'a str) -> (bool, bool, usize, &'a str) {
        // A real label first: an `equ` equal to this address is a number, not
        // a place, and naming a frame after one points the reader at something
        // the program never entered.
        let is_value = !self.address_symbols.is_empty() && !self.address_symbols.contains(name);
        let is_end = ends_with_ignore_case(name, "_end")
            || ends_with_ignore_case(name, ".end")
            || ends_with_ignore_case(name, "_fin");
        (is_value, is_end, name.len(), name)
    }

    pub fn symbol_at(&self, address: u32) -> Option<&str> {
        // Several labels routinely share one address: the end of a table is
        // the start of the routine after it. `find` over a `HashMap` picked
        // whichever the hasher happened to yield first, so a call frame was
        // named `PLY_AKG_PeriodTable_End` instead of the routine actually
        // entered - and differently between runs.
        //
        // Preferred, in order: a name that does not read as the *end* of
        // something (an end marker names where code stops, never where it
        // starts), then the shortest, then alphabetical so the answer is at
        // least stable.
        self.symbols
            .iter()
            .filter(|(_, at)| **at == address)
            .min_by_key(|(name, _)| self.preference(name))
            .map(|(name, _)| name.as_str())
    }

    /// The nearest label at or before `address`, and how far past it that is.
    ///
    /// `screen_buffer+3` is what a register holding a pointer into a buffer
    /// should read as; `None` when nothing is within `window`, because a label
    /// two kilobytes back says nothing about where the register points.
    pub fn symbol_near(&self, address: u32, window: u32) -> Option<(&str, u32)> {
        self.symbols
            .iter()
            .filter(|(_, at)| **at <= address && address - **at <= window)
            .min_by_key(|(_, at)| address - **at)
            .map(|(name, at)| (name.as_str(), address - *at))
    }

    /// Every known label, for completion and for listing what can be watched.
    pub fn symbols(&self) -> impl Iterator<Item = (&str, u32)> {
        self.symbols
            .iter()
            .map(|(name, address)| (name.as_str(), *address))
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
                let page = self
                    .spans
                    .iter()
                    .find(|s| s.file == id && s.line == candidate && s.start == address)
                    .map(|s| s.page)
                    .unwrap_or(0);
                return Some(BreakpointPlacement {
                    address,
                    line: candidate,
                    page
                });
            }
        }
        None
    }

    /// Which source line an address belongs to, or `None` when it belongs to
    /// none - which is a real answer, not a failure.
    ///
    /// Also `None` when more than one page claim the address: see
    /// [`Self::resolution_at`] for the distinction. Returning a line from an
    /// arbitrary bank would be worse than returning nothing, because the
    /// editor would highlight it and the user would believe it.
    pub fn location_at(&self, address: u32) -> Option<SourceLocation> {
        match self.resolution_at(address) {
            AddressResolution::Line(location) => Some(location),
            _ => None
        }
    }

    /// Every page holding code at this logical address, with what each would
    /// say the line is.
    ///
    /// One entry means the answer is certain. Several means the banking state
    /// decides, and something outside the map has to choose between them -
    /// comparing the bytes really in memory against each page's image is what
    /// the debugger does.
    pub fn candidates_at(&self, address: u32) -> Vec<(u8, SourceLocation)> {
        let index = self.spans.partition_point(|s| s.start <= address);
        let covering = self.spans[..index]
            .iter()
            .rev()
            .filter(|s| address >= s.start && address < s.end);

        // Within one page the *latest* span wins, as it always has: a macro
        // expanded five times, or an `ORG` rewind, legitimately puts several
        // rows on one address and the last one assembled is what is there.
        let mut candidates: Vec<(u8, SourceLocation)> = Vec::new();
        for span in covering {
            if candidates.iter().any(|(page, _)| *page == span.page) {
                continue;
            }
            let Some(file) = self.files.get(span.file as usize)
            else {
                continue;
            };
            candidates.push((
                span.page,
                SourceLocation {
                    file: file.clone(),
                    line: span.line,
                    column: span.column as u32,
                    column_end: span.column_end as u32,
                    is_data: span.is_data,
                    len: span.end - span.start
                }
            ));
        }
        candidates.sort_by_key(|(page, _)| *page);
        candidates
    }

    /// The full answer for a logical address, ambiguity included.
    pub fn resolution_at(&self, address: u32) -> AddressResolution {
        let mut candidates = self.candidates_at(address);
        match candidates.len() {
            0 => AddressResolution::Unknown,
            1 => AddressResolution::Line(candidates.pop().unwrap().1),
            _ => {
                AddressResolution::Ambiguous {
                    pages: candidates.iter().map(|(page, _)| *page).collect(),
                    candidates
                }
            },
        }
    }

    /// Which source line a *long* address belongs to - a page plus the 16-bit
    /// address the Z80 sees.
    ///
    /// This is the question `location_at` cannot answer on a paged program.
    /// Nothing reports banking to us yet, but the map holds the answer for
    /// when something does, and the disassembly path can use it whenever the
    /// page is known from where the bytes were read.
    pub fn location_at_long(&self, page: u8, address: u16) -> Option<SourceLocation> {
        let address = address as u32;
        let index = self.spans.partition_point(|s| s.start <= address);
        let span = self.spans[..index]
            .iter()
            .rev()
            .find(|s| s.page == page && address >= s.start && address < s.end)?;
        Some(SourceLocation {
            file: self.files.get(span.file as usize)?.clone(),
            line: span.line,
            column: span.column as u32,
            column_end: span.column_end as u32,
            is_data: span.is_data,
            len: span.end - span.start
        })
    }

    /// The unbroken run of bytes the source line covering `address` occupies.
    ///
    /// A line is not one instruction and not one row: `ld a,l : inc a` is two
    /// rows, and `defs 59` is one row 59 bytes long. Both are still *the line*,
    /// so anything asking what the line at `PC` costs has to see the whole run
    /// rather than the row `PC` happens to sit in.
    ///
    /// Contiguity is what keeps this honest. The same line inside a macro body
    /// or a `repeat` emits at several unrelated addresses, and only the run
    /// containing `address` is the one being executed - the others belong to
    /// other iterations and summing them would price a loop as if it ran once
    /// per copy.
    pub fn line_extent_at(&self, page: u8, address: u16) -> Option<std::ops::Range<u32>> {
        let address = address as u32;
        let anchor = self
            .spans
            .iter()
            .find(|s| s.page == page && address >= s.start && address < s.end)?;
        let (file, line) = (anchor.file, anchor.line);
        let mut extent = anchor.start..anchor.end;

        // Grow to a fixpoint rather than in one pass: rows of one line are not
        // required to appear in address order, so a row that does not touch
        // the run yet may touch it once another row has widened it.
        let mut growing = true;
        while growing {
            growing = false;
            for span in &self.spans {
                if span.page != page || span.file != file || span.line != line {
                    continue;
                }
                // `end` is exclusive, so touching includes merely abutting.
                if span.end < extent.start || span.start > extent.end {
                    continue;
                }
                if span.start < extent.start {
                    extent.start = span.start;
                    growing = true;
                }
                if span.end > extent.end {
                    extent.end = span.end;
                    growing = true;
                }
            }
        }
        Some(extent)
    }

    /// Whether this program puts code from different pages at the same logical
    /// address - the condition under which `location_at` has to give up.
    pub fn has_banked_ambiguity(&self) -> bool {
        self.banked_ambiguity
    }

    /// Every page this program emitted code into, in order.
    pub fn pages(&self) -> Vec<u8> {
        let mut pages: Vec<u8> = self.spans.iter().map(|s| s.page).collect();
        pages.sort_unstable();
        pages.dedup();
        pages
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
mod symbol_at_tests {
    use super::*;

    fn map_with(symbols: &[(&str, u32)]) -> SourceMap {
        SourceMap::default().with_symbols(
            symbols
                .iter()
                .map(|(name, at)| (name.to_string(), *at))
                .collect()
        )
    }

    /// An `equ` is a number, not a place.
    ///
    /// `SPRITE_BASE equ 0x8F2A` and a routine that really starts at 0x8F2A are
    /// both "symbols at 0x8F2A", but only one of them is somewhere the program
    /// can be. Naming a call frame after the constant sends the reader to a
    /// definition the program never entered.
    #[test]
    fn a_real_label_beats_a_value_that_equals_it() {
        let map = map_with(&[("SPRITE_BASE", 0x8F2A), ("draw_sprite", 0x8F2A)])
            .with_address_symbols(["draw_sprite".to_string()].into_iter().collect());
        assert_eq!(map.symbol_at(0x8F2A), Some("draw_sprite"));
    }

    /// Being a real label outranks the other preferences, not the reverse.
    #[test]
    fn a_real_end_label_still_beats_a_value() {
        let map = map_with(&[("SHORT", 0x8F2A), ("table_end", 0x8F2A)])
            .with_address_symbols(["table_end".to_string()].into_iter().collect());
        assert_eq!(map.symbol_at(0x8F2A), Some("table_end"));
    }

    /// Without the distinction recorded, the older preferences still apply -
    /// a map from before this was carried is not made worse by it.
    #[test]
    fn nothing_recorded_means_nothing_is_demoted() {
        let map = map_with(&[("PLY_Table_End", 0x8F2A), ("move_along_curve", 0x8F2A)]);
        assert_eq!(map.symbol_at(0x8F2A), Some("move_along_curve"));
    }

    /// The end of a table is not the routine that starts there.
    ///
    /// Reported from a real call stack: the frame read
    /// `PLY_AKG_PeriodTable_End` where the program had entered
    /// `spectral_sprite_move_along_curve`. Both labels are at that address;
    /// the arbitrary one won.
    #[test]
    fn a_routine_beats_the_end_of_whatever_precedes_it() {
        let map = map_with(&[
            ("PLY_AKG_PeriodTable_End", 0x8F2A),
            ("spectral_sprite_move_along_curve", 0x8F2A)
        ]);
        assert_eq!(
            map.symbol_at(0x8F2A),
            Some("spectral_sprite_move_along_curve")
        );
    }

    /// With nothing to tell them apart, the answer is at least the same every
    /// time - an unstable name is a name nobody can report a bug about.
    #[test]
    fn the_choice_is_stable() {
        let names = [("draw_sprite", 0x4000), ("draw_sprite_alias", 0x4000)];
        for _ in 0..8 {
            assert_eq!(map_with(&names).symbol_at(0x4000), Some("draw_sprite"));
        }
    }

    /// An address nothing claims is still nothing.
    #[test]
    fn an_unclaimed_address_has_no_symbol() {
        assert_eq!(map_with(&[("start", 0x4000)]).symbol_at(0x5000), None);
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
                .map(|&(file, line, logical, len)| SourceMapRow::flat(file, line, logical, len))
                .collect()
        })
    }

    /// A line's extent is the whole unbroken run it emitted, not the one row
    /// the address happens to land in.
    #[test]
    fn a_lines_extent_covers_every_row_it_emitted() {
        // `ld a,0 : ld b,0` on line 3: two rows, one line.
        let map = map(&[(0, 3, 0x4000, 2), (0, 3, 0x4002, 2), (0, 4, 0x4004, 1)]);

        assert_eq!(map.line_extent_at(0, 0x4000), Some(0x4000..0x4004));
        assert_eq!(map.line_extent_at(0, 0x4003), Some(0x4000..0x4004));
        assert_eq!(map.line_extent_at(0, 0x4004), Some(0x4004..0x4005));
    }

    /// One row 59 bytes long - a `defs` run - is one extent, from anywhere
    /// inside it.
    #[test]
    fn an_address_inside_a_long_row_still_finds_the_whole_row() {
        let map = map(&[(0, 4, 0x4002, 60)]);

        assert_eq!(map.line_extent_at(0, 0x4020), Some(0x4002..0x403E));
    }

    /// A line emitted twice - a macro body, a `repeat` - has one extent per
    /// copy, and only the copy being executed is it.
    #[test]
    fn a_line_emitted_twice_does_not_merge_its_copies() {
        let map = map(&[(0, 3, 0x4000, 1), (0, 3, 0x5000, 1)]);

        assert_eq!(map.line_extent_at(0, 0x4000), Some(0x4000..0x4001));
        assert_eq!(map.line_extent_at(0, 0x5000), Some(0x5000..0x5001));
    }

    /// A line in another page is another line, however identical its address.
    #[test]
    fn an_extent_never_crosses_a_page() {
        let map = banked_map(&[(0, 3, 0x4000, 0, 2), (0, 3, 0x4002, 5, 2)]);

        assert_eq!(map.line_extent_at(0, 0x4000), Some(0x4000..0x4002));
        assert_eq!(map.line_extent_at(5, 0x4002), Some(0x4002..0x4004));
    }

    /// An address no row covers has no extent - which is a real answer, not a
    /// failure.
    #[test]
    fn an_unmapped_address_has_no_extent() {
        let map = map(&[(0, 3, 0x4000, 2)]);

        assert_eq!(map.line_extent_at(0, 0x9000), None);
    }

    /// Same shape, but each row also says which page it was assembled into.
    fn banked_map(rows: &[(u16, u32, u32, u8, u16)]) -> SourceMap {
        SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".to_string(), "inc.asm".to_string()],
            rows: rows
                .iter()
                .map(|&(file, line, logical, page, len)| {
                    SourceMapRow {
                        file,
                        line,
                        logical,
                        physical: logical + page as u32 * 0x1_0000,
                        page,
                        column: 1,
                        column_end: 1,
                        len,
                        is_data: false
                    }
                })
                .collect()
        })
    }

    /// The same logical address in two banks resolves to neither by itself.
    ///
    /// Picking one would highlight a line the CPU is provably not executing
    /// half the time, and the user has no way to tell.
    #[test]
    fn one_address_in_two_banks_is_reported_as_ambiguous() {
        let m = banked_map(&[(0, 10, 0x4000, 0, 3), (1, 77, 0x4000, 5, 3)]);
        assert!(m.has_banked_ambiguity());
        assert_eq!(m.pages(), vec![0, 5]);
        assert_eq!(m.location_at(0x4000), None, "no guess is made");

        match m.resolution_at(0x4001) {
            AddressResolution::Ambiguous { pages, candidates } => {
                assert_eq!(pages, vec![0, 5]);
                assert_eq!(candidates.len(), 2);
                assert_eq!(candidates[0].1.line, 10);
                assert_eq!(candidates[1].1.line, 77);
            },
            other => panic!("expected an ambiguity, got {other:?}")
        }
    }

    /// Told the page, it answers precisely.
    #[test]
    fn a_long_address_resolves_to_exactly_one_line() {
        let m = banked_map(&[(0, 10, 0x4000, 0, 3), (1, 77, 0x4000, 5, 3)]);
        assert_eq!(m.location_at_long(0, 0x4001).unwrap().line, 10);
        assert_eq!(m.location_at_long(5, 0x4001).unwrap().line, 77);
        assert_eq!(
            m.location_at_long(3, 0x4001),
            None,
            "a page that emitted nothing here answers nothing"
        );
    }

    /// A program that never banks is unaffected: no ambiguity, and the plain
    /// answer still comes out.
    #[test]
    fn a_single_page_program_keeps_its_plain_answers() {
        let m = banked_map(&[(0, 10, 0x4000, 0, 3), (0, 11, 0x4003, 0, 1)]);
        assert!(!m.has_banked_ambiguity());
        assert_eq!(m.location_at(0x4001).unwrap().line, 10);
        assert_eq!(m.pages(), vec![0]);
    }

    /// Two pages using *different* address ranges is not ambiguity - it is
    /// just a program with more memory, and every address still answers.
    #[test]
    fn pages_that_do_not_overlap_are_not_ambiguous() {
        let m = banked_map(&[(0, 10, 0x4000, 0, 3), (1, 77, 0x8000, 5, 3)]);
        assert!(!m.has_banked_ambiguity());
        assert_eq!(m.location_at(0x4001).unwrap().line, 10);
        assert_eq!(m.location_at(0x8001).unwrap().line, 77);
    }

    /// One page assembling twice over the same address (a macro, an `ORG`
    /// rewind) is the old behaviour and must stay: the last row assembled is
    /// what is really there.
    #[test]
    fn a_repeated_address_in_one_page_still_takes_the_last_row() {
        let m = banked_map(&[(0, 10, 0x4000, 0, 3), (0, 90, 0x4000, 0, 3)]);
        assert!(!m.has_banked_ambiguity());
        assert_eq!(m.location_at(0x4000).unwrap().line, 90);
    }

    #[test]
    fn a_line_maps_to_its_address_and_back() {
        let m = map(&[(0, 10, 0x4000, 3), (0, 11, 0x4003, 1)]);
        assert_eq!(m.addresses_at(Path::new("main.asm"), 10), &[0x4000]);
        assert_eq!(
            m.location_at(0x4000).unwrap(),
            SourceLocation {
                file: PathBuf::from("main.asm"),
                line: 10,
                column: 1,
                column_end: 1,
                is_data: false,
                len: 3
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
        assert!(m.location_at(0x3FFF).is_none());
        assert!(m.location_at(0x4003).is_none(), "one past the end");
        assert!(m.location_at(0xC000).is_none());
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
        assert_eq!(m.location_at(0x23FF).unwrap().line, 30);
        assert!(m.location_at(0x2400).is_none());
    }
}

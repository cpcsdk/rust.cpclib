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
    /// Logical address of the first byte - what the Z80 sees, and what a
    /// breakpoint is expressed in.
    pub logical: u32,
    /// Where those bytes really live: `offset_in_cpc()`, so the same logical
    /// address in two banks gives two different values. Without this, code at
    /// `C0.4000` and code at `C5.4000` are indistinguishable.
    pub physical: u32,
    /// RAM page holding the bytes (`0` for the base 64K).
    pub page: u8,
    /// 1-based columns this row's instruction occupies on its line.
    ///
    /// A line is often several instructions - `ld a,l : inc a : ld (.p),a` is
    /// three - and each gets its own row, so a debugger can point at the one
    /// executing rather than at the start of all three.
    pub column: u16,
    pub column_end: u16,
    /// How many bytes this row emitted. Zero for a line that produced none
    /// (an `EQU`, a comment) - kept, because "this line exists but has no
    /// address" is a different answer from "unknown line".
    pub len: u16
}

impl SourceMapRow {
    /// A row in the base 64K, where logical and physical coincide - the shape
    /// most programs have everywhere, and every program has somewhere.
    pub fn flat(file: u16, line: u32, logical: u32, len: u16) -> Self {
        Self {
            file,
            line,
            logical,
            physical: logical,
            page: 0,
            column: 1,
            column_end: 1,
            len
        }
    }
}

/// The real file behind a parser context name.
///
/// `main.asm:289:5 > MACRO SPRITE_BODY:` is `main.asm`. Anything that does not
/// have that shape is already a file name and is returned untouched.
///
/// The `:LINE:COL` is what makes this safe on Windows: a drive letter is `C:`
/// followed by a separator, never by digits-colon-digits-space-`>`.
fn real_file_name(name: &str) -> &str {
    let Some(marker) = name.find(" > ")
    else {
        return name;
    };
    let head = &name[..marker];

    // Walk back over `:COL` then `:LINE`, both of which must be all digits.
    let Some((rest, column)) = head.rsplit_once(':')
    else {
        return name;
    };
    if column.is_empty() || !column.bytes().all(|b| b.is_ascii_digit()) {
        return name;
    }
    let Some((path, line)) = rest.rsplit_once(':')
    else {
        return name;
    };
    if line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) || path.is_empty() {
        return name;
    }
    path
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
    ///
    /// The name is normalised first: code assembled inside a macro expansion is
    /// recorded by the parser against a *context* rather than a file, spelled
    /// `path/to/file.asm:289:5 > MACRO NAME:`. That is the right thing for a
    /// listing to print and the wrong thing entirely for a debugger, which
    /// tries to open it and reports "no such file". The line numbers in those
    /// rows are already the real file's, so the file name is the only part that
    /// needs recovering - and recovering it here rather than downstream means a
    /// macro body's rows share an id with the rest of their file, which is what
    /// lets a breakpoint inside a macro be placed at all.
    pub fn file_id(&mut self, name: &str) -> u16 {
        let name = real_file_name(name);
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
    #[allow(clippy::too_many_arguments)]
    pub fn push(
        &mut self,
        file: u16,
        line: u32,
        logical: u32,
        physical: u32,
        page: u8,
        column: u16,
        column_end: u16,
        len: u16
    ) {
        self.rows.push(SourceMapRow {
            file,
            line,
            logical,
            physical,
            page,
            column,
            column_end,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_macro_context_yields_the_file_it_is_written_in() {
        assert_eq!(
            real_file_name("src/spectral_sprites.asm:289:5 > MACRO SPRITE_BODY:"),
            "src/spectral_sprites.asm"
        );
        assert_eq!(real_file_name("<INLINE>:1:2 > MACRO DRAW:"), "<INLINE>");
    }

    #[test]
    fn a_plain_file_name_is_untouched() {
        assert_eq!(real_file_name("main.asm"), "main.asm");
        assert_eq!(real_file_name(""), "");
        assert_eq!(
            real_file_name(r"C:\demo\main.asm"),
            r"C:\demo\main.asm",
            "a drive letter is not a line number"
        );
    }

    /// A file whose *name* contains " > " is still that file: only the
    /// `:LINE:COL > ` shape means a parser context.
    #[test]
    fn only_the_line_column_shape_is_stripped() {
        assert_eq!(real_file_name("weird > name.asm"), "weird > name.asm");
        assert_eq!(
            real_file_name("a.asm:x:y > MACRO M:"),
            "a.asm:x:y > MACRO M:"
        );
    }

    /// Two rows from the same file - one inside a macro body, one not - share
    /// an id. That is what lets a breakpoint inside a macro body be placed at
    /// all, and what stops the debugger trying to open a context name as a
    /// path: "could not load source 'demo.asm:12:3 > MACRO DRAW:'".
    #[test]
    fn a_macro_body_shares_its_file_s_id() {
        let mut collector = SourceMapCollector::new();
        let outer = collector.file_id("demo.asm");
        let inside = collector.file_id("demo.asm:12:3 > MACRO DRAW:");
        assert_eq!(outer, inside);
        assert_eq!(collector.snapshot().files, vec!["demo.asm".to_string()]);
    }
}

//! Where each source line ended up, kept rather than printed.
//!
//! The listing already computes everything a debugger needs to map a line to
//! an address and back (which file, which line, what address, how many
//! bytes), and then formats it into text and forgets it. This collects the
//! same records instead, as a side channel: `--lst` and a source map are not
//! mutually exclusive, and asking for one must not change the other's output.
//!
//! Rows arrive in assembly order, so a `REPEAT` body or a macro called five
//! times naturally produces five rows for the same source line. That is
//! correct and load-bearing: each is a distinct address the line occupies.

use std::collections::{BTreeMap, HashMap};

/// One emitted run of bytes, and the source line it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    pub len: u16,
    /// Whether this row is a data directive (`db`/`defs`/`defw`/`incbin`/a
    /// string) rather than an instruction.
    ///
    /// `#[serde(default)]` rather than a `VERSION` bump: an old map missing
    /// this field is not wrong, it is simply silent on a question it never
    /// asked - it reads back as `false`, today's existing behaviour, rather
    /// than forcing every cached `--sourcemap` file to be thrown away for a
    /// purely additive piece of information. Direct precedent in this crate:
    /// `SourceMapFile::address_symbols`.
    #[serde(default)]
    pub is_data: bool
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
            len,
            is_data: false
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
pub(crate) fn real_file_name(name: &str) -> &str {
    split_expansion(name).map(|(path, _)| path).unwrap_or(name)
}

/// How far to move a line recorded inside an expansion to reach the file's own
/// numbering.
///
/// A macro or struct body is re-parsed as a source of its own, so the lines in
/// it count from the body rather than from the file - `0` for anything that is
/// not an expansion, so an ordinary file is untouched.
///
/// The two differ by one, which is a fact about where each body's text starts
/// rather than a choice: a `MACRO` body keeps the newline that ends its
/// `macro` line, so its line 1 *is* the definition line; a `STRUCT` body does
/// not, so its line 1 is the line after. Both are pinned by tests in
/// `cpclib-asm/tests/source_map_probe.rs`.
pub(crate) fn expansion_line_offset(name: &str) -> u32 {
    let Some((_, line)) = split_expansion(name)
    else {
        return 0;
    };
    if name[..name.find(" > ").unwrap_or(0)].is_empty() {
        return 0;
    }
    match name.contains(" > STRUCT ") {
        true => line,
        false => line.saturating_sub(1)
    }
}

/// Whether a parser context name is one of an expansion rather than a file.
///
/// The `:LINE:COL > ` shape is set only when a macro or struct body is
/// re-parsed as a source of its own, so it is also what says the spans in it
/// count from the body rather than from the file.
pub(crate) fn is_expansion_context(name: &str) -> bool {
    split_expansion(name).is_some()
}

/// `main.asm:289:5 > MACRO SPRITE_BODY:` into `("main.asm", 289)`.
///
/// `None` for anything that is not a parser context of that shape - which is
/// every ordinary file name, and is why this is safe to run over all of them.
/// The `:LINE:COL` is what makes it safe on Windows: a drive letter is `C:`
/// followed by a separator, never by digits-colon-digits-space-`>`.
fn split_expansion(name: &str) -> Option<(&str, u32)> {
    let marker = name.find(" > ")?;
    let head = &name[..marker];

    // Walk back over `:COL` then `:LINE`, both of which must be all digits.
    let (rest, column) = head.rsplit_once(':')?;
    if column.is_empty() || !column.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (path, line) = rest.rsplit_once(':')?;
    if line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) || path.is_empty() {
        return None;
    }
    Some((path, line.parse().ok()?))
}

/// The rows, plus the file table they index into.
///
/// Serialisable so an assemble can be *kept*: a build that already produced
/// this can hand it to a debugger instead of making it assemble the whole
/// program a second time to learn the same thing. See `basm --sourcemap`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RawSourceMap {
    pub files: Vec<String>,
    pub rows: Vec<SourceMapRow>
}

/// A source map as a file: the map itself, and the symbol table a debugger
/// needs beside it to turn a label into an address.
///
/// Written by `basm --sourcemap`, read by anything that would otherwise
/// re-assemble the program to rebuild both.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SourceMapFile {
    /// What produced it, so a reader can refuse a file from another version
    /// rather than mis-parse it.
    pub version: String,
    pub map: RawSourceMap,
    /// Label to address, as `Env` knows them at the end of the assemble.
    pub symbols: HashMap<String, u32>,
    /// The `-D` definitions the program was assembled with, verbatim and
    /// sorted.
    ///
    /// A map is only valid for the program that produced it, and `-D` is
    /// exactly how one source tree produces *different* programs - a demo
    /// assembled with another picture, another music, another sprite size is a
    /// different binary with the same file names. Nothing else in this file
    /// would notice: the version matches, the paths match, every line looks
    /// plausible and every address is wrong. So they are recorded, and a
    /// reader that knows which definitions it wants can check.
    #[serde(default)]
    pub definitions: BTreeMap<String, String>,
    /// The `BREAKPOINT` directives the program itself carries.
    ///
    /// Here because a debugger that has this file must not have to assemble
    /// the program again to learn them - which is the whole point - and
    /// because they are the one thing it cannot recover from the snapshot: an
    /// emulator does not adopt the breakpoints a `.sna` carries.
    #[serde(default)]
    pub breakpoints: Vec<crate::assembler::delayed_command::AssembledBreakpoint>,
    /// The assembled bytes, base64, for telling two pages apart by what is
    /// really in memory.
    ///
    /// Base64 rather than an array of numbers: 128K of JSON integers is half a
    /// megabyte of text and slow to parse, for bytes nobody reads by eye.
    #[serde(default)]
    pub image: String,
    /// Where the program starts, when it says.
    #[serde(default)]
    pub entry_point: Option<u16>,
    /// Which of `symbols` are real addresses rather than `equ`/`=` values.
    ///
    /// Both are worth watching, so both are in `symbols`; only a label is
    /// somewhere the program can *be*, so only a label should name a call
    /// frame. Recorded here so a session that reads this file makes the same
    /// distinction as one that assembled.
    #[serde(default)]
    pub address_symbols: std::collections::BTreeSet<String>
}

impl SourceMapFile {
    /// The version this build of the assembler writes and accepts.
    pub const VERSION: &'static str = "cpclib-source-map-1";

    pub fn new(
        map: RawSourceMap,
        symbols: HashMap<String, u32>,
        definitions: BTreeMap<String, String>
    ) -> Self {
        Self {
            version: Self::VERSION.to_string(),
            map,
            symbols,
            definitions,
            breakpoints: Vec::new(),
            image: String::new(),
            entry_point: None,
            address_symbols: std::collections::BTreeSet::new()
        }
    }

    /// Everything else a debugger would otherwise assemble the program to
    /// learn: its `BREAKPOINT` directives, its bytes, and where it starts.
    pub fn with_program(
        mut self,
        breakpoints: Vec<crate::assembler::delayed_command::AssembledBreakpoint>,
        image: &[u8],
        entry_point: Option<u16>
    ) -> Self {
        self.breakpoints = breakpoints;
        self.image = base64_encode(image);
        self.entry_point = entry_point;
        self
    }

    /// Note which symbols are real addresses - see `address_symbols`.
    pub fn with_address_symbols(mut self, addresses: std::collections::BTreeSet<String>) -> Self {
        self.address_symbols = addresses;
        self
    }

    /// The assembled bytes, decoded.
    pub fn image_bytes(&self) -> Vec<u8> {
        base64_decode(&self.image)
    }

    /// Whether this map was made with exactly these definitions.
    ///
    /// Exactly: a definition the reader does not know about is as much of a
    /// mismatch as one it has and the file does not, because either way the
    /// program on disc is not the program this map describes.
    pub fn assembled_with(&self, definitions: &BTreeMap<String, String>) -> bool {
        // Both sides unquoted at comparison time, not merely on the way in.
        // `-DFACE=\"face3\"` reaches the assembler as `"face3"` and reaches a
        // build-file reader as `face3`; they are the same definition, and a
        // map written before this was normalised is still a valid map. Doing
        // it here means the check does not depend on which version wrote the
        // file - it depends only on what the definitions mean.
        let bare = |values: &BTreeMap<String, String>| -> BTreeMap<String, String> {
            values
                .iter()
                .map(|(name, value)| (name.clone(), unquoted(value).to_string()))
                .collect()
        };
        bare(&self.definitions) == bare(definitions)
    }

    /// The `-D` arguments a command line carries, in this file's shape.
    ///
    /// `NAME=value`, or `NAME` alone for the bare form basm reads as `1`.
    pub fn definitions_from_arguments<I, S>(arguments: I) -> BTreeMap<String, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>
    {
        arguments
            .into_iter()
            .map(|argument| {
                let argument = argument.as_ref();
                match argument.split_once('=') {
                    Some((name, value)) => (name.to_string(), unquoted(value).to_string()),
                    None => (argument.to_string(), "1".to_string())
                }
            })
            .collect()
    }

    /// `None` for a file written by a different version - re-assembling is
    /// slow, and a map that does not match the program is worse than slow.
    pub fn from_json(text: &str) -> Option<Self> {
        let parsed: Self = serde_json::from_str(text).ok()?;
        (parsed.version == Self::VERSION).then_some(parsed)
    }
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
        len: u16,
        is_data: bool
    ) {
        self.rows.push(SourceMapRow {
            file,
            line,
            logical,
            physical,
            page,
            column,
            column_end,
            len,
            is_data
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

#[cfg(test)]
mod source_map_file_tests {
    use super::*;

    fn a_map() -> RawSourceMap {
        RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![SourceMapRow {
                file: 0,
                line: 12,
                logical: 0x4000,
                physical: 0x4000,
                page: 0,
                column: 2,
                column_end: 9,
                len: 3,
                is_data: false
            }]
        }
    }

    /// The whole point: an assemble that has been done once can be read back
    /// instead of done again.
    #[test]
    fn a_written_map_reads_back_the_same() {
        let mut symbols = HashMap::new();
        symbols.insert("start".to_string(), 0x4000u32);
        let definitions = SourceMapFile::definitions_from_arguments(["FACE=\"face3\"", "DEBUG"]);
        let written =
            serde_json::to_string(&SourceMapFile::new(a_map(), symbols, definitions)).unwrap();

        let read = SourceMapFile::from_json(&written).expect("reads back");
        assert_eq!(read.map.files, vec!["main.asm".to_string()]);
        assert_eq!(read.map.rows.len(), 1);
        assert_eq!(read.map.rows[0].line, 12);
        assert_eq!(read.map.rows[0].logical, 0x4000);
        assert_eq!(read.map.rows[0].column_end, 9);
        assert_eq!(read.symbols.get("start"), Some(&0x4000));
        // The *value*, not the quoting the shell carried it in - that is the
        // form whoever reads this back has.
        assert_eq!(
            read.definitions.get("FACE").map(String::as_str),
            Some("face3")
        );
        assert_eq!(
            read.definitions.get("DEBUG").map(String::as_str),
            Some("1"),
            "a bare -D is the 1 basm reads it as"
        );
    }

    /// Quoting is not a difference, whichever side carries it.
    ///
    /// A map written before the values were normalised still holds
    /// `"\"face3\""` where a build file reader has `face3`. Those are the same
    /// definition, and refusing the map over it sends the user to re-assemble
    /// a program that was perfectly well described.
    #[test]
    fn quoting_is_not_a_difference() {
        let mut quoted = BTreeMap::new();
        quoted.insert("FACE".to_string(), "\"face3\"".to_string());
        quoted.insert("WIDTH".to_string(), "24".to_string());
        let file = SourceMapFile {
            definitions: quoted,
            ..SourceMapFile::new(a_map(), HashMap::new(), BTreeMap::new())
        };

        let mut bare = BTreeMap::new();
        bare.insert("FACE".to_string(), "face3".to_string());
        bare.insert("WIDTH".to_string(), "24".to_string());
        assert!(
            file.assembled_with(&bare),
            "same definitions, other quoting"
        );

        bare.insert("FACE".to_string(), "face4".to_string());
        assert!(
            !file.assembled_with(&bare),
            "and a real difference still is one"
        );
    }

    /// A map made with other `-D` values describes another program.
    ///
    /// The same sources with `-DFACE=\"face4\"` assemble to different bytes at
    /// different addresses, under the same file names - so every line in the
    /// map looks plausible and every address is wrong. Nothing else in the
    /// file would catch it: the version matches and the paths match.
    #[test]
    fn a_map_made_with_other_definitions_is_recognised() {
        let file = SourceMapFile::new(
            a_map(),
            HashMap::new(),
            SourceMapFile::definitions_from_arguments(["FACE=\"face3\"", "SPRITE_WIDTH=24"])
        );

        assert!(
            file.assembled_with(&SourceMapFile::definitions_from_arguments([
                "FACE=\"face3\"",
                "SPRITE_WIDTH=24"
            ]))
        );
        // Order is not a difference; the values are.
        assert!(
            file.assembled_with(&SourceMapFile::definitions_from_arguments([
                "SPRITE_WIDTH=24",
                "FACE=\"face3\""
            ]))
        );
        assert!(
            !file.assembled_with(&SourceMapFile::definitions_from_arguments([
                "FACE=\"face4\"",
                "SPRITE_WIDTH=24"
            ])),
            "another picture is another program"
        );
        // One missing on either side is still a mismatch.
        assert!(
            !file.assembled_with(&SourceMapFile::definitions_from_arguments([
                "FACE=\"face3\""
            ]))
        );
        assert!(
            !file.assembled_with(&SourceMapFile::definitions_from_arguments([
                "FACE=\"face3\"",
                "SPRITE_WIDTH=24",
                "EXTRA=1"
            ]))
        );
    }

    /// A file from another version is refused rather than half-understood.
    ///
    /// Re-assembling is slow, which is the whole reason this file exists - but
    /// a map that does not match the program is worse than slow, because every
    /// line it reports is wrong and nothing says so.
    #[test]
    fn a_map_from_another_version_is_refused() {
        let mut file = SourceMapFile::new(a_map(), HashMap::new(), BTreeMap::new());
        file.version = "cpclib-source-map-0".into();
        let written = serde_json::to_string(&file).unwrap();
        assert!(SourceMapFile::from_json(&written).is_none());
    }

    /// The program travels with the map, so a debugger needs nothing else.
    #[test]
    fn the_program_itself_survives_the_round_trip() {
        let file = SourceMapFile::new(a_map(), HashMap::new(), BTreeMap::new()).with_program(
            vec![],
            &[0x3E, 0x01, 0x00, 0xC9, 0xFF],
            Some(0x4000)
        );
        let read =
            SourceMapFile::from_json(&serde_json::to_string(&file).unwrap()).expect("reads back");
        assert_eq!(read.image_bytes(), vec![0x3E, 0x01, 0x00, 0xC9, 0xFF]);
        assert_eq!(read.entry_point, Some(0x4000));
    }

    /// Base64 of every length, since the padding is where it goes wrong.
    #[test]
    fn every_length_of_image_comes_back_unchanged() {
        for len in 0..40usize {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 7 + 3) as u8).collect();
            let file = SourceMapFile::new(a_map(), HashMap::new(), BTreeMap::new()).with_program(
                vec![],
                &bytes,
                None
            );
            assert_eq!(file.image_bytes(), bytes, "length {len}");
        }
    }

    /// And so is anything that is not one of these at all.
    #[test]
    fn nonsense_is_refused() {
        assert!(SourceMapFile::from_json("{}").is_none());
        assert!(SourceMapFile::from_json("not json").is_none());
    }
}

/// A definition's *value*, without the shell quoting around it.
///
/// `-DFACE=\"face3\"` reaches the assembler as `"face3"` and means the string
/// `face3`. Whoever reads this file back has the value, not the command line
/// that carried it, so storing the quotes made every comparison fail - which
/// is worse than not comparing at all, because it looks like a mismatch.
fn unquoted(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
    {
        &value[1..value.len() - 1]
    }
    else {
        value
    }
}

/// Base64, without a dependency for sixty lines of table lookup.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0)
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 0x3F] as char);
        out.push(ALPHABET[(n >> 12) as usize & 0x3F] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 0x3F] as char
        }
        else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 0x3F] as char
        }
        else {
            '='
        });
    }
    out
}

fn base64_decode(text: &str) -> Vec<u8> {
    let value = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None
        }
    };
    // The `=` padding needs no counting: a trailing chunk of 2 or 3 digits is
    // itself how many bytes it carries, and subtracting the padding on top of
    // that removed real bytes.
    let digits: Vec<u32> = text.bytes().filter_map(value).collect();
    let mut out = Vec::with_capacity(digits.len() / 4 * 3);
    for chunk in digits.chunks(4) {
        let mut n = 0u32;
        for (i, digit) in chunk.iter().enumerate() {
            n |= digit << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    out
}

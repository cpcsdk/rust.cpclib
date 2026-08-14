use std::collections::HashMap;
use std::sync::LazyLock;
use cpclib_z80flow::InstructionCost;

use super::overflow::format_value_like_source;

include!(concat!(env!("OUT_DIR"), "/timings_generated.rs"));

// Index by uppercase mnemonic for O(1) lookup
static BY_MNEMONIC: LazyLock<HashMap<&'static str, Vec<&'static TimingEntry>>> =
    LazyLock::new(|| {
        let mut m: HashMap<&'static str, Vec<&'static TimingEntry>> = HashMap::new();
        for e in TIMINGS {
            m.entry(e.mnemonic).or_default().push(e);
        }
        m
    });

/// The 8 Z80 "block repeat" instructions: their `(nops, nops_alt)` pair
/// (e.g. LDIR's `6 / 5 for last iteration`) does *not* mean "branch taken/
/// not taken" like every other dual-value entry in the table (`djnz`/
/// `jr cc`/`call cc`/`ret cc`) - it means "6 NOPs per iteration except the
/// last, which costs 5", for a loop that repeats 1-65536 times (BC=0 wraps
/// to 65536). The generic taken/not-taken framing is wrong for these: the
/// real worst case isn't a fixed small number, it's unbounded unless BC's
/// value is statically known. See `is_block_repeat`/`block_repeat_total_nops`.
const BLOCK_REPEAT_MNEMONICS: &[&str] = &[
    "LDIR", "LDDR", "CPIR", "CPDR", "INIR", "INDR", "OTIR", "OTDR"
];

pub(super) fn is_block_repeat(mnemonic: &str) -> bool {
    BLOCK_REPEAT_MNEMONICS.contains(&mnemonic)
}

/// How many times a block-repeat instruction actually iterates for a given
/// `bc` value entering it. `bc == 0` is the real Z80 wraparound case (the
/// loop runs 65536 times, not zero).
fn block_repeat_iterations(bc: i32) -> u32 {
    let bc = (bc as u32) & 0xFFFF;
    if bc == 0 { 65536 } else { bc }
}

/// The exact NOP cost of a block-repeat instruction given a statically-known
/// `bc` value entering it - `per_iteration` NOPs for each iteration but the
/// last, which costs `last_iteration`.
fn block_repeat_total_nops(bc: i32, per_iteration: u8, last_iteration: u8) -> u32 {
    let iterations = block_repeat_iterations(bc);
    (iterations - 1) * per_iteration as u32 + last_iteration as u32
}

/// The four entries where `i`/`r` name the Z80's special interrupt-vector
/// and refresh registers *literally* (`LD I,A`/`LD R,A`/`LD A,I`/`LD A,R`),
/// not the generic 8-bit register-class placeholder that a bare `r` means
/// everywhere else in this table. Since the literal and the placeholder
/// happen to share the exact same spelling, `match_op`'s generic
/// per-operand scoring can't tell them apart from text alone: for a source
/// instruction like `ld b,a`, its second operand `a` earns the placeholder
/// pattern `ld r,r'` no advantage over the literal pattern `ld r,a` (both
/// treat `a` as matching), while `ld r,a`'s *first* operand additionally
/// gets to treat `b` as satisfying its own `r`-class check too — so
/// `ld r,a` (meant only for the real `LD R,A`) actually out-scores the
/// intended generic match and wins outright. Handled as an exact
/// whole-instruction match instead of routing through the generic scorer,
/// which is otherwise correct for every other pattern in the table.
const LITERAL_IR_REGISTER_PATTERNS: &[&str] = &["ld i,a", "ld r,a", "ld a,i", "ld a,r"];

/// One instruction's NOP cost, straight from `data/timings.txt`.
///
/// The single lookup behind every cost source in this crate - the two in
/// `cycles.rs` and `stabilize.rs`, and the "what does this real opcode cost"
/// half both of them answer for `cpclib-z80flow` so that a *fake* instruction
/// can be priced by what it assembles to.
///
/// Takes text rather than a token on purpose: it is asked both about real
/// `LocatedToken`s and about synthetic `Token`s that no one wrote, and text is
/// what the table is keyed by.
pub fn nops_of(instruction_text: &str) -> InstructionCost {
    match find_timings(instruction_text).first() {
        Some(entry) => {
            match entry.nops_alt {
                Some(alt) => {
                    InstructionCost::Conditional {
                        taken: entry.nops as u32,
                        not_taken: alt as u32
                    }
                },
                None => InstructionCost::Fixed(entry.nops as u32)
            }
        },
        None => InstructionCost::Unknown
    }
}

/// Given raw instruction text from the source line (e.g. `"LD A, (IX+5)"`),
/// return the best-matching timing entries.
pub fn find_timings(instruction_text: &str) -> Vec<&'static TimingEntry> {
    let text = instruction_text.split(';').next().unwrap_or("").trim();
    if text.is_empty() {
        return vec![];
    }
    let (mnemonic_raw, operands_raw) = split_head(text);
    let mnemonic = mnemonic_raw.to_uppercase();

    let candidates = match BY_MNEMONIC.get(mnemonic.as_str()) {
        Some(v) => v,
        None => return vec![]
    };

    if candidates.len() == 1 {
        return candidates.to_vec();
    }

    let src_ops = parse_ops(operands_raw);
    let mut scored: Vec<(i32, &'static TimingEntry)> = candidates
        .iter()
        .map(|e| {
            let (_, pat_rest) = split_head(e.pattern);
            let pat_ops = parse_ops(pat_rest);
            let entry_score = if LITERAL_IR_REGISTER_PATTERNS.contains(&e.pattern) {
                if src_ops == pat_ops { 1000 } else { -1000 }
            }
            else {
                score(&src_ops, &pat_ops)
            };
            (entry_score, *e)
        })
        .collect();

    scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    let top = scored.first().map(|(s, _)| *s).unwrap_or(-1);

    if top < 0 {
        candidates.to_vec()
    }
    else {
        scored
            .iter()
            .filter(|(s, _)| *s == top)
            .map(|(_, e)| *e)
            .collect()
    }
}

/// Split `without_comment` into `:`-separated segment byte ranges, ignoring
/// any `:` found inside parens (e.g. `(ix+n)` never contains one in
/// practice, but this stays robust regardless). Shared by
/// `extract_instruction_at_col` (find the one segment under a cursor) and
/// `classify_line` (classify every segment on the line).
fn split_segments(without_comment: &str) -> Vec<(usize, usize)> {
    let bytes = without_comment.as_bytes();
    let mut depth = 0u32;
    let mut seg_start = 0usize;
    let mut segments = Vec::new();

    for i in 0..=bytes.len() {
        let at_end = i == bytes.len();
        let is_sep = !at_end && bytes[i] == b':' && depth == 0;

        if at_end || is_sep {
            segments.push((seg_start, i));
            if is_sep {
                seg_start = i + 1;
            }
        }
        else {
            match bytes[i] {
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    segments
}

/// Extract the instruction text that the cursor (at `col`) is on.
///
/// Lines may contain multiple instructions separated by `:`.  This function
/// identifies which segment `col` falls into, then strips any leading label
/// from that segment and returns the mnemonic + operands as a trimmed string.
pub fn extract_instruction_at_col(line: &str, col: usize) -> Option<String> {
    // Drop comment — byte indices are still valid because the comment is a suffix.
    let without_comment = match line.find(';') {
        Some(i) => &line[..i],
        None => line
    };

    for (start, end) in split_segments(without_comment) {
        if col >= start && col <= end {
            let seg = without_comment[start..end].trim();
            return extract_mnemonic_from_segment(seg);
        }
    }
    None
}

/// What a single instruction slot on a line actually is, once any leading
/// label(s) have been skipped.
pub(super) enum LineSegment {
    /// A real Z80 instruction: mnemonic + operands, ready for `find_timings`.
    Instruction(String),
    /// A known assembler directive (`DB`/`ORG`/`EQU`/...) — zero execution
    /// cost, not a red flag.
    Directive,
    /// Blank, comment-only, or label-only — nothing to report.
    Blank,
    /// Leading word isn't a known instruction or directive — most likely a
    /// macro invocation, whose real cost can't be computed here.
    Unrecognized
}

/// Every instruction slot on `line` (a line may contain multiple
/// `:`-separated instructions), each classified — the whole-line
/// counterpart of `extract_instruction_at_col`, used by the selection
/// cycle-count feature to walk every instruction in a range rather than
/// just the one under a cursor.
///
/// Deliberately does **not** reuse `split_segments`/`extract_instruction_at_col`'s
/// naive "every top-level `:` is a separator" model: that model happens to
/// still work for cursor-position lookup (a bare label fragment left behind
/// by the split resolves to `None`, same as no instruction found there —
/// the cursor was never going to land exactly on a label with nothing
/// else), but it actively misclassifies here, since a label like `loop:`
/// would be treated as its own segment and wrongly flagged `Unrecognized`
/// instead of being absorbed into the instruction that follows it. Labels
/// must be skipped *before* deciding where the next separator is, not
/// after splitting on every colon uniformly.
pub(super) fn classify_line(line: &str) -> Vec<LineSegment> {
    let without_comment = match line.find(';') {
        Some(i) => &line[..i],
        None => line
    };
    let bytes = without_comment.as_bytes();
    let mut result = Vec::new();
    let mut pos = 0usize;

    loop {
        let content_start = skip_label_prefixes(without_comment, pos);
        let mut depth = 0u32;
        let mut i = content_start;
        while i < bytes.len() && !(bytes[i] == b':' && depth == 0) {
            match bytes[i] {
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                _ => {}
            }
            i += 1;
        }
        let content = without_comment[content_start..i].trim();
        result.push(classify_content(content));
        if i >= bytes.len() {
            break;
        }
        pos = i + 1; // past the separator ':'
    }
    result
}

/// From `pos`, skip whitespace then zero or more `identifier:` label
/// prefixes, returning the byte offset where real content (if any) begins.
fn skip_label_prefixes(s: &str, mut pos: usize) -> usize {
    let bytes = s.as_bytes();
    loop {
        while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'_' | b'.'))
        {
            pos += 1;
        }
        if pos == start {
            return start; // no word here - blank, or a stray separator
        }
        if pos < bytes.len() && bytes[pos] == b':' {
            pos += 1; // consume the label's own colon, keep looking for more labels
            continue;
        }
        return start; // real content begins at this word
    }
}

fn classify_content(content: &str) -> LineSegment {
    use super::token::{DIRECTIVE_SET, INSTRUCTION_SET};
    if content.is_empty() {
        return LineSegment::Blank;
    }
    let (mnemonic, _) = split_head(content);
    let mnemonic_upper = mnemonic.to_uppercase();
    if INSTRUCTION_SET.contains(mnemonic_upper.as_str()) {
        LineSegment::Instruction(content.to_string())
    }
    else if DIRECTIVE_SET.contains(mnemonic_upper.as_str()) {
        LineSegment::Directive
    }
    else {
        LineSegment::Unrecognized
    }
}

/// Given the text of a single segment (possibly `"label: LD A, B"` or just `"LD A, B"`),
/// skip any leading label definitions and return the mnemonic + operands.
fn extract_mnemonic_from_segment(seg: &str) -> Option<String> {
    use super::token::INSTRUCTION_SET;
    let (word_start, word) = leading_word(seg)?;
    if INSTRUCTION_SET.contains(word.to_uppercase().as_str()) {
        Some(seg[word_start..].trim().to_string())
    }
    else {
        None
    }
}

/// Skip whitespace, then skip one or more `label:` prefixes, returning the
/// byte offset and text of the first word that isn't itself immediately
/// followed by `:` (i.e. the mnemonic/directive/whatever-comes-after-labels,
/// or `None` if the segment is blank/label-only).
fn leading_word(seg: &str) -> Option<(usize, &str)> {
    let bytes = seg.as_bytes();
    let mut pos = 0usize;

    loop {
        // Skip whitespace
        while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }
        if pos >= bytes.len() {
            return None;
        }

        let start = pos;
        // Scan a word (alphanumeric + underscore + dot for local labels)
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'_' | b'.'))
        {
            pos += 1;
        }
        if pos == start {
            return None;
        }

        let word = &seg[start..pos];

        // If the word is immediately followed by `:` it is a label — skip it
        if pos < bytes.len() && bytes[pos] == b':' {
            pos += 1;
            continue;
        }

        return Some((start, word));
    }
}

// ─── formatting ─────────────────────────────────────────────────────────────

/// `src_ops`/`resolved` provide known operand values for pseudocode
/// substitution (see `render_pseudocode`) - pass empty slices when there's
/// no real parsed instruction to draw them from (e.g. the text-only
/// fallback path when the document doesn't currently parse), which just
/// shows each entry's pseudocode with its symbolic placeholders untouched.
/// `known_bc` is BC's statically-known value entering this instruction (via
/// `registers::register_state_at`), used only for the 8 block-repeat
/// mnemonics (`BLOCK_REPEAT_MNEMONICS`) to show an exact total instead of
/// "unbounded" - pass `None` when it isn't known or wasn't looked up.
pub fn format_hover(
    instruction_text: &str,
    entries: &[&TimingEntry],
    src_ops: &[String],
    resolved: &[Option<i32>],
    known_bc: Option<i32>
) -> String {
    let instr = instruction_text.trim();
    let mut md = format!("**{}**", instr);

    // Try to assemble and show the actual bytes produced
    if let Ok(bytes) = cpclib_asm::assemble(instr) {
        if !bytes.is_empty() {
            let hex: String = bytes
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            md.push_str(&format!(" → `{}`", hex));
        }
    }
    md.push_str("\n\n");

    for entry in entries {
        md.push_str("---\n");
        md.push_str(&format!("`{}`\n\n", entry.pattern));

        // NOPs
        match entry.nops_alt {
            None => {
                md.push_str(&format!(
                    "**{}** NOP{}\n\n",
                    entry.nops,
                    if entry.nops == 1 { "" } else { "s" }
                ))
            },
            Some(alt) if is_block_repeat(entry.mnemonic) => {
                match known_bc {
                    Some(bc) => {
                        let iterations = block_repeat_iterations(bc);
                        let total = block_repeat_total_nops(bc, entry.nops, alt);
                        md.push_str(&format!(
                            "**{total}** NOPs total (BC = {bc}, {iterations} iteration{}: \
                             {}× **{}**, then **{alt}** for the last iteration)\n\n",
                            if iterations == 1 { "" } else { "s" },
                            iterations - 1,
                            entry.nops
                        ));
                    },
                    None => {
                        md.push_str(&format!(
                            "**{alt}** NOPs for the last iteration, **{}** NOPs per iteration \
                             before that - BC's value isn't statically known here, so the total is \
                             unbounded (not just {alt}/{})\n\n",
                            entry.nops, entry.nops
                        ));
                    }
                }
            },
            Some(alt) => {
                md.push_str(&format!(
                    "**{}** / **{}** NOPs (taken/not taken)\n\n",
                    entry.nops, alt
                ))
            },
        }

        // Opcode template
        md.push_str(&format!("Opcodes: `{}`\n\n", entry.opcodes));

        // Flags
        let flags_str = describe_flags(entry.flags);
        md.push_str(&format!("Flags `{}`: {}\n", entry.flags, flags_str));

        // Notes in plain text (no bold/italic)
        if !entry.notes.is_empty() {
            md.push_str(&format!("\n{}\n", entry.notes));
        }

        // Pseudocode, with any known operand values substituted in.
        if let Some(pseudocode) = render_pseudocode(entry, src_ops, resolved) {
            md.push_str(&format!("\n`{pseudocode}`\n"));
        }
    }
    md
}

/// Substitute `entry.pattern`'s own operand-placeholder words (`r`, `rr`,
/// `n`, `nn`, ...) in `entry.pseudocode` with the real hovered operands:
/// register-class placeholders (`r`/`r'`/`r''`/`rr`/`qq`/`cc`/`ccc`) get the
/// operand's own source text (e.g. `SP`); immediate-class placeholders
/// (`n`/`nn`/`d`/`e`/`b`/`ttt`) get the *resolved* value when known
/// (`resolved[i]`), formatted in the same base the source wrote it in via
/// `format_value_like_source` (e.g. `&05` stays `&5`, not `5`) - falling
/// back to hexadecimal when the operand isn't a single bare literal (a
/// symbol reference or computed expression has no "original base" of its
/// own to preserve), same convention `overflow.rs` already established for
/// overflow-warning values. Otherwise the placeholder is left untouched
/// rather than showing something potentially wrong. A placeholder that
/// appears wrapped in one layer of parens in the pattern (e.g. `(n)` for an
/// I/O port address, `(nn)` for a memory address) still substitutes
/// correctly: the paren layer is stripped only for *classifying* the
/// placeholder, so `port(n)`'s bare `n` in the pseudocode text is what
/// actually gets replaced, and the surrounding parens are untouched literal
/// text.
///
/// Returns `None` when this entry has no pseudocode at all (a
/// `pseudocode.txt` gap - not every pattern is covered yet).
fn render_pseudocode(
    entry: &TimingEntry,
    src_ops: &[String],
    resolved: &[Option<i32>]
) -> Option<String> {
    if entry.pseudocode.is_empty() {
        return None;
    }

    let (_, pat_ops_text) = split_head(entry.pattern);
    let pat_ops = parse_ops(pat_ops_text);

    let mut replacements: Vec<(&str, String)> = Vec::new();
    for (i, placeholder) in pat_ops.iter().enumerate() {
        let Some(src) = src_ops.get(i)
        else {
            continue;
        };
        let bare = placeholder
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(placeholder.as_str());
        let replacement = match bare {
            "r" | "r'" | "r''" | "rr" | "qq" | "cc" | "ccc" => Some(src.to_uppercase()),
            "n" | "nn" | "d" | "e" | "b" | "ttt" => {
                resolved
                    .get(i)
                    .copied()
                    .flatten()
                    .map(|v| format_value_like_source(src, v))
            },
            _ => None
        };
        if let Some(replacement) = replacement {
            replacements.push((bare, replacement));
        }
    }

    if replacements.is_empty() {
        Some(entry.pseudocode.to_string())
    }
    else {
        Some(substitute_words(entry.pseudocode, &replacements))
    }
}

/// Replace whole-word occurrences of each `(word, replacement)` pair in
/// `text` - a "word" is a maximal run of ASCII alphanumerics/`'`, so this
/// never touches a placeholder letter that's merely a substring of a larger
/// word (e.g. the `r` in `Carry`, or the `n` in `port`).
fn substitute_words(text: &str, replacements: &[(&str, String)]) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = text[i..].chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            let start = i;
            let mut j = i;
            while j < bytes.len() {
                let cj = text[j..].chars().next().unwrap();
                if cj.is_ascii_alphanumeric() || cj == '\'' {
                    j += cj.len_utf8();
                }
                else {
                    break;
                }
            }
            let word = &text[start..j];
            match replacements.iter().find(|(k, _)| *k == word) {
                Some((_, replacement)) => result.push_str(replacement),
                None => result.push_str(word)
            }
            i = j;
        }
        else {
            result.push(c);
            i += c.len_utf8();
        }
    }
    result
}

pub(super) fn describe_flags(flags: &str) -> String {
    let names = ["S", "Z", "5", "H", "3", "V", "N", "C"];
    let mut modified = vec![];
    let mut forced0 = vec![];
    let mut forced1 = vec![];
    for (i, ch) in flags.chars().take(8).enumerate() {
        let n = *names.get(i).unwrap_or(&"?");
        match ch {
            '.' => {},
            '0' => forced0.push(n),
            '1' => forced1.push(n),
            _ => modified.push(n)
        }
    }
    let mut parts = vec![];
    if !modified.is_empty() {
        parts.push(format!("affected: **{}**", modified.join(" ")));
    }
    if !forced1.is_empty() {
        parts.push(format!("set: **{}**", forced1.join(" ")));
    }
    if !forced0.is_empty() {
        parts.push(format!("reset: **{}**", forced0.join(" ")));
    }
    if parts.is_empty() {
        "unchanged".to_string()
    }
    else {
        parts.join(" · ")
    }
}

// ─── internal helpers ────────────────────────────────────────────────────────

pub(super) fn split_head(s: &str) -> (&str, &str) {
    let s = s.trim();
    match s.find(|c: char| c.is_ascii_whitespace()) {
        Some(i) => (&s[..i], s[i..].trim()),
        None => (s, "")
    }
}

pub(super) fn parse_ops(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    let mut ops = Vec::new();
    let mut cur = String::new();
    let mut depth = 0u32;
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            },
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            },
            ',' if depth == 0 => {
                let t = cur.trim().to_ascii_lowercase();
                if !t.is_empty() {
                    ops.push(t);
                }
                cur = String::new();
            },
            _ => cur.push(c)
        }
    }
    let t = cur.trim().to_ascii_lowercase();
    if !t.is_empty() {
        ops.push(t);
    }
    ops
}

fn score(src: &[String], pat: &[String]) -> i32 {
    if src.len() != pat.len() {
        if src.is_empty() && pat.is_empty() {
            return 50;
        }
        return -100;
    }
    if src.is_empty() {
        return 50;
    }
    let mut total = 0i32;
    for (s, p) in src.iter().zip(pat.iter()) {
        let v = match_op(s, p);
        if v < 0 {
            return -100;
        }
        total += v;
    }
    total
}

fn match_op(src: &str, pat: &str) -> i32 {
    if src == pat {
        return 20;
    }

    const R8: &[&str] = &["a", "b", "c", "d", "e", "h", "l"];
    const R16: &[&str] = &["bc", "de", "hl", "sp"];
    const R16AF: &[&str] = &["bc", "de", "hl", "af"];
    const CC_SHORT: &[&str] = &["nz", "z", "nc", "c"];
    const CC_ALL: &[&str] = &["nz", "z", "nc", "c", "po", "pe", "p", "m"];

    match pat {
        "r" | "r'" | "r''" => {
            if R8.contains(&src) {
                10
            }
            else {
                -1
            }
        },
        "rr" => {
            if R16.contains(&src) {
                10
            }
            else {
                -1
            }
        },
        "qq" => {
            if R16AF.contains(&src) {
                10
            }
            else {
                -1
            }
        },
        "cc" => {
            if CC_SHORT.contains(&src) {
                10
            }
            else {
                -1
            }
        },
        "ccc" => {
            if CC_ALL.contains(&src) {
                10
            }
            else {
                -1
            }
        },
        "n" | "nn" | "d" | "e" | "b" | "ttt" => {
            if R8.contains(&src) || R16.contains(&src) || src.starts_with('(') {
                -1
            }
            else {
                5
            }
        },
        "(hl)" => {
            if src == "(hl)" {
                20
            }
            else {
                -1
            }
        },
        "(bc)" => {
            if src == "(bc)" {
                20
            }
            else {
                -1
            }
        },
        "(de)" => {
            if src == "(de)" {
                20
            }
            else {
                -1
            }
        },
        "(c)" => {
            if src == "(c)" {
                20
            }
            else {
                -1
            }
        },
        p if p.starts_with("(ix") => {
            if src.starts_with("(ix") {
                15
            }
            else {
                -1
            }
        },
        p if p.starts_with("(iy") => {
            if src.starts_with("(iy") {
                15
            }
            else {
                -1
            }
        },
        p if p.starts_with("(nn")
            || (p.starts_with('(') && !p.starts_with("(ix") && !p.starts_with("(iy")) =>
        {
            if src.starts_with('(')
                && !["(hl)", "(bc)", "(de)", "(c)"].contains(&src)
                && !src.starts_with("(ix")
                && !src.starts_with("(iy")
            {
                12
            }
            else {
                -1
            }
        },
        _ => 0
    }
}

#[cfg(test)]
mod find_timings_tests {
    use super::*;

    /// Regression test for a real, previously-live scoring bug: `ld X,a`/
    /// `ld a,X` (X a plain 8-bit register) used to resolve to the special
    /// `LD R,A`/`LD A,R` (refresh-register) timing entries instead of the
    /// generic 1-NOP `ld r,r'` form, because the literal `r` in those
    /// special patterns was indistinguishable from the `r` register-class
    /// placeholder used everywhere else in the table.
    #[test]
    fn generic_register_to_a_transfers_are_not_confused_with_the_special_ir_register_forms() {
        for instr in [
            "ld b,a", "ld a,b", "ld c,a", "ld a,c", "ld h,a", "ld a,h", "ld d,e"
        ] {
            let entries = find_timings(instr);
            assert_eq!(entries.len(), 1, "{instr}: {entries:?}");
            assert_eq!(entries[0].pattern, "ld r,r'", "{instr}: {entries:?}");
            assert_eq!(entries[0].nops, 1, "{instr}: {entries:?}");
        }
    }

    #[test]
    fn the_special_ir_register_forms_still_resolve_to_themselves() {
        for (instr, pattern) in [
            ("ld i,a", "ld i,a"),
            ("ld a,i", "ld a,i"),
            ("ld r,a", "ld r,a"),
            ("ld a,r", "ld a,r")
        ] {
            let entries = find_timings(instr);
            assert_eq!(entries.len(), 1, "{instr}: {entries:?}");
            assert_eq!(entries[0].pattern, pattern, "{instr}: {entries:?}");
            assert_eq!(entries[0].nops, 3, "{instr}: {entries:?}");
        }
    }

    #[test]
    fn otir_otdr_resolve_to_real_timing_data() {
        // Regression test for the `outir`/`outdr` -> `otir`/`otdr` spelling
        // fix in data/timings.txt.
        for instr in ["otir", "otdr"] {
            let entries = find_timings(instr);
            assert_eq!(entries.len(), 1, "{instr}: {entries:?}");
            assert_eq!(entries[0].nops, 6, "{instr}: {entries:?}");
            assert_eq!(entries[0].nops_alt, Some(5), "{instr}: {entries:?}");
        }
    }
}

#[cfg(test)]
mod format_hover_block_repeat_tests {
    use super::*;

    #[test]
    fn block_repeat_iterations_handles_the_bc_zero_wraparound() {
        assert_eq!(block_repeat_iterations(1), 1);
        assert_eq!(block_repeat_iterations(3), 3);
        assert_eq!(block_repeat_iterations(0), 65536);
    }

    #[test]
    fn block_repeat_total_nops_matches_the_documented_formula() {
        // BC=1: a single iteration, costing exactly the last-iteration NOPs.
        assert_eq!(block_repeat_total_nops(1, 6, 5), 5);
        // BC=3: two iterations at 6, one (the last) at 5.
        assert_eq!(block_repeat_total_nops(3, 6, 5), 2 * 6 + 5);
        // BC=0 wraps to 65536 iterations.
        assert_eq!(block_repeat_total_nops(0, 6, 5), 65535 * 6 + 5);
    }

    #[test]
    fn format_hover_shows_unbounded_for_ldir_without_a_known_bc() {
        let entries = find_timings("ldir");
        assert_eq!(entries.len(), 1, "{entries:?}");
        let md = format_hover("ldir", &entries, &[], &[], None);
        assert!(md.contains("unbounded"), "{md}");
        // Must not show the old, wrong "6 / 5" taken/not-taken framing.
        assert!(!md.contains("taken/not taken"), "{md}");
    }

    #[test]
    fn format_hover_shows_the_exact_total_for_ldir_with_a_known_bc() {
        let entries = find_timings("ldir");
        let md = format_hover("ldir", &entries, &[], &[], Some(3));
        assert!(md.contains("17"), "{md}"); // 2*6 + 5
        assert!(md.contains("3 iterations"), "{md}");
    }

    #[test]
    fn format_hover_handles_the_bc_zero_wraparound() {
        let entries = find_timings("ldir");
        let md = format_hover("ldir", &entries, &[], &[], Some(0));
        assert!(md.contains(&(65535 * 6 + 5).to_string()), "{md}");
        assert!(md.contains("65536 iterations"), "{md}");
    }

    #[test]
    fn format_hover_leaves_ordinary_conditional_instructions_unaffected() {
        // djnz is dual-valued (branch taken/not taken) but not a
        // block-repeat instruction - must keep the original framing.
        let entries = find_timings("djnz $");
        let md = format_hover("djnz $", &entries, &[], &[], None);
        assert!(md.contains("taken/not taken"), "{md}");
    }
}

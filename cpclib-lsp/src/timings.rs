use std::collections::HashMap;
use std::sync::LazyLock;

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
        None    => return vec![],
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
            (score(&src_ops, &pat_ops), *e)
        })
        .collect();

    scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    let top = scored.first().map(|(s, _)| *s).unwrap_or(-1);

    if top < 0 {
        candidates.to_vec()
    } else {
        scored.iter().filter(|(s, _)| *s == top).map(|(_, e)| *e).collect()
    }
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
        None    => line,
    };

    let bytes = without_comment.as_bytes();
    let mut depth = 0u32;
    let mut seg_start = 0usize;

    // Walk byte-by-byte, splitting on `:` at paren-depth 0
    for i in 0..=bytes.len() {
        let at_end    = i == bytes.len();
        let is_sep    = !at_end && bytes[i] == b':' && depth == 0;

        if at_end || is_sep {
            // Is the cursor inside [seg_start, i]?
            if col >= seg_start && col <= i {
                let seg = without_comment[seg_start..i].trim();
                return extract_mnemonic_from_segment(seg);
            }
            if is_sep {
                seg_start = i + 1;
            }
        } else {
            match bytes[i] {
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    None
}

/// Given the text of a single segment (possibly `"label: LD A, B"` or just `"LD A, B"`),
/// skip any leading label definitions and return the mnemonic + operands.
fn extract_mnemonic_from_segment(seg: &str) -> Option<String> {
    use crate::asm::INSTRUCTION_SET;
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

        // Check if this word is an instruction mnemonic
        if INSTRUCTION_SET.contains(word.to_uppercase().as_str()) {
            return Some(seg[start..].trim().to_string());
        }

        // Not a label and not a mnemonic — give up
        return None;
    }
}

// ─── formatting ─────────────────────────────────────────────────────────────

pub fn format_hover(instruction_text: &str, entries: &[&TimingEntry]) -> String {
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

        // NOPs / T-states
        match entry.nops_alt {
            None => md.push_str(&format!(
                "**{}** NOP{} ({} T-states)\n\n",
                entry.nops,
                if entry.nops == 1 { "" } else { "s" },
                entry.nops as u16 * 4
            )),
            Some(alt) => md.push_str(&format!(
                "**{}** / **{}** NOPs ({}/{} T-states — taken/not taken)\n\n",
                entry.nops,
                alt,
                entry.nops as u16 * 4,
                alt as u16 * 4
            )),
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
    }
    md
}

fn describe_flags(flags: &str) -> String {
    let names = ["S", "Z", "5", "H", "3", "V", "N", "C"];
    let mut modified = vec![];
    let mut forced0   = vec![];
    let mut forced1   = vec![];
    for (i, ch) in flags.chars().take(8).enumerate() {
        let n = *names.get(i).unwrap_or(&"?");
        match ch {
            '.' => {}
            '0' => forced0.push(n),
            '1' => forced1.push(n),
            _   => modified.push(n),
        }
    }
    let mut parts = vec![];
    if !modified.is_empty() { parts.push(format!("affected: **{}**", modified.join(" "))); }
    if !forced1.is_empty()  { parts.push(format!("set: **{}**",      forced1.join(" "))); }
    if !forced0.is_empty()  { parts.push(format!("reset: **{}**",    forced0.join(" "))); }
    if parts.is_empty() { "unchanged".to_string() } else { parts.join(" · ") }
}

// ─── internal helpers ────────────────────────────────────────────────────────

fn split_head(s: &str) -> (&str, &str) {
    let s = s.trim();
    match s.find(|c: char| c.is_ascii_whitespace()) {
        Some(i) => (&s[..i], s[i..].trim()),
        None    => (s, ""),
    }
}

fn parse_ops(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    let mut ops = Vec::new();
    let mut cur = String::new();
    let mut depth = 0u32;
    for c in s.chars() {
        match c {
            '(' => { depth += 1; cur.push(c); }
            ')' => { depth = depth.saturating_sub(1); cur.push(c); }
            ',' if depth == 0 => {
                let t = cur.trim().to_ascii_lowercase();
                if !t.is_empty() { ops.push(t); }
                cur = String::new();
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim().to_ascii_lowercase();
    if !t.is_empty() { ops.push(t); }
    ops
}

fn score(src: &[String], pat: &[String]) -> i32 {
    if src.len() != pat.len() {
        if src.is_empty() && pat.is_empty() { return 50; }
        return -100;
    }
    if src.is_empty() { return 50; }
    let mut total = 0i32;
    for (s, p) in src.iter().zip(pat.iter()) {
        let v = match_op(s, p);
        if v < 0 { return -100; }
        total += v;
    }
    total
}

fn match_op(src: &str, pat: &str) -> i32 {
    if src == pat { return 20; }

    const R8: &[&str]    = &["a", "b", "c", "d", "e", "h", "l"];
    const R16: &[&str]   = &["bc", "de", "hl", "sp"];
    const R16AF: &[&str] = &["bc", "de", "hl", "af"];
    const CC_SHORT: &[&str] = &["nz", "z", "nc", "c"];
    const CC_ALL: &[&str]   = &["nz", "z", "nc", "c", "po", "pe", "p", "m"];

    match pat {
        "r" | "r'" | "r''" => if R8.contains(&src) { 10 } else { -1 },
        "rr"  => if R16.contains(&src) { 10 } else { -1 },
        "qq"  => if R16AF.contains(&src) { 10 } else { -1 },
        "cc"  => if CC_SHORT.contains(&src) { 10 } else { -1 },
        "ccc" => if CC_ALL.contains(&src) { 10 } else { -1 },
        "n" | "nn" | "d" | "e" | "b" | "ttt" => {
            if R8.contains(&src) || R16.contains(&src) || src.starts_with('(') { -1 }
            else { 5 }
        }
        "(hl)" => if src == "(hl)" { 20 } else { -1 },
        "(bc)" => if src == "(bc)" { 20 } else { -1 },
        "(de)" => if src == "(de)" { 20 } else { -1 },
        "(c)"  => if src == "(c)"  { 20 } else { -1 },
        p if p.starts_with("(ix") => if src.starts_with("(ix") { 15 } else { -1 },
        p if p.starts_with("(iy") => if src.starts_with("(iy") { 15 } else { -1 },
        p if p.starts_with("(nn") || (p.starts_with('(') && !p.starts_with("(ix") && !p.starts_with("(iy")) => {
            if src.starts_with('(') && !["(hl)", "(bc)", "(de)", "(c)"].contains(&src)
               && !src.starts_with("(ix") && !src.starts_with("(iy")
            { 12 } else { -1 }
        }
        _ => 0,
    }
}

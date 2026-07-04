use super::Formatter;
use crate::options::SpaceAroundColumn;

impl<'src> Formatter<'src> {
    // Split "content ; comment" → (content.trim_end(), Option<"; comment">)
    pub(super) fn split_comment(line: &str) -> (&str, Option<&str>) {
        match line.find(';') {
            Some(pos) => (line[..pos].trim_end(), Some(line[pos..].trim_end())),
            None => (line, None),
        }
    }

    // Split `content` (already stripped of trailing comment) into `:` separated instruction
    // segments. Colons inside parentheses or double-quoted strings are not split points.
    // A `:` is only an instruction separator when BOTH the preceding and following bytes are
    // ASCII whitespace (or boundary): this avoids splitting label prefixes (`other: equ 5`),
    // global-scope paths (`jp ::label1`), and bare label colons (`myloop: ld a,0`).
    // Empty segments (e.g. from a trailing `:` on a label) are discarded.
    pub(super) fn split_instructions(content: &str) -> Vec<&str> {
        let mut result = Vec::new();
        let mut depth = 0i32;
        let mut in_string = false;
        // Track unmatched `?` so we can suppress splitting at the `:` of a ternary
        // expression (`cond ? then : else`).
        let mut ternary_depth = 0u32;
        let mut start = 0;
        let bytes = content.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if in_string {
                if b == b'"' { in_string = false; }
            } else {
                match b {
                    b'"' => in_string = true,
                    b'(' | b'[' => depth += 1,
                    b')' | b']' => { depth -= 1; if depth < 0 { depth = 0; } }
                    b'?' if depth == 0 => ternary_depth += 1,
                    b':' if depth == 0 => {
                        if ternary_depth > 0 {
                            // This `:` closes a ternary — not an instruction separator.
                            ternary_depth -= 1;
                        } else {
                            // Only split when prev byte is whitespace (or at start) AND
                            // next byte is whitespace or end-of-content.
                            let prev_ws = i == 0 || bytes[i - 1].is_ascii_whitespace();
                            let next_ws = i + 1 >= bytes.len() || bytes[i + 1].is_ascii_whitespace();
                            if prev_ws && next_ws {
                                let seg = content[start..i].trim();
                                if !seg.is_empty() { result.push(seg); }
                                start = i + 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        let last = content[start..].trim();
        if !last.is_empty() { result.push(last); }
        result
    }

    // Reformat `:` instruction separators in `content` according to `spacing`.
    // Only separators that are already surrounded by whitespace (` : `) are
    // recognised — label colons and other `:` uses are left untouched.
    // When `spacing` is `Untouched` the string is returned as-is.
    pub(super) fn normalize_colon_spacing(content: &str, spacing: SpaceAroundColumn) -> String {
        if matches!(spacing, SpaceAroundColumn::Untouched) {
            return content.to_string();
        }
        let segs = Self::split_instructions(content);
        if segs.len() <= 1 {
            return content.to_string();
        }
        let sep = match spacing {
            SpaceAroundColumn::None => ":",
            SpaceAroundColumn::Before => " :",
            SpaceAroundColumn::After => ": ",
            SpaceAroundColumn::Both => " : ",
            SpaceAroundColumn::Untouched => unreachable!(),
        };
        segs.join(sep)
    }

    // Reformat the assignment operator spacing in a `label [op]= value` statement.
    // Locates the first `=` and scans back over compound-operator prefix characters
    // (`+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<`, `>`) to find the full operator.
    // Whitespace on both sides of the operator is then replaced according to `spacing`.
    pub(super) fn normalize_assignment_spacing(content: &str, spacing: SpaceAroundColumn) -> String {
        if matches!(spacing, SpaceAroundColumn::Untouched) {
            return content.to_string();
        }
        let bytes = content.as_bytes();
        let is_op_prefix = |b: u8| matches!(b, b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'<' | b'>');
        let Some(eq_pos) = bytes.iter().position(|&b| b == b'=') else {
            return content.to_string();
        };
        // Find where the operator starts (scan back over prefix chars only, no whitespace).
        let mut op_start = eq_pos;
        while op_start > 0 && is_op_prefix(bytes[op_start - 1]) {
            op_start -= 1;
        }
        let label = content[..op_start].trim_end();
        let op    = &content[op_start..=eq_pos];
        let value = content[eq_pos + 1..].trim_start();
        let (sp_before, sp_after) = match spacing {
            SpaceAroundColumn::None   => ("", ""),
            SpaceAroundColumn::Before => (" ", ""),
            SpaceAroundColumn::After  => ("", " "),
            SpaceAroundColumn::Both   => (" ", " "),
            SpaceAroundColumn::Untouched => unreachable!(),
        };
        format!("{}{}{}{}{}", label, sp_before, op, sp_after, value)
    }

    // Initialise the per-line segment cache when we move to a new source line.
    // Must be called at the top of format_token (after warning/comment guards).
    pub(super) fn init_segments_for_line(&mut self, line_0: usize) {
        if self.seg_line == line_0 { return; }
        let src = self.source_lines.get(line_0).copied().unwrap_or("");
        let (content, trailing) = Self::split_comment(src.trim());
        self.seg_items = Self::split_instructions(content)
            .into_iter().map(str::to_string).collect();
        self.seg_trailing = trailing.map(str::to_string);
        self.seg_idx = 0;
        self.seg_line = line_0;
    }
}

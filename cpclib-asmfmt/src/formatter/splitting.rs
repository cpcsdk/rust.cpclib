use super::Formatter;
use crate::options::SpaceAroundColumn;

impl<'src> Formatter<'src> {
    // Split "content ; comment" → (content.trim_end(), Option<"; comment">)
    pub(super) fn split_comment(line: &str) -> (&str, Option<&str>) {
        match line.find(';') {
            Some(pos) => (line[..pos].trim_end(), Some(line[pos..].trim_end())),
            None => (line, None)
        }
    }

    // Split `content` (already stripped of trailing comment) into `:` separated instruction
    // segments. Colons inside parentheses or double-quoted strings are not split points.
    //
    // Only used for the two places that must keep a whole physical line intact rather than
    // splitting it into several output lines (`one_instruction_per_line = false`): reformatting
    // `:` spacing in place (`normalize_colon_spacing`), and pulling a same-line instruction's
    // text out from after a label's own name (`format_label`). Deciding *where a token actually
    // ends* for real content - which is what `one_instruction_per_line = true` (the default)
    // needs - is answered by the real parser's own token spans instead (`format_simple`,
    // `format_label`): asking the assembler, not re-guessing its grammar from text, is what
    // settles a `NOP:NOP:NOP:NOP` chain, a numeric operand butting against `:`, or a plain
    // identifier operand that happens to be spelled like a label, all of which this function's
    // own text-only heuristic (still applied below, since these two remaining uses have no
    // parsed tokens of their own to consult) cannot always get right.
    //
    // A `:` is an instruction separator when the following byte is ASCII whitespace (or
    // end-of-content) AND either the preceding byte is whitespace too, or the word ending
    // right at the `:` could not possibly be a label name anyway (it starts with a digit -
    // `0x7F54:`, `100:`, `&C0:` - the byte right before the `:` isn't an identifier character
    // at all - `dknr3 (void):`, closed by `)` - it isn't the first word of its statement, so it
    // must be an operand, not a label - `AND SOME_CONST:` - or it's a reserved Z80 mnemonic,
    // which can't be a label at all - `NOP:NOP:NOP:NOP`). This still leaves genuine label
    // colons alone (`other: equ 5`, `myloop: ld a,0`) and global-scope paths (`jp ::label1`,
    // caught by `next_ws` alone: the second `:` isn't followed by whitespace either).
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
                if b == b'"' {
                    in_string = false;
                }
            }
            else {
                match b {
                    b'"' => in_string = true,
                    b'(' | b'[' => depth += 1,
                    b')' | b']' => {
                        depth -= 1;
                        if depth < 0 {
                            depth = 0;
                        }
                    },
                    b'?' if depth == 0 => ternary_depth += 1,
                    b':' if depth == 0 => {
                        if ternary_depth > 0 {
                            // This `:` closes a ternary — not an instruction separator.
                            ternary_depth -= 1;
                        }
                        else {
                            // Split when the next byte is whitespace/end-of-content AND
                            // either the prev byte is whitespace too, or the word ending here
                            // cannot be a label name in the first place (see this function's
                            // own doc comment for why).
                            let prev_ws = i == 0 || bytes[i - 1].is_ascii_whitespace();
                            let next_ws =
                                i + 1 >= bytes.len() || bytes[i + 1].is_ascii_whitespace();
                            let could_be_label = !prev_ws && Self::word_could_start_a_label(bytes, start, i);
                            if (prev_ws || !could_be_label) && next_ws {
                                let seg = content[start..i].trim();
                                if !seg.is_empty() {
                                    result.push(seg);
                                }
                                start = i + 1;
                            }
                        }
                    },
                    _ => {}
                }
            }
            i += 1;
        }
        let last = content[start..].trim();
        if !last.is_empty() {
            result.push(last);
        }
        result
    }

    // Whether the word ending at byte offset `colon_pos` (exclusive) could be a
    // label definition, given the current statement began at `stmt_start`: walks
    // back over identifier-continuation bytes (alnum, `_`, `.`, `@`) to find where
    // the word starts, then requires BOTH that its first byte could start an
    // identifier AND that nothing but whitespace precedes it within this statement.
    // The second half matters as much as the first: `SPECTRAL_MASK` in
    // `AND SPECTRAL_MASK: ld (HL), A` is a perfectly identifier-shaped word, but it
    // is `AND`'s *operand*, not a label - a label can only be a statement's first
    // word. Without this a plain operand that happens to be a bare symbol name
    // (extremely common - most operands are) was mistaken for a label and the line
    // never split, exactly like the digit-starting/punctuation-preceded cases this
    // function's own doc comment already covers.
    fn word_could_start_a_label(bytes: &[u8], stmt_start: usize, colon_pos: usize) -> bool {
        let mut j = colon_pos;
        while j > 0
            && matches!(bytes[j - 1], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'.' | b'@')
        {
            j -= 1;
        }
        if j >= colon_pos
            || !matches!(bytes[j], b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'.' | b'@')
            || !bytes[stmt_start..j].iter().all(u8::is_ascii_whitespace)
        {
            return false;
        }
        // Even as the very first word of a statement, a Z80 mnemonic can't be a
        // label - it's a reserved word, not an identifier. Without this,
        // `NOP:NOP:NOP:NOP` (a common cycle-exact NOP-padding idiom - see this
        // project's own "timing in NOPs" convention) never split at all: `NOP` is
        // the first word, starts with a letter, nothing precedes it - lexically
        // indistinguishable from a real label unless the word itself is checked
        // against the reserved set.
        !Self::is_reserved_mnemonic(&bytes[j..colon_pos])
    }

    // Standard Z80 mnemonics (undocumented/pseudo opcodes aside) - a fixed,
    // decades-stable instruction set, so a hand-maintained list carries none of
    // the "will drift out of sync" risk a fast-moving language's keyword list
    // would. Checked case-insensitively against the bare word only (no operands),
    // since that's all `word_could_start_a_label` ever hands it.
    const RESERVED_MNEMONICS: &'static [&'static str] = &[
        "ADC", "ADD", "AND", "BIT", "CALL", "CCF", "CP", "CPD", "CPDR", "CPI", "CPIR", "CPL",
        "DAA", "DEC", "DI", "DJNZ", "EI", "EX", "EXX", "HALT", "IM", "IN", "INC", "IND", "INDR",
        "INI", "INIR", "JP", "JR", "LD", "LDD", "LDDR", "LDI", "LDIR", "NEG", "NOP", "OR", "OTDR",
        "OTIR", "OUT", "OUTD", "OUTI", "POP", "PUSH", "RES", "RET", "RETI", "RETN", "RL", "RLA",
        "RLC", "RLCA", "RLD", "RR", "RRA", "RRC", "RRCA", "RRD", "RST", "SBC", "SCF", "SET",
        "SLA", "SRA", "SRL", "SUB", "XOR"
    ];

    fn is_reserved_mnemonic(word: &[u8]) -> bool {
        Self::RESERVED_MNEMONICS
            .iter()
            .any(|m| m.len() == word.len() && m.as_bytes().eq_ignore_ascii_case(word))
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
            SpaceAroundColumn::Untouched => unreachable!()
        };
        segs.join(sep)
    }

    // Reformat the assignment operator spacing in a `label [op]= value` statement.
    // Locates the first `=` and scans back over compound-operator prefix characters
    // (`+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<`, `>`) to find the full operator.
    // Whitespace on both sides of the operator is then replaced according to `spacing`.
    pub(super) fn normalize_assignment_spacing(
        content: &str,
        spacing: SpaceAroundColumn
    ) -> String {
        if matches!(spacing, SpaceAroundColumn::Untouched) {
            return content.to_string();
        }
        let bytes = content.as_bytes();
        let is_op_prefix = |b: u8| {
            matches!(
                b,
                b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'<' | b'>'
            )
        };
        let Some(eq_pos) = bytes.iter().position(|&b| b == b'=')
        else {
            return content.to_string();
        };
        // Find where the operator starts (scan back over prefix chars only, no whitespace).
        let mut op_start = eq_pos;
        while op_start > 0 && is_op_prefix(bytes[op_start - 1]) {
            op_start -= 1;
        }
        let label = content[..op_start].trim_end();
        let op = &content[op_start..=eq_pos];
        let value = content[eq_pos + 1..].trim_start();
        let (sp_before, sp_after) = match spacing {
            SpaceAroundColumn::None => ("", ""),
            SpaceAroundColumn::Before => (" ", ""),
            SpaceAroundColumn::After => ("", " "),
            SpaceAroundColumn::Both => (" ", " "),
            SpaceAroundColumn::Untouched => unreachable!()
        };
        format!("{}{}{}{}{}", label, sp_before, op, sp_after, value)
    }

}

/// Renumbering of Locomotive BASIC programs.
///
/// Line numbers are reassigned to `start, start+step, start+2*step, …` and
/// all GOTO / GOSUB / RESTORE / RUN / THEN / ELSE / ON ERROR GOTO targets are
/// updated to match the new numbering.
use std::collections::HashMap;

use crate::located::{LocatedBasicProgram, LocatedBasicToken, LocatedTokenKind};
use crate::tokens::BasicTokenNoPrefix as K;
use crate::BasicError;

// ─── Public types ─────────────────────────────────────────────────────────────

/// A single text replacement produced by the renumbering: (line, col, len, new_text).
/// `line` and `col` are 0-based; `len` is in bytes (BASIC source is ASCII).
pub type Substitution = (u32, u32, u32, String);

// ─── Trait ────────────────────────────────────────────────────────────────────

/// Anything that can produce a renumbering substitution list.
pub trait Renumber {
    fn renum_substitutions(&self, start: u16, step: u16) -> Vec<Substitution>;
}

// ─── impl for LocatedBasicProgram ─────────────────────────────────────────────

impl Renumber for LocatedBasicProgram {
    fn renum_substitutions(&self, start: u16, step: u16) -> Vec<Substitution> {
        if self.lines.is_empty() {
            return Vec::new();
        }

        let mapping: HashMap<u16, u16> = self.lines.iter()
            .enumerate()
            .map(|(i, l)| (l.line_number, start.saturating_add((i as u16).saturating_mul(step))))
            .collect();

        let mut subs: Vec<Substitution> = Vec::new();

        for bline in &self.lines {
            // `after_jump` tracks whether the next Number token is a line reference.
            let mut after_jump = false;

            for tok in &bline.tokens {
                match &tok.kind {
                    LocatedTokenKind::LineNumber(old) => {
                        let new_n = mapping.get(old).copied().unwrap_or(*old);
                        if new_n != *old {
                            subs.push(tok_sub(tok, new_n.to_string()));
                        }
                        after_jump = false;
                    }

                    LocatedTokenKind::Keyword(kw) => {
                        after_jump = matches!(kw,
                            K::Goto | K::Gosub | K::Restore | K::Run
                                | K::Then | K::Else | K::OnErrorGoto
                        );
                    }

                    // Number in jump context → line reference.
                    LocatedTokenKind::Number(n) if after_jump => {
                        if let Ok(old) = n.parse::<u16>() {
                            if let Some(&new_n) = mapping.get(&old) {
                                if new_n != old {
                                    subs.push(tok_sub(tok, new_n.to_string()));
                                }
                            }
                        }
                        // Keep `after_jump`: comma may follow for ON GOTO n,n,n lists.
                    }

                    // Comma in a line-number list — keep state.
                    LocatedTokenKind::Other(',') => {}

                    // Whitespace — keep state.
                    LocatedTokenKind::Space => {}

                    // ':' resets the state machine.
                    LocatedTokenKind::Separator => {
                        after_jump = false;
                    }

                    // Anything else (operator, variable, string literal, …) resets.
                    _ => {
                        after_jump = false;
                    }
                }
            }
        }

        subs
    }
}

// ─── Convenience free functions ───────────────────────────────────────────────

/// Parse `text`, renumber, and return the modified source.
/// `start` defaults to 10, `step` defaults to 10.
pub fn renum_text(text: &str, start: u16, step: u16) -> Result<String, BasicError> {
    let prog = LocatedBasicProgram::parse(text)?;
    let subs = prog.renum_substitutions(start, step);
    Ok(apply_substitutions(text, &subs))
}

/// Apply a set of substitutions (from [`Renumber::renum_substitutions`]) to
/// `text` and return the modified source.  Substitutions are applied from the
/// end of the text towards the beginning so that byte offsets remain valid.
pub fn apply_substitutions(text: &str, subs: &[Substitution]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();

    // Sort in REVERSE document order so that column offsets within a line remain
    // valid as we mutate earlier ranges.
    let mut sorted: Vec<&Substitution> = subs.iter().collect();
    sorted.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));

    for &&(line_idx, col, len, ref new_text) in &sorted {
        if let Some(line) = lines.get_mut(line_idx as usize) {
            let start = col as usize;
            let end = (col + len) as usize;
            if end <= line.len() {
                line.replace_range(start..end, new_text);
            }
        }
    }

    // Preserve the original line ending style (LF vs CRLF).
    let crlf = text.contains("\r\n");
    let sep = if crlf { "\r\n" } else { "\n" };
    let mut result = lines.join(sep);
    if text.ends_with('\n') {
        result.push_str(sep);
    }
    result
}

// ─── Private helpers ──────────────────────────────────────────────────────────

fn tok_sub(tok: &LocatedBasicToken, new_text: String) -> Substitution {
    (tok.span.line, tok.span.col, tok.span.len, new_text)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn renum(text: &str, start: u16, step: u16) -> String {
        renum_text(text, start, step).expect("renum_text should not fail on valid BASIC")
    }

    #[test]
    fn test_no_change_when_already_correct() {
        let src = "10 PRINT \"hello\"\n20 END\n";
        assert_eq!(renum(src, 10, 10), src);
    }

    #[test]
    fn test_renumber_gaps() {
        let src = "5 PRINT \"hello\"\n15 END";
        assert_eq!(renum(src, 10, 10), "10 PRINT \"hello\"\n20 END");
    }

    #[test]
    fn test_renumber_custom_start_step() {
        let src = "10 PRINT \"hi\"\n20 END";
        assert_eq!(renum(src, 100, 100), "100 PRINT \"hi\"\n200 END");
    }

    #[test]
    fn test_goto_updated() {
        let src = "10 GOTO 30\n20 PRINT \"hi\"\n30 END";
        assert_eq!(renum(src, 100, 100), "100 GOTO 300\n200 PRINT \"hi\"\n300 END");
    }

    #[test]
    fn test_gosub_updated() {
        let src = "10 GOSUB 30\n20 END\n30 PRINT \"sub\"\n40 RETURN";
        let expected = "100 GOSUB 300\n200 END\n300 PRINT \"sub\"\n400 RETURN";
        assert_eq!(renum(src, 100, 100), expected);
    }

    #[test]
    fn test_then_shorthand() {
        // IF X THEN line_number  (no GOTO keyword)
        let src = "10 IF X=1 THEN 30\n20 PRINT \"no\"\n30 PRINT \"yes\"";
        assert_eq!(renum(src, 10, 10), "10 IF X=1 THEN 30\n20 PRINT \"no\"\n30 PRINT \"yes\"");
        // All already 10/20/30 → no change expected
        let src2 = "5 IF X=1 THEN 15\n15 PRINT \"yes\"";
        assert_eq!(renum(src2, 10, 10), "10 IF X=1 THEN 20\n20 PRINT \"yes\"");
    }

    #[test]
    fn test_on_goto_list() {
        let src = "10 ON X GOTO 10,20,30\n20 PRINT \"b\"\n30 PRINT \"c\"";
        // Already 10/20/30 → no change
        assert_eq!(renum(src, 10, 10), src);

        let src2 = "1 ON X GOTO 1,2,3\n2 PRINT \"b\"\n3 PRINT \"c\"";
        assert_eq!(renum(src2, 10, 10), "10 ON X GOTO 10,20,30\n20 PRINT \"b\"\n30 PRINT \"c\"");
    }

    #[test]
    fn test_restore_updated() {
        let src = "10 RESTORE 30\n20 PRINT \"x\"\n30 DATA 1,2,3";
        let expected = "100 RESTORE 300\n200 PRINT \"x\"\n300 DATA 1,2,3";
        assert_eq!(renum(src, 100, 100), expected);
    }

    #[test]
    fn test_number_in_print_not_changed() {
        // The 30 in PRINT is NOT a line reference and must not change.
        let src = "10 PRINT 30\n20 END\n30 REM unused";
        let expected = "100 PRINT 30\n200 END\n300 REM unused";
        assert_eq!(renum(src, 100, 100), expected);
    }

    #[test]
    fn test_string_content_preserved() {
        let src = "10 PRINT \"GOTO 30\"\n20 END";
        let expected = "100 PRINT \"GOTO 30\"\n200 END";
        // The GOTO inside the string must NOT be treated as a jump keyword.
        assert_eq!(renum(src, 100, 100), expected);
    }

    #[test]
    fn test_crlf_preserved() {
        let src = "10 PRINT \"hi\"\r\n20 END\r\n";
        let result = renum(src, 100, 100);
        assert!(result.contains("\r\n"));
        assert_eq!(result, "100 PRINT \"hi\"\r\n200 END\r\n");
    }
}

//! On-type (`textDocument/onTypeFormatting`) formatting for Locomotive
//! BASIC: when the user presses Enter in a line-numbered program, start the
//! new line with the next line number. For whole-document formatting
//! (`textDocument/formatting`), see `format.rs`.

use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use crate::common::document::Document;

impl BasicAnalyzer {
    /// Called after a newline has been inserted; `position` is the cursor at
    /// the start of the freshly created line. Returns an edit inserting the
    /// next line number when the program uses line numbering.
    pub fn on_type_newline(
        &self,
        document: &Document,
        position: Position
    ) -> Option<Vec<TextEdit>> {
        let new_line_idx = position.line as usize;
        if new_line_idx == 0 {
            return None;
        }

        // The new line must not already carry a number.
        let current = document.line(new_line_idx).unwrap_or_default();
        if current
            .trim_start()
            .starts_with(|c: char| c.is_ascii_digit())
        {
            return None;
        }

        // Previous line must be numbered (otherwise numbering is not in use here).
        let prev = document.line(new_line_idx - 1)?;
        let prev_num = leading_number(&prev)?;

        // Infer the step from the two previous numbered lines; default to 10.
        let step = if new_line_idx >= 2 {
            document
                .line(new_line_idx - 2)
                .and_then(|l| leading_number(&l))
                .and_then(|before| prev_num.checked_sub(before))
                .filter(|s| *s > 0)
                .unwrap_or(10)
        }
        else {
            10
        };

        let mut candidate = prev_num.saturating_add(step);

        // Stay below the next numbered line, if any.
        let total_lines = document.rope.len_lines();
        let next_num = (new_line_idx + 1..total_lines)
            .filter_map(|i| document.line(i))
            .find_map(|l| leading_number(&l));
        if let Some(next_num) = next_num {
            if candidate >= next_num {
                candidate = prev_num + (next_num - prev_num) / 2;
                if candidate <= prev_num {
                    return None; // no room between the two lines
                }
            }
        }

        Some(vec![TextEdit {
            range: Range {
                start: Position {
                    line: position.line,
                    character: 0
                },
                end: Position {
                    line: position.line,
                    character: 0
                }
            },
            new_text: format!("{candidate} ")
        }])
    }
}

/// Parse the line number at the very start of a BASIC line.
fn leading_number(line: &str) -> Option<u32> {
    let trimmed = line.trim_start();
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit_for(text: &str, line: u32) -> Option<String> {
        let uri = Url::parse("file:///t.bas").unwrap();
        let doc = Document::new(uri, text.to_string(), 1);
        BasicAnalyzer::new()
            .on_type_newline(&doc, Position { line, character: 0 })
            .map(|edits| edits[0].new_text.clone())
    }

    #[test]
    fn continues_numbering_with_default_step() {
        // cursor on the empty new line 1 after "10 PRINT"
        assert_eq!(edit_for("10 PRINT \"A\"\n", 1), Some("20 ".to_string()));
    }

    #[test]
    fn infers_step_from_previous_lines() {
        assert_eq!(edit_for("5 CLS\n10 PRINT\n", 2), Some("15 ".to_string()));
        assert_eq!(
            edit_for("100 CLS\n200 PRINT\n", 2),
            Some("300 ".to_string())
        );
    }

    #[test]
    fn squeezes_between_existing_lines() {
        // Inserting between 10 and 20: candidate 10+10=20 collides -> midpoint 15
        assert_eq!(edit_for("10 CLS\n\n20 PRINT\n", 1), Some("15 ".to_string()));
    }

    #[test]
    fn gives_up_when_no_room() {
        assert_eq!(edit_for("10 CLS\n\n11 PRINT\n", 1), None);
    }

    #[test]
    fn no_insertion_without_numbering() {
        assert_eq!(edit_for("PRINT \"A\"\n", 1), None);
    }

    #[test]
    fn no_insertion_when_line_already_numbered() {
        assert_eq!(edit_for("10 CLS\n30 PRINT\n", 1), None);
    }
}

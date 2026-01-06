//! Error handling for CSL parser with rich diagnostic messages

use std::fmt;
use std::ops::Range;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::Buffer;
use codespan_reporting::term::{Chars, Config, DisplayStyle};

/// CSL parsing error with source location information
#[derive(Debug, Clone)]
pub struct CslError {
    /// The source code being parsed
    pub source: String,
    /// Optional filename
    pub filename: Option<String>,
    /// Byte range in the source where the error occurred
    pub span: Range<usize>,
    /// Error message
    pub message: String,
    /// Optional context/label for the error location
    pub label: Option<String>,
    /// Additional notes to help fix the error
    pub notes: Vec<String>
}

impl CslError {
    /// Create a new CSL error
    pub fn new(source: String, span: Range<usize>, message: String) -> Self {
        Self {
            source,
            filename: None,
            span,
            message,
            label: None,
            notes: Vec::new()
        }
    }

    /// Set the filename
    pub fn with_filename(mut self, filename: String) -> Self {
        self.filename = Some(filename);
        self
    }

    /// Set the label for the error location
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /// Add a note to help fix the error
    pub fn with_note(mut self, note: String) -> Self {
        self.notes.push(note);
        self
    }

    /// Add multiple notes
    pub fn with_notes(mut self, notes: Vec<String>) -> Self {
        self.notes.extend(notes);
        self
    }

    /// Get line and column information for the error
    pub fn line_col(&self) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in self.source.chars().enumerate() {
            if i >= self.span.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            }
            else {
                col += 1;
            }
        }

        (line, col)
    }

    /// Format the error with codespan_reporting
    pub fn format_error(&self) -> String {
        let mut files = SimpleFiles::new();
        let filename = self.filename.as_deref().unwrap_or("<input>");
        let file_id = files.add(filename, &self.source);

        let mut labels = vec![Label::primary(file_id, self.span.clone())];

        if let Some(label) = &self.label {
            labels[0] = labels[0].clone().with_message(label);
        }

        let mut diagnostic = Diagnostic::error()
            .with_message(&self.message)
            .with_labels(labels);

        if !self.notes.is_empty() {
            diagnostic = diagnostic.with_notes(self.notes.clone());
        }

        let mut buffer = Buffer::no_color();
        let config = Config {
            display_style: DisplayStyle::Rich,
            tab_width: 4,
            chars: Chars::ascii(),
            start_context_lines: 2,
            end_context_lines: 1,
            before_label_lines: 1,
            after_label_lines: 1
        };

        term::emit_to_write_style(&mut buffer, &config, &files, &diagnostic).unwrap();
        String::from_utf8(buffer.into_inner()).unwrap()
    }
}

impl fmt::Display for CslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_error())
    }
}

impl std::error::Error for CslError {}

/// Suggest similar instruction names for typos
pub fn suggest_instruction(input: &str) -> Option<String> {
    const INSTRUCTIONS: &[&str] = &[
        "csl_version",
        "reset",
        "crtc_select",
        "gate_array",
        "cpc_model",
        "memory_exp",
        "rom_dir",
        "rom_config",
        "disk_insert",
        "disk_dir",
        "tape_insert",
        "tape_dir",
        "tape_play",
        "tape_stop",
        "tape_rewind",
        "snapshot_load",
        "snapshot_dir",
        "key_delay",
        "key_output",
        "key_from_file",
        "wait",
        "wait_driveonoff",
        "wait_vsyncoffon",
        "wait_ssm0000",
        "screenshot_name",
        "screenshot_dir",
        "screenshot",
        "snapshot_name",
        "snapshot",
        "snapshot_version",
        "csl_load"
    ];

    let input_lower = input.to_lowercase();

    // Find closest match using simple Levenshtein-like logic
    let mut best_match: Option<(&str, usize)> = None;

    for &instruction in INSTRUCTIONS {
        let distance = levenshtein_distance(&input_lower, instruction);
        if distance <= 2 {
            match best_match {
                None => best_match = Some((instruction, distance)),
                Some((_, best_dist)) if distance < best_dist => {
                    best_match = Some((instruction, distance));
                },
                _ => {}
            }
        }
    }

    best_match.map(|(name, _)| name.to_string())
}

/// Simple Levenshtein distance calculation
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            }
            else {
                1
            };
            matrix[i][j] = *[
                matrix[i - 1][j] + 1,        // deletion
                matrix[i][j - 1] + 1,        // insertion
                matrix[i - 1][j - 1] + cost  // substitution
            ]
            .iter()
            .min()
            .unwrap();
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatting() {
        let source = "csl_version 1.0\nrset H\nwait 100".to_string();
        let error = CslError::new(
            source,
            16..20, // "rset"
            "Unknown instruction".to_string()
        )
        .with_label("invalid instruction name".to_string())
        .with_note("Did you mean 'reset'?".to_string());

        let formatted = error.format_error();
        assert!(formatted.contains("error: Unknown instruction"));
        assert!(formatted.contains("rset"));
    }

    #[test]
    fn test_suggest_instruction() {
        assert_eq!(suggest_instruction("rset"), Some("reset".to_string()));
        assert_eq!(
            suggest_instruction("disk_inser"),
            Some("disk_insert".to_string())
        );
        assert_eq!(
            suggest_instruction("snapshoot"),
            Some("snapshot".to_string())
        );
        assert_eq!(suggest_instruction("completely_wrong_name"), None);
    }

    #[test]
    fn test_line_col() {
        let source = "line1\nline2\nline3".to_string();
        let error = CslError::new(source, 6..11, "test".to_string());
        assert_eq!(error.line_col(), (2, 1));
    }
}

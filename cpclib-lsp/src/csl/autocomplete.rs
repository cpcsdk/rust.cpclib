//! Completion for CSL files: instruction-keyword completion at the start
//! of a line, and argument-value completion once a recognized instruction
//! keyword has been typed - both driven directly by
//! `cpclib_csl::lsp::INSTRUCTION_SPECS`, the parser's own authoritative
//! description of what each keyword accepts, so suggestions can never
//! drift from what actually parses.

use cpclib_csl::lsp::{self, ArgShape, INSTRUCTION_SPECS};
use tower_lsp::lsp_types::*;

use super::CslAnalyzer;
use crate::common::document::Document;

/// Splits `line` into whitespace-delimited tokens, treating a `'...'` span
/// as a single token (so a quoted path containing spaces isn't split) -
/// returns each token's `(start, end)` byte offset within `line`, in
/// order. Token 0 is the instruction keyword itself.
fn tokenize(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        if bytes[i] == b'\'' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'\'' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // consume the closing quote
            }
        }
        else {
            while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\t' {
                i += 1;
            }
        }
        tokens.push((start, i));
    }
    tokens
}

/// Which token `byte_cursor` is at (still being typed) or about to start -
/// 0 means the instruction keyword itself; N means the Nth argument
/// (1-based). Cursor in the whitespace gap right before a token counts as
/// "about to type that token", same as being inside it.
fn token_index_at_cursor(tokens: &[(usize, usize)], byte_cursor: usize) -> usize {
    for (i, &(_, end)) in tokens.iter().enumerate() {
        if byte_cursor <= end {
            return i;
        }
    }
    tokens.len()
}

fn keyword_item(spec: &lsp::InstructionSpec) -> CompletionItem {
    CompletionItem {
        label: spec.name.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(describe_shape(spec.args).to_string()),
        insert_text: Some(spec.name.to_string()),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    }
}

fn value_item(label: impl Into<String>, detail: Option<&str>) -> CompletionItem {
    let label = label.into();
    CompletionItem {
        label: label.clone(),
        kind: Some(CompletionItemKind::VALUE),
        detail: detail.map(str::to_string),
        insert_text: Some(label),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    }
}

/// One-line human description of what a keyword's arguments look like -
/// shown as the keyword completion item's `detail`.
fn describe_shape(shape: ArgShape) -> &'static str {
    match shape {
        ArgShape::None => "(no arguments)",
        ArgShape::Version => "<major>.<minor>",
        ArgShape::ResetType => "[soft|hard]",
        ArgShape::CrtcModel => "<0|1|1A|1B|2|3|4>",
        ArgShape::GateArrayModel => "<40007|40008|40010>",
        ArgShape::CpcModel => "<0=464|1=664|2=6128|4=6128+|5=464+|6=GX4000>",
        ArgShape::MemoryExpansion => "<0-4>",
        ArgShape::RomConfig => "<U|L|C|M> <0-255> '<path>'",
        ArgShape::QuotedPath => "'<path>'",
        ArgShape::DiskInsert => "[A|B] '<path>'",
        ArgShape::KeyDelay => "<press_delay> [delay_after_key] [delay_after_cr]",
        ArgShape::KeyboardOutput => "'<text, \\(KEY) escapes allowed>'",
        ArgShape::KeyboardWrite => "<10 comma-separated byte values>",
        ArgShape::Duration => "<microseconds>",
        ArgShape::OptionalCount => "[count]",
        ArgShape::OptionalVsyncFlag => "[vsync]",
        ArgShape::SnapshotVersion => "<1|2|3>"
    }
}

/// Value completions for `shape`'s `arg_index`-th argument (1-based - the
/// first argument after the keyword is 1). Empty where the parser accepts
/// a free-form value (a number, an arbitrary path) rather than one of a
/// fixed set of tokens - there is nothing useful to suggest there.
fn argument_completions(shape: ArgShape, arg_index: usize) -> Vec<CompletionItem> {
    match (shape, arg_index) {
        (ArgShape::ResetType, 1) => {
            vec![
                value_item("soft", Some("Memory cleared by ROM, only 64K central RAM")),
                value_item("hard", Some("Full hardware reset")),
            ]
        },
        (ArgShape::CrtcModel, 1) => lsp::crtc_model_tokens().map(|t| value_item(t, None)).collect(),
        (ArgShape::GateArrayModel, 1) => {
            lsp::gate_array_model_tokens()
                .map(|t| value_item(t, None))
                .collect()
        },
        (ArgShape::CpcModel, 1) => {
            vec![
                value_item("0", Some("CPC 464")),
                value_item("1", Some("CPC 664")),
                value_item("2", Some("CPC 6128")),
                value_item("4", Some("CPC 6128 Plus")),
                value_item("5", Some("CPC 464 Plus")),
                value_item("6", Some("GX4000")),
            ]
        },
        (ArgShape::MemoryExpansion, 1) => {
            lsp::memory_expansion_tokens()
                .map(|t| value_item(t, None))
                .collect()
        },
        (ArgShape::RomConfig, 1) => {
            vec![
                value_item("U", Some("Upper ROM")),
                value_item("L", Some("Lower ROM")),
                value_item("C", Some("Cartridge (set of ROMs)")),
                value_item("M", Some("Multiface 2")),
            ]
        },
        (ArgShape::DiskInsert, 1) => vec![value_item("A", None), value_item("B", None)],
        (ArgShape::OptionalVsyncFlag, 1) => vec![value_item("vsync", None)],
        (ArgShape::SnapshotVersion, 1) => {
            vec![value_item("1", None), value_item("2", None), value_item("3", None)]
        },
        _ => Vec::new()
    }
}

impl CslAnalyzer {
    pub fn completion(&self, document: &Document, position: Position) -> Vec<CompletionItem> {
        let Some(line) = document.line(position.line as usize)
        else {
            return Vec::new();
        };
        let byte_cursor = document.byte_column(position);
        let tokens = tokenize(&line);
        let index = token_index_at_cursor(&tokens, byte_cursor);

        if index == 0 {
            let current_version = self
                .parse_document(document)
                .ok()
                .and_then(|script| script.get_version())
                .unwrap_or_else(cpclib_csl::CslVersion::latest);
            return INSTRUCTION_SPECS
                .iter()
                .filter(|spec| spec.min_version <= current_version)
                .map(keyword_item)
                .collect();
        }

        let Some(&(kw_start, kw_end)) = tokens.first()
        else {
            return Vec::new();
        };
        let keyword = &line[kw_start..kw_end];
        let Some(spec) = INSTRUCTION_SPECS
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(keyword))
        else {
            return Vec::new();
        };

        argument_completions(spec.args, index)
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(text: &str) -> Document {
        Document::new_with_language(
            Url::parse("file:///t.csl").unwrap(),
            text.to_string(),
            1,
            Some("csl")
        )
    }

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn empty_line_offers_every_keyword_when_no_version_is_declared_yet() {
        // No `csl_version` line means the parser's own validator
        // (`CslScriptBuilder::current_version`) assumes the *latest*
        // version, not 1.0 - completion mirrors that exact default rather
        // than picking its own, different one.
        let analyzer = CslAnalyzer::new();
        let items = analyzer.completion(&doc(""), pos(0, 0));
        let names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(names.contains(&"reset"));
        assert!(names.contains(&"disk_insert"));
        assert!(names.contains(&"memory_exp"));
        assert!(names.contains(&"keyboard_write"));
    }

    #[test]
    fn declaring_an_old_version_gates_out_newer_keywords() {
        // Two separate analyzers (each with its own parse cache) - the
        // documents below share a URI+version (an artifact of the `doc()`
        // test helper), which would otherwise make the second call read
        // back the first call's cached parse instead of really re-parsing
        // the new text.
        let document = doc("csl_version 1.0\n");
        let items = CslAnalyzer::new().completion(&document, pos(1, 0));
        let names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            !names.contains(&"memory_exp"),
            "memory_exp needs 1.1, script declared 1.0: {names:?}"
        );
        assert!(
            !names.contains(&"keyboard_write"),
            "keyboard_write needs 1.2, script declared 1.0: {names:?}"
        );

        let document = doc("csl_version 1.1\n");
        let items = CslAnalyzer::new().completion(&document, pos(1, 0));
        let names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            names.contains(&"memory_exp"),
            "memory_exp should be offered once csl_version 1.1 is declared: {names:?}"
        );
        assert!(
            !names.contains(&"keyboard_write"),
            "keyboard_write needs 1.2, not just 1.1: {names:?}"
        );
    }

    #[test]
    fn typing_a_keyword_still_offers_keyword_completions() {
        let analyzer = CslAnalyzer::new();
        // Cursor still inside "cr" - not yet a recognized instruction name.
        let items = analyzer.completion(&doc("cr"), pos(0, 2));
        let names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(names.contains(&"crtc_select"));
    }

    #[test]
    fn crtc_select_offers_its_model_tokens_as_the_first_argument() {
        let analyzer = CslAnalyzer::new();
        let items = analyzer.completion(&doc("crtc_select "), pos(0, 12));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["0", "1", "1A", "1B", "2", "3", "4"]);
    }

    #[test]
    fn reset_offers_soft_and_hard() {
        let analyzer = CslAnalyzer::new();
        let items = analyzer.completion(&doc("reset "), pos(0, 6));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["soft", "hard"]);
    }

    #[test]
    fn disk_insert_offers_drive_letters_at_the_first_argument() {
        let analyzer = CslAnalyzer::new();
        let items = analyzer.completion(&doc("disk_insert "), pos(0, 12));
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["A", "B"]);
    }

    #[test]
    fn a_quoted_path_argument_is_not_split_on_its_internal_spaces() {
        let analyzer = CslAnalyzer::new();
        // Cursor right after the drive letter, inside the second token
        // (still argument 1, the drive) - the quoted path with an
        // embedded space must not have been treated as two tokens.
        let tokens = tokenize("disk_insert A 'my game.dsk'");
        assert_eq!(tokens.len(), 3, "{tokens:?}");
        let _ = analyzer; // keep the analyzer construction covered too
    }

    #[test]
    fn no_argument_completions_for_a_free_form_number() {
        let analyzer = CslAnalyzer::new();
        let items = analyzer.completion(&doc("wait "), pos(0, 5));
        assert!(items.is_empty(), "{items:?}");
    }

    #[test]
    fn an_unrecognized_keyword_offers_no_argument_completions() {
        let analyzer = CslAnalyzer::new();
        let items = analyzer.completion(&doc("bogus_instruction "), pos(0, 18));
        assert!(items.is_empty(), "{items:?}");
    }
}

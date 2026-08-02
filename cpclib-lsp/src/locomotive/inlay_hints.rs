//! Inlay hints for Locomotive BASIC: `CHR$(n)` gets a hint right after the
//! call showing what it actually prints - the literal character for
//! printable codes, the firmware's own name for control codes, and (for the
//! CPC's redefinable/graphics character set, 128-255, which has no reliable
//! textual name) a placeholder whose tooltip renders the real ROM glyph.

use tower_lsp::lsp_types::*;

use super::BasicAnalyzer;
use super::token::parse_basic_integer;
use crate::common::document::{Document, byte_offset_to_utf16_col};

impl BasicAnalyzer {
    pub fn inlay_hints(&self, document: &Document, range: Range) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        for line_idx in range.start.line..=range.end.line {
            let Some(line) = document.line(line_idx as usize)
            else {
                continue;
            };
            let line = line.trim_end_matches(['\n', '\r']);
            let upper = line.to_uppercase();

            let mut search_from = 0;
            while let Some(rel) = upper[search_from..].find("CHR$(") {
                let paren_open = search_from + rel + 4; // index of '('
                let Some(rel_close) = line.get(paren_open + 1..).and_then(|s| s.find(')'))
                else {
                    break;
                };
                let close = paren_open + 1 + rel_close;
                let arg_text = line[paren_open + 1..close].trim();

                if let Some(value) = parse_basic_integer(arg_text)
                    && (0..=255).contains(&value)
                {
                    let (label, tooltip) = char_hint_label(value as u8);
                    let end_char = byte_offset_to_utf16_col(line, close + 1) as u32;
                    hints.push(InlayHint {
                        position: Position {
                            line: line_idx,
                            character: end_char
                        },
                        label: InlayHintLabel::String(label),
                        kind: None,
                        text_edits: None,
                        tooltip,
                        padding_left: Some(true),
                        padding_right: None,
                        data: None
                    });
                }
                search_from = close + 1;
            }
        }
        hints
    }
}

/// The inline text (and, for the 128-255 range, a bitmap tooltip) shown
/// after a `CHR$(code)` call:
/// - 0-31 and 127 (ASCII control codes / DEL): there's nothing to print, so
///   showing the character itself would just be confusing (or invisible) -
///   the firmware's own name for it is the useful hint instead.
/// - 32-126 (printable ASCII): the literal character, unquoted - quoting it
///   read as if the quotes were part of the printed output.
/// - 128-255 (the CPC's redefinable/graphics character set): no reliable
///   textual name exists for these (they can even be redefined by a `SYMBOL`
///   statement) - shown as the bare code, with the real ROM glyph rendered
///   as an image in the hint's own tooltip (reusing the same rasterizer as
///   `CHR$`'s hover, `hover::glyph_hover_markdown`).
fn char_hint_label(code: u8) -> (String, Option<InlayHintTooltip>) {
    if let Some(name) = control_char_name(code) {
        (format!(" {name}"), None)
    }
    else if (32..=126).contains(&code) {
        (format!(" {}", code as char), None)
    }
    else {
        let tooltip = InlayHintTooltip::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: super::hover::glyph_hover_markdown(code)
        });
        (format!(" #{code}"), Some(tooltip))
    }
}

/// The Amstrad CPC firmware's own name for an ASCII control code (0-31) or
/// `DEL` (127) - mirrors `cpclib_catart::basic_chars`'s constants (using the
/// BASIC-command-flavored alias where one exists, e.g. `PAPER`/`PEN`/`CLS`,
/// and the plain control mnemonic otherwise, e.g. `BEL`/`LF`/`CR`). `None`
/// for anything outside that range - those are handled elsewhere.
fn control_char_name(code: u8) -> Option<&'static str> {
    use cpclib_catart::basic_chars::*;
    Some(match code {
        NUL => "NUL",
        SOH => "SOH",
        STX => "CURSOR_0",
        ETX => "CURSOR_1",
        EOT => "MODE",
        ENQ => "ENQ",
        ACK => "ACK",
        BEL => "BEL",
        BS => "BS",
        TAB => "TAB",
        LF => "LF",
        VT => "VT",
        FF => "CLS",
        CR => "CR",
        SO => "PAPER",
        SI => "PEN",
        DLE => "DLE",
        DC1 => "DC1",
        DC2 => "DC2",
        DC3 => "DC3",
        DC4 => "DC4",
        NAK => "NAK",
        SYN => "SYN",
        ETB => "ETB",
        CAN => "CAN",
        EM => "SYMBOL",
        SUB => "WINDOW",
        ESC => "ESC",
        FS => "INK",
        GS => "BORDER",
        RS => "RS",
        US => "LOCATE",
        127 => "DEL",
        _ => return None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::new(Url::parse("file:///t.bas").unwrap(), text.to_string(), 1)
    }

    fn full_range(text: &str) -> Range {
        Range {
            start: Position {
                line: 0,
                character: 0
            },
            end: Position {
                line: text.lines().count() as u32,
                character: 0
            }
        }
    }

    fn label_of(hint: &InlayHint) -> &str {
        match &hint.label {
            InlayHintLabel::String(s) => s.as_str(),
            _ => panic!("expected a string label: {hint:?}")
        }
    }

    #[test]
    fn crlf_line_endings_do_not_break_hint_detection() {
        let text = "70 PRINT CHR$(34);:PRINT \"HBL\";\r\n80 PRINT CHR$(&0d);:PRINT CHR$(&0b);\r\n";
        let d = doc(text);
        let hints = BasicAnalyzer::new().inlay_hints(&d, full_range(text));
        // CHR$(34) is printable; CHR$(&0d)=13 (CR) and CHR$(&0b)=11 (VT) are
        // control codes - all three now get a hint (control codes used to
        // get none at all).
        assert_eq!(hints.len(), 3, "{hints:?}");
        assert_eq!(label_of(&hints[0]), " \"");
        assert_eq!(label_of(&hints[1]), " CR");
        assert_eq!(label_of(&hints[2]), " VT");
    }

    #[test]
    fn chr_call_with_a_printable_code_gets_the_bare_character_no_quotes() {
        let text = "10 PRINT CHR$(65)\n";
        let d = doc(text);
        let hints = BasicAnalyzer::new().inlay_hints(&d, full_range(text));
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert_eq!(label_of(&hints[0]), " A");
        // Right after the closing ')'.
        let close_col = text.find("CHR$(65)").unwrap() + "CHR$(65)".len();
        assert_eq!(hints[0].position.character, close_col as u32);
    }

    #[test]
    fn chr_call_with_a_control_code_shows_its_firmware_name() {
        let text = "10 PRINT CHR$(7)\n"; // BEL
        let d = doc(text);
        let hints = BasicAnalyzer::new().inlay_hints(&d, full_range(text));
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert_eq!(label_of(&hints[0]), " BEL");
        assert!(hints[0].tooltip.is_none());
    }

    #[test]
    fn chr_call_with_a_graphics_character_gets_a_placeholder_and_a_glyph_tooltip() {
        let text = "10 PRINT CHR$(144)\n";
        let d = doc(text);
        let hints = BasicAnalyzer::new().inlay_hints(&d, full_range(text));
        assert_eq!(hints.len(), 1, "{hints:?}");
        assert_eq!(label_of(&hints[0]), " #144");
        match &hints[0].tooltip {
            Some(InlayHintTooltip::MarkupContent(md)) => {
                assert!(md.value.contains("data:image/svg+xml;base64,"), "{md:?}");
            },
            other => panic!("expected a markdown glyph tooltip, got {other:?}")
        }
    }

    #[test]
    fn multiple_chr_calls_on_one_line_each_get_a_hint() {
        let text = "10 PRINT CHR$(72);CHR$(73)\n";
        let d = doc(text);
        let hints = BasicAnalyzer::new().inlay_hints(&d, full_range(text));
        assert_eq!(hints.len(), 2, "{hints:?}");
    }
}

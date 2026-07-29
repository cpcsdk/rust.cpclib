//! Firmware address/label documentation, extracted from `cpclib-asm`'s
//! embedded `inner://firmware/*.asm` assets and looked up by either symbol
//! name (`TXT_OUTPUT`) or resolved numeric address (`&BB5A`) - shared by
//! both `basm::hover` (asm files) and `locomotive::hover` (BASIC/CatArt
//! files, including BASIC blocks embedded inside `.asm` files), since a
//! `CALL &BB18`-style firmware call is just as common in BASIC as in asm.

use std::collections::HashMap;
use std::sync::LazyLock;

use cpclib_basmdoc::{
    DocMarkers, DocumentedItem, UndocumentedConfig, aggregate_documentation_on_tokens,
    build_documentation_page_from_aggregates
};

use crate::basm::includes::{inner_file_names, read_inner_file};
use crate::common::render::parse_numeric_literal_str;

pub struct FirmwareDoc {
    pub symbol: String,
    pub value: String,
    pub doc: String,
    pub source_file: String
}

struct FirmwareDocs {
    by_symbol: HashMap<String, FirmwareDoc>,
    by_value: HashMap<i64, FirmwareDoc>
}

/// Converts the extracted comment's own line breaks (plain `\n`, joined by
/// `cpclib_basmdoc::aggregate_documentation_on_tokens` from consecutive
/// source comment lines) into real Markdown hard breaks (two trailing
/// spaces before the newline) - a bare `\n` collapses to a space in
/// rendered Markdown, which would silently run e.g. "Action:"/"Entry:"/
/// "Exit:"/"Notes:" together into one line and lose the layout the
/// firmware doc's own author (or generator script) chose.
fn markdown_hard_breaks(text: &str) -> String {
    text.replace('\n', "  \n")
}

fn build_firmware_docs() -> FirmwareDocs {
    let mut by_symbol = HashMap::new();
    let mut by_value = HashMap::new();

    for name in inner_file_names().filter(|n| n.starts_with("inner://firmware/")) {
        let Some(content) = read_inner_file(&name)
        else {
            continue;
        };
        let Ok(listing) = cpclib_asm::parser::parse_z80_str(content)
        else {
            continue;
        };
        let agg = aggregate_documentation_on_tokens(
            &listing,
            UndocumentedConfig::none(),
            DocMarkers::single_semicolon()
        );
        let page = build_documentation_page_from_aggregates(&name, agg);

        for item in page.equ_iter() {
            let DocumentedItem::Equ(symbol, value) = item.item()
            else {
                continue;
            };
            let doc = FirmwareDoc {
                symbol: symbol.clone(),
                value: value.clone(),
                doc: markdown_hard_breaks(item.doc()),
                source_file: name.clone()
            };
            if let Some(v) = parse_numeric_literal_str(value) {
                by_value.insert(
                    v,
                    FirmwareDoc {
                        symbol: doc.symbol.clone(),
                        value: doc.value.clone(),
                        doc: doc.doc.clone(),
                        source_file: doc.source_file.clone()
                    }
                );
            }
            by_symbol.insert(symbol.to_uppercase(), doc);
        }
    }

    FirmwareDocs {
        by_symbol,
        by_value
    }
}

static FIRMWARE_DOCS: LazyLock<FirmwareDocs> = LazyLock::new(build_firmware_docs);

/// Look up a firmware routine/constant by its symbolic name (case-insensitive),
/// e.g. `"TXT_OUTPUT"`.
pub fn lookup_by_symbol(symbol: &str) -> Option<&'static FirmwareDoc> {
    FIRMWARE_DOCS.by_symbol.get(&symbol.to_uppercase())
}

/// Look up a firmware routine/constant by its resolved numeric address,
/// e.g. `0xBB5A` (from hovering `&BB5A`/`#BB5A`/`0xBB5A`).
pub fn lookup_by_value(value: i64) -> Option<&'static FirmwareDoc> {
    FIRMWARE_DOCS.by_value.get(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_by_symbol_finds_a_real_firmware_routine() {
        let doc = lookup_by_symbol("TXT_OUTPUT").expect("TXT_OUTPUT should be documented");
        assert!(!doc.doc.is_empty(), "{}", doc.doc);
        assert_eq!(doc.symbol, "TXT_OUTPUT");
    }

    #[test]
    fn lookup_by_symbol_is_case_insensitive() {
        assert!(lookup_by_symbol("txt_output").is_some());
    }

    #[test]
    fn lookup_by_value_finds_the_same_routine() {
        let doc = lookup_by_value(0xBB5A).expect("0xBB5A should resolve to TXT_OUTPUT");
        assert_eq!(doc.symbol, "TXT_OUTPUT");
    }

    #[test]
    fn unrecognized_symbol_or_value_returns_none() {
        assert!(lookup_by_symbol("NOT_A_REAL_FIRMWARE_SYMBOL").is_none());
        assert!(lookup_by_value(0x1234_5678).is_none());
    }

    /// `TXT_OUTPUT`'s real doc is 4 lines (Action/Entry/Exit/Notes) - each
    /// must survive as its own visual line in the rendered Markdown, not
    /// collapse into one run-on paragraph.
    #[test]
    fn multi_line_docs_use_markdown_hard_breaks_not_bare_newlines() {
        let doc = lookup_by_symbol("TXT_OUTPUT").unwrap();
        assert!(doc.doc.contains("  \n"), "{}", doc.doc);
        // Every newline is part of a "  \n" hard break - none left bare.
        assert!(
            !doc.doc.replace("  \n", "").contains('\n'),
            "a bare newline slipped through: {}",
            doc.doc
        );
        assert!(doc.doc.contains("Action:"), "{}", doc.doc);
        assert!(doc.doc.contains("Notes:"), "{}", doc.doc);
    }

    #[test]
    fn markdown_hard_breaks_converts_every_newline() {
        assert_eq!(markdown_hard_breaks("a\nb\nc"), "a  \nb  \nc");
        assert_eq!(markdown_hard_breaks("no newline"), "no newline");
    }
}

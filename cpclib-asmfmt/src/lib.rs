#[cfg(feature = "cmdline")]
pub mod cli;
mod config;
mod formatter;
mod options;

pub use config::{CONFIG_FILE_NAME, find_config_file, load_config, load_config_from};
pub use formatter::{format, format_listing};
pub use options::{
    AsmFormatOptions, BinaryEncoding, CaseStyle, HexEncoding, LabelPostfix, OctalEncoding,
    SpaceAroundColumn
};

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        format(src, &AsmFormatOptions::default()).expect("parse failed")
    }

    #[test]
    fn test_simple_instructions() {
        let out = fmt("push af\n pop bc\n push hl");
        assert!(out.contains("    PUSH AF\n"), "got: {out:?}");
        assert!(out.contains("    POP BC\n"), "got: {out:?}");
    }

    #[test]
    fn test_label_at_col0() {
        let out = fmt("  myloop:\n    push af");
        assert!(out.starts_with("myloop:\n"), "got: {out:?}");
        assert!(out.contains("    PUSH AF"), "got: {out:?}");
    }

    #[test]
    fn test_repeat_block() {
        let out = fmt("repeat 10\n push af\n endrepeat");
        assert!(out.contains("        PUSH AF\n"), "got: {out:?}");
        assert!(out.contains("REPEAT 10"), "got: {out:?}");
        assert!(out.contains("ENDREPEAT"), "got: {out:?}");
    }

    #[test]
    fn test_block_header_directive_case() {
        let out = fmt("repeat 5, i, 3\n  ld a, i\nendr");
        assert!(
            out.contains("REPEAT 5, i, 3"),
            "REPEAT not uppercased: {out:?}"
        );
        assert!(
            out.contains("ENDR") || out.contains("ENDREPEAT"),
            "closer not uppercased: {out:?}"
        );
    }

    #[test]
    fn test_blank_lines_preserved() {
        let out = fmt("push af\n\npop bc");
        assert!(out.contains("\n\n"), "blank line not preserved: {out:?}");
    }

    #[test]
    fn test_comment_preserved() {
        let out = fmt("push af ; save af");
        assert!(out.contains("; save af"), "comment not preserved: {out:?}");
    }

    #[test]
    fn test_comment_column() {
        let out = fmt("push af ; save af");
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment found");
        assert_eq!(col, 30, "comment not at column 30: {line:?}");
    }

    #[test]
    fn test_comment_column_long_content() {
        let long = "ld hl, (some_very_long_symbol_name_that_is_long)";
        let src = format!("{long} ; cmnt");
        let out = fmt(&src);
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment");
        let content_end = 4 + long.len();
        assert!(
            col >= content_end + 2,
            "less than 2 spaces before comment: {line:?}"
        );
    }

    #[test]
    fn test_macro_call_no_panic() {
        let out = fmt("MY_MACRO arg1, arg2\npush af");
        assert!(
            out.contains("MY_MACRO arg1, arg2"),
            "macro call lost: {out:?}"
        );
        assert!(
            out.contains("PUSH AF"),
            "opcode after macro call lost: {out:?}"
        );
    }

    #[test]
    fn test_equ_preserved() {
        let out = fmt("FOO EQU 42\npush af");
        assert!(out.contains("FOO"), "EQU lost: {out:?}");
        assert!(out.contains("42"), "EQU value lost: {out:?}");
    }

    #[test]
    fn test_case_lowercase() {
        let opt = AsmFormatOptions {
            mnemonic_case: CaseStyle::LowerCase,
            directive_case: CaseStyle::LowerCase,
            register_case: CaseStyle::LowerCase,
            ..AsmFormatOptions::default()
        };
        let out = format("PUSH AF\nORG 0x40\n", &opt).unwrap();
        assert!(out.contains("push af"), "mnemonic not lowercased: {out:?}");
        assert!(
            out.contains("org 0x40"),
            "directive not lowercased: {out:?}"
        );
    }

    #[test]
    fn test_case_untouched() {
        let opt = AsmFormatOptions {
            mnemonic_case: CaseStyle::Untouched,
            directive_case: CaseStyle::Untouched,
            register_case: CaseStyle::Untouched,
            ..AsmFormatOptions::default()
        };
        let out = format("Push Af\nOrg 0x40\n", &opt).unwrap();
        assert!(out.contains("Push Af"), "mnemonic case changed: {out:?}");
        assert!(out.contains("Org 0x40"), "directive case changed: {out:?}");
    }

    #[test]
    fn test_register_case_independent() {
        let opt = AsmFormatOptions {
            mnemonic_case: CaseStyle::UpperCase,
            register_case: CaseStyle::LowerCase,
            ..AsmFormatOptions::default()
        };
        let out = format("PUSH AF\nLD HL, BC\n", &opt).unwrap();
        assert!(
            out.contains("PUSH af\n"),
            "register not lowercased: {out:?}"
        );
        assert!(
            out.contains("LD hl, bc\n"),
            "registers not lowercased: {out:?}"
        );
    }

    #[test]
    fn test_literal_not_hex_encoded() {
        let out = fmt("ld a, 1\nld hl, 100\nld de, 0x40\nadd a, %00001111");
        assert!(out.contains("LD A, 1\n"), "literal 1 re-encoded: {out:?}");
        assert!(
            out.contains("LD HL, 100\n"),
            "literal 100 re-encoded: {out:?}"
        );
        assert!(
            out.contains("LD DE, 0x40\n"),
            "literal 0x40 re-encoded: {out:?}"
        );
        assert!(
            out.contains("ADD A, %00001111\n"),
            "literal %… re-encoded: {out:?}"
        );
    }

    #[test]
    fn test_registers_uppercased() {
        let out = fmt("ld hl, (ix+2)\npush af\nex af, af'");
        assert!(
            out.contains("LD HL, (IX+2)\n"),
            "registers not uppercased: {out:?}"
        );
        assert!(out.contains("PUSH AF\n"), "AF not uppercased: {out:?}");
        assert!(out.contains("EX AF, AF'\n"), "AF' not uppercased: {out:?}");
    }

    #[test]
    fn test_trailing_comment_no_duplicate() {
        let src = "    org 0x40  ; comment 1\n    push af  ; comment 2\n    pop af\n";
        let out = fmt(src);
        assert_eq!(
            out.matches("comment 1").count(),
            1,
            "comment 1 duplicated: {out:?}"
        );
        assert_eq!(
            out.matches("comment 2").count(),
            1,
            "comment 2 duplicated: {out:?}"
        );
    }

    #[test]
    fn test_colon_separator_no_duplicate() {
        // Multiple instructions on one line must not be duplicated.
        let src = "    pop hl : push af : pop af\n";
        let out = fmt(src);
        assert_eq!(
            out.matches("POP HL").count(),
            1,
            "POP HL duplicated: {out:?}"
        );
        assert_eq!(
            out.matches("PUSH AF").count(),
            1,
            "PUSH AF duplicated: {out:?}"
        );
        assert_eq!(
            out.matches("POP AF").count(),
            1,
            "POP AF duplicated: {out:?}"
        );
    }

    #[test]
    fn test_one_instruction_per_line_splits() {
        let src = "pop hl : push af : pop af\n";
        let out = fmt(src);
        let lines: Vec<&str> = out.lines().collect();
        // Each instruction on its own line
        assert!(
            lines.iter().any(|l| l.trim() == "POP HL"),
            "POP HL not on own line: {out:?}"
        );
        assert!(
            lines.iter().any(|l| l.trim() == "PUSH AF"),
            "PUSH AF not on own line: {out:?}"
        );
        assert!(
            lines.iter().any(|l| l.trim() == "POP AF"),
            "POP AF not on own line: {out:?}"
        );
    }

    #[test]
    fn test_one_instruction_per_line_false_keeps_line() {
        let opt = AsmFormatOptions {
            one_instruction_per_line: false,
            ..AsmFormatOptions::default()
        };
        let src = "pop hl : push af\n";
        let out = format(src, &opt).unwrap();
        // Both instructions must be on a single line (no splitting).
        // The second mnemonic keyword is not in first-word position so its case is not
        // transformed; only registers like HL/AF are uppercased within the line.
        assert!(
            out.lines()
                .any(|l| l.contains("POP HL") && l.contains("push AF")),
            "line was split when one_instruction_per_line=false: {out:?}"
        );
    }

    #[test]
    fn test_colon_comment_on_last_instruction() {
        // Trailing comment must appear once, on the last instruction.
        let src = "pop hl : push af ; my comment\n";
        let out = fmt(src);
        assert_eq!(
            out.matches("my comment").count(),
            1,
            "comment duplicated: {out:?}"
        );
        // The comment should be on the PUSH AF line, not the POP HL line.
        let push_line = out
            .lines()
            .find(|l| l.contains("PUSH AF"))
            .expect("no PUSH AF line");
        assert!(
            push_line.contains("my comment"),
            "comment not on last instruction: {push_line:?}"
        );
    }

    #[test]
    fn test_user_sample() {
        let src = "\tei\n\txor b\n\n\txor b\n\n\tld de, ix\n\n\torg 40\n    pop hl : push af: pop af\n\tpop AF\n";
        let result = format(src, &AsmFormatOptions::default());
        match result {
            Ok(out) => {
                assert!(out.contains("    EI\n"), "EI missing: {out:?}");
                assert!(out.contains("    ORG 40\n"), "ORG missing: {out:?}");
            },
            Err(e) => panic!("format failed: {e}")
        }
    }

    #[test]
    fn test_assign_symbol_case_not_changed() {
        // Symbol names in assignment directives must not be case-transformed.
        let out = fmt("my_label = 42\nassert my_label == 42");
        assert!(
            out.contains("my_label = 42"),
            "symbol name changed: {out:?}"
        );
        assert!(
            out.contains("ASSERT my_label"),
            "symbol in expr changed: {out:?}"
        );
    }

    #[test]
    fn test_equ_keyword_case_changed() {
        // EQU keyword should be case-transformed, symbol name should not.
        let out = fmt("my_sym equ 10\ndb my_sym");
        assert!(
            out.contains("my_sym EQU 10"),
            "EQU not uppercased or symbol changed: {out:?}"
        );
    }

    #[test]
    fn test_label_with_instruction_on_same_line() {
        // Label followed by instruction on the same line (no colon separator).
        let out = fmt("myloop\tpush af");
        assert!(out.contains("myloop:"), "label missing colon: {out:?}");
        assert!(
            out.contains("PUSH AF"),
            "instruction after inline label lost: {out:?}"
        );
    }

    #[test]
    fn test_equ_at_column_zero() {
        // EQU labels must start at column 0 regardless of any surrounding block depth.
        let out = fmt("FOO EQU 42");
        let line = out.lines().next().unwrap();
        assert!(!line.starts_with(' '), "EQU line is indented: {line:?}");
        assert!(
            line.starts_with("FOO"),
            "EQU label not at column 0: {line:?}"
        );
    }

    #[test]
    fn test_assign_at_column_zero() {
        // Symbol assignments (=) must start at column 0.
        let out = fmt("my_var = 10");
        let line = out.lines().next().unwrap();
        assert!(
            !line.starts_with(' '),
            "assignment line is indented: {line:?}"
        );
        assert!(
            line.starts_with("my_var"),
            "assignment not at column 0: {line:?}"
        );
    }

    #[test]
    fn test_comment_column_custom() {
        // comment_column should be honoured for non-default values.
        let opt = AsmFormatOptions::builder().comment_column(50).build();
        let out = format("nop ; hi", &opt).unwrap();
        let line = out.lines().next().unwrap();
        let col = line.find(';').expect("no comment found");
        assert_eq!(col, 50, "comment not at column 50: {line:?}");
    }

    #[test]
    fn test_space_around_column_both() {
        // SpaceAroundColumn::Both forces ` : ` between instructions.
        let opt = AsmFormatOptions::builder()
            .one_instruction_per_line(false)
            .space_around_column(SpaceAroundColumn::Both)
            .build();
        let out = format("nop : ld a, 5", &opt).unwrap();
        let line = out.lines().next().unwrap();
        assert!(line.contains(" : "), "separator not ' : ': {line:?}");
    }

    #[test]
    fn test_space_around_column_none() {
        // SpaceAroundColumn::None removes all spaces around `:`.
        let opt = AsmFormatOptions::builder()
            .one_instruction_per_line(false)
            .space_around_column(SpaceAroundColumn::None)
            .build();
        let out = format("nop : ld a, 5", &opt).unwrap();
        let line = out.lines().next().unwrap();
        assert!(
            line.contains(':') && !line.contains(" :") && !line.contains(": "),
            "unexpected spacing around ':': {line:?}"
        );
    }

    #[test]
    fn test_space_around_column_untouched_preserves() {
        // SpaceAroundColumn::Untouched (default) must not alter existing spacing.
        let opt = AsmFormatOptions::builder()
            .one_instruction_per_line(false)
            .space_around_column(SpaceAroundColumn::Untouched)
            .build();
        let src = "nop : ld a, 5";
        let out = format(src, &opt).unwrap();
        // The ` : ` from source should be preserved.
        assert!(out.contains(" : "), "spacing was altered: {out:?}");
    }

    // ── space_around_assignment ──────────────────────────────────────────────

    fn fmt_assign(src: &str, spacing: SpaceAroundColumn) -> String {
        let opt = AsmFormatOptions::builder()
            .space_around_assignment(spacing)
            .build();
        format(src, &opt).unwrap()
    }

    #[test]
    fn test_assign_spacing_both() {
        let out = fmt_assign("my_var=5", SpaceAroundColumn::Both);
        assert!(out.contains("my_var = 5"), "Both: {out:?}");
    }

    #[test]
    fn test_assign_spacing_none() {
        let out = fmt_assign("my_var = 5", SpaceAroundColumn::None);
        assert!(out.contains("my_var=5"), "None: {out:?}");
    }

    #[test]
    fn test_assign_spacing_before() {
        let out = fmt_assign("my_var=5", SpaceAroundColumn::Before);
        assert!(out.contains("my_var =5"), "Before: {out:?}");
    }

    #[test]
    fn test_assign_spacing_after() {
        let out = fmt_assign("my_var=5", SpaceAroundColumn::After);
        assert!(out.contains("my_var= 5"), "After: {out:?}");
    }

    #[test]
    fn test_assign_spacing_untouched() {
        // Untouched (default) must preserve original spacing exactly.
        let out = fmt_assign("my_var=5", SpaceAroundColumn::Untouched);
        assert!(out.contains("my_var=5"), "Untouched: {out:?}");
        let out2 = fmt_assign("my_var = 5", SpaceAroundColumn::Untouched);
        assert!(out2.contains("my_var = 5"), "Untouched spaces: {out2:?}");
    }

    #[test]
    fn test_assign_compound_operator_both() {
        let out = fmt_assign("my_var+=10", SpaceAroundColumn::Both);
        assert!(out.contains("my_var += 10"), "compound Both: {out:?}");
    }

    #[test]
    fn test_assign_compound_operator_none() {
        let out = fmt_assign("my_var += 10", SpaceAroundColumn::None);
        assert!(out.contains("my_var+=10"), "compound None: {out:?}");
    }

    #[test]
    fn test_assign_shift_operator_both() {
        let out = fmt_assign("my_var>>=2", SpaceAroundColumn::Both);
        assert!(out.contains("my_var >>= 2"), "shift Both: {out:?}");
    }

    // ── TOML config roundtrip ─────────────────────────────────────────────────

    #[test]
    fn test_toml_config_roundtrip() {
        let toml = r#"
indent_size = 4
comment_column = 13
mnemonic_case = "LowerCase"
directive_case = "UpperCase"
register_case = "LowerCase"
one_instruction_per_line = false
space_around_column = "Both"
space_around_assignment = "Both"
hexadecimal_case = "UpperCase"
hexadecimal_encoding = "0x"
octal_encoding = "0o"
binary_encoding = "0b"
label_definition_postfix_with_column = "NoColumn"
"#;
        let cfg: AsmFormatOptions = toml::from_str(toml).expect("TOML parse failed");
        assert!(
            matches!(cfg.mnemonic_case, CaseStyle::LowerCase),
            "mnemonic_case: {cfg:?}"
        );
        assert!(
            matches!(cfg.register_case, CaseStyle::LowerCase),
            "register_case"
        );
        assert!(
            matches!(cfg.hexadecimal_encoding, HexEncoding::Prefix0x),
            "hex_enc"
        );
        assert!(
            matches!(cfg.octal_encoding, OctalEncoding::Prefix0o),
            "oct_enc"
        );
        assert!(
            matches!(cfg.binary_encoding, BinaryEncoding::Prefix0b),
            "bin_enc"
        );
        assert!(
            matches!(
                cfg.label_definition_postfix_with_column,
                LabelPostfix::NoColumn
            ),
            "label_postfix"
        );
    }

    // ── hexadecimal_case ─────────────────────────────────────────────────────

    #[test]
    fn test_hex_case_upper() {
        let opt = AsmFormatOptions::builder()
            .hexadecimal_case(CaseStyle::UpperCase)
            .build();
        let out = format("ld a, 0xff\nld b, $ab", &opt).unwrap();
        assert!(
            out.contains("0xFF") || out.contains("0XFF"),
            "hex not uppercased: {out:?}"
        );
        assert!(out.contains("$AB"), "dollar hex not uppercased: {out:?}");
    }

    #[test]
    fn test_hex_case_lower() {
        let opt = AsmFormatOptions::builder()
            .hexadecimal_case(CaseStyle::LowerCase)
            .build();
        let out = format("ld a, 0xFF\nld b, $AB", &opt).unwrap();
        assert!(
            out.contains("0xff") || out.contains("ff"),
            "hex not lowercased: {out:?}"
        );
        assert!(out.contains("$ab"), "dollar hex not lowercased: {out:?}");
    }

    // ── hexadecimal_encoding ─────────────────────────────────────────────────

    #[test]
    fn test_hex_encoding_prefix_dollar() {
        let opt = AsmFormatOptions::builder()
            .hexadecimal_encoding(HexEncoding::PrefixDollar)
            .build();
        let out = format("ld a, 0xff", &opt).unwrap();
        assert!(
            out.contains("$FF") || out.contains("$ff"),
            "not $ prefix: {out:?}"
        );
        assert!(
            !out.contains("0xff") && !out.contains("0xFF"),
            "old prefix still present: {out:?}"
        );
    }

    #[test]
    fn test_hex_encoding_suffix_h() {
        let opt = AsmFormatOptions::builder()
            .hexadecimal_encoding(HexEncoding::SuffixLower)
            .build();
        let out = format("ld a, 0x1A", &opt).unwrap();
        assert!(
            out.contains("1ah") || out.contains("1Ah"),
            "not h suffix: {out:?}"
        );
    }

    #[test]
    fn test_hex_encoding_suffix_h_leading_zero() {
        // When the first hex digit is alphabetic, a leading 0 must be added.
        let opt = AsmFormatOptions::builder()
            .hexadecimal_encoding(HexEncoding::SuffixUpper)
            .build();
        let out = format("ld a, 0xFF", &opt).unwrap();
        assert!(
            out.contains("0FFH") || out.contains("0ffH"),
            "leading 0 missing: {out:?}"
        );
    }

    // ── octal_encoding ────────────────────────────────────────────────────────

    #[test]
    fn test_octal_encoding_prefix_at() {
        let opt = AsmFormatOptions::builder()
            .octal_encoding(OctalEncoding::PrefixAt)
            .build();
        let out = format("ld a, 0o17", &opt).unwrap();
        assert!(out.contains("@17"), "not @ prefix: {out:?}");
    }

    #[test]
    fn test_octal_encoding_prefix_0o() {
        let opt = AsmFormatOptions::builder()
            .octal_encoding(OctalEncoding::Prefix0o)
            .build();
        let out = format("ld a, @17", &opt).unwrap();
        assert!(out.contains("0o17"), "not 0o prefix: {out:?}");
    }

    // ── binary_encoding ───────────────────────────────────────────────────────

    #[test]
    fn test_binary_encoding_percent() {
        let opt = AsmFormatOptions::builder()
            .binary_encoding(BinaryEncoding::PrefixPercent)
            .build();
        let out = format("ld a, 0b00001111", &opt).unwrap();
        assert!(
            out.contains("%1111") || out.contains("%00001111"),
            "not % prefix: {out:?}"
        );
    }

    #[test]
    fn test_binary_encoding_0b() {
        let opt = AsmFormatOptions::builder()
            .binary_encoding(BinaryEncoding::Prefix0b)
            .build();
        let out = format("ld a, %00001111", &opt).unwrap();
        assert!(out.contains("0b"), "not 0b prefix: {out:?}");
    }

    // ── label_definition_postfix_with_column ──────────────────────────────────

    #[test]
    fn test_label_postfix_no_column() {
        let opt = AsmFormatOptions::builder()
            .label_definition_postfix_with_column(LabelPostfix::NoColumn)
            .build();
        let out = format("myloop:\n  push af", &opt).unwrap();
        let label_line = out.lines().next().unwrap();
        assert!(
            !label_line.contains(':'),
            "colon present with NoColumn: {out:?}"
        );
        assert!(label_line.trim() == "myloop", "wrong label line: {out:?}");
    }

    #[test]
    fn test_label_postfix_with_column() {
        let opt = AsmFormatOptions::builder()
            .label_definition_postfix_with_column(LabelPostfix::WithColumn)
            .build();
        let out = format("myloop:\n  push af", &opt).unwrap();
        let label_line = out.lines().next().unwrap();
        assert!(
            label_line.contains(':'),
            "colon missing with WithColumn: {out:?}"
        );
    }

    // ── single space after directive ──────────────────────────────────────────

    #[test]
    fn test_single_space_after_directive() {
        let out = fmt("ORG  0x40\nDB   1, 2, 3");
        assert!(
            out.contains("ORG 0x40"),
            "double space after ORG not collapsed: {out:?}"
        );
        assert!(
            out.contains("DB 1, 2, 3"),
            "double space after DB not collapsed: {out:?}"
        );
    }
}

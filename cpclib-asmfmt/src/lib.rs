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

    /// Regression: `is_assign`/`is_equ` used to hardcode column 0
    /// unconditionally, so a local assignment inside a FUNCTION body was
    /// flattened to the left margin - out from under its own block, and out
    /// of step with every sibling statement around it (`RETURN`, ...).
    ///
    /// FUNCTION is the one construct that actually gets this treatment: it
    /// has real local scope in basm (confirmed live: a name assigned inside
    /// one is `Unknown symbol` if read after it), so an assignment there is
    /// an ordinary local statement, not a declaration - see
    /// `test_assign_inside_a_bare_if_or_repeat_stays_at_column_zero` for the
    /// (deliberately different) IF/REPEAT/... case, which never scopes
    /// anything and keeps the column-0 convention regardless of nesting.
    #[test]
    fn test_assign_and_equ_inside_a_function_keep_the_bodys_depth() {
        let out = fmt("function foo x\n    a = 1\n    b equ 2\n    return a\nendfunction\n");
        let lines: Vec<&str> = out.lines().collect();
        let body_indent = |line: &str| line.len() - line.trim_start().len();
        let a_line = lines.iter().find(|l| l.contains("a = 1")).unwrap();
        let b_line = lines.iter().find(|l| l.contains("EQU 2")).unwrap();
        let return_line = lines.iter().find(|l| l.to_uppercase().contains("RETURN")).unwrap();
        assert!(body_indent(a_line) > 0, "assignment flattened to column 0 inside FUNCTION: {a_line:?}");
        assert_eq!(
            body_indent(a_line),
            body_indent(return_line),
            "assignment and RETURN should sit at the same depth inside the body: {a_line:?} vs {return_line:?}"
        );
        assert_eq!(body_indent(b_line), body_indent(return_line), "EQU should match its sibling statements' depth: {b_line:?}");
    }

    /// IF/REPEAT/WHILE/... never scope a symbol at all (confirmed live: a
    /// variable set inside a plain `IF` or `REPEAT` is still readable
    /// straight after it, unlike FUNCTION) - a definition textually inside
    /// one is exactly as global as a top-level one, and real project style
    /// found this way agrees: skyline's own source keeps such definitions
    /// flush left even several IFs deep. So, deliberately unlike FUNCTION,
    /// these keep the column-0 convention regardless of nesting.
    #[test]
    fn test_assign_inside_a_bare_if_or_repeat_stays_at_column_zero() {
        for src in ["if true\n    c = 1\nendif\n", "repeat 3, i, 0\n    c = i\nendrepeat\n"] {
            let out = fmt(src);
            let line = out.lines().find(|l| l.trim_start().starts_with('c')).unwrap();
            assert!(!line.starts_with(' '), "assignment indented outside any FUNCTION: {line:?} (from {src:?})");
        }
    }

    /// An IF/REPEAT/... nested *inside* a FUNCTION doesn't open a scope of
    /// its own either - an assignment inside it is still local to the
    /// enclosing FUNCTION, and should keep tracking depth like its sibling
    /// statements, not fall back to column 0 just because the nearest
    /// wrapper isn't the FUNCTION keyword itself.
    #[test]
    fn test_assign_inside_an_if_nested_in_a_function_still_tracks_depth() {
        let out = fmt("function foo x\n    if x\n        a = 1\n    endif\n    return a\nendfunction\n");
        let lines: Vec<&str> = out.lines().collect();
        let body_indent = |line: &str| line.len() - line.trim_start().len();
        let a_line = lines.iter().find(|l| l.contains("a = 1")).unwrap();
        assert!(body_indent(*a_line) > 0, "assignment flattened to column 0 inside IF-inside-FUNCTION: {a_line:?}");
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

    // ── one-instruction-per-line splitting is token-driven, not text-guessed ──
    //
    // Found via a real project's own source (birthtro), and specifically the
    // reason each of these is real: a text-only "is this `:` a label colon"
    // heuristic cannot fully disambiguate itself no matter how many special
    // cases it grows (confirmed by needing four rounds of them) - the real
    // parser already knows the answer for every one of these, from its own
    // grammar, with no guessing. `format_simple`/`format_label` now split
    // purely from each token's own span instead.

    /// A chain of zero-operand mnemonics with no spaces at all - a real,
    /// deliberate idiom in cycle-exact code (NOP padding for timing), and
    /// this project's own "timing in NOPs" convention. `NOP` is the first
    /// word of its own statement and starts with a letter - lexically
    /// indistinguishable from a label unless the *word itself* is checked
    /// against the real mnemonic set the parser already enforces.
    #[test]
    fn test_nop_chain_splits_into_separate_lines() {
        let out = fmt("NOP:NOP:NOP:NOP");
        let count = out.lines().filter(|l| l.trim() == "NOP").count();
        assert_eq!(count, 4, "expected 4 separate NOP lines: {out:?}");
    }

    /// A numeric operand immediately followed by `:` (no space) - can never
    /// be a label (labels are identifiers, not bare numbers), so the `:`
    /// is a real instruction separator.
    #[test]
    fn test_numeric_operand_before_colon_still_splits() {
        let out = fmt("org 0x4000\n\tld bc, 0x7F54: xor a");
        assert!(out.lines().any(|l| l.trim() == "LD BC, 0x7F54"), "{out:?}");
        assert!(out.lines().any(|l| l.trim() == "XOR A"), "{out:?}");
    }

    /// An identifier-shaped operand (not the first word of its statement)
    /// immediately followed by `:` - lexically shaped just like a label, but
    /// it's `AND`'s operand, not a label definition; a label can only be the
    /// first word of a statement.
    #[test]
    fn test_operand_identifier_before_colon_still_splits() {
        let out = fmt("org 0x4000\n\tAND SOME_CONST: ld (HL), A");
        assert!(out.lines().any(|l| l.trim() == "AND SOME_CONST"), "{out:?}");
        assert!(out.lines().any(|l| l.trim() == "LD (HL), A"), "{out:?}");
    }

    /// `:` immediately closed by `)` on its left - can never be a label
    /// colon (a label is an identifier, `)` isn't one), so this still splits
    /// even though nothing but a macro call precedes it.
    #[test]
    fn test_colon_after_closing_paren_still_splits() {
        let out = fmt("org 0x4000\n\tfoo (void): ld de,4");
        assert!(out.lines().any(|l| l.trim() == "foo (void)"), "{out:?}");
        assert!(out.lines().any(|l| l.trim() == "LD DE,4"), "{out:?}");
    }

    /// A genuine label immediately followed by an instruction on the same
    /// line, with no `:` between them at all - the real parser still
    /// produces two separate tokens (a label definition, then an
    /// instruction), so this splits correctly with no text-side "does this
    /// look like label-then-instruction" guessing at all.
    #[test]
    fn test_label_with_trailing_instruction_no_colon() {
        let out = fmt("org 0x4000\nmyloop ld a,0");
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.iter().any(|l| l.trim() == "myloop:"), "{out:?}");
        assert!(lines.iter().any(|l| l.trim() == "LD A,0"), "{out:?}");
    }

    /// A trailing comment after a label+instruction pair on one source line
    /// attaches to the instruction (the real last content on the line), not
    /// the label.
    #[test]
    fn test_trailing_comment_attaches_to_the_real_last_token() {
        let out = fmt("org 0x4000\nmyloop: ld a,0 ; hello");
        let instr_line = out.lines().find(|l| l.contains("LD A,0")).unwrap();
        assert!(instr_line.contains("; hello"), "{instr_line:?}");
        let label_line = out.lines().find(|l| l.trim() == "myloop:").unwrap();
        assert!(!label_line.contains(';'), "comment leaked onto the label line: {label_line:?}");
    }

    // ── MACRO bodies are formatted for real, not copied verbatim ───────────

    /// A macro body is captured as raw text (genuinely re-parsed on every
    /// call), not a token list - but confirmed against two real projects,
    /// ~97% of real macro bodies parse fine completely on their own even
    /// with `{param}` placeholders (recognised syntax everywhere an
    /// expression can appear), so they're formatted for real: case
    /// transforms, numeric literals, re-indentation one level deeper than
    /// the MACRO/ENDM lines, same as any other block body.
    #[test]
    fn test_macro_body_is_actually_formatted() {
        let out = fmt("macro FOO x\n\tld   a,b\n\tld hl,0x1234\nendm");
        assert!(out.contains("        LD A,B"), "macro body not formatted: {out:?}");
        assert!(out.contains("        LD HL,0x1234"), "{out:?}");
    }

    /// A macro body whose placeholder use genuinely doesn't parse on its own
    /// (e.g. `{reg}` standing in for a whole register operand, not a value)
    /// falls back to a verbatim copy - the only thing possible before this -
    /// rather than losing or corrupting content the real parser can't make
    /// sense of without an actual call to substitute into.
    #[test]
    fn test_unparseable_macro_body_falls_back_to_verbatim() {
        // `:=` is not valid Z80/basm syntax on its own - guaranteed to fail
        // to parse standalone regardless of what this formatter ever learns
        // to recognise, unlike a real placeholder use that might start
        // parsing successfully as this crate's own coverage improves.
        let src = "macro FOO x\n\tld a, {x} := broken\nendm";
        let out = fmt(src);
        assert!(out.contains("\tld a, {x} := broken"), "verbatim fallback lost/changed content: {out:?}");
    }

    /// Regression: reformatting a macro body used to silently drop a genuine
    /// blank line sitting right before `ENDM` on a *second* formatting pass
    /// (found chasing idempotency on real macro-heavy files) - traced to
    /// `Vec::join("\n")` not round-tripping a trailing blank line back
    /// through a later `.lines()` call, and separately to nothing flushing
    /// trailing blank/comment lines after the last real token in any
    /// buffer (whole file or macro body alike).
    #[test]
    fn test_blank_line_before_endm_survives_two_formatting_passes() {
        let once = fmt("macro FOO x\n\tld a,0\n\nendm");
        let twice = format(&once, &AsmFormatOptions::default()).unwrap();
        assert_eq!(once, twice, "blank line before ENDM did not survive a second pass: {once:?} -> {twice:?}");
        assert!(once.contains("\n\n"), "the blank line should still be there at all: {once:?}");
    }

    // ── `; fmt: off` / `; fmt: on` ──────────────────────────────────────────

    /// The region between the markers passes through completely unchanged -
    /// case, spacing, indentation, all of it - while code outside it still
    /// gets formatted normally. `pragma`'s own unit tests cover marker
    /// recognition/range-finding in isolation; this pins the end-to-end
    /// behavior through the public `format` entry point.
    #[test]
    fn test_fmt_off_on_preserves_the_region_verbatim() {
        let src = "org 0x4000\nld a,0\n; fmt: off\nFONT_CREATE_CHAR( dot,\n\t\"..\" ,\n      \"##\" )\n; fmt: on\nld   b,1\n";
        let out = fmt(src);
        assert!(out.contains("FONT_CREATE_CHAR( dot,\n\t\"..\" ,\n      \"##\" )"), "region not preserved verbatim: {out:?}");
        assert!(out.contains("LD A,0"), "code before the region should still be formatted: {out:?}");
        assert!(out.contains("LD B,1"), "code after the region should still be formatted: {out:?}");
    }

    /// An unclosed `; fmt: off` disables formatting for the rest of the file
    /// rather than silently reformatting past it.
    #[test]
    fn test_unclosed_fmt_off_disables_to_end_of_file() {
        let out = fmt("org 0x4000\nld a,0\n; fmt: off\nld   b,1");
        assert!(out.contains("LD A,0"));
        assert!(out.contains("ld   b,1"), "unclosed fmt:off must still suppress everything after it: {out:?}");
    }

    /// Round-trip sanity: formatting must be idempotent - running it twice
    /// must produce the same output as running it once. (This is what
    /// surfaced every case above in the first place: some of them left a
    /// growing phantom blank line behind instead of splitting.)
    #[test]
    fn test_formatting_is_idempotent_on_every_case_above() {
        for src in [
            "NOP:NOP:NOP:NOP",
            "org 0x4000\n\tld bc, 0x7F54: xor a",
            "org 0x4000\n\tAND SOME_CONST: ld (HL), A",
            "org 0x4000\n\tfoo (void): ld de,4",
            "org 0x4000\nmyloop ld a,0",
            "org 0x4000\nmyloop: ld a,0 ; hello"
        ] {
            let once = fmt(src);
            let twice = format(&once, &AsmFormatOptions::default()).unwrap();
            assert_eq!(once, twice, "not idempotent for {src:?}");
        }
    }
}

//! Validates the pattern parser against the real, unmodified upstream
//! `mdlz80optimizer` pattern library (vendored verbatim under
//! `src/vendor/`, see the `NOTICE.md` there).
//!
//! The point is coverage against reality rather than against hand-written
//! examples: these files use every corner of the format (wildcards, repeats,
//! `?op`/`?const`/`?any` variables, arithmetic in operands, `include`,
//! constraints this crate does not evaluate yet), so a grammar regression that
//! would break real files fails here instead of surfacing later.

use cpclib_asmoptim::dsl::{MnemonicPattern, RuleSet};

const MAIN: &str = include_str!("../src/vendor/pbo-patterns.txt");
const SIZE: &str = include_str!("../src/vendor/pbo-patterns-size.txt");
const SPEED: &str = include_str!("../src/vendor/pbo-patterns-speed.txt");

fn resolve(path: &str) -> Option<String> {
    match path {
        "pbo-patterns.txt" => Some(MAIN.to_string()),
        "pbo-patterns-size.txt" => Some(SIZE.to_string()),
        "pbo-patterns-speed.txt" => Some(SPEED.to_string()),
        _ => None
    }
}

#[test]
fn the_whole_upstream_pattern_library_parses() {
    let set = RuleSet::parse(MAIN).expect("upstream pbo-patterns.txt must parse");
    // 187 `pattern:` blocks upstream at the time of vendoring; assert a floor
    // rather than an exact count so a future re-vendor that adds patterns
    // doesn't fail spuriously, while a parser regression that silently drops
    // most of them still does.
    assert!(
        set.rules.len() >= 180,
        "expected ~187 patterns, parsed {}",
        set.rules.len()
    );
    // Every rule must have an anchor line - enforced by the parser, asserted
    // here so the guarantee is visible to the engine that relies on it.
    assert!(set.rules.iter().all(|r| r.has_anchor()));
}

#[test]
fn the_size_and_speed_files_parse_and_pull_in_the_base_library_via_include() {
    let base = RuleSet::parse(MAIN).unwrap().rules.len();

    let size = RuleSet::parse_with_includes(SIZE, resolve).unwrap();
    assert!(
        size.rules.len() > base,
        "size file should add its own patterns on top of the included base"
    );

    let speed = RuleSet::parse_with_includes(SPEED, resolve).unwrap();
    assert!(speed.rules.len() > base);
}

#[test]
fn the_real_cpc_tagged_jp_to_jr_pattern_is_present_and_well_formed() {
    let set = RuleSet::parse(MAIN).unwrap();
    let rule = set
        .rules
        .iter()
        .find(|r| r.name.as_deref() == Some("jp2jr"))
        .expect("upstream carries a `cpc`-tagged jp2jr pattern");

    assert!(rule.tags.iter().any(|t| t == "cpc"));
    assert_eq!(rule.match_lines.len(), 1);
    assert_eq!(rule.replacement_lines.len(), 1);
    assert!(rule.constraints.iter().any(|c| c.name == "reachableByJr"));
}

/// Sanity-check that mnemonics really were parsed as mnemonics rather than
/// swallowed into operands - a whole-corpus regression of that kind would
/// otherwise still "parse" and silently match nothing later.
#[test]
fn parsed_rules_carry_plausible_mnemonics() {
    let set = RuleSet::parse(MAIN).unwrap();
    let mut literal = 0usize;
    let mut variable = 0usize;
    for rule in &set.rules {
        for line in &rule.match_lines {
            if let cpclib_asmoptim::dsl::InstrPattern::Instr { mnemonic, .. } = &line.instr {
                match mnemonic {
                    MnemonicPattern::Literal(name) => {
                        assert!(
                            name.chars().all(|c| c.is_ascii_alphanumeric()),
                            "implausible mnemonic {name:?}"
                        );
                        literal += 1;
                    },
                    MnemonicPattern::Variable(_) => variable += 1
                }
            }
        }
    }
    assert!(literal > 200, "expected many literal mnemonics, got {literal}");
    assert!(variable > 0, "upstream uses ?op mnemonic variables");
}

/// The constraint vocabulary actually used upstream. Recorded as a test so
/// that implementing a new constraint in the engine has a concrete, real
/// target list, and so a parser change that starts mis-reading constraint
/// names is caught immediately.
#[test]
fn the_upstream_constraint_vocabulary_is_recognised() {
    let set = RuleSet::parse(MAIN).unwrap();
    let mut names: Vec<String> = set
        .rules
        .iter()
        .flat_map(|r| r.constraints.iter().map(|c| c.name.clone()))
        .collect();
    names.sort();
    names.dedup();

    for expected in [
        "atLeastOneCPUOp",
        "equal",
        "evenPushPopsSPNotRead",
        "flagsNotModified",
        "flagsNotUsed",
        "flagsNotUsedAfter",
        "in",
        "memoryNotWritten",
        "notEqual",
        "notIn",
        "reachableByJr",
        "regFlagEffectsNotUsedAfter",
        "regpair",
        "regsNotModified",
        "regsNotUsed",
        "regsNotUsedAfter"
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "expected upstream constraint {expected:?} among {names:?}"
        );
    }
}

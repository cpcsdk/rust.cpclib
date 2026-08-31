//! Our engine, run against the corpus the reference implementation is held to.
//!
//! `tests/fixtures/upstream_potests/` is mdlz80optimizer's own peephole test
//! data (see `src/vendor/NOTICE.md`): 88 inputs, 37 of them paired with an
//! `-expected.asm` recording what upstream actually produces. The value of
//! these files is that they are not ours - every other test in this crate
//! checks the engine against assumptions we wrote down ourselves.
//!
//! Comparison is **semantic**, never textual. Upstream reformats as it emits
//! (`ld a,(value)` becomes `ld a, (value)`) and renames labels it moves
//! (`end:` becomes `__mdlrenamed__end:`), so the only meaningful question is
//! what the two outputs assemble to.
//!
//! We deliberately do not aim to reproduce upstream exactly. This crate targets
//! the CPC and is more conservative in several documented places: `tstatez80`
//! rules are filtered, `TIMING_HOSTILE_RULES` are held back because deleting an
//! instruction whose output is dead usually means deleting cycle padding, and
//! `memoryNot*`/`noStackArguments` answer `Unknown` rather than guess. So the
//! property worth asserting is not equality - it is that **we never go further
//! than upstream did**.

use std::path::{Path, PathBuf};

use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::parser::{LocatedToken, parse_z80_str};
use cpclib_asmoptim::edit::edit_for_match;
use cpclib_asmoptim::engine::find_matches;
use cpclib_asmoptim::{OptimizationGoal, builtin_rules};
use cpclib_tokens::{ListingElement, Token};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upstream_potests")
}

/// Every input in the corpus, `-expected.asm` files excluded.
fn inputs() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .expect("the vendored corpus must be present")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|e| e == "asm")
                && !p.file_stem().is_some_and(|s| {
                    s.to_string_lossy().ends_with("-expected")
                })
        })
        .collect();
    paths.sort();
    paths
}

/// The instructions a source assembles to, ignoring layout, labels and
/// comments - what "the same code" actually means here.
fn instructions(source: &str) -> Option<Vec<Token>> {
    let listing = parse_z80_str(source).ok()?;
    Some(
        flatten_for_analysis(listing.iter())
            .filter(|t: &&LocatedToken| t.mnemonic().is_some())
            .map(|t| t.to_token().into_owned())
            .collect()
    )
}

/// How many of each instruction disappeared between `before` and `after`.
///
/// Keyed by the instruction's own debug form, which is enough to tell
/// `push bc` from `push de` while ignoring layout entirely.
fn removed(before: &[Token], after: &[Token]) -> std::collections::HashMap<String, usize> {
    let count = |ts: &[Token]| {
        let mut m: std::collections::HashMap<String, usize> = Default::default();
        for t in ts {
            *m.entry(format!("{t:?}")).or_default() += 1;
        }
        m
    };
    let (b, a) = (count(before), count(after));
    b.into_iter()
        .filter_map(|(k, n)| {
            let left = a.get(&k).copied().unwrap_or(0);
            // `then`, not `then_some`: the latter evaluates the subtraction
            // eagerly and underflows when nothing was removed.
            (n > left).then(|| (k, n - left))
        })
        .collect()
}

/// Run our optimizer over `source` and return the rewritten text.
///
/// Applies every suggestion at once, highest offset first, the same way
/// `cpclib_basmopt::apply_fixes` does - reusing `edit_for_match` so this tests
/// the real apply path rather than a test-only reimplementation of it.
fn optimize(source: &str) -> String {
    let listing = parse_z80_str(source).expect("fixture must parse");
    let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();
    let matches = find_matches(&tokens, builtin_rules(OptimizationGoal::Size));

    let mut edits: Vec<_> = matches
        .iter()
        .filter_map(|m| edit_for_match(source, &tokens, m))
        .collect();
    edits.sort_by_key(|e| std::cmp::Reverse(e.range.start));

    let mut out = source.to_owned();
    for edit in edits {
        out.replace_range(edit.range.clone(), &edit.text);
    }
    out
}

/// Nothing in the corpus may panic the engine, and whatever it produces must
/// still be assemblable. This is the floor: 88 real files written by someone
/// else, exercising instruction shapes our own tests never thought of.
#[test]
fn every_upstream_input_is_analysed_and_rewritten_into_valid_assembly() {
    let mut analysed = 0;
    for path in inputs() {
        let source = std::fs::read_to_string(&path).unwrap();
        // A handful of the fixtures use directives basm does not share; those
        // are not our concern, and skipping them is honest as long as the
        // count below stays high.
        if parse_z80_str(&source).is_err() {
            continue;
        }
        analysed += 1;

        let fixed = optimize(&source);
        assert!(
            parse_z80_str(&fixed).is_ok(),
            "{}: optimized output no longer parses:\n{fixed}",
            path.display()
        );
    }
    assert!(
        analysed >= 60,
        "only {analysed} of the corpus parsed - the fixtures or the parser regressed"
    );
}

/// The real differential assertion: **we must never optimize more aggressively
/// than the reference implementation.**
///
/// If our output ever contains *fewer* instructions than upstream's expected
/// output, we removed something upstream chose to keep - which, for an engine
/// whose entire job is deciding when *not* to act, is the failure that matters.
/// The reverse (us keeping more) is expected and fine: it is what all the
/// documented conservatism produces.
#[test]
fn we_are_never_more_aggressive_than_upstream() {
    let mut compared = 0;
    let mut identical = 0;
    let mut violations: Vec<String> = Vec::new();

    for path in inputs() {
        let expected_path = path.with_file_name(format!(
            "{}-expected.asm",
            path.file_stem().unwrap().to_string_lossy()
        ));
        if !expected_path.exists() {
            continue;
        }

        let source = std::fs::read_to_string(&path).unwrap();
        let expected = std::fs::read_to_string(&expected_path).unwrap();
        let (Some(before), Some(theirs)) = (instructions(&source), instructions(&expected))
        else {
            continue;
        };
        let Some(ours) = instructions(&optimize(&source))
        else {
            panic!("{}: our own output does not parse", path.display())
        };
        compared += 1;

        // Compare *what was removed*, not how many instructions are left.
        // A rewrite can perfectly well add instructions (upstream's expected
        // output for `test79` is longer than its input), so a count is no
        // proxy for aggressiveness. What "never more aggressive" really means
        // is: every instruction we took out, upstream took out too.
        let theirs_removed = removed(&before, &theirs);
        for (kind, gone_from_ours) in removed(&before, &ours) {
            let gone_from_theirs = theirs_removed.get(&kind).copied().unwrap_or(0);
            if gone_from_ours > gone_from_theirs {
                violations.push(format!(
                    "{}: removed {gone_from_ours}x {kind} (upstream: {gone_from_theirs})",
                    path.file_name().unwrap().to_string_lossy()
                ));
            }
        }
        if ours == theirs {
            identical += 1;
        }
    }

    // Every place we go further than upstream must be named and justified
    // here. The list is short on purpose: the two entries it started with were
    // both real bugs (a mid-line `:` join that commented out two instructions,
    // and an indexed operand whose `-` became `+`, silently changing an
    // address), and only this one survived scrutiny.
    let expected_divergences = [
        // `ld (ix+2),a` immediately followed by `ld a,(ix+2)`: the reload is
        // redundant and rewriting it is sound. Upstream declines only because
        // an *empty* `*` region is absent from its match map, and its
        // `regsNotUsed` returns false for a missing index (`Pattern.java`) -
        // an artefact of how the region is recorded, not a safety judgement.
        "test71.asm"
    ];
    let unexpected: Vec<&String> = violations
        .iter()
        .filter(|v| !expected_divergences.iter().any(|e| v.starts_with(e)))
        .collect();
    assert!(
        unexpected.is_empty(),
        "we optimized away something the reference implementation kept:\n{}",
        unexpected
            .iter()
            .map(|v| format!("  {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        compared >= 25,
        "only {compared} pairs were comparable; the corpus or the parser regressed"
    );
    // Recorded rather than demanded. Matching upstream exactly is not the goal
    // (see this file's own header), but the number moving should be a
    // deliberate, visible event either way.
    assert!(
        identical >= 3,
        "only {identical} of {compared} pairs match upstream exactly - a drop this \
         large means a rule stopped firing, not that we got more careful"
    );
    eprintln!("upstream potests: {identical}/{compared} pairs match exactly");
}

//! The rule sets compiled into this crate.
//!
//! The pattern files under `vendor/` are upstream's own, kept verbatim (see
//! `vendor/NOTICE.md`). Selecting which of their rules actually apply happens
//! here, at load time, rather than by hand-editing a fork - so re-vendoring a
//! newer upstream release stays a plain file copy.
//!
//! Two things do the selecting:
//!
//! * **The optimization goal.** Upstream ships a size-oriented and a
//!   speed-oriented file, and they contain *directly opposing* rules (size
//!   rewrites `jp` to `jr`; speed rewrites `jr` to `jp`). Loading both would
//!   produce contradictory suggestions, so a goal has to be chosen.
//! * **Platform tags.** Upstream tags rules with the CPU/dialect they apply
//!   to; see [`is_applicable`] for what each tag means here.
//!
//! Rules whose constraints this crate cannot evaluate are *not* filtered out
//! at this level - the engine skips them on its own, so they start working
//! automatically as constraints get implemented.

use std::sync::LazyLock;

use crate::dsl::{Rule, RuleSet};

const BASE: &str = include_str!("vendor/pbo-patterns.txt");
const SIZE: &str = include_str!("vendor/pbo-patterns-size.txt");
const SPEED: &str = include_str!("vendor/pbo-patterns-speed.txt");

/// What the suggestions should optimize for.
///
/// Upstream's size and speed rule files genuinely disagree with one another,
/// so this is a real choice rather than a preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizationGoal {
    /// Only the base rules - the ones that are wins either way.
    #[default]
    Neutral,
    /// Prefer smaller code (e.g. `jp` becomes `jr` where it reaches).
    Size,
    /// Prefer faster code (e.g. `jr` becomes `jp` on CPUs where that helps).
    Speed
}

/// Whether a rule applies to the Amstrad CPC.
///
/// Upstream documents its tag vocabulary in `pbo-patterns.txt`'s own header:
///
/// * `cpc` - "will only be loaded when z80cpc cpu is selected". That is us.
/// * `tstatez80` - "will only be loaded on t-state-based z80s (z80/z80msx)".
///   **Not** us: the CPC's memory contention rounds every instruction up to a
///   multiple of 4 t-states, which is exactly why upstream carries a separate
///   `cpc`-tagged rule that goes the *opposite* way from the `tstatez80` one.
/// * `sdcc-unsafe` - "will not be loaded when the sdcc/sdasz80 dialects are
///   selected". We assemble with basm, so these are fine.
///
/// An unknown tag is treated as applicable: a future upstream tag should not
/// silently disable rules, and the engine's own constraint checking is what
/// actually guarantees safety.
///
/// This used to also hold back, by name, the handful of rules that delete an
/// instruction purely because its output is dead - real, sound data-flow
/// judgments that are nonetheless risky to apply *unreviewed* on the CPC, an
/// instruction's duration being part of its meaning here (see
/// `Rule::is_pure_dead_output_deletion`'s own doc comment for the measured
/// real-world reason). That's a bulk-*application* concern, not a "should
/// this rule exist at all" one, so it no longer excludes anything at load
/// time - every evaluable rule stays in the normal set, offered as an
/// ordinary diagnostic/quickfix, and each bulk-apply call site
/// (`cpclib-lsp::basm::peephole::fix_all_peephole_edit`, `basmopt
/// --in-place`) is responsible for skipping `is_pure_dead_output_deletion`
/// matches itself.
pub fn is_applicable(rule: &Rule) -> bool {
    !rule.tags.iter().any(|tag| tag == "tstatez80")
}

/// Parse one of the vendored files, resolving its `include` against the others.
fn parse_vendored(source: &str) -> RuleSet {
    RuleSet::parse_with_includes(source, |path| {
        match path {
            "pbo-patterns.txt" => Some(BASE.to_string()),
            "pbo-patterns-size.txt" => Some(SIZE.to_string()),
            "pbo-patterns-speed.txt" => Some(SPEED.to_string()),
            _ => None
        }
    })
    .expect("vendored pattern files must parse")
}

fn load(source: &str) -> RuleSet {
    let mut set = parse_vendored(source);
    set.rules.retain(is_applicable);
    set
}

static NEUTRAL_RULES: LazyLock<RuleSet> = LazyLock::new(|| load(BASE));
static SIZE_RULES: LazyLock<RuleSet> = LazyLock::new(|| load(SIZE));
static SPEED_RULES: LazyLock<RuleSet> = LazyLock::new(|| load(SPEED));

/// The built-in rules for `goal`, parsed once.
pub fn builtin_rules(goal: OptimizationGoal) -> &'static RuleSet {
    match goal {
        OptimizationGoal::Neutral => &NEUTRAL_RULES,
        OptimizationGoal::Size => &SIZE_RULES,
        OptimizationGoal::Speed => &SPEED_RULES
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::all_supported;

    fn supported_names(goal: OptimizationGoal) -> Vec<&'static str> {
        builtin_rules(goal)
            .rules
            .iter()
            .filter(|r| all_supported(&r.constraints))
            .map(|r| r.name.as_deref().unwrap_or("<unnamed>"))
            .collect()
    }

    /// Forces every `LazyLock`, so a bad vendored file or a parser regression
    /// fails here rather than panicking on first real use.
    #[test]
    fn every_builtin_rule_set_parses() {
        for goal in [
            OptimizationGoal::Neutral,
            OptimizationGoal::Size,
            OptimizationGoal::Speed
        ] {
            let set = builtin_rules(goal);
            assert!(!set.rules.is_empty(), "{goal:?} rule set is empty");
            assert!(
                set.rules.iter().all(|r| r.has_anchor()),
                "{goal:?} has a rule with no anchor line"
            );
        }
    }

    /// The size and speed goals must each include the base library through
    /// their `include` directive, not just their own handful of rules.
    #[test]
    fn the_goal_sets_include_the_base_library() {
        let base = builtin_rules(OptimizationGoal::Neutral).rules.len();
        assert!(builtin_rules(OptimizationGoal::Size).rules.len() > base);
        assert!(builtin_rules(OptimizationGoal::Speed).rules.len() > base);
    }

    /// The reason goals exist at all: upstream's size and speed files carry
    /// rules that undo one another, so they must never be loaded together.
    #[test]
    fn the_size_and_speed_goals_really_do_disagree() {
        let size_has_jp2jr = builtin_rules(OptimizationGoal::Size)
            .rules
            .iter()
            .any(|r| r.name.as_deref() == Some("jp2jr"));
        assert!(size_has_jp2jr, "the size goal should turn jp into jr");

        // The opposing speed rule is `tstatez80`-tagged, so on the CPC it is
        // filtered out - which is the whole point of the tag handling. It
        // must be present in the raw file but absent after filtering.
        let raw_speed = parse_vendored(SPEED);
        assert!(
            raw_speed
                .rules
                .iter()
                .any(|r| r.description.contains("Replace jr") && !is_applicable(r)),
            "upstream's speed file should carry a tstatez80-tagged jr->jp rule"
        );
        assert!(
            !builtin_rules(OptimizationGoal::Speed)
                .rules
                .iter()
                .any(|r| r.description.contains("Replace jr ?const with jp")),
            "the tstatez80 jr->jp rule must not survive tag filtering on the CPC"
        );
    }

    /// The `cpc`-tagged rule upstream ships specifically for this platform
    /// must survive filtering.
    #[test]
    fn the_cpc_tagged_rule_is_kept() {
        let rule = builtin_rules(OptimizationGoal::Size)
            .rules
            .iter()
            .find(|r| r.name.as_deref() == Some("jp2jr"))
            .expect("jp2jr must be present under the size goal");
        assert!(rule.tags.iter().any(|t| t == "cpc"));
        assert!(is_applicable(rule));
    }

    /// Which rules are actually executable today - so implementing a
    /// constraint shows what it unlocked rather than only moving a count.
    #[test]
    fn the_executable_builtin_rules_are_known() {
        let neutral = supported_names(OptimizationGoal::Neutral);
        // 185 base rules are *evaluable* (see `upstream_engine.rs`) - all 185
        // are in the normal set; `Rule::is_pure_dead_output_deletion` rules
        // are only excluded from bulk-apply call sites, not from this set.
        assert_eq!(
            neutral.len(),
            185,
            "executable rule count changed; see upstream_engine.rs's own \
             assertion for the same number over the raw corpus"
        );

        // The ones the original structural-only constraint set covered - none
        // may regress...
        for name in [
            "neg-to-sub",
            "unnecessary-ld-to-itself",
            "regpair-transfer",
            "redundant-op",
            "jp2jr"
        ] {
            assert!(neutral.contains(&name), "{name} regressed");
        }
        // ...a few the forward-liveness constraints unlocked...
        for name in ["cp02ora", "ld0-to-xor", "cp12deca"] {
            assert!(neutral.contains(&name), "{name} should now be executable");
        }
        // ...and a few more from the block-local family and the
        // whole-instruction-effects one.
        for name in [
            "unnecessary-intermediate-reg",
            "unnecessary-ld-after-pop",
            "unnecessary-push-pop",
            "tail-recursion"
        ] {
            assert!(
                neutral.contains(&name),
                "{name} should be unlocked by the block-local constraints"
            );
        }

        // The size goal adds its own supported rules on top of the base ones.
        let size = supported_names(OptimizationGoal::Size);
        assert!(size.len() > supported_names(OptimizationGoal::Neutral).len());
        assert!(size.contains(&"jp2jr"));
    }
}

#[cfg(test)]
mod timing_tests {
    use super::*;

    /// Every rule this exhaustive, hand-verified list names is a genuine
    /// dead-output deletion (empty replacement + a liveness constraint) -
    /// found by dumping every empty-replacement rule in the base corpus and
    /// checking each one's constraints by hand (see the module doc comment
    /// on `Rule::is_pure_dead_output_deletion` for why this had to be done
    /// structurally rather than by name: the corpus turns out to contain an
    /// *unnamed* rule, "Remove redundant ?op a", with exactly this shape,
    /// which a name-based list could never cover). Two rules from that same
    /// "empty replacement" scan are deliberately absent because they are
    /// *not* liveness-dependent: `unnecessary-ld-to-itself` (`ld reg,reg` is
    /// always a no-op, independent of context) and the named `redundant-op`
    /// (its replacement keeps one copy - it rewrites, it doesn't delete).
    const KNOWN_DEAD_OUTPUT_DELETIONS: &[&str] = &[
        "unused-ld-any",
        "unused-ld-i",
        "unnecessary-op-const",
        "unnecessary-op-a-const",
        "unused-op-regpair",
        "unused-op-1arg",
        "unused-op-1arg2",
        "unused-op-2args",
        "unused-sub",
        "unnecessary-add",
        "unnecessary-adc-sbc",
        "unused-double-ld",
        "unnecessary-0args",
        "unnecessary-1args",
        "unnecessary-2args",
        "unnecessary-2args-ex"
    ];

    /// `Rule::is_pure_dead_output_deletion` classifies exactly the named
    /// rules above, plus the one unnamed one - not more, not fewer. Pinned
    /// so a future upstream re-vendor that adds or removes a rule of this
    /// shape is caught here rather than silently changing which rules are
    /// bulk-unsafe.
    #[test]
    fn is_pure_dead_output_deletion_matches_the_known_list_exactly() {
        let all = parse_vendored(BASE);
        let flagged: Vec<&Rule> = all
            .rules
            .iter()
            .filter(|r| r.is_pure_dead_output_deletion())
            .collect();

        for name in KNOWN_DEAD_OUTPUT_DELETIONS {
            assert!(
                flagged.iter().any(|r| r.name.as_deref() == Some(*name)),
                "{name} should be classified as a dead-output deletion"
            );
        }
        let unnamed_count = flagged.iter().filter(|r| r.name.is_none()).count();
        assert_eq!(
            unnamed_count, 1,
            "expected exactly the one known unnamed dead-output deletion \
             (\"Remove redundant ?op a\"): {flagged:?}"
        );
        assert_eq!(
            flagged.len(),
            KNOWN_DEAD_OUTPUT_DELETIONS.len() + 1,
            "a rule was added to or removed from this shape without updating \
             KNOWN_DEAD_OUTPUT_DELETIONS: {flagged:?}"
        );
    }

    /// Two structurally-similar-looking rules must NOT be flagged:
    /// `unnecessary-ld-to-itself` (no liveness constraint at all - `ld
    /// reg,reg` is always a no-op) and `redundant-op` (its replacement
    /// keeps one copy, so it isn't even a deletion).
    #[test]
    fn structurally_similar_but_safe_rules_are_not_flagged() {
        let all = parse_vendored(BASE);
        for name in ["unnecessary-ld-to-itself", "redundant-op"] {
            let rule = all
                .rules
                .iter()
                .find(|r| r.name.as_deref() == Some(name))
                .unwrap_or_else(|| panic!("{name} must exist in the corpus"));
            assert!(
                !rule.is_pure_dead_output_deletion(),
                "{name} should not be classified as a dead-output deletion"
            );
        }
    }

    /// Every dead-output deletion rule is still fully *evaluable* (not held
    /// back for lack of engine support) - the classification is about
    /// unreviewed bulk application, not a gap in the engine.
    #[test]
    fn the_dead_output_deletion_rules_are_still_fully_supported() {
        let all = parse_vendored(BASE);
        for rule in all.rules.iter().filter(|r| r.is_pure_dead_output_deletion()) {
            assert!(
                crate::constraints::all_supported(&rule.constraints),
                "{:?} should be fully evaluable: {rule:?}",
                rule.name
            );
        }
    }

    /// They reach a CPC user like any other rule - as an ordinary
    /// diagnostic/quickfix, individually reviewed. Only a bulk-apply call
    /// site is responsible for skipping them (see
    /// `Rule::is_pure_dead_output_deletion`'s own doc comment).
    #[test]
    fn the_dead_output_deletion_rules_are_present_in_the_normal_set() {
        for goal in [
            OptimizationGoal::Neutral,
            OptimizationGoal::Size,
            OptimizationGoal::Speed
        ] {
            let names: Vec<&str> = builtin_rules(goal)
                .rules
                .iter()
                .filter_map(|r| r.name.as_deref())
                .collect();
            for name in KNOWN_DEAD_OUTPUT_DELETIONS {
                assert!(
                    names.contains(name),
                    "{name} should be present (as an individually-reviewed \
                     rule) under {goal:?}"
                );
            }
        }
    }
}

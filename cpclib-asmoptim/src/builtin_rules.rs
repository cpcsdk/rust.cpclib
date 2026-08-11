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

/// Whether a rule applies to the Amstrad CPC, by its upstream tags.
///
/// Upstream documents the tag vocabulary in `pbo-patterns.txt`'s own header:
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
        assert_eq!(
            neutral.len(),
            173,
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
        // ...and a few more from the block-local family.
        for name in ["unnecessary-intermediate-reg", "unnecessary-ld-after-pop"] {
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

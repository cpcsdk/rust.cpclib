//! Ranking several candidate rule matches at the same token position.
//!
//! Upstream (`mdlz80optimizer`'s `PatternBasedOptimizer.java`) picks the
//! objectively best pattern at each position rather than the first one that
//! happens to match: most bytes saved, then most cycles saved, then fewest
//! constraints, then fewest lines involved. This crate's engine used to just
//! take whichever rule was listed first in the pattern file - see
//! `engine.rs`'s own module doc comment and its `find_matches_with_resolver`
//! for why that was a real gap, not a deliberate simplification. This module
//! is the comparator that closes it.
//!
//! One deliberate departure from upstream's fixed order: which of
//! bytes-saved/cycles-saved gets checked *first* depends on
//! [`crate::OptimizationGoal`] (see [`MatchCost::cmp_better`]) - a
//! demomaking-specific need, not something upstream's own general-purpose
//! optimizer has to care about. A standard demo cares about runtime speed;
//! a size-constrained intro (4k/8k) cares about what ships after crunching,
//! which raw byte savings only approximates (more regular/repetitive code
//! sometimes crunches better than fewer-but-irregular bytes) - a real
//! crunched-size estimate would need running an actual cruncher per
//! candidate, which this does not attempt.

use cpclib_asm::implementation::tokens::TokenExt;
use cpclib_tokens::{DataAccessElem, ListingElement};

use crate::OptimizationGoal;

/// One instruction's cycle cost, straight from `cpclib_z80flow::cost`'s own
/// static table - no assemble needed. `None` (unknown) for anything the
/// table does not cover, most notably a *fake* instruction (this corpus has
/// roughly 30 of them, e.g. `ld hl, de`) - `opcode_duration` only prices real
/// opcodes, and the expansion logic that would price a fake one is
/// `pub(crate)` to `cpclib-z80flow` and unreachable from here. Deliberately
/// not `TokenExt::estimated_duration` for the same reason: its `OpCode` path
/// calls `opcode_duration` directly too, so it buys nothing extra while
/// adding a `Debug + Visited` bound this module does not otherwise need.
fn token_cycles<T>(token: &T) -> Option<u32>
where
    T: ListingElement,
    T::DataAccess: DataAccessElem
{
    match token.mnemonic() {
        // A label/comment/directive inside the span costs nothing at
        // runtime - not "unknown", genuinely zero.
        None => Some(0),
        Some(mnemonic) => {
            cpclib_z80flow::cost::opcode_duration(
                mnemonic,
                token.mnemonic_arg1(),
                token.mnemonic_arg2()
            )
        }
    }
}

/// Total cycle cost of a span of tokens, or `None` (unknown) the moment any
/// one of them is - deliberately all-or-nothing, the same "a partial sum
/// looks right and is wrong" reasoning `cpclib-z80flow`'s own `sum_expansion`
/// documents for the identical situation.
///
/// Generic over an iterator, not a slice: this needs to run both on the
/// original span (`&[&T]`, trivially iterable) and, in `engine.rs`, on a
/// replacement's freshly-parsed tokens *before* they go out of scope -
/// `LocatedToken::clone` is `unimplemented!()` in this codebase, so nothing
/// here may ever collect owned tokens to hand back.
pub(crate) fn span_cycles<'a, T>(tokens: impl IntoIterator<Item = &'a T>) -> Option<u32>
where
    T: ListingElement + 'a,
    T::DataAccess: DataAccessElem
{
    tokens.into_iter().try_fold(0u32, |acc, t| Some(acc + token_cycles(t)?))
}

/// Total byte size of a span of tokens, assembling each one standalone
/// (`TokenExt::number_of_bytes`, against a lax symbol table - see that
/// method's own doc comment) - `None` the moment any one of them fails to
/// assemble in isolation. See [`span_cycles`] for why this takes an iterator
/// rather than a slice.
pub(crate) fn span_bytes<'a, T>(tokens: impl IntoIterator<Item = &'a T>) -> Option<usize>
where T: TokenExt + 'a {
    tokens
        .into_iter()
        .try_fold(0usize, |acc, t| Some(acc + t.number_of_bytes().ok()?))
}

/// How one candidate match at a position compares to another. Mirrors
/// upstream's own tie-break order exactly: most bytes saved, then most
/// cycles saved, then fewest constraints, then fewest instructions spanned -
/// see the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatchCost {
    /// `original_bytes - replacement_bytes`; positive means smaller.
    /// `None` when either side's byte count could not be determined.
    bytes_saved: Option<i64>,
    /// `original_cycles - replacement_cycles`; positive means faster.
    /// `None` when either side's cycle count could not be determined (most
    /// often a fake instruction in the replacement - see `token_cycles`).
    cycles_saved: Option<i64>,
    /// The matched rule's own constraint count - fewer is preferred as a
    /// tiebreak, on the same "less to go wrong" reasoning upstream uses.
    constraint_count: usize,
    /// How many original instructions the match spans - fewer is preferred
    /// as the final tiebreak.
    span_len: usize
}

impl MatchCost {
    /// `original` is the matched span, in the caller's own token type.
    /// `replacement_bytes`/`replacement_cycles` are pre-computed by
    /// `engine.rs::replacement_cost` at the point the replacement's parsed
    /// listing is still alive (see [`span_cycles`]'s own doc comment for
    /// why the replacement's tokens themselves never reach this far).
    pub(crate) fn compute<'a, T>(
        original: impl IntoIterator<Item = &'a T>,
        replacement_bytes: Option<usize>,
        replacement_cycles: Option<u32>,
        constraint_count: usize,
        span_len: usize
    ) -> Self
    where
        T: TokenExt + 'a,
        T::DataAccess: DataAccessElem
    {
        let original: Vec<&T> = original.into_iter().collect();
        let bytes_saved = match (span_bytes(original.iter().copied()), replacement_bytes) {
            (Some(before), Some(after)) => Some(before as i64 - after as i64),
            _ => None
        };
        let cycles_saved = match (span_cycles(original.iter().copied()), replacement_cycles) {
            (Some(before), Some(after)) => Some(before as i64 - after as i64),
            _ => None
        };
        MatchCost {
            bytes_saved,
            cycles_saved,
            constraint_count,
            span_len
        }
    }

    /// `Greater` when `self` should win over `other`, for `goal`. A tier
    /// where either side is `Unknown` decides nothing there - it is a tie
    /// for that tier, not a win for the known side, and comparison falls
    /// through to the next one. When every tier ties, the two are declared
    /// `Equal` - callers keep whichever candidate was found first in that
    /// case, the same file-order behavior this comparator otherwise
    /// replaces.
    ///
    /// `goal` decides which of bytes-saved/cycles-saved is checked *first*
    /// - `Speed` prefers cycles first (a standard demo cares about runtime),
    /// `Size` and `Neutral` prefer bytes first (a size-constrained intro
    /// cares about what ships, and upstream's own fixed order has no
    /// stronger claim than "bytes first" absent a reason to prefer
    /// otherwise). Whichever runs second still applies as the very next
    /// tiebreak, before falling through to constraint/span-length - a
    /// `Speed` goal does not *ignore* bytes, it just does not let them
    /// override a real cycle-count difference.
    pub(crate) fn cmp_better(&self, other: &Self, goal: OptimizationGoal) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        let bytes_tier = |a: &Self, b: &Self| -> Option<Ordering> {
            match (a.bytes_saved, b.bytes_saved) {
                (Some(x), Some(y)) if x != y => Some(x.cmp(&y)),
                _ => None
            }
        };
        let cycles_tier = |a: &Self, b: &Self| -> Option<Ordering> {
            match (a.cycles_saved, b.cycles_saved) {
                (Some(x), Some(y)) if x != y => Some(x.cmp(&y)),
                _ => None
            }
        };
        let tiers: [&dyn Fn(&Self, &Self) -> Option<Ordering>; 2] = match goal {
            OptimizationGoal::Speed => [&cycles_tier, &bytes_tier],
            OptimizationGoal::Size | OptimizationGoal::Neutral => [&bytes_tier, &cycles_tier]
        };
        for tier in tiers {
            if let Some(ordering) = tier(self, other) {
                return ordering;
            }
        }

        if self.constraint_count != other.constraint_count {
            // Fewer constraints wins.
            return other.constraint_count.cmp(&self.constraint_count);
        }
        if self.span_len != other.span_len {
            // Fewer instructions spanned wins.
            return other.span_len.cmp(&self.span_len);
        }
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cost(
        bytes_saved: Option<i64>,
        cycles_saved: Option<i64>,
        constraint_count: usize,
        span_len: usize
    ) -> MatchCost {
        MatchCost {
            bytes_saved,
            cycles_saved,
            constraint_count,
            span_len
        }
    }

    #[test]
    fn more_bytes_saved_wins_outright_under_size() {
        let better = cost(Some(3), Some(0), 5, 5);
        let worse = cost(Some(1), Some(100), 0, 0);
        assert_eq!(
            better.cmp_better(&worse, OptimizationGoal::Size),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            worse.cmp_better(&better, OptimizationGoal::Size),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn tied_bytes_falls_through_to_cycles_under_size() {
        let better = cost(Some(2), Some(5), 3, 3);
        let worse = cost(Some(2), Some(1), 0, 0);
        assert_eq!(
            better.cmp_better(&worse, OptimizationGoal::Size),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn more_cycles_saved_wins_outright_under_speed() {
        // Same two candidates as `more_bytes_saved_wins_outright_under_size`,
        // but with cycles clearly favoring the *other* one - under `Speed`
        // the winner must flip, proving the goal actually swaps tier order
        // rather than just being accepted as a parameter.
        let more_bytes_fewer_cycles = cost(Some(3), Some(0), 5, 5);
        let fewer_bytes_more_cycles = cost(Some(1), Some(100), 0, 0);
        assert_eq!(
            fewer_bytes_more_cycles.cmp_better(&more_bytes_fewer_cycles, OptimizationGoal::Speed),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn tied_cycles_falls_through_to_bytes_under_speed() {
        let better = cost(Some(5), Some(2), 3, 3);
        let worse = cost(Some(1), Some(2), 0, 0);
        assert_eq!(
            better.cmp_better(&worse, OptimizationGoal::Speed),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn neutral_prefers_bytes_first_same_as_size() {
        let better = cost(Some(3), Some(0), 5, 5);
        let worse = cost(Some(1), Some(100), 0, 0);
        assert_eq!(
            better.cmp_better(&worse, OptimizationGoal::Neutral),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn unknown_bytes_falls_through_rather_than_losing_outright() {
        // No bytes data on either side - and no cycles data either - so the
        // comparison must reach the constraint-count tier, not just report a
        // tie because bytes/cycles are both unknown.
        let fewer_constraints = cost(None, None, 1, 2);
        let more_constraints = cost(None, None, 4, 2);
        assert_eq!(
            fewer_constraints.cmp_better(&more_constraints, OptimizationGoal::Size),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn fewer_constraints_wins_when_bytes_and_cycles_tie() {
        let fewer = cost(Some(1), Some(1), 1, 10);
        let more = cost(Some(1), Some(1), 3, 10);
        assert_eq!(
            fewer.cmp_better(&more, OptimizationGoal::Size),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn fewer_lines_is_the_final_tiebreak() {
        let shorter = cost(Some(1), Some(1), 2, 2);
        let longer = cost(Some(1), Some(1), 2, 5);
        assert_eq!(
            shorter.cmp_better(&longer, OptimizationGoal::Size),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn total_tie_reports_equal() {
        let a = cost(Some(1), Some(1), 2, 2);
        let b = cost(Some(1), Some(1), 2, 2);
        assert_eq!(
            a.cmp_better(&b, OptimizationGoal::Size),
            std::cmp::Ordering::Equal
        );
    }
}

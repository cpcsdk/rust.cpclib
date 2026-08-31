//! The value being tracked across a forward liveness walk, and how each
//! instruction narrows it.
//!
//! A direct port of upstream's `CPUOpDependency` (`code/CPUOpDependency.java`),
//! whose model is subtler than "a set of registers" and deliberately so:
//!
//! * A pair and either of its halves **match** each other. Tracking `BC` and
//!   hitting an instruction that reads `B` counts as a use - the value really
//!   was consumed.
//! * A write **narrows** rather than kills. Tracking `BC` and hitting a write
//!   to `B` leaves `C` still tracked, not nothing. This is the piece a
//!   "two independent booleans" model gets wrong: it is one dependency that
//!   shrinks, and it stays alive until *every* part of it has been
//!   overwritten.
//! * `AF` decomposes into `A` and `F`, so overwriting `A` leaves the flags
//!   still live as `F` - and a dependency on `F` matches a flag effect, and
//!   vice versa.
//!
//! Getting any of these backwards produces a wrong *"safe to optimize"*
//! answer rather than a crash, so each branch below has its own test.

use crate::regflag::{Flag, Reg};

/// One thing whose liveness is being tracked: either a register (possibly a
/// pair, possibly already narrowed to one half) or a single flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dependency {
    Reg(Reg),
    Flag(Flag)
}

impl Dependency {
    /// Whether an instruction that touches `other` touches *this* dependency.
    ///
    /// Symmetric across the pair/half relationship: `BC` matches `B`, and `B`
    /// matches `BC`. Also crosses the register/flag boundary in the one place
    /// the hardware does - `AF` is where the flags live.
    pub fn matches(self, other: Self) -> bool {
        match (self, other) {
            (Self::Reg(a), Self::Reg(b)) => regs_overlap(a, b),
            (Self::Flag(a), Self::Flag(b)) => a == b,
            // A flag dependency is touched by anything touching `AF` (or `F`),
            // and vice versa.
            (Self::Flag(_), Self::Reg(r)) | (Self::Reg(r), Self::Flag(_)) => {
                matches!(r, Reg::Af | Reg::F)
            }
        }
    }

    /// Apply a write of `written` to this dependency, returning what remains
    /// live afterwards - or `None` once nothing is left.
    ///
    /// This is the narrowing step: writing one half of a tracked pair leaves
    /// the other half tracked.
    pub fn after_write(self, written: Self) -> Option<Self> {
        match (self, written) {
            (Self::Flag(tracked), Self::Flag(w)) => (tracked != w).then_some(self),

            // Writing the whole of `AF` (or `F` itself) clears a tracked flag.
            (Self::Flag(_), Self::Reg(r)) => (!matches!(r, Reg::Af | Reg::F)).then_some(self),

            // A flag write never clears a tracked *register*, except that it
            // does narrow `AF` down to `A` - the flags half is gone.
            (Self::Reg(tracked), Self::Flag(_)) => {
                Some(match tracked {
                    Reg::Af => Self::Reg(Reg::A),
                    Reg::F => return None,
                    _ => self
                })
            },

            (Self::Reg(tracked), Self::Reg(w)) => narrow_reg(tracked, w).map(Self::Reg)
        }
    }
}

/// Whether two register names refer to any storage in common - i.e. whether
/// one is the other, or one is a half of the other.
fn regs_overlap(a: Reg, b: Reg) -> bool {
    a == b || a.pair() == Some(b) || b.pair() == Some(a)
}

/// What remains of a tracked register after `written` is overwritten.
fn narrow_reg(tracked: Reg, written: Reg) -> Option<Reg> {
    if tracked == written {
        return None;
    }

    // The tracked register is a half of what was written: fully covered.
    if tracked.pair() == Some(written) {
        return None;
    }

    // A half of the tracked pair was written: the *other* half survives.
    if written.pair() == Some(tracked)
        && let Some((high, low)) = tracked.halves()
    {
        return Some(if written == high { low } else { high });
    }

    // Unrelated registers - the dependency is untouched.
    Some(tracked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(r: Reg) -> Dependency {
        Dependency::Reg(r)
    }

    fn flag(f: Flag) -> Dependency {
        Dependency::Flag(f)
    }

    /// A pair and its halves are the same storage, so a use of either is a
    /// use of the other - in both directions.
    #[test]
    fn a_pair_and_its_halves_match_each_other_both_ways() {
        assert!(reg(Reg::Bc).matches(reg(Reg::B)));
        assert!(reg(Reg::B).matches(reg(Reg::Bc)));
        assert!(reg(Reg::Bc).matches(reg(Reg::C)));
        assert!(reg(Reg::Hl).matches(reg(Reg::L)));
        assert!(reg(Reg::Ix).matches(reg(Reg::Ixh)));
        assert!(reg(Reg::Af).matches(reg(Reg::A)));

        // ...but unrelated registers do not.
        assert!(!reg(Reg::Bc).matches(reg(Reg::De)));
        assert!(!reg(Reg::B).matches(reg(Reg::C)));
        assert!(!reg(Reg::Hl).matches(reg(Reg::Ixh)));
    }

    /// The narrowing rule, and the reason this isn't a set of booleans:
    /// writing half a pair leaves the other half alive.
    #[test]
    fn writing_one_half_narrows_the_dependency_to_the_other() {
        assert_eq!(reg(Reg::Bc).after_write(reg(Reg::B)), Some(reg(Reg::C)));
        assert_eq!(reg(Reg::Bc).after_write(reg(Reg::C)), Some(reg(Reg::B)));
        assert_eq!(reg(Reg::De).after_write(reg(Reg::D)), Some(reg(Reg::E)));
        assert_eq!(reg(Reg::Hl).after_write(reg(Reg::L)), Some(reg(Reg::H)));
        assert_eq!(reg(Reg::Ix).after_write(reg(Reg::Ixh)), Some(reg(Reg::Ixl)));
    }

    /// Writing the whole pair, or the exact register, ends the dependency.
    #[test]
    fn writing_the_whole_thing_kills_the_dependency() {
        assert_eq!(reg(Reg::Bc).after_write(reg(Reg::Bc)), None);
        assert_eq!(reg(Reg::B).after_write(reg(Reg::B)), None);
        // A half is fully covered by a write to its pair.
        assert_eq!(reg(Reg::B).after_write(reg(Reg::Bc)), None);
        assert_eq!(reg(Reg::Ixl).after_write(reg(Reg::Ix)), None);
    }

    #[test]
    fn writing_an_unrelated_register_changes_nothing() {
        assert_eq!(reg(Reg::Bc).after_write(reg(Reg::De)), Some(reg(Reg::Bc)));
        assert_eq!(reg(Reg::A).after_write(reg(Reg::Hl)), Some(reg(Reg::A)));
        assert_eq!(reg(Reg::Sp).after_write(reg(Reg::Hl)), Some(reg(Reg::Sp)));
    }

    /// `AF` is the interesting one: `A` and the flags share a pair, so
    /// overwriting `A` must leave the *flags* still tracked rather than
    /// killing the dependency outright.
    #[test]
    fn overwriting_a_leaves_the_flags_half_of_af_still_live() {
        assert_eq!(reg(Reg::Af).after_write(reg(Reg::A)), Some(reg(Reg::F)));
        assert_eq!(reg(Reg::Af).after_write(reg(Reg::F)), Some(reg(Reg::A)));
        // ...and a flag write narrows `AF` down to just `A`.
        assert_eq!(reg(Reg::Af).after_write(flag(Flag::Z)), Some(reg(Reg::A)));
        // ...while a tracked `F` is ended by any flag write.
        assert_eq!(reg(Reg::F).after_write(flag(Flag::C)), None);
    }

    #[test]
    fn flags_are_tracked_individually() {
        assert_eq!(flag(Flag::Z).after_write(flag(Flag::Z)), None);
        assert_eq!(
            flag(Flag::Z).after_write(flag(Flag::C)),
            Some(flag(Flag::Z))
        );
        assert!(flag(Flag::PV).matches(flag(Flag::PV)));
        assert!(!flag(Flag::PV).matches(flag(Flag::N)));
    }

    /// The register/flag crossover: everything about the flags lives in `AF`,
    /// so an instruction that saves or restores `AF` wholesale (`push af`,
    /// `pop af`, `ex af,af'`) really does use and clobber the flags.
    #[test]
    fn af_and_flags_touch_each_other() {
        assert!(flag(Flag::Z).matches(reg(Reg::Af)));
        assert!(reg(Reg::Af).matches(flag(Flag::Z)));
        assert!(flag(Flag::Z).matches(reg(Reg::F)));

        // `pop af` overwrites every flag.
        assert_eq!(flag(Flag::Z).after_write(reg(Reg::Af)), None);
        assert_eq!(flag(Flag::C).after_write(reg(Reg::F)), None);
        // ...but an unrelated register write leaves flags alone.
        assert_eq!(flag(Flag::Z).after_write(reg(Reg::Hl)), Some(flag(Flag::Z)));

        // A flag is not touched by `A` alone - only the `F` half matters.
        assert!(!flag(Flag::Z).matches(reg(Reg::A)));
        assert_eq!(flag(Flag::Z).after_write(reg(Reg::A)), Some(flag(Flag::Z)));
    }

    /// `SP` and `PC` are 16-bit but have no addressable halves, so they can
    /// only ever be written whole.
    #[test]
    fn registers_without_halves_are_all_or_nothing() {
        assert_eq!(reg(Reg::Sp).after_write(reg(Reg::Sp)), None);
        assert_eq!(reg(Reg::Pc).after_write(reg(Reg::Pc)), None);
        assert_eq!(reg(Reg::Sp).after_write(reg(Reg::Pc)), Some(reg(Reg::Sp)));
    }
}

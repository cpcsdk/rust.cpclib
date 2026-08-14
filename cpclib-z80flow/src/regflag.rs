//! The register and flag vocabulary the dataflow analysis speaks.
//!
//! `cpclib-tokens` already models registers, but split across four enums by
//! width and kind (`Register8`, `Register16`, `IndexRegister8`,
//! `IndexRegister16`) - the right shape for *assembling*, where an operand's
//! width is part of its syntax. Dataflow needs the opposite: one flat name
//! space where `B` and `BC` are directly comparable, because "is `BC` still
//! live?" has to be answerable when the next instruction writes only `B`.
//! It also needs names those enums deliberately don't carry as operands -
//! `F`, `PC` - because they show up as *effects* (`ADD A,B` writes `F`; every
//! jump writes `PC`).
//!
//! So [`Reg`] is one flat enum over every register name that can appear as an
//! instruction's input or output, with `From` conversions back to
//! `cpclib-tokens`' types where they exist.

use cpclib_tokens::{IndexRegister8, IndexRegister16, Register8, Register16};

/// A Z80 register name, at whatever width it was referred to.
///
/// Pairs and halves are *separate* variants rather than one canonical form:
/// an instruction that writes `B` must not be recorded as writing `BC`, and
/// the relationship between them is a question the dependency model answers
/// deliberately (see [`Reg::halves`]/[`Reg::pair`]), not something flattened
/// away at parse time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reg {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    /// The flags register. Never an operand, but a real effect target: it is
    /// what `AF` narrows down to once `A` alone has been overwritten.
    F,
    /// Interrupt vector register (`LD A,I`).
    I,
    /// Memory refresh register (`LD A,R`).
    R,
    Af,
    Bc,
    De,
    Hl,
    Ix,
    Iy,
    Ixh,
    Ixl,
    Iyh,
    Iyl,
    Sp,
    /// Written by every jump, call and return. Never an operand, but the
    /// instruction table lists it as an output, and carrying it keeps that
    /// data faithful rather than silently dropped.
    Pc
}

impl Reg {
    /// Parse a register name as written in a pattern constraint or in the
    /// vendored instruction table. Case-insensitive; `None` for anything that
    /// isn't a register name.
    pub fn parse(name: &str) -> Option<Self> {
        let upper = name.trim().to_ascii_uppercase();
        Some(match upper.as_str() {
            "A" => Self::A,
            "B" => Self::B,
            "C" => Self::C,
            "D" => Self::D,
            "E" => Self::E,
            "H" => Self::H,
            "L" => Self::L,
            "F" => Self::F,
            "I" => Self::I,
            "R" => Self::R,
            "AF" | "AF'" => Self::Af,
            "BC" => Self::Bc,
            "DE" => Self::De,
            "HL" => Self::Hl,
            "IX" => Self::Ix,
            "IY" => Self::Iy,
            "IXH" | "HX" | "XH" => Self::Ixh,
            "IXL" | "LX" | "XL" => Self::Ixl,
            "IYH" | "HY" | "YH" => Self::Iyh,
            "IYL" | "LY" | "YL" => Self::Iyl,
            "SP" => Self::Sp,
            "PC" => Self::Pc,
            _ => return None
        })
    }

    /// The canonical name, as the instruction table and pattern files write
    /// it.
    pub fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::H => "H",
            Self::L => "L",
            Self::F => "F",
            Self::I => "I",
            Self::R => "R",
            Self::Af => "AF",
            Self::Bc => "BC",
            Self::De => "DE",
            Self::Hl => "HL",
            Self::Ix => "IX",
            Self::Iy => "IY",
            Self::Ixh => "IXH",
            Self::Ixl => "IXL",
            Self::Iyh => "IYH",
            Self::Iyl => "IYL",
            Self::Sp => "SP",
            Self::Pc => "PC"
        }
    }

    /// The two halves of a 16-bit register, high first - `None` for anything
    /// that isn't a decomposable pair (`SP`/`PC` are 16-bit but have no
    /// separately-addressable halves).
    ///
    /// `AF`'s halves are `A` and `F`: the flags register really is the low
    /// half, which is exactly why overwriting `A` leaves a live `AF`
    /// dependency narrowed to `F` rather than killed.
    pub fn halves(self) -> Option<(Self, Self)> {
        Some(match self {
            Self::Af => (Self::A, Self::F),
            Self::Bc => (Self::B, Self::C),
            Self::De => (Self::D, Self::E),
            Self::Hl => (Self::H, Self::L),
            Self::Ix => (Self::Ixh, Self::Ixl),
            Self::Iy => (Self::Iyh, Self::Iyl),
            _ => return None
        })
    }

    /// The 16-bit register this one is a half of, if any.
    pub fn pair(self) -> Option<Self> {
        Some(match self {
            Self::A | Self::F => Self::Af,
            Self::B | Self::C => Self::Bc,
            Self::D | Self::E => Self::De,
            Self::H | Self::L => Self::Hl,
            Self::Ixh | Self::Ixl => Self::Ix,
            Self::Iyh | Self::Iyl => Self::Iy,
            _ => return None
        })
    }

    /// Whether this is a 16-bit name (whether or not it decomposes).
    pub fn is_pair(self) -> bool {
        matches!(
            self,
            Self::Af | Self::Bc | Self::De | Self::Hl | Self::Ix | Self::Iy | Self::Sp | Self::Pc
        )
    }
}

impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl From<Register8> for Reg {
    fn from(r: Register8) -> Self {
        match r {
            Register8::A => Self::A,
            Register8::B => Self::B,
            Register8::C => Self::C,
            Register8::D => Self::D,
            Register8::E => Self::E,
            Register8::H => Self::H,
            Register8::L => Self::L
        }
    }
}

impl From<Register16> for Reg {
    fn from(r: Register16) -> Self {
        match r {
            Register16::Af => Self::Af,
            Register16::Bc => Self::Bc,
            Register16::De => Self::De,
            Register16::Hl => Self::Hl,
            Register16::Sp => Self::Sp
        }
    }
}

impl From<IndexRegister8> for Reg {
    fn from(r: IndexRegister8) -> Self {
        match r {
            IndexRegister8::Ixh => Self::Ixh,
            IndexRegister8::Ixl => Self::Ixl,
            IndexRegister8::Iyh => Self::Iyh,
            IndexRegister8::Iyl => Self::Iyl
        }
    }
}

impl From<IndexRegister16> for Reg {
    fn from(r: IndexRegister16) -> Self {
        match r {
            IndexRegister16::Ix => Self::Ix,
            IndexRegister16::Iy => Self::Iy
        }
    }
}

/// One of the Z80's six meaningful condition flags.
///
/// The two undocumented bits (3 and 5) are deliberately absent: no pattern
/// constraint refers to them and no rule can depend on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Flag {
    /// Sign.
    S,
    /// Zero.
    Z,
    /// Half-carry.
    H,
    /// Parity/overflow - written `P/V`, one name containing a slash.
    PV,
    /// Add/subtract.
    N,
    /// Carry.
    C
}

impl Flag {
    /// Parse a flag name as written in a pattern constraint or in the
    /// vendored instruction table. Case-insensitive.
    ///
    /// Accepts `P/V` (the canonical spelling everywhere upstream) as well as
    /// the `PV`/`P` spellings, since the slash is easy to lose in transit -
    /// see `dsl::ident`, which had to be taught the same thing.
    pub fn parse(name: &str) -> Option<Self> {
        let upper = name.trim().to_ascii_uppercase();
        Some(match upper.as_str() {
            "S" => Self::S,
            "Z" => Self::Z,
            "H" => Self::H,
            "P/V" | "PV" | "P" | "V" => Self::PV,
            "N" => Self::N,
            "C" => Self::C,
            _ => return None
        })
    }

    /// The canonical name, as the instruction table and pattern files write
    /// it.
    pub fn name(self) -> &'static str {
        match self {
            Self::S => "S",
            Self::Z => "Z",
            Self::H => "H",
            Self::PV => "P/V",
            Self::N => "N",
            Self::C => "C"
        }
    }

    /// Every flag, for the "this instruction overwrites all of them" case.
    pub const ALL: [Flag; 6] = [Flag::S, Flag::Z, Flag::H, Flag::PV, Flag::N, Flag::C];
}

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_register_name_round_trips_through_parse_and_name() {
        for reg in [
            Reg::A,
            Reg::B,
            Reg::C,
            Reg::D,
            Reg::E,
            Reg::H,
            Reg::L,
            Reg::F,
            Reg::I,
            Reg::R,
            Reg::Af,
            Reg::Bc,
            Reg::De,
            Reg::Hl,
            Reg::Ix,
            Reg::Iy,
            Reg::Ixh,
            Reg::Ixl,
            Reg::Iyh,
            Reg::Iyl,
            Reg::Sp,
            Reg::Pc
        ] {
            assert_eq!(Reg::parse(reg.name()), Some(reg), "{reg}");
        }
    }

    #[test]
    fn register_parsing_is_case_insensitive_and_rejects_non_registers() {
        assert_eq!(Reg::parse("hl"), Some(Reg::Hl));
        assert_eq!(Reg::parse("  IxH  "), Some(Reg::Ixh));
        assert_eq!(Reg::parse("nonsense"), None);
        assert_eq!(Reg::parse(""), None);
    }

    /// `AF`'s low half is `F`, not nothing - this is what makes "overwriting
    /// `A` leaves the flags still live" expressible.
    #[test]
    fn pairs_and_halves_agree_with_each_other() {
        for pair in [Reg::Af, Reg::Bc, Reg::De, Reg::Hl, Reg::Ix, Reg::Iy] {
            let (high, low) = pair.halves().expect("decomposable");
            assert_eq!(high.pair(), Some(pair), "{high} should belong to {pair}");
            assert_eq!(low.pair(), Some(pair), "{low} should belong to {pair}");
            assert!(pair.is_pair());
        }
        assert_eq!(Reg::Af.halves(), Some((Reg::A, Reg::F)));
        // 16-bit, but no separately-addressable halves.
        assert_eq!(Reg::Sp.halves(), None);
        assert_eq!(Reg::Pc.halves(), None);
        assert!(Reg::Sp.is_pair());
        // ...and a plain 8-bit register is not a pair.
        assert!(!Reg::B.is_pair());
        assert_eq!(Reg::I.pair(), None);
    }

    #[test]
    fn conversions_from_the_assembler_register_types_agree_with_parsing() {
        assert_eq!(Reg::from(Register8::H), Reg::H);
        assert_eq!(Reg::from(Register16::Hl), Reg::Hl);
        assert_eq!(Reg::from(IndexRegister8::Ixl), Reg::Ixl);
        assert_eq!(Reg::from(IndexRegister16::Iy), Reg::Iy);
        // The conversions and the textual parser must not disagree.
        assert_eq!(Reg::from(Register16::Af), Reg::parse("AF").unwrap());
    }

    /// The whole point of the `dsl::ident` fix: `P/V` names one flag.
    #[test]
    fn every_flag_name_round_trips_and_the_slashed_one_parses_every_way() {
        for flag in Flag::ALL {
            assert_eq!(Flag::parse(flag.name()), Some(flag), "{flag}");
        }
        assert_eq!(Flag::parse("P/V"), Some(Flag::PV));
        assert_eq!(Flag::parse("p/v"), Some(Flag::PV));
        assert_eq!(Flag::parse("PV"), Some(Flag::PV));
        assert_eq!(Flag::parse("nonsense"), None);
    }
}

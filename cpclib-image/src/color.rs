//! What every colour of a palette must be able to do, whichever machine it is
//! for.
//!
//! [`Palette<C>`](crate::palette::Palette) is generic over this, and is
//! instantiated at [`Ink`] for the Gate Array and at [`AsicColor`] for the
//! Plus's ASIC. Code that has to work with either at runtime goes through
//! [`AnyPalette`](crate::palette::AnyPalette) rather than through a colour-level
//! enum - one runtime seam, at the container.

use crate::asic::AsicColor;
use crate::ink::Ink;

pub trait AmstradColor:
    Default
    + Copy
    + Clone
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + std::hash::Hash
    // Conversion parallelises over rows (`colors_to_pens`). Requiring it here
    // rather than on each generic function keeps the bound from propagating by
    // hand through every signature, and costs nothing: a colour is a small
    // `Copy` value.
    + Send
    + Sync
    + std::fmt::Debug
    + std::fmt::Display
    // A palette colour has to survive the round trip to a pixel and back:
    // reading a PNG produces `Rgb<u8>`, and rendering a preview needs one
    // again. Requiring it here rather than at each generic method is what lets
    // the whole conversion pipeline be written once.
    + From<image::Rgb<u8>>
    + Into<image::Rgb<u8>> {
    fn black() -> Self;
    fn white() -> Self;

    /// The colour standing for "background" in a sprite-with-mask image.
    fn mask_background() -> Self;
    /// The colour standing for "part of the sprite" in a sprite-with-mask image.
    fn mask_foreground() -> Self;
    /// The colour marking a pixel outside the mask entirely.
    fn not_in_mask() -> Self;
}

impl AmstradColor for Ink {
    fn mask_background() -> Self {
        Ink::BRIGHTWHITE
    }

    fn mask_foreground() -> Self {
        Ink::BLACK
    }

    fn not_in_mask() -> Self {
        Ink::RED
    }

    fn black() -> Self {
        Ink::BLACK
    }

    fn white() -> Self {
        Ink::BRIGHTWHITE
    }
}

/// The ASIC's answers are the Gate Array's, converted - the mask conventions
/// are about *which* colour means what, not about how many are available, so
/// they must agree between the two machines.
impl AmstradColor for AsicColor {
    fn mask_background() -> Self {
        Ink::mask_background().into()
    }

    fn mask_foreground() -> Self {
        Ink::mask_foreground().into()
    }

    fn not_in_mask() -> Self {
        Ink::not_in_mask().into()
    }

    fn black() -> Self {
        Ink::black().into()
    }

    fn white() -> Self {
        Ink::white().into()
    }
}

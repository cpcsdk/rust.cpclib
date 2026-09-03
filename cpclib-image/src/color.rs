//! What every colour of a palette must be able to do, whichever machine it is
//! for.
//!
//! [`Palette<C>`](crate::palette::Palette) is generic over this, and is
//! instantiated at [`Ink`] for the Gate Array and at [`AsicColor`] for the
//! Plus's ASIC. Code that has to work with either at runtime goes through
//! [`AnyPalette`](crate::palette::AnyPalette) rather than through a colour-level
//! enum - one runtime seam, at the container.

use crate::asic::AsicColor;
use crate::ga::INKS_RGB_VALUES;
use crate::ink::Ink;
use image as im;

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
    + Into<image::Rgb<u8>>
    // Every Amstrad colour space can name the 27 Gate Array inks - the Plus's
    // ASIC is a superset of them, not a replacement. Command-line options that
    // are spelled as an ink (`--mask-ink`) therefore mean something on both
    // machines, and generic code can say so without knowing which it is on.
    + From<Ink> {
    fn black() -> Self;
    fn white() -> Self;

    /// Wrap a palette of this colour into the runtime seam.
    ///
    /// This is the one place a monomorphised pipeline can hand back to code
    /// that has to know which machine it is talking to - writing the palette
    /// out, or emitting the Z80 that installs it. Everything else stays
    /// generic and never asks the question.
    fn into_any_palette(palette: crate::palette::Palette<Self>) -> crate::palette::AnyPalette;

    /// The colour standing for "background" in a sprite-with-mask image.
    fn mask_background() -> Self;
    /// The colour standing for "part of the sprite" in a sprite-with-mask image.
    fn mask_foreground() -> Self;
    /// The colour marking a pixel outside the mask entirely.
    fn not_in_mask() -> Self;

    /// Get the RGB color value of the ink
    fn color(&self) -> im::Rgb<u8>;

    fn owo_color(&self) -> owo_colors::DynColors {
        let color = self.color();
        owo_colors::DynColors::Rgb(color.0[0], color.0[1], color.0[2])
    }

    fn is_plus() -> bool;
}

impl AmstradColor for Ink {
    fn into_any_palette(palette: crate::palette::Palette<Self>) -> crate::palette::AnyPalette {
        crate::palette::AnyPalette::GateArray(palette)
    }

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


    /// Get the RGB color value of the ink
    fn color(&self) -> im::Rgb<u8> {
        INKS_RGB_VALUES[self.firmware_number() as usize]
    }

    fn is_plus() -> bool {
        false
    }

}

/// The ASIC's answers are the Gate Array's, converted - the mask conventions
/// are about *which* colour means what, not about how many are available, so
/// they must agree between the two machines.
impl AmstradColor for AsicColor {
    fn into_any_palette(palette: crate::palette::Palette<Self>) -> crate::palette::AnyPalette {
        crate::palette::AnyPalette::Asic(palette)
    }

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

    fn color(&self) -> im::Rgb<u8> {
        im::Rgb([
            self.get_red().value()<<4,
            self.get_green().value()<<4,
            self.get_blue().value()<<4,
        ])
    }

    fn is_plus() -> bool {
        true
    }
}


#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AnyColor {
    GateArray(Ink),
    Asic(AsicColor),
}

impl From<Ink> for AnyColor {
    fn from(ink: Ink) -> Self {
        AnyColor::GateArray(ink)
    }
}

impl From<AsicColor> for AnyColor {
    fn from(asic: AsicColor) -> Self {
        AnyColor::Asic(asic)
    }
}

impl TryFrom<AnyColor> for Ink {
    type Error = String;

    fn try_from(value: AnyColor) -> Result<Self, Self::Error> {
        match value {
            AnyColor::GateArray(ink) => Ok(ink),
            AnyColor::Asic(_) => Err("Cannot convert AsicColor to Ink".to_owned()),
        }
    }
}

impl TryFrom<AnyColor> for AsicColor {
    type Error = String;

    fn try_from(value: AnyColor) -> Result<Self, Self::Error> {
        match value {
            AnyColor::GateArray(ink) => Err(format!("Cannot convert Ink ({ink:?}) to AsicColor")),
            AnyColor::Asic(asic) => Ok(asic),
        }
    }
}

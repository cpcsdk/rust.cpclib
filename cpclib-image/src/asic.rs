//! Colours as the Amstrad Plus's ASIC stores them.
//!
//! Where the Gate Array offers 27 fixed [`Ink`]s, the ASIC stores a 12-bit RGB
//! value per pen - 4 bits per component, 4096 colours. A palette entry occupies
//! two bytes, laid out as
//!
//! ```text
//! byte 0:  RRRR BBBB
//! byte 1:  0000 GGGG
//! ```
//!
//! i.e. red at bits 12-15 of the little-endian `u16`, blue at 8-11, green at
//! 0-3, with bits 4-7 unused. This is the layout `.kit` palette files use (see
//! [`crate::kit`]).

use std::fmt::{Debug, Formatter, Result};

use image as im;

use crate::ink::{Ink, InkComponentQuantity};

/// One 4-bit colour component, 0 (off) to 15 (full).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AsicColorComponent(u8);

impl AsicColorComponent {
    /// The largest value a component can hold.
    pub const MAX: u8 = 0xF;

    pub fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for AsicColorComponent {
    /// Values wider than 4 bits are truncated: the hardware has nowhere to put
    /// them, and silently keeping the low nibble is what writing to the ASIC
    /// would do anyway.
    fn from(value: u8) -> Self {
        AsicColorComponent(value & Self::MAX)
    }
}

impl From<AsicColorComponent> for u8 {
    fn from(value: AsicColorComponent) -> Self {
        value.0
    }
}

impl From<InkComponentQuantity> for AsicColorComponent {
    /// The three levels a Gate Array ink can express, on the ASIC's 0-15 scale.
    fn from(value: InkComponentQuantity) -> Self {
        let value = match value {
            InkComponentQuantity::Zero => 0x0,
            InkComponentQuantity::Half => 0x6,
            InkComponentQuantity::Full => 0xF
        };
        AsicColorComponent(value)
    }
}

/// A 12-bit RGB colour in the ASIC's own packing - see the module comment.
///
/// Written by hand rather than with `bitfield!`: three nibbles at fixed offsets
/// is less code this way than the macro's conversion syntax, and it keeps the
/// packing visible right next to the layout it documents.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AsicColor(u16);

impl AsicColor {
    /// The bits the hardware actually uses - 4-7 are unused.
    pub const VALID_BITS: u16 = 0b1111_1111_0000_1111;

    pub fn new(
        red: impl Into<AsicColorComponent>,
        green: impl Into<AsicColorComponent>,
        blue: impl Into<AsicColorComponent>
    ) -> Self {
        let red = red.into().value() as u16;
        let green = green.into().value() as u16;
        let blue = blue.into().value() as u16;
        AsicColor((red << 12) | (blue << 8) | green)
    }

    pub fn get_red(self) -> AsicColorComponent {
        AsicColorComponent(((self.0 >> 12) & 0xF) as u8)
    }

    pub fn get_blue(self) -> AsicColorComponent {
        AsicColorComponent(((self.0 >> 8) & 0xF) as u8)
    }

    pub fn get_green(self) -> AsicColorComponent {
        AsicColorComponent((self.0 & 0xF) as u8)
    }

    pub fn set_red(&mut self, value: impl Into<AsicColorComponent>) {
        *self = Self::new(value, self.get_green(), self.get_blue());
    }

    pub fn set_green(&mut self, value: impl Into<AsicColorComponent>) {
        *self = Self::new(self.get_red(), value, self.get_blue());
    }

    pub fn set_blue(&mut self, value: impl Into<AsicColorComponent>) {
        *self = Self::new(self.get_red(), self.get_green(), value);
    }

    /// The packed value, as it is stored in a `.kit` file or written to the
    /// ASIC palette at `&6400`.
    pub fn value(self) -> u16 {
        self.0
    }

    /// The two bytes of a `.kit` entry, in file order.
    pub fn to_bytes(self) -> [u8; 2] {
        [(self.0 >> 8) as u8, (self.0 & 0xFF) as u8]
    }

    /// Read one `.kit` entry.
    pub fn from_bytes(bytes: [u8; 2]) -> Self {
        Self::from(((bytes[0] as u16) << 8) | (bytes[1] as u16))
    }
}

impl From<Ink> for AsicColor {
    fn from(ink: Ink) -> Self {
        AsicColor::new(
            AsicColorComponent::from(ink.red_quantity()),
            AsicColorComponent::from(ink.green_quantity()),
            AsicColorComponent::from(ink.blue_quantity())
        )
    }
}

impl From<im::Rgb<u8>> for AsicColor {
    /// Quantise a 24-bit pixel to the ASIC's 4 bits per component.
    ///
    /// Unlike [`Ink`]'s own conversion, which searches the 27 fixed hardware
    /// colours for the nearest one, this is exact arithmetic: the ASIC's space
    /// is a regular grid, so the closest representable value of an 8-bit
    /// component is simply the nearest sixteenth. Rounding (`+ 8`), not
    /// truncation - truncating biases every colour darker, which is visible
    /// across a whole image.
    fn from(color: im::Rgb<u8>) -> Self {
        fn quantise(component: u8) -> u8 {
            (((component as u16 * AsicColorComponent::MAX as u16) + 127) / 255) as u8
        }
        AsicColor::new(
            quantise(color[0]),
            quantise(color[1]),
            quantise(color[2])
        )
    }
}

impl From<AsicColor> for im::Rgb<u8> {
    /// The pixel an ASIC colour displays as - each component scaled back up so
    /// that 0 stays 0 and 15 becomes 255.
    fn from(color: AsicColor) -> Self {
        fn expand(component: AsicColorComponent) -> u8 {
            ((component.value() as u16 * 255) / AsicColorComponent::MAX as u16) as u8
        }
        im::Rgb([
            expand(color.get_red()),
            expand(color.get_green()),
            expand(color.get_blue())
        ])
    }
}

impl From<u16> for AsicColor {
    fn from(value: u16) -> Self {
        AsicColor(value & Self::VALID_BITS)
    }
}

impl std::fmt::Display for AsicColor {
    /// The `#RGB` form a Plus user writes in a palette editor - three hex
    /// nibbles, red first.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "#{:X}{:X}{:X}",
            self.get_red().value(),
            self.get_green().value(),
            self.get_blue().value()
        )
    }
}

impl Debug for AsicColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "AsicColor({:X}, {:X}, {:X})",
            self.get_red().value(),
            self.get_green().value(),
            self.get_blue().value()
        )
    }
}

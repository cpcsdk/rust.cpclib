use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result};
use std::ops::{Add, Deref};

use cpclib_common::itertools::Itertools;
use cpclib_common::num::Integer;
use image as im;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use self::im::Pixel;
use crate::image::Mode;

const INK0: im::Rgba<u8> = im::Rgba([0, 0, 0, 255]);
const INK1: im::Rgba<u8> = im::Rgba([0x00, 0x00, 0x80, 255]);
const INK2: im::Rgba<u8> = im::Rgba([0x00, 0x00, 0xFF, 255]);
const INK3: im::Rgba<u8> = im::Rgba([0x80, 0x00, 0x00, 255]);
const INK4: im::Rgba<u8> = im::Rgba([0x80, 0x00, 0x80, 255]);
const INK5: im::Rgba<u8> = im::Rgba([0x80, 0x00, 0xFF, 255]);
const INK6: im::Rgba<u8> = im::Rgba([0xFF, 0x00, 0x00, 255]);
const INK7: im::Rgba<u8> = im::Rgba([0xFF, 0x00, 0x80, 255]);
const INK8: im::Rgba<u8> = im::Rgba([0xFF, 0x00, 0xFF, 255]);
const INK9: im::Rgba<u8> = im::Rgba([0x00, 0x80, 0x00, 255]);
const INK10: im::Rgba<u8> = im::Rgba([0x00, 0x80, 0x80, 255]);
const INK11: im::Rgba<u8> = im::Rgba([0x00, 0x80, 0xFF, 255]);
const INK12: im::Rgba<u8> = im::Rgba([0x80, 0x80, 0x00, 255]);
const INK13: im::Rgba<u8> = im::Rgba([0x80, 0x80, 0x80, 255]);
const INK14: im::Rgba<u8> = im::Rgba([0x80, 0x80, 0xFF, 255]);
const INK15: im::Rgba<u8> = im::Rgba([0xFF, 0x80, 0x00, 255]);
const INK16: im::Rgba<u8> = im::Rgba([0xFF, 0x80, 0x80, 255]);
const INK17: im::Rgba<u8> = im::Rgba([0xFF, 0x80, 0xFF, 255]);
const INK18: im::Rgba<u8> = im::Rgba([0x00, 0xFF, 0x00, 255]);
const INK19: im::Rgba<u8> = im::Rgba([0x00, 0xFF, 0x80, 255]);
const INK20: im::Rgba<u8> = im::Rgba([0x00, 0xFF, 0xFF, 255]);
const INK21: im::Rgba<u8> = im::Rgba([0x80, 0xFF, 0x00, 255]);
const INK22: im::Rgba<u8> = im::Rgba([0x80, 0xFF, 0x80, 255]);
const INK23: im::Rgba<u8> = im::Rgba([0x80, 0xFF, 0xFF, 255]);
const INK24: im::Rgba<u8> = im::Rgba([0xFF, 0xFF, 0x00, 255]);
const INK25: im::Rgba<u8> = im::Rgba([0xFF, 0xFF, 0x80, 255]);
const INK26: im::Rgba<u8> = im::Rgba([0xFF, 0xFF, 0xFF, 255]);

/// Number of inks managed by the system. Do not take into account the few duplicates
const NB_INKS: u8 = 27;

/// Number of pens, including the border
const NB_PENS: u8 = 16 + 1;

/// RGB color for each ink
pub const INKS_RGB_VALUES: [im::Rgba<u8>; 27] = [
    INK0, INK1, INK2, INK3, INK4, INK5, INK6, INK7, INK8, INK9, INK10, INK11, INK12, INK13, INK14,
    INK15, INK16, INK17, INK18, INK19, INK20, INK21, INK22, INK23, INK24, INK25, INK26
];

/// Ga value for each ink
pub const INKS_GA_VALUE: [u8; 27] = [
    0x54, // 0
    0x44, // 1
    0x55, // 2
    0x5C, // 3
    0x58, // 4
    0x5D, // 5
    0x4C, // 6
    0x45, // 7
    0x4D, // 8
    0x56, // 9
    0x46, // 10
    0x57, // 11
    0x5E, // 12
    0x40, // 13
    0x5F, // 14
    0x4E, // 15
    0x47, // 16
    0x4F, // 17
    0x52, // 18
    0x42, // 19
    0x53, // 20
    0x5A, // 21
    0x59, // 22
    0x5B, // 23
    0x4A, // 24
    0x43, // 25
    0x4B  // 26
];

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
/// Amstrad INK
pub struct Ink {
    /// Ink value
    value: u8
}

/// Describes the quantity for a given component
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InkComponentQuantity {
    /// 0%
    Zero,
    /// 50%
    Half,
    /// 100%
    Full
}

impl Display for InkComponentQuantity {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let code = match self {
            InkComponentQuantity::Zero => 0,
            InkComponentQuantity::Half => 127,
            InkComponentQuantity::Full => 255
        };

        write!(f, "{code:3}")
    }
}

impl InkComponentQuantity {
    /// Build a lower quantity
    pub fn decrease(&self) -> InkComponentQuantity {
        match self {
            Self::Zero => Self::Zero,
            Self::Half => Self::Zero,
            Self::Full => Self::Half
        }
    }

    pub fn increase(&self) -> InkComponentQuantity {
        match self {
            Self::Zero => Self::Half,
            Self::Half => Self::Full,
            Self::Full => Self::Full
        }
    }
}

impl From<(InkComponent, InkComponentQuantity)> for Ink {
    fn from(value: (InkComponent, InkComponentQuantity)) -> Self {
        match value.0 {
            InkComponent::Red => {
                (
                    value.1,
                    InkComponentQuantity::Zero,
                    InkComponentQuantity::Zero
                )
                    .into()
            },
            InkComponent::Green => {
                (
                    InkComponentQuantity::Zero,
                    value.1,
                    InkComponentQuantity::Zero
                )
                    .into()
            },
            InkComponent::Blue => {
                (
                    InkComponentQuantity::Zero,
                    InkComponentQuantity::Zero,
                    value.1
                )
                    .into()
            },
        }
    }
}

/// Build an ink from its RGB components
impl
    From<(
        InkComponentQuantity,
        InkComponentQuantity,
        InkComponentQuantity
    )> for Ink
{
    fn from(
        d: (
            InkComponentQuantity,
            InkComponentQuantity,
            InkComponentQuantity
        )
    ) -> Self {
        let value = match d {
            //   R           G           B
            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Zero
            ) => 0,
            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Half
            ) => 1,
            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Full
            ) => 2,

            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Half,
                InkComponentQuantity::Zero
            ) => 9,
            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Half,
                InkComponentQuantity::Half
            ) => 10,
            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Half,
                InkComponentQuantity::Full
            ) => 11,

            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Full,
                InkComponentQuantity::Zero
            ) => 18,
            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Full,
                InkComponentQuantity::Half
            ) => 19,
            (
                InkComponentQuantity::Zero,
                InkComponentQuantity::Full,
                InkComponentQuantity::Full
            ) => 20,

            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Zero
            ) => 3,
            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Half
            ) => 4,
            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Full
            ) => 5,

            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Half,
                InkComponentQuantity::Zero
            ) => 12,
            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Half,
                InkComponentQuantity::Half
            ) => 13,
            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Half,
                InkComponentQuantity::Full
            ) => 14,

            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Full,
                InkComponentQuantity::Zero
            ) => 21,
            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Full,
                InkComponentQuantity::Half
            ) => 22,
            (
                InkComponentQuantity::Half,
                InkComponentQuantity::Full,
                InkComponentQuantity::Full
            ) => 23,

            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Zero
            ) => 6,
            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Half
            ) => 7,
            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Zero,
                InkComponentQuantity::Full
            ) => 8,

            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Half,
                InkComponentQuantity::Zero
            ) => 15,
            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Half,
                InkComponentQuantity::Half
            ) => 16,
            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Half,
                InkComponentQuantity::Full
            ) => 17,

            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Full,
                InkComponentQuantity::Zero
            ) => 24,
            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Full,
                InkComponentQuantity::Half
            ) => 25,
            (
                InkComponentQuantity::Full,
                InkComponentQuantity::Full,
                InkComponentQuantity::Full
            ) => 26
        };
        value.into()
    }
}

/// Represents one of the 3 components of an Ink
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InkComponent {
    /// Red component
    Red,
    /// Green component
    Green,
    /// Blue component
    Blue
}

impl Display for InkComponent {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let code = match self {
            InkComponent::Red => "red",
            InkComponent::Green => "green",
            InkComponent::Blue => "blue"
        };

        write!(f, "{code}")
    }
}

#[allow(missing_docs)]
impl Ink {
    /// Get the RGB color value of the ink
    pub fn color(self) -> im::Rgba<u8> {
        INKS_RGB_VALUES[self.value as usize]
    }

    /// Give the quantity of red for the given color
    /// <http://cpc.sylvestre.org/technique/technique_coul1.html>
    pub fn red_quantity(&self) -> InkComponentQuantity {
        match self.value {
            0 | 1 | 2 | 9 | 10 | 11 | 18 | 19 | 20 => InkComponentQuantity::Zero,
            3 | 4 | 5 | 12 | 13 | 14 | 21 | 22 | 23 => InkComponentQuantity::Half,
            6 | 7 | 8 | 15 | 16 | 17 | 24 | 25 | 26 => InkComponentQuantity::Full,
            _ => unreachable!()
        }
    }

    /// Give the quantit of blue for the given ink
    /// <http://cpc.sylvestre.org/technique/technique_coul1.html>
    pub fn blue_quantity(&self) -> InkComponentQuantity {
        match self.value {
            0 | 3 | 6 | 9 | 12 | 15 | 18 | 21 | 24 => InkComponentQuantity::Zero,
            1 | 4 | 7 | 10 | 13 | 16 | 19 | 22 | 25 => InkComponentQuantity::Half,
            2 | 5 | 8 | 11 | 14 | 17 | 20 | 23 | 26 => InkComponentQuantity::Full,
            _ => unreachable!()
        }
    }

    /// Give the quantity of green for the given ink
    /// <http://cpc.sylvestre.org/technique/technique_coul1.html>
    pub fn green_quantity(&self) -> InkComponentQuantity {
        match self.value {
            0..=8 => InkComponentQuantity::Zero,
            9..=17 => InkComponentQuantity::Half,
            18..=26 => InkComponentQuantity::Full,
            _ => unreachable!()
        }
    }

    /// Returns the quantity for the required color component
    pub fn component_quantity(&self, comp: InkComponent) -> InkComponentQuantity {
        match comp {
            InkComponent::Red => self.red_quantity(),
            InkComponent::Green => self.green_quantity(),
            InkComponent::Blue => self.blue_quantity()
        }
    }

    /// Decrease the component of interest
    pub fn decrease_component(&mut self, comp: InkComponent) -> &mut Self {
        let (mut r, mut g, mut b) = (
            self.red_quantity(),
            self.green_quantity(),
            self.blue_quantity()
        );
        match comp {
            InkComponent::Red => r = r.decrease(),
            InkComponent::Green => g = g.decrease(),
            InkComponent::Blue => b = b.decrease()
        };
        let new_ink: Ink = (r, g, b).into();

        self.value = new_ink.value;
        self
    }

    /// Increase the component of interest
    pub fn increase_component(&mut self, comp: InkComponent) -> &mut Self {
        let (mut r, mut g, mut b) = (
            self.red_quantity(),
            self.green_quantity(),
            self.blue_quantity()
        );
        match comp {
            InkComponent::Red => r = r.increase(),
            InkComponent::Green => g = g.increase(),
            InkComponent::Blue => b = b.increase()
        };
        let new_ink: Ink = (r, g, b).into();

        self.value = new_ink.value;
        self
    }

    /// Get the ink number (firmware wise)
    pub fn number(self) -> u8 {
        self.value
    }

    /// Get the value required by the gate array the select the ink
    pub fn gate_array(self) -> u8 {
        INKS_GA_VALUE[self.value as usize]
    }

    pub fn from_gate_array_color_number(col: u8) -> Ink {
        let idx = INKS_GA_VALUE.iter().position(|i| *i == col).unwrap();
        INKS[idx]
    }

    pub fn from_hardware_color_number(col: u8) -> Ink {
        match col {
            20 => 0,
            4 => 1,
            21 => 2,
            28 => 3,
            24 => 4,
            29 => 5,
            12 => 6,
            5 => 7,
            13 => 8,
            22 => 9,
            6 => 10,
            23 => 11,
            30 => 12,
            0 => 13,
            31 => 14,
            14 => 15,
            7 => 16,
            15 => 17,
            18 => 18,
            2 => 19,
            19 => 20,
            26 => 21,
            25 => 22,
            27 => 23,
            10 => 24,
            3 => 25,
            11 => 26,
            _ => panic!("{col} bad value")
        }
        .into()
    }
}

impl From<Ink> for u8 {
    fn from(val: Ink) -> Self {
        (&val).into()
    }
}

impl From<&Ink> for u8 {
    fn from(val: &Ink) -> Self {
        val.gate_array()
    }
}

impl From<im::Rgba<u8>> for Ink {
    fn from(color: im::Rgba<u8>) -> Self {
        Self::from(color.to_rgb())
    }
}

impl From<im::Rgb<u8>> for Ink {
    /// Convert an rgb value to the corresponding ink.
    /// The closest color is provided if the provided color is not strictly corresponding to a CPC color.
    fn from(color: im::Rgb<u8>) -> Self {
        // Not strict comparison
        let distances = INKS_RGB_VALUES
            .iter()
            .map(|color_ink| {
                (i32::from(color_ink[0]) - i32::from(color[0])).pow(2)
                    + (i32::from(color_ink[1]) - i32::from(color[1])).pow(2)
                    + (i32::from(color_ink[2]) - i32::from(color[2])).pow(2)
            })
            .collect::<Vec<_>>();
        let mut selected_idx = 0;
        let mut smallest = distances[0];
        for (idx, &distance) in distances.iter().enumerate().skip(1) {
            if smallest > distance {
                smallest = distance;
                selected_idx = idx;
            }
        }
        Self::from(selected_idx as u8)
    }
}

macro_rules! impl_from_ink_integer {
    ( $($t: ty),* ) => {
      $(  impl From<$t> for Ink {
            fn from(item: $t) -> Self {
                assert!(item < 32);
                Self { value: item as _}
            }
        } )*
    }
}

impl_from_ink_integer! {u8, u16, u32, u64, i8, i16, i32, i64}

impl From<String> for Ink {
    fn from(item: String) -> Self {
        match item.to_uppercase().replace([' ', '_'], "").as_str() {
            "BLACK" => Self::BLACK,
            "BLUE" => Self::BLUE,
            "BRIGHTBLUE" => Self::BRIGHTBLUE,
            "RED" => Self::RED,
            "MAGENTA" => Self::MAGENTA,
            "MAUVE" => Self::MAUVE,
            "BRIGHTRED" => Self::BRIGHTRED,
            "PURPLE" => Self::PURPLE,
            "BRIGHTMAGENTA" => Self::BRIGHTMAGENTA,
            "GREEN" => Self::GREEN,
            "CYAN" => Self::CYAN,
            "SKYBLUE" => Self::SKYBLUE,
            "YELLOW" => Self::YELLOW,
            "WHITE" => Self::WHITE,
            "PASTELBLUE" => Self::PASTELBLUE,
            "ORANGE" => Self::ORANGE,
            "PINK" => Self::PINK,
            "PASTELMAGENTA" => Self::PASTELMAGENTA,
            "BRIGHTGREEN" => Self::BRIGHTGREEN,
            "SEAGREEN" => Self::SEAGREEN,
            "BRIGHTCYAN" => Self::BRIGHTCYAN,
            "LIME" => Self::LIME,
            "PASTELGREEN" => Self::PASTELGREEN,
            "PASTELCYAN" => Self::PASTELCYAN,
            "BRIGHTYELLOW" => Self::BRIGHTYELLOW,
            "PASTELYELLOW" => Self::PASTELYELLOW,
            "BRIGHTWHITE" => Self::BRIGHTWHITE,

            "GRAY" | "GREY" => "WHITE".into(),

            _ => panic!("{item} color does not exist")
        }
    }
}

impl Debug for Ink {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{} ({})", self, self.value)
    }
}

impl Display for Ink {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let repr = match self.value {
            0 => "BLACK",
            1 => "BLUE",
            2 => "BRIGHTBLUE",
            3 => "RED",
            4 => "MAGENTA",
            5 => "MAUVE",
            6 => "BRIGHTRED",
            7 => "PURPLE",
            8 => "BRIGHTMAGENTA",
            9 => "GREEN",
            10 => "CYAN",
            11 => "SKYBLUE",
            12 => "YELLOW",
            13 => "WHITE",
            14 => "PASTELBLUE",
            15 => "ORANGE",
            16 => "PINK",
            17 => "PASTELMAGENTA",
            18 => "BRIGHTGREEN",
            19 => "SEAGREEN",
            20 => "BRIGHTCYAN",
            21 => "LIME",
            22 => "PASTELGREEN",
            23 => "PASTELCYAN",
            24 => "BRIGHTYELLOW",
            25 => "PASTELYELLOW",
            26 => "BRIGHTWHITE",
            _ => panic!()
        };

        write!(f, "{repr}")
    }
}

impl<'a> From<&'a str> for Ink {
    fn from(item: &'a str) -> Self {
        Self::from(String::from(item))
    }
}

/// Available inks
pub const INKS: [Ink; NB_INKS as usize] = [
    Ink { value: 0 },
    Ink { value: 1 },
    Ink { value: 2 },
    Ink { value: 3 },
    Ink { value: 4 },
    Ink { value: 5 },
    Ink { value: 6 },
    Ink { value: 7 },
    Ink { value: 8 },
    Ink { value: 9 },
    Ink { value: 10 },
    Ink { value: 11 },
    Ink { value: 12 },
    Ink { value: 13 },
    Ink { value: 14 },
    Ink { value: 15 },
    Ink { value: 16 },
    Ink { value: 17 },
    Ink { value: 18 },
    Ink { value: 19 },
    Ink { value: 20 },
    Ink { value: 21 },
    Ink { value: 22 },
    Ink { value: 23 },
    Ink { value: 24 },
    Ink { value: 25 },
    Ink { value: 26 }
];

impl Ink {
    pub const BLACK: Ink = Self { value: 0 };
    pub const BLUE: Ink = Self { value: 1 };
    pub const BRIGHTBLUE: Ink = Self { value: 2 };
    pub const BRIGHTCYAN: Ink = Self { value: 20 };
    pub const BRIGHTGREEN: Ink = Self { value: 18 };
    pub const BRIGHTMAGENTA: Ink = Self { value: 8 };
    pub const BRIGHTRED: Ink = Self { value: 6 };
    pub const BRIGHTWHITE: Ink = Self { value: 26 };
    pub const BRIGHTYELLOW: Ink = Self { value: 24 };
    pub const BRIGHT_BLUE: Ink = Self::BRIGHTBLUE;
    pub const BRIGHT_CYAN: Ink = Self::BRIGHTCYAN;
    pub const BRIGHT_GREEN: Ink = Self::BRIGHTGREEN;
    pub const BRIGHT_RED: Ink = Self::BRIGHTRED;
    pub const BRIGHT_WHITE: Ink = Self::BRIGHTWHITE;
    pub const BRIGHT_YELLOW: Ink = Self::BRIGHTYELLOW;
    pub const CYAN: Ink = Self { value: 10 };
    pub const GREEN: Ink = Self { value: 9 };
    pub const LIME: Ink = Self { value: 21 };
    pub const MAGENTA: Ink = Self { value: 4 };
    pub const MAUVE: Ink = Self { value: 5 };
    pub const ORANGE: Ink = Self { value: 15 };
    pub const PASTELBLUE: Ink = Self { value: 14 };
    pub const PASTELCYAN: Ink = Self { value: 23 };
    pub const PASTELGREEN: Ink = Self { value: 22 };
    pub const PASTELMAGENTA: Ink = Self { value: 17 };
    pub const PASTELYELLOW: Ink = Self { value: 25 };
    pub const PASTEL_BLUE: Ink = Self::PASTELBLUE;
    pub const PASTEL_CYAN: Ink = Self::PASTELCYAN;
    pub const PASTEL_GREEN: Ink = Self::PASTELGREEN;
    pub const PASTEL_MAGENTA: Ink = Self::PASTELMAGENTA;
    pub const PASTEL_YELLOW: Ink = Self::PASTELYELLOW;
    pub const PINK: Ink = Self { value: 16 };
    pub const PURPLE: Ink = Self { value: 7 };
    pub const RED: Ink = Self { value: 3 };
    pub const SEAGREEN: Ink = Self { value: 19 };
    pub const SEA_GREEN: Ink = Self::SEAGREEN;
    pub const SKYBLUE: Ink = Self { value: 11 };
    pub const SKY_BLUE: Ink = Self::SKYBLUE;
    pub const WHITE: Ink = Self { value: 13 };
    pub const YELLOW: Ink = Self { value: 12 };
}

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug, Serialize, Deserialize, PartialOrd, Ord)]
/// Represents a Pen. There a 16 pens + the border in the Amstrad CPC
pub struct Pen {
    /// pen value
    value: u8
}

// Constructor of Pen from an integer
impl<T: Integer> From<T> for Pen
where i32: From<T>
{
    fn from(item: T) -> Self {
        let item: i32 = item.into();
        Self { value: item as u8 }
    }
}

// Constructor of Pen reference from an integer
impl<T: Integer> From<T> for &Pen
where i32: From<T>
{
    fn from(item: T) -> Self {
        let pos: i32 = item.into();
        &PENS[pos as usize]
    }
}

#[allow(missing_docs)]
impl Pen {
    /// Get the number of the pen
    pub fn number(self) -> u8 {
        self.value
    }

    /// Change the value of the pen in order to not exceed the number of pens available in the
    /// given mode
    pub fn limit(&mut self, mode: Mode) {
        self.value = match mode {
            Mode::Zero => self.value,
            Mode::Three | Mode::One => self.value & 3,
            Mode::Two => self.value & 1
        };
    }
}

impl Add<i8> for Pen {
    type Output = Self;

    fn add(self, delta: i8) -> Self {
        Self {
            value: (self.value as i8 + delta) as u8
        }
    }
}

/// Available pens
pub const PENS: [Pen; NB_PENS as usize] = [
    Pen { value: 0 },
    Pen { value: 1 },
    Pen { value: 2 },
    Pen { value: 3 },
    Pen { value: 4 },
    Pen { value: 5 },
    Pen { value: 6 },
    Pen { value: 7 },
    Pen { value: 8 },
    Pen { value: 9 },
    Pen { value: 10 },
    Pen { value: 11 },
    Pen { value: 12 },
    Pen { value: 13 },
    Pen { value: 14 },
    Pen { value: 15 },
    Pen { value: 16 } // Border
];

/// The palette maps one Ink for each Pen
pub struct Palette {
    /// Values for the palette. Some items may be absent
    values: HashMap<Pen, Ink>
}

impl Clone for Palette {
    fn clone(&self) -> Self {
        let mut map: HashMap<Pen, Ink> = HashMap::new();
        for (pen, ink) in &self.values {
            map.insert(*pen, *ink);
        }

        Self { values: map }
    }
}

impl Debug for Palette {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for i in 0..16 {
            writeln!(f, "{} => {:?}", i, self.values.get(&Pen::from(i)))?;
        }
        Ok(())
    }
}

impl Default for Palette {
    /// Create a new palette.
    /// Pens ink are the same than Amsdos ones.
    fn default() -> Self {
        let mut p = Self::new();
        for i in 0..15 {
            p.set(PENS[i], INKS[i]);
        }
        p
    }
}

// /
// impl<T> From<Vec<T>> for Palette
// where
// Ink: From<T>,
// T: Copy
// {
// fn from(items: Vec<T>) -> Self {
// let mut p = Self::new();
//
// for (idx, ink) in items.iter().enumerate() {
// p.set(Pen::from(idx as u8), Ink::from(*ink));
// }
//
// p
// }
// }

impl<S, T> From<S> for Palette
where
    S: IntoIterator<Item = T>,
    Ink: From<T>
{
    fn from(items: S) -> Self {
        let mut p = Self::empty();
        let items = items.into_iter();

        for (idx, ink) in items.enumerate().take(16 + 1) {
            p.set(Pen::from(idx as u8), Ink::from(ink));
        }

        p
    }
}

// impl<T> From<[T; 16]> for Palette
// where
// Ink: From<T>,
// T: Copy
// {
// fn from(items: [T; 16]) -> Self {
// items.to_vec().into()
// }
// }

/// Create a palette with the right inks
/// Usage
/// `palette![1, 2, 3]`
#[macro_export]
macro_rules! palette {
    ( $( $x:expr_2021 ),* ) => {
        {
            use cpclib_image as cpc;
            use cpc::ga;
            use cpc::ga::Ink;
            use cpc::ga::Pen;

            let mut palette = ga::Palette::default();
            let mut idx = 0;

            $(
                let pen = Pen::from(idx);
                let ink = Ink::from($x);
                palette.set(pen, ink);
                idx += 1;
            )*

            // Ensure the other inks are black
            for i in idx..15 {
                palette.set(Pen::from(i), Ink::from(0));
            }
            palette
        }
    };
}

impl Serialize for Palette {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(17))?;
        for i in 0..17 {
            let entry = self.get(i.into());
            seq.serialize_element(entry)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Palette {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let inks: Vec<Ink> = Vec::<Ink>::deserialize(deserializer)?;
        let palette: Self = inks.into_iter().into();
        Ok(palette)
    }
}

impl<P> std::ops::Index<P> for Palette
where P: Into<Pen>
{
    type Output = Ink;

    fn index(&self, p: P) -> &Self::Output {
        self.get(&p.into())
    }
}

#[allow(missing_docs)]
impl Palette {
    /// Create a new palette.
    /// All pens are black.
    pub fn new() -> Self {
        let mut map: HashMap<Pen, Ink> = HashMap::new();

        for pen in 0..NB_PENS {
            map.insert(Pen { value: pen }, Ink { value: 0 });
        }

        Self { values: map }
    }

    /// Create an empty Palette.
    /// An empty palette does not contains all the inks and must make crash most of the code that has been previously written !
    pub fn empty() -> Self {
        Self {
            values: HashMap::<Pen, Ink>::default()
        }
    }

    /// Returns true if all standard inks are different
    pub fn nb_different_inks(&self) -> usize {
        use std::collections::HashSet;
        let mut set = HashSet::<Ink>::default();
        for pen in 0..16 {
            set.insert(*self.get(&pen.into()));
        }

        set.len()
    }

    /// Verifies if the palette contains the required pen
    pub fn contains_pen(&self, pen: Pen) -> bool {
        self.values.contains_key(&pen)
    }

    pub fn contains_border(&self) -> bool {
        self.contains_pen(Pen::from(16))
    }

    /// Provides the next unused pen if there is one. a 16 palette mode is considered
    pub fn next_unused_pen(&self) -> Option<Pen> {
        self.next_unused_pen_for_mode(Mode::Zero)
    }

    /// Provides the next unused pen, if there is one, for the requested mode
    pub fn next_unused_pen_for_mode(&self, mode: Mode) -> Option<Pen> {
        for i in 0..(mode.max_colors() as i32) {
            let pen = Pen::from(i);
            if !self.contains_pen(pen) {
                return Some(pen);
            }
        }
        None
    }

    /// Returns an array of gate array values
    /// Crash when pen is not set up
    /// TODO Return an option
    pub fn to_gate_array(&self) -> [u8; NB_PENS as usize] {
        let mut res = [0; NB_PENS as usize];
        for pen in 0..NB_PENS {
            res[pen as usize] = self.get(&pen.into()).gate_array();
        }
        res
    }

    pub fn to_gate_array_with_default(&self, default: Ink) -> [u8; NB_PENS as usize] {
        let mut res = [0; NB_PENS as usize];
        for pen in 0..NB_PENS {
            res[pen as usize] = self.get_with_default(&pen.into(), &default).gate_array();
        }
        res
    }

    /// Add the inks if not present in empty slots of the palette as soon as it is possible. Returns the number of inks added a,d the number of inks impossible to add because of the lack of space.
    pub fn add_novel_inks_except_in_border(&mut self, inks: &[Ink]) -> (usize, usize) {
        let counter_added = 0;
        let mut counter_impossible = 0;

        for ink in inks {
            // skip if already present
            if self.contains_ink(*ink) {
                continue;
            }

            match self.next_unused_pen() {
                None => counter_impossible += 1,
                Some(pen) => {
                    self.set(pen, *ink);
                }
            }
        }

        (counter_added, counter_impossible)
    }

    /// Returns the list of inks contained in the palette with the border
    /// the number of inks corresponds to the number of available pens
    pub fn inks_with_border(&self) -> Vec<Ink> {
        let mut vec = Vec::with_capacity(17);
        for pen in 0..17 {
            let pen = Pen::from(pen);
            if self.contains_pen(pen) {
                vec.push(*self.get(&pen));
            }
        }
        vec
    }

    /// Returns the list of inks contained in the palette without taking into account the border
    /// the number of inks corresponds to the number of available pens
    pub fn inks(&self) -> Vec<Ink> {
        let mut vec = Vec::with_capacity(16);
        for pen in 0..16 {
            let pen = Pen::from(pen);
            if self.contains_pen(pen) {
                vec.push(*self.get(&pen));
            }
        }
        vec
    }

    /// Returns all the set pens (without the border)
    pub fn pens_with_border(&self) -> Vec<Pen> {
        self.values.keys().copied().collect::<Vec<Pen>>()
    }

    /// Returns all the set pens (without the border)
    pub fn pens(&self) -> Vec<Pen> {
        self.values
            .iter()
            .sorted_by(|a, b| Ord::cmp(&a.0.number(), &b.0.number()))
            .filter_map(|(&p, _)| if p.number() == 16 { None } else { Some(p) })
            .collect::<Vec<Pen>>()
    }

    /// Get the ink of the requested pen. Pen MUST be present
    pub fn get(&self, pen: &Pen) -> &Ink {
        match self.values.get(pen) {
            Some(ink) => ink,
            None => panic!("Wrong pen {pen:?}")
        }
    }

    pub fn safe_get(&self, pen: &Pen) -> Option<&Ink> {
        self.values.get(pen)
    }

    pub fn get_with_default<'a>(&'a self, pen: &'a Pen, default: &'a Ink) -> &'a Ink {
        match self.values.get(pen) {
            Some(ink) => ink,
            None => default
        }
    }

    // Get the ink of the border
    pub fn get_border(&self) -> &Ink {
        self.values.get(&Pen::from(16)).expect("Border unavailable")
    }

    /// Change the ink of the specified pen
    pub fn set<P: Into<Pen>, I: Into<Ink>>(&mut self, pen: P, ink: I) {
        self.values.insert(pen.into(), ink.into());
    }

    pub fn set_border(&mut self, ink: Ink) {
        self.values.insert(Pen::from(16), ink);
    }

    /// Get the pen that corresponds to the required ink.
    /// Ink 16 (border) is never tested
    pub fn get_pen_for_ink<I: Into<Ink>>(&self, expected: I) -> Option<Pen> {
        let ink: Ink = expected.into();
        self.values
            .iter()
            .filter(|&(&p, _)| p.number() != 16)
            .filter(|&(&p_, &i)| i == ink)
            .min()
            .map(|(p, _i)| *p)
    }

    /// Returns true if the palette contains the inks in one of its pens (except border)
    pub fn contains_ink(&self, expected: Ink) -> bool {
        self.get_pen_for_ink(expected).is_some()
    }

    /// Replicate the firsts 4 pens in order to manage special texture that contains both mode 0
    /// and mode 3 patterns
    pub fn to_mode3_mixed_with_mode0(&self) -> Self {
        let mut p = self.clone();

        let ink0 = self.get(&PENS[0]);
        let ink1 = self.get(&PENS[1]);
        let ink2 = self.get(&PENS[2]);
        let ink3 = self.get(&PENS[3]);

        p.set(PENS[4], *ink3);
        p.set(PENS[5], *ink0);
        p.set(PENS[6], *ink0);
        p.set(PENS[7], *ink0);
        p.set(PENS[8], *ink1);
        p.set(PENS[9], *ink3);
        p.set(PENS[10], *ink1);
        p.set(PENS[11], *ink1);
        p.set(PENS[12], *ink2);
        p.set(PENS[13], *ink2);
        p.set(PENS[14], *ink3);
        p.set(PENS[15], *ink2);

        p
    }

    /// Decrease all the values of a given component
    pub fn decrease_component(&mut self, c: InkComponent) {
        self.values.iter_mut().for_each(|(_p, i)| {
            i.decrease_component(c);
        });
    }

    /// Generate the list of palette needed to obtain an RGB fadout.
    /// The current palette is included in the list of palette
    /// <http://cpc.sylvestre.org/technique/technique_coul5.html>
    pub fn rgb_fadout(&self) -> Vec<Palette> {
        // Check if we can still decrease the components
        let is_finished = |p: &Palette, c: InkComponent| {
            p.inks()
                .iter()
                .all(|ink| ink.component_quantity(c) == InkComponentQuantity::Zero)
        };

        // Decrease a given component
        let decrease_component = |p: &Palette, c: InkComponent| {
            let mut decreased_palettes = Vec::new();

            loop {
                let current = match decreased_palettes.last() {
                    Some(palette) => palette,
                    None => p
                };
                if is_finished(current, c) {
                    break;
                }

                let mut new_palette = current.clone();
                new_palette.decrease_component(c);
                decreased_palettes.push(new_palette);
            }
            decreased_palettes
        };

        // Progressively decrease the components
        let mut palettes = Vec::new();
        for component in [InkComponent::Green, InkComponent::Red, InkComponent::Blue].iter() {
            //  println!("Decrease for {:?}", &component);
            let current = match palettes.last() {
                Some(palette) => palette,
                None => self
            };
            let new_palettes = decrease_component(current, *component);
            palettes.extend_from_slice(&new_palettes);
        }

        palettes
    }

    pub fn nb_pens_used(&self) -> usize {
        self.values.len()
    }
}

impl From<&Palette> for Vec<u8> {
    fn from(val: &Palette) -> Self {
        let mut vec = Vec::with_capacity(16);
        for pen in 0..17 {
            let pen = Pen::from(pen);
            if val.contains_pen(pen) {
                vec.push(val.get(&pen).into());
            }
            else {
                vec.push(0x54); // No pens => ink black
            }
        }
        vec
    }
}

impl From<Palette> for Vec<u8> {
    fn from(val: Palette) -> Self {
        (&val).into()
    }
}

#[allow(missing_docs)]
impl Palette {
    pub fn to_vec(&self) -> Vec<u8> {
        self.into()
    }
}

/// Represents a palette that can be read-only by construction or updatable
#[derive(Clone, Debug)]
pub struct LockablePalette {
    pal: Palette,
    locked: bool
}

impl Into<Palette> for LockablePalette {
    fn into(self) -> Palette {
        self.pal
    }
}

impl Into<Palette> for &LockablePalette {
    fn into(self) -> Palette {
        self.pal.clone()
    }
}

impl Deref for LockablePalette {
    type Target = Palette;

    fn deref(&self) -> &Self::Target {
        self.as_palette()
    }
}

impl LockablePalette {
    /// Build a read-only palette
    pub fn locked(pal: Palette) -> Self {
        Self { pal, locked: true }
    }

    /// Build a modifiable possibly non-empty palette
    pub fn unlocked(pal: Palette) -> Self {
        Self { pal, locked: false }
    }

    /// Build a modifable empty palette
    pub fn empty() -> Self {
        Self::unlocked(Palette::empty())
    }

    #[inline]
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    #[inline]
    pub fn is_unlocked(&self) -> bool {
        !self.is_locked()
    }

    /// Get the modifiable version of the palette if unlocked
    pub fn as_palette_mut(&mut self) -> Option<&mut Palette> {
        if self.is_unlocked() {
            Some(&mut self.pal)
        }
        else {
            None
        }
    }

    pub fn as_palette(&self) -> &Palette {
        &self.pal
    }

    #[inline]
    pub fn into_palette(&self) -> Palette {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use cpclib_common::itertools::Itertools;

    use crate::ga::{self, Ink, InkComponentQuantity, Palette};

    #[test]
    fn test_ink() {
        assert_eq!(ga::Ink::from(ga::INK0), ga::Ink::from(0));
    }

    #[test]
    fn test_macro() {
        // let p = palette![0, 1, 11, 20];
    }

    #[test]
    fn test_into_ink() {
        assert_eq!(Ink::from(5u8), Ink { value: 5 });
        assert_eq!(Ink::from(5u64), Ink { value: 5 });
        assert_eq!(Ink::from(5i64), Ink { value: 5 });
    }

    #[test]
    fn test_from1() {
        let p: ga::Palette = vec![7, 8, 9, 10].into();

        assert_eq!(*p.get(0.into()), ga::INKS[7]);
        assert_eq!(*p.get(1.into()), ga::INKS[8]);
        assert_eq!(*p.get(2.into()), ga::INKS[9]);
        assert_eq!(*p.get(3.into()), ga::INKS[10]);
    }

    #[test]
    #[should_panic]
    fn test_from2() {
        let p: ga::Palette = vec![7, 8, 9, 10].into();

        p.get(4.into());
    }

    #[test]
    fn test_rgb() {
        const RGB_RATIOS: &[InkComponentQuantity] =
            &[InkComponentQuantity::Zero, InkComponentQuantity::Full];
        let rgb_palette = RGB_RATIOS
            .iter()
            .cartesian_product(RGB_RATIOS)
            .cartesian_product(RGB_RATIOS)
            .map(|t| (*t.0.0, *t.0.1, *t.1))
            .map(Ink::from);
        let rgb_palette = Palette::from(rgb_palette);
    }
}

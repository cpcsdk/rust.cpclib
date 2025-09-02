use std::fmt::{Debug, Display, Formatter, Result};

use image as im;
use nutype::nutype;

use self::im::Pixel;

/// Number of inks managed by the system. Do not take into account the few duplicates
const NB_INKS: u8 = 27 + 5;

const INK0_RGB: im::Rgb<u8> = im::Rgb([0, 0, 0]);
const INK1_RGB: im::Rgb<u8> = im::Rgb([0x00, 0x00, 0x80]);
const INK2_RGB: im::Rgb<u8> = im::Rgb([0x00, 0x00, 0xFF]);
const INK3_RGB: im::Rgb<u8> = im::Rgb([0x80, 0x00, 0x00]);
const INK4_RGB: im::Rgb<u8> = im::Rgb([0x80, 0x00, 0x80]);
const INK5_RGB: im::Rgb<u8> = im::Rgb([0x80, 0x00, 0xFF]);
const INK6_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0x00, 0x00]);
const INK7_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0x00, 0x80]);
const INK8_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0x00, 0xFF]);
const INK9_RGB: im::Rgb<u8> = im::Rgb([0x00, 0x80, 0x00]);
const INK10_RGB: im::Rgb<u8> = im::Rgb([0x00, 0x80, 0x80]);
const INK11_RGB: im::Rgb<u8> = im::Rgb([0x00, 0x80, 0xFF]);
const INK12_RGB: im::Rgb<u8> = im::Rgb([0x80, 0x80, 0x00]);
const INK13_RGB: im::Rgb<u8> = im::Rgb([0x80, 0x80, 0x80]);
const INK14_RGB: im::Rgb<u8> = im::Rgb([0x80, 0x80, 0xFF]);
const INK15_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0x80, 0x00]);
const INK16_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0x80, 0x80]);
const INK17_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0x80, 0xFF]);
const INK18_RGB: im::Rgb<u8> = im::Rgb([0x00, 0xFF, 0x00]);
const INK19_RGB: im::Rgb<u8> = im::Rgb([0x00, 0xFF, 0x80]);
const INK20_RGB: im::Rgb<u8> = im::Rgb([0x00, 0xFF, 0xFF]);
const INK21_RGB: im::Rgb<u8> = im::Rgb([0x80, 0xFF, 0x00]);
const INK22_RGB: im::Rgb<u8> = im::Rgb([0x80, 0xFF, 0x80]);
const INK23_RGB: im::Rgb<u8> = im::Rgb([0x80, 0xFF, 0xFF]);
const INK24_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0xFF, 0x00]);
const INK25_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0xFF, 0x80]);
const INK26_RGB: im::Rgb<u8> = im::Rgb([0xFF, 0xFF, 0xFF]);

/// RGB color for each ink
pub const INKS_RGB_VALUES: [im::Rgb<u8>; NB_INKS as usize] = [
    INK0_RGB, INK1_RGB, INK2_RGB, INK3_RGB, INK4_RGB, INK5_RGB, INK6_RGB, INK7_RGB, INK8_RGB,
    INK9_RGB, INK10_RGB, INK11_RGB, INK12_RGB, INK13_RGB, INK14_RGB, INK15_RGB, INK16_RGB,
    INK17_RGB, INK18_RGB, INK19_RGB, INK20_RGB, INK21_RGB, INK22_RGB, INK23_RGB, INK24_RGB,
    INK25_RGB, INK26_RGB, INK13_RGB, INK7_RGB, INK25_RGB, INK1_RGB, INK19_RGB // extra clones
];

/// Ga value for each ink
pub const INKS_GA_VALUE: [u8; NB_INKS as usize] = [
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
    0x4B, // 26
    0x41, // 27 => 13
    0x48, // 28 => 7
    0x49, // 29 => 25
    0x50, // 30 => 1
    0x51  // 31 => 19
];

/// Amstrad INK
#[nutype(
    const_fn,
    new_unchecked,
    derive(
        AsRef,
        Clone,
        Copy,
        Default,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize,
    ),
    default = 0,
    validate(less_or_equal = 31, greater_or_equal = 0)
)]
pub struct Ink(u8);

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
/// Do not generate a duplicated ink
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
    /// Return the duplicated ink when it exists
    pub fn duplicate(&self) -> Option<Ink> {
        match self.into_inner() {
            27 => Some(Ink::INKS[13]),
            28 => Some(Ink::INKS[7]),
            29 => Some(Ink::INKS[25]),
            30 => Some(Ink::INKS[1]),
            31 => Some(Ink::INKS[19]),

            13 => Some(Ink::INKS[27]),
            7 => Some(Ink::INKS[28]),
            25 => Some(Ink::INKS[29]),
            1 => Some(Ink::INKS[30]),
            19 => Some(Ink::INKS[31]),

            _ => None
        }
    }

    /// Get the RGB color value of the ink
    pub fn color(&self) -> im::Rgb<u8> {
        INKS_RGB_VALUES[self.firmware_number() as usize]
    }

    /// Give the quantity of red for the given color
    /// <http://cpc.sylvestre.org/technique/technique_coul1.html>
    pub fn red_quantity(&self) -> InkComponentQuantity {
        match self.as_ref() {
            0 | 1 | 2 | 9 | 10 | 11 | 18 | 19 | 20 => InkComponentQuantity::Zero,
            3 | 4 | 5 | 12 | 13 | 14 | 21 | 22 | 23 => InkComponentQuantity::Half,
            6 | 7 | 8 | 15 | 16 | 17 | 24 | 25 | 26 => InkComponentQuantity::Full,
            _ => self.duplicate().unwrap().red_quantity()
        }
    }

    /// Give the quantit of blue for the given ink
    /// <http://cpc.sylvestre.org/technique/technique_coul1.html>
    pub fn blue_quantity(&self) -> InkComponentQuantity {
        match self.as_ref() {
            0 | 3 | 6 | 9 | 12 | 15 | 18 | 21 | 24 => InkComponentQuantity::Zero,
            1 | 4 | 7 | 10 | 13 | 16 | 19 | 22 | 25 => InkComponentQuantity::Half,
            2 | 5 | 8 | 11 | 14 | 17 | 20 | 23 | 26 => InkComponentQuantity::Full,
            _ => self.duplicate().unwrap().blue_quantity()
        }
    }

    /// Give the quantity of green for the given ink
    /// <http://cpc.sylvestre.org/technique/technique_coul1.html>
    pub fn green_quantity(&self) -> InkComponentQuantity {
        match self.as_ref() {
            0..=8 => InkComponentQuantity::Zero,
            9..=17 => InkComponentQuantity::Half,
            18..=26 => InkComponentQuantity::Full,
            _ => self.duplicate().unwrap().green_quantity()
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
        *self = (r, g, b).into();
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
        *self = (r, g, b).into();
        self
    }

    pub fn is_duplicate(&self) -> bool {
        self.number() > 26
    }

    /// Get the ink number (firmware wise)
    /// Returns a number between 0 and 26
    pub fn firmware_number(&self) -> u8 {
        if self.is_duplicate() {
            self.duplicate().unwrap().into_inner()
        }
        else {
            self.into_inner()
        }
    }

    /// Get the value required by the gate array the select the ink
    pub fn gate_array_value(&self) -> u8 {
        INKS_GA_VALUE[self.firmware_number() as usize]
    }

    /// Returns a number between 0 and 31
    pub fn number(&self) -> u8 {
        self.into_inner()
    }

    pub fn from_gate_array_color_number(col: u8) -> Ink {
        let idx = INKS_GA_VALUE.iter().position(|i| *i == col).unwrap();
        Ink::INKS[idx]
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
        val.gate_array_value()
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

                let item = if item < 32 {
                    // Basic number provided
                    item
                } else {
                    assert!((item as i64) < 256);
                    // gate array number provided
                    INKS_GA_VALUE.iter()
                        .position(|&ink| ink == item as u8)
                        .unwrap() as _
                };
                unsafe{Self::new_unchecked(item as _)}
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
            "BRIGHTBLUE" | "BRIGHT_BLUE" => Self::BRIGHTBLUE,
            "RED" => Self::RED,
            "MAGENTA" => Self::MAGENTA,
            "MAUVE" => Self::MAUVE,
            "BRIGHTRED" => Self::BRIGHTRED,
            "PURPLE" => Self::PURPLE,
            "BRIGHTMAGENTA" | "BRIGHT_MAGENTA" => Self::BRIGHTMAGENTA,
            "GREEN" => Self::GREEN,
            "CYAN" => Self::CYAN,
            "SKYBLUE" | "SKY_BLUE" => Self::SKYBLUE,
            "YELLOW" => Self::YELLOW,
            "WHITE" => Self::WHITE,
            "PASTELBLUE" | "PASTEL_BLUE" => Self::PASTELBLUE,
            "ORANGE" => Self::ORANGE,
            "PINK" => Self::PINK,
            "PASTELMAGENTA" | "PASTEL_MAGENTA" => Self::PASTELMAGENTA,
            "BRIGHTGREEN" | "BRIGHT_GREEN" => Self::BRIGHTGREEN,
            "SEAGREEN" | "SEA_GREEN" => Self::SEAGREEN,
            "BRIGHTCYAN" | "BRIGHT_CYAN" => Self::BRIGHTCYAN,
            "LIME" => Self::LIME,
            "PASTELGREEN" | "PASTEL_GREEN"  => Self::PASTELGREEN,
            "PASTELCYAN" | "PASTEL_CYAN"  => Self::PASTELCYAN,
            "BRIGHTYELLOW" | "BRIGHT_YELLOW"  => Self::BRIGHTYELLOW,
            "PASTELYELLOW" | "PASTEL_YELLOW" => Self::PASTELYELLOW,
            "BRIGHTWHITE" | "BRIGHT_WHITE" => Self::BRIGHTWHITE,

            "GRAY" | "GREY" => "WHITE".into(),

            _ => panic!("`{item}` color does not exist")
        }
    }
}

impl Debug for Ink {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{} ({})", self, self.as_ref())
    }
}

impl Display for Ink {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let repr = match self.as_ref() {
            0 => "BLACK",
            1 => "BLUE",
            2 => "BRIGHT_BLUE",
            3 => "RED",
            4 => "MAGENTA",
            5 => "MAUVE",
            6 => "BRIGHT_RED",
            7 => "PURPLE",
            8 => "BRIGHT_MAGENTA",
            9 => "GREEN",
            10 => "CYAN",
            11 => "SKY_BLUE",
            12 => "YELLOW",
            13 => "WHITE",
            14 => "PASTEL_BLUE",
            15 => "ORANGE",
            16 => "PINK",
            17 => "PASTEL_MAGENTA",
            18 => "BRIGHT_GREEN",
            19 => "SEA_GREEN",
            20 => "BRIGHT_CYAN",
            21 => "LIME",
            22 => "PASTEL_GREEN",
            23 => "PASTEL_CYAN",
            24 => "BRIGHT_YELLOW",
            25 => "PASTEL_YELLOW",
            26 => "BRIGHT_WHITE",
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

impl Ink {
    pub const BLACK: Ink = Self::INKS[0];
    pub const BLUE: Ink = Self::INKS[1];
    pub const BRIGHTBLUE: Ink = Self::INKS[2];
    pub const BRIGHTCYAN: Ink = Self::INKS[20];
    pub const BRIGHTGREEN: Ink = Self::INKS[18];
    pub const BRIGHTMAGENTA: Ink = Self::INKS[8];
    pub const BRIGHTRED: Ink = Self::INKS[6];
    pub const BRIGHTWHITE: Ink = Self::INKS[26];
    pub const BRIGHTYELLOW: Ink = Self::INKS[24];
    pub const BRIGHT_BLUE: Ink = Self::BRIGHTBLUE;
    pub const BRIGHT_CYAN: Ink = Self::BRIGHTCYAN;
    pub const BRIGHT_GREEN: Ink = Self::BRIGHTGREEN;
    pub const BRIGHT_RED: Ink = Self::BRIGHTRED;
    pub const BRIGHT_WHITE: Ink = Self::BRIGHTWHITE;
    pub const BRIGHT_YELLOW: Ink = Self::BRIGHTYELLOW;
    pub const CYAN: Ink = Self::INKS[10];
    pub const GREEN: Ink = Self::INKS[9];
    /// Available inks
    pub const INKS: [Ink; NB_INKS as usize] = unsafe {
        [
            Ink::new_unchecked(0),
            Ink::new_unchecked(1),
            Ink::new_unchecked(2),
            Ink::new_unchecked(3),
            Ink::new_unchecked(4),
            Ink::new_unchecked(5),
            Ink::new_unchecked(6),
            Ink::new_unchecked(7),
            Ink::new_unchecked(8),
            Ink::new_unchecked(9),
            Ink::new_unchecked(10),
            Ink::new_unchecked(11),
            Ink::new_unchecked(12),
            Ink::new_unchecked(13),
            Ink::new_unchecked(14),
            Ink::new_unchecked(15),
            Ink::new_unchecked(16),
            Ink::new_unchecked(17),
            Ink::new_unchecked(18),
            Ink::new_unchecked(19),
            Ink::new_unchecked(20),
            Ink::new_unchecked(21),
            Ink::new_unchecked(22),
            Ink::new_unchecked(23),
            Ink::new_unchecked(24),
            Ink::new_unchecked(25),
            Ink::new_unchecked(26),
            Ink::new_unchecked(27),
            Ink::new_unchecked(28),
            Ink::new_unchecked(29),
            Ink::new_unchecked(30),
            Ink::new_unchecked(31)
        ]
    };
    pub const LIME: Ink = Self::INKS[21];
    pub const MAGENTA: Ink = Self::INKS[4];
    pub const MAUVE: Ink = Self::INKS[5];
    pub const ORANGE: Ink = Self::INKS[15];
    pub const PASTELBLUE: Ink = Self::INKS[14];
    pub const PASTELCYAN: Ink = Self::INKS[23];
    pub const PASTELGREEN: Ink = Self::INKS[22];
    pub const PASTELMAGENTA: Ink = Self::INKS[17];
    pub const PASTELYELLOW: Ink = Self::INKS[25];
    pub const PASTEL_BLUE: Ink = Self::PASTELBLUE;
    pub const PASTEL_CYAN: Ink = Self::PASTELCYAN;
    pub const PASTEL_GREEN: Ink = Self::PASTELGREEN;
    pub const PASTEL_MAGENTA: Ink = Self::PASTELMAGENTA;
    pub const PASTEL_YELLOW: Ink = Self::PASTELYELLOW;
    pub const PINK: Ink = Self::INKS[16];
    pub const PURPLE: Ink = Self::INKS[7];
    pub const RED: Ink = Self::INKS[3];
    pub const SEAGREEN: Ink = Self::INKS[19];
    pub const SEA_GREEN: Ink = Self::SEAGREEN;
    pub const SKYBLUE: Ink = Self::INKS[11];
    pub const SKY_BLUE: Ink = Self::SKYBLUE;
    pub const WHITE: Ink = Self::INKS[13];
    pub const YELLOW: Ink = Self::INKS[12];
}

#[cfg(test)]
mod tests {
    use cpclib_common::itertools::Itertools;

    use crate::ga::{self, Ink, InkComponentQuantity, Palette};
    use crate::ink::INK0_RGB;

    #[test]
    fn test_ink() {
        assert_eq!(ga::Ink::from(INK0_RGB), ga::Ink::from(0));
    }

    #[test]
    fn test_macro() {
        // let p = palette![0, 1, 11, 20];
    }

    #[test]
    fn test_into_ink() {
        assert_eq!(Ink::from(5u8), Ink::INKS[5]);
        assert_eq!(Ink::from(5u64), Ink::INKS[5]);
        assert_eq!(Ink::from(5i64), Ink::INKS[5]);
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

use image as im;

use self::im::Pixel;
use crate::image::Mode;
use num::Integer;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result};
use std::ops::Add;

const INK0: im::Rgba<u8> = im::Rgba {
    data: [0, 0, 0, 255],
};
const INK1: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0x00, 0x80, 255],
};
const INK2: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0x00, 0xFF, 255],
};
const INK3: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0x00, 0x00, 255],
};
const INK4: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0x00, 0x80, 255],
};
const INK5: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0x00, 0xFF, 255],
};
const INK6: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0x00, 0x00, 255],
};
const INK7: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0x00, 0x80, 255],
};
const INK8: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0x00, 0xFF, 255],
};
const INK9: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0x80, 0x00, 255],
};
const INK10: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0x80, 0x80, 255],
};
const INK11: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0x80, 0xFF, 255],
};
const INK12: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0x80, 0x00, 255],
};
const INK13: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0x80, 0x80, 255],
};
const INK14: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0x80, 0xFF, 255],
};
const INK15: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0x80, 0x00, 255],
};
const INK16: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0x80, 0x80, 255],
};
const INK17: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0x80, 0xFF, 255],
};
const INK18: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0xFF, 0x00, 255],
};
const INK19: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0xFF, 0x80, 255],
};
const INK20: im::Rgba<u8> = im::Rgba {
    data: [0x00, 0xFF, 0xFF, 255],
};
const INK21: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0xFF, 0x00, 255],
};
const INK22: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0xFF, 0x80, 255],
};
const INK23: im::Rgba<u8> = im::Rgba {
    data: [0x80, 0xFF, 0xFF, 255],
};
const INK24: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0xFF, 0x00, 255],
};
const INK25: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0xFF, 0x80, 255],
};
const INK26: im::Rgba<u8> = im::Rgba {
    data: [0xFF, 0xFF, 0xFF, 255],
};

/// Number of inks managed by the system. Do not take into account the few duplicates
const NB_INKS: u8 = 27;

/// Number of pens, including the border
const NB_PENS: u8 = 16 + 1;

/// RGB color for each ink
pub const INKS_RGB_VALUES: [im::Rgba<u8>; 27] = [
    INK0, INK1, INK2, INK3, INK4, INK5, INK6, INK7, INK8, INK9, INK10, INK11, INK12, INK13, INK14,
    INK15, INK16, INK17, INK18, INK19, INK20, INK21, INK22, INK23, INK24, INK25, INK26,
];

/// Ga value for each ink
pub const INKS_GA_VALUE: [u8; 27] = [
    0x54, 0x44, 0x55, 0x5c, 0x58, 0x5d, 0x4c, 0x45, 0x4d, 0x56, 0x46, 0x57, 0x5e, 0x40, 0x5f, 0x4e,
    0x47, 0x4f, 0x52, 0x42, 0x53, 0x5a, 0x59, 0x5b, 0x4a, 0x43, 0x4b,
];

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
/// Amstrad INK
pub struct Ink {
    /// Ink value
    value: u8,
}

#[allow(missing_docs)]
impl Ink {
    /// Get the RGB color value of the ink
    pub fn color(&self) -> im::Rgba<u8> {
        INKS_RGB_VALUES[self.value as usize]
    }

    /// Get the ink number (firmware wise)
    pub fn number(&self) -> u8 {
        self.value
    }

    /// Get the value required by the gate array the select the ink
    pub fn gate_array(&self) -> u8 {
        INKS_GA_VALUE[self.value as usize]
    }
}

impl Into<u8> for Ink {
    fn into(self) -> u8 {
        (&self).into()
    }
}

impl Into<u8> for &Ink {
    fn into(self) -> u8 {
        self.gate_array()
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
                (color_ink[0] as i32 - color[0] as i32).pow(2)
                    + (color_ink[1] as i32 - color[1] as i32).pow(2)
                    + (color_ink[2] as i32 - color[2] as i32).pow(2)
            })
            .collect::<Vec<_>>();
        let mut selected_idx = 0;
        let mut smallest = distances[0];
        for idx in 1..distances.len() {
            if smallest > distances[idx] {
                smallest = distances[idx];
                selected_idx = idx;
            }
        }
        Self::from(selected_idx as u8)
    }
}

impl From<u8> for Ink {
    fn from(item: u8) -> Self {
        Self { value: item }
    }
}

impl From<String> for Ink {
    fn from(item: String) -> Self {
        match item
            .to_uppercase()
            .replace(" ", "")
            .replace("_", "")
            .as_str()
        {
            "BLACK" => Self { value: 0 },
            "BLUE" => Self { value: 1 },
            "BRIGHTBLUE" => Self { value: 2 },
            "RED" => Self { value: 3 },
            "MAGENTA" => Self { value: 4 },
            "MAUVE" => Self { value: 5 },
            "BRIGHTRED" => Self { value: 6 },
            "PURPLE" => Self { value: 7 },
            "BRIGHTMAGENTA" => Self { value: 8 },
            "GREEN" => Self { value: 9 },
            "CYAN" => Self { value: 10 },
            "SKYBLUE" => Self { value: 11 },
            "YELLOW" => Self { value: 12 },
            "WHITE" => Self { value: 13 },
            "PASTELBLUE" => Self { value: 14 },
            "ORANGE" => Self { value: 15 },
            "PINK" => Self { value: 16 },
            "PASTELMAGENTA" => Self { value: 17 },
            "BRIGHTGREEN" => Self { value: 18 },
            "SEAGREEN" => Self { value: 19 },
            "BRIGHTCYAN" => Self { value: 20 },
            "LIME" => Self { value: 21 },
            "PASTELGREEN" => Self { value: 22 },
            "PASTELCYAN" => Self { value: 23 },
            "BRIGHTYELLOW" => Self { value: 24 },
            "PASTELYELLOW" => Self { value: 25 },
            "BRIGHTWHITE" => Self { value: 26 },

            "GRAY" | "GREY" => "WHITE".into(),

            _ => panic!("{} color does not exist", item),
        }
    }
}

impl Debug for Ink {
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
            _ => panic!(),
        };

        write!(f, "{} ({})", repr, self.value)
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
    Ink { value: 26 },
];

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug, Serialize, Deserialize)]
/// Represents a Pen. There a 16 pens + the border in the Amstrad CPC
pub struct Pen {
    /// pen value
    value: u8,
}

// Constructor of Pen from an integer
impl<T: Integer> From<T> for Pen
where
    i32: From<T>,
{
    fn from(item: T) -> Self {
        let item: i32 = item.into();
        Self { value: item as u8 }
    }
}

// Constructor of Pen reference from an integer
impl<'a, T: Integer> From<T> for &'a Pen
where
    i32: From<T>,
{
    fn from(item: T) -> Self {
        let pos: i32 = item.into();
        &PENS[pos as usize]
    }
}

#[allow(missing_docs)]
impl Pen {
    /// Get the number of the pen
    pub fn number(&self) -> u8 {
        self.value
    }

    /// Change the value of the pen in order to not exceed the number of pens available in the
    /// given mode
    pub fn limit(&mut self, mode: Mode) {
        self.value = match mode {
            Mode::Zero => self.value,
            Mode::Three | Mode::One => self.value & 3,
            Mode::Two => self.value & 1,
        };
    }
}

impl Add<i8> for Pen {
    type Output = Self;

    fn add(self, delta: i8) -> Self {
        Self {
            value: (self.value as i8 + delta) as u8,
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
    Pen { value: 16 }, // Border
];

/// The palette maps one Ink for each Pen
pub struct Palette {
    /// Values for the palette. Some itms may be absent
    values: HashMap<Pen, Ink>,
}

impl Clone for Palette {
    fn clone(&self) -> Self {
        let mut map: HashMap<Pen, Ink> = HashMap::new();
        for (pen, ink) in &self.values {
            map.insert(pen.clone(), ink.clone());
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
            p.set(&PENS[i], INKS[i]);
        }
        p
    }
}

impl<T> From<Vec<T>> for Palette
where
    Ink: From<T>,
    T: Copy,
{
    fn from(items: Vec<T>) -> Self {
        let mut p = Self::new();

        for (idx, ink) in items.iter().enumerate() {
            p.set(&Pen::from(idx as u8), Ink::from(ink.clone()));
        }

        p
    }
}

/// Create a palette with the right inks
/// Usage
/// `palette![1, 2, 3]`
#[macro_export]
macro_rules! palette {
    ( $( $x:expr ),* ) => {
        {
            extern crate cpclib as cpc;
            use cpc::ga;
            let mut palette = ga::Palette::default();
            let mut idx = 0;
            $(
                palette.set(&Pen::from(idx), Ink::from($x));
                idx += 1;
            )*

            // Ensure the other inks are black
            for i in idx..15 {
                palette.set(&Pen::from(i), Ink::from(0));
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
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let inks: Vec<Ink> = Vec::<Ink>::deserialize(deserializer)?;
        let palette: Self = inks.into();
        Ok(palette)
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

    /// Verifies if the palette contains the required pen
    pub fn contains_pen(&self, pen: &Pen) -> bool {
        self.values.contains_key(pen)
    }

    pub fn contains_border(&self) -> bool {
        self.contains_pen(&Pen::from(16))
    }

    /// Provides the next unused pen if there is one
    pub fn next_unused_pen(&self) -> Option<Pen> {
        for i in 0..16 {
            let pen = i.into();
            if !self.contains_pen(&pen) {
                return Some(pen);
            }
        }
        None
    }

    pub fn to_gate_array(&self) -> [u8; NB_PENS as usize] {
        let mut res = [0; NB_PENS as usize];
        for pen in 0..NB_PENS {
            res[pen as usize] = self.get(&pen.into()).gate_array();
        }
        res
    }

    /// Add the inks if not present in empty slots of the palette as soon as it is possible. Returns the number of inks added a,d the number of inks impossible to add because of the lack of space.
    pub fn add_novel_inks_except_in_border(&mut self, inks: &[Ink]) -> (usize, usize) {
        let counter_added = 0;
        let mut counter_impossible = 0;

        for ink in inks {
            // skip if already present
            if self.contains_ink(ink) {
                continue;
            }

            match self.next_unused_pen() {
                None => counter_impossible += 1,
                Some(pen) => {
                    self.set(&pen, ink.clone());
                }
            }
        }

        (counter_added, counter_impossible)
    }

    /// Returns the list of inks contained in the palette with the border
    pub fn inks_with_border(&self) -> Vec<Ink> {
        self.values
            .iter()
            .map(|(_, i)| i.clone())
            .collect::<Vec<Ink>>()
    }

    /// Returns the list of inks contained in the palette without taking into account the border
    pub fn inks(&self) -> Vec<Ink> {
        self.values
            .iter()
            .filter_map(|(&p, i)| {
                if p.number() == 16 {
                    None
                } else {
                    Some(i.clone())
                }
            })
            .collect::<Vec<Ink>>()
    }

    /// Returns all the set pens (without the border)
    pub fn pens_with_border(&self) -> Vec<Pen> {
        self.values
            .iter()
            .map(|(p, _)| p.clone())
            .collect::<Vec<Pen>>()
    }

    /// Returns all the set pens (without the border)
    pub fn pens(&self) -> Vec<Pen> {
        self.values
            .iter()
            .filter_map(|(&p, _)| {
                if p.number() == 16 {
                    None
                }
                else {
                    Some(p.clone())
                }
            })
            .collect::<Vec<Pen>>()
    }

    /// Get the ink of the requested pen. Pen MUST be present
    pub fn get(&self, pen: &Pen) -> &Ink {
        self.values.get(pen).expect("Wrong pen")
    }

    // Get the ink of the border
    pub fn get_border(&self) -> &Ink {
        self.values.get(&Pen::from(16)).expect("Border unavailable")
    }

    /// Change the ink of the specified pen
    pub fn set(&mut self, pen: &Pen, ink: Ink) {
        self.values.insert(pen.clone(), ink);
    }

    pub fn set_border(&mut self, ink: Ink) {
        self.values.insert(Pen::from(16), ink);
    }

    /// Get the pen that corresponds to the required ink.
    /// Ink 16 (border) is never tested
    pub fn get_pen_for_ink(&self, expected: &Ink) -> Option<Pen> {
        self.values
            .iter()
            .filter(|(&p, _)| p.number() != 16)
            .find_map(|(p, i)| {
               if i == expected {
                   Some(p.clone())
               }
               else {
                   None
               }
            })
    }

    /// Returns true if the palette contains the inks in one of its pens (except border)
    pub fn contains_ink(&self, expected: &Ink) -> bool {
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

        p.set(&PENS[4], ink3.clone());
        p.set(&PENS[5], ink0.clone());
        p.set(&PENS[6], ink0.clone());
        p.set(&PENS[7], ink0.clone());
        p.set(&PENS[8], ink1.clone());
        p.set(&PENS[9], ink3.clone());
        p.set(&PENS[10], ink1.clone());
        p.set(&PENS[11], ink1.clone());
        p.set(&PENS[12], ink2.clone());
        p.set(&PENS[13], ink2.clone());
        p.set(&PENS[14], ink3.clone());
        p.set(&PENS[15], ink2.clone());

        p
    }
}

impl Into<Vec<u8>> for &Palette {
    fn into(self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(16);
        for pen in 0..17 {
            let pen = Pen::from(pen);
            if self.contains_pen(&pen) {
                vec.push(self.get(&pen).into());
            } else {
                vec.push(0x54); // No pens => ink black
            }
        }
        vec
    }
}

impl Into<Vec<u8>> for Palette {
    fn into(self) -> Vec<u8> {
        (&self).into()
    }
}

#[allow(missing_docs)]
impl Palette {
    pub fn to_vec(self) -> Vec<u8> {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::ga;

    #[test]
    fn test_ink() {
        assert_eq!(ga::Ink::from(ga::INK0), ga::Ink::from(0));
    }

    #[test]
    fn test_macro() {
        // let p = palette![0, 1, 11, 20];
    }

    #[test]
    fn test_from() {
        let p: ga::Palette = vec![7, 8, 9, 10].into();

        assert_eq!(*p.get(0.into()), ga::INKS[7]);
        assert_eq!(*p.get(1.into()), ga::INKS[8]);
        assert_eq!(*p.get(2.into()), ga::INKS[9]);
        assert_eq!(*p.get(3.into()), ga::INKS[10]);
        assert_eq!(*p.get(4.into()), ga::INKS[0]);
    }
}

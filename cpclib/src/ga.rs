
extern crate image as im;

use self::im::Pixel;
use std::collections::HashMap;
use std::ops::Add;
use std::fmt::{Formatter, Debug, Result} ;
use num::Integer;
use image::Mode;

const INK0:im::Rgba<u8> = im::Rgba{data:[0, 0, 0, 255]};
const INK1:im::Rgba<u8> = im::Rgba{data: [0x00, 0x00, 0x80, 255]};
const INK2:im::Rgba<u8> = im::Rgba{data: [0x00, 0x00, 0xFF, 255]};
const INK3:im::Rgba<u8> = im::Rgba{data: [0x80, 0x00, 0x00, 255]};
const INK4:im::Rgba<u8> = im::Rgba{data: [0x80, 0x00, 0x80, 255]};
const INK5:im::Rgba<u8> = im::Rgba{data: [0x80, 0x00, 0xFF, 255]};
const INK6:im::Rgba<u8> = im::Rgba{data: [0xFF, 0x00, 0x00, 255]};
const INK7:im::Rgba<u8> = im::Rgba{data: [0xFF, 0x00, 0x80, 255]};
const INK8:im::Rgba<u8> = im::Rgba{data: [0xFF, 0x00, 0xFF, 255]};
const INK9:im::Rgba<u8> = im::Rgba{data: [0x00, 0x80, 0x00, 255]};
const INK10:im::Rgba<u8> = im::Rgba{data: [0x00, 0x80, 0x80, 255]};
const INK11:im::Rgba<u8> = im::Rgba{data: [0x00, 0x80, 0xFF, 255]};
const INK12:im::Rgba<u8> = im::Rgba{data: [0x80, 0x80, 0x00, 255]};
const INK13:im::Rgba<u8> = im::Rgba{data: [0x80, 0x80, 0x80, 255]};
const INK14:im::Rgba<u8> = im::Rgba{data: [0x80, 0x80, 0xFF, 255]};
const INK15:im::Rgba<u8> = im::Rgba{data: [0xFF, 0x80, 0x00, 255]};
const INK16:im::Rgba<u8> = im::Rgba{data: [0xFF, 0x80, 0x80, 255]};
const INK17:im::Rgba<u8> = im::Rgba{data: [0xFF, 0x80, 0xFF, 255]};
const INK18:im::Rgba<u8> = im::Rgba{data: [0x00, 0xFF, 0x00, 255]};
const INK19:im::Rgba<u8> = im::Rgba{data: [0x00, 0xFF, 0x80, 255]};
const INK20:im::Rgba<u8> = im::Rgba{data: [0x00, 0xFF, 0xFF, 255]};
const INK21:im::Rgba<u8> = im::Rgba{data: [0x80, 0xFF, 0x00, 255]};
const INK22:im::Rgba<u8> = im::Rgba{data: [0x80, 0xFF, 0x80, 255]};
const INK23:im::Rgba<u8> = im::Rgba{data: [0x80, 0xFF, 0xFF, 255]};
const INK24:im::Rgba<u8> = im::Rgba{data: [0xFF, 0xFF, 0x00, 255]};
const INK25:im::Rgba<u8> = im::Rgba{data: [0xFF, 0xFF, 0x80, 255]};
const INK26:im::Rgba<u8> = im::Rgba{data: [0xFF, 0xFF, 0xFF, 255]};

pub const INKS_VALUES:[im::Rgba<u8>; 27] = [
	INK0,
	INK1,
	INK2,
	INK3,
	INK4,
	INK5,
	INK6,
	INK7,
	INK8,
	INK9,
	INK10,
	INK11,
	INK12,
	INK13,
	INK14,
	INK15,
	INK16,
	INK17,
	INK18,
	INK19,
	INK20,
	INK21,
	INK22,
	INK23,
	INK24,
	INK25,
	INK26
];


const NB_INKS: u8 = 27;
const NB_PENS: u8 = 16 + 1;


#[derive(Clone, Copy, Ord, PartialOrd, Eq, Hash)]
pub struct Ink {
    value: u8
}

/// Two inks are equal only if they share the same number
impl PartialEq for Ink {
    fn eq(&self, other: &Ink) -> bool {
        self.value == other.value
    }
}

impl Ink {
    pub fn color(&self) -> im::Rgba<u8> {
        INKS_VALUES[self.value as usize]
    }

    pub fn number(&self) -> u8 {
        return self.value;
    }
}


impl From<im::Rgba<u8>> for Ink{
    
    fn from(color: im::Rgba<u8>) -> Self {
        Ink::from(color.to_rgb())
    }
}

impl From<im::Rgb<u8>> for Ink {
    fn from(color: im::Rgb<u8>) -> Self {
        // Not strict comparison
        let distances = INKS_VALUES.into_iter().map(|color_ink|{
              (color_ink[0] as i32 - color[0] as i32).pow(2)
            + (color_ink[1] as i32 - color[1] as i32).pow(2)
            + (color_ink[2] as i32 - color[2] as i32).pow(2)
        }).collect::<Vec<_>>();
        let mut selected_idx = 0;
        let mut smallest = distances[0];
        for idx in 1..distances.len() {
            if smallest > distances[idx] {
                smallest = distances[idx];
                selected_idx = idx;
            }
        }
        Ink::from(selected_idx as u8)

        /*
         //Strict comparison
        for (idx, color_ink) in INKS_VALUES.iter().enumerate() {
            if color_ink[0] == color[0] && color_ink[1] == color[1] && color_ink[2] == color[2] {
                ink = Some(Ink::from(idx as u8));
            }
        }
        assert!(ink.is_some());
        ink.unwrap()
        */
    }

}

impl From<u8> for Ink {
    fn from(item: u8) -> Self {
        Ink{value: item}
    }
}

impl From<String> for Ink {
    fn from(item: String) -> Self {
        match item
                .to_uppercase()
                .replace(" ", "")
                .replace("_", "")
                .as_str() {
            "BLACK" => Ink{value: 0},
            "BLUE" => Ink{value: 1},
            "BRIGHTBLUE" => Ink{value: 2},
            "RED" => Ink{value: 3},
            "MAGENTA" => Ink{value: 4},
            "MAUVE" => Ink{value: 5},
            "BRIGHTRED" => Ink{value: 6},
            "PURPLE" => Ink{value: 7},
            "BRIGHTMAGENTA" => Ink{value: 8},
            "GREEN" => Ink{value: 9},
            "CYAN" => Ink{value: 10},
            "SKYBLUE" => Ink{value: 11},
            "YELLOW" => Ink{value: 12},
            "WHITE" => Ink{value: 13},
            "PASTELBLUE" => Ink{value: 14},
            "ORANGE" => Ink{value: 15},
            "PINK" => Ink{value: 16},
            "PASTELMAGENTA" => Ink{value: 17},
            "BRIGHTGREEN" => Ink{value: 18},
            "SEAGREEN" => Ink{value: 19},
            "BRIGHTCYAN" => Ink{value: 20},
            "LIME" => Ink{value: 21},
            "PASTELGREEN" => Ink{value: 22},
            "PASTELCYAN" => Ink{value: 23},
            "BRIGHTYELLOW" => Ink{value: 24},
            "PASTELYELLOW" => Ink{value: 25},
            "BRIGHTWHITE" => Ink{value: 26},
            _ => panic!("{} color does not exist", item)
        }
    }
}


impl Debug for Ink {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let repr = match self.value {
            0=> "BLACK" ,
            1=> "BLUE" ,
            2=> "BRIGHTBLUE" ,
            3=> "RED" ,
            4=> "MAGENTA" ,
            5=> "MAUVE" ,
            6=> "BRIGHTRED" ,
            7=> "PURPLE" ,
            8=> "BRIGHTMAGENTA" ,
            9=> "GREEN" ,
            10=> "CYAN" ,
            11=> "SKYBLUE" ,
            12=> "YELLOW" ,
            13=> "WHITE" ,
            14=> "PASTELBLUE" ,
            15=> "ORANGE" ,
            16=> "PINK" ,
            17=> "PASTELMAGENTA" ,
            18=> "BRIGHTGREEN" ,
            19=> "SEAGREEN" ,
            20=> "BRIGHTCYAN" ,
            21=> "LIME" ,
            22=> "PASTELGREEN" ,
            23=> "PASTELCYAN" ,
            24=> "BRIGHTYELLOW" ,
            25=> "PASTELYELLOW" ,
            26=> "BRIGHTWHITE" ,
            _ => panic!()
        };

        write!(f, "{} ({})", repr, self.value)
    }
}

impl<'a> From<&'a str> for Ink {
    fn from(item: &'a str) -> Self {
        Ink::from(String::from(item))
    }
}

pub const INKS:[Ink; NB_INKS as usize] = [
    Ink{value: 0},
    Ink{value: 1},
    Ink{value: 2},
    Ink{value: 3},
    Ink{value: 4},
    Ink{value: 5},
    Ink{value: 6},
    Ink{value: 7},
    Ink{value: 8},
    Ink{value: 9},
    Ink{value: 10},
    Ink{value: 11},
    Ink{value: 12},
    Ink{value: 13},
    Ink{value: 14},
    Ink{value: 15},
    Ink{value: 16},
    Ink{value: 17},
    Ink{value: 18},
    Ink{value: 19},
    Ink{value: 20},
    Ink{value: 21},
    Ink{value: 22},
    Ink{value: 23},
    Ink{value: 24},
    Ink{value: 25},
    Ink{value: 26},
];

#[derive(Eq,  PartialEq, Hash, Clone, Copy, Debug)]
pub struct Pen {
    value: u8
}

// Constructor of Pen from an integer
impl<T:Integer> From<T> for Pen
where i32: From<T>
{
    fn from(item: T) -> Self {
        let item:i32 = item.into();
        Pen{value: item as u8}
    }
}

// Constructor of Pen reference from an integer
impl<'a, T:Integer> From<T> for &'a Pen
where i32: From<T>
{
    fn from(item: T) -> Self {
        let pos:i32= item.into();
        & PENS[pos as usize]
    }
}

impl Pen{
    pub fn number(&self) -> u8 {
        return self.value;
    }

    /// Change the value of the pen in order to not exceed the number of pens available in the
    /// given mode
    pub fn limit(&mut self, mode: Mode) {
        self.value = match mode {
            Mode::Mode0 => self.value,
            Mode::Mode3 | Mode::Mode1 => self.value & 3,
            Mode::Mode2 => self.value & 1,
        };
    }
}


/*
 * XXX To be removed once impl<T:Integer> From<T> for Pen is validated
impl From<u8> for Pen{
    fn from(item: u8) -> Self {
        Pen{value: item}
    }
}

impl From<usize> for Pen{
    fn from(item: usize) -> Self {
        Pen{value: item as u8}
    }
}


impl From<i32> for Pen{
    fn from(item: i32) -> Self {
        Pen{value: item as u8}
    }
}
*/


impl Add<i8> for Pen {
    type Output = Pen;

    fn add(self, delta: i8) -> Pen{
        Pen{
            value: (self.value as i8 + delta) as u8
        }
    }
}



pub const PENS:[Pen; NB_PENS as usize] = [
    Pen{value: 0},
    Pen{value: 1},
    Pen{value: 2},
    Pen{value: 3},
    Pen{value: 4},
    Pen{value: 5},
    Pen{value: 6},
    Pen{value: 7},
    Pen{value: 8},
    Pen{value: 9},
    Pen{value: 10},
    Pen{value: 11},
    Pen{value: 12},
    Pen{value: 13},
    Pen{value: 14},
    Pen{value: 15},
    Pen{value: 16},
];


pub struct Palette {
    values: HashMap<Pen, Ink>
}


impl Clone for Palette {
    fn clone(&self) -> Self {
        let mut map:HashMap<Pen, Ink> = HashMap::new();
        for (pen, ink) in &self.values{
            map.insert(pen.clone(), ink.clone());
        }

        Palette {
            values: map
        }
    }
}


impl Debug for Palette{
    fn fmt(&self, f: &mut Formatter) -> Result {
        for i in 0..16 {
            write!(f, "{} => {:?}\n", i, self.values.get(&Pen::from(i)))?;
        }
        Ok(())
    }
}

impl Default for Palette {
    /// Create a new palette.
    /// Pen colors are the same than Amsdos ones.
    fn default() -> Palette {
        let mut p = Palette::new();
        for i in 0..15 {
            p.set(&PENS[i], INKS[i]);
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
            use cpc::ga::{Palette, Pen, Ink};
            let mut palette = Palette::default();
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

impl Palette {

    /// Create a new palette.
    /// All pens are black.
    pub fn new() -> Palette {
        let mut  map:HashMap<Pen, Ink> = HashMap::new();

        for pen in 0..NB_PENS {
            map.insert(Pen{value:pen}, Ink{value:0});
        }

        Palette {
            values: map
        }
    }


    /// Get the ink of the requested pen
    pub fn get(&self, pen: &Pen) -> &Ink {
        self.values.get(pen).expect("Wrong pen")
    }

    /// Change the ink of the specified pen
    pub fn set(& mut self, pen: &Pen, ink: Ink) {
        self.values.insert(pen.clone(), ink);
    }


    /// Get the pen that corresponds to the required ink.
    /// Ink 16 (border) is never tested
    pub fn get_pen_for_ink(&self, expected: &Ink) -> Option<Pen> {
        let mut res = None;

        for i in 0..16 {
            let pen = Pen::from(i);
            let ink = self.values.get(&pen).unwrap();
            if ink == expected && i != 16 {
                res = Some(pen);
                break;
            }
        }

        res
    }

    /// Replicate the firsts 4 pens in order to manage special texture that contains both mode 0
    /// and mode 3 patterns
    pub fn to_mode3_mixed_with_mode0(&self) -> Palette {
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



#[cfg(test)]
mod tests {
    use ga;

    #[test]
    fn test_ink() {
            assert_eq!(ga::Ink::from(ga::INK0), ga::Ink::from(0));
    
    }
}

use std::fmt::Debug;
use std::ops::Add;

use cpclib_common::num::Integer;
use serde::{Deserialize, Serialize};

use crate::image::Mode;

/// Number of pens, including the border
const NB_PENS: u8 = 16 + 1;

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
        &Pen::PENS[pos as usize]
    }
}

#[allow(missing_docs)]
impl Pen {
    pub const NB_PENS: u8 = NB_PENS;
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

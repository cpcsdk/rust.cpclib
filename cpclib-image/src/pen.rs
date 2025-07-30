use std::fmt::Debug;
use std::ops::Add;

use cpclib_common::num::Integer;
use nutype::nutype;

use crate::image::Mode;

/// Number of pens, including the border
const NB_PENS: u8 = 16 + 1;

#[nutype(
    const_fn,
    new_unchecked,
    derive(
        AsRef,
        Clone,
        Copy,
        Debug,
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
    validate(less_or_equal = 16, greater_or_equal = 0)
)]
/// Represents a Pen. There a 16 pens + the border in the Amstrad CPC
pub struct Pen(u8);

// Constructor of Pen from an integer
impl<T: Integer> From<T> for Pen
where i32: From<T>
{
    fn from(item: T) -> Self {
        let item: i32 = item.into();
        Pen::PENS[item as usize]
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
    pub const PENS: [Pen; NB_PENS as usize] = unsafe {
        [
            Pen::new_unchecked(0),
            Pen::new_unchecked(1),
            Pen::new_unchecked(2),
            Pen::new_unchecked(3),
            Pen::new_unchecked(4),
            Pen::new_unchecked(5),
            Pen::new_unchecked(6),
            Pen::new_unchecked(7),
            Pen::new_unchecked(8),
            Pen::new_unchecked(9),
            Pen::new_unchecked(10),
            Pen::new_unchecked(11),
            Pen::new_unchecked(12),
            Pen::new_unchecked(13),
            Pen::new_unchecked(14),
            Pen::new_unchecked(15),
            Pen::new_unchecked(16)
        ]
    };

    /// Get the number of the pen
    #[inline]
    pub fn number(&self) -> u8 {
        self.into_inner()
    }

    /// Change the value of the pen in order to not exceed the number of pens available in the
    /// given mode
    pub fn limit(&mut self, mode: Mode) {
        *self = match mode {
            Mode::Zero => self.number(),
            Mode::Three | Mode::One => self.number() & 3,
            Mode::Two => self.number() & 1
        }
        .into();
    }
}

impl Add<i8> for Pen {
    type Output = Self;

    fn add(self, delta: i8) -> Self {
        let value = ((self.number() as i8 + 17 + delta) as u8) % 17;

        unsafe { Self::new_unchecked(value) }
    }
}

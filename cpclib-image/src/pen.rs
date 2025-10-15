use std::fmt::Debug;
use std::ops::{Add, Sub};

use cpclib_common::num::Integer;
use serde::{Deserialize, Serialize};

use crate::image::Mode;

/// Number of pens, including the border
const NB_PENS: u8 = 16 + 1;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Pen {
    #[default]
    Pen0=0, Pen1, Pen2, Pen3, Pen4,
    Pen5, Pen6, Pen7, Pen8, Pen9,
    Pen10, Pen11, Pen12, Pen13, Pen14,
    Pen15, Border
}

// Constructor of Pen from an integer
impl<T: Integer> From<T> for Pen
where i32: From<T>
{
    fn from(item: T) -> Self {
        let pen: &Pen = <&Pen>::from(item);
        *pen
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

macro_rules! into_integer {
    ( $($ty:ty),+ ) => {
        $(
            impl Into<$ty> for Pen {
                fn into(self) -> $ty {
                    self.number() as _
                }
            
            }
        )+
    };
}

into_integer!{u8, i8, u16, i16, u32, i32, u64, i64, usize}


#[allow(missing_docs)]
impl Pen {
    pub const NB_PENS: u8 = NB_PENS;
    /// Available pens
    pub const PENS: [Pen; NB_PENS as usize] = [
        Self::Pen0, Self::Pen1, Self::Pen2, Self::Pen3, Self::Pen4,
        Self::Pen5, Self::Pen6, Self::Pen7, Self::Pen8, Self::Pen9,
        Self::Pen10, Self::Pen11, Self::Pen12, Self::Pen13, Self::Pen14,
        Self::Pen15, Self::Border
    ]; 

    pub fn border() -> Self {
        Self::Border
    }

    pub fn pen<T: Integer> (v: T) -> Self where i32: From<T> {
        let mut p = Pen::from(v);
        p.limit(Mode::Zero); // ensure border is not set
        p
    }

    /// Get the number of the pen
    #[inline]
    pub fn number(&self) -> u8 {
        *self as u8
    }

    /// Change the value of the pen in order to not exceed the number of pens available (border excluded) in the
    /// given mode
    pub fn limit(&mut self, mode: Mode) {
        *self = match mode {
            Mode::Zero => self.number() & 15,
            Mode::Three | Mode::One => self.number() & 3,
            Mode::Two => self.number() & 1
        }
        .into();
    }

    pub fn wrapping_add_border_included<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_wrapping_add(delta.into(), 17)
    }
    pub fn wrapping_add_border_excluded<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_wrapping_add(delta.into(), 17)
    }
    pub fn saturating_add_border_included<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_saturating_add(delta.into(), 16)
    }
    pub fn saturating_add_border_excluded<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_saturating_add(delta.into(), 16)
    }

    pub fn wrapping_sub_border_included<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_wrapping_sub(delta.into(), 17)
    }
    pub fn wrapping_sub_border_excluded<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_wrapping_sub(delta.into(), 17)
    }
    pub fn saturating_sub_border_included<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_saturating_sub(delta.into(), 16)
    }
    pub fn saturating_sub_border_excluded<P: Into<i8>> (&self, delta: P) -> Self {
        self.inner_saturating_sub(delta.into(), 16)
    }


    fn inner_wrapping_add(&self, delta: i8, max: u8 ) -> Self {
        let value = ((self.number() as i8 + max as i8 + delta) as u8) % max;  
        value.into()
    }
    fn inner_saturating_add(&self, delta: i8, max: u8) -> Self {
        let value = ((self.number() as i8 + max as i8 + delta) as u8).max(max);  
        value.into()
    }

    fn inner_wrapping_sub(&self, delta: i8, max: u8 ) -> Self {
        let value = ((self.number() as i8 + max as i8 - delta) as u8) % max;  
        value.into()
    }
    fn inner_saturating_sub(&self, delta: i8, max: u8) -> Self {
        let value = ((self.number() as i8 + max as i8 - delta) as u8).max(max);  
        value.into()
    }

}

impl<P: Into<i8>> Add<P> for Pen {
    type Output = Self;

    /// Wrapping add without selecting the border
    fn add(self, delta: P) -> Self {
        self.wrapping_add_border_excluded(delta)
    }
}

impl<P: Into<i8>> Sub<P> for Pen {
    type Output = Self;

    /// Wrapping sub without selecting the border
    fn sub(self, delta: P) -> Self {
        self.wrapping_sub_border_excluded(delta)
    }
}


#[cfg(test)]
mod test {
    use crate::pen::Pen;
    #[test]
    fn pen_add() {

        for i in (0..16) {
            assert_eq!(
                Pen::pen(0) + i,
                Pen::pen(i)
            );
        }
    }
}

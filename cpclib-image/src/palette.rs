use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result};
use std::ops::Deref;

use cpclib_common::itertools::Itertools;
use image as im;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use self::im::Pixel;
use crate::ga::{Ink, InkComponent, InkComponentQuantity, Pen};
use crate::image::Mode;

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
            p.set(Pen::PENS[i], Ink::INKS[i]);
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

        for pen in 0..Pen::NB_PENS {
            map.insert(Pen::from(pen), Ink::from(0));
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
    pub fn to_gate_array(&self) -> [u8; Pen::NB_PENS as usize] {
        let mut res = [0; Pen::NB_PENS as usize];
        for pen in 0..Pen::NB_PENS {
            res[pen as usize] = self.get(&pen.into()).gate_array_value();
        }
        res
    }

    pub fn to_gate_array_with_default(&self, default: Ink) -> [u8; Pen::NB_PENS as usize] {
        let mut res = [0; Pen::NB_PENS as usize];
        for pen in 0..Pen::NB_PENS {
            res[pen as usize] = self
                .get_with_default(&pen.into(), &default)
                .gate_array_value();
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

        let ink0 = self.get(&Pen::PENS[0]);
        let ink1 = self.get(&Pen::PENS[1]);
        let ink2 = self.get(&Pen::PENS[2]);
        let ink3 = self.get(&Pen::PENS[3]);

        p.set(Pen::PENS[4], *ink3);
        p.set(Pen::PENS[5], *ink0);
        p.set(Pen::PENS[6], *ink0);
        p.set(Pen::PENS[7], *ink0);
        p.set(Pen::PENS[8], *ink1);
        p.set(Pen::PENS[9], *ink3);
        p.set(Pen::PENS[10], *ink1);
        p.set(Pen::PENS[11], *ink1);
        p.set(Pen::PENS[12], *ink2);
        p.set(Pen::PENS[13], *ink2);
        p.set(Pen::PENS[14], *ink3);
        p.set(Pen::PENS[15], *ink2);

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

impl From<LockablePalette> for Palette {
    fn from(val: LockablePalette) -> Self {
        val.pal
    }
}

impl From<&LockablePalette> for Palette {
    fn from(val: &LockablePalette) -> Self {
        val.pal.clone()
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

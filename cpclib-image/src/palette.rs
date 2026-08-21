use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result};
use std::ops::Deref;

use cpclib_common::itertools::Itertools;
use owo_colors::OwoColorize;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::asic::AsicColor;
use crate::color::AmstradColor;
use crate::ga::{Ink, InkComponent, InkComponentQuantity, Pen};
use crate::image::Mode;

/// The palette maps one color for each Pen
pub struct Palette< C: AmstradColor > {
    /// Values for the palette. Some items may be absent
    values: HashMap<Pen, C>
}

impl<C: AmstradColor + Clone> Clone for Palette<C> {
    fn clone(&self) -> Self {
        let mut map: HashMap<Pen, C> = HashMap::new();
        for (pen, color) in &self.values {
            map.insert(*pen, color.clone());
        }

        Self { values: map }
    }
}

impl<C: AmstradColor + Debug> Debug for Palette<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for i in 0..16 {
            writeln!(f, "{} => {:?}", i, self.values.get(&Pen::from(i)))?;
        }
        Ok(())
    }
}

impl Default for Palette<Ink> {
    /// Create a new palette.
    /// Pens ink are the same than Amsdos ones.
    fn default() -> Self {
        let mut pal = Self::new();
        for (p, i) in [1, 24, 20, 6, 26, 0, 2, 8, 12, 14, 16, 18, 22, 1, 11]
            .into_iter()
            .enumerate()
        {
            pal.set(Pen::from(p as u8), Ink::from(i));
        }
        pal.set_border(Ink::from(1));
        pal
    }
}

impl Default for Palette<AsicColor> {
    /// Create a new palette.
    /// Pens ink are the same than Amsdos ones.
    fn default() -> Self {
        let p = Palette::<Ink>::default();
        p.into()
    }
}

impl From<Palette<Ink>> for Palette<AsicColor> {
    fn from(p: Palette<Ink>) -> Self {
        let mut map: HashMap<Pen, AsicColor> = HashMap::new();
        for (pen, ink) in p.values {
            map.insert(pen, AsicColor::from(ink));
        }

        Self { values: map }
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

impl<C: AmstradColor> Palette<C> {
    /// Create a palette from an iterator of items that can convert to Ink
    pub fn from_iter<S, T>(items: S) -> Self
    where
        S: IntoIterator<Item = T>,
        C: From<T>
    {
        let mut p = Self::empty();
        let items = items.into_iter();

        for (idx, color) in items.enumerate().take(16 + 1) {
            p.set(Pen::from(idx as u8), C::from(color));
        }

        p
    }
}

impl<C: AmstradColor> From<Vec<C>> for Palette<C> {
    fn from(items: Vec<C>) -> Self {
        // Turbofished on purpose: `AmstradColor` requires `From<Rgb<u8>>`, so
        // `from_iter`'s `T` is ambiguous between `C` and `Rgb<u8>` here.
        Self::from_iter::<Vec<C>, C>(items)
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

            // Fully qualified: the macro expands into the caller's scope,
            // which need not have `Palette` in it.
            let mut palette: ga::Palette<Ink> = ga::Palette::<Ink>::default();
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

impl<C: AmstradColor + Serialize> Serialize for Palette<C> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(17))?;
        for i in 0..17 {
            let entry = self.get(i.into());
            seq.serialize_element(entry)?;
        }
        seq.end()
    }
}

impl<'de, C: AmstradColor + Deserialize<'de>> Deserialize<'de> for Palette<C> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let inks: Vec<C> = Vec::<C>::deserialize(deserializer)?;
        let palette: Self = Self::from_iter::<Vec<C>, C>(inks);
        Ok(palette)
    }
}

impl<C: AmstradColor, P> std::ops::Index<P> for Palette<C>
where P: Into<Pen>
{
    type Output = C;

    fn index(&self, p: P) -> &Self::Output {
        self.get(&p.into())
    }
}


impl Palette<Ink> {
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

    #[inline]
    pub fn inks(&self) -> Vec<Ink> {
        self.colors()
    }

    #[inline]
    pub fn get_pen_for_ink<I: Into<Ink>>(&self, expected: I) -> Option<Pen> {
        self.get_pen_for_color(expected.into())
    }



    pub fn add_novel_inks_except_in_border(&mut self, inks: &[Ink]) -> (usize, usize) {
        self.add_novel_colors_except_in_border(inks)
    }

    pub fn inks_with_border(&self) -> Vec<Ink> {
        self.colors_with_border()
    }

    pub fn contains_ink(&self, expected: Ink) -> bool {
        self.contains_color(expected)
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
    pub fn rgb_fadout(&self) -> Vec<Palette<Ink>> {
        // Check if we can still decrease the components
        let is_finished = |p: &Palette<Ink>, c: InkComponent| {
            p.inks()
                .iter()
                .all(|ink| ink.component_quantity(c) == InkComponentQuantity::Zero)
        };

        // Decrease a given component
        let decrease_component = |p: &Palette<Ink>, c: InkComponent| {
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
        palettes.push(self.clone());
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
}

impl Palette<AsicColor> {
    pub fn to_asic(&self) -> [u16; Pen::NB_PENS as usize] {
        let mut res = [0; Pen::NB_PENS as usize];
        for pen in 0..Pen::NB_PENS {
            res[pen as usize] = self.get(&pen.into()).value();
        }
        res
    }

    pub fn to_asic_with_default(&self, default: AsicColor) -> [u16; Pen::NB_PENS as usize] {
        let mut res = [0; Pen::NB_PENS as usize];
        for pen in 0..Pen::NB_PENS {
            res[pen as usize] = self.get_with_default(&pen.into(), &default).value();
        }
        res
    }
}

#[allow(missing_docs)]
impl<C: AmstradColor> Palette<C> {
    /// Create a new palette.
    /// All pens are black.
    pub fn new() -> Self {
        let mut map: HashMap<Pen, C> = HashMap::new();

        for pen in 0..Pen::NB_PENS {
            map.insert(Pen::from(pen), C::black());
        }

        Self { values: map }
    }

    /// Create an empty Palette.
    /// An empty palette does not contains all the inks and must make crash most of the code that has been previously written !
    pub fn empty() -> Self {
        Self {
            values: Default::default()
        }
    }

    /// Returns true if all standard inks are different
    pub fn nb_different_colors(&self) -> usize {
        use std::collections::HashSet;
        let mut set = HashSet::<C>::default();
        for pen in 0..16 {
            set.insert(*self.get(&pen.into()));
        }

        set.len()
    }

    #[deprecated(note = "Use nb_different_colors instead")]
    pub fn nb_different_inks(&self) -> usize {
        self.nb_different_colors()
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


    /// Add the colors if not present in empty slots of the palette as soon as it is possible. Returns the number of colors added and the number of colors impossible to add because of the lack of space.
    pub fn add_novel_colors_except_in_border(&mut self, colors: &[C]) -> (usize, usize) {
        let counter_added = 0;
        let mut counter_impossible = 0;

        for color in colors.iter() {
            // skip if already present
            if self.contains_color(*color) {
                continue;
            }

            match self.next_unused_pen() {
                None => counter_impossible += 1,
                Some(pen) => {
                    self.set(pen, *color);
                }
            }
        }

        (counter_added, counter_impossible)
    }

    /// Returns the list of inks contained in the palette with the border
    /// the number of inks corresponds to the number of available pens
    pub fn colors_with_border(&self) -> Vec<C> {
        let mut vec = Vec::with_capacity(17);
        for pen in 0..17 {
            let pen = Pen::from(pen);
            if self.contains_pen(pen) {
                vec.push(*self.get(&pen));
            }
        }
        vec
    }

    /// Returns the list of colors contained in the palette without taking into account the border
    /// the number of colors corresponds to the number of available pens
    pub fn colors(&self) -> Vec<C> {
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

    /// Get the color of the requested pen. Pen MUST be present
    pub fn get(&self, pen: &Pen) -> &C {
        match self.values.get(pen) {
            Some(color) => color,
            None => panic!("Wrong pen {pen:?}")
        }
    }

    pub fn safe_get(&self, pen: &Pen) -> Option<&C> {
        self.values.get(pen)
    }

    pub fn get_with_default<'a>(&'a self, pen: &'a Pen, default: &'a C) -> &'a C {
        match self.values.get(pen) {
            Some(color) => color,
            None => default
        }
    }

    // Get the color of the border
    pub fn get_border(&self) -> &C {
        self.values.get(&Pen::from(16)).expect("Border unavailable")
    }

    /// Change the color of the specified pen
    pub fn set<P: Into<Pen>, C2: Into<C>>(&mut self, pen: P, color: C2) {
        self.values.insert(pen.into(), color.into());
    }

    pub fn set_border(&mut self, color: C) {
        self.values.insert(Pen::from(16), color);
    }

    /// Get the pen that corresponds to the required color.
    /// Color 16 (border) is never tested
    pub fn get_pen_for_color<I: Into<C>>(&self, expected: I) -> Option<Pen> {
        let color: C = expected.into();
        self.values
            .iter()
            .filter(|&(&p, _)| p.number() != 16)
            .filter(|&(&_p_, &c)| c == color)
            .min()
            .map(|(p, _i)| *p)
    }

    /// Returns true if the palette contains the color in one of its pens (except border)
    pub fn contains_color(&self, expected: C) -> bool {
        self.get_pen_for_color(expected).is_some()
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


    /// This palette, as something that knows which machine it is for.
    pub fn into_any(self) -> AnyPalette {
        C::into_any_palette(self)
    }

    pub fn nb_pens_used(&self) -> usize {
        self.values.len()
    }


    pub fn to_ansi_string(&self) -> String {
        self.values
            .iter()
            .sorted_by(|a, b| Ord::cmp(&a.0.number(), &b.0.number()))
            .map(|(p, c)| {
                let color = c.owo_color();
                format!("{:<2} => {} {}", p.number(), "   ".on_color(color), c)
            })
            .join("\n")
    }

      pub fn is_plus(&self) -> bool {
        C::is_plus()
    }
}


impl From<&Palette<Ink>> for Vec<u8> {
    fn from(val: &Palette<Ink>) -> Self {
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

impl From<Palette<Ink>> for Vec<u8> {
    fn from(val: Palette<Ink>) -> Self {
        (&val).into()
    }
}

#[allow(missing_docs)]
impl Palette<Ink> {
    pub fn to_vec(&self) -> Vec<u8> {
        self.into()
    }
}

/// Represents a palette that can be read-only by construction or updatable
#[derive(Clone, Debug)]
pub struct LockablePalette<C: AmstradColor> {
    pal: Palette<C>,
    locked: bool
}

impl<C: AmstradColor> From<LockablePalette<C>> for Palette<C> {
    fn from(val: LockablePalette<C>) -> Self {
        val.pal
    }
}

impl<C: AmstradColor> From<&LockablePalette<C>> for Palette<C> {
    fn from(val: &LockablePalette<C>) -> Self {
        val.pal.clone()
    }
}

impl<C: AmstradColor> Deref for LockablePalette<C> {
    type Target = Palette<C>;

    fn deref(&self) -> &Self::Target {
        self.as_palette()
    }
}

impl<C: AmstradColor> LockablePalette<C> {
    /// Build a read-only palette
    pub fn locked(pal: Palette<C>) -> Self {
        Self { pal, locked: true }
    }

    /// Build a modifiable possibly non-empty palette
    pub fn unlocked(pal: Palette<C>) -> Self {
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
    pub fn as_palette_mut(&mut self) -> Option<&mut Palette<C>> {
        if self.is_unlocked() {
            Some(&mut self.pal)
        }
        else {
            None
        }
    }

    pub fn as_palette(&self) -> &Palette<C> {
        &self.pal
    }

    #[inline]
    pub fn into_palette(&self) -> Palette<C> {
        self.into()
    }
}



/// A palette whose machine is only known at runtime.
///
/// The conversion pipeline is monomorphised - `Palette<Ink>` for the Gate
/// Array, `Palette<AsicColor>` for the Plus - because the two are genuinely
/// different colour spaces, not one with a flag. But the *command line* only
/// learns which is meant when it reads `--pen0` or `--colb0`/`--kit`, so
/// something has to carry that choice from there to the code that acts on it.
/// This is that something: one runtime seam, at the container, delegating to
/// the concrete palettes underneath.
#[derive(Clone, Debug)]
pub enum AnyLockablePalette {
    GateArray(LockablePalette<Ink>),
    Asic(LockablePalette<AsicColor>)
}

impl AnyLockablePalette {
    pub fn as_palette(&self) -> AnyPaletteRef<'_> {
        match self {
            AnyLockablePalette::GateArray(p) => AnyPaletteRef::GateArray(p.as_palette()),
            AnyLockablePalette::Asic(p) => AnyPaletteRef::Asic(p.as_palette())
        }
    }

    pub fn as_palette_mut(&mut self) -> Option<AnyPaletteRefMut<'_>> {
        match self {
            AnyLockablePalette::GateArray(p) => {
                p.as_palette_mut().map(AnyPaletteRefMut::GateArray)
            },
            AnyLockablePalette::Asic(p) => p.as_palette_mut().map(AnyPaletteRefMut::Asic)
        }
    }

    pub fn into_palette(self) -> AnyPalette {
        match self {
            AnyLockablePalette::GateArray(p) => AnyPalette::GateArray(p.into_palette()),
            AnyLockablePalette::Asic(p) => AnyPalette::Asic(p.into_palette())
        }
    }

    pub fn is_locked(&self) -> bool {
        match self {
            AnyLockablePalette::GateArray(p) => p.is_locked(),
            AnyLockablePalette::Asic(p) => p.is_locked()
        }
    }

    pub fn is_unlocked(&self) -> bool {
        !self.is_locked()
    }

    /// The Gate Array palette, or `None` for a Plus one.
    ///
    /// Named rather than a bare `match` at every call site, because "this code
    /// path is Gate Array only" is a real, recurring statement while the
    /// conversion pipeline is monomorphised at `Ink`.
    pub fn gate_array(&self) -> Option<&LockablePalette<Ink>> {
        match self {
            AnyLockablePalette::GateArray(p) => Some(p),
            AnyLockablePalette::Asic(_) => None
        }
    }

    pub fn into_gate_array(self) -> Option<LockablePalette<Ink>> {
        match self {
            AnyLockablePalette::GateArray(p) => Some(p),
            AnyLockablePalette::Asic(_) => None
        }
    }

    pub fn asic(&self) -> Option<&LockablePalette<AsicColor>> {
        match self {
            AnyLockablePalette::GateArray(_) => None,
            AnyLockablePalette::Asic(p) => Some(p)
        }
    }

    /// What to call this palette in a message to the user.
    pub fn machine(&self) -> &'static str {
        match self {
            AnyLockablePalette::GateArray(_) => "Amstrad CPC (Gate Array)",
            AnyLockablePalette::Asic(_) => "Amstrad Plus (ASIC)"
        }
    }
}

impl From<LockablePalette<Ink>> for AnyLockablePalette {
    fn from(p: LockablePalette<Ink>) -> Self {
        Self::GateArray(p)
    }
}

impl From<LockablePalette<AsicColor>> for AnyLockablePalette {
    fn from(p: LockablePalette<AsicColor>) -> Self {
        Self::Asic(p)
    }
}

#[derive(Clone, Debug)]
pub enum AnyPalette {
    GateArray(Palette<Ink>),
    Asic(Palette<AsicColor>)
}

impl AnyPalette {
    /// The bytes a display routine writes to the hardware.
    ///
    /// Two very different things share this name because they answer the same
    /// question - "what do I poke to make these colours appear" - and the
    /// caller emitting the Z80 has to branch on the machine anyway.
    pub fn gate_array_bytes(&self) -> Option<[u8; Pen::NB_PENS as usize]> {
        match self {
            // A converted image often uses fewer than 16 pens; the hardware
            // still wants a value for each, and black is the one that shows
            // nothing.
            AnyPalette::GateArray(p) => Some(p.to_gate_array_with_default(Ink::BLACK)),
            AnyPalette::Asic(_) => None
        }
    }

    /// The 32 bytes of an ASIC palette, in `.kit` order - what gets copied to
    /// `&6400`.
    pub fn asic_bytes(&self) -> Option<[u8; 32]> {
        match self {
            AnyPalette::GateArray(_) => None,
            AnyPalette::Asic(p) => {
                let mut bytes = [0; 32];
                // A converted image often uses fewer than 16 pens. The ASIC
                // still reads all 32 bytes, so the unused ones are black -
                // the same choice `to_gate_array_with_default` makes.
                let black = AsicColor::default();
                for pen in 0..Pen::NB_PENS.min(16) {
                    let [high, low] = p.get_with_default(&Pen::from(pen), &black).to_bytes();
                    bytes[pen as usize * 2] = high;
                    bytes[pen as usize * 2 + 1] = low;
                }
                Some(bytes)
            }
        }
    }

    pub fn gate_array(&self) -> Option<&Palette<Ink>> {
        match self {
            AnyPalette::GateArray(p) => Some(p),
            AnyPalette::Asic(_) => None
        }
    }

    pub fn asic(&self) -> Option<&Palette<AsicColor>> {
        match self {
            AnyPalette::GateArray(_) => None,
            AnyPalette::Asic(p) => Some(p)
        }
    }

    pub fn machine(&self) -> &'static str {
        match self {
            AnyPalette::GateArray(_) => "Amstrad CPC (Gate Array)",
            AnyPalette::Asic(_) => "Amstrad Plus (ASIC)"
        }
    }
}

impl From<Palette<Ink>> for AnyPalette {
    fn from(p: Palette<Ink>) -> Self {
        Self::GateArray(p)
    }
}

impl From<Palette<AsicColor>> for AnyPalette {
    fn from(p: Palette<AsicColor>) -> Self {
        Self::Asic(p)
    }
}

pub enum AnyPaletteRef<'a> {
    GateArray(&'a Palette<Ink>),
    Asic(&'a Palette<AsicColor>)
}

pub enum AnyPaletteRefMut<'a> {
    GateArray(&'a mut Palette<Ink>),
    Asic(&'a mut Palette<AsicColor>)
}

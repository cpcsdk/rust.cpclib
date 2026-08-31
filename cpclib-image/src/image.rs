#![allow(clippy::needless_range_loop)]

use std::collections::HashSet;

use anyhow::{self, Context};
use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use image as im;
use owo_colors::{DynColors, OwoColorize};

use crate::asic::AsicColor;
use crate::color::AmstradColor;
use crate::ga::*;
use crate::pixels;
use crate::pixels::bytes_to_pens;

/// Screen mode
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    /// Mode 0 - 16 colors
    Zero,
    /// Mode 1 - 4 colors
    One,
    /// Mode 2 - 2 colors
    Two,
    /// Mode 3 - 4 colors / same resolution than Mode 0
    Three
}

impl From<u8> for Mode {
    fn from(val: u8) -> Self {
        match val {
            0 => Mode::Zero,
            1 => Mode::One,
            2 => Mode::Two,
            3 => Mode::Three,
            _ => panic!("{val} is not a valid mode.")
        }
    }
}

#[allow(missing_docs)]
impl Mode {
    /// Return the maximum number of colors for the current mode (without using rasters)
    pub fn max_colors(&self) -> usize {
        match self {
            Mode::Zero => 16,
            Mode::One | Mode::Three => 4,
            Mode::Two => 2
        }
    }

    /// Return the number of pixels encode by one byte in the given mode
    pub fn nb_pixels_per_byte(&self) -> usize {
        match self {
            Mode::Zero | Mode::Three => 2,
            Mode::One => 4,
            Mode::Two => 8
        }
    }

    pub fn nb_pixels_for_bytes_width(&self, width: usize) -> usize {
        width * self.nb_pixels_per_byte()
    }

    pub fn nb_bytes_for_pixels_width(self, width: usize) -> usize {
        let extra = if !width.is_multiple_of(self.nb_pixels_per_byte()) {
            1
        }
        else {
            0
        };
        width / self.nb_pixels_per_byte() + extra
    }

    pub fn nb_bytes_for_char(&self) -> usize {
        match self {
            Mode::Zero | Mode::Three => 4,
            Mode::One => 2,
            Mode::Two => 1
        }
    }
}

/// Conversion rules
#[derive(Copy, Clone, Debug)]
pub enum ConversionRule {
    /// All pixels are used
    AnyModeUseAllPixels,
    /// One pixel out of two is skiped (used for mode0 pictures where the graphician has doubled each pixel)
    ZeroSkipOddPixels
}

/// Browse the image and returns the list of colors
#[allow(unused)]
fn get_unique_colors(img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>) -> HashSet<im::Rgb<u8>> {
    let mut set = HashSet::new();
    for pixel in img.pixels() {
        set.insert(*pixel);
    }
    set
}

/// Browse the image and returns the palette to use
#[allow(unused)]
fn extract_palette<C: AmstradColor>(img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>) -> Palette<C> {
    let colors = get_unique_colors(img);
    let mut p = Palette::<C>::empty();

    assert!(colors.len() <= 16);

    for (idx, color) in colors.iter().enumerate() {
        let color = *color;
        p.set(Pen::from(idx as u8), C::from(color))
    }

    p
}

/// Encode the raw array of Pens in an array of CPC bytes encoded for the right screen mode
fn encode(pens: &[Vec<Pen>], mode: Mode, missing_pen: Option<Pen>) -> Vec<Vec<u8>> {
    let mut rows = Vec::new();
    for input_row in pens.iter() {
        let row = {
            if let Some(replacement) = missing_pen {
                match mode {
                    Mode::Zero => {
                        pixels::mode0::pens_to_vec_with_replacement(input_row, replacement)
                    },
                    Mode::One => {
                        pixels::mode1::pens_to_vec_with_replacement(input_row, replacement)
                    },
                    Mode::Two => {
                        pixels::mode2::pens_to_vec_with_replacement(input_row, replacement)
                    },
                    _ => panic!("Unimplemented yet ...")
                }
            }
            else {
                match mode {
                    Mode::Zero => pixels::mode0::pens_to_bytes_with_crop(input_row),
                    Mode::One => pixels::mode1::pens_to_bytes_with_crop(input_row),
                    Mode::Two => pixels::mode2::pens_to_bytes_with_crop(input_row),
                    _ => panic!("Unimplemented yet ...")
                }
            }
        };
        rows.push(row);
    }

    rows
}

/// Build a new screen line that reprents line1 in mode 0 and line2 in mode3
fn merge_mode0_mode3(line1: &[u8], line2: &[u8]) -> Vec<u8> {
    assert_eq!(line1.len(), line2.len());

    line1
        .iter()
        .zip(line2.iter())
        .map(|(&u1, &u2)| {
            let [p10, p11] = pixels::mode0::byte_to_pens(u1);
            let [p20, p21] = pixels::mode0::byte_to_pens(u2);

            let p0 = pixels::mode0::mix_mode0_mode3(p10, p20);
            let p1 = pixels::mode0::mix_mode0_mode3(p11, p21);

            pixels::mode0::pens_to_byte(p0, p1)
        })
        .collect::<Vec<u8>>()
}

// Convert inks to pens
fn colors_to_pens<C: AmstradColor>(colors: &[Vec<C>], p: &Palette<C>) -> Vec<Vec<Pen>> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
    let iter = colors.par_iter();
    #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
    let iter = colors.iter();

    iter.map(|line| {
        line.iter()
            .map(|color| {
                p.get_pen_for_color(*color).unwrap_or_else(|| {
                    panic!("Unable to find a correspondance for color {color:?} in given palette {p:?}")
                })
            })
            .collect::<Vec<Pen>>()
    })
    .collect::<Vec<_>>()
}

#[deprecated(note = "Use colors_to_pens instead")]
#[allow(unused)]
fn inks_to_pens(inks: &[Vec<Ink>], p: &Palette<Ink>) -> Vec<Vec<Pen>> {
    colors_to_pens(inks, p)
}

/// A ColorMatrix represents an image through a list of Inks.
/// It has no real meaning in CPC world but can be used for image transformaton
/// There is no mode information
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColorMatrix<C: AmstradColor> {
    /// List of inks
    data: Vec<Vec<C>>
}

impl<C: AmstradColor> From<Vec<Vec<C>>> for ColorMatrix<C> {
    fn from(data: Vec<Vec<C>>) -> Self {
        ColorMatrix { data }
    }
}

/// We have to choose a strategy when reducing the number of colors of an image.
/// This enumeration allows to set up them
#[derive(Debug, Copy, Clone)]
pub enum ColorConversionStrategy {
    /// Impossible colors are replace by the first possible ink
    ReplaceWrongColorByFirstColor,
    /// The color is replaced by the closest one
    ReplaceWrongColorByClosestInk,
    /// An error is generated
    Fail
}


impl<C:AmstradColor> ColorMatrix<C> {

    /// Build a representation of a palette
    pub fn from_palette(pal: &Palette<C>, ink_size: usize) -> Self {
        let height = ink_size;
        let width = ink_size * 17;

        // create a full black image
        let mut matrix = ColorMatrix::new(width, height);

        for p in Pen::PENS {
            let x = p.number() as usize * ink_size;
            let y = 0;

            let i = pal.get(&p);
            for w in 0..ink_size {
                for h in 0..ink_size {
                    matrix.set_color(x + w, y + h, *i);
                }
            }
        }

        matrix
    }

    pub fn from_screen(data: &[u8], bytes_width: usize, mode: Mode, palette: &Palette<C>) -> Self {
        let pixel_height = {
            let mut height = 0x4000 / bytes_width;
            while !height.is_multiple_of(8) {
                height -= 1;
            }
            height
        };

        Self::from_screen_at(data, 0xC000, bytes_width, pixel_height, 8, mode, palette)
    }

    /// Like [`Self::from_screen`], but the interleaved "every 8th raster
    /// line is +0x800" addressing is anchored at `base_address` instead of
    /// the hard-coded `0xC000`, and `pixel_height` is taken directly rather
    /// than derived from a fixed 16K budget - for decoding a live debugger
    /// memory read, where the screen may sit at any 16K-page start and the
    /// caller wants a specific height, not "as much as fits".
    ///
    /// A scrolled BASIC screen's start address is not necessarily 16K-bank
    /// aligned (reported live: a real CPC's own CRTC start address mid-
    /// program, R12=0x30/R13=0x88, landing at 0xC110), and the interleaved
    /// layout's own `+0x800`-per-subline term can then push the computed
    /// offset past a bank's own 0x4000 span. **The wrap stays inside
    /// whichever of the four independent 16K banks (0000-3FFF, 4000-7FFF,
    /// 8000-BFFF, C000-FFFF) `base_address` itself falls in, confirmed
    /// live**: the CRTC's own `MA`/`RA` counters are a 14-bit address with
    /// no idea about paging at all - the Gate Array places that 14-bit
    /// result into whichever bank R12's page-select bits chose, a bank that
    /// never changes mid-frame, so an overflowing `MA`/`RA` computation
    /// wraps back into the *same* bank rather than spilling into the next
    /// one (a computed offset that would land on 0x0006 lands on 0xC006 for
    /// a C000-based screen, not on the bare 0x0006 of a different bank
    /// entirely).
    ///
    /// **This reverses an earlier version of this same fix**, which wrapped
    /// at the full 64K address space instead (crossing banks freely) after
    /// live-testing one specific address (0xC5E0, from
    /// `cpclib-dap/tests/graphics/hello/`) against that fixture's own
    /// memory bytes and a WinAPE screen capture: the byte at the bank-
    /// confined candidate (0xC420) read as a real, non-background 0xE0,
    /// while the cross-bank candidate (0x0420) read 0x00 and looked clean
    /// against the capture. That comparison is not itself explained by the
    /// bank-confined formula reinstated here - flagged rather than silently
    /// dropped, since the earlier finding was real data, not a guess. Two
    /// honest candidates for the discrepancy: the source screenshot's own
    /// alignment was never guaranteed pixel-exact (flagged as a real risk
    /// from the very first fixture this feature was built against), or
    /// 0xC420 genuinely held live (non-screen) data that only *looked*
    /// like garbling by coincidence. Re-confirming against a live emulator
    /// - this crate's own tests cannot, `hello`/`blight` are static
    /// fixtures - is the way to actually settle it if it resurfaces.
    /// `data` is still addressed modulo its own length as a safety bound,
    /// but every real caller passes the full 64K address space
    /// (`data[0]` = real address 0x0000), not just one bank.
    ///
    /// `lines_per_char_row` replaces what used to be a hard-coded `8` for
    /// when `MA` (character position) itself advances to the next row - the
    /// Gate Array combines `MA` and `RA` (raster-within-row) as `address =
    /// MA*2 + RA*0x800 + base`, and how many `RA` values occur before `MA`
    /// advances is the CRTC's own `R9 + 1`, not always 8. `RA` itself,
    /// though, is a 3-bit field in this address path on real hardware
    /// regardless of how tall `R9` configures a row to actually be -
    /// reported live: a row configured taller than 8 lines does not reach
    /// new addresses past line 7, it repeats them - so only the low 3 bits
    /// of the within-row line number ever reach `RA*0x800`, even though the
    /// full `lines_per_char_row` still governs when `MA` advances.
    pub fn from_screen_at(
        data: &[u8],
        base_address: usize,
        bytes_width: usize,
        pixel_height: usize,
        lines_per_char_row: usize,
        mode: Mode,
        palette: &Palette<C>
    ) -> Self {
        let _pixel_width = mode.nb_pixels_for_bytes_width(bytes_width);
        let space_size = data.len();
        let lines_per_char_row = lines_per_char_row.max(1);

        // Corrected against real hardware, reported live: the CRTC's own MA
        // (character position) and RA (raster-within-row) counters combine
        // into a 14-bit address, and the CRTC itself has no idea about
        // paging at all - the Gate Array places that 14-bit result into
        // whichever of the four independent 16K banks (0000-3FFF,
        // 4000-7FFF, 8000-BFFF, C000-FFFF) `base_address`'s own top two
        // bits select, and that bank never changes mid-frame. So MA/RA
        // arithmetic wraps at 0x4000 (one bank), confined to it, never
        // spilling into the next one - a screen based at 0xC5E0 can only
        // ever read 0xC000-0xFFFF, and an overflowing computation wraps
        // back into that same range (e.g. an overflow that would land on
        // 0x0006 lands on 0xC006 instead), not into page 0.
        //
        // `RA` specifically is 3 bits wide in this address path - real CRTC
        // hardware accepts `R9` values that configure a character row
        // taller than 8 lines, but the extra lines do not reach new
        // addresses: `RA` wraps at 8 regardless of how tall the row is
        // configured to be, repeating the same 8 raster lines' worth of
        // addresses again for the remainder of the row. `lines_per_char_row`
        // still governs when `MA` itself advances to the next row (the
        // *real*, possibly-taller-than-8 row height), only the term this
        // wrap feeds into `RA*0x800` is narrowed.
        let page_base = base_address & !0x3FFFusize;
        let offset_in_page = base_address & 0x3FFF;

        (0..pixel_height)
            .map(|line| {
                let row = line / lines_per_char_row;
                let ra = (line % lines_per_char_row) % 8;
                let offset_in_page = (offset_in_page + row * bytes_width + ra * 0x800) % 0x4000;
                let line_bytes: Vec<u8> = (0..bytes_width)
                    .map(|col| {
                        let byte_offset = (offset_in_page + col) % 0x4000;
                        data[(page_base + byte_offset) % space_size]
                    })
                    .collect();
                Self::line_bytes_to_inks(&line_bytes, mode, palette)
            })
            .collect::<Vec<_>>()
            .into()
    }

    /// Shared by [`Self::from_screen_at`] and [`Self::from_linear_memory`] -
    /// the two encodings differ only in how a line's source address is
    /// computed, never in how its bytes decode into pens and then inks.
    fn line_bytes_to_inks(line_bytes: &[u8], mode: Mode, palette: &Palette<C>) -> Vec<C> {
        line_bytes
            .iter()
            .flat_map(|b| pixels::byte_to_pens(*b, mode))
            .map(|pen| palette.get(&pen).clone())
            .collect()
    }

    /// The other of WinAPE's own two "browse memory as pixels" encodings -
    /// "CPC" there, next to "Screen" (what [`Self::from_screen_at`]
    /// implements). No CRTC interleaving at all: `bytes_width` bytes read,
    /// then the very next `bytes_width` bytes, and so on - straight,
    /// sequential memory, top to bottom, left to right, wrapped at the full
    /// 64K address space rather than confined to one 16K bank (there is no
    /// real CRTC/Gate Array display behaviour being modelled here, so
    /// there is no bank to stay confined to - this is a raw memory scan,
    /// looking for a repeating structure whose real shape is not yet
    /// known). Bytes still decode via the chosen CPC screen `mode`
    /// (0-3) exactly as `from_screen_at` does - only the *addressing*
    /// differs between the two encodings, never the pixel decode.
    pub fn from_linear_memory(
        data: &[u8],
        base_address: usize,
        bytes_width: usize,
        pixel_height: usize,
        mode: Mode,
        palette: &Palette<C>
    ) -> Self {
        let space_size = data.len();

        (0..pixel_height)
            .map(|line| {
                let data_address = (base_address + line * bytes_width) % space_size;
                let line_bytes: Vec<u8> = (0..bytes_width)
                    .map(|col| data[(data_address + col) % space_size])
                    .collect();
                Self::line_bytes_to_inks(&line_bytes, mode, palette)
            })
            .collect::<Vec<_>>()
            .into()
    }

    pub fn from_sprite(data: &[u8], pixels_width: u16, mode: Mode, palette: &Palette<C>) -> Self {
        let width = mode.nb_bytes_for_pixels_width(pixels_width as _);

        // convert it
        data.chunks_exact(width)
            .map(|line| {
                // build lines of pen
                let line = line.iter();
                line.flat_map(|b| pixels::byte_to_pens(*b, mode))
                    .collect::<Vec<crate::ga::Pen>>()
            })
            .map(move |pens| {
                // build lines of inks
                pens.iter()
                    .map(|pen| palette.get(pen))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .into()
    }
}

#[allow(missing_docs)]
impl<C: AmstradColor> ColorMatrix<C> {
    /// Create a new empty color matrix for the given dimensions
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![vec![C::default(); width]; height]
        }
    }

    pub fn to_ansi_string(&self) ->String{
        self.data.iter().map(|line| {
            line.iter().map(|ink| {
                let color = ink.owo_color();
                format!("{}", "   ".on_color(color))
            }).join("")
        }).join("\n")
    }

    /// The matrix represents both the mask (with an unexpected color), and the sprite (<ith the expected color).
    /// This method returns two matrices:
    /// - The mask where bright white stands for pixels of the sprite and black stands for the pixels of the background
    /// - The sprite where the background is replaced by a selected ink (Ideally the one that will be considered as being pen 0)
    pub fn extract_mask_and_sprite(
        &self,
        mask_ink: impl Into<C>,
        replacement_ink: impl Into<C>
    ) -> (Self, Self) {
        let mask_ink = mask_ink.into();
        let replacement_ink = replacement_ink.into();

        let mut mask_data = self.clone();
        mask_data.convert_to_mask(mask_ink);

        let mut sprite_data = self.clone();
        sprite_data.replace_color(mask_ink, replacement_ink);

        (mask_data, sprite_data)
    }


    /// Destroy the image to build the mask according to the background ink
    pub fn convert_to_mask(&mut self, mask: C) -> &mut Self {
        self.data.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|ink| {
                *ink = if *ink == mask {
                    C::mask_background()
                }
                else {
                    C::mask_foreground()
                }
            })
        });
        self
    }


    /// Exchange all the occurrences of `from` Ink with `to` ink
    pub fn replace_color(&mut self, from: C, to: C) -> &mut Self {
        self.data.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|ink| {
                if *ink == from {
                    *ink = to;
                }
            })
        });
        self
    }

    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a new ColorMatrix that encodes a new image full of black
    pub fn empty_like(&self) -> Self {
        Self {
            data: vec![vec![C::black(); self.width() as usize]; self.height() as usize]
        }
    }

    /// Double the width (usefull for chunky conversions)
    #[allow(clippy::needless_range_loop, clippy::identity_op)]
    pub fn double_horizontally(&mut self) {
        // Create the doubled pixels
        let mut new_data =
            vec![vec![C::black(); (2 * self.width()) as usize]; self.height() as usize];
        for x in 0..(self.width() as usize) {
            for y in 0..(self.height() as usize) {
                let color = self.get_color(x, y);
                new_data[y][x * 2 + 0] = *color;
                new_data[y][x * 2 + 1] = *color;
            }
        }

        // Set them in the right position
        std::mem::swap(&mut self.data, &mut new_data)
    }

    /// Double the height (each row printed twice) - the vertical half of the
    /// CPC's own pixel aspect ratio: a raster line is roughly twice as tall
    /// as it is wide compared to a Mode 2 dot, on every mode alike, unlike
    /// the horizontal ratio which is mode-dependent (see
    /// [`Self::double_horizontally`], called a mode-dependent number of times
    /// instead).
    #[allow(clippy::needless_range_loop, clippy::identity_op)]
    pub fn double_vertically(&mut self) {
        let mut new_data =
            vec![vec![C::black(); self.width() as usize]; (2 * self.height()) as usize];
        for y in 0..(self.height() as usize) {
            for x in 0..(self.width() as usize) {
                let color = self.get_color(x, y);
                new_data[y * 2 + 0][x] = *color;
                new_data[y * 2 + 1][x] = *color;
            }
        }
        std::mem::swap(&mut self.data, &mut new_data)
    }

    pub fn remove_odd_columns(&mut self) {
        // Create the doubled pixels
        let mut new_data =
            vec![vec![C::black(); (self.width() / 2) as usize]; self.height() as usize];
        for x in 0..((self.width() / 2) as usize) {
            for y in 0..(self.height() as usize) {
                let color = self.get_color(x * 2, y);
                new_data[y][x] = *color;
            }
        }

        // Set them in the right position
        std::mem::swap(&mut self.data, &mut new_data)
    }

    /// Get the height (in pixels) of the image
    /// TODO Use a trait for that
    pub fn height(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the color at the right position
    pub fn get_color(&self, x: usize, y: usize) -> &C {
        &self.data[y][x]
    }

    /// Set color
    pub fn set_color(&mut self, x: usize, y: usize, color: C) {
        self.data[y][x] = color;
    }

    /// Add a line within the image
    /// Panic if impossible
    pub fn add_line(&mut self, position: usize, line: &[C]) {
        assert_eq!(line.len(), self.width() as usize);
        self.data.insert(position, line.to_vec());
    }

    /// Returns a reference on the wanted line of inks
    pub fn get_line(&self, y: usize) -> &[C] {
        &self.data[y]
    }

    /// Return a mutable version of the line. Care needs to be taken in order to not destroy the data structure
    fn get_line_mut(&mut self, y: usize) -> &mut Vec<C> {
        &mut self.data[y]
    }

    /// Add a column within the image
    /// Panic if impossible
    pub fn add_column(&mut self, position: usize, column: &[C]) {
        assert_eq!(column.len(), self.height() as usize);
        for (row, color) in column.iter().enumerate() {
            self.get_line_mut(row).insert(position, *color);
        }
    }

    /// Build a vector of Colors that contains all the colors of the given column
    pub fn get_column(&self, x: usize) -> Vec<C> {
        self.data.iter().map(|line| line[x]).collect::<Vec<C>>()
    }

    /// Return a copy of the colors for the given window definition
    pub fn window(&self, start_x: usize, start_y: usize, width: usize, height: usize) -> Self {
        let selected_lines = &self.data[start_y..start_y + height];
        let window = selected_lines
            .iter()
            .map(|line| &line[start_x..start_x + width])
            .map(|line| {
                let mut new_line = Vec::with_capacity(line.len());
                new_line.extend_from_slice(line);
                new_line
            })
            .collect();
        Self { data: window }
    }

    /// Return the number of different colors in the image
    pub fn nb_colors(&self) -> usize {
        self.data.iter().flatten().unique().count()
    }

    /// Returns the palette used (as soon as there is less than the maximum number of colors for the requested mode)
    pub fn extract_palette(&self, mode: Mode) -> Palette<C> {
        self.extract_palette_with_hint(mode, LockablePalette::empty())
            .unwrap()
    }

    /// Return the palette used. The final palette is based on the hint one
    pub fn extract_palette_with_hint(
        &self,
        mode: Mode,
        mut hint: LockablePalette<C>
    ) -> Result<Palette<C>, String> {
        for &color in self.data.iter().flatten().unique().sorted() {
            if !hint.contains_color(color) {
                // here the palette does not contain the color, so we have to add it
                if hint.is_locked() {
                    return Err(format!(
                        "Palette is locked, it is not possible to add color {color}"
                    ));
                }

                let target_pen = hint.next_unused_pen_for_mode(mode);
                if let Some(target_pen) = target_pen {
                    hint.as_palette_mut().unwrap().set(target_pen, color);
                }
                else {
                    return Err(format!(
                        "Palette is full, it is not possible to add extra color {color}"
                    ));
                }
            }
            else {
                // the ink is already there, nothing has to be done
            }
        }

        Ok(hint.into())
    }

    /// Modify the image in order to keep the right amount of inks
    pub fn reduce_colors_for_mode(
        &mut self,
        mode: Mode,
        strategy: ColorConversionStrategy
    ) -> Result<(), anyhow::Error> {
        // Get the reduced palette
        let inks = self
            .data
            .iter()
            .flatten()
            .unique()
            .copied()
            .collect::<Vec<C>>();
        let max_count = mode.max_colors().min(inks.len());
        let inks = &inks[..max_count];

        self.reduce_colors_with(inks, strategy)
    }

    /// Modify the image in order to use only the provided palette
    pub fn reduce_colors_with(
        &mut self,
        inks: &[C],
        strategy: ColorConversionStrategy
    ) -> Result<(), anyhow::Error> {
        for y in 0..(self.height() as usize) {
            for x in 0..(self.width() as usize) {
                let ink = &mut self.data[y][x];
                if !inks.contains(ink) {
                    match strategy {
                        ColorConversionStrategy::ReplaceWrongColorByFirstColor => {
                            *ink = inks[0];
                        },
                        ColorConversionStrategy::ReplaceWrongColorByClosestInk => unimplemented!(),
                        ColorConversionStrategy::Fail => {
                            return Err(anyhow::anyhow!(
                                "{:?} not available in {:?} at [{}, {}]",
                                ink,
                                inks,
                                x,
                                y
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the width (in bytes) of the image
    /// TODO Use a trait for that
    pub fn width(&self) -> u32 {
        match self.height() {
            0 => 0,
            _ => self.data[0].len() as u32
        }
    }

    pub fn convert_from_fname(fname: &str, conversion: ConversionRule) -> anyhow::Result<Self> {
        let img = im::open(fname).with_context(|| format!("{fname} does not exists."))?;
        Ok(Self::convert(&img.to_rgb8(), conversion))
    }

    pub fn convert(
        img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>,
        conversion: ConversionRule
    ) -> Self {
        // Get destination image size
        let height = img.height();
        let width = {
            match conversion {
                ConversionRule::AnyModeUseAllPixels => img.width(),
                ConversionRule::ZeroSkipOddPixels => img.width() / 2
            }
        };

        // Make the pixels extraction
        let mut lines = Vec::with_capacity(height as usize);
        for y in 0..height {
            let src_y = y;
            let mut line = Vec::with_capacity(width as usize);
            for x in 0..width {
                let src_x = {
                    match conversion {
                        ConversionRule::AnyModeUseAllPixels => x,
                        ConversionRule::ZeroSkipOddPixels => x * 2
                    }
                };

                let src_color = img.get_pixel(src_x, src_y);
                let dest_ink = C::from(*src_color);

                // Add the current ink to the current line
                line.push(dest_ink);
            }
            // Add the current complete line to the current image
            lines.push(line);
        }

        // And create the sprite structure
        Self { data: lines }
    }

    /// Compute a difference map to see the problematic positions
    pub fn diff(&self, other: &Self) -> Self {
        // Create a map encoding a complete success
        let mut data = vec![vec![C::white(); other.width() as usize]; other.height() as usize];

        // Set the error positions
        for x in 0..(self.width() as usize) {
            for y in 0..(self.height() as usize) {
                if self.data[y][x] != other.data[y][x] {
                    data[y][x] = C::black();
                }
            }
        }

        // Return the object
        Self { data }
    }

    /// From a ColorMatrix computed with the diff method, returns the (x,y) coordinates having a difference
    pub fn diff_to_positions(&self) -> Vec<(usize, usize)> {
        let mut res = Vec::new();
        for x in 0..(self.width() as usize) {
            for y in 0..(self.height() as usize) {
                if self.data[y][x] == C::black() {
                    res.push((x, y));
                }
            }
        }
        res
    }

    /// Convert the buffer as an image
    pub fn as_image(&self) -> im::ImageBuffer<im::Rgb<u8>, Vec<u8>> {
        let mut buffer: im::ImageBuffer<im::Rgb<u8>, Vec<u8>> =
            im::ImageBuffer::new(self.width(), self.height());

        for x in 0..(self.width()) {
            for y in 0..(self.height()) {
                buffer.put_pixel(x, y, (*self.get_color(x as usize, y as usize)).into());
            }
        }

        buffer
    }

    /// [`Self::as_image`], encoded as PNG bytes in memory - for a caller
    /// that wants to hand the image to something other than the filesystem
    /// (e.g. a debugger webview, over a `data:` URI).
    pub fn as_png_bytes(&self) -> Result<Vec<u8>, im::ImageError> {
        let buffer = self.as_image();
        let mut bytes = Vec::new();
        buffer.write_to(&mut std::io::Cursor::new(&mut bytes), im::ImageFormat::Png)?;
        Ok(bytes)
    }

    /// Convert the matrix as a sprite, given the right mode and an optional palette
    pub fn as_sprite(
        &self,
        mode: Mode,
        palette: LockablePalette<C>,
        missing_pen: Option<Pen>
    ) -> Sprite<C> {

        println!("image\n{}\n", self.to_ansi_string());
        println!("provided palette\n{}\n", palette.to_ansi_string());


        // Extract the palette is not provided as an argument
        let palette = if palette.is_locked() {
            palette.into_palette()
        }
        else {
            self.extract_palette_with_hint(mode, palette).unwrap()
        };

        println!("obtained palette in as_sprite\n{}\n", palette.to_ansi_string());


        // Really make the conversion
        let pens = colors_to_pens(&self.data, &palette);

        // Build the sprite
        Sprite {
            mode: Some(mode),
            palette: Some(palette),
            data: encode(&pens, mode, missing_pen)
        }
    }

}

impl ColorMatrix<Ink> {


    /// Convert the matrix as a sprite in mode1. Pen 1/2/3 are changed at each line. Pen 0 is constant
    pub fn as_mode1_sprite_with_different_inks_per_line(
        &self,
        palette: &[(Ink, Ink, Ink, Ink)],
        dummy_palette: &Palette<Ink>,
        missing_pen: Option<Pen>
    ) -> Sprite<Ink> {
        // Build the matrix of pens
        let mut data: Vec<Vec<Pen>> = Vec::new();
        for y in 0..self.height() {
            let y = y as usize;

            // Build the palette for the current ink
            let line_palette = {
                let mut p = Palette::<Ink>::new(); // Palette full of 0
                p.set(Pen::from(0), palette[y].0);
                p.set(Pen::from(1), palette[y].1);
                p.set(Pen::from(2), palette[y].2);
                p.set(Pen::from(3), palette[y].3);
                p
            };

            // get the pens for the current line
            let pens = self
                .get_line(y)
                .iter()
                .map(|ink| -> Pen {
                    let pen = line_palette.get_pen_for_ink(*ink);
                    if let Some(pen) = pen {
                        pen
                    }
                    else {
                        // eprintln!("
                        // [ERROR] In line {}, pixel {} color ({:?}) is not in the palette {:?}. Background is used insted",
                        // y,
                        // x,
                        // ink,
                        // line_palette
                        // );
                        Pen::from(0)
                    } // If the color is not in the palette, use pen 0
                })
                .collect::<Vec<Pen>>();

            // Transform the pens in bytes
            data.push(pens);
        }

        let encoded_pixels = encode(&data, Mode::One, missing_pen);

        // Convert the matrix of pens as a sprite
        Sprite {
            mode: Some(Mode::One),
            palette: Some(dummy_palette.clone()),
            data: encoded_pixels
        }
    }

    /// Generate an iterator on the pixels
    pub fn inks(&self) -> Inks<'_, Ink> {
        Inks {
            image: self,
            x: 0,
            y: 0,
            width: self.width(),
            height: self.height()
        }
    }

    pub fn vstack(stack: &[Self]) -> Self {
        let max_width = stack.iter().map(|row| row.width()).max().unwrap() as usize;
        let tot_height = stack.iter().map(|row| row.height()).sum::<u32>() as usize;

        let mut matrix = Self::new(max_width, tot_height);
        let mut cumulative_height = 0;
        for row in stack.iter() {
            matrix.draw_matrix_at(0, cumulative_height, row);
            cumulative_height += row.height() as usize;
        }

        matrix
    }
}

impl<C: AmstradColor> ColorMatrix<C> {
    pub fn draw_matrix_at(&mut self, x: usize, y: usize, other: &Self) {
        for w in 0..(other.width() as usize) {
            for h in 0..(other.height() as usize) {
                let i = other.get_color(w, h);
                self.set_color(x + w, y + h, *i);
            }
        }
    }
}

/// Immutable ink iterator for generate (x, y, ink)
#[derive(Debug)]
pub struct Inks<'a, C: AmstradColor> {
    image: &'a ColorMatrix<C>,
    x: u32,
    y: u32,
    width: u32,
    height: u32
}

impl<C: AmstradColor> Iterator for Inks<'_, C> {
    type Item = (u32, u32, C);

    fn next(&mut self) -> Option<(u32, u32, C)> {
        if self.x >= self.width {
            self.x = 0;
            self.y += 1;
        }

        if self.y >= self.height {
            None
        }
        else {
            let ink = self.image.get_color(self.x as _, self.y as _);
            let i = (self.x, self.y, *ink);

            self.x += 1;

            Some(i)
        }
    }
}

/// Animation are stored in lists of ColorMatrices of same sze
#[derive(Debug)]
pub struct ColorMatrixList<C: AmstradColor>(Vec<ColorMatrix<C>>);

impl<C: AmstradColor> From<Vec<ColorMatrix<C>>> for ColorMatrixList<C> {
    fn from(src: Vec<ColorMatrix<C>>) -> Self {
        ColorMatrixList(src)
    }
}

impl<C: AmstradColor> From<&ColorMatrixList<C>> for Vec<ColorMatrix<C>> {
    fn from(val: &ColorMatrixList<C>) -> Self {
        val.0.clone()
    }
}

impl<C: AmstradColor> std::ops::Deref for ColorMatrixList<C> {
    type Target = Vec<ColorMatrix<C>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Defines potential constraints when automatically cropping the image
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum HorizontalCropConstraint {
    /// No constrain at all
    None,
    /// Consider we are working in a specific and screen mode and bytes must be full
    CompleteByteForMode(Mode)
}

/// Defines how cropping occurs horizontally
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum HorizontalCrop {
    /// Cropping only on right
    Right(HorizontalCropConstraint),
    /// Cropping only on left
    Left(HorizontalCropConstraint),
    /// Cropping on left and right
    Both(HorizontalCropConstraint, HorizontalCropConstraint),
    /// No horinzotnalropping
    None
}

/// Defines how cropping occurs vertically
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum VerticalCrop {
    /// Cropping only on top
    Top,
    /// Cropping only on botton
    Bottom,
    /// Cropping on top and bottom
    Both,
    /// No vertical cropping
    None
}

impl<C: AmstradColor> ColorMatrixList<C> {
    /// Provide a Vec version of the items
    pub fn to_vec(&self) -> Vec<ColorMatrix<C>> {
        self.into()
    }

    /// Animations are stored within GIF files.
    /// TODO allow over kind of image data
    pub fn convert_from_fname(fname: &str, conversion: ConversionRule) -> anyhow::Result<Self> {
        use fs_err::File;

        // Decode a gif into frames
        let input = File::open(fname)?;
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::Indexed);
        let mut decoder = options.read_info(input).unwrap();
        let mut screen = gif_dispose::Screen::new_decoder(&decoder);

        let mut matrix_list = ColorMatrixList(Vec::new());
        while let Some(frame) = decoder.read_next_frame()? {
            screen.blit_frame(frame)?;

            let content = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(
                screen.pixels_rgba().width() as u32,
                screen.pixels_rgba().height() as u32,
                screen
                    .pixels_rgba()
                    .buf()
                    .iter()
                    .flat_map(|pix| [pix.r, pix.g, pix.b].to_vec())
                    .collect::<Vec<u8>>()
            )
            .unwrap();

            matrix_list
                .0
                .push(ColorMatrix::convert(&content, conversion));
        }

        Ok(matrix_list)
    }

    /// Delegate the color reduction to the underlying ColorMatrix objects
    pub fn reduce_colors_with(
        &mut self,
        colors: &[C],
        strategy: ColorConversionStrategy
    ) -> Result<(), anyhow::Error> {
        self.0
            .iter_mut()
            .try_for_each(|matrix| matrix.reduce_colors_with(colors, strategy))
    }

    /// Number of frames in the animation
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the list is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Assume there is one sprite at least and all of them have the same size
    pub fn width(&self) -> u32 {
        self.0[0].width()
    }

    /// Assume there is one sprite at least and all of them have the same size
    pub fn height(&self) -> u32 {
        self.0[0].height()
    }

    /// Convert each matrice as a sprite using the same conversion method
    pub fn as_sprites(
        &self,
        mode: Mode,
        palette: LockablePalette<C>,
        missing_pen: Option<Pen>
    ) -> SpriteList<C> {
        self.to_vec()
            .iter()
            .map(|matrix| matrix.as_sprite(mode, palette.clone(), missing_pen))
            .collect::<Vec<Sprite<C>>>()
            .into()
    }

    /// Crop each matrix in order to only keep the maximal window where at least one pixel change over the animation
    pub fn crop(&mut self, hor_conf: HorizontalCrop, vert_conf: VerticalCrop) -> Self {
        use std::collections::BTreeSet;

        // Collect the lines/row modified
        let (modified_x, modified_y) = {
            let mut modified_x = BTreeSet::new();
            let mut modified_y = BTreeSet::new();

            for (mata, matb) in self.0.iter().tuple_windows() {
                let diff = mata.diff(matb);
                let diff_coords = diff.diff_to_positions();

                diff_coords.iter().for_each(|(x, y)| {
                    modified_x.insert(*x);
                    modified_y.insert(*y);
                });
            }

            (
                modified_x.iter().map(|x| *x as u32).collect::<Vec<_>>(),
                modified_y.iter().map(|y| *y as u32).collect::<Vec<_>>()
            )
        };

        // Make the croping on the left (first column to keep)
        let mut start_x = match hor_conf {
            HorizontalCrop::Both(..) | HorizontalCrop::Left(_) => {
                let mut current_x = 0;
                while current_x < self.width() - 1 && current_x < modified_x[0] {
                    current_x += 1;
                }
                current_x
            },
            _ => 0
        } as usize;

        // Make the cropping on the right (last column to keep)
        let mut stop_x = match hor_conf {
            HorizontalCrop::Both(..) | HorizontalCrop::Right(_) => {
                let mut current_x = self.width() - 1;
                while current_x > 0 && current_x > *modified_x.last().unwrap() {
                    current_x -= 1;
                }
                current_x
            },
            _ => self.width() - 1
        } as usize;

        // Make the cropping to the top
        let start_y = match vert_conf {
            VerticalCrop::Both | VerticalCrop::Top => {
                let mut current_y = 0;
                while current_y < self.height() - 1 && current_y < modified_y[0] {
                    current_y += 1;
                }
                current_y
            },
            _ => 0
        } as usize;

        // Make the cropping to the bottom
        let stop_y = match vert_conf {
            VerticalCrop::Both | VerticalCrop::Bottom => {
                let mut current_y = self.height() - 1;
                while current_y > 0 && current_y > *modified_y.last().unwrap() {
                    current_y -= 1;
                }
                current_y
            },
            _ => self.height() - 1
        } as usize;

        // Ensure horizontal start constraint is respected
        match hor_conf {
            HorizontalCrop::Left(HorizontalCropConstraint::CompleteByteForMode(ref mode))
            | HorizontalCrop::Both(HorizontalCropConstraint::CompleteByteForMode(ref mode), _) => {
                while !start_x.is_multiple_of(mode.nb_pixels_per_byte()) {
                    start_x -= 1;
                }
            },
            _ => {}
        }

        // Ensure horizontal stop contraint is respected
        match hor_conf {
            HorizontalCrop::Right(HorizontalCropConstraint::CompleteByteForMode(ref mode))
            | HorizontalCrop::Both(_, HorizontalCropConstraint::CompleteByteForMode(ref mode)) => {
                while !(stop_x + 1).is_multiple_of(mode.nb_pixels_per_byte()) {
                    stop_x += 1;
                }
            },
            _ => {}
        }

        // Return the selected window
        self.window(start_x, start_y, stop_x - start_x + 1, stop_y - start_y + 1)
    }

    /// Apply the window operator on each ColorMatrix
    pub fn window(&self, start_x: usize, start_y: usize, width: usize, height: usize) -> Self {
        self.to_vec()
            .iter()
            .map(|matrix| matrix.window(start_x, start_y, width, height))
            .collect::<Vec<ColorMatrix<C>>>()
            .into()
    }
}

/// List of sprites for animations
#[derive(Debug)]
pub struct SpriteList<C: AmstradColor>(Vec<Sprite<C>>);    

impl<C: AmstradColor> From<Vec<Sprite<C>>> for SpriteList<C> {
    fn from(src: Vec<Sprite<C>>) -> Self {
        SpriteList(src)
    }
}

impl<C: AmstradColor> std::ops::Deref for SpriteList<C> {
    type Target = Vec<Sprite<C>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A Sprite corresponds to a set of bytes encoded to the right CPC pixel format for a given
/// palette.
/// TODO Check why mode nad palette are optionnals. Force them if it is not mandatory to have htem
/// optionnal
#[derive(Debug)]
pub struct Sprite<C: AmstradColor> {
    /// Optional screen mode of the sprite
    pub(crate) mode: Option<Mode>,
    /// Optionnal palete of the sprite
    pub(crate) palette: Option<Palette<C>>,
    /// Content of the sprite
    pub(crate) data: Vec<Vec<u8>>
}

#[allow(missing_docs)]
impl<C: AmstradColor> Sprite<C> {
    pub fn from_pens(pens: &[Vec<Pen>], mode: Mode, palette: Option<Palette<C>>) -> Self {
        let data = pens
            .iter()
            .map(|line| crate::pixels::pens_to_bytes(line, mode))
            .collect();
        Sprite {
            data,
            mode: Some(mode),
            palette
        }
    }

    pub fn from_bytes(bytes: &[u8], bytes_width: usize, mode: Mode, palette: Palette<C>) -> Self {
        let pens: Vec<Vec<_>> = bytes
            .chunks(bytes_width)
            .map(|chunk| pixels::bytes_to_pens(chunk, mode).collect::<Vec<_>>())
            .collect();

        Self::from_pens(&pens, mode, Some(palette))
    }

    /// TODO Use TryFrom once in standard rust
    /// The conversion can only work if a palette and a mode is provided
    pub fn to_color_matrix(&self) -> Option<ColorMatrix<C>> {
        if self.mode.is_none() || self.palette.is_none() {
            return None;
        }

        let mut data = Vec::with_capacity(self.data.len());
        let p = self.palette.as_ref().unwrap();
        for line in &self.data {
            let inks = match self.mode {
                Some(Mode::Zero) | Some(Mode::Three) => {
                    line.iter()
                        .flat_map(|b: &u8| {
                            let pens = {
                                let mut pens = pixels::mode0::byte_to_pens(*b);
                                pens[0].limit(self.mode.unwrap());
                                pens[1].limit(self.mode.unwrap());
                                pens
                            };
                            vec![*p.get(&pens[0]), *p.get(&pens[1])]
                        })
                        .collect::<Vec<C>>()
                },

                Some(mode) => {
                    bytes_to_pens(line, mode)
                        .map(|pen| *p.get(&pen))
                        .collect_vec()
                },

                _ => unimplemented!()
            };
            data.push(inks);
        }

        Some(ColorMatrix { data })
    }

    /// Produce a linearized version of the sprite.
    pub fn to_linear_vec(&self) -> Vec<u8> {
        let size = self.height() * self.bytes_width();
        let mut bytes: Vec<u8> = Vec::with_capacity(size as usize);

        for y in 0..self.height() {
            bytes.extend_from_slice(&self.data[y as usize]);
        }

        bytes
    }

    /// Get the palette of the sprite
    pub fn palette(&self) -> Option<Palette<C>> {
        self.palette.clone()
    }

    pub fn set_palette(&mut self, palette: Palette<C>) {
        self.palette = Some(palette);
    }

    pub fn bytes(&self) -> &Vec<Vec<u8>> {
        &self.data
    }

    /// Get hte sprite Mode
    /// Cannot manage multimode sprites of course
    pub fn mode(&self) -> Option<Mode> {
        self.mode
    }

    /// Get the height (in pixels) of the image
    /// TODO Use a trait for that
    pub fn height(&self) -> u32 {
        self.data.len() as u32
    }

    /// Get the width (in bytes) of the image
    /// TODO Use a trait for that
    pub fn bytes_width(&self) -> u32 {
        match self.height() {
            0 => 0,
            _ => self.data[0].len() as u32
        }
    }

    /// Get the width in pixels of the image.
    /// The mode must be specified
    pub fn pixel_width(&self) -> u32 {
        match self.mode {
            None => panic!("Unable to get the pixel width when mode is not specified"),
            Some(mode) => mode.nb_pixels_per_byte() as u32 * self.bytes_width()
        }
    }

    /// Returns the byte at the right position and crash if it does not exists
    pub fn get_byte(&self, x: usize, y: usize) -> u8 {
        let line = &self.data[y];
        line[x]
    }

    /// Returns the byte at the right position if exists
    pub fn get_byte_safe(&self, x: usize, y: usize) -> Option<u8> {
        self.data.get(y).and_then(|v| v.get(x)).copied()
    }

    /// Returns the line of interest
    pub fn get_line(&self, y: usize) -> &[u8] {
        self.data[y].as_ref()
    }

    /// Convert an RGB image to a sprite that code the pixels
    /// XXX Since 2018-06-16, most of code is delagated to ColorMatrix => maybe some bugs has been
    /// added
    pub fn convert(
        img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>,
        mode: Mode,
        conversion: ConversionRule,
        palette: LockablePalette<C>,
        missing_pen: Option<Pen>
    ) -> Self {
        // Get the list of Inks that represent the image
        let matrix = ColorMatrix::convert(img, conversion);
        matrix.as_sprite(mode, palette, missing_pen)
    }

    pub fn convert_from_fname<P: AsRef<Utf8Path>>(
        fname: P,
        mode: Mode,
        conversion: ConversionRule,
        palette: LockablePalette<C>,
        missing_pen: Option<Pen>
    ) -> Result<Self, im::ImageError> {
        let img = im::open(fname.as_ref())?;
        Ok(Self::convert(
            &img.to_rgb8(),
            mode,
            conversion,
            palette,
            missing_pen
        ))
    }

    /// Apply a transformation function on each line
    /// It can change there size
    pub fn horizontal_transform<F>(&mut self, f: F)
    where F: Fn(&Vec<u8>) -> Vec<u8> {
        let mut transformed = self.data.iter().map(f).collect::<Vec<_>>();
        ::std::mem::swap(&mut transformed, &mut self.data);
    }

    pub fn as_image(&self) -> im::ImageBuffer<im::Rgb<u8>, Vec<u8>> {
        self.to_color_matrix().unwrap().as_image()
    }
}

/// Simple multimode sprite where each line can have its own resolution mode
/// The palette is assumed to be the same on all the lines
#[derive(Clone, Debug)]
#[allow(missing_docs, unused)]
pub struct MultiModeSprite<C: AmstradColor>  {
    mode: Vec<Mode>,
    palette: Palette<C>,
    data: Vec<Vec<u8>>
}

#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum MultiModeConversion {
    FirstHalfSecondHalf,
    OddEven
}

#[allow(missing_docs)]
impl<C: AmstradColor> MultiModeSprite<C> {
    /// Build an empty multimode sprite BUT provide the palette
    pub fn new(p: Palette<C>) -> Self {
        Self {
            palette: p,
            mode: Vec::new(), // Color modes for the real lines
            data: Vec::new()  // Data for texture lines (twice less than real ones
        }
    }

    pub fn bytes(&self) -> &Vec<Vec<u8>> {
        &self.data
    }

    pub fn height(&self) -> usize {
        self.data.len()
    }

    pub fn width(&self) -> usize {
        self.data[0].len()
    }

    /// Build a standard mode 0 sprite from a multimode sprite
    /// Bytes values will be strictly the same. However representation is loss (bytes supposed to
    /// be displayed in mode 1, 2, 3 will be represented in mode 0)
    /// The multimode sprite is consummed
    pub fn to_mode0_sprite(&self) -> Sprite<C> {
        Sprite {
            mode: Some(Mode::Zero),
            palette: Some(self.palette.clone()),
            data: self.data.clone()
        }
    }

    pub fn to_mode3_sprite(&self) -> Sprite<C> {
        Sprite {
            mode: Some(Mode::Three),
            palette: Some(self.palette.clone()),
            data: self.data.clone()
        }
    }

    /// Generate a multimode sprite that mixes mode 0 and mode 3 and uses only 4 colors
    #[allow(clippy::similar_names, clippy::identity_op)]
    pub fn mode0_mode3_mix_from_mode0(sprite: &Sprite<C>, conversion: MultiModeConversion) -> Self {
        // TODO check that there are only the first 4 inks used
        let p_orig = sprite.palette().unwrap();

        //  Build the specific palette for multimode
        let p = {
            let mut p = Palette::<C>::new();

            // First 4 inks are strictly the same
            for i in 0..4 {
                p.set(i, *p_orig.get(i.into()));
            }

            // The others depends on the bits kept in mode 0 or mode 4
            let lut = [
                (0, [5, 6, 7]),
                (1, [8, 10, 11]),
                (2, [12, 13, 15]),
                (3, [4, 9, 14])
            ];

            // Fill inks depending on the lut
            for (src, dsts) in &lut {
                dsts.iter().for_each(|dst| {
                    p.set(*dst, *p_orig.get((*src).into()));
                });
            }

            p
        };

        // Really makes the conversion of the lines
        let (modes, lines) = match conversion {
            MultiModeConversion::FirstHalfSecondHalf => {
                let sprite_height = sprite.height() as usize;
                let encoded_height = if sprite_height % 2 == 1 {
                    sprite_height / 2 + 1
                }
                else {
                    sprite_height / 2 + 0
                };

                let mut modes = Vec::with_capacity(sprite_height);
                let mut lines = Vec::with_capacity(encoded_height);

                // Create the vector of modes
                for i in 0..sprite_height {
                    let mode = if i < encoded_height {
                        Mode::Zero
                    }
                    else {
                        Mode::Three
                    };
                    modes.push(mode);
                }

                // Create the vector of lines
                for i in 0..encoded_height {
                    let line1 = &sprite.data[i + 0]; // always available
                    let line2 = sprite.data.get(i + encoded_height); // may be absent the very last time

                    let line = match line2 {
                        Some(line2) => merge_mode0_mode3(line1, line2),
                        None => line1.clone()
                    };

                    lines.push(line);
                }

                (modes, lines)
            },

            _ => unimplemented!()
        };

        Self {
            palette: p,
            mode: modes,
            data: lines
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorMatrix, Mode};
    use crate::ga::Ink;
    use crate::palette::Palette;

    /// Reported live: a real, scrolled BASIC screen (CRTC start address not
    /// 16K-page-aligned) rendered via `-sv` cut the bottom of the screen
    /// off ("Ready" and everything after it missing). Root cause: real CPC
    /// hardware wraps the interleaved "+0x800 per subline" address as
    /// plain 16-bit arithmetic, at the full 64K boundary - `from_screen_at`
    /// used to index a plain, non-wrapping slice instead, which either
    /// panicked (out of bounds) or forced a caller-side safety clamp that
    /// shrank the image to stay in bounds.
    ///
    /// **This crate's own first fix was itself wrong**: it wrapped within
    /// just the *page* (`base_address`'s own 16K region) rather than the
    /// full 64K space, on the theory that the CRTC's page-select bits are
    /// separate circuitry from the interleave offset. Live-verified wrong
    /// against a second real fixture
    /// (`cpclib-dap/tests/graphics/hello/`, address 0xC5E0): its own last
    /// on-screen line computes to raw address 0x10420, which the page-wrap
    /// theory placed at 0xC420 (a real, non-background byte, 0xE0) where
    /// the *correct* value - matching both a real screenshot and a WinAPE
    /// capture of the identical memory - is 0x00, only found by wrapping
    /// the full 16-bit address (0x10420 -> 0x0420) instead.
    ///
    /// This pins the *corrected* (full 64K) wrap down directly:
    /// `base_address` (0xFFF0) sits only 16 bytes before the very top of
    /// the address space, so line 1's own subline term alone (`1 * 0x800`)
    /// pushes the raw address past 0xFFFF - the exact shape of both real
    /// bugs. A marker byte placed at the address the *full-space* wrap
    /// should land on must be the one actually read, not a panic and not
    /// the (out of bounds, if it even ran) unwrapped position - nor the
    /// wrong, page-relative position the first fix would have used.
    #[test]
    fn from_screen_at_wraps_within_the_same_16k_bank_not_into_the_next_one() {
        let mut palette = Palette::<Ink>::new();
        palette.set(0u8, Ink::BLACK);
        palette.set(1u8, Ink::WHITE);

        let mut data = vec![0u8; 0x10000];
        let base_address = 0xFFF0usize; // bank 3: 0xC000-0xFFFF
        // Where line 1 (subline 1, `1 * 0x800`) wraps to: confined to the
        // *same* bank as `base_address` - (0xFFF0 & 0x3FFF) + 0x800,
        // wrapped at 0x4000 and re-based at 0xC000 - not the bank `base_
        // address`'s own raw arithmetic would spill into if left
        // unconfined (0xFFF0 + 0x800 = 0x107F0, which crosses out of
        // 16-bit range entirely, let alone out of bank 3).
        let offset_in_page = (base_address & 0x3FFF) + 0x800;
        let wrapped_offset = 0xC000 + (offset_in_page % 0x4000);
        assert_eq!(wrapped_offset, 0xC7F0);
        data[wrapped_offset] = 0xFF; // one full byte "on" - Mode::Two, 8 lit pixels

        let matrix = ColorMatrix::from_screen_at(&data, base_address, 1, 16, 8, Mode::Two, &palette);

        let line0_lit = (0..matrix.width()).any(|x| *matrix.get_color(x as usize, 0) == Ink::WHITE);
        let line1_lit = (0..matrix.width()).any(|x| *matrix.get_color(x as usize, 1) == Ink::WHITE);
        assert!(!line0_lit, "line 0 reads an untouched (zero) byte, must stay background");
        assert!(
            line1_lit,
            "line 1 must read the marker byte wrapped within the same 16K bank, not panic or miss it"
        );
    }

    /// Reported live: real CRTC hardware accepts an `R9` (maximum raster
    /// address) taller than 8 lines, but the address path's own `RA` field
    /// is only 3 bits wide - a row configured taller than 8 lines does not
    /// reach new addresses past line 7, it repeats them. `lines_per_char_row`
    /// = 16 here (an `R9` of 15) still governs when `MA` advances (once
    /// every 16 lines, not 8), but line 8 must read the *same* byte line 0
    /// did - not a new one 8*0x800 further on, and not (with the old,
    /// pre-fix `line % lines_per_char_row` feeding `RA*0x800` directly)
    /// skip the 8 lines in between as though they used addresses that were
    /// never really reachable.
    #[test]
    fn from_screen_at_wraps_the_raster_address_at_8_regardless_of_the_configured_row_height() {
        let mut palette = Palette::<Ink>::new();
        palette.set(0u8, Ink::BLACK);
        palette.set(1u8, Ink::WHITE);

        let mut data = vec![0u8; 0x10000];
        let base_address = 0xC000usize;
        data[base_address] = 0xFF; // line 0's own byte - Mode::Two, 8 lit pixels

        let matrix =
            ColorMatrix::from_screen_at(&data, base_address, 1, 16, 16, Mode::Two, &palette);

        let lit = |line: usize| {
            let m = &matrix;
            (0..m.width()).any(|x| *m.get_color(x as usize, line) == Ink::WHITE)
        };
        assert!(lit(0), "line 0 reads its own marker byte");
        assert!(
            lit(8),
            "line 8 (RA=8, wraps to RA=0) must read the same byte line 0 did, not a blank one"
        );
        assert!(!lit(1), "line 1 (RA=1) reads an untouched, still-blank byte");
    }

    /// WinAPE's "CPC" encoding, next to its "Screen" one: plain sequential
    /// bytes, no interleave, wrapped at the full 64K space rather than
    /// confined to one 16K bank.
    #[test]
    fn from_linear_memory_reads_sequential_bytes_wrapped_at_the_full_64k_space() {
        let mut palette = Palette::<Ink>::new();
        palette.set(0u8, Ink::BLACK);
        palette.set(1u8, Ink::WHITE);

        let mut data = vec![0u8; 0x10000];
        let base_address = 0xFFFEusize;
        data[0xFFFE] = 0xFF; // line 0, byte 0
        data[0xFFFF] = 0x00; // line 0, byte 1
        data[0x0000] = 0xFF; // line 1, byte 0 - only reachable by wrapping past 0xFFFF
        data[0x0001] = 0x00; // line 1, byte 1

        let matrix =
            ColorMatrix::from_linear_memory(&data, base_address, 2, 2, Mode::Two, &palette);

        let lit = |line: usize, x: usize| *matrix.get_color(x, line) == Ink::WHITE;
        assert!(lit(0, 0), "line 0 reads 0xFFFE, its own marker byte");
        assert!(
            lit(1, 0),
            "line 1 must wrap straight past 0xFFFF to 0x0000, not stop or panic"
        );
    }

    #[test]
    fn test_masking() {
        let fg1 = Ink::BLUE;
        let fg2 = Ink::SKY_BLUE;
        let fg3 = Ink::PASTEL_BLUE;
        let fg4 = Ink::BRIGHT_BLUE;
        let bg_ = Ink::RED;
        let rep = Ink::BLACK;

        let sprite_with_mask = ColorMatrix {
            data: vec![
                vec![bg_, fg2, fg3, fg4],
                vec![bg_, bg_, fg1, fg2],
                vec![bg_, bg_, bg_, fg3],
                vec![bg_, bg_, bg_, fg4],
                vec![bg_, bg_, bg_, bg_],
            ]
        };

        let (mask, sprite) = sprite_with_mask.extract_mask_and_sprite(bg_, rep);

        assert_eq!(
            sprite.data,
            vec![
                vec![rep, fg2, fg3, fg4],
                vec![rep, rep, fg1, fg2],
                vec![rep, rep, rep, fg3],
                vec![rep, rep, rep, fg4],
                vec![rep, rep, rep, rep],
            ]
        );

        assert_eq!(
            mask.data,
            vec![
                vec![Ink::BRIGHT_WHITE, Ink::BLACK, Ink::BLACK, Ink::BLACK],
                vec![Ink::BRIGHT_WHITE, Ink::BRIGHT_WHITE, Ink::BLACK, Ink::BLACK],
                vec![
                    Ink::BRIGHT_WHITE,
                    Ink::BRIGHT_WHITE,
                    Ink::BRIGHT_WHITE,
                    Ink::BLACK
                ],
                vec![
                    Ink::BRIGHT_WHITE,
                    Ink::BRIGHT_WHITE,
                    Ink::BRIGHT_WHITE,
                    Ink::BLACK
                ],
                vec![
                    Ink::BRIGHT_WHITE,
                    Ink::BRIGHT_WHITE,
                    Ink::BRIGHT_WHITE,
                    Ink::BRIGHT_WHITE
                ],
            ]
        );

        let mask2 = sprite_with_mask.clone().convert_to_mask(bg_).clone();
        let sprite2 = sprite_with_mask.clone().replace_color(bg_, rep).clone();

        assert_eq!(mask, mask2);
        assert_eq!(sprite, sprite2);
    }

    /// Every row printed twice, in place, with the width and the row order
    /// otherwise untouched - the vertical half of the CPC pixel-aspect-ratio
    /// stretch `-sv`'s screen viewer applies on every mode alike.
    #[test]
    fn double_vertically_repeats_each_row_immediately_after_itself() {
        let mut matrix: ColorMatrix<Ink> = vec![
            vec![Ink::BLACK, Ink::WHITE],
            vec![Ink::WHITE, Ink::BLACK]
        ]
        .into();

        matrix.double_vertically();

        assert_eq!(matrix.width(), 2);
        assert_eq!(matrix.height(), 4);
        assert_eq!(
            *matrix.get_color(0, 0), Ink::BLACK
        );
        assert_eq!(*matrix.get_color(0, 1), Ink::BLACK);
        assert_eq!(*matrix.get_color(0, 2), Ink::WHITE);
        assert_eq!(*matrix.get_color(0, 3), Ink::WHITE);
    }

}

#![allow(clippy::needless_range_loop)]

use image as im;

use crate::ga::*;
use crate::pixels;
use itertools::{flatten, Itertools};
use std::collections::HashSet;

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
    Three,
}

impl From<u8> for Mode {
    fn from(val: u8) -> Self {
        match val {
            0 => Mode::Zero,
            1 => Mode::One,
            2 => Mode::Two,
            3 => Mode::Three,
            _ => panic!(format!("{} is not a valid mode.", val)),
        }
    }
}

#[allow(missing_docs)]
impl Mode {
    /// Return the maximum number of colors for the current mode (without using rasters)
    pub fn max_colors(self) -> usize {
        match self {
            Mode::Zero => 16,
            Mode::One | Mode::Three => 4,
            Mode::Two => 2,
        }
    }

    /// Return the number of pixels encode by one byte in the given mode
    pub fn nb_pixels_per_byte(self) -> usize {
        match self {
            Mode::Zero | Mode::Three => 2,
            Mode::One => 4,
            Mode::Two => 8,
        }
    }
}

/// Conversion rules
#[derive(Copy, Clone, Debug)]
pub enum ConversionRule {
    /// All pixels are used
    AnyModeUseAllPixels,
    /// One pixel out of two is skiped (used for mode0 pictures where the graphician has doubled each pixel)
    ZeroSkipOddPixels,
}

/// Browse the image and returns the list of colors
#[allow(unused)]
fn get_unique_colors(img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>) -> HashSet<im::Rgb<u8>> {
    let mut set = HashSet::new();
    for pixel in img.pixels() {
        set.insert(pixel.clone());
    }
    set
}

/// Browse the image and returns the palette to use
#[allow(unused)]
fn extract_palette(img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>) -> Palette {
    let colors = get_unique_colors(img);
    let mut p = Palette::empty();

    assert!(colors.len() <= 16);

    for (idx, color) in colors.iter().enumerate() {
        let color = *color;
        p.set(Pen::from(idx as u8), Ink::from(color))
    }

    p
}

/// Encode the raw array of Pens in an array of CPC bytes encoded for the right screen mode
fn encode(pens: &[Vec<Pen>], mode: Mode) -> Vec<Vec<u8>> {
    let mut rows = Vec::new();
    for input_row in pens.iter() {
        let row = {
            match mode {
                Mode::Zero => pixels::mode0::pens_to_vec(input_row),
                Mode::One => pixels::mode1::pens_to_vec(input_row),
                _ => panic!("Unimplemented yet ..."),
            }
        };
        rows.push(row);
    }

    rows
}

/// Build a new screen line that reprents line1 in mode 0 and line2 in mode3
fn merge_mode0_mode3(line1: &[u8], line2: &[u8]) -> Vec<u8> {
    assert_eq!(line1.len(), line2.len());

    eprintln!("Line 1 {:?}", line1);
    eprintln!("Line 2 {:?}", line2);
    line1
        .iter()
        .zip(line2.iter())
        .map(|(&u1, &u2)| {
            let (p10, p11) = pixels::mode0::byte_to_pens(u1);
            let (p20, p21) = pixels::mode0::byte_to_pens(u2);

            let p0 = pixels::mode0::mix_mode0_mode3(p10, p20);
            let p1 = pixels::mode0::mix_mode0_mode3(p11, p21);

            eprintln!("{}/{} {:?} + {:?} = {:?}", u1, u2, &p10, &p20, &p0);
            eprintln!("{}/{} {:?} + {:?} = {:?}", u1, u2, &p11, &p21, &p1);

            pixels::mode0::pens_to_byte(p0, p1)
        })
        .collect::<Vec<u8>>()
}

// Convert inks to pens
fn inks_to_pens(inks: &[Vec<Ink>], p: &Palette) -> Vec<Vec<Pen>> {
    inks.iter()
        .map(|line| {
            line.iter()
                .map(|ink| {
                    p.get_pen_for_ink(*ink).unwrap_or_else(|| {
                        panic!(
                            "Unable to find a correspondance for ink {:?} in given palette {:?}",
                            ink, p
                        )
                    })
                })
                .collect::<Vec<Pen>>()
        })
        .collect::<Vec<_>>()
}

/// A ColorMatrix represents an image through a list of Inks.
/// It has no real meaning in CPC world but can be used for image transformaton
/// There is no mode information
#[derive(Clone, Debug)]
pub struct ColorMatrix {
    /// List of inks
    data: Vec<Vec<Ink>>,
}

#[allow(missing_docs)]
impl ColorMatrix {
    /// Create a new empty color matrix for the given dimensions
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![vec![Ink::from(0); width]; height],
        }
    }

    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a new ColorMatrix that encodes a new image full of black
    pub fn empty_like(&self) -> Self {
        Self {
            data: vec![vec![Ink::from(0); self.width() as usize]; self.height() as usize],
        }
    }

    /// Double the width (usefull for chuncky conversions)
    #[allow(clippy::needless_range_loop, clippy::identity_op)]
    pub fn double_horizontally(&mut self) {
        // Create the doubled pixels
        let mut new_data =
            vec![vec![Ink::from(0); (2 * self.width()) as usize]; self.height() as usize];
        for x in 0..(self.width() as usize) {
            for y in 0..(self.height() as usize) {
                let color = self.get_ink(x, y);
                new_data[y][x * 2 + 0] = *color;
                new_data[y][x * 2 + 1] = *color;
            }
        }

        // Set them in the right position
        std::mem::swap(&mut self.data, &mut new_data)
    }

    pub fn remove_odd_columns(&mut self) {
        // Create the doubled pixels
        let mut new_data =
            vec![vec![Ink::from(0); (self.width() / 2) as usize]; self.height() as usize];
        for x in 0..((self.width() / 2) as usize) {
            for y in 0..(self.height() as usize) {
                let color = self.get_ink(x * 2, y);
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

    /// Returns the ink at the right position
    pub fn get_ink(&self, x: usize, y: usize) -> &Ink {
        &self.data[y][x]
    }

    /// Set ink
    pub fn set_ink(&mut self, x: usize, y: usize, ink: Ink) {
        self.data[y][x] = ink;
    }

    /// Add a line within the image
    /// Panic if impossible
    pub fn add_line(&mut self, position: usize, line:&[Ink]) {
        assert_eq!(line.len(), self.width() as usize);
        self.data.insert(position, line.to_vec());
    }

    /// Returns a reference on the wanted line of inks
    pub fn get_line(&self, y: usize) -> &[Ink] {
        &self.data[y]
    }

    /// Build a vector of Inks that contains all the inks of the given column
    pub fn get_column(&self, x: usize) -> Vec<Ink> {
        self.data.iter().map(|line| line[x]).collect::<Vec<Ink>>()
    }

    /// Return a copy of the inks for the given window definition
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

    /// Return the number of different inks in the image
    pub fn nb_inks(&self) -> usize {
        flatten(self.data.iter()).unique().count()
    }

    /// Returns the palette used (as soon as there is less than 16 inks)
    pub fn extract_palette(&self) -> Palette {
        let mut p = Palette::empty();
        for (idx, color) in flatten(self.data.iter()).unique().enumerate() {
            if idx >= 16 {
                panic!("[ERROR] your picture uses more than 16 different colors. Palette: {:?}. Wrong ink: {:?}", p, color);
            }
            p.set(Pen::from(idx as u8), *color);
        }
        p
    }

    /// Modify the image in order to keep the right amount of inks
    pub fn reduce_colors_for_mode(&mut self, mode: Mode) {
        // Get the reduced palette
        let inks = flatten(self.data.iter())
            .unique()
            .copied()
            .collect::<Vec<Ink>>();
        let max_count = mode.max_colors().min(inks.len());
        let inks = &inks[..max_count];

        // Replace all wrong inks by first possible ink
        for y in 0..(self.height() as usize) {
            for x in 0..(self.width() as usize) {
                if !inks.contains(&self.data[y][x]) {
                    self.data[y][x] = inks[0];
                }
            }
        }
    }

    /// Get the width (in bytes) of the image
    /// TODO Use a trait for that
    pub fn width(&self) -> u32 {
        match self.height() {
            0 => 0,
            _ => self.data[0].len() as u32,
        }
    }

    pub fn convert_from_fname(
        fname: &str,
        conversion: ConversionRule,
    ) -> Result<Self, im::ImageError> {
        let img = im::open(fname)?;
        Ok(Self::convert(&img.to_rgb(), conversion))
    }

    pub fn convert(
        img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>,
        conversion: ConversionRule,
    ) -> Self {
        // Get destination image size
        let height = img.height();
        let width = {
            match conversion {
                ConversionRule::AnyModeUseAllPixels => img.width(),
                ConversionRule::ZeroSkipOddPixels => img.width() / 2,
            }
        };

        // Make the pixels extraction
        let mut lines = Vec::new();
        lines.reserve(height as usize);
        for y in 0..height {
            let src_y = y;
            let mut line = Vec::new();
            line.reserve(width as usize);
            for x in 0..width {
                let src_x = {
                    match conversion {
                        ConversionRule::AnyModeUseAllPixels => x,
                        ConversionRule::ZeroSkipOddPixels => x * 2,
                    }
                };

                let src_color = img.get_pixel(src_x, src_y);
                let dest_ink = Ink::from(*src_color);

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
        let mut data = vec![vec![Ink::from(26); other.width() as usize]; other.height() as usize];

        // Set the error positions
        for x in 0..(self.width() as usize) {
            for y in 0..(self.height() as usize) {
                if self.data[y][x] != other.data[y][x] {
                    data[y][x] = Ink::from(0);
                }
            }
        }

        // Return the object
        Self { data }
    }

    /// Convert the buffer as an image
    pub fn as_image(&self) -> im::ImageBuffer<im::Rgba<u8>, Vec<u8>> {
        let mut buffer: im::ImageBuffer<im::Rgba<u8>, Vec<u8>> =
            im::ImageBuffer::new(self.width(), self.height());

        for x in 0..(self.width()) {
            for y in 0..(self.height()) {
                buffer.put_pixel(x, y, self.get_ink(x as usize, y as usize).color());
            }
        }

        buffer
    }

    /// Convert the matrix as a sprite, given the right mode and an optionnal palette
    pub fn as_sprite(&self, mode: Mode, palette: Option<Palette>) -> Sprite {
        // Extract the palette is not provided as an argument
        let palette = palette.unwrap_or_else(|| self.extract_palette());

        // Really make the conversion
        let pens = inks_to_pens(&self.data, &palette);

        // Build the sprite
        Sprite {
            mode: Some(mode),
            palette: Some(palette),
            data: encode(&pens, mode),
        }
    }

    /// Convert the matrix as a sprite in mode1. Pen 1/2/3 are changed at each line. Pen 0 is constant
    pub fn as_mode1_sprite_with_different_inks_per_line(
        &self,
        palette: &[(Ink, Ink, Ink, Ink)],
        dummy_palette: &Palette,
    ) -> Sprite {
        // Build the matrix of pens
        let mut data: Vec<Vec<Pen>> = Vec::new();
        for y in 0..self.height() {
            let y = y as usize;

            // Build the palette for the current ink
            let line_palette = {
                let mut p = Palette::new(); // Palette full of 0
                p.set(Pen::from(0), palette[y].0);
                p.set(Pen::from(1), palette[y].1);
                p.set(Pen::from(2), palette[y].2);
                p.set(Pen::from(3), palette[y].3);
                p
            };

            // get the pens for the current line
            let pens = self.get_line(y).iter().enumerate().map(|(x, ink)|-> Pen {
                let pen = line_palette.get_pen_for_ink(*ink);
                if let Some(pen) = pen { 
                    pen
                } else {
                        eprintln!("
                            [ERROR] In line {}, pixel {} color ({:?}) is not in the palette {:?}. Background is used insted", 
                            y,
                            x,
                            ink,
                            line_palette
                        );
                        Pen::from(0)
                    } // If the color is not in the palette, use pen 0
            }).collect::<Vec<Pen>>();

            // Transform the pens in bytes
            data.push(pens);
        }

        let encoded_pixels = encode(&data, Mode::One);

        // Convert the matrix of pens as a sprite
        Sprite {
            mode: Some(Mode::One),
            palette: Some(dummy_palette.clone()),
            data: encoded_pixels,
        }
    }
}

/// A Sprite corresponds to a set of bytes encoded to the right CPC pixel format for a given
/// palette.
/// TODO Check why mode nad palette are optionnals. Force them if it is not mandatory to have htem
/// optionnal
#[derive(Debug)]
pub struct Sprite {
    /// Optional screen mode of the sprite
    pub(crate) mode: Option<Mode>,
    /// Optinal palete of the sprite
    pub(crate) palette: Option<Palette>,
    /// Content of the sprite
    pub(crate) data: Vec<Vec<u8>>,
}

#[allow(missing_docs)]
impl Sprite {
    /// TODO Use TryFrom once in standard rust
    /// The conversion can only work if a palette and a mode is provided
    pub fn to_color_matrix(&self) -> Option<ColorMatrix> {
        if self.mode.is_none() && self.palette.is_none() {
            return None;
        }

        let mut data = Vec::with_capacity(self.data.len());
        let p = self.palette.as_ref().unwrap();
        for line in &self.data {
            let inks = match self.mode {
                Some(Mode::Zero) | Some(Mode::Three) => line
                    .iter()
                    .flat_map(|b: &u8| {
                        let pens = {
                            let mut pens = pixels::mode0::byte_to_pens(*b);
                            pens.0.limit(self.mode.unwrap());
                            pens.1.limit(self.mode.unwrap());
                            pens
                        };
                        vec![*p.get(&pens.0), *p.get(&pens.1)]
                    })
                    .collect::<Vec<Ink>>(),

                _ => unimplemented!(),
            };
            data.push(inks);
        }

        Some(ColorMatrix { data })
    }

    /// Produce a linearized version of the sprite.
    pub fn to_linear_vec(&self) -> Vec<u8> {
        let size = self.height() * self.byte_width();
        let mut bytes: Vec<u8> = Vec::with_capacity(size as usize);

        for y in 0..self.height() {
            bytes.extend_from_slice(&self.data[y as usize]);
        }

        bytes
    }

    /// Get the palette of the sprite
    pub fn palette(&self) -> Option<Palette> {
        self.palette.clone()
    }

    pub fn set_palette(&mut self, palette: Palette) {
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
    pub fn byte_width(&self) -> u32 {
        match self.height() {
            0 => 0,
            _ => self.data[0].len() as u32,
        }
    }

    /// Get the width in pixels of the image.
    /// The mode must be specified
    pub fn pixel_width(&self) -> u32 {
        match self.mode {
            None => panic!("Unable to get the pixel width when mode is not specified"),
            Some(mode) => mode.nb_pixels_per_byte() as u32 * self.byte_width(),
        }
    }

    /// Returns the byte at the right position and crash if it does not exists
    pub fn get_byte(&self, x: usize, y: usize) -> u8 {
        let line = &self.data[y];
        line[x]
    }

    /// Returns the byte at the right position if exists
    pub fn get_byte_safe(&self, x: usize, y: usize) -> Option<u8> {
        self.data
            .get(y)
            .and_then(|v| v.get(x))
            .and_then(|v| Some(*v))
    }

    /// Returns the line of interest
    pub fn get_line(&self, y: usize) -> &Vec<u8> {
        &self.data[y]
    }

    /// Convert an RGB image to a sprite that code the pixels
    /// XXX Since 2018-06-16, most of code is delagated to ColorMatrix => maybe some bugs has been
    /// added
    pub fn convert(
        img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>,
        mode: Mode,
        conversion: ConversionRule,
        palette: Option<Palette>,
    ) -> Self {
        // Get the list of Inks that represent the image
        let matrix = ColorMatrix::convert(img, conversion);
        matrix.as_sprite(mode, palette)
    }

    pub fn convert_from_fname(
        fname: &str,
        mode: Mode,
        conversion: ConversionRule,
        palette: Option<Palette>,
    ) -> Result<Self, im::ImageError> {
        let img = im::open(fname)?;
        Ok(Self::convert(&img.to_rgb(), mode, conversion, palette))
    }

    /// Apply a transformation function on each line
    /// It can change there size
    pub fn horizontal_transform<F>(&mut self, f: F)
    where
        F: Fn(&Vec<u8>) -> Vec<u8>,
    {
        let mut transformed = self.data.iter().map(f).collect::<Vec<_>>();
        ::std::mem::swap(&mut transformed, &mut self.data);
    }
}

/// Simple multimode sprite where each line can have its own resolution mode
/// The palette is assumed to be the same on all the lines
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct MultiModeSprite {
    mode: Vec<Mode>,
    palette: Palette,
    data: Vec<Vec<u8>>,
}

#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum MultiModeConversion {
    FirstHalfSecondHalf,
    OddEven,
}

#[allow(missing_docs)]
impl MultiModeSprite {
    /// Build an empty multimode sprite BUT provide the palette
    pub fn new(p: Palette) -> Self {
        Self {
            palette: p,
            mode: Vec::new(), // Color modes for the real lines
            data: Vec::new(), // Data for texture lines (twice less than real ones
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
    pub fn to_mode0_sprite(&self) -> Sprite {
        Sprite {
            mode: Some(Mode::Zero),
            palette: Some(self.palette.clone()),
            data: self.data.clone(),
        }
    }

    pub fn to_mode3_sprite(&self) -> Sprite {
        Sprite {
            mode: Some(Mode::Three),
            palette: Some(self.palette.clone()),
            data: self.data.clone(),
        }
    }

    /// Generate a multimode sprite that mixes mode 0 and mode 3 and uses only 4 colors
    #[allow(clippy::similar_names, clippy::identity_op)]
    pub fn mode0_mode3_mix_from_mode0(sprite: &Sprite, conversion: MultiModeConversion) -> Self {
        // TODO check that there are only the first 4 inks used
        let p_orig = sprite.palette().unwrap();

        //  Build the specific palette for multimode
        let p = {
            let mut p = Palette::new();

            // First 4 inks are strictly the same
            for i in 0..4 {
                p.set(i.into(), p_orig.get(i.into()).clone());
            }

            // The others depends on the bits kept in mode 0 or mode 4
            let lut = [
                (0, [5, 6, 7]),
                (1, [8, 10, 11]),
                (2, [12, 13, 15]),
                (3, [4, 9, 14]),
            ];

            // Fill inks depending on the lut
            for (src, dsts) in &lut {
                dsts.iter().for_each(|dst| {
                    p.set((*dst).into(), p_orig.get((*src).into()).clone());
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
                } else {
                    sprite_height / 2 + 0
                };

                let mut modes = Vec::with_capacity(sprite_height);
                let mut lines = Vec::with_capacity(encoded_height);

                // Create the vector of modes
                for i in 0..sprite_height {
                    let mode = if i < encoded_height {
                        Mode::Zero
                    } else {
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
                        None => line1.clone(),
                    };

                    lines.push(line);
                }

                (modes, lines)
            }

            _ => unimplemented!(),
        };

        Self {
            palette: p,
            mode: modes,
            data: lines,
        }
    }
}

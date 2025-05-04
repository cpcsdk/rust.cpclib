#![allow(clippy::needless_range_loop)]

use std::collections::HashSet;

use anyhow::Context;
use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use {anyhow, image as im};

use crate::ga::*;
use crate::pixels;

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
            _ => panic!("{} is not a valid mode.", val)
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
        let extra = if 0 != width % self.nb_pixels_per_byte() {
            1
        }
        else {
            0
        };
        width / self.nb_pixels_per_byte() + extra
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
                Mode::Two => pixels::mode2::pens_to_vec(input_row),
                _ => panic!("Unimplemented yet ...")
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

            eprintln!("{}/{} {:?} + {:?} = {:?}", u1, u2, &p10, &p20, &p0);
            eprintln!("{}/{} {:?} + {:?} = {:?}", u1, u2, &p11, &p21, &p1);

            pixels::mode0::pens_to_byte(p0, p1)
        })
        .collect::<Vec<u8>>()
}

// Convert inks to pens
fn inks_to_pens(inks: &[Vec<Ink>], p: &Palette) -> Vec<Vec<Pen>> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
    let iter = inks.par_iter();
    #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
    let iter = inks.iter();

    iter.map(|line| {
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
    data: Vec<Vec<Ink>>
}

impl From<Vec<Vec<Ink>>> for ColorMatrix {
    fn from(data: Vec<Vec<Ink>>) -> Self {
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

impl ColorMatrix {
    pub const INK_MASK_BACKGROUND: Ink = Ink::BRIGHTWHITE;
    pub const INK_MASK_FOREGROUND: Ink = Ink::BLACK;

    pub fn from_screen(data: &[u8], bytes_width: usize, mode: Mode, palette: &Palette) -> Self {
        let pixel_height = {
            let mut height = 0x4000 / bytes_width;
            while height % 8 != 0 {
                height -= 1;
            }
            height
        };

        let _pixel_width = mode.nb_pixels_for_bytes_width(bytes_width);

        (0..pixel_height)
            .map(|line| {
                let screen_address = 0xC000 + ((line / 8) * bytes_width) + ((line % 8) * 0x800);
                let data_address = screen_address - 0xC000;
                let line_bytes = &data[data_address..(data_address + bytes_width)];
                line_bytes
                    .iter()
                    .flat_map(|b| pixels::byte_to_pens(*b, mode))
                    .collect_vec()
            })
            .map(move |pens| {
                // build lines of inks
                pens.iter()
                    .map(|pen| palette.get(pen))
                    .cloned()
                    .collect_vec()
            })
            .collect_vec()
            .into()
    }

    pub fn from_sprite(data: &[u8], pixels_width: u16, mode: Mode, palette: &Palette) -> Self {
        let width = mode.nb_bytes_for_pixels_width(pixels_width as _);

        // convert it
        data.chunks_exact(width)
            .map(|line| {
                // build lines of pen
                let line = line.iter();
                line.flat_map(|b| pixels::byte_to_pens(*b, mode))
                    .collect_vec()
            })
            .map(move |pens| {
                // build lines of inks
                pens.iter()
                    .map(|pen| palette.get(pen))
                    .cloned()
                    .collect_vec()
            })
            .collect_vec()
            .into()
    }
}

#[allow(missing_docs)]
impl ColorMatrix {
    /// Create a new empty color matrix for the given dimensions
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![vec![Ink::from(0); width]; height]
        }
    }

    /// The matrix represents both the mask (with an unexpected color), and the sprite (<ith the expected color).
    /// This method returns two matrices:
    /// - The mask where bright white stands for pixels of the sprite and black stands for the pixels of the background
    /// - The sprite where the background is replaced by a selected ink (Ideally the one that will be considered as being pen 0)
    pub fn extract_mask_and_sprite(
        &self,
        mask_ink: impl Into<Ink>,
        replacement_ink: impl Into<Ink>
    ) -> (Self, Self) {
        let mask_ink = mask_ink.into();
        let replacement_ink = replacement_ink.into();

        let mut mask_data = self.clone();
        mask_data.convert_to_mask(mask_ink);

        let mut sprite_data = self.clone();
        sprite_data.replace_ink(mask_ink, replacement_ink);

        (mask_data, sprite_data)
    }

    /// Destroy the image to build the mask according to the background ink
    pub fn convert_to_mask(&mut self, mask: Ink) {
        self.data.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|ink| {
                *ink = if *ink == mask {
                    Self::INK_MASK_BACKGROUND
                }
                else {
                    Self::INK_MASK_FOREGROUND
                }
            })
        });
    }

    /// Exchange all the occurrences of `from` Ink with `to` ink
    pub fn replace_ink(&mut self, from: Ink, to: Ink) {
        self.data.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|ink| {
                if *ink == from {
                    *ink = to;
                }
            })
        });
    }

    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a new ColorMatrix that encodes a new image full of black
    pub fn empty_like(&self) -> Self {
        Self {
            data: vec![vec![Ink::from(0); self.width() as usize]; self.height() as usize]
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
    pub fn add_line(&mut self, position: usize, line: &[Ink]) {
        assert_eq!(line.len(), self.width() as usize);
        self.data.insert(position, line.to_vec());
    }

    /// Returns a reference on the wanted line of inks
    pub fn get_line(&self, y: usize) -> &[Ink] {
        &self.data[y]
    }

    /// Return a mutable version of the line. Care needs to be taken in order to not destroy the data structure
    fn get_line_mut(&mut self, y: usize) -> &mut Vec<Ink> {
        &mut self.data[y]
    }

    /// Add a column within the image
    /// Panic if impossible
    pub fn add_column(&mut self, position: usize, column: &[Ink]) {
        assert_eq!(column.len(), self.height() as usize);
        for (row, ink) in column.iter().enumerate() {
            self.get_line_mut(row).insert(position, *ink);
        }
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
        self.data.iter().flatten().unique().count()
    }

    /// Returns the palette used (as soon as there is less than the maximum number of inks fr the requested mode)
    pub fn extract_palette(&self, mode: Mode) -> Palette {
        let mut p = Palette::empty();
        for (idx, color) in self.data.iter().flatten().unique().sorted().enumerate() {
            if idx >= mode.max_colors() {
                // do we really want to fail ? maybe we can have special modes to handle there
                panic!("[ERROR] your picture uses more than 16 different colors. Palette: {:?}. Wrong ink: {:?}", p, color);
            }
            p.set(Pen::from(idx as u8), *color);
        }
        p
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
            .collect::<Vec<Ink>>();
        let max_count = mode.max_colors().min(inks.len());
        let inks = &inks[..max_count];

        self.reduce_colors_with(inks, strategy)
    }

    /// Modify the image in order to use only the provided palette
    pub fn reduce_colors_with(
        &mut self,
        inks: &[Ink],
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
        let img = im::open(fname).with_context(|| format!("{} does not exists.", fname))?;
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
                        ConversionRule::ZeroSkipOddPixels => x * 2
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

    /// From a ColorMatrix computed with the diff method, returns the (x,y) coordinates having a difference
    pub fn diff_to_positions(&self) -> Vec<(usize, usize)> {
        let mut res = Vec::new();
        for x in 0..(self.width() as usize) {
            for y in 0..(self.height() as usize) {
                if self.data[y][x] == Ink::from(0) {
                    res.push((x, y));
                }
            }
        }
        res
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

    /// Convert the matrix as a sprite, given the right mode and an optional palette
    pub fn as_sprite(&self, mode: Mode, palette: Option<Palette>) -> Sprite {
        // Extract the palette is not provided as an argument
        let palette = palette.unwrap_or_else(|| self.extract_palette(mode));

        // Really make the conversion
        let pens = inks_to_pens(&self.data, &palette);

        // Build the sprite
        Sprite {
            mode: Some(mode),
            palette: Some(palette),
            data: encode(&pens, mode)
        }
    }

    /// Convert the matrix as a sprite in mode1. Pen 1/2/3 are changed at each line. Pen 0 is constant
    pub fn as_mode1_sprite_with_different_inks_per_line(
        &self,
        palette: &[(Ink, Ink, Ink, Ink)],
        dummy_palette: &Palette
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

        let encoded_pixels = encode(&data, Mode::One);

        // Convert the matrix of pens as a sprite
        Sprite {
            mode: Some(Mode::One),
            palette: Some(dummy_palette.clone()),
            data: encoded_pixels
        }
    }

    /// Generate an iterator on the pixels
    pub fn inks(&self) -> Inks<'_> {
        Inks {
            image: self,
            x: 0,
            y: 0,
            width: self.width(),
            height: self.height()
        }
    }
}

/// Immutable ink iterator for generate (x, y, ink)
#[derive(Debug)]
pub struct Inks<'a> {
    image: &'a ColorMatrix,
    x: u32,
    y: u32,
    width: u32,
    height: u32
}

impl Iterator for Inks<'_> {
    type Item = (u32, u32, Ink);

    fn next(&mut self) -> Option<(u32, u32, Ink)> {
        if self.x >= self.width {
            self.x = 0;
            self.y += 1;
        }

        if self.y >= self.height {
            None
        }
        else {
            let ink = self.image.get_ink(self.x as _, self.y as _);
            let i = (self.x, self.y, *ink);

            self.x += 1;

            Some(i)
        }
    }
}

/// Animation are stored in lists of ColorMatrices of same sze
#[derive(Debug)]
pub struct ColorMatrixList(Vec<ColorMatrix>);

impl From<Vec<ColorMatrix>> for ColorMatrixList {
    fn from(src: Vec<ColorMatrix>) -> Self {
        ColorMatrixList(src)
    }
}

impl From<&ColorMatrixList> for Vec<ColorMatrix> {
    fn from(val: &ColorMatrixList) -> Self {
        val.0.clone()
    }
}

impl std::ops::Deref for ColorMatrixList {
    type Target = Vec<ColorMatrix>;

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

impl ColorMatrixList {
    /// Provide a Vec version of the items
    pub fn to_vec(&self) -> Vec<ColorMatrix> {
        self.into()
    }

    /// Animations are stored within GIF files.
    /// TODO allow over kind of image data
    pub fn convert_from_fname(fname: &str, conversion: ConversionRule) -> anyhow::Result<Self> {
        use std::fs::File;

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
                screen.pixels.width() as u32,
                screen.pixels.height() as u32,
                screen
                    .pixels
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
        inks: &[Ink],
        strategy: ColorConversionStrategy
    ) -> Result<(), anyhow::Error> {
        self.0
            .iter_mut()
            .try_for_each(|matrix| matrix.reduce_colors_with(inks, strategy))
    }

    /// Number of frames in the animation
    pub fn len(&self) -> usize {
        self.0.len()
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
    pub fn as_sprites(&self, mode: Mode, palette: Option<Palette>) -> SpriteList {
        self.to_vec()
            .iter()
            .map(|matrix| matrix.as_sprite(mode, palette.clone()))
            .collect::<Vec<Sprite>>()
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
                while start_x % mode.nb_pixels_per_byte() != 0 {
                    start_x -= 1;
                }
            },
            _ => {}
        }

        // Ensure horizontal stop contraint is respected
        match hor_conf {
            HorizontalCrop::Right(HorizontalCropConstraint::CompleteByteForMode(ref mode))
            | HorizontalCrop::Both(_, HorizontalCropConstraint::CompleteByteForMode(ref mode)) => {
                while (stop_x + 1) % mode.nb_pixels_per_byte() != 0 {
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
            .collect::<Vec<ColorMatrix>>()
            .into()
    }
}

/// List of sprites for animations
#[derive(Debug)]
pub struct SpriteList(Vec<Sprite>);

impl From<Vec<Sprite>> for SpriteList {
    fn from(src: Vec<Sprite>) -> Self {
        SpriteList(src)
    }
}

impl std::ops::Deref for SpriteList {
    type Target = Vec<Sprite>;

    fn deref(&self) -> &Self::Target {
        &self.0
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
    pub(crate) data: Vec<Vec<u8>>
}

#[allow(missing_docs)]
impl Sprite {
    pub fn from_pens(pens: &[Vec<Pen>], mode: Mode, palette: Option<Palette>) -> Self {
        let data = pens
            .iter()
            .map(|line| crate::pixels::pens_to_vec(line, mode))
            .collect_vec();
        Sprite {
            data,
            mode: Some(mode),
            palette
        }
    }

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
                        .collect::<Vec<Ink>>()
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
        palette: Option<Palette>
    ) -> Self {
        // Get the list of Inks that represent the image
        let matrix = ColorMatrix::convert(img, conversion);
        matrix.as_sprite(mode, palette)
    }

    pub fn convert_from_fname<P: AsRef<Utf8Path>>(
        fname: P,
        mode: Mode,
        conversion: ConversionRule,
        palette: Option<Palette>
    ) -> Result<Self, im::ImageError> {
        let img = im::open(fname.as_ref())?;
        Ok(Self::convert(&img.to_rgb8(), mode, conversion, palette))
    }

    /// Apply a transformation function on each line
    /// It can change there size
    pub fn horizontal_transform<F>(&mut self, f: F)
    where F: Fn(&Vec<u8>) -> Vec<u8> {
        let mut transformed = self.data.iter().map(f).collect::<Vec<_>>();
        ::std::mem::swap(&mut transformed, &mut self.data);
    }
}

/// Simple multimode sprite where each line can have its own resolution mode
/// The palette is assumed to be the same on all the lines
#[derive(Clone, Debug)]
#[allow(missing_docs, unused)]
pub struct MultiModeSprite {
    mode: Vec<Mode>,
    palette: Palette,
    data: Vec<Vec<u8>>
}

#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum MultiModeConversion {
    FirstHalfSecondHalf,
    OddEven
}

#[allow(missing_docs)]
impl MultiModeSprite {
    /// Build an empty multimode sprite BUT provide the palette
    pub fn new(p: Palette) -> Self {
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
    pub fn to_mode0_sprite(&self) -> Sprite {
        Sprite {
            mode: Some(Mode::Zero),
            palette: Some(self.palette.clone()),
            data: self.data.clone()
        }
    }

    pub fn to_mode3_sprite(&self) -> Sprite {
        Sprite {
            mode: Some(Mode::Three),
            palette: Some(self.palette.clone()),
            data: self.data.clone()
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

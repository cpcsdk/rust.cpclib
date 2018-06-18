extern crate image as im;

use pixels;
use ga::*;
use std::collections::HashSet;
use itertools::Itertools;

/// Screen mode
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Mode0,
    Mode1,
    Mode2,
    Mode3
}



impl Mode {

    /// Return the maximum number of colors for the current mode (without using rasters)
    pub fn max_colors(&self) -> usize {
        match self {
            &Mode::Mode0 => 16,
            &Mode::Mode1 => 4,
            &Mode::Mode2 => 2,
            &Mode::Mode3 => 4
        }
    }
}

/// Conversion rules
#[derive(Copy, Clone)]
pub enum ConversionRule {
    AnyModeUseAllPixels,
    Mode0SkipOddPixels
}

/// Browse the image and returns the list of colors
fn get_unique_colors(img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>)  -> HashSet<im::Rgb<u8>>{
    let mut set = HashSet::new();
    for pixel in img.pixels() {
        set.insert(pixel.clone());
    }
    set
}


/// Browse the image and returnes the palette to use
fn extract_palette(img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>) -> Palette {
    let colors = get_unique_colors(img);
    let mut p = Palette::default();

    assert!(colors.len()<=16);

    for (idx, color) in colors.iter().enumerate() {
        let color = color.clone();
        p.set(
            &Pen::from(idx as u8),
            Ink::from(color)
        )
    }

    p
}


/// Encode the raw array of Pens in an array of CPC bytes encoded for the right screen mode
fn encode(pens: Vec<Vec<Pen>>, mode: Mode) -> Vec<Vec<u8>>{
    let mut rows = Vec::new();
    for input_row in pens.iter() {
        let row = {
            match mode {
                Mode::Mode0 => pixels::mode0::pens_to_vec(input_row),
                Mode::Mode1 => pixels::mode1::pens_to_vec(input_row),
                _ => panic!("Unimplemented yet ...")
            }
        };
        rows.push(row);
    }

    rows
}


/// Build a new screen line that reprents line1 in mode 0 and line2 in mode3
fn merge_mode0_mode3(line1: &Vec<u8>, line2: &Vec<u8>) -> Vec<u8> {
    assert_eq!(line1.len(), line2.len());

    eprintln!("Line 1 {:?}", line1);
    eprintln!("Line 2 {:?}", line2);
    line1.iter().zip(line2.iter()).map(|(u1, u2)|{

        let (p10, p11) = pixels::mode0::byte_to_pens(u1.clone());
        let (p20, p21) = pixels::mode0::byte_to_pens(u2.clone());

        let p0 = pixels::mode0::mix_mode0_mode3(&p10, &p20);
        let p1 = pixels::mode0::mix_mode0_mode3(&p11, &p21);

        eprintln!("{}/{} {:?} + {:?} = {:?}", *u1, *u2, &p10, &p20, &p0);
        eprintln!("{}/{} {:?} + {:?} = {:?}", *u1, *u2, &p11, &p21, &p1);

        pixels::mode0::pens_to_byte(&p0, &p1)
    }).collect::<Vec<u8>>()
}


// Convert inks to pens
fn inks_to_pens(inks: &Vec<Vec<Ink>>, p: &Palette) -> Vec<Vec<Pen>> {
    inks.iter().map( |line| {
        line.iter().map(|ink|{
            p.get_pen_for_ink(ink).unwrap()
        }).collect::<Vec<Pen>>()
    }).collect::<Vec<_>>()
}

/// A ColorMatrix represents an image through a list of Inks.
/// It has no real meaning in CPC world but can be used for image transformaton
/// There is no mode information
pub struct ColorMatrix {
    data: Vec<Vec<Ink>>
}


impl ColorMatrix {


    /// Create a new ColorMatrix that encodes a new image full of black
    pub fn empty_like(&self) -> ColorMatrix {
        ColorMatrix{
            data: vec![vec![Ink::from(0); self.width() as usize]; self.height() as usize]
        }
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

    pub fn get_line(&self, y: usize) -> &Vec<Ink> {
        &self.data[y]
    }


    /// Returns the palette used (as soon as there is less than 16 inks)
    pub fn extract_palette(&self) -> Palette {
        let mut p = Palette::default();
        for (idx, color) in self.data.iter().flatten().unique().enumerate() {
            assert!(idx < 16);
            let color = color.clone();
            p.set(
                &Pen::from(idx as u8),
                Ink::from(color)
            );
        }
        p
    }



    /// Get the width (in bytes) of the image
    /// TODO Use a trait for that
    pub fn width(&self) -> u32 {
        match self.height() {
            0 => 0 as u32,
            _ => self.data[0].len() as u32
        }
    }


       pub fn convert(
        img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>,
        conversion: ConversionRule) -> ColorMatrix {

        // Get destination image size
        let height = img.height();
        let width = {
            match conversion {
                ConversionRule::AnyModeUseAllPixels => img.width(),
                ConversionRule::Mode0SkipOddPixels => img.width()/2
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
                        ConversionRule::Mode0SkipOddPixels => x*2
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
        ColorMatrix{
            data: lines
        }


    }


    /// Compute a difference map to see the problematic positions
    pub fn diff(&self, other: &ColorMatrix) -> ColorMatrix {
        // Create a map encoding a complete success
        let mut data = vec![ vec![Ink::from(26); other.width() as usize]; other.height() as usize];

        // Set the error positions
        for x in 0..(self.width() as usize) {
            for y in 0..(self.height() as usize) {
                if self.data[y][x] != other.data[y][x] {
                    data[y][x] = Ink::from(0);
                }
            }
        }

        // Return the object
        ColorMatrix {
            data
        }
    }


    /// Convert the buffer as an image
    pub fn as_image(&self) -> im::ImageBuffer<im::Rgba<u8>, Vec<u8>> {
        let mut buffer : im::ImageBuffer<im::Rgba<u8>, Vec<u8>> = im::ImageBuffer::new(
            self.width(),
            self.height()
        );

        for x in 0..(self.width()) {
            for y in 0..(self.height()) {
                buffer.put_pixel(x, y, self.get_ink(x as usize, y as usize).color());
            }
        }

        buffer
    }


    /// Conver the matrix as a sprite, given the right mode and an optionnal palette
    pub fn as_sprite(&self, mode: Mode, palette: Option<Palette>) -> Sprite {

        // Extract the palette is not provided as an argument
        let palette = palette.unwrap_or(self.extract_palette());

        // Really make the conversion
        let pens = inks_to_pens(&self.data, &palette);

        // Build the sprite
        Sprite {
            mode: Some(mode),
            palette: Some(palette),
            data: encode(pens, mode.clone())
        }


    }
}




/// A Sprite corresponds to a set of bytes encoded to the right CPC pixel format for a given
/// palette.
/// TODO Check why mode nad palette are optionnals. Force them if it is not mandatory to have htem
/// optionnal
pub struct Sprite {
    mode: Option<Mode>,
    palette: Option<Palette>,
    data: Vec<Vec<u8>>
}


impl Sprite {

    /// TODO Use TryFrom once in standard rust
    /// The conversion can only work if a palette and a mode is provided
    pub fn to_color_matrix(&self) -> Option<ColorMatrix> {
        if self.mode.is_none() && self.palette.is_none() {
            return None;
        }

        let mut data = Vec::with_capacity(self.data.len());
        let p = self.palette.as_ref().unwrap();
        for line in self.data.iter() {
            let inks = match self.mode {
                Some(Mode::Mode0) => {
                    line
                        .iter()
                        .flat_map(|b: &u8| {
                            let pens = pixels::mode0::byte_to_pens(*b);
                            vec![p.get(&pens.0).clone(), p.get(&pens.1).clone()]
                        })
                        .collect::<Vec<Ink>>()
                },

                _ => unimplemented!()
            };
            data.push(inks);
        }

        Some(ColorMatrix {
            data
        })
    }

    /// Get the palette of the sprite
    pub fn palette(&self) -> Option<Palette>{
        self.palette.clone()
    }

    pub fn bytes(&self) -> &Vec<Vec<u8>> {
        &self.data
    }

    /// Get hte sprite Mode
    /// Cannot manage multimode sprites of course
    pub fn mode(&self) -> Option<Mode>{
        self.mode.clone()
    }

    /// Get the height (in pixels) of the image
    /// TODO Use a trait for that
    pub fn height(&self) -> u32 {
        self.data.len() as u32
    }

    /// Get the width (in bytes) of the image
    /// TODO Use a trait for that
    pub fn width(&self) -> u32 {
        match self.height() {
            0 => 0 as u32,
            _ => self.data[0].len() as u32
        }
    }

    /// Returns the byte at the right position
    pub fn get_byte(&self, x: usize, y: usize) -> u8 {
        self.data[y][x]
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
        palette: Option<Palette>) -> Sprite {


        // Get the list of Inks that represent the image
        let matrix = ColorMatrix::convert(img, conversion);
        matrix.as_sprite(mode, palette)

    }

    /// Apply a transformation function on each line
    /// It can change there size
    pub fn horizontal_transform<F>(&mut self, f: F)
        where F: Fn(&Vec<u8>) -> Vec<u8>{
        let mut transformed = self.data.iter().map(f).collect::<Vec<_>>();
        ::std::mem::swap(&mut transformed, &mut self.data);

    }

}

/// Simple multimode sprite where each line can have its own resolution mode
/// The palette is assumed to be the same on all the lines
pub struct MultiModeSprite {
    mode: Vec<Mode>,
    palette: Palette,
    data: Vec<Vec<u8>>
}


pub enum MultiModeConversion {
    FirstHalfSecondHalf,
    OddEven
}

impl MultiModeSprite {
    /// Build an empty multimode sprite BUT provide the palette
    pub fn new(p: Palette) -> MultiModeSprite {
        MultiModeSprite {
            palette : p,
            mode: Vec::new(), // Color modes for the real lines
            data: Vec::new() // Data for texture lines (twice less than real ones
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
    pub fn to_mode0_sprite(self) -> Sprite {
        Sprite {
            mode: Some(Mode::Mode0),
            palette: Some(self.palette),
            data: self.data
        }
    }

    /// Generate a multimode sprite that mixes mode 0 and mode 3 and uses only 4 colors
    pub fn mode0_mode3_mix_from_mode0(sprite: &Sprite, conversion: MultiModeConversion) -> MultiModeSprite {
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
                (3, [4, 9, 14])
            ];

            // Fill inks depending on the lut
            for (src, dsts) in lut.iter() {
                dsts.iter().for_each(|dst|{
                    p.set((*dst).into(), p_orig.get( (*src).into()).clone());
                });
            }

            p
        };

        // Really makes the conversion of the lines
        let (modes, lines) = match conversion {
            MultiModeConversion::FirstHalfSecondHalf => {
                let sprite_height = sprite.height() as usize;
                let encoded_height = if sprite_height %2 == 1 {
                    sprite_height/2 + 1
                }
                else {
                    sprite_height/2 + 0
                } as usize;

                let mut modes = Vec::with_capacity(sprite_height);
                let mut lines = Vec::with_capacity(encoded_height);

                // Create the vector of modes
                for i in 0..sprite_height {
                    let mode = if i < encoded_height {
                        Mode::Mode0
                    }
                    else {
                        Mode::Mode3
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
            }

            _ => unimplemented!()
        };


        MultiModeSprite{
            palette: p,
            mode: modes,
            data: lines
        }

    }

}




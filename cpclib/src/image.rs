extern crate image as im;

use pixels;
use ga::*;
use std::collections::HashSet;

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
                if self.data[y][x] == other.data[y][x] {
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
}

/// A Sprite corresponds to a set of bytes encoded to the right CPC pixel format for a given
/// palette.
/// TODO Use the ColorMatrix to make the conversion !
pub struct Sprite {
    mode: Option<Mode>,
    palette: Option<Palette>,
    data: Vec<Vec<u8>>
}


impl Sprite {

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
    /// TODO delegate beginning of work to ColorMatrix
    pub fn convert(
        img: &im::ImageBuffer<im::Rgb<u8>, Vec<u8>>,
        mode: Mode,
        conversion: ConversionRule,
        palette: Option<Palette>) -> Sprite {

        // Extract the palette if it is not provided
        let palette = palette.unwrap_or(extract_palette(img));


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
        for y in 0..height {
            let src_y = y;
            let mut line = Vec::new();
            for x in 0..width {
                let src_x = {
                    match conversion {
                        ConversionRule::AnyModeUseAllPixels => x,
                        ConversionRule::Mode0SkipOddPixels => x*2
                    }
                };

                let src_color = img.get_pixel(src_x, src_y);
                let dest_ink = Ink::from(*src_color);
                let dest_pen = palette.get_pen_for_ink(&dest_ink).expect("color not present in palette");

                // Add the current pen to the current line
                line.push(dest_pen);

            }
            // Add the current complete line to the current image
            lines.push(line);
        }

        // Transform the pixels in encoded bytes
        let lines_len = lines.len();
        let bytes = encode(lines, mode.clone());

        // check result
        let bytes_len = bytes.len();
        assert_eq!(lines_len, bytes_len);


        // And create the sprite structure
        Sprite {
            mode: Some(mode),
            palette: Some(palette),
            data: bytes
        }


    }

}

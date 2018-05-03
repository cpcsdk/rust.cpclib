extern crate image as im;

use pixels;
use ga::*;
use std::collections::HashSet;

/// Screen mode
#[derive(Clone,Copy)]
pub enum Mode {
    Mode0,
    Mode1,
    Mode2,
    Mode3
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



pub struct Sprite {
    mode: Option<Mode>,
    palette: Option<Palette>,
    data: Vec<Vec<u8>>
}


impl Sprite {

    /// Get the height (in pixels) of the image
    pub fn height(&self) -> u32 {
        self.data.len() as u32
    }


    /// Get the width (in bytes) of the image
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

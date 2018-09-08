// This module manage high level image conversion functions

extern crate image as im;

use std::path::Path;
use std::mem::swap;
use std::collections::HashSet;

use image::*;
use ga::*;


/// Encode the screen dimension in CRTC measures
#[derive(Clone)]
pub struct CPCScreenDimension {
    /// Number of bytes in width
    horizontalDisplayed: u8,
    /// Number of chars in height
    verticalDisplayed: u8,
    /// Number of pixel line per char line
    maximumRasterAddress: u8
}

impl CPCScreenDimension {

    /// Return screen dimension for a standard screen
    pub fn standard() -> Self {
        CPCScreenDimension {
            horizontalDisplayed: 80/2,
            verticalDisplayed: 25, /// Unsure of this value
            maximumRasterAddress: 7
        }
    }

    /// Return the screen dimension for a standard overscan screen
    pub fn overscan() -> Self {
        CPCScreenDimension {
            horizontalDisplayed: 96/2,
            verticalDisplayed: 36,
            maximumRasterAddress: 7
        }
    }

    /// Specify a tailored dimension
    pub fn new(horizontalDisplayed: u8, verticalDisplayed: u8, maximumRasterAddress: u8) -> Self {
        CPCScreenDimension {
            horizontalDisplayed,
            verticalDisplayed,
            maximumRasterAddress
        }
    }


    /// Number of lines to display a char
    pub fn nbLinesPerChar(&self) -> u8 {
        1 + self.maximumRasterAddress
    }

    /// Number of chars used to vertically encode the screen
    pub fn nbCharLines(&self) -> u8{
        self.verticalDisplayed
    }

    pub fn nbWordColumns(&self) -> u8 {
        self.horizontalDisplayed
    }

    /// Number of chars used to horizontally encode the screen
    pub fn nbByteColumns(&self) -> u8 {
        self.nbWordColumns()*2
    }

    /// Height of the screen in pixels
    pub fn height(&self) -> u16 {
        self.nbCharLines() as u16 * self.nbLinesPerChar() as u16
    }

    /// Width of the screen in pixels
    pub fn width(&self, mode: &Mode) -> u16 {
        self.nbByteColumns() as u16 * mode.nbPixelsPerByte() as u16
    }

    /// Return true if the image needs two banks
    pub fn use_two_banks(&self) -> bool {
        self.nbByteColumns() as u16 * self.height() > 0x4000
    }


}

/// Manage the display address contained in R12-R13
#[derive(Clone)]
pub struct DisplayAddress(u16);

impl DisplayAddress {
    const OFFSET_MASK:u16 = 0b1111111111;
    const PAGE_MASK:u16 = 0b11000000000000;
    const PAGE_SHIFT:u8 = 12;

    /// Create the display address
    pub fn new(val: u16) -> DisplayAddress {
        DisplayAddress(val & 0b0011111111111111)
    }

    /// Return the offset part of the address
    pub fn offset(&self) -> u16 {
        self.0 & DisplayAddress::OFFSET_MASK
    }

    pub fn set_offset(&mut self, offset:u16) {
        self.0 = self.0 & (!DisplayAddress::OFFSET_MASK) | (offset & DisplayAddress::OFFSET_MASK);
    }

    /// Return the buffer configuration
    pub fn buffer(&self) -> u16 {
        (self.0 & 0b110000000000) >> 10
    }

    /// Return the page configuration
    pub fn page(&self) -> u16 {
        (self.0 & DisplayAddress::PAGE_MASK) >> DisplayAddress::PAGE_SHIFT
    }

    pub fn set_page(&mut self, page:u16) {
        self.0 = self.0 & (!DisplayAddress::PAGE_MASK) | ( (page << DisplayAddress::PAGE_SHIFT) & DisplayAddress::PAGE_MASK);
    }

    pub fn R12(&self) -> u8 {
        (self.0 / 256) as u8
    }

    pub fn R13(&self) -> u8 {
        (self.0 % 256) as u8
    }

    /// Return the page value
    pub fn page_start(&self) -> u16 {
        match self.page() {
            0 => 0x0000,
            1 => 0x4000,
            2 => 0x8000,
            3 => 0xc000,
            _ => panic!()
        }
    }

    /// Check of the configuration correspond to an overscan
    pub fn is_overscan(&self) -> bool {
        match self.buffer() {
            0 | 1 | 2 => false,
            3 => true,
            _ => panic!()
        }

    }

    /// Returns the CPC address.
    pub fn address(&self) -> u16{
        self.page_start() + self.offset()*2
    }

    /// Assume the object represent the character of interest and move to next one
    pub fn move_to_next_word(&mut self) {
        let expected_offset = self.offset()+1;
        let truncated_expected_offset = expected_offset & 0b1111111111;

        // Move the offset of one char
        self.set_offset(truncated_expected_offset);

        // In overscan screen, change the page
        if truncated_expected_offset != expected_offset && self.is_overscan() {
            let val = self.page()+1;
            self.set_page(val);
        }
    }


}

/// Specify the output format to be used
/// TODO - add additional output format (for example zigzag sprites that can be usefull or sprite display routines)
#[derive(Clone)]
pub enum OutputFormat {
    /// Mode specific bytes are stored consecutively in a linear way (line 0, line 1, ... line n)
    LinearEncodedSprite,

    /// CPC memory encoded. The binary can be directly included in a snapshot
    CPCMemory{
        outputDimension: CPCScreenDimension,
        displayAddress: DisplayAddress
    },

    /// CPC memory encoded to be used with hardware splitting. The vector only contains the Variant CPCMemory
    CPCSplittingMemory(Vec<OutputFormat>)
}

/// Embeds the conversion output
/// There must be one implementation per OuputFormat
pub enum Output {
    LinearEncodedSprite(Palette),

    /// Result using one bank
    CPCMemoryStandard([u8; 0x4000], Palette),

    /// Result using two banks 
    CPCMemoryOverscan(
        [u8; 0x4000], 
        [u8; 0x4000], 
        Palette
    ),

    /// Result using several chunks of memory
    CPCSplittingMemory(Vec<Output>)
}

/// ImageConverter is able to make the conversion of images to several output format
pub struct ImageConverter<'a> {

    // TODO add a crop area to not keep the complete image
     // cropArea: Option<???>

    /// A palette can be specified
    palette: Option<Palette>,

    /// Screen mode
    mode: Mode,

    /// Output format
    output: &'a OutputFormat
        
}



impl<'a> ImageConverter<'a> {

    /// Create the object that will be used to make the conversion
    pub fn convert (input_file: &Path, palette: Option<Palette>, mode: Mode, output: &'a OutputFormat) -> Output
    {
        
        let mut converter = ImageConverter {
            palette,
            mode,
            output
        };

        let sprite = converter.load(input_file);
        converter.apply_conversion(&sprite)
    }


    /// Load the initial image
    /// TODO make compatibility tests are alike
    /// TODO propagate errors when needed
    fn load(&mut self, input_file: &Path) -> Sprite {

        let img = im::open(input_file).unwrap();
        let matrix = ColorMatrix::convert(
            &img.to_rgb(),
            ConversionRule::AnyModeUseAllPixels
        );
        let sprite = matrix.as_sprite(self.mode, self.palette.clone());
        self.palette = sprite.palette();

        sprite
    }


    /// Manage the conversion on the given sprite
    fn apply_conversion(&mut self, sprite: & Sprite) -> Output {
        let output = self.output.clone();

        match output {
            OutputFormat::LinearEncodedSprite 
                => unimplemented!(),
            OutputFormat::CPCMemory{ref outputDimension, ref displayAddress}
                => self.build_memory_blocks(sprite, outputDimension.clone(), displayAddress.clone()),
            OutputFormat::CPCSplittingMemory(ref vec)
                => unimplemented!()
        }
    } 



    /// Manage the creation of the memory blocks
    fn build_memory_blocks(&mut self, sprite: & Sprite, dim: CPCScreenDimension, displayAddress: DisplayAddress) -> Output {
        // Simulate the memory
        let mut pages  = [
            [0 as u8; 0x4000],
            [0 as u8; 0x4000],
            [0 as u8; 0x4000],
            [0 as u8; 0x4000],
        ];

        let mut used_pages = HashSet::new();
        let is_overscan = dim.use_two_banks();
        if !is_overscan && displayAddress.is_overscan() {
            panic!("Image requires an overscan configuration for R12/R13")
        }

        
        let mut current_address = displayAddress.clone();
        used_pages.insert(current_address.page());

        // loop over the chars
        for char_y in 0..dim.nbCharLines() {
            for char_x in 0..dim.nbWordColumns() {
 
                // Loop over the lines of the current char
                for char_line in 0..dim.nbLinesPerChar() {
                    // Loop over the bytes of the current char 
                    for byte_nb in 0..2 {
                        let value = sprite.get_byte_safe(
                                    (char_x as u16*2 + byte_nb) as usize, 
                                    (char_y as u16 *dim.nbLinesPerChar() as u16 + char_line as u16) as usize);

                        match value {
                            None => {},
                            Some(byte) => {
                                pages
                                    [current_address.page() as usize]
                                    [ (current_address.offset()*2+byte_nb) as usize] 
                                        = byte;
                            }
                        };
                    }
                }

                // Manage the next word (on the same line or not)
                current_address.move_to_next_word();
                used_pages.insert(current_address.page());
            }

        }

        // By construction, the order should be good
        let used_pages = used_pages.iter().map(|idx| {pages[*idx as usize]}).collect::<Vec<_>>();

        // Generate the right output format
        let palette = sprite.palette().unwrap();
        match used_pages.len() {
            1 => Output::CPCMemoryStandard(used_pages[0], palette),
            2 => Output::CPCMemoryOverscan(used_pages[0], used_pages[1], palette),
            _ => unreachable!()
        }
    }


}


#[cfg(test)]
mod tests {
    use imageconverter::*;

    #[test]
    fn overscan_test() {
        assert!(CPCScreenDimension::overscan().use_two_banks());
        assert!(!CPCScreenDimension::standard().use_two_banks());
    }


    #[test]
    fn manipulation_test() {
        let mut address = DisplayAddress::new(0x3000);

        assert_eq!(address.address(), 0xC000);
        assert_eq!(address.R12(), 0x30);
        assert_eq!(address.R13(), 0x00);
        assert!(!address.is_overscan());

        address.set_page(1);
        assert_eq!(address.page(), 1);
        assert_eq!(address.address(), 0x4000);

        address.move_to_next_word();
        assert_eq!(address.address(), 0x4002);

      }
}
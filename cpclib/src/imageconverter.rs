// This module manage high level image conversion functions

extern crate image as im;

use std::path::Path;
use std::mem::swap;
use std::collections::HashSet;
use itertools::Itertools;
use std::mem;
use std::fmt::Debug;
use bitfield::BitRange;

use crate::image::*;
use crate::ga::*;


#[derive(Clone)]
/// List of all the possible transformations applicable to a ColorMAtrix
pub enum Transformation {
    /// When using mode 0, do not read all the pixels lines
    SkipOddPixels
}


impl Transformation {
    pub fn apply(&self, matrix: ColorMatrix) -> ColorMatrix {
        match self {
           Transformation::SkipOddPixels => {
               let mut res = matrix.clone();
               res.remove_odd_columns();
               res
           }
        }
    } 
}



/// Container of transformations
#[derive(Clone)]
pub struct TransformationsList {
    transformations: Vec<Transformation>
}

impl TransformationsList {
    /// Create an empty list of transformations
    pub fn new() -> Self {
        TransformationsList {
            transformations: Vec::new()
        }
    }

    /// Add a transformation that remove one pixel column out of two
    pub fn skip_odd_pixels(self) -> Self {
        let mut transformations = self.transformations.clone();
        transformations.push(Transformation::SkipOddPixels);
        TransformationsList {
            transformations
        }
    }

    /// Apply ALL the transformation (in order of addition)
    pub fn apply(&self, matrix: ColorMatrix) -> ColorMatrix {
        let mut matrix = matrix;
        for transformation in self.transformations.iter() {
            matrix = transformation.apply(matrix);
        }
        matrix
    }
}


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

impl Debug for CPCScreenDimension{

    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(
            fmt,
            "CPCScreenDimension {{ horizontalDisplayed: {}, verticalDisplayed: {}, maximumRasterAddress: {}, use_two_banks: {} }}",
            self.horizontalDisplayed,
            self.verticalDisplayed,
            self.maximumRasterAddress,
            self.use_two_banks()
        )
    }
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
/// TODO move that later in a CRTC emulator code
#[derive(Clone, Debug)]
pub struct DisplayAddress(u16);

pub type DisplayCRTCAddress = DisplayAddress;

impl DisplayAddress {
    const OFFSET_START: usize = 9;
    const OFFSET_END: usize = 0;

    const BUFFER_START: usize = 11;
    const BUFFER_END : usize = 10;

    const PAGE_START: usize = 13;
    const PAGE_END: usize = 12;


    /// Create the display address
    pub fn new_from(val: u16) -> DisplayAddress {
        assert!(val < 0b1100000000000000);
        DisplayAddress(val) 
    }

    pub fn new(page: u16, is_overscan: bool, offset: u16) -> DisplayAddress {
        let mut address = Self::new_from(0);
        address.set_page(page);
        address.set_overscan(is_overscan);
        address.set_offset(offset);
        address
    }

    pub fn new_standard_from_page(page: u16) -> DisplayAddress {
        Self::new(page, false, 0)
    }

    pub fn new_overscan_from_page(page: u16) -> DisplayAddress {
        Self::new(page, true, 0)
    }

    pub fn new_standard_from_address(address: u16) -> DisplayAddress {
        unimplemented!()
    }

    pub fn new_overscan_from_address(address: u16) -> DisplayAddress {
        unimplemented!()
    }
    /// Return the offset part of the address
    pub fn offset(&self) -> u16 {
        self.0.bit_range(Self::OFFSET_START, Self::OFFSET_END)
    }

    pub fn set_offset(&mut self, offset:u16) {
        self.0.set_bit_range(Self::OFFSET_START, Self::OFFSET_END, offset)
    }

    /// Return the buffer configuration
    /// 0 0 16k
    /// 0 1 16k
    /// 1 0 16k
    /// 1 1 16k
    pub fn buffer(&self) -> u16 {
        self.0.bit_range(Self::BUFFER_START, Self::BUFFER_END)
    }

    pub fn set_buffer(&mut self, buffer: u16) {
        self.0.set_bit_range(Self::BUFFER_START, Self::BUFFER_END, buffer)
    }

    pub fn set_overscan(&mut self, is_overscan: bool) {
        if is_overscan {
            self.set_buffer(0b11);
        }
        else {
            self.set_buffer(0b00);
        }
    }

    /// Return the page configuration
    /// 0 0 0x0000
    /// 0 1 0x4000
    /// 1 0 0x8000
    /// 1 1 0xc000
    pub fn page(&self) -> u16 {
        self.0.bit_range(Self::PAGE_START, Self::PAGE_END)
    }

    pub fn set_page(&mut self, page:u16) {
        self.0.set_bit_range(Self::PAGE_START, Self::PAGE_END, page);
    }

    pub fn R12(&self) -> u8 {
        self.0.bit_range(15, 8) 
    }

    pub fn R13(&self) -> u8 {
        self.0.bit_range(7, 0) 
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

    /// Returns the CPC address of the first word.
    pub fn address(&self) -> u16{
        self.page_start() + self.offset()*2
    }

    /// Assume the object represent the character of interest and move to next one
    pub fn move_to_next_word(&mut self) {
        let was_overscan = self.is_overscan();

        let expected_offset = self.offset()+1;
        let truncated_expected_offset = expected_offset.bit_range(Self::OFFSET_START, Self::OFFSET_END);

        // Move the offset of one char
        self.set_offset(truncated_expected_offset);
if truncated_expected_offset != expected_offset {
    println!("From {} to {} / {} / {:?}", expected_offset, truncated_expected_offset, self.is_overscan(), self);
}
        // In overscan screen, change the page
        if truncated_expected_offset != expected_offset && self.is_overscan() {
            println!("Change of page");
            let val = self.page()+1;
            self.set_page(val);
        }

        assert_eq!(was_overscan, self.is_overscan());
    }


}

/// Specify the output format to be used
/// TODO - add additional output format (for example zigzag sprites that can be usefull or sprite display routines)
#[derive(Clone, Debug)]
pub enum OutputFormat {
    /// Mode specific bytes are stored consecutively in a linear way (line 0, line 1, ... line n)
    LinearEncodedSprite,

    /// Chuncky output where each pixel is encoded in one byte (and is supposed to be vertically duplicated)
    LinearEncodedChuncky,

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
    LinearEncodedSprite{data: Vec<u8>, palette: Palette, byte_width: usize, height: usize},

    LinearEncodedChuncky{data: Vec<u8>, palette: Palette, byte_width: usize, height: usize},

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


impl Debug for Output {

    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            &Output::LinearEncodedSprite{ref data, ref palette, ref byte_width,ref height} => {
                writeln!(fmt, "LinearEncodedSprite")
            },
            &Output::LinearEncodedChuncky{ref data, ref palette, ref byte_width,ref height}=> {
                writeln!(fmt, "LinearEncodedChuncky")
            },
            &Output::CPCMemoryStandard(_, _) => {
               writeln!(fmt, "CPCMemoryStandard (16kb)")
            },
            &Output::CPCMemoryOverscan(_, _, _) => {
                writeln!(fmt, "CPCMemoryStandard (32kb)")
            },
            &Output::CPCSplittingMemory(ref vec) =>{
               writeln!(fmt, "CPCSplitterdMemory {:?}", &vec)
            }
        }
    }
}


impl Output {

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
    output: &'a OutputFormat,

    /// List of transformations
    transformations: TransformationsList,
        
}

impl<'a> ImageConverter<'a> {

    /// Create the object that will be used to make the conversion
    pub fn convert<P> (
            input_file: P, 
            palette: Option<Palette>, 
            mode: Mode, 
            transformations: TransformationsList,
            output: &'a OutputFormat) -> Output
    where P: AsRef<Path>
    {
        Self::convert_impl(input_file.as_ref(), palette, mode, transformations,  output)        
    }

    fn convert_impl(
        input_file: &Path, 
        palette: Option<Palette>, 
        mode: Mode, 
        transformations: TransformationsList,
        output: &'a OutputFormat) -> Output
    {
        let mut converter = ImageConverter {
            palette,
            mode,
            transformations,
            output
        };

        match output {
            OutputFormat::LinearEncodedChuncky => {
                let mut matrix = converter.load_color_matrix(input_file);
                matrix.double_horizontally();
                let sprite = matrix.as_sprite(mode, None);
                Output::LinearEncodedChuncky{
                    data: sprite.to_linear_vec(),
                    palette: sprite.palette.as_ref().unwrap().clone(), // By definition, we expect the palette to be set 
                    byte_width: sprite.byte_width() as _, 
                    height: sprite.height() as _
                }
            },
            _ => {
                let sprite = converter.load_sprite(input_file);
                converter.apply_sprite_conversion(&sprite)
            }
        }
    }

    pub fn import(sprite: &Sprite, output: &'a OutputFormat) -> Output {
        let mut converter = ImageConverter {
            palette: None,
            mode: Mode::Mode0, // TODO make the mode optional,
            output: output,
            transformations: TransformationsList::new()
        };
      
        converter.apply_sprite_conversion(&sprite)
    }


    /// Load the initial image
    /// TODO make compatibility tests are alike
    /// TODO propagate errors when needed
    fn load_sprite(&mut self, input_file: &Path) -> Sprite {
        let matrix = self.load_color_matrix(input_file);
        let sprite = matrix.as_sprite(self.mode, self.palette.clone());
        self.palette = sprite.palette();

        sprite
    }

    fn load_color_matrix(&self, input_file: &Path) -> ColorMatrix {
        let img = im::open(input_file).expect(&format!("Unable to convert {:?} properly.", input_file));
        let mat = ColorMatrix::convert(
            &img.to_rgb(),
            ConversionRule::AnyModeUseAllPixels
        );
        self.transformations.apply(mat)
    }


    /// Manage the conversion on the given sprite
    fn apply_sprite_conversion(&mut self, sprite: & Sprite) -> Output {
        let output = self.output.clone();

        match output {
            OutputFormat::LinearEncodedSprite 
                => self.linearize_sprite(sprite),
            OutputFormat::CPCMemory{ref outputDimension, ref displayAddress}
                => self.build_memory_blocks(sprite, outputDimension.clone(), displayAddress.clone()),
            OutputFormat::CPCSplittingMemory(ref _vec)
                => unimplemented!(),

             _ => unreachable!()
        }
    } 


    /// Produce the linearized version of the sprite.
    /// TODO add size constraints to keep a small part of the sprite
    fn linearize_sprite(&mut self, sprite: &Sprite) -> Output {
        Output::LinearEncodedSprite{
            data: sprite.to_linear_vec(),
            palette: sprite.palette.as_ref().unwrap().clone(), // By definition, we expect the palette to be set 
            byte_width: sprite.byte_width() as _, 
            height: sprite.height() as _
        }
    }



    /// Manage the creation of the memory blocks
    /// XXX Warning, overscan is wrongly used, it is more fullscreen with 2 pages
    fn build_memory_blocks(&mut self, sprite: & Sprite, dim: CPCScreenDimension, displayAddress: DisplayAddress) -> Output {

        let screen_width = dim.width(&sprite.mode().unwrap()) as u32;
        let screen_height = dim.height() as u32;

        // Check if the destination is compatible
        if screen_width < sprite.pixel_width() {
            panic!(
                "The image width ({}) is larger than the cpc screen width ({})",
                sprite.pixel_width(),
                screen_width
            );
        }
        else if screen_width > sprite.pixel_width() {
            eprintln!(
                "[Warning] The image width ({}) is smaller than the cpc screen width ({})",
                sprite.pixel_width(),
                screen_width
            );
        }

        if screen_height < sprite.height() {
            panic!(
                "The image height ({}) is larger than the cpc screen height ({})",
                sprite.height(),
                screen_height
            );
        }
        else if  screen_height > sprite.height() {
            eprintln!(
                "[Warning] The image height ({}) is smaller than the cpc screen height ({})",
                sprite.height(),
                screen_height
            );   
        }

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

        // loop over the chars vertically
        for char_y in 0..dim.nbCharLines() {
            let char_y = char_y as usize;

            // loop over the chars horiontally (2 bytes)
            for char_x in 0..dim.nbWordColumns() {
               let char_x = char_x as usize;
 
                // Loop over the lines of the current char (8 lines for a standard screen)
                for line_in_char in 0..dim.nbLinesPerChar() {
                    let line_in_char = line_in_char as usize;

                    // Loop over the bytes of the current char 
                    for byte_nb in 0..2 {
                       let byte_nb = byte_nb as usize;

                        let x_coord = 2*char_x  + byte_nb;
                        let y_coord = dim.nbLinesPerChar() as usize *char_y + line_in_char;

                        let value = sprite.get_byte_safe(x_coord as _, y_coord as _);
                        //let value = Some(sprite.get_byte(x_coord as _, y_coord as _));

                        match value {
                            None => {
                                //eprintln!("Unable to access byte in {}, {}", x_coord, y_coord);
                            },
                            Some(byte) => {

                                let page = current_address.page() as usize;
                                let address = current_address.offset() as usize *2 + byte_nb + line_in_char*0x800;

                                pages[page][address] = byte;
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
        let used_pages = used_pages
                            .iter()
                            .sorted()
                            .iter()
                            .map(|idx| {
                                pages[**idx as usize]
                            }).collect::<Vec<_>>();

        if is_overscan && used_pages.len() != 2 {
            panic!("An overscan screen is requested but {} pages has been feed", used_pages.len());
        }

        // Generate the right output format
        let palette = sprite.palette().unwrap();
        if is_overscan {
            Output::CPCMemoryOverscan(used_pages[0], used_pages[1], palette)
        }
        else {
            Output::CPCMemoryStandard(used_pages[0], palette)
        }
    }


}


#[cfg(test)]
mod tests {
    use crate::imageconverter::*;

    #[test]
    fn overscan_test() {
        assert!(CPCScreenDimension::overscan().use_two_banks());
        assert!(!CPCScreenDimension::standard().use_two_banks());
    }


    #[test]
    fn manipulation_test() {
        let mut address = DisplayAddress::new_from(0x3000);

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
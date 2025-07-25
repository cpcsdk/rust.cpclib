// This module manage high level image conversion functions

use std::collections::HashSet;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;

use cpclib_common::bitfield::{BitRange, BitRangeMut};
use cpclib_common::camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use image as im;

use crate::ga::*;
use crate::image::*;

/// Encode the position of a line or column to transform in the source image
#[derive(Copy, Clone, Debug)]
pub enum TransformationPosition {
    /// This is the very first line or column of the image
    First,
    /// This is the very last line or column of the image
    Last,
    /// This is a specific index
    Index(usize)
}

impl TransformationPosition {
    /// Get the absolute position regarding the image size
    pub fn absolute_position(self, size: usize) -> Option<usize> {
        match self {
            TransformationLinePosition::First => Some(0),
            TransformationLinePosition::Last => Some(size - 1),
            TransformationLinePosition::Index(idx) => {
                if idx >= size {
                    None
                }
                else {
                    Some(idx)
                }
            },
        }
    }
}

/// Type that represent the position in a line of the image
pub type TransformationLinePosition = TransformationPosition;
/// Type that represent the position in a column of the image
pub type TransformationColumnPosition = TransformationPosition;

/// List of all the possible transformations applicable to a ColorMatrix
#[derive(Clone, Debug)]
pub enum Transformation {
    /// When using mode 0, do not read all the pixel columns
    SkipOddPixels,
    /// Shorten lines of several pixel columns on the left
    SkipLeftPixelColumns(u16),
    /// Shorten columns of several pixel columns on the top
    SkipTopPixelLines(u16),
    KeepLeftPixelColumns(u16),
    KeepTopPixelLines(u16),
    /// Add artifical blank lines. The line is build by repeating the background the right amount of time
    BlankLines {
        /// The pattern to use to fill the background
        pattern: Vec<Ink>,
        /// The location of the line within the image
        position: TransformationPosition,
        /// The amount of lines to add
        amount: u16
    },

    /// Add artificial blank columns given a pattern
    BlankColumns {
        /// The pattern to use to fill the background
        pattern: Vec<Ink>,
        /// The location of the column within the image
        position: TransformationPosition,
        /// The amount of columns to add
        amount: u16
    },

    /// Replace one Ink by another one
    ReplaceInk {
        from: Ink,
        to: Ink
    },

    /// Create a mask from the background Ink MaskFromBackgroundInk
    MaskFromBackgroundInk(Ink)
}

impl Transformation {
    /// Apply the transformation to the list of colormatrix
    /// TODO find a way to use the same function name than for a ColorMatrix
    pub fn apply_to_list(&self, list: &ColorMatrixList) -> ColorMatrixList {
        list.to_vec()
            .iter()
            .map(|matrix| self.apply(matrix))
            .collect::<Vec<ColorMatrix>>()
            .into()
    }

    /// Apply the transformation to the given image
    pub fn apply(&self, matrix: &ColorMatrix) -> ColorMatrix {
        match self {
            Transformation::SkipOddPixels => {
                let mut res = matrix.clone();
                res.remove_odd_columns();
                res
            },
            Transformation::SkipLeftPixelColumns(amount) => {
                matrix.window(
                    *amount as _,
                    0,
                    matrix.width().saturating_sub(*amount as _) as _,
                    matrix.height() as _
                )
            },
            Transformation::SkipTopPixelLines(amount) => {
                matrix.window(
                    0 as _,
                    *amount as _,
                    matrix.width() as _,
                    matrix.height().saturating_sub(*amount as _) as _
                )
            },
            Transformation::KeepLeftPixelColumns(usize) => {
                matrix.window(0, 0, *usize as _, matrix.height() as _)
            },
            Transformation::KeepTopPixelLines(usize) => {
                matrix.window(0, 0, matrix.width() as _, *usize as _)
            },
            Transformation::BlankLines {
                pattern,
                position,
                amount
            } => {
                // Build the line according to the background pattern
                let line = {
                    let mut lines = Vec::new();
                    for idx in 0..(matrix.width() as usize) {
                        lines.push(pattern[idx % pattern.len()]);
                    }
                    lines
                };

                // Get the real position (will not change over the additions)
                let position = position.absolute_position(matrix.height() as _).unwrap();

                // Modify the image
                let mut res = matrix.clone();
                (0..*amount).for_each(|_| {
                    res.add_line(position, &line);
                });
                res
            },
            Transformation::BlankColumns {
                pattern,
                position,
                amount
            } => {
                let column = {
                    let mut column = Vec::new();
                    for idx in 0..(matrix.height() as usize) {
                        column.push(pattern[idx % pattern.len()])
                    }
                    column
                };

                let position = position.absolute_position(matrix.width() as _).unwrap();

                let mut res = matrix.clone();
                (0..*amount).for_each(|_| {
                    res.add_column(position, &column);
                });

                res
            },

            Transformation::ReplaceInk { from, to } => {
                let mut res = matrix.clone();
                res.replace_ink(*from, *to);
                res
            },
            Transformation::MaskFromBackgroundInk(ink) => {
                let mut res = matrix.clone();
                res.convert_to_mask(*ink);
                res
            }
        }
    }

    /// Create a transformation that adds blank lines
    pub fn blank_lines<I: Into<Ink> + Copy>(
        pattern: &[I],
        position: TransformationLinePosition,
        amount: u16
    ) -> Self {
        Self::BlankLines {
            pattern: pattern.iter().map(|&i| i.into()).collect::<Vec<Ink>>(),
            position,
            amount
        }
    }

    /// Create a transformation that adds blanck columns
    pub fn blank_columns<I: Into<Ink> + Copy>(
        pattern: &[I],
        position: TransformationColumnPosition,
        amount: u16
    ) -> Self {
        Self::BlankColumns {
            pattern: pattern.iter().map(|&i| i.into()).collect::<Vec<_>>(),
            position,
            amount
        }
    }
}

/// Container of transformations
#[derive(Clone, Debug, Default)]
pub struct TransformationsList {
    /// list of transformations
    transformations: Vec<Transformation>
}

#[allow(missing_docs)]
impl TransformationsList {
    /// Create an empty list of transformations
    pub fn new(transformations: &[Transformation]) -> Self {
        TransformationsList {
            transformations: transformations.to_vec()
        }
    }

    /// Add a transformation that remove one pixel column out of two
    pub fn skip_odd_pixels(mut self) -> Self {
        self.transformations.push(Transformation::SkipOddPixels);
        self
    }

    pub fn column_start(mut self, count: u16) -> Self {
        self.transformations
            .push(Transformation::SkipLeftPixelColumns(count));
        self
    }

    pub fn columns_kept(mut self, count: u16) -> Self {
        self.transformations
            .push(Transformation::KeepLeftPixelColumns(count));
        self
    }

    pub fn lines_kept(mut self, count: u16) -> Self {
        self.transformations
            .push(Transformation::KeepTopPixelLines(count));
        self
    }

    pub fn line_start(mut self, count: u16) -> Self {
        self.transformations
            .push(Transformation::SkipTopPixelLines(count));
        self
    }

    pub fn replace(mut self, from: Ink, to: Ink) -> Self {
        self.transformations
            .push(Transformation::ReplaceInk { from, to });
        self
    }

    pub fn build_mask_from_background_ink(mut self, background: Ink) -> Self {
        self.transformations
            .push(Transformation::MaskFromBackgroundInk(background));
        self
    }

    /// Apply ALL the transformation (in order of addition)
    pub fn apply(&self, matrix: &ColorMatrix) -> ColorMatrix {
        let mut result = matrix.clone();
        for transformation in &self.transformations {
            result = transformation.apply(&result);
        }
        result
    }
}

/// Encode the screen dimension in CRTC measures
#[derive(Clone, Copy)]
pub struct CPCScreenDimension {
    /// Number of bytes in width
    pub horizontal_displayed: u8,
    /// Number of chars in height
    pub vertical_displayed: u8,
    /// Number of pixel line per char line
    pub maximum_raster_address: u8
}

impl Debug for CPCScreenDimension {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "CPCScreenDimension {{ horizontal_displayed: {}, vertical_displayed: {}, maximum_raster_address: {}, use_two_banks: {} }}",
            self.horizontal_displayed,
            self.vertical_displayed,
            self.maximum_raster_address,
            self.use_two_banks()
        )
    }
}

#[allow(missing_docs)]
impl CPCScreenDimension {
    /// Return screen dimension for a standard screen
    pub fn standard() -> Self {
        Self {
            horizontal_displayed: 80 / 2,
            vertical_displayed: 25,
            /// Unsure of this value
            maximum_raster_address: 7
        }
    }

    /// Return the screen dimension for a standard overscan screen
    pub fn overscan() -> Self {
        Self {
            horizontal_displayed: 96 / 2,
            vertical_displayed: 39, // it was 36 before? need to find why,
            maximum_raster_address: 7
        }
    }

    /// Specify a tailored dimension
    pub fn new(
        horizontal_displayed: u8,
        vertical_displayed: u8,
        maximum_raster_address: u8
    ) -> Self {
        Self {
            horizontal_displayed,
            vertical_displayed,
            maximum_raster_address
        }
    }

    /// Number of lines to display a char
    pub fn nb_lines_per_char(self) -> u8 {
        1 + self.maximum_raster_address
    }

    /// Number of chars used to vertically encode the screen
    pub fn nb_char_lines(self) -> u8 {
        self.vertical_displayed
    }

    pub fn nb_word_columns(self) -> u8 {
        self.horizontal_displayed
    }

    /// Number of chars used to horizontally encode the screen
    pub fn nb_byte_columns(self) -> u8 {
        self.nb_word_columns() * 2
    }

    /// Height of the screen in pixels
    pub fn height(self) -> u16 {
        u16::from(self.nb_char_lines()) * u16::from(self.nb_lines_per_char())
    }

    /// Width of the screen in pixels
    pub fn width(self, mode: Mode) -> u16 {
        u16::from(self.nb_byte_columns()) * mode.nb_pixels_per_byte() as u16
    }

    /// Return true if the image needs two banks
    pub fn use_two_banks(self) -> bool {
        u16::from(self.nb_byte_columns()) * self.height() > 0x4000
    }
}

/// Manage the display address contained in R12-R13
/// TODO move that later in a CRTC emulator code
#[derive(Clone, Copy, Debug)]
pub struct DisplayAddress(u16);

#[allow(missing_docs)]
pub type DisplayCRTCAddress = DisplayAddress;

#[allow(missing_docs)]
impl DisplayAddress {
    const BUFFER_END: usize = 10;
    const BUFFER_START: usize = 11;
    const OFFSET_END: usize = 0;
    const OFFSET_START: usize = 9;
    const PAGE_END: usize = 12;
    const PAGE_START: usize = 13;

    /// Create the display address
    pub fn new_from(val: u16) -> Self {
        assert!(val < 0b1100_0000_0000_0000);
        Self(val)
    }

    pub fn new(page: u16, is_overscan: bool, offset: u16) -> Self {
        let mut address = Self::new_from(0);
        address.set_page(page);
        address.set_overscan(is_overscan);
        address.set_offset(offset);

        dbg!(address.r12(), address.r13());
        address
    }

    pub fn new_standard_from_page(page: u16) -> Self {
        Self::new(page, false, 0)
    }

    /// Generate an address that allow to display overscan picture from the given page
    pub fn new_overscan_from_page(page: u16) -> Self {
        Self::new(page, true, 0)
    }

    /// Generate an overscan address where each line is contained in a single bank
    pub fn new_overscan_from_page_one_bank_per_line(page: u16, char_width: u16) -> Self {
        // number of words missing
        let delta = (0x800 % (char_width * 2)) / 2;
        Self::new(page, true, delta)
    }

    pub fn new_standard_from_address(_address: u16) -> Self {
        unimplemented!()
    }

    pub fn new_overscan_from_address(_address: u16) -> Self {
        unimplemented!()
    }

    /// Return the offset part of the address
    pub fn offset(self) -> u16 {
        self.0.bit_range(Self::OFFSET_START, Self::OFFSET_END)
    }

    pub fn set_offset(&mut self, offset: u16) {
        self.0
            .set_bit_range(Self::OFFSET_START, Self::OFFSET_END, offset)
    }

    /// Return the buffer configuration
    /// 0 0 16k
    /// 0 1 16k
    /// 1 0 16k
    /// 1 1 16k
    pub fn buffer(self) -> u16 {
        self.0.bit_range(Self::BUFFER_START, Self::BUFFER_END)
    }

    pub fn set_buffer(&mut self, buffer: u16) {
        self.0
            .set_bit_range(Self::BUFFER_START, Self::BUFFER_END, buffer)
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
    pub fn page(self) -> u16 {
        self.0.bit_range(Self::PAGE_START, Self::PAGE_END)
    }

    pub fn set_page(&mut self, page: u16) {
        self.0.set_bit_range(Self::PAGE_START, Self::PAGE_END, page);
    }

    pub fn r12(self) -> u8 {
        self.0.bit_range(15, 8)
    }

    pub fn r13(self) -> u8 {
        self.0.bit_range(7, 0)
    }

    /// Return the page value
    pub fn page_start(self) -> u16 {
        match self.page() {
            0 => 0x0000,
            1 => 0x4000,
            2 => 0x8000,
            3 => 0xC000,
            _ => panic!()
        }
    }

    /// Check of the configuration correspond to an overscan
    pub fn is_overscan(self) -> bool {
        match self.buffer() {
            0..=2 => false,
            3 => true,
            _ => panic!()
        }
    }

    /// Returns the CPC address of the first word.
    pub fn address(self) -> u16 {
        self.page_start() + self.offset() * 2
    }

    /// Change the adress to point to the previous word
    pub fn move_to_previous_word(&mut self) {
        unimplemented!()
    }

    /// Assume the object represent the character of interest and move to next one
    pub fn move_to_next_word(&mut self) {
        let was_overscan = self.is_overscan();

        let expected_offset = self.offset() + 1;
        let truncated_expected_offset =
            expected_offset.bit_range(Self::OFFSET_START, Self::OFFSET_END);

        // Move the offset of one char
        self.set_offset(truncated_expected_offset);
        if truncated_expected_offset != expected_offset {
            println!(
                "From {} to {} / {} / {:?}",
                expected_offset,
                truncated_expected_offset,
                self.is_overscan(),
                self
            );
        }
        // In overscan screen, change the page
        if truncated_expected_offset != expected_offset && self.is_overscan() {
            println!("Change of page");
            let val = self.page() + 1;
            self.set_page(val);
        }

        assert_eq!(was_overscan, self.is_overscan());
    }
}

/// Specify the output format to be used
/// TODO - add additional output format (for example zigzag sprites that can be usefull or sprite display routines)
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum OutputFormat {
    MaskedSprite {
        sprite_format: SpriteEncoding,
        mask_ink: Ink,
        replacement_ink: Ink
    },
    Sprite(SpriteEncoding),

    /// Chuncky output where each pixel is encoded in one byte (and is supposed to be vertically duplicated)
    LinearEncodedChuncky,

    /// CPC memory encoded. The binary can be directly included in a snapshot
    CPCMemory {
        output_dimension: CPCScreenDimension,
        display_address: DisplayAddress
    },

    /// CPC memory encoded to be used with hardware splitting. The vector only contains the Variant CPCMemory
    CPCSplittingMemory(Vec<OutputFormat>),

    /// For quite complexe coding more related to very fast display
    TileEncoded {
        /// The width of a tile
        tile_width: TileWidthCapture,
        /// The height of a tile
        tile_height: TileHeightCapture,
        /// The way tile are horizontally captured
        horizontal_movement: TileHorizontalCapture,
        /// The way tile are vertically captured
        vertical_movement: TileVerticalCapture,
        /// The width of the grid (i.e., the number of tiles present in a row)
        grid_width: GridWidthCapture,
        /// The height of the gris (i.e., the number of tiles present in a column)
        grid_height: GridHeightCapture
    }
}

#[allow(missing_docs)]
impl OutputFormat {
    /// For formats manipulating a display address, modify it vertically in order to make scroll the image
    pub fn vertically_shift_display_address(&mut self, delta: i32) {
        if let Self::CPCMemory {
            output_dimension,
            display_address
        } = self
        {
            if delta >= 0 {
                for _ in 0..delta * i32::from(output_dimension.nb_word_columns()) {
                    display_address.move_to_next_word();
                }
            }
            else {
                for _ in 0..(-delta) * i32::from(output_dimension.nb_word_columns()) {
                    display_address.move_to_previous_word();
                }
            }
        }
    }

    /// Generate output format for a linear sprite
    pub fn create_linear_encoded_sprite() -> Self {
        Self::Sprite(SpriteEncoding::Linear)
    }

    pub fn create_graycode_encoded_sprite() -> Self {
        Self::Sprite(SpriteEncoding::GrayCoded)
    }

    pub fn create_zigzag_graycode_encoded_sprite() -> Self {
        Self::Sprite(SpriteEncoding::ZigZagGrayCoded)
    }

    /// Generate output format for an overscan screen
    pub fn create_overscan_cpc_memory() -> Self {
        Self::CPCMemory {
            output_dimension: CPCScreenDimension::overscan(),
            display_address: DisplayAddress::new_overscan_from_page(2) /* we do not care of the page */
        }
    }

    /// Generate output format for an overscan screen for which each imageline is in a single bank (this is not the case for the standard overscan)
    pub fn create_overscan_cpc_memory_one_bank_per_line() -> Self {
        let output_dimension = CPCScreenDimension::overscan();
        let display_address = DisplayAddress::new_overscan_from_page_one_bank_per_line(
            2,
            output_dimension.nb_word_columns() as _
        );
        Self::CPCMemory {
            output_dimension,
            display_address
        }
    }

    pub fn create_standard_cpc_memory() -> Self {
        Self::CPCMemory {
            output_dimension: CPCScreenDimension::standard(),
            display_address: DisplayAddress::new_standard_from_page(2)
        }
    }
}

/// Defines the width of the capture
#[derive(Debug, Clone, Copy)]
pub enum TileWidthCapture {
    /// All the width is captured
    FullWidth,
    /// Only the given number of bytes is captured
    NbBytes(usize)
}

/// Defines the width of the capture
#[derive(Debug, Clone, Copy)]
pub enum TileHeightCapture {
    /// All the height is captured
    FullHeight,
    /// Only the given number of lines is captured
    NbLines(usize)
}

/// Defines the width of the capture
#[derive(Debug, Clone, Copy)]
pub enum GridWidthCapture {
    /// All the width is captured
    FullWidth,
    /// Only the given number of tiles are capture in a row
    TilesInRow(usize)
}

/// Defines the width of the capture
#[derive(Debug, Clone, Copy)]
pub enum GridHeightCapture {
    /// All the height is captured
    FullHeight,
    /// Only the given number of tiles is captured in a column
    TilesInColumn(usize)
}

/// Defines the horizontal movement when capturing bytes
#[derive(Debug, Clone, Copy)]
pub enum TileHorizontalCapture {
    /// Bytes are always captured from left to right
    AlwaysFromLeftToRight,
    /// Bytes are always captured from right to left
    AlwaysFromRightToLeft,
    /// Bytes are read in a right-left left-right way
    StartFromRightAndFlipAtTheEndOfLine,
    /// Bytes are read in a left-right right-left way
    StartFromLeftAndFlipAtTheEndOfLine
}

#[allow(missing_docs)]
pub trait HorizontalWordCounter {
    fn get_column_index(&self) -> usize {
        unimplemented!()
    }
    /// goto the next position to compute (that is configuration dependant)
    fn next(&mut self) {
        unimplemented!();
    }

    // Acknowledge that line is ended
    fn line_ended(&mut self) {
        unimplemented!();
    }
}

#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub struct StartFromLeftAndFlipAtTheEndOfLine {
    current_column: usize,
    left_to_right: bool
}

#[allow(missing_docs)]
impl Default for StartFromLeftAndFlipAtTheEndOfLine {
    fn default() -> Self {
        Self {
            current_column: 0,
            left_to_right: true
        }
    }
}
#[allow(missing_docs)]
impl HorizontalWordCounter for StartFromLeftAndFlipAtTheEndOfLine {
    fn get_column_index(&self) -> usize {
        self.current_column
    }

    fn next(&mut self) {
        if self.left_to_right {
            self.current_column += 1;
        }
        else {
            self.current_column -= 1
        }
    }

    fn line_ended(&mut self) {
        self.left_to_right = !self.left_to_right;
    }
}

/// Structure to manage the horizontal movement of the sprite cursor
#[derive(Debug, Copy, Clone)]
pub struct StandardHorizontalCounter {
    left_to_right: bool,
    current_step: usize,
    // We cannot have sprite of width
    nb_columns: Option<std::num::NonZeroUsize>
}

impl StandardHorizontalCounter {
    /// Generate a counter for always copy from left to right
    pub fn always_from_left_to_right() -> StandardHorizontalCounter {
        StandardHorizontalCounter {
            left_to_right: true,
            current_step: 0,
            nb_columns: None
        }
    }

    /// Generate a counter for always copy from right to left
    /// The number of columns MUST be specified in some way
    pub fn always_from_right_to_left() -> StandardHorizontalCounter {
        StandardHorizontalCounter {
            left_to_right: false,
            current_step: 0,
            nb_columns: None
        }
    }
}

#[allow(missing_docs)]
impl HorizontalWordCounter for StandardHorizontalCounter {
    fn get_column_index(&self) -> usize {
        if self.left_to_right {
            self.current_step
        }
        else {
            usize::from(self.nb_columns.unwrap()) - self.current_step
        }
    }

    fn next(&mut self) {
        self.current_step += 1;
    }

    fn line_ended(&mut self) {
        self.current_step = 0;
    }
}

#[allow(missing_docs)]
impl TileHorizontalCapture {
    pub fn counter(self) -> Box<dyn HorizontalWordCounter> {
        match self {
            Self::AlwaysFromLeftToRight => {
                Box::new(StandardHorizontalCounter::always_from_left_to_right())
            },
            Self::AlwaysFromRightToLeft => unimplemented!(),
            Self::StartFromRightAndFlipAtTheEndOfLine => unimplemented!(),
            Self::StartFromLeftAndFlipAtTheEndOfLine => {
                Box::<StartFromLeftAndFlipAtTheEndOfLine>::default()
            },
        }
    }
}

/// Utility structure that helps in playing with gray code movement in lines.
/// We assume that chars are 8 lines tall. Some modification are possible for chars of 4 lines
///
/// Addresses ordered by lines on screen
///
/// 0 0x00??  000
/// 1 0x08??  001
/// 2 0x10??  010
/// 3 0x18??  011
/// 4 0x20??  100
/// 5 0x28??  101
/// 6 0x30??  110
/// 7 0x38??  111
///
/// Adresses ordered by graycode
///
/// 0 000 => 0
/// 1 001 => 1
/// 2 011 => 3
/// 3 010 => 2
/// 4 110 => 6
/// 5 111 => 7
/// 6 101 => 5
/// 7 100 => 4
#[derive(Debug, Copy, Clone)]
#[allow(missing_docs)]
#[derive(Default)]
pub struct GrayCodeLineCounter {
    char_line: usize,
    pos_in_char: u8 // in gray code space
}

/// Standard line counter
#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub struct StandardLineCounter {
    pos_in_screen: usize,
    top_to_bottom: bool
}

/// LineCounter manage the choice of the line when iterateing a sprite vertically
pub trait LineCounter {
    /// Return the real number of the line
    fn get_line_index_in_screen(&self) -> usize;

    /// goto the next position to compute (that is configuration dependant)
    fn next(&mut self) {
        unimplemented!();
    }
}

#[allow(missing_docs)]
impl StandardLineCounter {
    pub fn top_to_bottom() -> Self {
        Self {
            pos_in_screen: 0,
            top_to_bottom: true
        }
    }

    pub fn bottom_to_top(start: usize) -> Self {
        Self {
            pos_in_screen: start,
            top_to_bottom: false
        }
    }
}

#[allow(missing_docs)]
impl LineCounter for StandardLineCounter {
    fn get_line_index_in_screen(&self) -> usize {
        self.pos_in_screen
    }

    fn next(&mut self) {
        if self.top_to_bottom {
            self.pos_in_screen += 1;
        }
        else {
            self.pos_in_screen -= 1;
        }
    }
}

#[allow(missing_docs)]
impl GrayCodeLineCounter {
    pub const GRAYCODE_INDEX_TO_SCREEN_INDEX: [u8; 8] = [0, 1, 3, 2, 6, 7, 5, 4];
    #[allow(unused)]
    pub const SCREEN_INDEX_TO_GRAYCODE_INDEX: [u8; 8] = [0, 1, 3, 2, 7, 6, 4, 5];

    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_char_line(&self) -> usize {
        self.char_line
    }

    pub fn get_line_index_in_char(&self) -> u8 {
        Self::GRAYCODE_INDEX_TO_SCREEN_INDEX[self.get_graycode_index_in_char() as usize]
    }

    pub fn get_graycode_index_in_char(&self) -> u8 {
        self.pos_in_char
    }

    /// Modify the state to represent the previous line
    pub fn goto_previous_line(&mut self) {
        if self.pos_in_char == 0 {
            self.pos_in_char = 7;
            self.char_line -= 1;
        }
        else {
            self.pos_in_char -= 1;
        }
    }

    /// Modify the state to represent the next line
    pub fn goto_next_line(&mut self) {
        self.pos_in_char += 1;
        if self.pos_in_char == 8 {
            self.pos_in_char = 0;
            self.char_line += 1;
        }
    }
}

impl LineCounter for GrayCodeLineCounter {
    fn get_line_index_in_screen(&self) -> usize {
        self.char_line * 8 + self.get_line_index_in_char() as usize
    }

    fn next(&mut self) {
        self.goto_next_line()
    }
}

/// Defines the vertical movement when capturing lines
#[derive(Debug, Clone, Copy)]
pub enum TileVerticalCapture {
    /// Lines are always captured from the top to the bottom
    AlwaysFromTopToBottom,
    /// Lines are always captured from the bottom to the top
    AlwaysFromBottomToTop,
    /// Lines are captured in a top-bottom bottom-top way
    StartFromTopAndFlipAtEndOfScreen,
    /// Lines are captured in bottom-top top-bottom way
    StartFromBottomAndFlipAtEndOfScreen,
    /// Lines are captured following a gray-code way that starts from the top
    GrayCodeFromTop,
    /// Lines are captured following a gray-code way that starts from the bottom
    GrayCodeFromBottom
}

#[allow(missing_docs)]
impl TileVerticalCapture {
    /// Generates the counter when it is possible.
    /// Panics if contextual information is needed
    pub fn counter(self) -> Box<dyn LineCounter> {
        match self {
            Self::AlwaysFromTopToBottom => Box::new(StandardLineCounter::top_to_bottom()),
            Self::AlwaysFromBottomToTop => panic!("A parameter is needed there"),
            Self::GrayCodeFromTop => Box::new(GrayCodeLineCounter::new()),
            _ => unimplemented!()
        }
    }

    pub fn counter_with_context(self, _screen_height: usize) -> Box<dyn LineCounter> {
        unimplemented!("TODO once someone will code it")
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum SpriteEncoding {
    Linear,
    GrayCoded,
    LeftToRightToLeft,
    ZigZagGrayCoded
}

#[derive(Clone)]
pub struct SpriteOutput {
    data: Vec<u8>,
    palette: Palette,
    mode: Mode,
    bytes_width: usize,
    height: usize,
    encoding: SpriteEncoding
}

impl AsRef<[u8]> for SpriteOutput {
    fn as_ref(&self) -> &[u8] {
        self.data()
    }
}

impl SpriteOutput {
    pub fn with_encoding(&self, encoding: SpriteEncoding) -> Self {
        if encoding == self.encoding {
            return self.clone();
        }

        if (encoding == SpriteEncoding::LeftToRightToLeft
            && self.encoding == SpriteEncoding::Linear)
            || (self.encoding == SpriteEncoding::LeftToRightToLeft
                && encoding == SpriteEncoding::Linear)
        {
            let mut res = self.clone();
            let width = res.bytes_width();
            res.data = res
                .data
                .into_iter()
                .chunks(width)
                .into_iter()
                .enumerate()
                .flat_map(|(idx, row)| {
                    let mut row = row.collect_vec();
                    if !idx.is_multiple_of(2) {
                        row.reverse();
                    }
                    row
                })
                .collect_vec();
            return res;
        }

        unimplemented!(
            "Encoding conversion has not yet been used and is not coded. A leftRightTopBottom iterator is probably expected to ease conversion."
        )
    }

    pub fn as_sprite(&self) -> Sprite {
        Sprite::from_bytes(
            &self.data,
            self.bytes_width(),
            self.mode,
            self.palette.clone()
        )
    }

    pub fn encoding(&self) -> SpriteEncoding {
        self.encoding
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn bytes_width(&self) -> usize {
        self.bytes_width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn save_sprite<P: AsRef<Utf8Path>>(&self, fname: P) -> std::io::Result<()> {
        let fname = fname.as_ref();
        let mut file = File::create(fname)?;
        file.write_all(self.data())
    }
}

/// Embeds the conversion output
/// There must be one implementation per OuputFormat
#[allow(missing_docs)]
#[allow(clippy::large_enum_variant)]
pub enum Output {
    // Mask and sprite are encoded the very same way
    SpriteAndMask {
        sprite: SpriteOutput,
        mask: SpriteOutput
    },

    // Any kind of sprite
    Sprite(SpriteOutput),

    LinearEncodedChuncky {
        data: Vec<u8>,
        palette: Palette,
        bytes_width: usize,
        height: usize
    },

    /// Result using one bank
    CPCMemoryStandard([u8; 0x4000], Palette),

    /// Result using two banks
    CPCMemoryOverscan([u8; 0x4000], [u8; 0x4000], Palette),

    /// Result using several chunks of memory
    CPCSplittingMemory(Vec<Output>),

    /// Result containing several tiles
    TilesList {
        tile_height: u32,
        tile_width: u32,
        horizontal_movement: TileHorizontalCapture,
        vertical_movement: TileVerticalCapture,
        palette: Palette,
        list: Vec<Vec<u8>>
    }
}

impl Debug for SpriteEncoding {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Linear => writeln!(fmt, "Linear"),
            Self::GrayCoded => writeln!(fmt, "GrayCoded"),
            Self::LeftToRightToLeft => writeln!(fmt, "ZigZag"),
            Self::ZigZagGrayCoded => writeln!(fmt, "ZigZagGrayCoded")
        }
    }
}

impl Debug for SpriteOutput {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(fmt, "{:?}Sprite", self.encoding)
    }
}

impl Debug for Output {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Output::LinearEncodedChuncky { .. } => writeln!(fmt, "LinearEncodedChuncky"),
            Output::CPCMemoryStandard(..) => writeln!(fmt, "CPCMemoryStandard (16kb)"),
            Output::CPCMemoryOverscan(..) => writeln!(fmt, "CPCMemoryStandard (32kb)"),
            Output::CPCSplittingMemory(vec) => writeln!(fmt, "CPCSplitteringMemory {:?}", &vec),
            Output::TilesList {
                tile_height,
                tile_width,
                list,
                ..
            } => {
                writeln!(
                    fmt,
                    "{} tiles of {}x{}",
                    list.len(),
                    tile_width,
                    tile_height
                )
            },
            Output::SpriteAndMask { sprite, mask } => writeln!(fmt, "SpriteAndMask"),
            Output::Sprite(sprite_output) => writeln!(fmt, "{sprite_output:?}")
        }
    }
}

#[allow(missing_docs)]
impl Output {
    /// Returns the bank that contains the first half of the screen
    pub fn overscan_screen1(&self) -> Option<&[u8; 0x4000]> {
        match self {
            Self::CPCMemoryOverscan(s1, ..) => Some(s1),
            _ => None
        }
    }

    pub fn sprite(self) -> Option<SpriteOutput> {
        if let Self::Sprite(sprite) = self {
            Some(sprite)
        }
        else {
            None
        }
    }

    /// Returns the bank that contains the second half of the screen
    pub fn overscan_screen2(&self) -> Option<&[u8; 0x4000]> {
        match self {
            Self::CPCMemoryOverscan(_, s1, _) => Some(s1),
            _ => None
        }
    }

    /// Returns the list of tiles
    pub fn tiles_list(&self) -> Option<&[Vec<u8>]> {
        match self {
            Self::TilesList { list, .. } => Some(list),
            _ => None
        }
    }
}

/// ImageConverter is able to make the conversion of images to several output format
#[derive(Debug, Clone)]
pub struct ImageConverter {
    // TODO add a crop area to not keep the complete image
    // cropArea: Option<???>
    /// A palette can be specified
    palette: LockablePalette,

    /// Screen mode
    mode: Mode,

    /// Output format
    output: OutputFormat,

    /// List of transformations
    transformations: TransformationsList,

    /// Crop image if too large in comparison to result screen
    crop_if_too_large: bool
}

#[allow(missing_docs)]
impl ImageConverter {
    /// Create the object that will be used to make the conversion
    pub fn convert<P>(
        input_file: P,
        palette: LockablePalette,
        mode: Mode,
        transformations: TransformationsList,
        output: OutputFormat,
        crop_if_too_large: bool,
        missing_pen: Option<Pen>
    ) -> anyhow::Result<Output>
    where
        P: AsRef<Utf8Path>
    {
        Self::convert_impl(
            input_file.as_ref(),
            palette,
            mode,
            transformations,
            output,
            crop_if_too_large,
            missing_pen
        )
    }

    fn convert_to_sprite(
        input_file: &Utf8Path,
        palette: LockablePalette,
        mode: Mode,
        transformations: TransformationsList,
        encoding: SpriteEncoding,
        crop_if_too_large: bool,
        missing_pen: Option<Pen>
    ) -> anyhow::Result<SpriteOutput> {
        match &encoding {
            SpriteEncoding::Linear => {
                let mut converter = ImageConverter {
                    palette: palette.clone(),
                    mode,
                    transformations: transformations.clone(),
                    output: OutputFormat::Sprite(encoding),
                    crop_if_too_large
                };

                let sprite = converter.load_sprite(input_file, missing_pen);
                converter
                    .apply_sprite_conversion(&sprite)
                    .map(|output| output.sprite().unwrap())
            },

            SpriteEncoding::ZigZagGrayCoded => {
                let SpriteOutput {
                    data,
                    palette,
                    bytes_width,
                    height,
                    encoding,
                    mode
                } = Self::convert_impl(
                    input_file,
                    palette,
                    mode,
                    transformations,
                    OutputFormat::Sprite(SpriteEncoding::GrayCoded),
                    crop_if_too_large,
                    missing_pen
                )?
                .sprite()
                .unwrap();

                assert_eq!(encoding, SpriteEncoding::GrayCoded);

                let mut new_data = Vec::new();
                new_data.reserve_exact(data.len());

                for j in 0..height {
                    let mut current_line = data[j * bytes_width..(j + 1) * bytes_width].to_vec();

                    if j % 2 == 1 {
                        current_line.reverse();
                    }

                    new_data.extend(current_line);
                }

                Ok(SpriteOutput {
                    data: new_data,
                    palette,
                    bytes_width,
                    height,
                    encoding: SpriteEncoding::ZigZagGrayCoded,
                    mode
                })
            },

            SpriteEncoding::GrayCoded => {
                // get the linear version
                let linear = Self::convert_impl(
                    input_file,
                    palette,
                    mode,
                    transformations,
                    OutputFormat::Sprite(SpriteEncoding::Linear),
                    crop_if_too_large,
                    missing_pen
                )?
                .sprite()
                .unwrap();

                assert_eq!(linear.encoding, SpriteEncoding::Linear);

                assert_eq!(linear.height() % 8, 0);

                let nb_chars = linear.height() / 8;
                let mut new_data = Vec::new();
                for char_idx in 0..nb_chars {
                    for line_idx in GrayCodeLineCounter::GRAYCODE_INDEX_TO_SCREEN_INDEX.iter() {
                        let line_idx = *line_idx as usize;
                        let start = line_idx + 8 * char_idx;
                        new_data.extend_from_slice(
                            &linear.data()
                                [start * linear.bytes_width()..(start + 1) * linear.bytes_width()]
                        );
                    }
                }

                Ok(SpriteOutput {
                    encoding: SpriteEncoding::GrayCoded,
                    mode: linear.mode,
                    data: new_data,
                    palette: linear.palette,
                    bytes_width: linear.bytes_width,
                    height: linear.height
                })
            },
            SpriteEncoding::LeftToRightToLeft => unimplemented!()
        }
    }

    fn convert_impl(
        input_file: &Utf8Path,
        palette: LockablePalette,
        mode: Mode,
        transformations: TransformationsList,
        output: OutputFormat,
        crop_if_too_large: bool,
        missing_pen: Option<Pen>
    ) -> anyhow::Result<Output> {
        let mut converter = ImageConverter {
            palette: palette.clone(),
            mode,
            transformations: transformations.clone(),
            output: output.clone(),
            crop_if_too_large
        };

        if let OutputFormat::LinearEncodedChuncky = &output {
            let mut matrix = converter.load_color_matrix(input_file);
            matrix.double_horizontally();
            let sprite = matrix.as_sprite(mode, LockablePalette::empty(), None);
            Ok(Output::LinearEncodedChuncky {
                data: sprite.to_linear_vec(),
                palette: sprite.palette.as_ref().unwrap().clone(), /* By definition, we expect the palette to be set */
                bytes_width: sprite.bytes_width() as _,
                height: sprite.height() as _
            })
        }
        else if let OutputFormat::Sprite(sprite_output_format) = &output {
            Self::convert_to_sprite(
                input_file,
                palette,
                mode,
                transformations,
                *sprite_output_format,
                crop_if_too_large,
                missing_pen
            )
            .map(Output::Sprite)
        }
        else if let OutputFormat::MaskedSprite {
            sprite_format,
            mask_ink,
            replacement_ink
        } = &output
        {
            let sprite_transformations =
                transformations.clone().replace(*mask_ink, *replacement_ink);
            let sprite = Self::convert_to_sprite(
                input_file,
                palette,
                mode,
                sprite_transformations,
                *sprite_format,
                crop_if_too_large,
                missing_pen
            )?;

            let mask_transformations = transformations
                .clone()
                .build_mask_from_background_ink(*mask_ink);
            let mut mask_palette = vec![ColorMatrix::INK_NOT_USED_IN_MASK; 16];
            mask_palette[0] = ColorMatrix::INK_MASK_FOREGROUND; // at the position with all bits reset
            mask_palette[mode.max_colors() - 1] = ColorMatrix::INK_MASK_BACKGROUND; // at the position with all bits set up
            let mask = Self::convert_to_sprite(
                input_file,
                LockablePalette::locked(mask_palette.into()), /* we want and 0 ; or byte where we plot */
                mode,
                mask_transformations,
                *sprite_format,
                crop_if_too_large,
                missing_pen
            )?;

            // Just for debug stuff
            if true {
                let mask_img = mask.as_sprite().as_image();
                let sprite_img = sprite.as_sprite().as_image();
                mask_img.save_with_format("/tmp/mask.png", image::ImageFormat::Png);
                sprite_img.save_with_format("/tmp/sprite.png", image::ImageFormat::Png);
            }

            Ok(Output::SpriteAndMask { sprite, mask })
        }
        else {
            let sprite = converter.load_sprite(input_file, missing_pen);
            converter.apply_sprite_conversion(&sprite)
        }
    }

    /// Makes the conversion of the provided sprite to the expected format
    pub fn import(sprite: &Sprite, output: OutputFormat) -> anyhow::Result<Output> {
        let mut converter = ImageConverter {
            palette: LockablePalette::empty(),
            mode: Mode::Zero, // TODO make the mode an optional argument,
            output,
            transformations: TransformationsList::default(),
            crop_if_too_large: false
        };

        converter.apply_sprite_conversion(sprite)
    }

    /// Load the initial image
    /// TODO make compatibility tests are alike
    /// TODO propagate errors when needed
    fn load_sprite(&mut self, input_file: &Utf8Path, missing_pen: Option<Pen>) -> Sprite {
        let matrix = self.load_color_matrix(input_file);
        let sprite = matrix.as_sprite(self.mode, self.palette.clone(), missing_pen);
        self.palette = LockablePalette::locked(sprite.palette().unwrap());

        sprite
    }

    fn load_color_matrix(&self, input_file: &Utf8Path) -> ColorMatrix {
        let img = im::open(input_file)
            .unwrap_or_else(|_| panic!("Unable to convert {input_file:?} properly."));
        let mat = ColorMatrix::convert(&img.to_rgb8(), ConversionRule::AnyModeUseAllPixels);
        self.transformations.apply(&mat)
    }

    /// Manage the conversion on the given sprite
    fn apply_sprite_conversion(&mut self, sprite: &Sprite) -> anyhow::Result<Output> {
        let output = self.output.clone();

        match output {
            OutputFormat::Sprite(SpriteEncoding::Linear) => {
                self.linearize_sprite(sprite).map(Output::Sprite)
            },
            OutputFormat::CPCMemory {
                ref output_dimension,
                ref display_address
            } => self.build_memory_blocks(sprite, *output_dimension, *display_address),
            OutputFormat::CPCSplittingMemory(ref _vec) => unimplemented!(),
            OutputFormat::TileEncoded {
                tile_width,
                tile_height,
                horizontal_movement,
                vertical_movement,
                grid_width,
                grid_height
            } => {
                self.extract_tiles(
                    tile_width,
                    tile_height,
                    horizontal_movement,
                    vertical_movement,
                    grid_width,
                    grid_height,
                    sprite
                )
            },

            _ => unreachable!()
        }
    }

    /// Produce the linearized version of the sprite.
    /// TODO add size constraints to keep a small part of the sprite
    fn linearize_sprite(&mut self, sprite: &Sprite) -> anyhow::Result<SpriteOutput> {
        Ok(SpriteOutput {
            encoding: SpriteEncoding::Linear,
            mode: sprite.mode.unwrap(),
            data: sprite.to_linear_vec(),
            palette: sprite.palette.as_ref().unwrap().clone(), /* By definition, we expect the palette to be set */
            bytes_width: sprite.bytes_width() as _,
            height: sprite.height() as _
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_tiles(
        &mut self,
        tile_width: TileWidthCapture,
        tile_height: TileHeightCapture,
        horizontal_movement: TileHorizontalCapture,
        vertical_movement: TileVerticalCapture,
        grid_width: GridWidthCapture,
        grid_height: GridHeightCapture,
        sprite: &Sprite
    ) -> anyhow::Result<Output> {
        // Compute the real value of the arguments
        let tile_width = match tile_width {
            TileWidthCapture::FullWidth => sprite.bytes_width(),
            TileWidthCapture::NbBytes(nb) => nb as _
        };
        let tile_height = match tile_height {
            TileHeightCapture::FullHeight => sprite.height(),
            TileHeightCapture::NbLines(nb) => nb as _
        };
        let nb_columns = match grid_width {
            GridWidthCapture::TilesInRow(nb) => nb,
            GridWidthCapture::FullWidth => sprite.bytes_width() as usize / tile_width as usize
        };
        let nb_rows = match grid_height {
            GridHeightCapture::TilesInColumn(nb) => nb,
            GridHeightCapture::FullHeight => sprite.height() as usize / tile_height as usize
        };

        if (sprite.height() as usize) < tile_height as usize * nb_rows {
            return Err(anyhow::anyhow!(
                "{} lines expected on a tileset of {} lines.",
                tile_height as usize * nb_rows,
                sprite.height()
            ));
        }
        if (sprite.bytes_width() as usize) < tile_width as usize * nb_columns {
            return Err(anyhow::anyhow!(
                "{} byte-columns  expected on a tileset of {} byte-columns.",
                tile_width as usize * nb_columns,
                sprite.bytes_width()
            ));
        }

        // Really makes the extraction
        let mut tiles_list: Vec<Vec<u8>> = Vec::new();
        for row in 0..nb_rows {
            for column in 0..nb_columns {
                // TODO add an additional parameter to read x before y
                // Manage the sprite in this cell
                let mut y_counter = vertical_movement.counter();
                let mut x_counter = horizontal_movement.counter();
                let mut current_tile: Vec<u8> = Vec::new();
                for _y in 0..tile_height {
                    for x in 0..tile_width {
                        // Get the line of interest
                        let real_line =
                            row * tile_height as usize + y_counter.get_line_index_in_screen();

                        let real_col = column * tile_width as usize + x_counter.get_column_index();
                        if x != tile_width - 1 {
                            x_counter.next();
                        }

                        // Get the byte from the sprite ...
                        //    dbg!(real_line, real_col);
                        let byte: u8 = sprite.get_byte(real_col, real_line);

                        // ... and store it at the right place
                        current_tile.push(byte);
                    }
                    x_counter.line_ended();
                    y_counter.next();
                }
                tiles_list.push(current_tile);
            }
        }

        // build the object to return
        Ok(Output::TilesList {
            tile_height,
            tile_width,
            horizontal_movement,
            vertical_movement,
            palette: sprite.palette().unwrap(),
            list: tiles_list
        })
    }

    /// Manage the creation of the memory blocks
    /// XXX Warning, overscan is wrongly used, it is more fullscreen with 2 pages
    fn build_memory_blocks(
        &mut self,
        sprite: &Sprite,
        dim: CPCScreenDimension,
        display_address: DisplayAddress
    ) -> anyhow::Result<Output> {
        let screen_width = u32::from(dim.width(sprite.mode().unwrap()));
        let screen_height = u32::from(dim.height());

        // Check if the destination is compatible
        if screen_width < sprite.pixel_width() {
            if !self.crop_if_too_large {
                return Err(anyhow::anyhow!(
                    "The image width ({}) is larger than the cpc screen width ({})",
                    sprite.pixel_width(),
                    screen_width
                ));
            }
        }
        else if screen_width > sprite.pixel_width() {
            eprintln!(
                "[Warning] The image width ({}) is smaller than the cpc screen width ({})",
                sprite.pixel_width(),
                screen_width
            );
        }

        if screen_height < sprite.height() {
            if !self.crop_if_too_large {
                return Err(anyhow::anyhow!(
                    "The image height ({}) is larger than the cpc screen height ({})",
                    sprite.height(),
                    screen_height
                ));
            }
        }
        else if screen_height > sprite.height() {
            eprintln!(
                "[Warning] The image height ({}) is smaller than the cpc screen height ({})",
                sprite.height(),
                screen_height
            );
        }

        // Simulate the memory
        let mut pages = [[0; 0x4000], [0; 0x4000], [0; 0x4000], [0; 0x4000]];

        let mut used_pages = HashSet::new();
        let is_overscan = dim.use_two_banks();
        if !is_overscan && display_address.is_overscan() {
            return Err(anyhow::anyhow!(
                "Image requires an overscan configuration for R12/R13={:?}",
                display_address
            ));
        }

        let mut current_address = display_address;
        used_pages.insert(current_address.page());

        // loop over the chars vertically
        for char_y in 0..dim
            .nb_char_lines()
            .min((sprite.height() as f32 / 8_f32).ceil() as u8)
        {
            let char_y = char_y as usize;

            // loop over the chars horiontally (2 bytes)
            for char_x in 0..dim.nb_word_columns() {
                let char_x = char_x as usize;

                // Loop over the lines of the current char (8 lines for a standard screen)
                for line_in_char in 0..dim.nb_lines_per_char() {
                    let line_in_char = line_in_char as usize;

                    // Loop over the bytes of the current char
                    for byte_nb in 0..2 {
                        let x_coord = 2 * char_x + byte_nb;
                        let y_coord = dim.nb_lines_per_char() as usize * char_y + line_in_char;

                        let value = sprite.get_byte_safe(x_coord, y_coord);
                        // let value = Some(sprite.get_byte(x_coord as _, y_coord as _));

                        match value {
                            None => {
                                // eprintln!("Unable to access byte in {}, {}", x_coord, y_coord);
                            },
                            Some(byte) => {
                                let page = current_address.page() as usize;
                                let address = current_address.offset() as usize * 2
                                    + byte_nb
                                    + line_in_char * 0x800;

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
            .map(|idx| pages[*idx as usize])
            .collect::<Vec<_>>();

        if is_overscan && used_pages.len() != 2 {
            // return Err(anyhow::anyhow!(
            // "An overscan screen is requested but {} pages has been feed",
            // used_pages.len()
            // ));

            eprintln!(
                "An overscan screen is requested but {} pages has been feed",
                used_pages.len()
            ); // TODO need to investigate
        }

        // Generate the right output format
        let palette = sprite.palette().unwrap();
        if is_overscan {
            Ok(Output::CPCMemoryOverscan(
                used_pages[0],
                used_pages[1],
                palette
            ))
        }
        else {
            Ok(Output::CPCMemoryStandard(used_pages[0], palette))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::convert::*;

    #[test]
    fn overscan_test() {
        assert!(CPCScreenDimension::overscan().use_two_banks());
        assert!(!CPCScreenDimension::standard().use_two_banks());
    }

    #[test]
    fn manipulation_test() {
        let mut address = DisplayAddress::new_from(0x3000);

        assert_eq!(address.address(), 0xC000);
        assert_eq!(address.r12(), 0x30);
        assert_eq!(address.r13(), 0x00);
        assert!(!address.is_overscan());

        address.set_page(1);
        assert_eq!(address.page(), 1);
        assert_eq!(address.address(), 0x4000);

        address.move_to_next_word();
        assert_eq!(address.address(), 0x4002);
    }

    #[test]
    fn test_masking() {
        let fg1 = Ink::BLUE;
        let fg2 = Ink::SKY_BLUE;
        let fg3 = Ink::PASTEL_BLUE;
        let fg4 = Ink::BRIGHT_BLUE;
        let bg_ = Ink::RED;
        let rep = Ink::BLACK;

        let sprite_with_mask = ColorMatrix::from(vec![
            vec![bg_, fg2, fg3, fg4],
            vec![bg_, bg_, fg1, fg2],
            vec![bg_, bg_, bg_, fg3],
            vec![bg_, bg_, bg_, fg4],
            vec![bg_, bg_, bg_, bg_],
        ]);

        let transformation_sprite = Transformation::ReplaceInk { from: bg_, to: rep };
        let transformation_mask = Transformation::MaskFromBackgroundInk(bg_);

        let sprite = transformation_sprite.apply(&sprite_with_mask);
        let mask = transformation_mask.apply(&sprite_with_mask);

        let (mask2, sprite2) = sprite_with_mask.extract_mask_and_sprite(bg_, rep);

        assert_eq!(mask, mask2);
        assert_eq!(sprite, sprite2);
    }
}

//! The aim of this file is to ease debuggin of the conversion from Basic commands to Char commands
//! by simulating an Amstrad CPC screen and interpreting the char commands to produce a visual output.
//! This is mainly useful for tests.
//! #[derive(Clone, Debug)]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
    pub visible: bool
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            x: 1,
            y: 1,
            visible: true
        }
    }

    pub fn locate(&mut self, x: u16, y: u16, mode: &Mode) {
        let (width, height) = mode.resolution();
        self.x = x.clamp(1, width);
        self.y = y.clamp(1, height);
    }

    pub fn inc_x(&mut self, mode: &Mode) {
        let (width, _) = mode.resolution();
        self.x += 1;
        if self.x > width {
            self.x = 1;
            self.y += 1;
        }
    }

    pub fn dec_x(&mut self, mode: &Mode) {
        let (width, _) = mode.resolution();
        if self.x > 1 {
            self.x -= 1;
        }
        else {
            self.x = width;
            self.dec_y(mode);
        }
    }

    pub fn inc_y(&mut self, mode: &Mode) {
        let (_, height) = mode.resolution();
        self.y += 1;
        if self.y > height {
            self.y = height;
        }
    }

    pub fn dec_y(&mut self, mode: &Mode) {
        if self.y > 1 {
            self.y -= 1;
        }
    }
}

use std::fmt::{self, Display, Write};

use crate::basic_chars::{ACK, CURSOR_1};
use crate::basic_command::{BasicCommand, BasicCommandList, PrintArgument};
use crate::char_command::{CharCommand, CharCommandList};

/// Locale/Language for character font
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    /// English (UK) font
    English,
    /// French font
    French,
    /// Spanish font (placeholder using English)
    Spanish,
    /// German font (placeholder using English)
    German,
    /// Danish font (placeholder using English)
    Danish
}

impl Default for Locale {
    fn default() -> Self {
        Locale::English
    }
}

impl Locale {
    /// Get the font data for this locale
    fn font_data(&self) -> &'static [u8] {
        match self {
            Locale::English => include_bytes!("fonts/font_english.bin"),
            Locale::French => include_bytes!("fonts/font_french.bin"),
            // For now, use English as fallback for other languages
            // Users can add proper fonts by extracting from ROMs at:
            // https://www.cpcwiki.eu/index.php/ROM_List#Lower_ROMs
            Locale::Spanish => include_bytes!("fonts/font_spanish.bin"),
            Locale::German => include_bytes!("fonts/font_english.bin"),
            Locale::Danish => include_bytes!("fonts/font_danish.bin")
        }
    }
}

// CPC screen memory layout constants
const CPC_SCREEN_MEMORY_SIZE: usize = 0x4000; // 16KB video memory
const CPC_LINE_INTERLEAVE: usize = 0x800; // Distance between pixel lines within a char row
const CPC_BYTES_PER_CHARACTER_LINE: usize = 80; // Bytes per character row
const CPC_SCREEN_HEIGHT_PIXELS: usize = 200; // Screen height in pixels
const CPC_CHARACTER_HEIGHT: usize = 8; // Character height in pixels

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Mode0,
    Mode1,
    Mode2
}

impl Mode {
    /// Returns the resolution (width, height) for the given mode (in chars).
    pub fn resolution(&self) -> (u16, u16) {
        match self {
            Mode::Mode0 => (160 / 8, 200 / 8),
            Mode::Mode1 => (320 / 8, 200 / 8),
            Mode::Mode2 => (640 / 8, 200 / 8)
        }
    }

    pub fn buffer(&self) -> Vec<Vec<u8>> {
        let (width, height) = self.resolution();
        vec![vec![b' '; width as usize]; height as usize]
    }

    /// Convert Mode to cpclib_image::image::Mode
    pub fn to_image_mode(&self) -> cpclib_image::image::Mode {
        match self {
            Mode::Mode0 => cpclib_image::image::Mode::Zero,
            Mode::Mode1 => cpclib_image::image::Mode::One,
            Mode::Mode2 => cpclib_image::image::Mode::Two
        }
    }

    pub fn max_pens(&self) -> usize {
        match self {
            Mode::Mode0 => 16, // Mode 0 supports 16 colors (pens 0-15)
            Mode::Mode1 => 4,  // Mode 1 supports 4 colors (pens 0-3)
            Mode::Mode2 => 2   // Mode 2 supports 2 colors (pens 0-1)
        }
    }
}

use bon::{self, builder};
use cpclib_common::smallvec::SmallVec;
use cpclib_image::ga::{Palette, Pen};
use cpclib_image::pixels;

/// Pixel-accurate memory representation of the CPC screen
///
/// # Examples
///
/// ```
/// use cpclib_catart::interpret::{BasicMemoryScreen, Locale};
/// use cpclib_image::ga::Palette;
/// use cpclib_image::image::Mode;
///
/// // Create screen with default English locale
/// let screen = BasicMemoryScreen::new(Mode::One, Palette::default());
///
/// // Create screen with French locale
/// let screen_fr = BasicMemoryScreen::new_with_locale(Mode::One, Palette::default(), Locale::French);
/// ```
#[derive(Clone, Debug)]
pub struct BasicMemoryScreen {
    mode: cpclib_image::image::Mode,
    palette: Palette,
    locale: Locale,
    memory: [u8; 0x4000],
    r12r13: u16 // CRTC registers R12/R13 - screen start address offset
}

impl BasicMemoryScreen {
    /// Create a new screen with default English locale
    pub fn new(mode: cpclib_image::image::Mode, palette: Palette) -> Self {
        Self::new_with_locale(mode, palette, Locale::default())
    }

    /// Create a new screen with specified locale
    pub fn new_with_locale(
        mode: cpclib_image::image::Mode,
        palette: Palette,
        locale: Locale
    ) -> Self {
        Self {
            mode,
            palette,
            locale,
            memory: [0; CPC_SCREEN_MEMORY_SIZE],
            r12r13: 0
        }
    }

    /// Get bytes per character for the current mode
    fn bytes_per_char(&self) -> usize {
        match self.mode {
            cpclib_image::image::Mode::Zero => 2, /* Mode 0: 2 bytes per char (2 bits/pixel, 4 pixels/byte) */
            cpclib_image::image::Mode::One => 2, /* Mode 1: 2 bytes per char (2 bits/pixel, 4 pixels/byte) */
            cpclib_image::image::Mode::Two => 1, /* Mode 2: 1 byte per char (1 bit/pixel, 8 pixels/byte) */
            _ => 2
        }
    }

    /// Get screen width in pixels for the current mode
    fn screen_width_pixels(&self) -> usize {
        match self.mode {
            cpclib_image::image::Mode::Zero => 160,
            cpclib_image::image::Mode::One => 320,
            cpclib_image::image::Mode::Two => 640,
            _ => 320
        }
    }

    pub fn reset_mode(&mut self, mode: cpclib_image::image::Mode, pen: Pen, paper: Pen) {
        self.mode = mode;
        self.r12r13 = 0; // Reset CRTC offset on mode change
        self.cls(pen, paper);
    }

    pub fn cls(&mut self, pen: Pen, paper: Pen) {
        // Clear memory with paper color pattern
        let paper_pattern = Self::get_paper_pattern(paper, self.mode);
        for byte in self.memory.iter_mut() {
            *byte = paper_pattern;
        }
    }

    /// Get the byte pattern for a solid color (used for paper)
    fn get_paper_pattern(pen: Pen, mode: cpclib_image::image::Mode) -> u8 {
        // Create a pattern of 8 pixels all with the same pen color
        let pens = vec![pen; 8];
        let bytes = pixels::pens_to_bytes(&pens, mode);
        bytes[0] // Return first byte (pattern repeats)
    }

    /// Get the 8x8 bitmap for a character using the screen's locale font
    fn get_char_bitmap(&self, ch: u8) -> &'static [u8; 8] {
        let font_data = self.locale.font_data();
        // Font data is directly from CPC ROM (no header)
        let offset = (ch as usize) * 8;
        if offset + 8 <= font_data.len() {
            unsafe { &*(font_data.as_ptr().add(offset) as *const [u8; 8]) }
        }
        else {
            // Return empty bitmap for out of range chars
            &[0; 8]
        }
    }

    /// Convert a bitmap line (8 bits) to an array of Pens based on pen/paper colors
    fn bitmap_to_pens(bitmap_line: u8, pen: Pen, paper: Pen) -> [Pen; 8] {
        let mut pens = [paper; 8];
        for bit_idx in 0..8 {
            let mask = 1 << (7 - bit_idx);
            if (bitmap_line & mask) != 0 {
                pens[bit_idx] = pen;
            }
        }
        pens
    }

    /// Clamp pen number to valid range for current mode
    /// Mode 0 and Mode 1: pens must be 0-3 (4 colors)
    /// Mode 2: pens must be 0-1 (2 colors)
    fn clamp_pen_for_mode(&self, pen: Pen) -> Pen {
        let pen_number = pen.number();
        let max_pen = match self.mode {
            cpclib_image::image::Mode::Zero => 16, // Mode 0: 16 colors (pens 0-15)
            cpclib_image::image::Mode::One => 3,   // Mode 1: 4 colors (pens 0-3)
            cpclib_image::image::Mode::Two => 1,   // Mode 2: 2 colors (pens 0-1)
            _ => 3
        };

        if pen_number <= max_pen {
            pen
        }
        else {
            // Use modulo to wrap pen number into valid range
            Pen::from(pen_number % (max_pen))
        }
    }

    /// Write a character at the given position (x, y in character coordinates)
    /// If transparent is true, the character is overlaid on existing pixels (transparent mode)
    pub fn write_char(
        &mut self,
        ch: u8,
        char_x: u16,
        char_y: u16,
        pen: Pen,
        paper: Pen,
        transparent: bool
    ) {
        // CPC coordinates are 1-based, so we need to check for valid values
        if char_x == 0 || char_y == 0 {
            return; // Invalid coordinates, do nothing
        }

        // Clamp pen and paper to valid range for current mode to prevent assertion failures
        let pen = self.clamp_pen_for_mode(pen);
        let paper = self.clamp_pen_for_mode(paper);

        // Calculate how many bytes we need for this character line
        let char_width_in_bytes = self.mode.nb_bytes_for_char();

        // Get the character bitmap for this locale
        let char_bitmap = self.get_char_bitmap(ch);
        let char_pens: SmallVec<[[Pen; 8]; 8]> = char_bitmap
            .iter()
            .map(|&line| Self::bitmap_to_pens(line, pen, paper))
            .collect();

        let bloc_x = (char_x - 1) * char_width_in_bytes as u16;
        let bloc_y = char_y - 1;

        // we treat column per colummn (so 1 byte)
        for bloc_x_delta in 0 as u16..char_width_in_bytes as u16 {
            let bloc_addr = self.address_for_byte_bloc(bloc_x_delta + bloc_x, bloc_y) as usize;
            for line_idx in 0..8 {
                // Convert bitmap line to pens
                let char_pens = &char_pens[line_idx];

                let byte_addr = (bloc_addr + 0x800 * line_idx) as usize;
                assert!(
                    byte_addr < CPC_SCREEN_MEMORY_SIZE,
                    "Calculated byte address out of bounds"
                );

                // Write the bytes to memory
                let screen_byte = if transparent {
                    // Transparent mode: merge with existing pixels
                    // Read existing bytes from memory
                    let existing_byte = self.memory[byte_addr];

                    // Convert existing bytes to pens
                    let existing_pens: SmallVec<[Pen; 8]> =
                        pixels::byte_to_pens(existing_byte, self.mode).collect();
                    debug_assert_eq!(
                        existing_pens.len(),
                        self.mode.nb_pixels_per_byte(),
                        "Expected exactly {} pens for a character line",
                        self.mode.nb_pixels_per_byte()
                    );

                    // Merge: for each pixel, use char pen if bitmap bit is set, otherwise keep existing pen
                    let mut merged_pens: SmallVec<[Pen; 8]> = existing_pens
                        .iter()
                        .enumerate()
                        .map(|(i, current_pen)| {
                            let mask = 1 << (7 - i);
                            if (char_bitmap[line_idx] & mask) != 0 {
                                // Bitmap pixel is set: use character pen
                                pen
                            }
                            else {
                                // Bitmap pixel is not set: keep existing pixel (transparent)
                                *current_pen
                            }
                        })
                        .collect();

                    // Convert merged pens back to bytes
                    let merged_bytes: Vec<u8> = pixels::pens_to_bytes(&merged_pens, self.mode);
                    assert_eq!(
                        merged_bytes.len(),
                        1,
                        "We treat one byte at a time, expected exactly 1 byte for merged character line"
                    );

                    merged_bytes[0]
                }
                else {
                    let bloc_x_delta = bloc_x_delta as usize;
                    let nb_pix_per_byte = self.mode.nb_pixels_per_byte();
                    // Opaque mode: write character directly with pen/paper
                    let bytes = pixels::pens_to_bytes(
                        &char_pens[bloc_x_delta * nb_pix_per_byte
                            ..((bloc_x_delta + 1) * nb_pix_per_byte)],
                        self.mode
                    );
                    assert_eq!(
                        bytes.len(),
                        1,
                        "We treat one byte at a time, expected exactly 1 byte for character line"
                    );
                    bytes[0]
                };
                self.memory[byte_addr] = screen_byte;
            }
        }
    }

    /// Convert the memory to pens (2D array of Pen values)
    pub fn to_pens(&self) -> Vec<Vec<Pen>> {
        let width_pixels = self.screen_width_pixels();
        let height_pixels = CPC_SCREEN_HEIGHT_PIXELS;

        let mut pens = Vec::with_capacity(height_pixels);

        for y in 0..height_pixels {
            let mut line_pens = Vec::with_capacity(width_pixels);

            // Calculate the memory offset for this line (interleaved CPC format)
            // CPC memory: character rows are 80 bytes apart, pixel lines within a char row are 0x800 bytes apart
            let char_row = y / CPC_CHARACTER_HEIGHT;
            let pixel_line_in_char = y % CPC_CHARACTER_HEIGHT;
            let base_offset =
                char_row * CPC_BYTES_PER_CHARACTER_LINE + pixel_line_in_char * CPC_LINE_INTERLEAVE;

            // DO NOT apply R12R13 offset when reading - read raw memory
            // The R12R13 offset is only for WRITING characters to scrolled positions
            // When reading for comparison, we read the raw memory as-is
            let line_offset = base_offset;

            // All modes use 80 bytes per line (full character row width)
            let bytes_per_line = CPC_BYTES_PER_CHARACTER_LINE;

            // Read bytes for this line and convert to pens
            let line_bytes = &self.memory[line_offset..line_offset + bytes_per_line];
            let pixels_iter = pixels::bytes_to_pens(line_bytes, self.mode);
            line_pens.extend(pixels_iter.take(width_pixels));

            pens.push(line_pens);
        }

        pens
    }

    /// Convert the memory screen to a Sprite
    pub fn to_sprite(&self) -> cpclib_image::image::Sprite {
        let pens = self.to_pens();
        cpclib_image::image::Sprite::from_pens(&pens, self.mode, Some(self.palette.clone()))
    }

    pub fn to_color_matrix(&self) -> Option<cpclib_image::image::ColorMatrix> {
        self.to_color_matrix_with_border(3, 4)
    }

    /// Convert the memory screen to a ColorMatrix (with border)
    pub fn to_color_matrix_with_border(
        &self,
        border_horizontal: usize,
        border_vertical: usize
    ) -> Option<cpclib_image::image::ColorMatrix> {
        // First convert to sprite
        let sprite = self.to_sprite();

        // Then convert sprite to color matrix
        let mut color_matrix = sprite.to_color_matrix()?;

        // For Mode 1, duplicate each line and column to match ground truth size
        // This makes it easier to compare with reference images
        if let cpclib_image::image::Mode::One = self.mode {
            // Duplicate lines first
            let height = color_matrix.height() as usize;
            for line_idx in (0..height).rev() {
                let line = color_matrix.get_line(line_idx).to_vec();
                color_matrix.add_line((line_idx + 1) as usize, &line);
            }
            // Then duplicate columns
            let width = color_matrix.width() as usize;
            for col_idx in (0..width).rev() {
                let column = color_matrix.get_column(col_idx).to_vec();
                color_matrix.add_column((col_idx + 1) as usize, &column);
            }
        }

        // For Mode 2, duplicate each line to maintain correct aspect ratio
        // Mode 2 pixels are twice as thin on real hardware, so each scanline should appear twice
        if let cpclib_image::image::Mode::Two = self.mode {
            let height = color_matrix.height() as usize;
            // Work backwards to avoid index issues when inserting
            for line_idx in (0..height).rev() {
                // Get the line data
                let line = color_matrix.get_line(line_idx).to_vec();
                // Insert a duplicate right after the current line
                color_matrix.add_line((line_idx + 1) as usize, &line);
            }
        }

        // For Mode 0, duplicate each column to maintain correct aspect ratio
        // Mode 0 pixels are wider on real hardware
        if let cpclib_image::image::Mode::Zero = self.mode {
            let width = color_matrix.width() as usize;
            // Work backwards to avoid index issues when inserting
            for col_idx in (0..width).rev() {
                // Get the column data
                let column = color_matrix.get_column(col_idx).to_vec();
                // Insert a duplicate right after the current column
                color_matrix.add_column((col_idx + 1) as usize, &column);
            }
        }

        // Add border around the image
        let border_ink = self.palette.get_border();

        // Step 1: Add left border columns (insert at position 0, border_horizontal times)
        let left_column = vec![*border_ink; color_matrix.height() as usize];
        for _ in 0..border_horizontal {
            color_matrix.add_column(0, &left_column);
        }

        // Step 2: Add right border columns (append at the end)
        let right_column = vec![*border_ink; color_matrix.height() as usize];
        for _ in 0..border_horizontal {
            color_matrix.add_column(color_matrix.width() as usize, &right_column);
        }

        // Step 3: Add top border lines (insert at position 0, border_vertical times)
        // Now the width has increased, so create border_line with new width
        let top_line = vec![*border_ink; color_matrix.width() as usize];
        for _ in 0..border_vertical {
            color_matrix.add_line(0, &top_line);
        }

        // Step 4: Add bottom border lines (append at the end)
        // The height has increased from adding top lines, but add_line handles that
        let bottom_line = vec![*border_ink; color_matrix.width() as usize];
        for _ in 0..border_vertical {
            color_matrix.add_line(color_matrix.height() as usize, &bottom_line);
        }

        Some(color_matrix)
    }

    /// Get a reference to the memory buffer
    pub fn memory(&self) -> &[u8; CPC_SCREEN_MEMORY_SIZE] {
        &self.memory
    }

    /// Get the current mode
    pub fn mode(&self) -> cpclib_image::image::Mode {
        self.mode
    }

    /// Get the current palette
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Get the current CRTC R12R13 register value (screen start offset)
    pub fn r12r13(&self) -> u16 {
        self.r12r13
    }

    /// Set a palette entry
    pub fn set_palette(&mut self, pen: Pen, ink: cpclib_image::ga::Ink) {
        self.palette.set(pen, ink);
    }

    /// Set the border ink
    pub fn set_border(&mut self, ink: cpclib_image::ga::Ink) {
        self.palette.set_border(ink);
    }

    /// Hardware scroll up: increment R12R13 by 40 (one character line)
    pub fn hardware_scroll_up(&mut self) {
        self.r12r13 = (self.r12r13 + 40) & 0x3FF; // Wrap at 0x2000 (half of 0x4000 since R12R13 is in words)
    }

    /// A bloc is one byte width and 8 line tall.
    /// A mode 1 char is made of 2 blocs
    pub fn address_for_byte_bloc(&self, bloc_x: u16, bloc_y: u16) -> u16 {
        assert!(
            bloc_x < 80 && bloc_y < 25,
            "Character coordinates out of bounds"
        );

        let delta = bloc_y * 80 + bloc_x;
        let address = (self.r12r13() * 2 + delta) & 0x7FF;
        address
    }

    /// Clear a specific character line with paper color
    pub fn clear_line(&mut self, y: u16, paper: Pen) {
        let paper_pattern = Self::get_paper_pattern(paper, self.mode);

        // Clear all 8 pixel rows of this character line
        for line_idx in 0..8 {
            let pixel_y = (y - 1) * 8 + line_idx as u16;
            let base_addr = (0x800 * (pixel_y % 8) + 80 * (pixel_y / 8)) as usize;

            // Apply R12R13 offset and clear the entire line
            for byte_offset in 0..CPC_BYTES_PER_CHARACTER_LINE {
                let final_addr = ((base_addr + byte_offset) + (2 * self.r12r13 as usize))
                    % CPC_SCREEN_MEMORY_SIZE;
                self.memory[final_addr] = paper_pattern;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScreenCell {
    pub ch: u8,
    pub pen: Pen,
    pub paper: Pen
}

impl ScreenCell {
    pub fn make(ch: u8, pen: Pen, paper: Pen) -> Self {
        Self { ch, pen, paper }
    }
}

#[derive(Clone, Debug)]
pub struct Screen {
    mode: Mode,
    buffer: Vec<Vec<ScreenCell>>
}

impl Screen {
    pub fn reset_mode(&mut self, mode: Mode, pen: Pen, paper: Pen) {
        self.mode = mode;
        self.buffer = Screen::make_buffer(&self.mode, pen, paper);
    }

    pub fn cls(&mut self, pen: Pen, paper: Pen) {
        self.buffer = Screen::make_buffer(&self.mode, pen, paper);
    }

    pub fn resolution(&self) -> (u16, u16) {
        self.mode.resolution()
    }

    fn make_buffer(mode: &Mode, pen: Pen, paper: Pen) -> Vec<Vec<ScreenCell>> {
        let (width, height) = mode.resolution();
        vec![vec![ScreenCell::make(b' ', pen, paper); width as usize]; height as usize]
    }

    pub fn cell_mut(&mut self, x: u16, y: u16) -> Option<&mut ScreenCell> {
        let (width, height) = self.resolution();
        if x == 0 || y == 0 || x > width || y > height {
            None
        }
        else {
            Some(&mut self.buffer[(y - 1) as usize][(x - 1) as usize])
        }
    }

    pub fn cell(&self, x: u16, y: u16) -> Option<&ScreenCell> {
        let (width, height) = self.resolution();
        if x == 0 || y == 0 || x > width || y > height {
            None
        }
        else {
            Some(&self.buffer[(y - 1) as usize][(x - 1) as usize])
        }
    }
}

/// Interpreter for CPC BASIC character commands
pub struct Interpreter {
    enable_vdu: bool,
    screen: Screen,
    memory_screen: BasicMemoryScreen, // Pixel-accurate representation
    cursor: Cursor,
    palette: Palette,
    pen: Pen,
    paper: Pen,
    border: Pen,
    window: Option<(u16, u16, u16, u16)>, // (left, right, top, bottom)
    transparent: bool,                    // Transparency mode for character printing
    locale: Locale
}

#[bon::bon]
impl Interpreter {
    /// Public getter for the screen (for testing)
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Public getter for the palette
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Public getter for the pixel-accurate memory screen
    pub fn memory_screen(&self) -> &BasicMemoryScreen {
        &self.memory_screen
    }

    pub fn new_6128() -> Self {
        Self::builder()
            .as_6128(true)
            .screen_mode(Mode::Mode1)
            .locale(Locale::English)
            .build()
    }

    #[builder]
    pub fn new(screen_mode: Mode, locale: Locale, as_6128: bool) -> Self {
        let pen = Pen::Pen1;
        let paper = Pen::Pen0;
        let palette = Palette::default();
        let image_mode = screen_mode.to_image_mode();

        let mut interpreter = Self {
            screen: Screen {
                mode: screen_mode.clone(),
                buffer: Screen::make_buffer(&screen_mode, pen, paper)
            },
            memory_screen: BasicMemoryScreen::new_with_locale(image_mode, palette.clone(), locale),
            cursor: Cursor::new(),
            palette,
            pen,
            paper,
            border: paper,
            window: None,
            enable_vdu: true,
            transparent: false,
            locale
        };

        if as_6128 {
            interpreter.initialize_6128();
        }

        interpreter
    }

    fn initialize_6128(&mut self) {
        let start_text = BasicCommandList::from(vec![
            BasicCommand::move_cursor_down(),
            BasicCommand::print_string_crlf(b" Amstrad 128K Microcomputer  (v3)"),
            BasicCommand::move_cursor_down(),
            BasicCommand::print_string(b" "),
            BasicCommand::print_string(PrintArgument::ChrDollar(0xA4)),
            BasicCommand::print_string_crlf(b"1985 Amstrad Consumer Electronics plc"),
            BasicCommand::print_string_crlf(b"           and Locomotive Software Ltd."),
            BasicCommand::move_cursor_down(),
            BasicCommand::print_string_crlf(b" BASIC 1.1"),
            BasicCommand::move_cursor_down(),
        ]);

        let commands = start_text
            .to_char_commands()
            .expect("Char command conversion failed");
        self.interpret(commands.as_slice(), true)
            .expect("Initialization failed");
    }

    pub fn inc_cursor_x(&mut self) {
        let (width, _) = self.screen.resolution();
        let (left, right, ..) = self
            .window
            .unwrap_or((1, width, 1, self.screen.resolution().1));
        self.cursor.x += 1;
        if self.cursor.x > right {
            self.cursor.x = left;
            // Use inc_cursor_y to properly handle scrolling when wrapping to next line
            self.inc_cursor_y();
        }
    }

    pub fn dec_cursor_x(&mut self) {
        let (width, _) = self.screen.resolution();
        let (left, right, top, bottom) =
            self.window
                .unwrap_or((1, width, 1, self.screen.resolution().1));
        if self.cursor.x > left {
            self.cursor.x -= 1;
        }
        else {
            self.cursor.x = right;
            if self.cursor.y > top {
                self.cursor.y -= 1;
            }
        }
    }

    pub fn inc_cursor_y(&mut self) {
        let (_, height) = self.screen.resolution();
        let (left, right, top, bottom) =
            self.window
                .unwrap_or((1, self.screen.resolution().0, 1, height));
        self.cursor.y += 1;
        if self.cursor.y > bottom {
            // Cursor went past bottom - scroll and keep at bottom
            self.scroll_screen_up();
            self.cursor.y = bottom;
        }
    }

    pub fn dec_cursor_y(&mut self) {
        let (_, height) = self.screen.resolution();
        let (left, right, top, bottom) =
            self.window
                .unwrap_or((1, self.screen.resolution().0, 1, height));
        if self.cursor.y > top {
            self.cursor.y -= 1;
        }
    }

    pub fn locate_cursor(&mut self, x: u16, y: u16) {
        let (width, height) = self.screen.resolution();
        let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));

        // LOCATE command provides 0-based coordinates, convert to 1-based for cursor
        self.cursor.x = (x + 1).clamp(left, right);
        self.cursor.y = (y + 1).clamp(top, bottom);
    }

    pub fn move_cursor_to_left(&mut self) {
        let (width, _) = self.screen.resolution();
        let (left, _, top, bottom) =
            self.window
                .unwrap_or((1, width, 1, self.screen.resolution().1));
        self.cursor.x = left;
        if self.cursor.y < top {
            self.cursor.y = top;
        }
        else if self.cursor.y > bottom {
            self.cursor.y = bottom;
        }
    }

    /// Write a character to both text screen and pixel-accurate memory screen
    /// This centralizes the char writing logic and transparency handling
    fn write_char_to_screens(&mut self, ch: u8, x: u16, y: u16) {
        // Update text screen
        if let Some(cell) = self.screen.cell_mut(x, y) {
            cell.ch = ch;
            cell.pen = self.pen;
            cell.paper = self.paper;
        }

        // Update pixel-accurate memory screen with transparency support
        self.memory_screen
            .write_char(ch, x, y, self.pen, self.paper, self.transparent);
    }

    pub fn scroll_screen_up(&mut self) {
        let width = self.screen.resolution().0;
        let height = self.screen.resolution().1;
        let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));

        // Check if we're using hardware scrolling (no window or full-screen window)
        let is_full_screen =
            self.window.is_none() || (left == 1 && right == width && top == 1 && bottom == height);

        if is_full_screen {
            // Hardware scroll: increment R12R13 and clear last line
            self.memory_screen.hardware_scroll_up();
            self.memory_screen.clear_line(height, self.paper);
        }

        // Always do software scroll for the character buffer
        // Collect source cells first to avoid borrow checker issues
        let mut next_row_cells =
            vec![
                vec![ScreenCell::make(b' ', self.pen, self.paper); (right - left + 1) as usize];
                (bottom - top + 1) as usize
            ];
        for y in top..=bottom {
            for x in left..=right {
                let next_y = y + 1;
                let idx_y = (y - top) as usize;
                let idx_x = (x - left) as usize;
                if next_y <= bottom {
                    if let Some(src) = self.screen.cell(x, next_y) {
                        next_row_cells[idx_y][idx_x] = src.clone();
                    }
                }
            }
        }
        for y in top..=bottom {
            for x in left..=right {
                let idx_y = (y - top) as usize;
                let idx_x = (x - left) as usize;
                if let Some(cell) = self.screen.cell_mut(x, y) {
                    let src = &next_row_cells[idx_y][idx_x];
                    *cell = src.clone();
                    // If this is the last row, clear it
                    if y == bottom {
                        cell.ch = b' ';
                        cell.pen = self.pen;
                        cell.paper = self.paper;
                    }
                }
            }
        }

        // If not full screen (windowed), also update the memory screen with software scroll
        // by redrawing all visible characters
        if !is_full_screen {
            for y in top..=bottom {
                for x in left..=right {
                    if let Some(cell) = self.screen.cell(x, y) {
                        self.memory_screen
                            .write_char(cell.ch, x, y, cell.pen, cell.paper, false);
                    }
                }
            }
        }
    }

    pub fn scroll_screen_down(&mut self) {
        let width = self.screen.resolution().0;
        let (left, right, top, bottom) =
            self.window
                .unwrap_or((1, width, 1, self.screen.resolution().1));
        // Collect source cells first to avoid borrow checker issues
        let mut prev_row_cells =
            vec![
                vec![ScreenCell::make(b' ', self.pen, self.paper); (right - left + 1) as usize];
                (bottom - top + 1) as usize
            ];
        for y in (top..=bottom).rev() {
            for x in left..=right {
                let prev_y = if y > top { y - 1 } else { top };
                let idx_y = (y - top) as usize;
                let idx_x = (x - left) as usize;
                if prev_y >= top {
                    if let Some(src) = self.screen.cell(x, prev_y) {
                        prev_row_cells[idx_y][idx_x] = src.clone();
                    }
                }
            }
        }
        for y in (top..=bottom).rev() {
            for x in left..=right {
                let idx_y = (y - top) as usize;
                let idx_x = (x - left) as usize;
                if let Some(cell) = self.screen.cell_mut(x, y) {
                    let src = &prev_row_cells[idx_y][idx_x];
                    *cell = src.clone();
                    // If this is the first row, clear it
                    if y == top {
                        cell.ch = b' ';
                        cell.pen = self.pen;
                        cell.paper = self.paper;
                    }
                }
            }
        }
    }

    pub fn interpret<'a, I>(&mut self, commands: I, print_ready: bool) -> Result<(), String>
    where I: IntoIterator<Item = &'a CharCommand> {
        // Process main commands first
        for command in commands {
            self.interpret_command(command)?;
        }

        // If we need to print "Ready" and cursor, ensure we're at the start of a line
        if print_ready {
            // Get window bounds (or use full screen if no window defined)
            let (width, height) = self.screen.resolution();
            let (left, ..) = self.window.unwrap_or((1, width, 1, height));

            // Move to beginning of window line if not already there
            if self.cursor.x != left {
                self.interpret_command(&CharCommand::CarriageReturn)?;
                self.interpret_command(&CharCommand::CursorDown)?;
            }

            // Now add the finalize sequence
            let finalize = BasicCommandList::from(vec![
                BasicCommand::print_string(ACK), // activate visualisation
                BasicCommand::cursor_on(),       // show cursor
                BasicCommand::print_string_crlf(b"Ready"),
            ])
            .to_char_commands()
            .unwrap();

            for command in finalize {
                self.interpret_command(&command)?;
            }
        }

        // Draw cursor if visible (cursor is stored in CPC video memory)
        if self.cursor.visible {
            self.memory_screen.write_char(
                0x8F, // Cursor character
                self.cursor.x,
                self.cursor.y,
                self.pen,
                self.paper,
                false // Cursor is always opaque
            );
        }

        Ok(())
    }

    pub fn mode(&self) -> Mode {
        assert_eq!(self.screen.mode.to_image_mode(), self.memory_screen.mode());
        self.screen.mode.clone()
    }

    pub fn interpret_command(&mut self, command: &CharCommand) -> Result<(), String> {
        match command {
            CharCommand::Symbol(..) => {
                // Symbols are not simulated; ignore.
            },
            CharCommand::Nop => {
                // No operation - do nothing
            },
            CharCommand::SendGraphics(_) => {
                // Sending graphics is not simulated; ignore.
            },
            CharCommand::PrintSymbol(ch) | CharCommand::Char(ch) if self.enable_vdu => {
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));
                if self.cursor.y >= top
                    && self.cursor.y <= bottom
                    && self.cursor.x >= left
                    && self.cursor.x <= right
                {
                    // Write character using centralized method (handles transparency)
                    self.write_char_to_screens(*ch, self.cursor.x, self.cursor.y);
                    self.inc_cursor_x();
                }
            },
            CharCommand::Locate(x, y) if self.enable_vdu => {
                // Locate coordinates are relative to the window
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));
                // Convert window-relative (1-based) to absolute screen coordinates
                let abs_x = left + (*x as u16) - 1;
                let abs_y = top + (*y as u16) - 1;
                self.locate_cursor(abs_x, abs_y);
            },
            CharCommand::Cls if self.enable_vdu => {
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));

                for y in top..=bottom {
                    for x in left..=right {
                        self.write_char_to_screens(b' ', x, y);
                    }
                }
                self.locate_cursor(left, top);
            },
            CharCommand::Home if self.enable_vdu => {
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));
                self.locate_cursor(left, top);
            },
            CharCommand::Esc if self.enable_vdu => {
                // Escape is ignored.
            },

            CharCommand::CarriageReturn if self.enable_vdu => {
                self.move_cursor_to_left();
            },
            CharCommand::CursorDown if self.enable_vdu => {
                self.inc_cursor_y();
            },
            CharCommand::CursorUp if self.enable_vdu => {
                self.dec_cursor_y();
            },
            CharCommand::CursorLeft if self.enable_vdu => {
                self.dec_cursor_x();
            },
            CharCommand::CursorRight if self.enable_vdu => {
                self.inc_cursor_x();
            },
            CharCommand::DisableVdu => {
                self.enable_vdu = false;
            },
            CharCommand::EnableVdu => {
                self.enable_vdu = true;
            },
            CharCommand::GraphicsInkMode(_) => {
                // Graphics ink mode is not simulated; ignore.
            },
            CharCommand::Mode(m) if self.enable_vdu => {
                let m = match m {
                    0 => Mode::Mode0,
                    1 => Mode::Mode1,
                    2 => Mode::Mode2,
                    _ => return Err(format!("Invalid mode {}", m))
                };
                self.locate_cursor(1, 1);
                self.screen.reset_mode(m.clone(), self.pen, self.paper);
                self.memory_screen
                    .reset_mode(m.to_image_mode(), self.pen, self.paper);
                self.window = None;
                self.cursor.x = 1;
                self.cursor.y = 1;
            },
            CharCommand::Pen(p) if self.enable_vdu => {
                self.pen = Pen::from(*p);
            },
            CharCommand::Paper(p) if self.enable_vdu => {
                let p = (*p) % self.mode().max_pens() as u8;
                self.paper = Pen::from(p);
            },
            CharCommand::Ink(pen, ink1, _ink2) if self.enable_vdu => {
                let pen = (*pen).clamp(0, 15);
                let ink1 = (*ink1).clamp(0, 31);
                let pen = Pen::from(pen);
                let ink = cpclib_image::ga::Ink::from(ink1);
                self.palette.set(pen, ink); // blinking is ignored
                self.memory_screen.set_palette(pen, ink);
            },
            CharCommand::Border(ink1, _ink2) if self.enable_vdu => {
                let ink = cpclib_image::ga::Ink::from(*ink1);
                self.palette.set_border(ink);
                self.memory_screen.set_border(ink);
                self.border = Pen::Border;
            },
            CharCommand::ClearLineStartToCursor if self.enable_vdu => {
                let y = self.cursor.y;
                let x = self.cursor.x;
                let (left, ..) = self.window.unwrap_or((
                    1,
                    self.screen.resolution().0,
                    1,
                    self.screen.resolution().1
                ));
                for col in left..=x {
                    self.write_char_to_screens(b' ', col, y);
                }
            },
            CharCommand::ClearCursorToLineEnd if self.enable_vdu => {
                let y = self.cursor.y;
                let x = self.cursor.x;
                let (_, right, ..) = self.window.unwrap_or((
                    1,
                    self.screen.resolution().0,
                    1,
                    self.screen.resolution().1
                ));
                for col in x..=right {
                    self.write_char_to_screens(b' ', col, y);
                }
            },
            CharCommand::ClearScreenStart if self.enable_vdu => {
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));
                let cur_x = self.cursor.x;
                let cur_y = self.cursor.y;
                for y in top..=cur_y {
                    let max_x = if y == cur_y { cur_x } else { right };
                    for x in left..=max_x {
                        self.write_char_to_screens(b' ', x, y);
                    }
                }
            },
            CharCommand::ClearScreenEnd if self.enable_vdu => {
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));
                let cur_x = self.cursor.x;
                let cur_y = self.cursor.y;
                // Fill from cursor to right edge on current line
                for x in cur_x..=right {
                    self.write_char_to_screens(b' ', x, cur_y);
                }
                // Fill all lines below
                for y in (cur_y + 1)..=bottom {
                    for x in left..=right {
                        self.write_char_to_screens(b' ', x, y);
                    }
                }
            },
            CharCommand::Window(left, right, top, bottom) if self.enable_vdu => {
                // CharCommand stores 0-based, window needs 1-based
                self.window = Some((
                    *left as u16 + 1,
                    *right as u16 + 1,
                    *top as u16 + 1,
                    *bottom as u16 + 1
                ));
                // Position cursor at top-left corner (CharCommand is 0-based, locate_cursor expects 0-based)
                self.locate_cursor(*left as u16, *top as u16);
            },
            CharCommand::Transparency(p) if self.enable_vdu => {
                // Handle transparency mode
                // When p is 0, transparency is off (opaque)
                // When p is 1, transparency is on (transparent)
                self.transparent = (*p % 2) == 1;
            },
            CharCommand::ExchangePenAndPaper if self.enable_vdu => {
                std::mem::swap(&mut self.pen, &mut self.paper);
            },

            CharCommand::CursorOn => {
                self.cursor.visible = true;
            },
            CharCommand::CursorOff => {
                self.cursor.visible = false;
            },

            c if !self.enable_vdu => {
                // When VDU is disabled, ignore all commands except those handled above
            },

            CharCommand::Beep => {
                // ignore
            },

            c => {
                todo!(
                    "Interpreter: unhandled CharCommand {:?}. needs an implementation",
                    c
                );
            }
        }

        Ok(())
    }
}

impl Display for Interpreter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use cpclib_image::ga::Ink;
        use owo_colors::colors::CustomColor;
        use owo_colors::{DynColors, OwoColorize};
        // Border thickness
        let border = 4;
        let screen_width = self.screen.buffer[0].len();
        let screen_height = self.screen.buffer.len();
        let border_ink = self.palette.get_border();
        let border_rgb = border_ink.color();
        let border_color = DynColors::Rgb(border_rgb[0], border_rgb[1], border_rgb[2]);

        // Top border
        for _ in 0..border {
            for _ in 0..(screen_width + 2 * border) {
                write!(f, "{}", " ".on_color(border_color))?;
            }
            writeln!(f)?;
        }

        // Screen rows with left/right border
        for (row_index, row) in self.screen.buffer.iter().enumerate() {
            // Left border
            for _ in 0..border {
                write!(f, "{}", " ".on_color(border_color))?;
            }
            // Screen content
            for (col_index, cell) in row.iter().enumerate() {
                let (pen, paper, ch) = if row_index as u16 + 1 == self.cursor.y
                    && col_index as u16 + 1 == self.cursor.x
                {
                    // Caret: use current interpreter pen/paper
                    (self.pen, self.paper, '█')
                }
                else {
                    (cell.pen, cell.paper, cell.ch as char)
                };
                let pen_ink = self.palette.get(&pen);
                let paper_ink = self.palette.get(&paper);
                let pen_rgb = pen_ink.color();
                let paper_rgb = paper_ink.color();
                let fg = DynColors::Rgb(pen_rgb[0], pen_rgb[1], pen_rgb[2]);
                let bg = DynColors::Rgb(paper_rgb[0], paper_rgb[1], paper_rgb[2]);
                write!(f, "{}", ch.color(fg).on_color(bg))?;
            }
            // Right border
            for _ in 0..border {
                write!(f, "{}", " ".on_color(border_color))?;
            }
            writeln!(f)?;
        }

        // Bottom border
        for _ in 0..border {
            for _ in 0..(screen_width + 2 * border) {
                write!(f, "{}", " ".on_color(border_color))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Compares two screens and returns true if they are identical
pub fn screens_are_equal(screen1: &Screen, screen2: &Screen) -> bool {
    if screen1.buffer.len() != screen2.buffer.len() {
        return false;
    }

    for (row1, row2) in screen1.buffer.iter().zip(screen2.buffer.iter()) {
        if row1.len() != row2.len() {
            return false;
        }
        for (cell1, cell2) in row1.iter().zip(row2.iter()) {
            if cell1.ch != cell2.ch || cell1.pen != cell2.pen || cell1.paper != cell2.paper {
                return false;
            }
        }
    }

    true
}

/// Displays two screens side-by-side with a difference map in the middle
pub fn display_screen_diff(
    screen1: &Screen,
    palette1: &Palette,
    screen2: &Screen,
    palette2: &Palette
) -> String {
    use cpclib_image::ga::Ink;
    use owo_colors::{DynColors, OwoColorize};

    let border = 2;
    let height = screen1.buffer.len();
    let width = screen1.buffer[0].len();

    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "\n{:^width$}   {:^width$}   {:^width$}\n",
        "ORIGINAL",
        "DIFF",
        "RECONSTRUCTED",
        width = width + 2 * border
    ));
    output.push_str(&format!("{}\n", "=".repeat((width + 2 * border) * 3 + 6)));

    // Border thickness
    let border_ink1 = palette1.get_border();
    let border_rgb1 = border_ink1.color();
    let border_color1 = DynColors::Rgb(border_rgb1[0], border_rgb1[1], border_rgb1[2]);

    let border_ink2 = palette2.get_border();
    let border_rgb2 = border_ink2.color();
    let border_color2 = DynColors::Rgb(border_rgb2[0], border_rgb2[1], border_rgb2[2]);

    // Top borders
    for _ in 0..border {
        for _ in 0..(width + 2 * border) {
            output.push_str(&format!("{}", " ".on_color(border_color1)));
        }
        output.push_str("   ");
        for _ in 0..(width + 2 * border) {
            output.push(' ');
        }
        output.push_str("   ");
        for _ in 0..(width + 2 * border) {
            output.push_str(&format!("{}", " ".on_color(border_color2)));
        }
        output.push('\n');
    }

    // Screen rows
    for (row_idx, (row1, row2)) in screen1.buffer.iter().zip(screen2.buffer.iter()).enumerate() {
        // Left border for screen1
        for _ in 0..border {
            output.push_str(&format!("{}", " ".on_color(border_color1)));
        }

        // Screen1 content
        for cell in row1.iter() {
            let pen_ink = palette1.get(&cell.pen);
            let paper_ink = palette1.get(&cell.paper);
            let pen_rgb = pen_ink.color();
            let paper_rgb = paper_ink.color();
            let fg = DynColors::Rgb(pen_rgb[0], pen_rgb[1], pen_rgb[2]);
            let bg = DynColors::Rgb(paper_rgb[0], paper_rgb[1], paper_rgb[2]);
            output.push_str(&format!("{}", (cell.ch as char).color(fg).on_color(bg)));
        }

        // Right border for screen1
        for _ in 0..border {
            output.push_str(&format!("{}", " ".on_color(border_color1)));
        }

        output.push_str("   ");

        // Diff map (no border)
        for _ in 0..border {
            output.push(' ');
        }
        for (cell1, cell2) in row1.iter().zip(row2.iter()) {
            let is_different =
                cell1.ch != cell2.ch || cell1.pen != cell2.pen || cell1.paper != cell2.paper;
            if is_different {
                // Red background for differences
                output.push_str(&format!("{}", "█".red()));
            }
            else {
                // Black/dark for matches
                output.push_str(&format!("{}", "░".black()));
            }
        }
        for _ in 0..border {
            output.push(' ');
        }

        output.push_str("   ");

        // Left border for screen2
        for _ in 0..border {
            output.push_str(&format!("{}", " ".on_color(border_color2)));
        }

        // Screen2 content
        for cell in row2.iter() {
            let pen_ink = palette2.get(&cell.pen);
            let paper_ink = palette2.get(&cell.paper);
            let pen_rgb = pen_ink.color();
            let paper_rgb = paper_ink.color();
            let fg = DynColors::Rgb(pen_rgb[0], pen_rgb[1], pen_rgb[2]);
            let bg = DynColors::Rgb(paper_rgb[0], paper_rgb[1], paper_rgb[2]);
            output.push_str(&format!("{}", (cell.ch as char).color(fg).on_color(bg)));
        }

        // Right border for screen2
        for _ in 0..border {
            output.push_str(&format!("{}", " ".on_color(border_color2)));
        }

        output.push('\n');
    }

    // Bottom borders
    for _ in 0..border {
        for _ in 0..(width + 2 * border) {
            output.push_str(&format!("{}", " ".on_color(border_color1)));
        }
        output.push_str("   ");
        for _ in 0..(width + 2 * border) {
            output.push(' ');
        }
        output.push_str("   ");
        for _ in 0..(width + 2 * border) {
            output.push_str(&format!("{}", " ".on_color(border_color2)));
        }
        output.push('\n');
    }

    output
}

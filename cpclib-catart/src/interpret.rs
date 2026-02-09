//! The aim of this file is to ease debuggin of the conversion from Basic commands to Char commands
//! by simulating an Amstrad CPC screen and interpreting the char commands to produce a visual output.
//! This is mainly useful for tests.
//! #[derive(Clone, Debug)]
pub struct Cursor {
    pub x: u16,
    pub y: u16
}

impl Cursor {
    pub fn new() -> Self {
        Self { x: 1, y: 1 }
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

use crate::basic_command::{BasicCommand, BasicCommandList, PrintArgument};
use crate::char_command::{CharCommand, CharCommandList};

/// The CPC font bitmap data (8 bytes per character, 8x8 pixel matrix)
const FONTE_BIN: &[u8] = include_bytes!("FONTE.BIN");

// CPC screen memory layout constants
const CPC_SCREEN_MEMORY_SIZE: usize = 0x4000;  // 16KB video memory
const CPC_LINE_INTERLEAVE: usize = 0x800;      // Distance between pixel lines within a char row
const CPC_BYTES_PER_CHARACTER_LINE: usize = 80; // Bytes per character row
const CPC_SCREEN_HEIGHT_PIXELS: usize = 200;   // Screen height in pixels
const CPC_CHARACTER_HEIGHT: usize = 8;         // Character height in pixels

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
            Mode::Mode2 => cpclib_image::image::Mode::Two,
        }
    }
}

use cpclib_image::ga::{Palette, Pen};
use cpclib_image::pixels;

/// Pixel-accurate memory representation of the CPC screen
#[derive(Clone, Debug)]
pub struct BasicMemoryScreen {
    mode: cpclib_image::image::Mode,
    palette: Palette,
    memory: [u8; 0x4000]
}

impl BasicMemoryScreen {
    pub fn new(mode: cpclib_image::image::Mode, palette: Palette) -> Self {
        Self {
            mode,
            palette,
            memory: [0; CPC_SCREEN_MEMORY_SIZE]
        }
    }
    
    /// Get bytes per character for the current mode
    fn bytes_per_char(&self) -> usize {
        match self.mode {
            cpclib_image::image::Mode::Zero => 2,  // Mode 0: 2 bytes per char (2 bits/pixel, 4 pixels/byte)
            cpclib_image::image::Mode::One => 2,   // Mode 1: 2 bytes per char (2 bits/pixel, 4 pixels/byte)
            cpclib_image::image::Mode::Two => 1,   // Mode 2: 1 byte per char (1 bit/pixel, 8 pixels/byte)
            _ => 2,
        }
    }
    
    /// Get screen width in pixels for the current mode
    fn screen_width_pixels(&self) -> usize {
        match self.mode {
            cpclib_image::image::Mode::Zero => 160,
            cpclib_image::image::Mode::One => 320,
            cpclib_image::image::Mode::Two => 640,
            _ => 320,
        }
    }
    
    pub fn reset_mode(&mut self, mode: cpclib_image::image::Mode, pen: Pen, paper: Pen) {
        self.mode = mode;
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
        let bytes = pixels::pens_to_vec(&pens, mode);
        bytes[0] // Return first byte (pattern repeats)
    }
    
    /// Get the 8x8 bitmap for a character from FONTE.BIN
    fn get_char_bitmap(ch: u8) -> &'static [u8; 8] {
        // Font data is directly from CPC ROM (no header)
        let offset = (ch as usize) * 8;
        if offset + 8 <= FONTE_BIN.len() {
            unsafe {
                &*(FONTE_BIN.as_ptr().add(offset) as *const [u8; 8])
            }
        } else {
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
    
    /// Write a character at the given position (x, y in character coordinates)
    pub fn write_char(&mut self, ch: u8, x: u16, y: u16, pen: Pen, paper: Pen) {
        // Get the character bitmap
        let bitmap = Self::get_char_bitmap(ch);
        
        // For each of the 8 lines in the character
        for line_idx in 0..8 {
            // Convert bitmap line to pens
            let pens = Self::bitmap_to_pens(bitmap[line_idx], pen, paper);
            
            // Convert pens to bytes (depends on mode)
            let bytes = pixels::pens_to_vec(&pens, self.mode);
            
            // Calculate memory offset for this line
            // CPC memory layout: character rows are 80 bytes apart, pixel lines within char are 0x800 bytes apart
            let base_addr = ((y - 1) * CPC_BYTES_PER_CHARACTER_LINE as u16) as usize;
            let addr = base_addr + (line_idx as usize * CPC_LINE_INTERLEAVE);
            
            // Calculate byte position within the line based on x position and mode
            let char_x = (x - 1) as usize;
            let byte_offset = char_x * self.bytes_per_char();
            let final_addr = addr + byte_offset;
            
            // Write the bytes to memory
            if final_addr + bytes.len() <= CPC_SCREEN_MEMORY_SIZE {
                for (i, &byte) in bytes.iter().enumerate() {
                    self.memory[final_addr + i] = byte;
                }
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
            let line_offset = char_row * CPC_BYTES_PER_CHARACTER_LINE + pixel_line_in_char * CPC_LINE_INTERLEAVE;
            
            // All modes use 80 bytes per line (full character row width)
            let bytes_per_line = CPC_BYTES_PER_CHARACTER_LINE;
            
            // Read bytes for this line and convert to pens
            if line_offset + bytes_per_line <= CPC_SCREEN_MEMORY_SIZE {
                let line_bytes = &self.memory[line_offset..line_offset + bytes_per_line];
                let pixels_iter = pixels::bytes_to_pens(line_bytes, self.mode);
                line_pens.extend(pixels_iter.take(width_pixels));
            }
            
            pens.push(line_pens);
        }
        
        pens
    }
    
    /// Convert the memory screen to a Sprite
    pub fn to_sprite(&self) -> cpclib_image::image::Sprite {
        let pens = self.to_pens();
        cpclib_image::image::Sprite::from_pens(&pens, self.mode, Some(self.palette.clone()))
    }
    
    /// Convert the memory screen to a ColorMatrix (with border)
    pub fn to_color_matrix_with_border(&self, border_size: usize) -> Option<cpclib_image::image::ColorMatrix> {
        // First convert to sprite
        let sprite = self.to_sprite();
        
        // Then convert sprite to color matrix
        let mut color_matrix = sprite.to_color_matrix()?;
        
        // Add border around the image
        let border_ink = self.palette.get_border();
        
        // Step 1: Add left border columns (insert at position 0, border_size times)
        let left_column = vec![*border_ink; color_matrix.height() as usize];
        for _ in 0..border_size {
            color_matrix.add_column(0, &left_column);
        }
        
        // Step 2: Add right border columns (append at the end)
        let right_column = vec![*border_ink; color_matrix.height() as usize];
        for _ in 0..border_size {
            color_matrix.add_column(color_matrix.width() as usize, &right_column);
        }
        
        // Step 3: Add top border lines (insert at position 0, border_size times)
        // Now the width has increased, so create border_line with new width
        let top_line = vec![*border_ink; color_matrix.width() as usize];
        for _ in 0..border_size {
            color_matrix.add_line(0, &top_line);
        }
        
        // Step 4: Add bottom border lines (append at the end)
        // The height has increased from adding top lines, but add_line handles that
        let bottom_line = vec![*border_ink; color_matrix.width() as usize];
        for _ in 0..border_size {
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
    
    /// Set a palette entry
    pub fn set_palette(&mut self, pen: Pen, ink: cpclib_image::ga::Ink) {
        self.palette.set(pen, ink);
    }
    
    /// Set the border ink
    pub fn set_border(&mut self, ink: cpclib_image::ga::Ink) {
        self.palette.set_border(ink);
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

pub struct Interpreter {
    enable_vdu: bool,
    screen: Screen,
    memory_screen: BasicMemoryScreen,  // Pixel-accurate representation
    cursor: Cursor,
    palette: Palette,
    pen: Pen,
    paper: Pen,
    border: Pen,
    window: Option<(u16, u16, u16, u16)> // (left, right, top, bottom)
}

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

    pub fn new(mode: Mode) -> Self {
        let pen = Pen::Pen1;
        let paper = Pen::Pen0;
        let palette = Palette::default();
        let image_mode = mode.to_image_mode();
        Self {
            screen: Screen {
                mode: mode.clone(),
                buffer: Screen::make_buffer(&mode, pen, paper)
            },
            memory_screen: BasicMemoryScreen::new(image_mode, palette.clone()),
            cursor: Cursor::new(),
            palette,
            pen,
            paper,
            border: paper,
            window: None,
            enable_vdu: true
        }
    }

    pub fn new_6128() -> Self {
        let mut inter = Self::new(Mode::Mode1);
        inter.initialize_6128();
        inter
    }

    fn initialize_6128(&mut self) {
        let start_text = BasicCommandList::from(vec![
            BasicCommand::print_string_crlf(b" Amstrad 128K Microcomputer (v3)"),
            BasicCommand::print_string_crlf(b""),
            BasicCommand::print_string(b" "),
            BasicCommand::print_string(PrintArgument::ChrDollar(0xA4)),
            BasicCommand::print_string_crlf(b"1985 Amstrad Consumer Electronics plc"),
            BasicCommand::print_string_crlf(b"           and Locomotive Software Ltd."),
            BasicCommand::print_string_crlf(b""),
            BasicCommand::print_string_crlf(b" BASIC 1.1"),
            BasicCommand::print_string_crlf(b""),
            BasicCommand::print_string_crlf(b"Ready"),
        ]);

        let commands = start_text
            .to_char_commands()
            .expect("Char command conversion failed");
        self.interpret(commands.as_slice(), true)
            .expect("Initialization failed");
    }

    pub fn inc_cursor_x(&mut self) {
        let (width, _) = self.screen.resolution();
        let (left, right, top, bottom) =
            self.window
                .unwrap_or((1, width, 1, self.screen.resolution().1));
        self.cursor.x += 1;
        if self.cursor.x > right {
            self.cursor.x = left;
            self.cursor.y += 1;
            if self.cursor.y > bottom {
                self.cursor.y = bottom;
            }
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
        self.cursor.x = x.clamp(left, right);
        self.cursor.y = y.clamp(top, bottom);
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

    pub fn scroll_screen_up(&mut self) {
        let width = self.screen.resolution().0;
        let (left, right, top, bottom) =
            self.window
                .unwrap_or((1, width, 1, self.screen.resolution().1));
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
        let finalize = if print_ready {
            BasicCommandList::from(vec![
                BasicCommand::print_string_crlf(b""),
                BasicCommand::print_string_crlf(b""),
                BasicCommand::print_string_crlf(b"Ready")])
        }
        else {
            BasicCommandList::default()
        };
        let finalize = finalize
            .to_char_commands()
            .expect("Char command conversion failed");

        let commands = commands.into_iter().cloned().chain(finalize.into_iter());
        for command in commands {
            self.interpret_command(&command)?;
            // {
            // println!("{}", self);
            // let mut input = String::new();
            // match std::io::stdin().read_line(&mut input) {
            // Ok(n) => {
            // println!("{n} bytes read");
            // println!("{input}");
            // }
            // Err(error) => println!("error: {error}"),
            // }}
        }
        Ok(())
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
                    let ch_to_print = if matches!(command, CharCommand::Char(_)) {
                        *ch as u8
                    }
                    else {
                        '?' as u8
                    };
                    
                    // Update text screen
                    if let Some(cell) = self.screen.cell_mut(self.cursor.x, self.cursor.y) {
                        cell.ch = ch_to_print;
                        cell.pen = self.pen;
                        cell.paper = self.paper;
                    }
                    
                    // Update pixel-accurate memory screen
                    self.memory_screen.write_char(
                        ch_to_print,
                        self.cursor.x,
                        self.cursor.y,
                        self.pen,
                        self.paper
                    );
                    
                    self.inc_cursor_x();
                }
            },
            CharCommand::Locate(x, y) if self.enable_vdu => {
                self.locate_cursor(*x as _, *y as _);
            },
            CharCommand::Cls if self.enable_vdu => {
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));
                
                // Clear text screen
                for y in top..=bottom {
                    for x in left..=right {
                        if let Some(cell) = self.screen.cell_mut(x, y) {
                            cell.ch = b' ';
                            cell.pen = self.pen;
                            cell.paper = self.paper;
                        }
                        // Clear pixel-accurate memory screen
                        self.memory_screen.write_char(b' ', x, y, self.pen, self.paper);
                    }
                }
                self.locate_cursor(left, top);
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
                self.memory_screen.reset_mode(m.to_image_mode(), self.pen, self.paper);
                self.window = None;
                self.cursor.x = 1;
                self.cursor.y = 1;
            },
            CharCommand::Pen(p) if self.enable_vdu => {
                self.pen = Pen::from(*p);
            },
            CharCommand::Paper(p) if self.enable_vdu => {
                self.paper = Pen::from(*p);
            },
            CharCommand::Ink(pen, ink1, _ink2) if self.enable_vdu => {
                let pen = Pen::from(*pen);
                let ink = cpclib_image::ga::Ink::from(*ink1);
                self.palette.set(pen, ink); // blinking is ignored
                self.memory_screen.set_palette(pen, ink);
            },
            CharCommand::Border(ink1, _ink2) if self.enable_vdu => {
                let ink = cpclib_image::ga::Ink::from(*ink1);
                self.palette.set_border(ink);
                self.memory_screen.set_border(ink);
                self.border = Pen::Border;
            },
            CharCommand::ClearLineStart if self.enable_vdu => {
                let y = self.cursor.y;
                let x = self.cursor.x;
                let (left, ..) = self.window.unwrap_or((
                    1,
                    self.screen.resolution().0,
                    1,
                    self.screen.resolution().1
                ));
                for col in left..=x {
                    if let Some(cell) = self.screen.cell_mut(col, y) {
                        cell.ch = b' ';
                        cell.pen = self.pen;
                        cell.paper = self.paper;
                    }
                    self.memory_screen.write_char(b' ', col, y, self.pen, self.paper);
                }
            },
            CharCommand::ClearLineEnd if self.enable_vdu => {
                let y = self.cursor.y;
                let x = self.cursor.x;
                let (_, right, ..) = self.window.unwrap_or((
                    1,
                    self.screen.resolution().0,
                    1,
                    self.screen.resolution().1
                ));
                for col in x..=right {
                    if let Some(cell) = self.screen.cell_mut(col, y) {
                        cell.ch = b' ';
                        cell.pen = self.pen;
                        cell.paper = self.paper;
                    }
                    self.memory_screen.write_char(b' ', col, y, self.pen, self.paper);
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
                        if let Some(cell) = self.screen.cell_mut(x, y) {
                            cell.ch = b' ';
                            cell.pen = self.pen;
                            cell.paper = self.paper;
                        }
                        self.memory_screen.write_char(b' ', x, y, self.pen, self.paper);
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
                    if let Some(cell) = self.screen.cell_mut(x, cur_y) {
                        cell.ch = b' ';
                        cell.pen = self.pen;
                        cell.paper = self.paper;
                    }
                    self.memory_screen.write_char(b' ', x, cur_y, self.pen, self.paper);
                }
                // Fill all lines below
                for y in (cur_y + 1)..=bottom {
                    for x in left..=right {
                        if let Some(cell) = self.screen.cell_mut(x, y) {
                            cell.ch = b' ';
                            cell.pen = self.pen;
                            cell.paper = self.paper;
                        }
                        self.memory_screen.write_char(b' ', x, y, self.pen, self.paper);
                    }
                }
            },
            CharCommand::Window(left, right, top, bottom) if self.enable_vdu => {
                self.window = Some((*left as u16, *right as u16, *top as u16, *bottom as u16));
                self.locate_cursor(*left as u16, *top as u16);
            },
            CharCommand::Transparency(_p) if self.enable_vdu => {
                // Transparency is not simulated; ignore.
                // a way to implement that would be to stack characters and redraw all of them
            },
            CharCommand::ExchangePenAndPaper if self.enable_vdu => {
                std::mem::swap(&mut self.pen, &mut self.paper);
            },

            c if !self.enable_vdu => {
                // When VDU is disabled, ignore all commands except those handled above
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
pub fn display_screen_diff(screen1: &Screen, palette1: &Palette, screen2: &Screen, palette2: &Palette) -> String {
    use cpclib_image::ga::Ink;
    use owo_colors::{DynColors, OwoColorize};
    
    let border = 2;
    let height = screen1.buffer.len();
    let width = screen1.buffer[0].len();
    
    let mut output = String::new();
    
    // Header
    output.push_str(&format!("\n{:^width$}   {:^width$}   {:^width$}\n", 
        "ORIGINAL", "DIFF", "RECONSTRUCTED", width = width + 2 * border));
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
            let is_different = cell1.ch != cell2.ch || cell1.pen != cell2.pen || cell1.paper != cell2.paper;
            if is_different {
                // Red background for differences
                output.push_str(&format!("{}", "█".red()));
            } else {
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

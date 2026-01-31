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
}

use cpclib_image::ga::{Palette, Pen};

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

    pub fn new(mode: Mode) -> Self {
        let pen = Pen::Pen1;
        let paper = Pen::Pen0;
        let palette = Palette::default();
        Self {
            screen: Screen {
                mode: mode.clone(),
                buffer: Screen::make_buffer(&mode, pen, paper)
            },
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
            BasicCommandList::from(vec![BasicCommand::print_string_crlf(b"Ready")])
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
                    if let Some(cell) = self.screen.cell_mut(self.cursor.x, self.cursor.y) {
                        cell.ch = if matches!(command, CharCommand::Char(_)) {
                            *ch as u8
                        }
                        else {
                            '?' as u8
                        };
                        cell.pen = self.pen;
                        cell.paper = self.paper;
                    }
                    self.inc_cursor_x();
                }
            },
            CharCommand::Locate(x, y) if self.enable_vdu => {
                self.locate_cursor(*x as _, *y as _);
            },
            CharCommand::Cls if self.enable_vdu => {
                let (width, height) = self.screen.resolution();
                let (left, right, top, bottom) = self.window.unwrap_or((1, width, 1, height));
                for y in top..=bottom {
                    for x in left..=right {
                        if let Some(cell) = self.screen.cell_mut(x, y) {
                            cell.ch = b' ';
                            cell.pen = self.pen;
                            cell.paper = self.paper;
                        }
                    }
                }
                self.locate_cursor(left, top);
            },
            CharCommand::CarriageReturn if self.enable_vdu => {
                self.locate_cursor(self.window.map_or(1, |(l, ..)| l), self.cursor.y);
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
                self.screen.reset_mode(m, self.pen, self.paper);
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
            },
            CharCommand::Border(ink1, _ink2) if self.enable_vdu => {
                let ink = cpclib_image::ga::Ink::from(*ink1);
                self.palette.set_border(ink);
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
                }
                // Fill all lines below
                for y in (cur_y + 1)..=bottom {
                    for x in left..=right {
                        if let Some(cell) = self.screen.cell_mut(x, y) {
                            cell.ch = b' ';
                            cell.pen = self.pen;
                            cell.paper = self.paper;
                        }
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

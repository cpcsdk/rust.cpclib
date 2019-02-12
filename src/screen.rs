use image as im;

pub const MAX_WIDTH_MODE0: u32 = 96 * 2;
pub const MAX_WIDTH_MODE1: u32 = 96 * 4;
pub const MAX_WIDTH_MODE2: u32 = 96 * 8;
pub const MAX_WIDTH_MODE3: u32 = 96 * 2;
pub const MAX_HEIGHT: u32 = 39 * 8;

pub struct SimpleMonitor {
    pub width: u32,
    pub height: u32,
    pub mode: u8,
    pub pixels: Vec<Vec<u8>>,
    pub palette: Vec<char>,
    pub buffer: im::ImageBuffer<im::Rgba<u8>, Vec<u8>>,
}

impl SimpleMonitor {
    pub fn new(width: u32, height: u32, mode: u8) -> SimpleMonitor {
        SimpleMonitor {
            width,
            height,
            mode,
            pixels: vec![vec![0u8; width as usize]; height as usize],
            palette: vec![0 as char; 16],
            buffer: im::ImageBuffer::new(width, height),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, pen: u8) {
        self.pixels[y as usize][x as usize] = pen;
    }

    pub fn set_ink(&mut self, ink: char, color: char) {
        assert!(ink <= 17 as char);
        assert!(color <= 27 as char);

        self.palette[ink as usize] = color
    }

    pub fn update_buffer(&mut self) {
        // Here we have to draw our pixel array in the buffer
    }

    pub fn canvas(&mut self) -> &mut im::ImageBuffer<im::Rgba<u8>, Vec<u8>> {
        &mut self.buffer
    }
}

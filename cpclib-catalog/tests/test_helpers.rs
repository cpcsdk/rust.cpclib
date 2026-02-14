use std::fs;
use std::path::PathBuf;
use cpclib_catart::interpret::{BasicMemoryScreen, Interpreter, Locale};
use cpclib_image::ga::{Ink, Palette, Pen};
use cpclib_image::image::Mode;
use image::{ImageBuffer, Rgb, RgbImage};

// Border sizes in characters (not hardcoded elsewhere, configurable)
const BORDER_TOP_CHARS: usize = 5;
const BORDER_LEFT_CHARS: usize = 4;
const BORDER_RIGHT_CHARS: usize = 4;
const BORDER_BOTTOM_CHARS: usize = 5;
const CHAR_HEIGHT_PIXELS: usize = 8;

/// Convert memory address to character coordinates (1-indexed, mode-dependent)
/// 
/// CPC memory layout: Each character is 8 pixel lines tall.
/// Pixel lines are interleaved: line 0 at 0x0000-0x04FF, line 1 at 0x0800-0x0CFF, etc.
/// Within each pixel line, character rows are 80 bytes apart.
pub fn memory_addr_to_char_coords(addr: usize, mode: Mode) -> (u16, u16) {
    const LINE_INTERLEAVE: usize = 0x800;  // Distance between pixel lines
    const BYTES_PER_CHAR_ROW: usize = 80;  // Bytes per character row
    
    // Extract which character row we're in (0-24)
    // The character row is determined by the position within the 0x800 block
    let offset_in_block = addr % LINE_INTERLEAVE;
    let char_row = offset_in_block / BYTES_PER_CHAR_ROW;
    
    // Extract byte position within the character row
    let byte_in_row = offset_in_block % BYTES_PER_CHAR_ROW;
    
    // Convert byte position to character column (mode-dependent)
    // CRITICAL: Mode 0 uses 4 bytes per char, Mode 1 uses 2, Mode 2 uses 1
    let bytes_per_char = match mode {
        Mode::Zero => 4,  // Mode 0: 4 bytes per char
        Mode::One => 2,   // Mode 1: 2 bytes per char
        Mode::Two => 1,   // Mode 2: 1 byte per char
        _ => 2,
    };
    
    let char_col = byte_in_row / bytes_per_char;
    
    // Return 1-indexed coordinates
    ((char_col + 1) as u16, (char_row + 1) as u16)
}

/// Get all memory addresses for a character cell (given 1-indexed coordinates)
/// Each character spans 8 scanlines in interleaved memory layout
fn char_coords_to_memory_addrs(char_x: u16, char_y: u16, mode: Mode) -> Vec<usize> {
    const LINE_INTERLEAVE: usize = 0x800;  // Distance between pixel lines
    const BYTES_PER_CHAR_ROW: usize = 80;  // Bytes per character row
    
    let char_row = (char_y - 1) as usize;  // Convert to 0-indexed
    let char_col = (char_x - 1) as usize;  // Convert to 0-indexed
    
    let bytes_per_char = match mode {
        Mode::Zero => 4,  // Mode 0: 4 bytes per char
        Mode::One => 2,   // Mode 1: 2 bytes per char
        Mode::Two => 1,   // Mode 2: 1 byte per char
        _ => 2,
    };
    
    let mut addresses = Vec::new();
    
    // Each character spans 8 scanlines (pixel lines)
    for scanline in 0..8 {
        // Base address for this scanline's block
        let block_base = scanline * LINE_INTERLEAVE;
        // Offset within block for this character row
        let row_offset = char_row * BYTES_PER_CHAR_ROW;
        // Offset within row for this character column
        let col_offset = char_col * bytes_per_char;
        
        // Add all bytes for this character at this scanline
        for byte in 0..bytes_per_char {
            let addr = block_base + row_offset + col_offset + byte;
            if addr < 16384 {
                addresses.push(addr);
            }
        }
    }
    
    addresses
}

/// Compare two memory screens and generate a visual comparison if different
/// Returns true if screens are identical, false otherwise
pub fn compare_screens_with_visual_diff(
    test_name: &str,
    interpreter1: &Interpreter,
    interpreter2: &Interpreter,
    test_dir: &PathBuf,
) -> bool {
    let memory1 = interpreter1.memory_screen().memory();
    let memory2 = interpreter2.memory_screen().memory();
    
    // Compare memories
    let mut differences = Vec::new();
    for (addr, (&byte1, &byte2)) in memory1.iter().zip(memory2.iter()).enumerate() {
        if byte1 != byte2 {
            differences.push((addr, byte1, byte2));
        }
    }
    
    if differences.is_empty() {
        println!("✓ Test {} - Screens match perfectly!", test_name);
        
        // Delete comparison PNG if it exists (from previous failed run)
        let comparison_path = test_dir.join(format!("{}_comparison.png", test_name));
        if comparison_path.exists() {
            let _ = fs::remove_file(&comparison_path);
        }
        true
    } else {
        println!("✗ Test {} - Found {} screen differences", test_name, differences.len());
        
        // Convert memory addresses to char coordinates
        let screen_mode = interpreter1.memory_screen().mode();
        
        println!("\nDifferences (showing first 20):");
        for (addr, byte1, byte2) in differences.iter().take(20) {
            let coords = memory_addr_to_char_coords(*addr, screen_mode);
            println!("  Char position ({}, {}): screen1=0x{:02X}, screen2=0x{:02X} [addr: 0x{:04X}]", 
                coords.0, coords.1, byte1, byte2, addr);
        }
        
        // Generate side-by-side PNG comparison
        generate_comparison_png_from_interpreters(
            test_name,
            interpreter1,
            interpreter2,
            &differences,
            test_dir,
        );
        
        false
    }
}

/// Generate a three-way PNG comparison from two interpreters
/// Left: interpreter1 screen
/// Middle: difference visualization (red markers on black)
/// Right: interpreter2 screen
fn generate_comparison_png_from_interpreters(
    test_name: &str,
    interpreter1: &Interpreter,
    interpreter2: &Interpreter,
    differences: &[(usize, u8, u8)],
    output_dir: &PathBuf,
) {
    let memory_screen1 = interpreter1.memory_screen();
    let memory_screen2 = interpreter2.memory_screen();
    let mode = memory_screen1.mode();
    
    // Calculate border size in pixels (after any pixel doubling)
    // Character dimensions in pixels for each mode (before doubling)
    let (char_width_pixels, char_height_pixels) = match mode {
        Mode::Zero => (8, 8),   // Mode 0: 16 columns
        Mode::One => (4, 8),    // Mode 1: 20 columns
        Mode::Two => (2, 8),    // Mode 2: 40 columns
        _ => (4, 8),
    };
    
    let pixel_multiplier = match mode {
        Mode::One => 2,  // Mode 1: pixels are doubled
        _ => 1,
    };
    
    let max_h_border_chars = BORDER_LEFT_CHARS.max(BORDER_RIGHT_CHARS);
    let max_v_border_chars = BORDER_TOP_CHARS.max(BORDER_BOTTOM_CHARS);
    
    let border_horizontal = max_h_border_chars * char_width_pixels * pixel_multiplier;
    let border_vertical = max_v_border_chars * char_height_pixels * pixel_multiplier;
    
    // Convert first interpreter screen to PNG
    let color_matrix1 = memory_screen1.to_color_matrix_with_border(border_horizontal, border_vertical)
        .expect("Failed to convert screen1 to color matrix");
    let img1 = color_matrix_to_rgb_image(&color_matrix1);
    
    // Convert second interpreter screen to PNG
    let color_matrix2 = memory_screen2.to_color_matrix_with_border(border_horizontal, border_vertical)
        .expect("Failed to convert screen2 to color matrix");
    let img2 = color_matrix_to_rgb_image(&color_matrix2);
    
    // Create difference visualization screen
    let mut palette_diff = Palette::default();
    palette_diff.set(Pen::Pen0, Ink::BLACK);       // Paper: black
    palette_diff.set(Pen::Pen1, Ink::BRIGHTRED);   // Pen: red
    palette_diff.set_border(Ink::BLACK);
    
    let mut diff_screen = BasicMemoryScreen::new_with_locale(mode, palette_diff.clone(), Locale::English);
    
    // Fill with spaces (clear the screen)
    let (width, height) = match mode {
        Mode::Zero => (20, 25),
        Mode::One => (40, 25),
        Mode::Two => (80, 25),
        _ => (40, 25),
    };
    
    for y in 1..=height {
        for x in 1..=width {
            diff_screen.write_char(b' ', x, y, Pen::Pen1, Pen::Pen0, false);
        }
    }
    
    // Mark differences with 0x8F character (red on black)
    for &(addr, _byte1, _byte2) in differences {
        let coords = memory_addr_to_char_coords(addr, mode);
        if coords.0 >= 1 && coords.0 <= width && coords.1 >= 1 && coords.1 <= height {
            diff_screen.write_char(0x8F, coords.0, coords.1, Pen::Pen1, Pen::Pen0, false);
        }
    }
    
    // Convert difference screen to PNG
    let diff_color_matrix = diff_screen.to_color_matrix_with_border(border_horizontal, border_vertical)
        .expect("Failed to convert diff screen to color matrix");
    let diff_img = color_matrix_to_rgb_image(&diff_color_matrix);
    
    // Create three-way comparison image
    let separator = 20;
    let (img1_width, img1_height) = img1.dimensions();
    let (diff_width, diff_height) = diff_img.dimensions();
    let (img2_width, img2_height) = img2.dimensions();
    
    let target_height = img1_height.max(diff_height).max(img2_height);
    let comparison_width = img1_width + diff_width + img2_width + 2 * separator;
    let mut comparison_img: RgbImage = ImageBuffer::new(comparison_width, target_height);
    
    // Fill with white background
    for pixel in comparison_img.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }
    
    // Copy first image to left side
    for y in 0..img1_height {
        for x in 0..img1_width {
            let pixel = img1.get_pixel(x, y);
            comparison_img.put_pixel(x, y, *pixel);
        }
    }
    
    // Copy difference image to middle
    let diff_offset_x = img1_width + separator;
    for y in 0..diff_height {
        for x in 0..diff_width {
            let pixel = diff_img.get_pixel(x, y);
            comparison_img.put_pixel(x + diff_offset_x, y, *pixel);
        }
    }
    
    // Copy second image to right side
    let img2_offset_x = img1_width + diff_width + 2 * separator;
    for y in 0..img2_height {
        for x in 0..img2_width {
            let pixel = img2.get_pixel(x, y);
            comparison_img.put_pixel(x + img2_offset_x, y, *pixel);
        }
    }
    
    // Save comparison image
    let output_path = output_dir.join(format!("{}_comparison.png", test_name));
    comparison_img.save(&output_path)
        .unwrap_or_else(|e| panic!("Failed to save comparison PNG to {}: {}", output_path.display(), e));
    
    println!("Comparison image saved to: {}", output_path.display());
}

/// Helper to convert a color matrix to RGB image
fn color_matrix_to_rgb_image(color_matrix: &cpclib_image::image::ColorMatrix) -> RgbImage {
    let width = color_matrix.width() as u32;
    let height = color_matrix.height() as u32;
    
    let mut img: RgbImage = ImageBuffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let color = color_matrix.get_ink(x as usize, y as usize);
            let rgb = color.color();
            img.put_pixel(x, y, Rgb([rgb[0], rgb[1], rgb[2]]));
        }
    }
    img
}

/// Compare raw memory buffers and generate visual diff
/// Returns Result with error message containing details and file paths
pub fn compare_memory_with_visual_diff(
    palette: &Palette,
    expected: &[u8; 16384],
    actual: &[u8; 16384],
    output_prefix: &str,
    test_dir: &str,
) -> Result<(), String> {
    // Compare memories
    let mut differences = Vec::new();
    for (addr, (&byte1, &byte2)) in expected.iter().zip(actual.iter()).enumerate() {
        if byte1 != byte2 {
            differences.push((addr, byte1, byte2));
        }
    }
    
    if differences.is_empty() {
        println!("✓ Screen memory matches perfectly!");
        return Ok(());
    }
    
    println!("✗ Found {} screen differences", differences.len());
    
    //Use the actual screen memory (second parameter) to generate the actual image
    let mode = Mode::One;
    
    // Calculate border sizes based on Mode 1 character dimensions
    let char_width_pixels = 4;  // Mode 1
    let char_height_pixels = 8;
    let pixel_multiplier = 2;  // Mode 1 doubles pixels
    
    let max_h_border_chars = BORDER_LEFT_CHARS.max(BORDER_RIGHT_CHARS);
    let max_v_border_chars = BORDER_TOP_CHARS.max(BORDER_BOTTOM_CHARS);
    
    let border_horizontal = max_h_border_chars * char_width_pixels * pixel_multiplier;
    let border_vertical = max_v_border_chars * char_height_pixels * pixel_multiplier;
    
    // Create image from actual memory by building a basic screen and rendering it
    let img_actual = memory_to_image(actual, &mode, &palette, border_horizontal, border_vertical);
    let img_expected = memory_to_image(expected, &mode, &palette, border_horizontal, border_vertical);
    
    // Create difference visualization - start with blank memory and mark ONLY differences
    let mut diff_memory = [0u8; 16384];  // Blank screen
    
    // Collect unique character cells that have differences
    use std::collections::HashSet;
    let mut diff_chars: HashSet<(u16, u16)> = HashSet::new();
    for &(addr, _byte1, _byte2) in &differences {
        let coords = memory_addr_to_char_coords(addr, mode);
        diff_chars.insert(coords);
    }
    
    // Mark ALL bytes for each affected character cell (complete 8×8 blocks)
    // In Mode 1, 0xFF creates bright white pixels to highlight differences
    for &(char_x, char_y) in &diff_chars {
        let addresses = char_coords_to_memory_addrs(char_x, char_y, mode);
        for addr in addresses {
            if addr < diff_memory.len() {
                diff_memory[addr] = 0xFF;  // Bright pixels for entire character block
            }
        }
    }
    
    // Convert diff memory to image using the same method as expected/actual
    let diff_img = memory_to_image(&diff_memory, &mode, &palette, border_horizontal, border_vertical);
    
    // Create three-way comparison image
    let separator = 20;
    let (img1_width, img1_height) = img_expected.dimensions();
    let (diff_width, diff_height) = diff_img.dimensions();
    let (img2_width, img2_height) = img_actual.dimensions();
    
    let target_height = img1_height.max(diff_height).max(img2_height);
    let comparison_width = img1_width + diff_width + img2_width + 2 * separator;
    let mut comparison_img: RgbImage = ImageBuffer::new(comparison_width, target_height);
    
    // Fill with white background
    for pixel in comparison_img.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }
    
    // Copy images side by side (expected | diff | actual)
    for y in 0..img1_height {
        for x in 0..img1_width {
            comparison_img.put_pixel(x, y, *img_expected.get_pixel(x, y));
        }
    }
    
    let diff_offset_x = img1_width + separator;
    for y in 0..diff_height {
        for x in 0..diff_width {
            comparison_img.put_pixel(x + diff_offset_x, y, *diff_img.get_pixel(x, y));
        }
    }
    
    let img2_offset_x = img1_width + diff_width + 2 * separator;
    for y in 0..img2_height {
        for x in 0..img2_width {
            comparison_img.put_pixel(x + img2_offset_x, y, *img_actual.get_pixel(x, y));
        }
    }
    
    // Save comparison image
    let output_path = PathBuf::from(test_dir).join(format!("{}_comparison.png", output_prefix));
    comparison_img.save(&output_path)
        .unwrap_or_else(|e| panic!("Failed to save comparison PNG: {}", e));
    
    // Build detailed error message
    let mut msg = format!("❌ Screen memory mismatch: {} differences found\n", differences.len());
    msg.push_str(&format!("   Comparison image saved to: {}\n\n", output_path.display()));
    
    msg.push_str("First 20 differences:\n");
    for (addr, expected_byte, actual_byte) in differences.iter().take(20) {
        let coords = memory_addr_to_char_coords(*addr, Mode::One);
        msg.push_str(&format!(
            "  [0x{:04X}] (char {},{}) Expected: 0x{:02X}, Actual: 0x{:02X}\n",
            addr, coords.0, coords.1, expected_byte, actual_byte
        ));
    }
    
    Err(msg)
}

/// Helper to convert raw CPC memory to an RGB image with border
fn memory_to_image(memory: &[u8; 16384], mode: &Mode, palette: &Palette, border_horizontal: usize, border_vertical: usize) -> RgbImage {
    // Create a color matrix from the raw memory using cpclib_image
    // For Mode1: 40 characters wide, 2 bytes per character = 80 bytes per line
    let bytes_width = match mode {
        Mode::Zero => 80,  // 20 chars * 4 bytes
        Mode::One => 80,   // 40 chars * 2 bytes
        Mode::Two => 80,   // 80 chars * 1 byte
        _ => 80,
    };
    
    use cpclib_image::image::ColorMatrix;
    let mut color_matrix = ColorMatrix::from_screen(memory, bytes_width, *mode, palette);
    
    // Add borders to match typical CPC screen appearance
    if border_horizontal > 0 || border_vertical > 0 {
        let border_ink = *palette.get(&Pen::Pen0);  // Border uses pen 0 color
        let width = color_matrix.width() as usize;
        
        // Add top border lines
        for _ in 0..border_vertical {
            let border_line = vec![border_ink; width];
            color_matrix.add_line(0, &border_line);
        }
        
        // Add bottom border lines
        for _ in 0..border_vertical {
            let border_line = vec![border_ink; width];
            let current_height = color_matrix.height() as usize;
            color_matrix.add_line(current_height, &border_line);
        }
        
        // Add left and right border columns
        let final_height = color_matrix.height() as usize;
        for _ in 0..border_horizontal {
            // Add left border column
            let border_column = vec![border_ink; final_height];
            color_matrix.add_column(0, &border_column);
            
            // Add right border column (width increases after each left column)
            let current_width = color_matrix.width() as usize;
            let border_column = vec![border_ink; final_height];
            color_matrix.add_column(current_width, &border_column);
        }
    }
    
    color_matrix_to_rgb_image(&color_matrix)
}

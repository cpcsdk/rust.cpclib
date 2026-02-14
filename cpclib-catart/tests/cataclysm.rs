use std::fs;
use std::path::PathBuf;
use cpclib_basic::BasicProgram;
use cpclib_catart::basic_command::BasicCommandList;
use cpclib_catart::interpret::{Interpreter, Mode};

// Border sizes in characters (not hardcoded elsewhere, configurable)
const BORDER_TOP_CHARS: usize = 5;
const BORDER_LEFT_CHARS: usize = 4;
const BORDER_RIGHT_CHARS: usize = 4;
const BORDER_BOTTOM_CHARS: usize = 5;
const CHAR_HEIGHT_PIXELS: usize = 8;

/// Converts a screen mode index to Mode enum
fn mode_from_index(mode: u8) -> Mode {
    match mode {
        0 => Mode::Mode0,
        1 => Mode::Mode1,
        2 => Mode::Mode2,
        _ => panic!("Invalid mode: {}", mode),
    }
}

/// Test function that validates a triplet of files (TXT, SCR, PNG)
/// 
/// # Arguments
/// * `test_name` - The XXX prefix for the triplet files
/// * `mode` - The screen mode to use (0, 1, or 2)
/// 
/// # Process
/// 1. Read XXX.TXT and parse as BasicListing
/// 2. Convert BasicListing to CharCommandList
/// 3. Interpret with English locale and 6128 header
/// 4. Compare memory with XXX.SCR
/// 5. If different, generate side-by-side PNG and report differences
fn test_cataclysm_triplet(test_name: &str, mode: u8) {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cataclysm");
    
    let txt_path = test_dir.join(format!("{}.TXT", test_name));
    let scr_path = test_dir.join(format!("{}.SCR", test_name));
    let png_path = test_dir.join(format!("{}.PNG", test_name));
    
    // Step 1: Read and parse the BASIC listing
    let basic_text = fs::read_to_string(&txt_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", txt_path.display(), e));
    
    let basic_program = BasicProgram::parse(&basic_text)
        .unwrap_or_else(|e| panic!("Failed to parse BASIC from {}: {}", txt_path.display(), e));
    
    // Step 2: Convert to BasicCommandList
    let basic_commands: BasicCommandList = (&basic_program).try_into()
        .unwrap_or_else(|e| panic!("Failed to convert BASIC to commands: {:?}", e));
    
    // Step 3: Convert to CharCommandList
    let char_commands = basic_commands.to_char_commands()
        .unwrap_or_else(|e| panic!("Failed to convert commands to char commands: {}", e));
    
    // Step 4: Create interpreter with 6128 header and English locale
    let mut interpreter = Interpreter::new_6128();
    
    // Print the interpreter output to terminal
    println!("\n=== Test: {} ===", test_name);
    println!("{}", interpreter);
    
    // Step 5: Interpret the char commands (print_ready=true to show Ready+cursor)
    interpreter.interpret(char_commands.as_slice(), true)
        .unwrap_or_else(|e| panic!("Failed to interpret commands: {}", e));
    
    println!("{}", interpreter);
    
    // Step 6: Read the SCR file (expected memory)
    let expected_memory = fs::read(&scr_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", scr_path.display(), e));
    
    // The SCR file should be exactly 16KB
    assert_eq!(expected_memory.len(), 0x4000, 
        "SCR file {} should be exactly 16KB (0x4000 bytes), got {} bytes", 
        scr_path.display(), expected_memory.len());
    
    // Step 7: Get the actual memory from interpreter
    let actual_memory = interpreter.memory_screen().memory();
    
    // Step 8: Compare memories
    let mut differences = Vec::new();
    for (addr, (&expected, &actual)) in expected_memory.iter().zip(actual_memory.iter()).enumerate() {
        if expected != actual {
            differences.push((addr, expected, actual));
        }
    }
    
    if differences.is_empty() {
        println!("✓ Test {} PASSED - Memory matches perfectly!", test_name);
        
        // Delete comparison PNG if it exists (from previous failed run)
        let comparison_path = test_dir.join(format!("{}_comparison.png", test_name));
        if comparison_path.exists() {
            let _ = fs::remove_file(&comparison_path);
        }
    } else {
        println!("✗ Test {} FAILED - Found {} differences", test_name, differences.len());
        
        // Convert memory addresses to char coordinates
        let screen_mode = interpreter.memory_screen().mode();
        
        println!("\nDifferences (showing first 20):");
        for (addr, expected, actual) in differences.iter().take(20) {
            let coords = memory_addr_to_char_coords(*addr, screen_mode);
            println!("  Char position ({}, {}): expected 0x{:02X}, got 0x{:02X} [addr: 0x{:04X}]", 
                coords.0, coords.1, expected, actual, addr);
        }
        
        // Generate side-by-side PNG comparison
        generate_comparison_png(
            test_name,
            &png_path,
            interpreter.memory_screen(),
            &expected_memory,
            &differences,
            &test_dir,
        );
        
        panic!("Test failed with {} memory differences", differences.len());
    }
}

/// Convert memory address to character coordinates (1-indexed, mode-dependent)
/// 
/// CPC memory layout: Each character is 8 pixel lines tall.
/// Pixel lines are interleaved: line 0 at 0x0000-0x04FF, line 1 at 0x0800-0x0CFF, etc.
/// Within each pixel line, character rows are 80 bytes apart.
fn memory_addr_to_char_coords(addr: usize, mode: cpclib_image::image::Mode) -> (u16, u16) {
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
        cpclib_image::image::Mode::Zero => 4,  // Mode 0: 4 bytes per char
        cpclib_image::image::Mode::One => 2,   // Mode 1: 2 bytes per char
        cpclib_image::image::Mode::Two => 1,   // Mode 2: 1 byte per char
        _ => 2,
    };
    
    let char_col = byte_in_row / bytes_per_char;
    
    // Return 1-indexed coordinates
    ((char_col + 1) as u16, (char_row + 1) as u16)
}

/// Generate a three-way PNG comparison
/// Left: original PNG (expected)
/// Middle: difference visualization (red markers on black)
/// Right: generated PNG from BasicMemoryScreen (actual)
fn generate_comparison_png(
    test_name: &str,
    original_png_path: &PathBuf,
    memory_screen: &cpclib_catart::interpret::BasicMemoryScreen,
    expected_memory: &[u8],
    differences: &[(usize, u8, u8)],
    output_dir: &PathBuf,
) {
    use image::{ImageBuffer, Rgb, RgbImage};
    use cpclib_catart::interpret::{BasicMemoryScreen, Locale};
    use cpclib_image::ga::{Ink, Palette, Pen};
    
    // Load the original PNG
    let original_img = image::open(original_png_path)
        .unwrap_or_else(|e| panic!("Failed to load original PNG {}: {}", original_png_path.display(), e));
    
    // Calculate border size in pixels (after any pixel doubling)
    // The mode determines character dimensions and if pixels are doubled
    let mode = memory_screen.mode();
    
    // Character dimensions in pixels for each mode (before doubling)
    let (char_width_pixels, char_height_pixels) = match mode {
        cpclib_image::image::Mode::Zero | cpclib_image::image::Mode::Three => (8, 8),   // Mode 0: 16 columns
        cpclib_image::image::Mode::One => (4, 8),    // Mode 1: 20 columns
        cpclib_image::image::Mode::Two => (2, 8),    // Mode 2: 40 columns
    };
    
    // Pixel multiplier for the doubling that happens in to_color_matrix_with_border
    let pixel_multiplier = match mode {
        cpclib_image::image::Mode::One => 2,  // Mode 1: both dimensions are doubled
        _ => 1,
    };
    
    // Use the maximum border for each dimension
    let max_h_border_chars = BORDER_LEFT_CHARS.max(BORDER_RIGHT_CHARS);
    let max_v_border_chars = BORDER_TOP_CHARS.max(BORDER_BOTTOM_CHARS);
    
    let border_horizontal = max_h_border_chars * char_width_pixels * pixel_multiplier;
    let border_vertical = max_v_border_chars * char_height_pixels * pixel_multiplier;
    
    // Convert memory screen to PNG (actual result)
    let color_matrix = memory_screen.to_color_matrix_with_border(border_horizontal, border_vertical)
        .expect("Failed to convert memory screen to color matrix");
    
    // Convert color matrix to image
    let gen_width = color_matrix.width() as u32;
    let gen_height = color_matrix.height() as u32;
    
    let mut generated_img: RgbImage = ImageBuffer::new(gen_width, gen_height);
    for y in 0..gen_height {
        for x in 0..gen_width {
            let color = color_matrix.get_ink(x as usize, y as usize);
            let rgb = color.color();
            generated_img.put_pixel(x, y, Rgb([rgb[0], rgb[1], rgb[2]]));
        }
    }
    
    // Create difference visualization screen
    let mode = memory_screen.mode();
    let mut palette_diff = Palette::default();
    palette_diff.set(Pen::Pen0, Ink::BLACK);       // Paper: black
    palette_diff.set(Pen::Pen1, Ink::BRIGHTRED);   // Pen: red
    palette_diff.set_border(Ink::BLACK);
    
    let mut diff_screen = BasicMemoryScreen::new_with_locale(mode, palette_diff.clone(), Locale::English);
    
    // Fill with spaces (clear the screen)
    let (width, height) = match mode {
        cpclib_image::image::Mode::Zero => (20, 25),  // Mode 0: 20x25 chars
        cpclib_image::image::Mode::One => (40, 25),   // Mode 1: 40x25 chars
        cpclib_image::image::Mode::Two => (80, 25),   // Mode 2: 80x25 chars
        _ => (40, 25),
    };
    
    for y in 1..=height {
        for x in 1..=width {
            diff_screen.write_char(b' ', x, y, Pen::Pen1, Pen::Pen0, false);
        }
    }
    
    // Mark differences with 0x8F character (red on black)
    for &(addr, _expected, _actual) in differences {
        let coords = memory_addr_to_char_coords(addr, mode);
        if coords.0 >= 1 && coords.0 <= width && coords.1 >= 1 && coords.1 <= height {
            diff_screen.write_char(0x8F, coords.0, coords.1, Pen::Pen1, Pen::Pen0, false);
        }
    }
    
    // Convert difference screen to PNG (use same border size as the actual screen)
    let diff_color_matrix = diff_screen.to_color_matrix_with_border(border_horizontal, border_vertical)
        .expect("Failed to convert diff screen to color matrix");
    
    let diff_width = diff_color_matrix.width() as u32;
    let diff_height = diff_color_matrix.height() as u32;
    
    let mut diff_img: RgbImage = ImageBuffer::new(diff_width, diff_height);
    for y in 0..diff_height {
        for x in 0..diff_width {
            let color = diff_color_matrix.get_ink(x as usize, y as usize);
            let rgb = color.color();
            diff_img.put_pixel(x, y, Rgb([rgb[0], rgb[1], rgb[2]]));
        }
    }
    
    // Resize images to the same height if needed
    let orig_img = original_img.to_rgb8();
    let (orig_width, orig_height) = orig_img.dimensions();
    let (gen_width, gen_height) = generated_img.dimensions();
    let (diff_width, diff_height) = diff_img.dimensions();
    
    let target_height = orig_height.max(gen_height).max(diff_height);
    
    // Create three-way comparison image
    let separator = 20;
    let comparison_width = orig_width + diff_width + gen_width + 2 * separator;
    let mut comparison_img: RgbImage = ImageBuffer::new(comparison_width, target_height);
    
    // Fill with white background
    for pixel in comparison_img.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }
    
    // Copy original image to left side
    for y in 0..orig_height {
        for x in 0..orig_width {
            let pixel = orig_img.get_pixel(x, y);
            comparison_img.put_pixel(x, y, *pixel);
        }
    }
    
    // Copy difference image to middle
    let diff_offset_x = orig_width + separator;
    for y in 0..diff_height {
        for x in 0..diff_width {
            let pixel = diff_img.get_pixel(x, y);
            comparison_img.put_pixel(x + diff_offset_x, y, *pixel);
        }
    }
    
    // Copy generated image to right side
    let gen_offset_x = orig_width + diff_width + 2 * separator;
    for y in 0..gen_height {
        for x in 0..gen_width {
            let pixel = generated_img.get_pixel(x, y);
            comparison_img.put_pixel(x + gen_offset_x, y, *pixel);
        }
    }
    
    // Save comparison image
    let output_path = output_dir.join(format!("{}_comparison.png", test_name));
    comparison_img.save(&output_path)
        .unwrap_or_else(|e| panic!("Failed to save comparison PNG to {}: {}", output_path.display(), e));
    
    println!("Comparison image saved to: {}", output_path.display());
}

// ============================================================================
// Individual tests for each triplet
// ============================================================================

#[test]
fn cataclysm_mode0() {
    test_cataclysm_triplet("MODE0", 0);
}

#[test]
fn cataclysm_mode1() {
    test_cataclysm_triplet("MODE1", 1);
}

#[test]
fn cataclysm_mode2() {
    test_cataclysm_triplet("MODE2", 2);
}

#[test]
fn cataclysm_464good() {
    test_cataclysm_triplet("464GOOD", 1);
}

#[test]
fn cataclysm_force1() {
    test_cataclysm_triplet("FORCE1", 1);
}

#[test]
fn cataclysm_crtc() {
    test_cataclysm_triplet("CRTC", 1);
}


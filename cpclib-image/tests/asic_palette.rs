//! ASIC colours, `.kit` palettes, and the bridge from the Gate Array.

use cpclib_image::asic::{AsicColor, AsicColorComponent};
use cpclib_image::ga::{Ink, Pen};
use cpclib_image::kit::Kit;
use cpclib_image::palette::Palette;

/// The layout every other test here depends on: two bytes per colour,
/// `byte0 = RRRR BBBB`, `byte1 = 0000 GGGG`.
///
/// Written against the raw bytes on purpose - a swapped nibble would still
/// round trip through `Kit`, so a round-trip test alone would not catch it.
#[test]
fn a_kit_entry_places_each_component_in_its_documented_nibble() {
    // red = 4, blue = 10, green = 5
    let color = AsicColor::from_bytes([0x4A, 0x05]);
    assert_eq!(color.get_red().value(), 0x4);
    assert_eq!(color.get_blue().value(), 0xA);
    assert_eq!(color.get_green().value(), 0x5);

    assert_eq!(color.to_bytes(), [0x4A, 0x05], "the bytes must come back out");
    assert_eq!(AsicColor::new(0x4u8, 0x5u8, 0xAu8), color, "new() agrees");
}

/// Bits 4-7 are unused; the hardware does not store them and neither do we.
#[test]
fn the_unused_bits_are_discarded() {
    let color = AsicColor::from(0xFFFFu16);
    assert_eq!(color.value(), AsicColor::VALID_BITS);
    assert_eq!(color.to_bytes(), [0xFF, 0x0F]);
}

/// A whole 32-byte file survives the trip to a palette and back.
#[test]
fn a_kit_file_round_trips_through_a_palette() {
    let mut bytes = [0u8; Kit::BYTE_SIZE];
    for pen in 0..16u8 {
        // A different, valid colour per pen so a mis-indexed pen shows up.
        bytes[pen as usize * 2] = (pen << 4) | (15 - pen);
        bytes[pen as usize * 2 + 1] = pen;
    }

    let kit = Kit::from_bytes(bytes);
    let palette: Palette<AsicColor> = kit.into();
    for pen in 0..16u8 {
        let color = palette.get(&Pen::from(pen));
        assert_eq!(color.get_red().value(), pen, "red of pen {pen}");
        assert_eq!(color.get_blue().value(), 15 - pen, "blue of pen {pen}");
        assert_eq!(color.get_green().value(), pen, "green of pen {pen}");
    }

    assert_eq!(Kit::from(palette).to_bytes(), bytes);
}

/// The Gate Array's three levels per component, on the ASIC's 0-15 scale.
#[test]
fn every_ink_converts_to_an_asic_colour() {
    for value in 0..27u8 {
        let ink = Ink::from(value);
        let color = AsicColor::from(ink);
        for (name, component) in [
            ("red", color.get_red()),
            ("green", color.get_green()),
            ("blue", color.get_blue())
        ] {
            assert!(
                matches!(component.value(), 0x0 | 0x6 | 0xF),
                "ink {value}'s {name} became {:X}, which is not one of the \
                 Gate Array's three levels",
                component.value()
            );
        }
    }
}

/// Converting a whole palette keeps every pen where it was.
#[test]
fn a_gate_array_palette_converts_pen_for_pen() {
    let mut source = Palette::<Ink>::empty();
    for pen in 0..16u8 {
        source.set(Pen::from(pen), Ink::from(pen));
    }

    let converted: Palette<AsicColor> = source.clone().into();
    for pen in 0..16u8 {
        assert_eq!(
            *converted.get(&Pen::from(pen)),
            AsicColor::from(*source.get(&Pen::from(pen))),
            "pen {pen}"
        );
    }
}

/// Quantisation is the point of the Plus path: an arbitrary 24-bit pixel
/// becomes the nearest representable colour, rounding rather than truncating.
#[test]
fn a_pixel_quantises_to_the_nearest_representable_colour() {
    use image::Rgb;

    assert_eq!(AsicColor::from(Rgb([0, 0, 0])), AsicColor::new(0u8, 0u8, 0u8));
    assert_eq!(
        AsicColor::from(Rgb([255, 255, 255])),
        AsicColor::new(0xFu8, 0xFu8, 0xFu8)
    );

    // 136/255 is 8.0 sixteenths; truncation would give 7 and darken the image.
    assert_eq!(AsicColor::from(Rgb([136, 136, 136])).get_red().value(), 8);

    // ...and the value survives the trip back to a pixel.
    let color = AsicColor::new(0x4u8, 0xAu8, 0x5u8);
    let pixel: Rgb<u8> = color.into();
    assert_eq!(AsicColor::from(pixel), color, "round trip through a pixel");
}

/// A component wider than four bits cannot reach the hardware; it is truncated
/// rather than silently corrupting the neighbouring nibble.
#[test]
fn an_oversized_component_is_truncated() {
    assert_eq!(AsicColorComponent::from(0xFFu8).value(), 0xF);
    assert_eq!(AsicColor::new(0xFFu8, 0u8, 0u8).value(), 0xF000);
}

/// Both spellings the command line accepts, and that they agree.
#[test]
fn a_colour_can_be_written_packed_or_as_components() {
    use std::str::FromStr;

    let expected = AsicColor::new(0x4u8, 0xAu8, 0x5u8); // red 4, green 10, blue 5
    for spelling in ["4A5", "0x4A5", "0X4a5", "4,10,5", " 4 , 10 , 5 "] {
        assert_eq!(
            AsicColor::from_str(spelling).unwrap(),
            expected,
            "spelling {spelling:?}"
        );
    }
}

/// A wrong colour is refused with a reason, not silently clamped - a palette
/// entry the user did not mean is worse than a message.
#[test]
fn a_malformed_colour_is_refused_with_a_reason() {
    use std::str::FromStr;

    for (spelling, expected) in [
        ("4,10", "3 components"),
        ("4,16,5", "green is 16"),
        ("1FFF", "out of range"),
        ("zz", "hexadecimal"),
        ("", "empty")
    ] {
        let error = AsicColor::from_str(spelling)
            .expect_err(&format!("{spelling:?} must be refused"));
        assert!(
            error.contains(expected),
            "{spelling:?} said {error:?}, expected it to mention {expected:?}"
        );
    }
}

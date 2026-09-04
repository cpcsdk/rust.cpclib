//! `.pal` OCP palette files.

use cpclib_image::ocp::OcpPalette;

/// A well-formed buffer of the right size must decode; this is the baseline
/// the malformed-input tests below are contrasted against. The 3 header
/// bytes and the trailing excluded/projected pen bytes can be anything, but
/// the 204 ink bytes must each be a real Gate Array value (0x54 is ink 0's),
/// not an arbitrary byte.
#[test]
fn a_buffer_of_the_right_size_decodes() {
    let mut bytes = [0u8; OcpPalette::BYTE_SIZE];
    for b in &mut bytes[3..3 + 17 * 12] {
        *b = 0x54;
    }
    let pal = OcpPalette::from_buffer(&bytes).expect("well-formed buffer must decode");
    assert_eq!(pal.palettes().len(), 12);
}

/// `from_buffer` used to decode via `.next().unwrap()` on a plain iterator
/// with no length check at all - a short buffer panicked partway through
/// instead of being refused up front.
#[test]
fn a_truncated_buffer_is_refused_with_a_reason() {
    let bytes = [0u8; OcpPalette::BYTE_SIZE - 1];
    let err = OcpPalette::from_buffer(&bytes).err().expect("truncated buffer must be refused");
    assert!(
        err.contains(&(OcpPalette::BYTE_SIZE - 1).to_string()),
        "error {err:?} should mention the actual length"
    );
}

/// The trailing `assert!(data.next().is_none())` used to panic on a buffer
/// with extra trailing bytes; that must also become a refusal.
#[test]
fn an_oversized_buffer_is_refused_with_a_reason() {
    let bytes = [0u8; OcpPalette::BYTE_SIZE + 1];
    let err = OcpPalette::from_buffer(&bytes).err().expect("oversized buffer must be refused");
    assert!(
        err.contains(&(OcpPalette::BYTE_SIZE + 1).to_string()),
        "error {err:?} should mention the actual length"
    );
}

/// The empty buffer is the most trivially adversarial input.
#[test]
fn an_empty_buffer_is_refused() {
    assert!(OcpPalette::from_buffer(&[]).is_err());
}

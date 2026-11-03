//! Adversarial-input regression tests for the eDSK/AMSDOS parser.
//!
//! Each test hand-crafts a byte buffer that used to trigger a panic
//! (`assert!`/`assert_eq!`/`unreachable!`) in `cpclib_disc::edsk` and
//! checks it now returns `Err` instead, matching the fixes made to
//! `DiscInformation::from_buffer`, `TrackInformation::from_buffer` and
//! `ExtendedDsk::from_buffer`.

use cpclib_disc::edsk::{DiscInformation, ExtendedDsk, TrackInformation};

/// A valid 256-byte `DiscInformation` block: correct magic, 1 track, 1 head.
fn valid_disc_info_buffer() -> Vec<u8> {
    let mut buffer = vec![0u8; 256];
    buffer[..34].copy_from_slice(b"EXTENDED CPC DSK File\r\nDisk-Info\r\n");
    buffer[0x30] = 1; // number_of_tracks
    buffer[0x31] = 1; // number_of_heads
    buffer[0x34] = 1; // track_size_table[0]: one 256-byte track (header only, no sectors)
    buffer
}

/// A minimal valid `TrackInformation` header (0x18 bytes) with 0 sectors.
fn valid_track_header() -> Vec<u8> {
    let mut buffer = vec![0u8; 0x18];
    buffer[..12].copy_from_slice(b"Track-Info\r\n");
    buffer[0x10] = 0; // track_number
    buffer[0x11] = 0; // head_number
    buffer[0x12] = 1; // data_rate: SingleOrDoubleDensity
    buffer[0x13] = 2; // recording_mode: MFM
    buffer[0x14] = 2; // sector_size
    buffer[0x15] = 0; // number_of_sectors
    buffer[0x16] = 0x4E; // gap3_length
    buffer[0x17] = 0xE5; // filler_byte
    buffer
}

#[test]
fn truncated_disc_info_is_rejected() {
    let buffer = vec![0u8; 100];
    assert!(DiscInformation::from_buffer(&buffer).is_err());
}

#[test]
fn bad_magic_signature_is_rejected() {
    let mut buffer = valid_disc_info_buffer();
    buffer[0] = b'X';
    assert!(DiscInformation::from_buffer(&buffer).is_err());
}

#[test]
fn invalid_number_of_heads_is_rejected() {
    let mut buffer = valid_disc_info_buffer();
    buffer[0x31] = 3; // only 1 or 2 are valid
    assert!(DiscInformation::from_buffer(&buffer).is_err());
}

#[test]
fn overflowing_track_size_table_is_rejected() {
    let mut buffer = valid_disc_info_buffer();
    // 255 tracks * 2 heads of track-size bytes cannot fit in the
    // remaining `256 - 0x34` bytes of this block.
    buffer[0x30] = 255;
    buffer[0x31] = 2;
    assert!(DiscInformation::from_buffer(&buffer).is_err());
}

#[test]
fn truncated_track_header_is_rejected() {
    let buffer = vec![0u8; 5];
    assert!(TrackInformation::from_buffer(&buffer).is_err());
}

#[test]
fn bad_track_magic_is_rejected() {
    let mut buffer = valid_track_header();
    buffer[0] = b'X';
    assert!(TrackInformation::from_buffer(&buffer).is_err());
}

#[test]
fn invalid_data_rate_byte_is_rejected() {
    let mut buffer = valid_track_header();
    buffer[0x12] = 4; // only 0-3 are valid `DataRate` values
    assert!(TrackInformation::from_buffer(&buffer).is_err());
}

#[test]
fn invalid_recording_mode_byte_is_rejected() {
    let mut buffer = valid_track_header();
    buffer[0x13] = 3; // only 0-2 are valid `RecordingMode` values
    assert!(TrackInformation::from_buffer(&buffer).is_err());
}

#[test]
fn dsk_file_too_short_for_disc_info_is_rejected() {
    let buffer = vec![0u8; 10];
    assert!(ExtendedDsk::from_buffer(&buffer).is_err());
}

#[test]
fn dsk_track_declaring_more_bytes_than_present_is_rejected() {
    let disc_info = valid_disc_info_buffer();
    // Disc info declares one 256-byte track, but no track data follows at all.
    assert!(ExtendedDsk::from_buffer(&disc_info).is_err());
}

#[test]
fn well_formed_minimal_dsk_is_accepted() {
    // Sanity check: the same shape of buffer these tests corrupt, left
    // intact, must still parse - proving the errors above are really
    // caused by the specific corruption, not by the buffers' general shape.
    let mut buffer = valid_disc_info_buffer();
    buffer.extend(valid_track_header());
    buffer.resize(256 + 256, 0); // pad the track to its declared 256-byte size
    assert!(ExtendedDsk::from_buffer(&buffer).is_ok());
}

/// A standard DSK ("MV - CPC") is read as the extended one it converts to: its track size for
/// every track, its sectors 128 << N bytes.
#[test]
fn a_standard_dsk_is_read() {
    use cpclib_disc::amsdos::AmsdosManagerNonMut;
    use cpclib_disc::edsk::Head;

    for path in ["../cpclib-basic/tests/TEST-CAT.DSK", "../cpclib-catalog/tests/discs/BLOCUS/SRC.DSK"] {
        let bytes = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
        assert!(bytes.starts_with(b"MV - CPC"), "{path}: a standard image");
        let disc = ExtendedDsk::from_buffer(&bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
        let catalog = AmsdosManagerNonMut::new_from_disc(&disc, Head::A).catalog().unwrap().to_amsdos_catalog();
        assert!(!catalog.is_empty(), "{path}: its files");
        let first = catalog[0].file_name();
        let file = AmsdosManagerNonMut::new_from_disc(&disc, Head::A).get_file(*first).unwrap();
        assert!(file.is_some_and(|file| !file.header_and_content().is_empty()), "{path}: a file read");
    }
}

/// A standard DSK's sectors, read through the conversion, are its bytes; the tracks missing
/// from a short file are unformatted; garbage after the signature is refused, not a panic.
#[test]
fn a_standard_dsk_converts_to_the_same_sectors() {
    // 2 tracks, 1 head, 9 sectors of 512 bytes (&C1-&C9) a track; the file holds track 0 only
    let track_size = 256 + 9 * 512;
    let mut bytes = vec![0u8; 256];
    bytes[..34].copy_from_slice(b"MV - CPCEMU Disk-File\r\nDisk-Info\r\n");
    bytes[0x30] = 2;
    bytes[0x31] = 1;
    bytes[0x32..0x34].copy_from_slice(&(track_size as u16).to_le_bytes());
    let mut track = vec![0u8; 256];
    track[..12].copy_from_slice(b"Track-Info\r\n");
    track[0x14] = 2;
    track[0x15] = 9;
    track[0x16] = 0x4e;
    track[0x17] = 0xe5;
    for sector in 0..9u8 {
        let info = 0x18 + usize::from(sector) * 8;
        track[info + 2] = 0xc1 + sector;
        track[info + 3] = 2;
    }
    for sector in 0..9u8 {
        track.extend(std::iter::repeat_n(sector, 512));
    }
    bytes.extend(&track);

    let extended = cpclib_disc::edsk::standard_dsk_to_extended(&bytes).unwrap();
    assert_eq!(&extended[..8], b"EXTENDED");
    assert_eq!(&extended[0x34..0x36], &[(track_size / 256) as u8, 0], "track 1 is missing: unformatted");
    let disc = ExtendedDsk::from_buffer(&bytes).unwrap();
    use cpclib_disc::disc::Disc;
    for sector in 0..9u8 {
        let values = disc.sector(0, 0, 0xc1 + sector).unwrap().values().to_vec();
        assert_eq!(values, vec![sector; 512], "sector &{:02X}", 0xc1 + sector);
    }

    let mut garbage = bytes.clone();
    garbage[0x32..0x34].copy_from_slice(&100u16.to_le_bytes());
    assert!(ExtendedDsk::from_buffer(&garbage).is_err(), "a track size that isn't one");
}

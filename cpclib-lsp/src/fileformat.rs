//! Structural region decoding for binary CPC file formats - what a hex
//! viewer needs to highlight "this is the header," "this is chunk 'MEM0'."
//!
//! Static, no running emulator involved: `.sna` and `.cpr` both already have
//! a real, tested Rust decoder elsewhere in this workspace (`cpclib-sna`,
//! `cpclib-cpr`) - this only asks each what its own on-disk layout is,
//! walking the exact same byte-consuming steps their own `from_buffer`
//! already does, rather than re-parsing anything from scratch.
//!
//! `.dsk` and `.cdt` are deliberately not handled yet: `cpclib_disc::edsk::
//! DiscInformation::from_buffer` only understands the *extended* `.dsk`
//! signature (its own doc comment literally says
//! `TODO manage the case of standard dsk`), so a byte-accurate region list
//! for a standard-format disc would mean writing new, unverified parsing
//! rather than reusing something already trusted - exactly the guessing
//! this module exists to avoid. `.cdt`/TZX block structure was not
//! investigated this pass either. Both are a named follow-up, not a
//! silent gap: `file_regions` returns an empty list for them today, which
//! the client already renders as "no known structure" rather than an error.

use camino::Utf8Path;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FileRegion {
    pub offset: usize,
    pub length: usize,
    pub label: String,
    /// A coarse category ("header", "memory", "chunk") - the client picks
    /// colors from this, not this module, since palette choice is a
    /// presentation concern.
    pub kind: String
}

fn region(offset: usize, length: usize, label: impl Into<String>, kind: &str) -> FileRegion {
    FileRegion { offset, length, label: label.into(), kind: kind.to_string() }
}

/// The structural regions of `bytes`, dispatched by `path`'s extension.
/// Empty (not an error) for a format this module does not decode - a
/// hex view still opens, just with no overlay.
pub fn file_regions(path: &Utf8Path, bytes: &[u8]) -> Vec<FileRegion> {
    match path.extension().map(str::to_ascii_lowercase).as_deref() {
        Some("sna") => sna_regions(bytes),
        Some("cpr") => cpr_regions(bytes),
        _ => Vec::new()
    }
}

/// Mirrors `cpclib_sna::Snapshot::from_buffer` byte-for-byte: a fixed
/// `HEADER_SIZE`-byte header, then `memory_size_header()` KB of raw memory,
/// then (v3 only) a sequence of `[4-byte code][4-byte LE length][data]`
/// chunks with no padding - confirmed by reading `RiffChunk::from_buffer`
/// itself (`cpclib-common/src/riff.rs`), which drains exactly `cksz.value()`
/// data bytes and nothing else.
fn sna_regions(bytes: &[u8]) -> Vec<FileRegion> {
    let Ok(sna) = cpclib_sna::Snapshot::from_buffer(bytes.to_vec())
    else {
        return Vec::new();
    };

    let mut regions = vec![region(0, cpclib_sna::HEADER_SIZE, "Header", "header")];
    let mut offset = cpclib_sna::HEADER_SIZE;

    // 16K pages - a real CPC memory-mapping boundary, not invented
    // structure, so it is worth showing even though the file itself stores
    // this as one contiguous dump.
    let mut remaining = sna.memory_size_header() as usize * 1024;
    let mut page = 0usize;
    while remaining > 0 {
        let this = remaining.min(0x4000);
        regions.push(region(
            offset,
            this,
            format!("RAM page {page} (0x{:04X})", page * 0x4000),
            "memory"
        ));
        offset += this;
        remaining -= this;
        page += 1;
    }

    for chunk in sna.chunks() {
        let len = 8 + chunk.data().len();
        regions.push(region(offset, len, format!("Chunk '{}'", chunk.code()), "chunk"));
        offset += len;
    }

    regions
}

/// Mirrors `cpclib_cpr::Cpr::from_buffer` byte-for-byte: `"RIFF"` + a 4-byte
/// LE total length + `"AMS!"`, then a sequence of RIFF chunks (cartridge
/// banks, or a leading `"fmt "` chunk) with no padding - same `RiffChunk`
/// this reuses for `.sna`.
fn cpr_regions(bytes: &[u8]) -> Vec<FileRegion> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"AMS!" {
        return Vec::new();
    }

    let mut regions = vec![region(0, 12, "RIFF/AMS! header", "header")];
    let mut offset = 12;
    let mut remaining = bytes[12..].to_vec();

    while remaining.len() >= 8 {
        let chunk = cpclib_common::riff::RiffChunk::from_buffer(&mut remaining);
        let len = 8 + chunk.data().len();
        let label = if chunk.code().to_string() == "fmt " {
            "Format chunk 'fmt '".to_string()
        }
        else {
            format!("Cartridge bank '{}'", chunk.code())
        };
        regions.push(region(offset, len, label, "chunk"));
        offset += len;
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_sna_lists_its_header_memory_and_chunks_at_the_right_offsets() {
        use cpclib_sna::{Snapshot, SnapshotVersion};

        let mut sna = Snapshot::default();
        sna.add_chunk(cpclib_sna::MemoryChunk::new("MEM0", vec![0u8; 4]));

        let mut buffer = Vec::new();
        sna.write_all(&mut buffer, SnapshotVersion::V3).unwrap();

        let regions = file_regions(Utf8Path::new("test.sna"), &buffer);

        let header = &regions[0];
        assert_eq!((header.offset, header.length, header.kind.as_str()), (0, 256, "header"));

        let memory_size = sna.memory_size_header() as usize * 1024;
        let memory_regions: Vec<_> = regions.iter().filter(|r| r.kind == "memory").collect();
        let memory_total: usize = memory_regions.iter().map(|r| r.length).sum();
        assert_eq!(memory_total, memory_size);
        assert_eq!(memory_regions[0].offset, 256);

        let last_memory_end = memory_regions.last().unwrap().offset + memory_regions.last().unwrap().length;
        let chunk = regions
            .iter()
            .find(|r| r.kind == "chunk")
            .unwrap_or_else(|| panic!("no chunk region in {regions:?}"));
        assert_eq!(chunk.offset, last_memory_end);
        // 4-byte code + 4-byte length + the chunk's own 4 data bytes.
        assert_eq!(chunk.length, 12);
    }

    #[test]
    fn a_cpr_lists_its_header_and_each_bank_at_the_right_offset() {
        use cpclib_cpr::{CartridgeBank, Cpr};

        let cpr = Cpr::from(vec![CartridgeBank::new(0), CartridgeBank::new(1)]);
        let mut buffer = Vec::new();
        cpr.write_all(&mut buffer).unwrap();

        let regions = file_regions(Utf8Path::new("test.cpr"), &buffer);

        assert_eq!((regions[0].offset, regions[0].length, regions[0].kind.as_str()), (0, 12, "header"));
        assert_eq!(regions[1].offset, 12);
        assert_eq!(regions[1].length, 8 + 0x4000);
        assert!(regions[1].label.contains("cb00"), "{}", regions[1].label);
        assert_eq!(regions[2].offset, 12 + 8 + 0x4000);
        assert!(regions[2].label.contains("cb01"), "{}", regions[2].label);
    }

    #[test]
    fn an_unhandled_extension_yields_no_regions_rather_than_guessing() {
        assert!(file_regions(Utf8Path::new("test.dsk"), &[1, 2, 3]).is_empty());
        assert!(file_regions(Utf8Path::new("test.cdt"), &[1, 2, 3]).is_empty());
    }
}

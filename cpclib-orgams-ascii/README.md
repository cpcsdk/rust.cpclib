# cpclib-orgams-ascii

Rust library for reading and writing Orgams binary files (.O format).

## What is Orgams?

Orgams is a Z80 assembler for the Amstrad CPC. It uses a preprocessed binary format (.O files) that is more compact than plain ASCII assembly (.Z80 files).

## File Format

The Orgams binary format has been reverse-engineered through analysis of file pairs:

### Structure

```
[MAGIC: 4 bytes] - "ORGA"
[VERSION: 1 byte] - Typically 0x02
[METADATA: ~98 bytes] - Fixed-size header table (purpose: offsets/config)
[CONTENT: variable] - Encoded source lines and data
```

### Content Encoding

The content section contains preprocessed assembly lines encoded as:

```
[MARKER: 1 byte] [LENGTH: 1 byte] [TEXT: LENGTH bytes]
```

**Marker bytes:**
- `0x43` ('C') - Comment or text line (rendered as `;  text`)
- `0x49` ('I') - Indented comment/code (rendered with indentation)
- `0x4A` ('J') - New line or continuation marker  
- `0x64` ('d') - Data or directive
- `0x41` ('A') - Assembly instruction

The content may also contain:
- "SRC" section markers with metadata
- Embedded control codes and nested markers
- Binary data interspersed with text

## Usage

### Reading an Orgams file

```rust
use cpclib_orgams_ascii::OrgamsFile;
use std::fs::File;

let file = File::open("program.O")?;
let orgams = OrgamsFile::read(file)?;

println!("Version: {}", orgams.header.version);
println!("Content size: {} bytes", orgams.content.len());
```

### Extracting lines

```rust
let lines = orgams.extract_lines();

for (marker, text) in &lines {
    match marker {
        Some(LineMarker::Comment) => println!("; {}", text),
        Some(LineMarker::Assembly) => println!("  {}", text),
        _ => println!("{}", text),
    }
}
```

### Converting to Z80 text

```rust
let z80_text = orgams.to_z80_text();
println!("{}", z80_text);
```

### Writing an Orgams file

```rust
use std::io::Cursor;

let mut buffer = Vec::new();
orgams.write(&mut buffer)?;

// Perfect round-trip: reading and writing preserves the exact binary
```

## Current Status

✅ **Working:**
- Reading Orgams files with magic/version/metadata/content parsing
- Perfect round-trip (read → write produces identical binary)
- Basic line extraction with marker identification
- File format reverse-engineered and documented

⚠️ **In Progress:**
- Full text reconstruction (embedded markers and control codes need handling)
- Z80 source conversion improvements
- Creating Orgams files from scratch
- Detailed metadata table interpretation

## Testing

The crate includes comprehensive tests using real Orgams/Z80 file pairs:

```bash
cargo test -p cpclib-orgams-ascii
```

Test files are located in `tests/orgams-main/` with pairs of:
- `.O` files (Orgams binary format)
- `.Z80` files (ASCII assembly source)

## References

- No official documentation exists for the Orgams binary format
- Format reverse-engineered through empirical analysis
- Test corpus: 73+ file pairs from real Orgams projects

## License

Same as parent cpclib project.

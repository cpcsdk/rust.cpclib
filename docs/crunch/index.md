# Crunch - Data Compression Tool

The `crunch` tool is a command-line utility for compressing binary data for Amstrad CPC programs using various Z80-optimized compression algorithms.

## Installation

The crunch tool is built as part of the cpclib toolchain:

```bash
cargo install cpclib-crunch
```

## Overview

Crunch provides access to multiple compression algorithms specifically optimized for Z80-based systems like the Amstrad CPC. Each algorithm offers different trade-offs between compression ratio, decompression speed, and memory usage.

### Supported Compression Algorithms

- **Apultra** - High compression ratio with good decompression speed
- **Exomizer** - Excellent compression, widely used in retro computing
- **Lz4** - Very fast decompression, moderate compression
- **Lz48** - Lightweight LZ-based compression
- **Lz49** - Enhanced version of LZ48
- **Lzsa1** - Fast decompression with good compression ratio
- **Lzsa2** - Better compression than LZSA1, slightly slower decompression
- **Shrinkler** - Extreme compression for size-critical applications
- **Upkr** - Efficient general-purpose compression
- **Zx0** - Modern, efficient compression for Z80 systems

## Quick Start

Compress a binary file:

```bash
crunch -c apultra -i input.bin -o output.crunched
```

Extract Z80 decompression source code:

```bash
crunch -c apultra -z
```

## Usage

```
crunch [OPTIONS] -c <CRUNCHER>
```

### Options

## Integration with BndBuild

Crunch is available as a standalone `crunch` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `crunch` or `compress` command aliases. See [BndBuild Commands](../bndbuild/commands.md#compression-crunch-crunchcompress) for integration details.

### Options

- `-c, --cruncher <CRUNCHER>` - Select the compression algorithm to use (required)
  - Choices: `apultra`, `exomizer`, `lz4`, `lz48`, `lz49`, `lzsa1`, `lzsa2`, `shrinkler`, `upkr`, `zx0`

- `-i, --input <INPUT>` - Input file to compress
  - Can be a binary file (e.g., `my_file.o`)
  - Can be an Amsdos file (e.g., `FILE.BIN`)
  - Can be a file within a disc image (e.g., `my_disc.dsk#FILE.BIN`)

- `-o, --output <OUTPUT>` - Compressed output file (requires `--input`)
  - Can be a binary file
  - Can be an Amsdos file
  - Can be a file within a disc image

- `-k, --keep-header` - Also compress the Amsdos header
  - Useful for binary files where the first bytes contain a valid Amsdos header

- `-H, --header` - Add an Amsdos header when storing the file on the host

- `-z, --z80` - Display the Z80 decompression source code
  - Cannot be used with `--input`
  - Outputs the assembly code for decompressing data

## Examples

### Basic Compression

Compress a file using Apultra:

```bash
crunch -c apultra -i game.bin -o game.crunched
```

### Compress with Amsdos Header

Compress a file and add an Amsdos header:

```bash
crunch -c exomizer -i data.bin -o data.crunched -H
```

### Keep Original Header

Compress a file while preserving its Amsdos header:

```bash
crunch -c lzsa1 -i FILE.BIN -o FILE.CRN -k
```

### Compress File in DSK

Compress a file and store it directly in a disc image:

```bash
crunch -c zx0 -i sprite.bin -o game.dsk#SPRITE.CRN
```

### Get Decompression Code

Extract the Z80 assembly decompression routine:

```bash
crunch -c apultra -z > unaplib.asm
```

This outputs the Z80 decompression source code that can be included in your assembly projects.

## Choosing a Compression Algorithm

Different algorithms are suited for different use cases:

### For Maximum Compression
- **Shrinkler** - Best compression ratio, but slower decompression
- **Exomizer** - Excellent compression, widely tested and reliable

### For Fast Decompression
- **Lz4** - Very fast decompression, ideal for realtime applications
- **Lzsa1** - Good balance of speed and compression
- **Zx0** - Modern algorithm with fast decompression

### For General Use
- **Apultra** - Good all-around choice
- **Lzsa2** - Better compression than LZSA1, still reasonably fast

### Lightweight Options
- **Lz48/Lz49** - Small decompressor code, good for size-constrained projects
- **Upkr** - Efficient general-purpose compression

## File Format Support

### Binary Files
Standard binary files without headers:
```bash
crunch -c apultra -i data.o -o data.crunched
```

### Amsdos Files
Files with Amsdos headers (load address, execution address):
```bash
crunch -c exomizer -i SCREEN.BIN -o SCREEN.CRN
```

### Disc Images
Files stored within DSK disc images:
```bash
crunch -c zx0 -i game.dsk#LEVEL1.BIN -o game.dsk#LEVEL1.CRN
```

## Using Compressed Data

After compressing your data, you need to include the corresponding decompression routine in your Z80 code:

1. Get the decompression source:
```bash
crunch -c apultra -z > unaplib.asm
```

2. Include it in your assembly project:
```asm
    INCLUDE "unaplib.asm"
    
    ; Load compressed data address
    ld hl, compressed_data
    ld de, destination
    call decompress
```

Each decompressor has its own calling convention - check the generated source code for specific details.

## Integration with BASM

The crunch tool can be used directly in BASM assembly projects:

```asm
; In your BASM source
compressed_data:
    INCBIN "data.crunched"
```

Or use the CRUNCH directive in BASM for automatic compression:

```asm
CRUNCH "apultra", "data.bin"
```

## Notes

- Compression time may vary significantly between algorithms
- Shrinkler provides the best compression but can be slow to compress
- Always test decompression speed on actual hardware for timing-critical code
- The `-k, --keep-header` flag is useful when the header contains important data

## See Also

- [BASM Documentation](../basm) - Assembly language support with compression directives
- [BndBuild Documentation](../bndbuild) - Build system integration
- [Disc Management](../disc) - Working with DSK disc images

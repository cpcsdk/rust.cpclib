# Crunch Command Line Reference

Complete command-line reference for the `crunch` data compression tool.

## Synopsis

```
crunch [OPTIONS] -c <CRUNCHER>
```

## Global Options

### `-c, --cruncher <CRUNCHER>`

**Required.** Specifies the compression algorithm to use.

**Choices:**
- `apultra` - APultra compression
- `exomizer` - Exomizer compression
- `lz4` - LZ4 compression
- `lz48` - LZ48 compression
- `lz49` - LZ49 compression
- `lzsa1` - LZSA version 1
- `lzsa2` - LZSA version 2
- `shrinkler` - Shrinkler compression
- `upkr` - UPKR compression
- `zx0` - ZX0 compression

**Example:**
```bash
crunch -c apultra -i input.bin -o output.crunched
```

## Input/Output Options

### `-i, --input <INPUT>`

Specifies the input file to compress.

**Formats:**
- Simple binary: `data.bin`
- Amsdos file: `SCREEN.BIN`
- File in disc: `game.dsk#LEVEL1.BIN`

**Example:**
```bash
crunch -c exomizer -i sprite.bin -o sprite.crunched
```

### `-o, --output <OUTPUT>`

Specifies the compressed output file. Requires `--input` to be specified.

**Formats:**
- Simple binary: `data.crunched`
- Amsdos file: `DATA.CRN`
- File in disc: `game.dsk#DATA.CRN`

**Example:**
```bash
crunch -c zx0 -i data.bin -o game.dsk#DATA.CRN
```

## Header Options

### `-k, --keep-header`

Compress the Amsdos header along with the file data.

By default, Amsdos headers are stripped before compression. Use this flag when the header contains important data that should be compressed.

**Default:** `false`

**Example:**
```bash
crunch -c apultra -i FILE.BIN -o FILE.CRN -k
```

### `-H, --header`

Add an Amsdos header when storing the compressed file on the host.

**Default:** `false`

**Example:**
```bash
crunch -c lzsa1 -i data.bin -o DATA.CRN -H
```

## Source Code Options

### `-z, --z80`

Display the Z80 assembly decompression source code for the selected algorithm.

Cannot be used with `--input`. This option outputs the decompressor routine that you can include in your Z80 assembly projects.

**Conflicts with:** `--input`

**Default:** `false`

**Example:**
```bash
crunch -c apultra -z > unaplib.asm
```

## Common Usage Patterns

### Compress a Binary File

```bash
crunch -c apultra -i data.bin -o data.crunched
```

### Compress with Amsdos Header

```bash
crunch -c exomizer -i graphics.bin -o GRAPHICS.CRN -H
```

### Compress File Preserving Original Header

```bash
crunch -c lzsa2 -i MUSIC.BIN -o MUSIC.CRN -k
```

### Compress into Disc Image

```bash
crunch -c zx0 -i level1.bin -o game.dsk#LEVEL1.CRN
```

### Extract Decompression Routine

```bash
crunch -c apultra -z
```

### Compare Algorithms

Test different algorithms to find the best one for your data:

```bash
crunch -c apultra -i data.bin -o data.apultra
crunch -c zx0 -i data.bin -o data.zx0
crunch -c shrinkler -i data.bin -o data.shrink
```

Then compare file sizes to choose the most effective algorithm.

## Exit Status

- **0** - Success
- **Non-zero** - Error occurred (invalid arguments, compression failed, file I/O error)

## Notes

- The `--cruncher` option is always required
- Use `--z80` to get the decompression routine for your assembly projects
- Files in disc images use the format `disc.dsk#FILE.BIN`
- Amsdos headers are automatically detected and handled by default
- The `--keep-header` flag is useful when headers contain data beyond standard metadata

## See Also

- [Crunch Overview](index.md) - Main documentation
- [Examples](examples.md) - Detailed usage examples

# Crunch Examples

Practical examples of using the `crunch` data compression tool for Amstrad CPC projects.

## Basic Compression Examples

### Compress a Graphics File

```bash
# Compress a screen dump with Apultra
crunch -c apultra -i screen.scr -o screen.crunched

# File size comparison
ls -lh screen.scr screen.crunched
```

### Compress Game Data

```bash
# Compress level data with ZX0
crunch -c zx0 -i level1.dat -o level1.zx0

# Compress with Exomizer for better ratio
crunch -c exomizer -i level1.dat -o level1.exo
```

### Compress Multiple Files

```bash
# Compress all level files with a loop
for file in level*.bin; do
    crunch -c lzsa1 -i "$file" -o "${file%.bin}.lzsa"
done
```

## Working with Amsdos Headers

### Add Header During Compression

```bash
# Compress and add Amsdos header
crunch -c apultra -i graphics.bin -o GRAPHICS.CRN -H
```

### Preserve Original Header

```bash
# Keep the original Amsdos header in compressed data
crunch -c exomizer -i MUSIC.BIN -o MUSIC.EXO -k
```

### Compress Binary Without Header

```bash
# Default behavior - strip header before compression
crunch -c zx0 -i SPRITE.BIN -o sprite.zx0
```

## Disc Image Integration

### Compress File into DSK

```bash
# Compress and store directly in disc image
crunch -c apultra -i titlescreen.scr -o game.dsk#TITLE.CRN
```

### Compress from DSK to DSK

```bash
# Extract from one disc, compress, store in another
crunch -c zx0 -i source.dsk#DATA.BIN -o target.dsk#DATA.ZX0
```

### Build Compressed Disc

```bash
# Create a disc with all compressed assets
crunch -c lzsa1 -i screen1.scr -o game.dsk#SCR1.LSA
crunch -c lzsa1 -i screen2.scr -o game.dsk#SCR2.LSA
crunch -c lzsa1 -i music.bin -o game.dsk#MUSIC.LSA
```

## Algorithm Comparison

### Test Multiple Algorithms

```bash
#!/bin/bash
# Compare compression ratios for different algorithms

INPUT="level1.bin"
ORIGINAL_SIZE=$(stat -f%z "$INPUT")

echo "Original size: $ORIGINAL_SIZE bytes"
echo ""

for algo in apultra exomizer lz4 lzsa1 lzsa2 zx0 shrinkler; do
    OUTPUT="level1.$algo"
    crunch -c $algo -i "$INPUT" -o "$OUTPUT"
    COMPRESSED_SIZE=$(stat -f%z "$OUTPUT")
    RATIO=$(echo "scale=2; 100 * $COMPRESSED_SIZE / $ORIGINAL_SIZE" | bc)
    echo "$algo: $COMPRESSED_SIZE bytes ($RATIO%)"
done
```

### Performance Testing

Create a test program to measure decompression speed:

```asm
; test_decompress.asm
    org &8000

start:
    ; Get start time
    ld bc, &BC06
    out (c), c
    ld bc, &BD00
    in a, (c)           ; Get initial time
    ld (start_time), a
    
    ; Decompress
    ld hl, compressed_data
    ld de, &4000
    call decompress
    
    ; Get end time
    ld bc, &BC06
    out (c), c
    ld bc, &BD00
    in a, (c)
    ld hl, start_time
    sub (hl)
    
    ; Display result
    call display_time
    ret

    include "unaplib.asm"  ; Or other decompressor

compressed_data:
    incbin "data.crunched"

start_time:
    db 0
```

## Using Decompression Routines

### Extract Z80 Source

```bash
# Get Apultra decompressor
crunch -c apultra -z > unaplib.asm

# Get ZX0 decompressor
crunch -c zx0 -z > dzx0.asm

# Get LZSA1 decompressor
crunch -c lzsa1 -z > unlzsa1.asm
```

### Use in BASM Project

```asm
; main.asm - Load and decompress data
    org &8000

start:
    ; Decompress screen
    ld hl, screen_compressed
    ld de, &c000
    call decompress_apultra
    
    ; Decompress sprites
    ld hl, sprites_compressed
    ld de, sprite_buffer
    call decompress_apultra
    
    ret

    ; Include decompressor
    include "unaplib.asm"

decompress_apultra:
    jp decompress  ; Entry point from unaplib.asm

screen_compressed:
    incbin "screen.crunched"

sprites_compressed:
    incbin "sprites.crunched"

sprite_buffer:
    ds 4096
```

## Real-World Scenarios

### Loading Screen Compression

```bash
# Compress a mode 0 loading screen
crunch -c apultra -i loading.scr -o LOADING.CRN -H

# Use in BASIC loader:
# LOAD "LOADING.CRN"
# CALL &8000
```

### Multi-Part Game Data

```bash
# Compress game resources with optimal algorithms

# Simple graphics - use fast decompression
crunch -c lz4 -i tiles.bin -o game.dsk#TILES

# Large level data - use best compression
crunch -c shrinkler -i world.bin -o game.dsk#WORLD

# Frequently accessed - balance speed/size
crunch -c zx0 -i sprites.bin -o game.dsk#SPRITES
```

### Music Data Compression

```bash
# Compress AY music data
crunch -c lzsa2 -i music.aky -o MUSIC.LSA -H

# Compress multiple tracks
for track in track*.aky; do
    name=$(basename "$track" .aky)
    crunch -c lzsa2 -i "$track" -o "game.dsk#${name}.LSA"
done
```

## Build System Integration

### Makefile Example

```makefile
# Makefile for game with compressed assets

CRUNCH = crunch
CRUNCHER = apultra

SCREENS = screen1.scr screen2.scr screen3.scr
SCREENS_CRN = $(SCREENS:.scr=.crunched)

GAME.DSK: loader.bin $(SCREENS_CRN) sprites.crunched
	# Create disc and add files
	$(CRUNCH) -c $(CRUNCHER) -i loader.bin -o $@#LOADER.BIN
	$(CRUNCH) -c $(CRUNCHER) -i screen1.crunched -o $@#SCR1.CRN
	$(CRUNCH) -c $(CRUNCHER) -i screen2.crunched -o $@#SCR2.CRN
	$(CRUNCH) -c $(CRUNCHER) -i screen3.crunched -o $@#SCR3.CRN
	$(CRUNCH) -c $(CRUNCHER) -i sprites.crunched -o $@#SPRITE.CRN

%.crunched: %.scr
	$(CRUNCH) -c $(CRUNCHER) -i $< -o $@

%.crunched: %.bin
	$(CRUNCH) -c $(CRUNCHER) -i $< -o $@

clean:
	rm -f *.crunched GAME.DSK
```

### BndBuild Integration

```yaml
# build.yml for bndbuild
build:
  - cmd: basm
    input: main.asm
    output: main.bin
  
  - cmd: crunch
    args: ["-c", "apultra", "-i", "graphics.bin", "-o", "graphics.crunched"]
  
  - cmd: dsk
    args: ["create", "game.dsk"]
  
  - cmd: dsk
    args: ["add", "game.dsk", "main.bin"]
  
  - cmd: dsk
    args: ["add", "game.dsk", "graphics.crunched", "GRAPHICS.CRN"]
```

## Optimization Tips

### Choose Algorithm Based on Data Type

```bash
# Screens (high redundancy) - use strong compression
crunch -c exomizer -i screen.scr -o screen.exo

# Code (medium redundancy) - balance speed/size
crunch -c zx0 -i code.bin -o code.zx0

# Sprite data (low redundancy) - fast decompression
crunch -c lz4 -i sprites.bin -o sprites.lz4

# Music (specific patterns) - test multiple
crunch -c lzsa2 -i music.bin -o music.lzsa
```

### Batch Processing

```bash
#!/bin/bash
# Compress all assets for a game

# Create output directory
mkdir -p compressed

# Compress screens with maximum compression
for scr in assets/screens/*.scr; do
    name=$(basename "$scr" .scr)
    echo "Compressing screen: $name"
    crunch -c shrinkler -i "$scr" -o "compressed/${name}.crn"
done

# Compress sprites with fast decompression
for spr in assets/sprites/*.bin; do
    name=$(basename "$spr" .bin)
    echo "Compressing sprites: $name"
    crunch -c lz4 -i "$spr" -o "compressed/${name}.lz4"
done

echo "Compression complete!"
```

## See Also

- [Crunch Overview](index.md) - Main documentation
- [Command Line Reference](cmdline.md) - Complete options
- [BASM Documentation](../basm) - Assembly integration

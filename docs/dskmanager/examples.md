# DSKManager Examples

## Basic Operations

### Create a New Formatted DSK

```bash
# Format a standard Data format DSK
dskmanager format mydisk.dsk

# Format a System format DSK
dskmanager format system.dsk --system
```

### Add Files to DSK

```bash
# Add single file
dskmanager add mydisk.dsk program.bin

# Add multiple files
dskmanager add mydisk.dsk file1.bin file2.bas screen.scr

# Add files with specific AMSDOS parameters
dskmanager add mydisk.dsk code.bin --load 0x4000 --exec 0x4000
```

### View DSK Contents

```bash
# List all files
dskmanager catalog mydisk.dsk

# Show detailed information
dskmanager catalog mydisk.dsk --verbose
```

### Extract Files

```bash
# Extract all files to current directory
dskmanager extract mydisk.dsk

# Extract to specific directory
dskmanager extract mydisk.dsk output/

# Extract specific files
dskmanager extract mydisk.dsk --files "PROGRAM.BIN,SCREEN.SCR"
```

## Workflow Examples

### Building a Demo Distribution Disk

```bash
# Create new disk
dskmanager format demo.dsk

# Add loader
dskmanager add demo.dsk loader.bin --load 0x0170 --exec 0x0170

# Add demo parts
dskmanager add demo.dsk part1.scr part2.bin music.bin

# Add catalog screen
catalog demo.dsk build --output catart.scr
dskmanager add demo.dsk catart.scr
```

### Using with BndBuild

In your `bndbuild.yml`:

```yaml
- tgt: release.dsk
  dep: 
    - demo.bin
    - loader.bin
    - screen.scr
  cmd:
    - dsk format release.dsk
    - dsk add release.dsk loader.bin --load 0x170 --exec 0x170
    - dsk add release.dsk demo.bin screen.scr
    - catalog release.dsk build --output catalog.scr
    - dsk add release.dsk catalog.scr
```

## Advanced Usage

### Custom Disc Formats

```bash
# Create disc with custom track/sector layout
dskmanager format custom.dsk --tracks 40 --sectors 9 --sector-size 512
```

### Working with AMSDOS Headers

```bash
# Add binary with full AMSDOS header control
dskmanager add game.dsk code.bin \
  --user 0 \
  --load 0x4000 \
  --exec 0x4000 \
  --type 2 \
  --name "GAME"
```

## Integration Examples

### With Hideur

```bash
# Add AMSDOS header first, then add to disk
hideur add myfile.bin --load 0x8000 --exec 0x8000
dskmanager add mydsk.dsk myfile.bin
```

### With Catalog

```bash
# Create disk, add files, generate catalog
dskmanager format demo.dsk
dskmanager add demo.dsk *.bin *.scr
catalog demo.dsk build --style classic --output catalog.scr
dskmanager add demo.dsk catalog.scr
```

## Tips

- Always format a new DSK before adding files
- Use `--verbose` to see detailed operation progress
- AMSDOS headers are preserved during add/extract operations
- For CPC transfers, pair with `xfertool` to send DSKs to real hardware

## Related Documentation

- [Command Line Reference](cmdline.md) - Complete command syntax
- [Catalog Examples](../catalog/examples.md) - Creating catalog screens
- [Hideur Examples](../hideur/examples.md) - Managing AMSDOS headers
- [BndBuild Examples](../bndbuild/examples.md) - Build system integration

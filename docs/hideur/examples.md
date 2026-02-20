# Hideur Examples

## Display File Information

Check AMSDOS header of an existing file:

```bash
hideur myfile.bin --info
```

## Binary Files

### Add Binary Header with Load and Execution Address

```bash
hideur code.bin -o code.bin -t binary -l 0x4000 -x 0x4000
```

### Binary with Different Execution Address

```bash
hideur loader.bin -o loader.bin -t binary -l 0x170 -x 0x4000
```

### Binary for Specific User

```bash
hideur game.bin -o game.bin -t binary -l 0x8000 -x 0x8000 -u 1
```

## Basic Programs

### Add Basic Header

```bash
hideur program.bas -o program.bas -t basic
```

### Protected Basic

```bash
hideur protected.bas -o protected.bas -t protected
```

## Workflow Examples

### Preparing Files for DSK

Add proper AMSDOS headers before adding files to a disk:

```bash
# Add header to binary
hideur demo.bin -o demo.bin -t binary -l 0x4000 -x 0x4000

# Then add to DSK using catalog tool
catalog mydisk.dsk modify --add demo.bin
```

### Converting Raw Binary to Executable

```bash
# Raw binary from assembler
hideur output.bin -o DEMO.BIN -t binary -l 0x8000 -x 0x8000
```

## Integration with Build Systems

In a bndbuild.yml file:

```yaml
- tgt: game.bin
  dep: game_raw.bin
  cmd: hideur $< -o $@ -t binary -l 0x4000 -x 0x4000
```

## Common Patterns

### Screen File
```bash
hideur screen.scr -o SCREEN.SCR -t binary -l 0xC000
```

### Demo Loader
```bash
hideur loader.bin -o LOADER.BIN -t binary -l 0x170 -x 0x4000
```

### Music Data
```bash
hideur music.bin -o MUSIC.BIN -t binary -l 0x8000
```

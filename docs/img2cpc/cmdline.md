# img2cpc Command Line Reference

## Synopsis

```bash
img2cpc [OPTIONS] <SOURCE> [COMMAND]
```

## Description

Converts modern image formats (PNG, BMP, JPEG, etc.) to various Amstrad CPC output formats.

## Arguments

- `<SOURCE>` - Filename to convert

## Commands

### `sna`
Generate a snapshot with the converted image.

```bash
img2cpc image.png sna IMAGE.SNA
```

### `dsk`
Generate a DSK with an executable of the converted image.

```bash
img2cpc image.png dsk IMAGE.DSK
```

### `scr`
Generate an OCP SCR file.

```bash
img2cpc image.png scr --output IMAGE.SCR
```

### `exec`
Generate a binary file to manually copy in a DSK or M4 folder.

```bash
img2cpc image.png exec IMAGE.BIN
```

### `sprite`
Generate a sprite file to be included inside an application.

See `img2cpc sprite --help` for detailed usage.

Every subcommand that produces data can also write the palette out beside it:

- `-p, --palette <FILE>` - the 17 Gate Array bytes
- `--kit <FILE>` - the 32 Amstrad Plus bytes
- `-i, --inks <FILE>` - the ink numbers
- `--palette_fadeout <FILE>`, `--ink_fadeout <FILE>` - every step of a fade to black

`--palette`, `--inks` and the fade-outs describe Gate Array inks, so they are
refused for an Amstrad Plus palette and point at `--kit` instead. `--kit` works
for either: the 27 inks all have an exact ASIC colour.

### `tile`
Generate a list of sprites (tile map).

See `img2cpc tile --help` for detailed usage.

### `m4`
Directly send the code on the M4 through a snapshot.

```bash
img2cpc image.png m4
```

## Options

### Video Mode
- `-m, --mode <MODE>` - Screen mode of the image to convert [default: 0]
  - `0` - 160×200, 16 colors
  - `1` - 320×200, 4 colors
  - `2` - 640×200, 2 colors

### Screen Configuration
- `--fullscreen` - Specify a full screen displayed using 2 non consecutive banks
- `--overscan` - Specify an overscan screen (CRTC meaning)
- `--standard` - Specify a standard screen manipulation

### Image Processing
- `--crop` - Crop the picture if it is too large according to the destination
- `-s, --skipoddpixels` - Skip odd pixels when reading the image (useful when the picture is mode 0 with duplicated pixels)
- `--columnstart <PIXEL_COLUMN_START>` - Number of pixel columns to skip on the left
- `--columnskept <PIXEL_COLUMNS_KEPT>` - Number of pixel columns to keep
- `--linestart <PIXEL_LINE_START>` - Number of pixel lines to skip
- `--lineskept <PIXEL_LINES_KEPT>` - Number of pixel lines to keep

### Palette Control (Gate Array)
- `--pal <OCP_PAL>` - OCP PAL file. The first palette among 12 is used
- `--pens <PENS>` - Separated list of ink number. Use ',' as a separator
- `--pen0` to `--pen15 <PEN>` - Ink number for each pen (0-15)
- `--pen16 <PEN16>` - Ink number of the pen 16 (border)
- `--unlock-pens` - When some pens are manually provided, allows to also use the other ones by automatically assigning them missing inks. By default, this is forbidden
- `--missing-pen <MISSING_PEN>` - Pen to use when the byte is too small

### Amstrad Plus palettes

The Gate Array offers 27 fixed inks; the Plus's ASIC offers 12 bits of RGB - 4096
colours. A palette is one or the other, never a mixture, so every option below
conflicts with every `--penN`/`--pens`/`--pal`/`--ga-pal`.

- `--plus` - Target the Amstrad Plus: build the palette out of ASIC colours taken
  from the image itself, rather than quantising it to the 27 inks
- `--colb0` to `--colb16 <COLOUR>` - ASIC colour for one pen, written either packed
  (`4A5`, `0x4A5`) or as `R,G,B` components 0-15 (`4,10,5`). Both spellings are the
  same colour. As with `--penN`, naming some pens locks the palette unless
  `--unlock-pens` is also given
- `--kit <FILE>` - Load a 32-byte `.kit` palette file (two bytes per colour:
  `RRRRBBBB`, then `0000GGGG`)

An Amstrad Plus palette cannot travel through a snapshot's Gate Array registers,
so the display code generated for `sna`/`dsk`/`exec` installs it itself: it
unlocks the ASIC, copies the 32 bytes to `&6400`, and locks it again.

A snapshot built this way also announces the machine it needs - CRTC type 3 (the
6845 inside the ASIC) and CPC type 4 (6128 Plus). Those fields only exist from
version 3 of the snapshot format, so a Plus snapshot is written as V3 where a CPC
one stays V2.

```bash
# Let the converter pick 16 twelve-bit colours from the image
img2cpc --mode 0 --plus artwork.png sprite -o artwork.spr --kit artwork.kit

# Or state them
img2cpc --mode 0 --colb0 4,10,5 --colb1 0xF0F --unlock-pens artwork.png sna PLUS.SNA

# Or reuse a palette made elsewhere
img2cpc --mode 0 --kit artwork.kit artwork.png sprite -o artwork.spr
```

### Other Options
- `-h, --help` - Print help
- `-V, --version` - Print version

## CPC Video Modes

| Mode | Resolution | Colors | Bytes/Line | Typical Use |
|------|-----------|--------|------------|-------------|
| 0 | 160×200 | 16 | 80 | Colorful graphics |
| 1 | 320×200 | 4 | 80 | Standard graphics |
| 2 | 640×200 | 2 | 80 | High resolution text |

## OCP Palette Format

OCP palette files (.PAL) contain up to 12 palettes of 16 colors each. The tools use the first palette by default.

## Exit Status

- `0` - Success
- Non-zero - Error occurred

## See Also

- [Examples](examples.md) - Usage examples and workflows
- [Index](index.md) - Tool overview

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

### Palette Control
- `--pal <OCP_PAL>` - OCP PAL file. The first palette among 12 is used
- `--pens <PENS>` - Separated list of ink number. Use ',' as a separator
- `--pen0` to `--pen15 <PEN>` - Ink number for each pen (0-15)
- `--pen16 <PEN16>` - Ink number of the pen 16 (border)
- `--unlock-pens` - When some pens are manually provided, allows to also use the other ones by automatically assigning them missing inks. By default, this is forbidden
- `--missing-pen <MISSING_PEN>` - Pen to use when the byte is too small

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

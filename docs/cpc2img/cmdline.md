# cpc2img Command Line Reference

## Synopsis

```bash
cpc2img [OPTIONS] <INPUT> <OUTPUT> [COMMAND]
```

## Description

Generates PNG images from CPC screen files, palette files, or sprite data.

## Arguments

- `<INPUT>` - File to read. Can be a .scr, a .pal
- `<OUTPUT>` - Output PNG filename

## Commands

### `palette`
Load an OCP palette file.

```bash
cpc2img colors.pal output.png palette
```

### `sprite`
Load from a linear sprite data.

```bash
cpc2img PLAYER.SPR sprite.png sprite
```

### `screen`
Load from a 16kb screen data.

```bash
cpc2img SCREEN.SCR output.png screen
```

## Options

### Video Mode
- `-m, --mode <MODE>` - Screen mode of the image to convert [default: 0]
  - `0` - 160×200, 16 colors
  - `1` - 320×200, 4 colors
  - `2` - 640×200, 2 colors
- `--mode0ratio` - Horizontally double the pixels (for mode 0 aspect ratio correction)

### Palette Control
- `--pal <OCP_PAL>` - OCP PAL file. The first palette among 12 is used
- `--pens <PENS>` - Separated list of ink number. Use ',' as a separator
- `--pen0` to `--pen15 <PEN>` - Ink number for each pen (0-15)
- `--pen16 <PEN16>` - Ink number of the pen 16 (border)
- `--unlock-pens` - When some pens are manually provided, allows to also use the other ones by automatically assigning them missing inks

### Other Options
- `-h, --help` - Print help
- `-V, --version` - Print version

## CPC Video Modes

| Mode | Resolution | Colors | Bytes/Line | Typical Use |
|------|-----------|--------|------------|-------------|
| 0 | 160×200 | 16 | 80 | Colorful graphics |
| 1 | 320×200 | 4 | 80 | Standard graphics |
| 2 | 640×200 | 2 | 80 | High resolution text |

## Exit Status

- `0` - Success
- Non-zero - Error occurred

## See Also

- [Examples](examples.md) - Usage examples and workflows
- [Index](index.md) - Tool overview

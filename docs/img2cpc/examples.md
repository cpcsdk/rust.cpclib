# img2cpc Examples

!!! info "Command Reference"
    For current syntax: `img2cpc --help`
    Complete reference: [CLI Help Reference](../CLI_HELP_REFERENCE.md)

## Basic Usage

Convert PNG/JPEG images to Amstrad CPC formats.

### Create Snapshot
```bash
img2cpc image.png sna output.sna
```

### Create Disk Image
```bash
img2cpc image.png dsk output.dsk
```

### Create Screen File
```bash
img2cpc image.png scr --output output.scr
```

### Convert a Real Photo (True-Color Conversion)

By default `img2cpc` expects the source to already match the target
resolution and colors. Give it `--dither`, `--resize-filter`, `--colors`, or
`--out-width`/`--out-height` (on `sprite`/`tile`) instead, and it will resize
an arbitrary photo and dither it into an automatically chosen CPC palette.
```bash
img2cpc --mode 0 --dither floyd-steinberg photo.png scr --output output.scr
```

For all options and subcommands: `img2cpc --help`

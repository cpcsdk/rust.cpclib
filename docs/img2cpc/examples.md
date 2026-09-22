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

A runnable, self-contained demo of this - `cpclib-imgconverter/examples/true_color_research_images.sh` in the repo - converts five images classically used in image-processing research (Cameraman, the "Astronaut" portrait, a cat, a colorwheel, a coffee cup - sourced from scikit-image's own bundled test data, no network fetch needed) through several `--dither` algorithms and renders the results back to PNG with `cpc2img` for visual inspection.

For all options and subcommands: `img2cpc --help`

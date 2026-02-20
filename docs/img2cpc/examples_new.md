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

For all options and subcommands: `img2cpc --help`

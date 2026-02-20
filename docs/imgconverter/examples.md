# Image Converter Examples

!!! info "For Current Syntax"
    These tools have complex CLIs. For accurate usage:
    ```bash
    img2cpc --help
    cpc2img --help
    fade --help
    ```
    Complete reference: [CLI Help Reference](../CLI_HELP_REFERENCE.md)

## img2cpc - Convert Images to CPC Format

```bash
img2cpc image.png sna output.sna
img2cpc image.png dsk output.dsk
img2cpc image.png scr --output output.scr
```

## cpc2img - Convert CPC Files to Images

```bash
cpc2img screen.bin output.png screen
cpc2img sprite.spr output.png sprite --width 16
cpc2img colors.pal output.png palette
```

## fade - Generate Color Fades

Refer to `fade --help` for usage.

For all options, use `--help` with each command.

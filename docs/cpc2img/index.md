# cpc2img - CPC to Image Converter

`cpc2img` converts CPC screen files (.scr), palette files (.pal), and sprite data to PNG format for viewing and editing on modern systems.

## Features

- **Screen conversion**: Convert 16KB CPC screen dumps to PNG
- **Palette support**: Load OCP palette files or specify manual pen assignments
- **Sprite extraction**: Convert sprite data to PNG images
- **All video modes**: Mode 0, Mode 1, Mode 2
- **Aspect ratio correction**: Optional pixel doubling for mode 0

## Quick Start

```bash
# Convert SCR file to PNG
cpc2img TITLE.SCR output.png --mode 0

# With OCP palette
cpc2img SCREEN.SCR image.png --pal colors.pal

# Mode 0 with correct aspect ratio
cpc2img MODE0.SCR wide.png --mode 0 --mode0ratio
```

## Input Formats

- **SCR files** - 16KB screen memory dumps
- **PAL files** - OCP palette files
- **Binary sprite data** - Linear sprite data

## See Also

## Integration with BndBuild

cpc2img is available as a standalone `cpc2img` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `cpc2img` command. See [BndBuild Commands](../bndbuild/commands.md#image-management-cpc-to-image-cpc2img) for integration details.

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all options
- [Examples](examples.md) - Usage examples and workflows

# img2cpc - Image to CPC Converter

`img2cpc` converts modern image formats (PNG, BMP, JPEG, etc.) to various Amstrad CPC formats including snapshots, DSK files, screen files, sprites, and tiles. This is the primary tool for importing graphics into CPC projects.

## Features

- **Multiple output formats**: Generate snapshots, DSK files, executable binaries, sprites, tiles
- **Full screen support**: Standard screens, overscan, fullscreen (2 banks)
- **All video modes**: Mode 0 (160x200, 16 colors), Mode 1 (320x200, 4 colors), Mode 2 (640x200, 2 colors)
- **Palette control**: Manual pen assignment, OCP palette files, automatic ink allocation
- **Image manipulation**: Cropping, column/line selection, odd pixel skipping
- **Direct M4 upload**: Send converted images directly to M4 board

## Quick Start

```bash
# Convert PNG to CPC snapshot
img2cpc title.png sna TITLE.SNA

# Convert PNG to executable in DSK
img2cpc logo.png dsk DEMO.DSK

# Send directly to M4
img2cpc screen.png m4
```

## Output Formats

- **sna** - Snapshot file (ready to run in emulator)
- **dsk** - DSK file with executable  
- **scr** - OCP screen file (16KB memory dump)
- **exec** - Executable binary  
- **sprite** - Sprite data file
- **tile** - Tile map file
- **m4** - Direct upload to M4 board

## See Also

## Integration with BndBuild

img2cpc is available as a standalone `img2cpc` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `img2cpc` or `imgconverter` command aliases. See [BndBuild Commands](../bndbuild/commands.md#image-management-benediction-transfer-tool-img2cpcimgconverter) for integration details.

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all options
- [Examples](examples.md) - Usage examples and workflows

# CPRCLI - CPR Cartridge Tool

CPRCLI is a command-line tool for analyzing and manipulating Amstrad CPC Plus CPR (Cartridge) files.

## Features

- Display CPR cartridge information
- Extract specific banks from cartridges
- Compare CPR files
- Dump cartridge memory contents
- Bank selection and filtering

## Quick Start

```bash
# Show cartridge information
cprcli --cpr1 game.cpr --info

# Dump specific banks
cprcli --cpr1 game.cpr --bank 0,1,2 --dump

# Compare two cartridges
cprcli --cpr1 version1.cpr --cpr2 version2.cpr --info
```

## What are CPR Files?

CPR files are ROM cartridge images for the Amstrad CPC Plus range (464+, 6128+, GX4000). Cartridges can contain up to 32 banks of 16KB each (512KB total).

## See Also

## Integration with BndBuild

CPRCLI is available as a standalone `cpr` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `cpr` command. See [BndBuild Commands](../bndbuild/commands.md#cartridge-management-cpr-analysis-cpr) for integration details.

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all options
- [Examples](examples.md) - Usage examples and workflows

# Hideur - AMSDOS Header Manager

!!! info "Command Reference"
    For current usage: `hideur --help`  
    Complete documentation: [CLI Help Reference](../CLI_HELP_REFERENCE.md)

Hideur is a tool to manipulate AMSDOS headers on files. It can display header information or add/modify AMSDOS headers on binary files.

## Overview

AMSDOS headers contain metadata about files on Amstrad CPC disks, including:
- File type (Basic, Protected Basic, or Binary)
- Load address (for binary files)
- Execution address (for binary files)
- User number (0-15)
- Length and checksum

## Quick Start

Display header information:
```bash
hideur input.bin --info
```

Add a binary header:
```bash
hideur input.bin -o output.bin -t binary -l 0x4000 -x 0x4000
```

## Common Use Cases

### Display File Information
```bash
hideur myfile.bin --info
```

### Create Binary File with Header
```bash
hideur code.bin -o code_with_header.bin -t binary -l 0x8000 -x 0x8000
```

### Create Basic File with Header
```bash
hideur program.bas -o program.bas -t basic
```

## See Also

## Integration with BndBuild

Hideur is available as a standalone `hideur` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `hideur` command. See [BndBuild Commands](../bndbuild/commands.md#file-management-amsdos-header-management-hideur) for integration details.

## See Also

- [Command Line Reference](cmdline.md)
- [Examples](examples.md)
- [Catalog Tool](../catalog/) - For working with DSK files

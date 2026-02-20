# XferTool - M4 Board Management Tool

XferTool is a command-line utility for managing files and operations on the M4 Board, an SD card interface for Amstrad CPC computers. It allows uploading files, executing programs, browsing directories, and controlling the M4 and CPC.

## Features

- Upload files to M4 Board SD card
- Execute files (binaries, snapshots) on CPC
- Browse M4 file system (ls, pwd, cd)
- Reboot M4 Board or CPC
- Automatic snapshot format conversion (V3 → V2)
- Interactive session mode

## What is M4 Board?

The M4 Board is an SD card interface for Amstrad CPC computers that allows:
- Loading programs from SD card
- Storing files on removable media
- Network connectivity for file transfers
- Enhanced CPC capabilities

## Quick Start

```bash
# Upload a file to M4
cpclib-xfertool -p game.dsk

# Upload and run a snapshot
cpclib-xfertool -y game.sna

# Execute a file on CPC
cpclib-xfertool -x loader.bin

# Browse M4 files
cpclib-xfertool --ls
```

## See Also

## Integration with BndBuild

XferTool is available as a standalone `cpclib-xfertool` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `xfer`, `cpcwifi`, or `m4` command aliases. See [BndBuild Commands](../bndbuild/commands.md#transfer-m4-support-xfercpcwifim4) for integration details.

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all options
- [Examples](examples.md) - Usage examples and workflows
- M4 Board documentation

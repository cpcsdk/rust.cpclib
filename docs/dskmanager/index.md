# DSKManager - DSK Disc Image Manager

## Overview

DSKManager is a command-line tool for managing Amstrad CPC DSK disc images. It provides low-level operations for creating, formatting, and manipulating DSK files.

## Features

- **Format DSK Images**: Create new formatted disc images
- **File Management**: Add and extract files from DSK images
- **Catalog Operations**: List disc contents
- **Track/Sector Access**: Low-level disc manipulation
- **Multiple Formats**: Support for various DSK formats and geometries

## Installation

DSKManager is part of the cpclib-disc crate.

### Download Pre-built Binary

Coming soon - check the [releases page](https://github.com/cpcsdk/rust.cpclib/releases).

### Build from Source

```bash
cargo install --path cpclib-disc --features dskmanager
```

## Usage in BndBuild

Within bndbuild, use the `dsk` or `disc` command:

```yaml
- tgt: mydisk.dsk
  cmd: dsk format mydisk.dsk
```

For complete bndbuild integration, see [BndBuild Commands](../bndbuild/commands.md#disc-management-benediction-dsk-manager-dskdisc).

## Quick Start

```bash
# Format a new DSK
dskmanager format output.dsk

# Add files to DSK
dskmanager add output.dsk file1.bin file2.bin

# List DSK contents
dskmanager catalog output.dsk
```

## Related Tools

- [Catalog](../catalog/) - View and create CatArt disc catalogs
- [Hideur](../hideur/) - Manage AMSDOS headers for proper CPC file compatibility
- [BndBuild](../bndbuild/) - Build system that integrates dskmanager

## Documentation

- [Command Line Reference](cmdline.md)
- [Examples](examples.md)

## Source Code

Part of [cpclib-disc](https://github.com/cpcsdk/rust.cpclib/tree/master/cpclib-disc) in the rust.cpclib repository.

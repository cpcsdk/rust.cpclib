# CPC Runner - Emulator-Agnostic Launcher

!!! info "Command Reference"
    For current usage: `bndbuild --help cpc`  
    Complete documentation: [CLI Help Reference](../CLI_HELP_REFERENCE.md)

The CPC runner provides a unified interface to launch Amstrad CPC programs across multiple emulators without changing your workflow.

## Overview

Instead of learning different command-line syntaxes for each emulator, use a single consistent interface:
- Automatically downloads and manages emulator binaries
- Supports multiple emulators: ACE, WinAPE, CPCEC, AMSpiriT, SugarboxV2
- Handles disk images, snapshots, and Albireo folders
- Works seamlessly in build scripts

## Quick Start

Run a snapshot:
```bash
bndbuild --direct -- cpc run --snapshot demo.sna
```

Run with a disk:
```bash
bndbuild --direct -- cpc run -a game.dsk -e ace
```

## Supported Emulators

## Integration with BndBuild

CPC Runner is available as a standalone `cpcrunner` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `cpc`, `emu`, `emuctrl`, or `emucontrol` command aliases. See [BndBuild Commands](../bndbuild/commands.md#emulator-emulator-agnostic-emulation-cpcemuemucetrlemucontrol) for integration details.

## Supported Emulators

- **ACE** (default) - Cross-platform, feature-rich
- **WinAPE** - Windows, accurate emulation
- **CPCEC** - Cross-platform, high compatibility
- **AMSpiriT** - Windows, hardware-accurate
- **SugarboxV2** - Cross-platform, modern

## Common Use Cases

### Launch with Disk Image
```bash
bndbuild --direct -- cpc run -a mydisk.dsk
```

### Choose Specific Emulator
```bash
bndbuild --direct -- cpc run --snapshot game.sna -e winape
```

### Set Memory Configuration
```bash
bndbuild --direct -- cpc run -a disk.dsk -m 128
```

### Keep Emulator Open
```bash
bndbuild --direct -- cpc run -a disk.dsk -k
```

## Integration with bndbuild

In your `bndbuild.yml`:

```yaml
- tgt: run
  cmd: cpc run --snapshot demo.sna
  phony: true
```

## See Also

- [Command Line Reference](cmdline.md)
- [Examples](examples.md)
- [BNDBUILD](../bndbuild/) - Build system integration

# Snapshot Tool

The `snapshot` tool is a command-line utility for creating, inspecting, and manipulating Amstrad CPC snapshot (SNA) files.

## Installation

The snapshot tool is built as part of the cpclib toolchain:

```bash
cargo install cpclib-snacli
```

## Overview

SNA (Snapshot) files preserve the complete state of an Amstrad CPC at a specific moment, including:

- CPU registers (Z80 state)
- Memory contents (RAM and ROM)
- Gate Array configuration (screen mode, colors, ROM selection)
- CRTC (Cathode Ray Tube Controller) registers

The snapshot tool allows you to:

- Create snapshots from raw memory dumps
- Inspect snapshot contents
- Modify snapshot properties
- Convert between different snapshot formats

## Quick Start

Create a snapshot with a memory image:

```bash
snapshot -l code.bin 0x4000 -- output.sna
```

Inspect a snapshot:

```bash
snapshot --info -i game.sna
```

## Documentation Sections

## Integration with BndBuild

Snapshot is available as a standalone `snapshot` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `sna` or `snapshot` command aliases. See [BndBuild Commands](../bndbuild/commands.md#file-management-snasnapshot-management-snasnpashot) for integration details.

## Documentation Sections

- [Command-Line Reference](cmdline.md) - Complete CLI documentation
- [Examples](examples.md) - Common use cases and recipes

## SNA File Format

The standard Amstrad CPC snapshot format stores:

- 64KB or 128KB of RAM depending on CPC model
- Complete Z80 register state
- Gate Array and CRTC configuration
- Optional chunks for extended information

## Related Tools

- [img2cpc](../img2cpc/index.md) - Convert images and save directly to SNA files
- cpclib-disc - Create DSK files that can be loaded in snapshots

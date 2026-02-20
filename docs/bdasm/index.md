# BDASM - Z80 Disassembler


BDASM is a Z80 disassembler for Amstrad CPC that converts binary machine code back into assembly language.

## Features

- Disassemble Z80 machine code into assembly syntax
- Support for various input formats (binary files, SNA snapshots, etc.)
- Configurable output options
- Label injection from symbol tables
- Memory range selection

## Quick Start

```bash
# Disassemble a binary file
bdasm input.bin

# Disassemble with custom output
bdasm -o output.asm input.bin
```

For advanced options, see `bdasm --help`.

## See Also

## Integration with BndBuild

BDASM is available as a standalone `bdasm` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `bdasm` or `dz80` command aliases. See [BndBuild Commands](../bndbuild/commands.md#disassembler-bdasm-bdasmdz80) for integration details.

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all command-line options
- [Examples](examples.md) - Usage examples and common workflows
- [BASM](../basm/index.md) - The companion assembler tool

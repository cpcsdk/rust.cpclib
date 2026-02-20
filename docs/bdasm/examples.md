# BDASM Examples

## Basic Disassembly

Disassemble a binary file to stdout:

```bash
bdasm game.bin
```

Save output to a file:

```bash
bdasm game.bin -o game.asm
```

## Advanced Usage

For information on additional options such as address ranges, symbol tables, and format handling, run:

```bash
bdasm --help
```

## See Also

- [Command Line Reference](cmdline.md) - Complete option reference
- [BASM Documentation](../basm/index.md) - For assembling the disassembled code

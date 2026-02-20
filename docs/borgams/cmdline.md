# Borgams Command Line Reference

!!! warning "Development Status"
    Borgams is currently under development and not yet usable. This documentation describes the intended command-line interface once the tool is complete.

## Synopsis

```bash
cpclib-borgams --input <INPUT> --output <OUTPUT>
```

## Description

Borgams converts Orgams binary format files to ASCII text format. The tool takes an Orgams binary file (preprocessed assembly) and outputs a human-readable ASCII representation of the assembly source code.

## Required Arguments

### `-i, --input <INPUT>`
Input Orgams file to convert. This should be an Orgams binary format file with preprocessed assembly code.

Example:
```bash
cpclib-borgams --input myprogram.org --output myprogram.asm
```

### `-o, --output <OUTPUT>`
Output ASCII file. The converted assembly source will be written to this file in plain text format.

Example:
```bash
cpclib-borgams --input binary.org --output source.asm
```

## Options

### `-h, --help`
Print help information showing command usage and available options.

### `-V, --version`
Print version information.

## Input Format

Borgams expects input files in the Orgams binary format:
- Preprocessed Z80 assembly
- String table for labels and symbols
- Encoded instructions and directives
- Macro definitions

## Output Format

The output ASCII file contains:
- Z80 assembly mnemonics
- Labels and symbols
- Assembler directives
- Comments (if preserved in binary)
- Macro definitions and uses

## Exit Status

- `0` - Success
- `1` - Error (file not found, invalid format, I/O error)

## Examples

See [Examples](examples.md) for detailed usage patterns.

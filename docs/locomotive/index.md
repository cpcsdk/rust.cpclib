# Locomotive - Amstrad CPC BASIC Tool

Locomotive is a command-line tool for converting between ASCII text and Amstrad CPC Locomotive BASIC binary formats. It enables you to write BASIC programs in a text editor and convert them to the tokenized format used by the CPC.

## Features

- **Encode**: Convert ASCII text to tokenized BASIC binary format
- **Decode**: Convert tokenized BASIC binary to readable ASCII text  
- **Amsdos Headers**: Optionally add Amsdos headers to generated files
- **Stdout Support**: Decode can print to stdout for easy piping
- **Token Validation**: Parser validates both tokens and command argument coherency

## Quick Start

```bash
# Convert BASIC text to binary format
locomotive encode -i program.txt -o program.bas

# Convert binary BASIC to text
locomotive decode -i program.bas -o program.txt

# Decode to stdout
locomotive decode -i program.bas

# Encode with Amsdos header
locomotive encode -i program.txt -o program.bas --header
```

## Use Cases

- **Development**: Write BASIC programs in a text editor with syntax highlighting
- **Version Control**: Store BASIC programs as readable text in Git
- **Code Analysis**: Convert binary BASIC files to inspect their content
- **Cross-Platform Development**: Edit on modern systems, convert for CPC

## Integration

**For DSK operations**, use the **dsk** tool:
```bash
# Add BASIC file to DSK
locomotive encode -i game.txt -o game.bas
dsk add disk.dsk game.bas

# Extract BASIC from DSK and decode
dsk extract disk.dsk GAME.BAS -o game.bas
locomotive decode -i game.bas -o game.txt
```

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of commands and options
- [Examples](examples.md) - Usage examples and workflows
- [BASIC Format](format.md) - Information about CPC BASIC file format
- DSK Tool - For DSK disk image manipulation (use `dsk` command in bndbuild or CLI)

**Author**: Krusty/Benediction 2019

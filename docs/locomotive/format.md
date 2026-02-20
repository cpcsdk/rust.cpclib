# CPC BASIC File Format

This document describes the Locomotive BASIC file format used by Amstrad CPC computers.

## Overview

Locomotive BASIC files can be stored in two formats:

1. **Text Format**: Human-readable ASCII text
2. **Binary Format**: Tokenized binary format (native CPC format)

## Binary Format Structure

### File Header
The binary format begins with a file header:

- Bytes 0-1: Length of the BASIC program (little-endian 16-bit)
- Byte 2: Start of BASIC content

### Line Structure
Each BASIC line in binary format has the following structure:

- Bytes 0-1: Line length (little-endian 16-bit)
- Bytes 2-3: Line number (little-endian 16-bit)
- Bytes 4+: Tokenized BASIC commands
- Final byte: 0x00 (line terminator)

### Tokens
BASIC keywords are stored as single-byte tokens:

- `0x80` - AFTER
- `0x81` - AUTO
- `0x82` - BORDER
- ...
- `0xBF` - PRINT
- `0xC0` - RAD
- ...

Special tokens:
- `0x00` - End of line
- `0x01-0x1F` - Control characters
- `0x20-0x7F` - ASCII characters
- `0x80-0xFF` - BASIC keywords

### Strings
String literals in the binary format:
- Start with opening quote: `0x22` (")
- Contain raw character bytes
- End with closing quote: `0x22` (")

### Numbers
Numbers can be stored as:
- Text representation (digits as ASCII)
- Binary representation (special encoding for performance)

## Text Format

The text format is straightforward:
- One line per BASIC line
- Line number followed by space
- BASIC commands in uppercase
- Strings in double quotes

Example:
```basic
10 MODE 1
20 PRINT "HELLO"
30 FOR I=1 TO 10
40 PRINT I
50 NEXT I
```

## Conversion Notes

When converting between formats, locomotive handles:

- **Tokenization**: Keywords (PRINT, IF, etc.) → single byte tokens
- **Detokenization**: Tokens → readable keywords
- **String handling**: Proper quote management
- **Line endings**: CPC-style line terminators
- **Whitespace**: Preserved where significant

## Special Cases

### REM (Comments)
REM statements are not fully tokenized - the comment text remains as-is after the REM token.

### DATA Statements
DATA contents are stored as text, not tokenized.

### Line Numbers
- Valid range: 1-65529
- Typically in increments of 10
- Must be in ascending order

## File Size Limits

- Maximum program size: ~48 KB (depends on available CPC memory)
- Line length: Limited by CPC BASIC constraints
- Number of lines: Limited by memory

## Compatibility

The locomotive tool handles tokenized BASIC files compatible with:
- Amstrad CPC 464, 664, 6128
- Locomotive BASIC 1.0 and 1.1
- Standard tokenized BASIC format (with or without Amsdos headers)

**Note**: For DSK disk operations, use the **dsk** tool in combination with locomotive.

## See Also

- [Command Line Reference](cmdline.md)
- [Examples](examples.md)
- DSK Tool - For DSK operations (use `dsk` command in bndbuild or CLI)
- Amstrad CPC User Manual for BASIC syntax

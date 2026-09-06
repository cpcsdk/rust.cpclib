# Orgams Binary Format Specification

## Overview

Orgams is a binary file format used for storing z80 assembler source code, particularly in the context of Amstrad CPC demo tooling. It is designed to compactly represent assembler instructions, labels, expressions, and control structures while maintaining fidelity to the original source code formatting and semantics.

The format is used by tools like the Orgams assembler to save and load assembler projects. This document describes the binary structure, decoding process, and key components based on the implementation in `binary_decoder.rs`.

## File Structure

An Orgams file consists of the following high-level components:

1. **Header**: Contains metadata about the file, including a dynamic-sized header with version and size information.
2. **Chunks**: The main content is divided into chunks, primarily:
   - `SRCc` chunks: Contain the source code instructions and statements.
   - `LBLs` chunks: Contain label definitions.
   - `ChCk` chunks: Contain checksums for validation.
3. **Terminator**: A marker indicating the end of chunks.

The file uses little-endian byte order throughout.

## Header

The header starts with the magic bytes `ORGA` (0x4F 0x52 0x47 0x41).

- **Magic**: 4 bytes: `0x4F 0x52 0x47 0x41` ("ORGA")
- **Version**: 1 byte (currently 0x02)
- **Header Size**: 1 byte, indicating the size of additional header data
- **Header Content**: Variable length data of the specified size

After the header, the file proceeds to chunks.

## Chunks

Chunks are the core data units. Each chunk starts with a 4-byte identifier followed by null separators:

- `SRCc` (0x63 0x52 0x43 0x53): Source code chunk, followed by 0x00
- `LBLs` (0x73 0x4C 0x42 0x4C): Labels chunk, followed by 0x00  
- `ChCk` (0x6B 0x43 0x68 0x43): Checksum chunk

### Source Code Chunk (SRCc)

This chunk contains the assembled instructions, statements, and control structures. It is parsed sequentially, with each element encoded as a sequence of bytes.

#### Encoding Principles

- **Markers**: Special bytes indicate the type of content:
  - `0x7F` (MARKER_ESCAPE): Indicates an escaped command or special statement.
  - `0x4A` (NL): Newline.
  - Other bytes represent raw data or opcodes.

#### Direct Markers (no MARKER_ESCAPE needed)

These bytes are used directly without the 0x7F escape prefix:

- `0x4A`: Newline (MARKER_NEWLINE)
- `0x43`: Comment (MARKER_COMMENT) - followed by SizedString
- `0x49`: Indentation (MARKER_INDENT) - followed by space count byte
- `0x64`: Assignment (MARKER_ASSIGN) - followed by label and expression
- `0x42`: Byte directive (MARKER_BYTE) - followed by expression list
- `0x57`: Word directive (MARKER_WORD) - followed by expression list
- `0x6D`: Macro definition (MARKER_MACRO_DEF) - followed by macro data
- `0x60-0xDF`: Label references (MARKER_LABEL_ADDR/MARKER_LOCAL_LABEL) - followed by label encoding

Instructions with opcodes matching these markers are prefixed with 0x7F to avoid conflicts.

#### Command Codes (after MARKER_ESCAPE)

- `0x01`: ASIS (raw string)
- `0x09`: IF
- `0x0A`: ELSE
- `0x0B`: END
- `0x0C`: BRK
- `0x0D`: RESTORE
- `0x0E`: FILL
- `0x0F`: ENT
- `0x10`: ORG
- `0x11`: ORG2
- `0x12`: SKIP
- `0x13`: IMPORT
- `0x14`: ENDM
- `0x15`: STORE_PC_INSTR
- `0x16`: STORE_PC_LINE
- `0x17`: REPEAT
- `0x18`: MACRO_USE
- `0x19`: END_BIS

#### Instruction Encoding

Instructions are stored as their raw z80 opcodes. For example:

- `PUSH AF` is `0xF5`
- `LD A, (HL)` is `0x7E`

Prefixed instructions use different prefix bytes depending on the operation:

- `0xDD`: IX register prefix (e.g., `PUSH IX` → `DD F5`)
- `0xFD`: IY register prefix (e.g., `PUSH IY` → `FD F5`)  
- `0xDF`: IX indirect addressing prefix (e.g., `LD A,(IX+5)` → `DF` + `displacement` + `7E`)
- `0xFF`: IY indirect addressing prefix (e.g., `LD A,(IY+5)` → `FF` + `displacement` + `7E`)
- `0xED`: Extended instruction prefix (e.g., `LDIR` → `ED B0`)
- `0xCB`: Bit operation prefix (e.g., `BIT 0,A` → `CB 47`)

For indirect addressing with `0xDF`/`0xFF`, the signed displacement byte (-128 to +127) immediately follows the prefix byte, before the opcode.

Operands are encoded as expressions following the opcode.

#### Strings

Orgams uses two string types for different purposes:

**SizedString**: Length-prefixed string

- Format: `length_byte` + `length` bytes of content
- Used for comments, raw strings, import paths
- Content uses Windows-1252 encoding

**Bit7OnString**: Bit-7 terminated string

- Format: Sequence of bytes ending with a byte where bit 7 is set
- When stored, bit 7 is cleared from the last byte
- Used for labels and identifiers in the string table
- Terminator detection: Last byte has bit 7 = 1 during encoding

#### Expressions

Expressions are binary-encoded mathematical operations stored in a compact form. There are two main types:

**SizedExpression**: `size_byte` + `size` bytes of expression data  
**UnsizedExpression**: Variable length expression data

Expressions consist of a sequence of ExpressionMembers:

##### Short Decimal Values (0-31)

- `0x00-0x1F`: Direct encoding of small decimal constants

##### Operators

- `0x2B`: Addition (+)
- `0x2D`: Subtraction (-) or Unary minus (after value)
- `0x2A`: Multiplication (*)
- `0x2F`: Division (/)
- `0x25`: Modulo (%)
- `0x26`: Bitwise AND (&)
- `0x7C`: Bitwise OR (|)
- `0x5E`: Bitwise XOR (^)
- `0x3D`: Equal (=)
- `0x3C`: Less than (<)
- `0x3E`: Greater than (>)
- `0x28`: Left parenthesis
- `0x29`: Right parenthesis
- `0x20`: Space

##### Numeric Values

- `0x30`: Decimal 8-bit - followed by 1 byte value
- `0x31`: Decimal 16-bit - followed by 2 bytes (little-endian)
- `0x32-0x33`: Decimal custom - followed by length byte + length bytes
- `0x34`: Hexadecimal 8-bit - followed by 1 byte value  
- `0x35`: Hexadecimal 16-bit - followed by 2 bytes (little-endian)
- `0x36-0x37`: Hexadecimal custom - followed by length byte + length bytes
- `0x38`: Binary 8-bit - followed by 1 byte value
- `0x39`: Binary 16-bit - followed by 2 bytes (little-endian)
- `0x3A-0x3B`: Binary custom - followed by length byte + length bytes

##### Labels and References

- `0x60-0xFF`: Label references (encoded as label indices)
- `0x24`: Dollar ($) symbol
- `0x44`: Double dollar ($$) symbol

##### Special Elements

- `0x41`: Iter1 (must be accompanied by Iter2 and Iter3)
- `0x43`: Iter3 (must be accompanied by Iter1 and Iter2)

##### Multi-term Expressions

- `0x42`: Begin multi-term expression (for parentheses and complex expressions)
- `0x45`: End multi-term expression

##### Strings in Expressions

- `0x53`: String value - followed by SizedString

Multi-term expressions are enclosed between `0x42` (begin) and `0x45` (end) markers.

##### Expression Examples

- `42`: `0x30 0x2A` (decimal 8-bit: 42)
- `0x100`: `0x35 0x00 0x01` (hexadecimal 16-bit: 0x0100)
- `label + 1`: `label_index 0x2B 0x01` (label ref + short decimal 1)
- `(a + b) * 2`: `0x42 label_a 0x2B label_b 0x45 0x2A 0x02` (multi-term expression)

### Labels Chunk (LBLs)

Contains a list of label names using Bit7OnString encoding:

- Starts with a version byte (0x02).
- Followed by a sequence of Bit7OnString entries, each terminated by a null byte (0x00).
- Each Bit7OnString has its last character with bit 7 set (cleared when stored).

Labels are referenced by index in expressions and statements.

### Checksum Chunk (ChCk)

Contains a checksum for validation:

- 4 bytes: CRC or similar checksum of the file content.

## Decoding Process

1. **Parse Header**: Read magic, header size, and skip to chunks.
2. **Parse Chunks**: Loop through chunks until terminator (0x00 after chunks).
3. **Decode Source**: For SRCc, parse byte-by-byte, handling markers and commands.
4. **Resolve Labels**: Use LBLs to map indices to names.
5. **Validate**: Check checksum if present.

## Display Formatting

When reconstructing source code, Orgams uses a state-based formatter:

- **LineState Enum**: Tracks current line state (Empty, AfterLabel, AfterStatement, etc.).
- **Tabs**: TAB_INSTR = 10 spaces, TAB_COMMAND = 6 spaces.
- **Auto-format**: Adds prefixes based on state:
  - Empty: TAB_COMMAND/TAB_INSTR spaces
  - AfterLabel: TAB_COMMAND/TAB_INSTR spaces
  - AfterStatement: ":"
  - AfterRepeatBloc: TAB_COMMAND spaces
  - AfterIf: ":"
  - AfterOrg: ":"

This ensures proper indentation and colon placement in reconstructed source.

## Encoding

Text content (comments, strings) uses WINDOWS-1252 encoding.

## Examples

### Simple Instruction

```asm
PUSH AF
```

Encoded as: `0xF5 0x4A`

### Label and Instruction

```asm
my_label: ld a,1
```

Encoded with LABEL command, then instruction.

### IF Statement

```asm
IF condition
  instruction
ELSE
  other
END
```

Uses IF, ELSE, END commands with expressions.

## Implementation Notes

- The decoder in `binary_decoder.rs` handles all these elements.
- Parsing is done with Winnow combinators for robustness.
- Display reconstruction aims for exact match with original source formatting.

This format allows efficient storage and reconstruction of assembler source code for CPC demos.

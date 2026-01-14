# Orgams File Format - Investigation Progress

## Summary

Successfully reversed-engineered parts of the Orgams binary format through analysis of 100+ test files.

## Format Structure

```
[4 bytes] Magic: "ORGA"
[1 byte]  Version: 0x02
[98 bytes] Metadata: Fixed header table
[3 bytes] Section marker: "SRC"
[2-7 bytes] Section header: 0x63 0x02 [optional extra bytes]
[N bytes] Content: Encoded instructions and data
```

## Content Encoding Discovered

### Markers

**Standalone markers (no length byte):**
- `0x41` (A) - Assembly line marker
- `0x4a` (J) - NewLine

**Text markers (with length+text):**
- `0x43` (C) - Comment [length] [text...]
- `0x49` (I) - Indented [length] [text...]

**Container marker:**
- `0x64` (d) - Data section [length] [nested content...]

**Command marker:**
- `0x7f` (~) - Command prefix

### Commands (0x7f prefix)

- `0x09` - IF directive
- `0x0a` - ELSE directive
- `0x0c` - END directive
- `0x17` - IMPORT directive (+ string)
- `0x01` - Unknown (possibly ORG or text block)
- `0x14` - Unknown (possibly MACRO/ENDM)
- Others: 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x10, 0x15, 0x41, 0x45

## Current Decoder Status

✅ **Working:**
- Read/write Orgams files with perfect round-trip
- Skip section header (0x63 0x02...)
- Decode IF/ELSE/END/IMPORT directives
- Recognize Comment and NewLine markers
- Collect raw text between markers
- Parse Data sections recursively

⚠️ **Partial:**
- Text extraction (gets fragments but not complete symbols)
- Command argument parsing (some commands not fully understood)
- String references (appear as truncated text)

❌ **Missing:**
- Z80 opcode disassembly (need to use cpclib-asm::disass)
- MACRO/ENDM detection (command 0x14?)
- Symbol table reconstruction
- Complete string decoding
- Variable/label references

## Key Insights

1. **Not source code**: Orgams files contain pre-processed, partially assembled code
2. **Mixed content**: Text, opcodes, references, and directives are interleaved
3. **String references**: Text fragments suggest symbol table or string pool
4. **Binary data**: Bytes like 0xd4, 0xe4 are likely indices, not text
5. **Nested structure**: Data sections (0x64) contain recursive encoding

## Test Results

- **MACRO.I** (164 bytes): Decodes IF/ELSE/END, missing MACRO/ENDM
- **CH.I** (599 bytes): Extracts 40 elements, many truncated symbols
- **EXCEPT.O** (7838 bytes): Extracts 80 elements from 499-line source

## Next Steps

1. Implement Z80 disassembly for opcode bytes
2. Map remaining command codes (0x03-0x08, 0x10, 0x14, 0x15, etc.)
3. Handle symbol/string references properly
4. Study relationship between byte values and symbol indices
5. Compare multiple files to find patterns in encoding

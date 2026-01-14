# Orgams Binary Format - Detailed Analysis

## File Structure

```
Offset  Size  Description
------  ----  -----------
0x00    4     Magic: "ORGA"
0x04    1     Version: 0x02
0x05    98    Metadata header (partially understood)
0x67    3     Section marker: "SRC"
0x6A    2-7   Section header: starts with 0x63 0x02, variable length
0x6B+   N     Encoded content (instructions, text, commands)
END-M   M     String table (starts with "LBLs" marker)
```

## Metadata Header (98 bytes at offset 0x05)

Observed patterns across files:

| File      | Size  | Content | Table | Metadata[0-1] | Metadata[2-3] | Metadata[4-5] |
|-----------|-------|---------|-------|---------------|---------------|---------------|
| MACRO.I   | 164   | 35      | 22    | 0x0060 (96)   | 0x0001        | 0x0D01 (3329) |
| EXCEPT.O  | 7838  | 6840    | 891   | 0x0060 (96)   | 0x0001        | 0x0801 (2049) |

**Findings:**
- First word always `0x0060` (96 decimal) - possibly header size or version marker
- Second word `0x0001` - possibly flags or format version
- Third word varies - might encode file type, compression, or other metadata
- No direct size encoding found in simple word format
- Likely uses complex packing or offsets not yet understood

## Section Header (after "SRC")

Format: `0x63 0x02 [variable data]`

Examples:
- MACRO.I:  `63 02 20 6d 03 60 61`
- EXCEPT.O: `63 02 be 64 70 01 01`

The variable part might encode:
- Initial content type
- Offset to first real instruction
- Encoding flags

## Content Encoding (Main Section)

### Marker Bytes (Escape Sequences)

| Byte  | Name              | Format                  | Description                          |
|-------|-------------------|-------------------------|--------------------------------------|
| 0x40  | ec_label_adr      | Standalone or with data | Label address (also '@' ASCII)       |
| 0x43  | ec_comment        | [len][text]             | Comment line                         |
| 0x49  | ec_tab            | [len][text]             | Indented/tabbed text                 |
| 0x4A  | ec_nl             | Standalone              | Newline                              |
| 0x64  | ec_label_equ      | [len][nested]           | Label equals / Data container        |
| 0x6D  | ec_space          | [count]                 | Space repetition                     |
| 0x7F  | ec_esc            | [cmd][args...]          | Command prefix                       |
| 0xCF  | ec_byte           | [value]                 | Byte literal value (not yet handled) |
| 0xD7  | ec_word           | [lo][hi]                | Word literal value (not yet handled) |
| 0xDF  | ec_ix_ind         | [offset]                | IX indexed addressing                |
| 0xFF  | ec_iy_ind         | [offset]                | IY indexed addressing                |

**Critical Rule (from ASS.Z80):** "From $40 to $7f : themself"
- Bytes in range 0x40-0x7F can be EITHER markers OR literal ASCII
- Context determines interpretation
- Currently treating only 0x43, 0x49, 0x4A, 0x64, 0x6D, 0x7F as definite markers

### Command Encoding (0x7F prefix)

Format: `0x7F [cmd_byte] [args...]`

| Code | Name            | Args Format                              | Example                  |
|------|-----------------|------------------------------------------|--------------------------|
| 0x01 | ORG             | [1-2 bytes]                              | 7F 01 09 20              |
| 0x03 | DataRef         | [arg][string_idx?]                       |                          |
| 0x04 | ExprRef         | [arg][string_idx?]                       |                          |
| 0x05 | SymbolRef       | [arg][string_idx?]                       |                          |
| 0x06 | StringRef       | [arg][string_idx?]                       | 7F 06 01 77              |
| 0x07 | NumberRef       | [arg][string_idx?]                       |                          |
| 0x08 | LabelRef        | [arg][string_idx?]                       |                          |
| 0x09 | IF              | [condition][value?]                      | 7F 09 01 61              |
| 0x0A | ELSE            | No args                                  | 7F 0A                    |
| 0x0C | END             | No args                                  | 7F 0C                    |
| 0x10 | MacroDef        | [metadata?]                              | 7F 10                    |
| 0x14 | CodeBlockStart  | [metadata?]                              | 7F 14                    |
| 0x15 | CodeBlockEnd    | [metadata?]                              | 7F 15 04                 |
| 0x17 | IMPORT          | [count] 0x22 [len][filename] 0x41        | 7F 17 0B 22 08 ...       |

## String Table (LBLs Section)

Located at end of content, format:
```
[4 bytes] "LBLs" marker
[2 bytes] Header (e.g., 0x02 0x41)
[N bytes] Packed strings: [text][index >= 0xC0][text][index]...
[1 byte]  0x00 terminator
```

### String Index Encoding

- Threshold: >= 0xC0 (not 0xE0 as initially thought)
- Range observed: 0xC1-0xFF
- Format: String text PRECEDES its index byte
- Example: `"tore_aap_le" 0xEE` means 0xEE → "tore_aap_le"

### Index Reuse (Compression)

**Important discovery:** Same index can map to multiple strings!

Example from CONST.I:
```
"tore_aap_le"  → 0xEE
"aap_store_le" → 0xEE  (overwrites!)
"symb_store_le" → 0xEE (overwrites again!)
```

**Hypothesis:** This is a compression technique where:
1. Index represents a common suffix or pattern
2. Content context determines which variant to use
3. Possibly combined with preceding text to build full identifiers

**Current implementation:** Keeps last string for each index (simple but lossy)

**Future improvement:** Store all variants per index, use heuristics or context to select

## Content Reference Types

### String References in Content

Bytes >= 0xC0 in content are looked up in string table.

Example:
```
Content: ... 0xD4 ...
String table: 0xD4 → "ASSER"
Result: Insert "ASSER" at that position
```

### Z80 Opcode Bytes

Not yet implemented. Raw Z80 machine code bytes need to be:
1. Identified (not confused with markers/text)
2. Extracted as sequences
3. Disassembled using `cpclib-asm::disass`

## Encoding Challenges

### 1. Ambiguous Byte Range (0x40-0x7F)

This range contains:
- ASCII printable characters ('A'=0x41, 'a'=0x61, etc.)
- Marker bytes (0x40, 0x43, 0x49, 0x4A, 0x64, 0x6D, 0x7F)

**Solution:** Only treat as markers when followed by expected patterns:
- 0x43, 0x49: Must be followed by length byte and text
- 0x4A: Standalone newline (safe)
- 0x64: Followed by length and nested content
- 0x6D: Followed by count byte
- 0x7F: Followed by command byte
- Others: Treat as ASCII

### 2. String Table Compression

Same index used for multiple strings requires:
- Context awareness
- Possibly forward/backward scanning
- Heuristic matching with surrounding text

### 3. Opcode Identification

No clear markers for "this is Z80 code" vs "this is text/data"
- Might be implicit in file structure (after certain commands)
- Might use Data sections (0x64) as containers
- Need to analyze more files to determine pattern

## Test Results

### Current Decoder Capabilities

✅ **Working:**
- File read/write with perfect round-trip (25/26 tests passing)
- Marker detection (Comment, Tab, NewLine, Data, Space, Command)
- Command extraction (IF/ELSE/END, ORG, IMPORT, etc.)
- String table parsing (indices >= 0xC0)
- String reference resolution
- Space count expansion (0x6D)
- ASCII text collection (avoiding false markers)

❌ **Not Yet Implemented:**
- Z80 opcode disassembly
- Byte/Word literal values (0xCF, 0xD7)
- IX/IY indexed addressing (0xDF, 0xFF)
- Full command argument parsing for all types
- String table compression (context-aware selection)
- Metadata header interpretation

### Example Decode: MACRO.I (164 bytes)

```
Input:  02 20 6d 03 60 61 41 4a 7f 09 01 61 7f 0a ...
Output: [Space×3] "`aA" [Newline] [IF 0x01] "a" [ELSE] ...
```

## Next Steps

1. **Metadata header:** Analyze more files, look for patterns correlating with sizes/offsets
2. **Z80 opcodes:** Identify where machine code appears, implement disassembly
3. **String context:** Improve string table to handle multiple values per index
4. **Advanced markers:** Implement 0xCF, 0xD7, 0xDF, 0xFF handling
5. **Full reconstruction:** Combine all elements to generate readable Z80 source

## References

- ASS.Z80 source code (orgass assembler)
- Test corpus: 73 file pairs in `tests/orgams-main/`
- Smallest test file: MACRO.I (164 bytes)
- Largest test file: EXCEPT.O (7,838 bytes)

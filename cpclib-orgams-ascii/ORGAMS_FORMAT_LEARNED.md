# Orgams .O File Format (learned from PARSE.Z80 and DISA.Z80)

## Overview
This document describes the Orgams .O file format as reverse-engineered from the official Orgams source code (PARSE.Z80 and DISA.Z80).

## Key Constants (from PARSE.Z80)

```asm
short_decimal_max = 31  ;code 0 a 31 inclus
short_label = &60       ;de &60 a &df : 128 first labels
long_label = &E0        ;from &E000 to &ffff : 8192 other labels
```

## Byte Range Meanings

### 0x00-0x1F: Short Decimal Numbers
- Values 0-31 are encoded directly as single bytes
- Used in expressions (e.g., `= 1` is encoded as `01`)

### 0x20-0x5F: Operators and Special Elements  
- Expression operators, delimiters, etc.
- Normal ASCII text can appear in this range (space to underscore)

### 0x40-0x4F: Command Markers (from DISA.Z80 dispatch table)
```
0x40 = alab   (label definition)
0x43 = acom   (comment)
0x49 = atab   (tab/indentation)
0x4A = anl    (newline)
0x51 = alocal (local label .label)
0x5B = afact  (factorization marker)
0x64 = aequ   (assignment/equ marker)
0x6D = amac   (macro definition)
0x7F = aesc   (escape to command)
```

### 0x60-0xDF: Short Label References
- 128 labels (indices 0-127)
- Calculated as: `label_index = byte - 0x60`
- These are references to the string table (LBLs section)

### 0xE0-0xFF: Long Label References  
- First byte 0xE0-0xFF indicates long label
- Followed by second byte for full 16-bit index
- Allows 8192 additional labels (indices 0xE000-0xFFFF)

## Assignment Format (0x64 = aequ)

Pattern: `64 [label_index] [expr_size] [expr_bytes...]`

Example:
```
64 b0 01 01
```
Decodes to:
- `64` = aequ marker
- `b0` = label reference (string table index 0xb0 = "inRom")  
- `01` = expression size (1 byte)
- `01` = expression value (the number 1)

Result: `inRom = 1`

## Comment Format (0x43 = acom)

Pattern: `43 [length] [text_bytes...]`

Example:
```
43 17 30 3a 20 74 65 73 74 2e 20 31 3a 61 75 74 6f 2d 69 6e 73 74 61 6c 6c
```
Decodes to:
- `43` = comment marker
- `17` = length (23 bytes)
- Text: "0: test. 1:auto-install"

Result: `; 0: test. 1:auto-install`

## String Table (LBLs section)

Located at end of file after marker: `00 'L' 'B' 'L' 's'`

### Encoding
Strings use **high-byte encoding**: last character has bit 7 set.

Example: "inRom" = `69 6e 52 6f ed`
- `69` = 'i'
- `6e` = 'n'  
- `52` = 'R'
- `6f` = 'o'
- `ed` = 'm' + 0x80 (0x6d | 0x80 = 0xed)

### Indices
Each string is assigned an index starting from 0x60:
- First string: index 0x60
- Second string: index 0x61
- ...
- 128th string: index 0xDF
- 129th+ strings: indices 0xE000+ (long labels, 2 bytes)

## Expression Decoding (from DECEXP.Z80)

The `deco_exp` function shows:
1. First byte is expression size
2. Following bytes are expression elements:
   - 0x00-0x1F: decimal numbers
   - 0x20-0x5F: operators
   - 0x60-0xDF: label references (short)
   - 0xE0-0xFF: label references (long, needs 2nd byte)

## File Structure

```
[Header: 98 bytes]
  "ORGA" magic
  Version info
  Metadata
  
[Content Section]
  Marker: "SRC" (3 bytes)
  Unknown: 3 bytes (63 02 be in EXCEPT.O)
  
  [Encoded Source Code]
    Mixed stream of:
    - Command markers (0x40-0x7F range)
    - Label references (0x60-0xFF)
    - Literal text (0x20-0x5F)
    - Expression data
    
[String Table Section]
  Marker: 00 4C 42 4C 73  (null + "LBLs")
  
  [Strings with high-byte encoding]
    Each string ends with char having bit 7 set
    Strings separated by this encoding
```

## Implementation Notes

### Label Reference Detection
To determine if a byte is a label reference:
```rust
if byte >= 0x60 {
    // It's a label reference - look up in string table
    let label_name = string_table.get(&byte)?;
}
```

### Assignment Decoding
```rust
if byte == 0x64 {  // aequ marker
    let label_index = read_byte();
    let var_name = string_table.get(&label_index)?;
    
    let expr_size = read_byte();
    let expr_bytes = read_n_bytes(expr_size);
    
    // Simple case: single decimal value
    if expr_bytes.len() == 1 && expr_bytes[0] <= 0x1F {
        println!("{} = {}", var_name, expr_bytes[0]);
    }
}
```

### String Table Parsing
```rust
// Find "LBLs" marker
let marker = &[0x00, b'L', b'B', b'L', b's'];

// Parse strings with high-byte encoding
let mut current_string = String::new();
let mut string_index = 0x60;

for &byte in content_after_marker {
    if byte >= 0x80 {
        // Last character of string
        current_string.push((byte & 0x7F) as char);
        string_table.insert(string_index, current_string.clone());
        string_index += 1;
        current_string.clear();
    } else {
        current_string.push(byte as char);
    }
}
```

## Known Issues

### File Synchronization
The .O file (binary) and .Z80 file (text) can become out of sync:
- .O file contains older variable names/order
- .Z80 file is regenerated but .O is stale
- This causes mismatches during reconstruction

Example: EXCEPT.O has label 0x70 in line 1, but string table doesn't contain 0x70 (it has 0xb0="inRom" instead). This means the .O file predates the current string table.

### Indentation
Per user feedback: "Indentation is not stored, must be rebuilt"
- The .O file doesn't preserve exact spacing/indentation
- Tab markers (0x49) indicate indentation points
- Reconstruction must apply indentation rules

## References

- Source: http://orgams.wikidot.com/orgamsmodules
- PARSE.Z80: Parser/encoder (pre-assemble)  
- DISA.Z80: Disassembler/decoder
- DECEXP.Z80: Expression decoder

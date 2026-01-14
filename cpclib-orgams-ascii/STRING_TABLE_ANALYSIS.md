# String Table Reference System in Orgams .O Files

## Discovery

The Orgams binary format uses a **string table reference system** where:
1. Common identifiers and symbols are stored in a string table at the end of the file
2. References to these strings use bytes >= 0xA0 (or perhaps >= 0xC0) in the content section
3. The string table starts after the "LBLs" marker

## Example from EXCEPT.O

### String Table Location
At offset `0x001b20`:
```
00 4c 42 4c 73  ...  
   L  B  L  s
```
The "LBLs" marker (with 00 prefix) indicates the start of the labels/symbols string table.

### String Table Content
At offset `0x001ba0`, we find:
```
69 6e 52 6f ed
i  n  R  o  0xed
```

This is "inRom" where:
- `69 6e 52 6f` = "inRo" (ASCII)
- `ed` = encoded 'm' (high byte encoding, 0xed = 0x6d + 0x80 = 'm' + 128)

## How References Work

### At offset 0x00000080
```
64 a1 01 01
```

Breaking this down:
- `64` (0x64) = Data/directive marker ('d')
- `a1` (0xa1 = 161) = **String table reference**
- `01` = value 1
- `01` = another byte (possibly related to the assignment)

So `64 a1 01 01` encodes something like: `inRom = 1`

The `0xa1` byte is an **index or pointer** into the string table to retrieve "inRom".

## Current Parser Problem

The `extract_lines()` function in `format.rs` does NOT handle string table references:

```rust
pub fn extract_lines(&self) -> Vec<(Option<LineMarker>, String)> {
    // ...
    if marker.is_some() && i + 1 < data.len() {
        let length = data[i + 1] as usize;  // ← Assumes next byte is length!
        // ...
    }
}
```

When it encounters `64 a1`, it treats:
- `64` as marker
- `a1` (161) as LENGTH → tries to read 161 bytes of text!

This is why reconstruction fails catastrophically.

## Proper Parsing Algorithm

To correctly parse, the decoder needs to:

1. **First pass**: Extract the string table
   - Find "LBLs" marker
   - Parse all strings after it
   - Build a lookup table: `byte_index → string`

2. **Second pass**: Parse content with string resolution
   - When encountering a marker (0x43, 0x49, 0x64, etc.)
   - Check if next byte is:
     - `< 0x80`: It's a length byte, read that many ASCII chars
     - `>= 0x80`: It's a string table reference, look it up

3. **String table encoding**: 
   - Strings use high-bit encoding: bytes with bit 7 set have the high bit stripped
   - Example: `0xed` → `0x6d` ('m')

## String Table Index Examples

From the hex dump at end of EXCEPT.O (offset 0x001b20+):
```
Offset  Hex                              ASCII/Decoded
------  -------------------------------  ----------------------
1b26    6d 79 5f 72 6f 75 74 69 6e e5   my_routin[0xe5='e']
1b36    65 78 63 65 70 74 5f 65 6e 74 65 f2  except_ente[0xf2='r']
1b46    65 78 69 74 5f 72 6f 75 74 69 6e e5  exit_routin[0xe5='e']
1b56    65 78 63 65 70 74 5f 65 78 69 f4     except_exi[0xf4='t']
1ba0    69 6e 52 6f ed                       inRo[0xed='m']
1ba5    72 6f ed                             ro[0xed='m']
```

Each string ends with a high-byte character (>= 0x80), where the last character has bit 7 set.

## Impact on Reconstruction

Without proper string table handling:
- `inRom = 1` becomes garbled binary
- Variable names are lost
- Label references fail
- Assignment statements are corrupted

## Solution Required

The decoder needs a complete rewrite of its parsing logic to:
1. Parse string table first (after "LBLs")
2. Handle high-byte encoding (0x80+ = last char with bit 7 stripped)
3. Resolve references during content parsing
4. Distinguish between length bytes and reference bytes

## Testing Commands

To examine the string table:
```bash
# Find LBLs marker
hexdump -C tests/orgams-main/EXCEPT.O | grep "4c 42 4c"

# Show string table area
hexdump -C tests/orgams-main/EXCEPT.O | tail -80

# Search for specific strings
strings tests/orgams-main/EXCEPT.O | grep inRom  # Won't work!
# Because "inRom" is encoded with high bytes: "inRo" + 0xed
```

## Conclusion

The Orgams .O format is **NOT lossy** - it contains all the information needed to reconstruct the Z80 source. However, it uses an efficient encoding with:
- String table for common identifiers
- References instead of repeated strings
- Compact encoding of assignments and directives

The current parser implementation simply doesn't decode these references, leading to the garbled output observed in testing.

# Orgams Binary Reconstruction Status

## Summary

After extensive testing of Orgams binary file (.O and .I) reconstruction, I found that **perfect reconstruction of the original Z80 source from binary files is not currently possible** with the existing codebase.

## Test Results

### SWAPI.I (Include File - 2021 bytes, 77 source lines)
- **Format**: `.I` file (include/intermediate format)
- **Result**: Failed reconstruction (10.4% match)
- **Issue**: .I files have different internal structure than .O files
- **Output**: Garbled text mixing binary markers and partial strings

### EXCEPT.O (Object File - 499 source lines)
- **Format**: `.O` file (compiled object format)
- **Result**: Failed reconstruction (minimal match)
- **Reconstructed**: Only 90 lines from 499 source lines
- **Issue**: OrgamsFile parser cannot accurately reconstruct Z80 source

## Why Reconstruction Fails

### 1. **Information Loss**
The binary format is a compiled representation that loses:
- Original spacing and formatting
- Comment positioning and indentation
- Line structure and organization
- Some textual context

### 2. **Complex Binary Encoding**
The Orgams binary format uses:
- Multiple marker types (0x43, 0x49, 0x4A, 0x6D, 0x7F, etc.)
- String table references (bytes >= 0xC0)
- Command encoding (IMPORT, ORG, IF, ELSE, END)
- Mixed ASCII and binary data
- Context-dependent interpretation of bytes

### 3. **Parser Limitations**
Current `OrgamsFile::to_z80_text()` implementation:
- Extracts only 90 lines from 499-line source (EXCEPT.O)
- Produces garbled output with binary markers visible
- Mixes control characters with text
- Cannot handle .I files at all

## What Works

### Binary Parsing (25/26 tests pass)
- ✅ Reading .O files successfully
- ✅ Extracting file structure (header, content, metadata)
- ✅ Parsing markers and commands
- ✅ Identifying string tables
- ✅ Recognizing line markers and indentation

### Partial Reconstruction
- ✅ Some comments are preserved (with extra spaces)
- ✅ Major structural markers visible
- ✅ Import statements partially reconstructed
- ✅ Some assembly mnemonics preserved

## Current Reconstruction Output Example

### EXCEPT.O Line 1
**Expected (source)**:
```
inRom = 1               ;0: test. 1:auto-install
```

**Got (reconstructed)**:
```
��C0: test. 1:auto-installd���Jd���C0: orgext (see warning below)C <<< Nested Exceptions >>>C; Escape a routi
```

The output shows:
- Binary markers (�, C, J, d) mixed with text
- Partial string fragments
- Multiple lines concatenated
- Control characters visible

## Recommendations

### For Perfect Reconstruction
To achieve accurate Z80 source reconstruction, you would need:

1. **Keep Original Source**
   - Store `.Z80` source files alongside `.O` files
   - Use binary files only for machine execution
   - Maintain version control of source

2. **Enhanced Binary Format**
   - Store complete formatting information
   - Preserve all whitespace and positioning
   - Include metadata for reconstruction

3. **Rewrite Reconstruction Logic**
   - Map all marker types to their meanings
   - Handle string table lookups correctly
   - Reconstruct proper line breaks and indentation
   - Process command arguments fully

### For Current Use
The existing parser is **excellent for**:
- Analyzing binary structure
- Understanding the Orgams format
- Extracting specific data sections
- Educational/research purposes

But **not suitable for**:
- Converting `.O` back to `.Z80` source
- Recovering lost source code
- Automated source regeneration

## Files Used in Testing

### Test Input Files
- `tests/orgams-main/SWAPI.I` - 2021 bytes, 77 source lines (`.I` format)
- `tests/orgams-main/SWAPI.Z80` - Reference Z80 source
- `tests/orgams-main/EXCEPT.O` - Object file, 499 source lines
- `tests/orgams-main/EXCEPT.Z80` - Reference Z80 source

### Test Files Created
- `tests/test_swapi_clean.rs` - Reconstruction comparison test
- `tests/test_swapi_reconstruction.rs` - Byte-by-byte manual parsing attempt

## Conclusion

The Orgams binary format is a **lossy compilation** that cannot be perfectly reversed to recreate the original Z80 assembly source. The binary files are meant for execution and linking, not for source code recovery.

**Always keep your `.Z80` source files safe!** The `.O` and `.I` files are build artifacts, not replacements for source code.

## Related Documentation

- `FORMAT_ANALYSIS.md` - Detailed binary format specification
- `tests/integration.rs` - Working tests that validate file parsing
- `src/decoder.rs` - Current decoder implementation
- `src/lib.rs` - OrgamsFile API

## Tested By
This analysis was performed through comprehensive testing including:
- Manual hex dump analysis of SWAPI.I (first 200 bytes)
- Byte-by-byte reconstruction attempts
- Comparison with reference `.Z80` sources
- Testing both `.O` (object) and `.I` (include) file formats
- Using existing `OrgamsFile` API for reconstruction

Date: 2025 (based on SWAPI.Z80 file comments)

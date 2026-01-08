# BASM Documentation Improvements TODO

Last updated: 2026-01-06

## Priority 1 - Missing Core Content

### 1. Add Lists and Matrices to expression-types.md
**Status**: Not started
**Files**: `docs/basm/expression-types.md`

Currently only functions are documented. Need to add:
- List literal syntax: `[1, 2, 3]`, `[1, 2.5, 3]`
- Empty lists: `[]`
- Matrix literal syntax (list of lists)
- Examples from test files:
  - `cpclib-basm/tests/asm/good_document_lists.asm` (already exists with comprehensive examples)
  - Matrix test files if they exist

### 2. Complete syntax.md Examples
**Status**: Needs fixing
**Files**: `docs/basm/syntax.md`

Issues found:
- Line ~98: Multiline comment example is incomplete (missing closing `*/`)
- Add more module/namespace usage examples showing benefits
- Add examples of expressions in different contexts (labels, data, conditionals)

### 3. Document Ternary Operator
**Status**: Not documented
**Files**: `docs/basm/expression-types.md` (add new section)

Test file exists: `cpclib-basm/tests/asm/good_document_ternary.asm`
Need to add:
- Syntax: `condition ? value_if_true : value_if_false`
- Examples from test file
- Nesting examples

## Priority 2 - Missing Major Topics

### 4. Create Macros Documentation
**Status**: Not started
**Files**: New file `docs/basm/macros.md` needed

Content needed:
- MACRO/ENDM syntax
- Parameters (with and without defaults)
- Local labels in macros
- Macro invocation syntax
- Nested macros
- Common patterns and best practices
- Examples from existing test files

### 5. Document Conditional Assembly
**Status**: Not started
**Files**: `docs/basm/directives.md` (expand)

Directives to document:
- IFDEF, IFNDEF
- IF/ELSE/ENDIF
- Conditional compilation use cases
- Build configurations

### 6. Document Loop/Repeat Directives
**Status**: Test files exist but not documented
**Files**: `docs/basm/directives.md` (expand)

Found test files:
- `cpclib-basm/tests/asm/good_document_for.asm`
- `cpclib-basm/tests/asm/good_iter.asm`
- REPT directive

Need to document:
- FOR loops syntax and usage
- REPT syntax
- Use cases for code generation

### 7. Include System Documentation
**Status**: Partial
**Files**: `docs/basm/directives.md` (expand)

Need to document:
- INCLUDE directive
- INCBIN directive
- Search paths (-I flag)
- Relative vs absolute paths
- Embedded files (inner://)

## Priority 3 - User Experience

### 8. Add Quick Start Tutorial
**Status**: Not started
**Files**: New file `docs/basm/quickstart.md` needed

Should cover:
- Installing basm
- First assembly (hello world step-by-step)
- Common workflows:
  - Assemble to binary
  - Assemble to snapshot
  - Assemble to DSK
  - Test in emulator
- Common errors and fixes

### 9. Add Practical Examples Section
**Status**: Not started
**Files**: New file `docs/basm/examples.md` needed

Example projects:
- Loading screen with BASIC loader
- Multi-file project with includes
- Using crunchers (LZSA, Exomizer, etc.)
- Sprite/graphics data generation
- Music player integration
- Memory banking for 128K

### 10. Improve functions.md Organization
**Status**: Needs refactoring
**Files**: `docs/basm/functions.md`

Improvements:
- Add quick reference table at top
- Reorganize by use case:
  - Assembly helpers (opcode, assemble)
  - Graphics (pen, mode conversions)
  - Data structures (list, matrix, string)
  - File I/O (load)
  - Compression (binary_transform)
  - Memory sections
- Add usage examples for complex functions
- Add "See also" cross-references

## Priority 4 - Advanced Topics

### 11. Multi-pass Assembly
**Status**: Not documented
**Files**: New section in `docs/basm/index.md` or `syntax.md`

Topics:
- Why multi-pass is needed
- Forward references
- How to handle circular dependencies
- Performance implications

### 12. Memory Banking
**Status**: Test files exist
**Files**: New file `docs/basm/advanced.md`

Found test file: `cpclib-basm/tests/asm/good_document_bank.asm`
Topics:
- Banking for 128K/Plus models
- BANK directive
- MMR register handling
- BANKSET directive

### 13. Output Formats
**Status**: Partially documented in cmdline.md
**Files**: New file `docs/basm/output-formats.md`

Need comprehensive guide for:
- Binary files (--binary)
- Snapshot files (--snapshot)
- CPR files (--cartridge)
- DSK files
- Snapshot chunks (BRKC, BRKS, REMU, SYMB)
- LOCOMOTIVE BASIC headers (--basic)

### 14. Debug Support
**Status**: Minimal
**Files**: New section needed

Found test files:
- `cpclib-basm/tests/asm/good_breakpoint.asm`
- `cpclib-basm/tests/asm/good_document_export.asm`
- `cpclib-basm/tests/asm/good_document_pause.asm`

Topics:
- Breakpoints (--breakpoint-as-opcode)
- Symbol files (--sym, --sym_kind)
- REMU files (--remu)
- WABP files (--wabp)
- Integration with emulators

## Priority 5 - Polish

### 15. Fix directives.md AI-Generated Content
**Status**: Needs review
**Files**: `docs/basm/directives.md`

Current warning: "Most content is IA generated and not yet proofread"
- Review all AI-generated content
- Verify examples compile
- Add cross-references
- Add "See also" sections

### 16. Add Missing Cross-References
**Status**: Not started
**Files**: All documentation files

Needed:
- Expression types → functions that use them
- Directives → related functions
- Functions → example code
- Searchable index of all symbols

### 17. Add Operator Precedence Table
**Status**: Missing
**Files**: `docs/basm/expression-types.md`

Need comprehensive table showing:
- Arithmetic operators
- Logical operators
- Comparison operators
- Bitwise operators
- Precedence levels
- Associativity

### 18. Migration Guide
**Status**: Not started
**Files**: New file `docs/basm/migration.md`

Compare with other assemblers:
- From Maxam/Winape
- From rasm
- From sjasmplus
- From vasm
- Syntax differences
- Feature comparison table

## Quick Wins (Easy to Implement)

1. ✅ Fix multiline comment example in syntax.md (just add `*/`)
2. ✅ Add Lists section to expression-types.md (test file already exists)
3. ✅ Document ternary operator (test file already exists)
4. Add FOR loop documentation (test file already exists)
5. Add PAUSE directive documentation (test file exists)
6. Add EXPORT directive documentation (test file exists)

## Notes

- Many test files in `cpclib-basm/tests/asm/good_document_*.asm` are ready to be included in docs
- All examples should be tested with actual assembly
- Consider adding interactive examples or a playground
- May want to generate API reference automatically from code

## Resources

Test files directory: `/home/romain/Perso/CPC/rust.cpcdemotools/cpclib-basm/tests/asm/`
Documentation directory: `/home/romain/Perso/CPC/rust.cpcdemotools/docs/basm/`

198 test files found with `good_*.asm` naming pattern - many can be used as documentation examples.

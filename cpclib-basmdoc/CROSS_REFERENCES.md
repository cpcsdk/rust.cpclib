# Cross-Reference Feature in basmdoc

## Overview

The basmdoc documentation generator now includes a cross-reference feature that shows where each documented symbol (label, macro, function, equ) is referenced throughout the codebase.

## Features

### What is tracked

- **Macro calls**: When a macro is invoked, it's recorded as a reference
- **Label references**: Labels used in any opcode operand (JP, JR, CALL, LD, ADD, SUB, etc.)
- **EQU references**: When an EQU constant is used in expressions or opcode operands
- **Expression symbols**: Any identifier appearing in opcode operands (addresses, immediate values, arithmetic expressions)

The extraction works by parsing operands from all opcodes, not just control flow instructions. This means references are detected in:
- Load instructions: `LD HL, my_label`, `LD A, (screen_addr)`, `LD B, my_constant`
- Arithmetic: `ADD A, my_value`, `SUB my_offset`, `INC (my_buffer)`
- Logical: `AND my_mask`, `OR my_flag`, `XOR my_pattern`
- Control flow: `JP my_label`, `CALL my_function`, `JR loop`
- Any other opcode that uses symbols in its operands

### What is displayed

For each documented item, the cross-references section shows:
- The file and line number where the symbol is referenced
- The context (the actual line of code containing the reference)

### Example Output

For a macro like:
```z80asm
;; Wait the vsync signal
macro WAIT_VSYNC comment
    ld b, 0xf5
@vsync
    in a, (c)
    rra
    jr nc, @vsync
endm

    WAIT_VSYNC("Wait for screen refresh")
```

The documentation will show:

**MACRO WAIT_VSYNC(comment)**

Wait the vsync signal

**Referenced in:**
- simple_code.asm:30 - `WAIT_VSYNC("Wait for screen refresh")`

## Implementation Details

### Data Structures

- `SymbolReference`: Stores information about each reference (file, line number, context)
- `ItemDocumentation.references`: A vector of all references to this item

### Symbol Extraction Logic

The implementation analyzes all tokens in the source code:

1. **Skip non-code tokens**: Comments are skipped
2. **Handle labels**: Label definitions are skipped (they're not references)
3. **Handle EQU definitions**: Extract symbol references from the value expression
4. **Handle opcodes**: Parse all opcode operands and extract identifiers
   - Split operands on delimiters (spaces, commas, parentheses, operators)
   - Extract identifiers (alphanumeric sequences starting with letter/underscore)
   - Filter out Z80 registers (A, B, C, D, E, H, L, AF, BC, DE, HL, IX, IY, etc.)
   - Filter out condition flags (Z, NZ, C, NC, P, M, PE, PO)
5. **Handle macro calls**: Identify lines that start with identifiers (not directives/instructions)

This approach catches symbol references in:

- Load instructions: `LD HL, my_label` extracts `my_label`
- Arithmetic operations: `ADD A, my_constant` extracts `my_constant`
- Memory operations: `LD (screen_addr), A` extracts `screen_addr`
- Jump instructions: `JP my_label` extracts `my_label`
- Complex expressions: `LD BC, buffer_start + offset` extracts both `buffer_start` and `offset`

### Template Integration

The cross-references are displayed in the HTML output using:
- CSS styling with blue left border and light background
- Clickable file:line references
- Formatted code context blocks

## Limitations

### Current Implementation

The symbol extraction uses string-based parsing due to limitations in the `ListingElement` trait, which doesn't expose all necessary methods for detailed token analysis. Specifically:

- No access to macro call methods (`is_macro_call()`, `macro_call_name()`)
- No access to expression parsing (`expr()`, `symbols_used()`)

As a result, the implementation:
- Uses heuristic-based detection for macro calls
- May miss some complex symbol references in expressions
- Relies on pattern matching against known directives and instructions

### Future Improvements

Potential enhancements:

1. **Enhanced trait**: Extend `ListingElement` to expose more token details
2. **Expression parsing**: Full analysis of expressions to extract all symbol references
3. **Type information**: Track whether a reference is read/write/call
4. **Cross-file references**: Better handling of symbols defined in included files
5. **Reference filtering**: Options to show/hide certain types of references

## Usage

The cross-reference feature is automatically enabled. When you generate documentation:

```bash
basmdoc --output output.html input.asm
```

All documented items will include their cross-references if any exist.

## Testing

Test files demonstrating the feature:
- `cpclib-basmdoc/tests/simple_code.asm` - Shows macro calls and EQU references
- `cpclib-basmdoc/tests/local_labels.asm` - Shows label references in jump instructions

## Technical Notes

### Performance

The cross-reference collection is performed once per file during documentation generation:

1. Parse all tokens
2. Extract symbols from each token
3. Build a HashMap mapping symbols to their references
4. Match references to documented items

This approach is efficient as it only requires a single pass through the tokens.

### Code Organization

Key functions in `cpclib-basmdoc/src/lib.rs`:

- `extract_symbols_from_token<T>()` - Extracts symbol names from a single token
- `collect_cross_references<T>()` - Builds the reference map for all tokens
- `populate_cross_references<T>()` - Matches references to documented items

Template: `cpclib-basmdoc/src/templates/html_documentation.jinja`
- `show_references` macro - Renders the cross-reference section

CSS: `cpclib-basmdoc/src/templates/documentation.css`
- Styles for `.cross-references`, `.reference-list`, `.reference-item`

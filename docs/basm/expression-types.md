# Expression Data Types

Basm supports several data types in expressions.

## Integer Types

Integers can be written in multiple formats:

- **Decimal**: `42`, `255`
- **Hexadecimal**: `$FF`, `#ABCD`, `&CAFE`, `0x1234`, `0X5678`
- **Binary**: `%11001100`, `0b10101010`, `0B11110000`  
- **Octal**: `0o377`, `0O177`, `@377`
- **Character**: `'A'` (evaluates to ASCII value 65)

All numeric formats are demonstrated in the test file:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_numeric_bases.asm"
```

Negative integers use the unary minus: `-42`

## Floats

Floating point values support decimal notation and scientific notation:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_floats.asm"
```

Examples:

- `3.14159`
- `2.5`
- `-0.5`
- `1.0e-6` (scientific notation)
- `1.5e3` equals 1500.0

## Strings

String literals are enclosed in double quotes and are primarily used with the `DB` directive:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_strings.asm"
```

Strings can contain escape sequences:

- `\n` - newline
- `\t` - tab
- `\\` - backslash
- `\"` - quote

String functions:

- `string_len(str)` - returns the length of a string
- `string_concat(str1, str2, ...)` - concatenates multiple strings (2 or more arguments)

## Booleans

Boolean values for conditional expressions:

- **True**: `true`, `1`
- **False**: `false`, `0`

Booleans are demonstrated in the test file:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_booleans.asm"
```

Boolean operators include:

- **Logical AND**: `&&`
- **Logical OR**: `||`
- **Logical NOT**: `!`, `NOT`
- **Comparison**: `==`, `!=`, `<`, `>`, `<=`, `>=`

## Labels

Labels can be referenced in expressions and resolve to addresses:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_labels.asm"
```

The special symbol `$` represents the current program counter.

## Lists

Lists are heterogeneous collections enclosed in square brackets:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_lists.asm"
```

Lists support:

- Indexing: `list[0]` (0-based)
- Nesting: `[[1, 2], [3, 4]]`
- Functions: `list_len()`, `list_get()`, etc.

## Matrices

Matrices are 2D arrays, created via `matrix_new()` or from nested lists:

```z80
--8<-- "cpclib-basm/tests/asm/good_matrix.asm"
```

Matrices support various operations through built-in functions (see [functions](functions.md#matrix-functions)).

Matrices support specialized access functions documented in the [functions page](functions.md#matrix-functions).

## Operators

### Binary Operators

Listed by precedence (highest to lowest):

1. **Multiplication/Division**: `*`, `/`, `%` (modulo)
2. **Addition/Subtraction**: `+`, `-`
3. **Bitwise Shift**: `<<`, `>>`
4. **Relational**: `<`, `>`, `<=`, `>=`
5. **Equality**: `==`, `!=`
6. **Bitwise AND**: `&`
7. **Bitwise XOR**: `^`
8. **Bitwise OR**: `|`
9. **Logical AND**: `&&`
10. **Logical OR**: `||`

### Unary Operators

- **Negation**: `-x` (arithmetic)
- **Bitwise NOT**: `~x`
- **Logical NOT**: `!x`
- **Low byte**: `<x` (equivalent to `low(x)`)
- **High byte**: `>x` (equivalent to `high(x)`)

### Operator Examples

```z80
--8<-- "cpclib-basm/tests/asm/good_document_operators.asm"
```

## Type Conversions

Implicit conversions occur in expressions:

- Integer to Float: automatic when mixed with floats
- Boolean to Integer: `true` → 1, `false` → 0
- Integer to Boolean: 0 → `false`, non-zero → `true`
- Character to Integer: automatic (ASCII value)

## Function Calls

Functions are called with parentheses:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_function_calls.asm"
```

See the [functions page](functions.md) for a complete list of built-in functions.

## Special Symbols

- **`$`** - Current program counter (assembly address)
- **`$$`** - Start of current section
- **`$-$$`** - Offset within current section

```z80
--8<-- "cpclib-basm/tests/asm/good_document_special_symbols.asm"
```

## Conditional Expressions

The ternary operator for inline conditionals:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_ternary.asm"
```


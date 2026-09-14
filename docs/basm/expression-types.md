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

- Indexing and slicing: `list[0]`, `list[1..3]` - see [Indexing and Slicing](#indexing-and-slicing)
- Nesting: `[[1, 2], [3, 4]]`
- Functions: `list_len()`, `list_get()`, etc.

## Ranges

A range denotes a sequence of integers without writing every value out by hand. Range syntax matches
Rust's own exactly:

- **`a..b`** - exclusive of `b`
- **`a..=b`** - inclusive of `b`

```z80
--8<-- "cpclib-basm/tests/asm/good_document_ranges.asm"
```

A range is empty when `a > b` - there is no auto-descending. There is no dedicated stepped-range
syntax (no `a..step..b`); to step through a range, either call `range_step_by(a_range, step)` or
combine a range with [broadcasting](#broadcasting), e.g. `base + (0..n) * stride`.

A range behaves like a list wherever a list is expected - `list_len()`, `list_get()`, `DB`/`DEFW`/`STR`
emission, and `ITERATE ... IN` all accept a range directly, with no conversion needed. Unlike a list
literal, a range never allocates its elements up front: `db 0..65536` and `list_len(0..65536)` compute
directly from the range's bounds instead of building a 65536-element list first.

A range used unparenthesized inside arithmetic is a parse error (the range operator has the same low
precedence Rust's own does) - write `(0..5) * 2`, not `0..5 * 2`.

## Broadcasting

Arithmetic (`+ - * / %`), bitwise (`&`, `|`), and relational (`< > <= >=`) operators apply
element-wise when one or both operands is a list (or a range):

```z80
--8<-- "cpclib-basm/tests/asm/good_document_broadcasting.asm"
```

- **List and scalar, either order**: the scalar combines with every element - `[1,2,3] + 10` and
  `10 + [1,2,3]` both give `[11,12,13]`.
- **Two lists of the same length**: elements combine pairwise - `[1,2,3] + [10,20,30]` gives
  `[11,22,33]`. Lists of different lengths are a hard error.
- **Nested lists**: broadcast recursively through every level.
- **A range**: converts to a list first, since a scaled or shifted range (e.g. `(0..1000) * 2`) is no
  longer a contiguous range.
- **`==` and `!=` do not broadcast.** They compare the whole list (or range) at once and return a
  single boolean, exactly as they always have - `[1,2] == [1,2]` is `true`, not `[true,true]`.
  Broadcasting `==`/`!=` would silently change that shape, breaking anything using such a comparison
  as an `IF`/`ASSERT` condition.
- A relational comparison broadcasts into a list of booleans, which - like any other list - cannot be
  used directly as an `IF`/`ASSERT` condition without picking an element out of it first.

## Matrices

Matrices are 2D arrays, created via `matrix_new()` or from nested lists:

```z80
--8<-- "cpclib-basm/tests/asm/good_matrix.asm"
```

Matrices support various operations through built-in functions (see [functions](functions.md#matrix-functions)).

Matrices support specialized access functions documented in the [functions page](functions.md#matrix-functions),
and the `[x, y]` bracket form below.

## Indexing and Slicing

`target[...]` accesses an element or a slice of a list, string, range, or matrix - the same bracket
notation used to *write* a list literal (`[1, 2, 3]`), applied *after* an existing value instead:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_subscript.asm"
```

- **`target[i]`** - a single element (0-based). On a list or range this gives a value; on a string
  it gives a character.
- **`target[a..b]`** - a slice, using a [range](#ranges) as the index. Works on lists and strings,
  giving back the same kind of value (a sub-list or a sub-string).
- **`target[[i, j, ...]]`** - a gather: gives back a new list holding the elements at each of the
  given positions, in order. Works anywhere `target[i]` does (list, string, range).
- **`target[x, y]`** - two indices, for a matrix only: `x` is the column, `y` is the row.
- Indexing a range is constant-time, just like `list_len`/`list_get` on a range - no list is
  materialized to answer `(0..1000000)[500000]`.
- Subscripts bind as tightly as possible, directly to the value they follow, before any binary
  operator - `a[0] + b[1]` is `(a[0]) + (b[1])`. They also chain: `a[0][1]` applies the second `[1]`
  to the result of `a[0]`.
- A literal can be indexed directly, without a named variable: `[1, 2, 3][1]`, `"abc"[0]`,
  `(0..5)[2]`.

## Operators

### Binary Operators

Listed by precedence (highest to lowest). [Indexing/slicing](#indexing-and-slicing) (`target[...]`)
binds tighter than any of these - it applies directly to the value it follows, before any operator
below gets a chance to.

1. **Multiplication/Division**: `*`, `/` (real division), `//` (integer division), `%` (modulo)
2. **Addition/Subtraction**: `+`, `-`
3. **Bitwise Shift**: `<<`, `>>`
4. **Relational**: `<`, `>`, `<=`, `>=`
5. **Equality**: `==`, `!=`
6. **Bitwise AND**: `&`
7. **Bitwise XOR**: `^`
8. **Bitwise OR**: `|`
9. **Logical AND**: `&&`
10. **Logical OR**: `||`

`/` always divides as a real number, even for two integer operands (e.g. `7 / 2` is `3.5`). `//` always divides as an integer, truncating toward zero (e.g. `7 // 2` is `3`, `-7 // 2` is `-3`). Loading a real value into a register (e.g. `ld a, 7 / 2`) emits a warning, since Z80 registers can only hold integers.

!!! warning "Breaking change"
    `//` used to also work as a line-comment marker, in addition to `;`. As
    of this release it is exclusively the integer-division operator - only
    `;` starts a line comment now.

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


# Built-in Functions

This page documents all built-in functions available in basm expressions.

## Mathematical Functions

### Trigonometric Functions

- **`sin(x)`** - Sine of x (x in radians)
- **`cos(x)`** - Cosine of x (x in radians)  
- **`asin(x)`** - Arc sine of x (returns radians)
- **`acos(x)`** - Arc cosine of x (returns radians)
- **`atan2(y, x)`** - Arc tangent of y/x (returns radians)

### Exponential and Logarithmic Functions

- **`exp(x)`** - Exponential function (e^x)
- **`ln(x)`** - Natural logarithm (base e)
- **`log10(x)`** - Logarithm base 10
- **`pow(base, exponent)`** - Power function (base^exponent)
- **`sqrt(x)`** - Square root

### Rounding and Modulo Functions

- **`floor(x)`** - Largest integer ≤ x
- **`ceil(x)`** - Smallest integer ≥ x  
- **`int(x)`** - Integer part (truncate towards zero)
- **`frac(x)`** - Fractional part
- **`abs(x)`** - Absolute value
- **`fmod(x, y)`** - Floating point remainder of x/y
- **`remainder(x, y)`** - IEEE remainder function

### Comparison and Utility Functions

- **`min(a, b, ...)`** - Minimum value (variadic)
- **`max(a, b, ...)`** - Maximum value (variadic)  
- **`clamp(value, min, max)`** - Clamp value between min and max
- **`fmin(a, b)`** - Minimum of two floats
- **`fmax(a, b)`** - Maximum of two floats
- **`fdim(x, y)`** - Positive difference (max(x-y, 0))
- **`fstep(edge, x)`** - Step function (0 if x<edge, 1 if x≥edge)
- **`isgreater(x, y)`** - Test if x > y (returns 0 or 1)
- **`isless(x, y)`** - Test if x < y (returns 0 or 1)
- **`hypot(x, y)`** - Euclidean distance sqrt(x²+y²)
- **`ldexp(x, exp)`** - x * 2^exp

## Bit Manipulation Functions

- **`high(value)`** / **`hi(value)`** - High byte of 16-bit value
- **`low(value)`** / **`lo(value)`** - Low byte of 16-bit value

## Memory Access Functions

- **`peek(address)`** / **`memory(address)`** - Read byte from memory at address during assembly

## String Functions

- **`char(value)`** - Convert integer to single character string
- **`string_new(length, filler)`** - Create string of given length filled with filler  
- **`string_push(string, char_or_string)`** - Append character or string
- **`string_concat(s1, s2, ...)`** - Concatenate strings (variadic)
- **`string_from_list(list)`** - Convert list of integers to string
- **`string_len(string)`** - Length of string (same as `list_len`)

## Pixels

- **`mode0_byte_to_pen_at(byte, position)`** - Extract pen number at position (0 or 1) from mode 0 byte
- **`mode1_byte_to_pen_at(byte, position)`** - Extract pen number at position (0-3) from mode 1 byte
- **`mode2_byte_to_pen_at(byte, position)`** - Extract pen number at position (0-7) from mode 2 byte
- **`pen_at_mode0_byte(byte, position)`** - Get pen at position in mode 0 byte
- **`pen_at_mode1_byte(byte, position)`** - Get pen at position in mode 1 byte
- **`pen_at_mode2_byte(byte, position)`** - Get pen at position in mode 2 byte
- **`pens_to_mode0_byte(pen0, pen1)`** - Convert 2 pens to mode 0 byte
- **`pens_to_mode1_byte(pen0, pen1, pen2, pen3)`** - Convert 4 pens to mode 1 byte
- **`pens_to_mode2_byte(pen0, ..., pen7)`** - Convert 8 pens to mode 2 byte

## List Functions

- **`list_new(length, filler)`** - Create list of given length filled with filler value
- **`list_get(list, index)`** - Get element at index
- **`list_set(list, index, value)`** - Set element at index (returns new list)
- **`list_len(list)`** - Length of list
- **`list_sublist(list, start, end)`** - Extract sublist (end is not included)
- **`list_sort(list)`** - Sort list in ascending order (returns new list)
- **`list_argsort(list)`** - Return indices that would sort the list
- **`list_push(list, element)`** - Append element to list (returns new list)
- **`list_extend(list1, list2)`** - Concatenate two lists (returns new list)

## Matrix Functions

To be called on a matrix object or a list of list object (WIP).

- **`matrix_new(width, height, filler)`** - Create matrix filled with value, or `matrix_new(list_of_lists)` to create from nested lists
- **`matrix_set(matrix, x, y, value)`** - Set element at position (returns new matrix)
- **`matrix_get(matrix, x, y)`** - Get element at position
- **`matrix_col(matrix, x)`** - Get column as list
- **`matrix_row(matrix, y)`** - Get row as list  
- **`matrix_set_col(matrix, x, list)`** - Set column from list (returns new matrix)
- **`matrix_set_row(matrix, y, list)`** - Set row from list (returns new matrix)
- **`matrix_width(matrix)`** - Get matrix width
- **`matrix_height(matrix)`** - Get matrix height

## File Functions

- **`load("filename")`** - Load file content as list of bytes

## Code Assembly Function

- **`assemble("z80_code")`** - Assemble Z80 code string and return bytes as list

Example:

```z80
bytes = assemble("LD A, 5")  ; Returns list of assembled bytes
```

## Binary Transformation Function

- **`binary_transform(data, "crunch_type")`** - Compress data using specified cruncher

Supported crunch types:

- `"LZEXO"`, `"LZ4"`, `"LZ48"`, `"LZ49"`
- `"LZSHRINKLER"`, `"LZX7"`, `"LZX0"`, `"LZAPU"`  
- `"LZSA1"`, `"LZSA2"`, `"LZUPKR"`
- `"BackwardZx0"` (backward variant)

Example:

```z80
data = [1, 2, 3, 4, 5]
compressed = binary_transform(data, "LZ48")
```

## Section Functions

- **`section_start("section_name")`** - Get start address of named section
- **`section_stop("section_name")`** - Get stop address of named section  
- **`section_length("section_name")`** - Get length of named section
- **`section_used("section_name")`** - Get number of bytes actually used in section
- **`section_mmr("section_name")`** - Get memory mapper register value for section


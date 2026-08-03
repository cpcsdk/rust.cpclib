# Z80 Syntax

## General syntax

```
LABEL OPCODE1
      OPCODE2 : OPCODE3
      DIRECTIVE
```



!!! warning

    There may be still some subtle parser bugs, but it is possible to span instructions and directives on several lines by ending the previous line with `\`

## Labels handling

`BASM` is quite lax on the z80 format: it does not impose to start a label at the very first char of a line and does not force an instruction or directive to not start at the very first line of a char (behavior stolen to `rasm`).
As a consequence there can be ambiguities between labels and macros.
If it fails in properly recognizing macros or label, you can guide it by suffixing label declaration by : or by using (void) for macros with no arguments. 


### Local labels
```z80
--8<-- "cpclib-basm/tests/asm/good_labels.asm"
```

### module handling

```z80
--8<-- "cpclib-basm/tests/asm/good_module.asm"
```
would generate a binary similar to
```z80
--8<-- "cpclib-basm/tests/asm/good_module.equiv"
```

### Labels generation

Labels can be generated thanks to the content of other ones.
```z80
--8<-- "cpclib-basm/tests/asm/good_labels_generated.asm"
```

### Proximity labels

`BASM` supports proximity labels (also known as anonymous labels), a feature inspired by `rasm` and `spasm-ng` assemblers.

Proximity labels use the underscore `_` character as a reusable label name:

- `_` defines an anonymous label at the current position
- `_+` references the **next** `_` label (forward reference)
- `_-` references the **previous** `_` label (backward reference)

Each time you define a new `_` label, it becomes independent from previous ones. This is particularly useful for small local jumps where inventing unique label names would be cumbersome.

```z80
--8<-- "cpclib-basm/tests/asm/good_proximity_labels.asm"
```

!!! note "Important notes"

    - Using `_-` before defining any `_` label will cause an error
    - Using `_+` without a subsequent `_` label will cause an error
    - Proximity labels are completely independent from normal labels that contain underscores (e.g., `my_label`)
    - Each `_` definition increments an internal counter, making it distinct from previous `_` labels

## Instructions

Here is the list of instructions used to validate `BASM`:

```z80
--8<-- "cpclib-basm/tests/asm/good_all.asm"
```

## Variadic macros

A macro can accept extra, unnamed arguments beyond its declared parameters by ending its
parameter list with a trailing `...` (either `MACRO foo(a, b, ...)` or the comma-separated
`MACRO foo, a, b, ...` form):

- Named parameters keep working exactly as before, referenced as `{a}`/`{b}`.
- Extra arguments are referenced positionally, continuing the index past the named ones - the
  first extra is `{2}`, the second `{3}`, and so on (0-based across *all* arguments, named or
  not).
- `{#}` expands to the total number of arguments actually passed at a given call - named plus
  extra combined.
- A variadic macro still requires at least its named arguments; only *more* than that is
  accepted. A macro without a trailing `...` is unaffected: extra arguments are still a hard
  arity error, exactly as before.

```z80
MACRO sum3(a, b, ...)
    ; {a}/{b} are named; {2} is the first extra argument (if this call
    ; passed one) and {#} is the total number of arguments passed
    db {a}, {b}, {#}
ENDM

sum3 1, 2        ; -> db 1, 2, 2   ({#}=2, no extra argument)
sum3 1, 2, 3     ; -> db 1, 2, 3   ({#}=3, {2} would be 3 if referenced)
```

Referencing an extra index a particular call doesn't actually provide (e.g. the body uses `{2}`
but that call only passed one argument) is an assembling error, not a fallback to `0`/empty -
the same way an out-of-range `string_format` placeholder is.

## Fake instructions

To ease coding, several fake instructions are allowed by `BASM`. It replaces them by the combination of true instructions.

Here is a subset of the possibilities.

### JQ

`JQ` (optionally with a flag test, e.g. `JQ NZ, target`) is a fake jump: `BASM` first tries to
assemble it as a `JR` (2 bytes); if the target turns out to be too far away for a relative jump
(more than 127 bytes forward or 128 bytes backward), it falls back to a `JP` (3 bytes) instead.
There is no reachability analysis beyond that simple try/fallback - use `JR`/`JP` directly if you
need a guaranteed encoding.

```z80
    jq near_or_far_target
near_or_far_target:
    nop
```

!!! failure Inacurate documentation

    Most accepted fake instructions are missing from the listing

```z80
--8<-- "cpclib-basm/tests/asm/good_fake_instructions.asm"
```

## Comments

### One line comment

```
; This is a comment
```

### Multiline comment
```
/*
 this is 
 another
 comment */


## Expressions

### Types

- int
- char, string
- list, matrix

### Filenames

A normal file is represented by a string.
```
"standard.filename"
```

A file insided a disk is represented in a string that contains the dsk name, followed by `#` then the file of interest within the dsk

```
"image.dsk#filename"
```

## Special variables

 - $: get the current code address
 - $$: get the current output address


Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_dollar.asm"
```

## Provided functions

!!! failure Inacurate documentation

    Need to document all functions

### Z80 related functions

#### assemble


`assemble(str)` consider the string `str` to be a list of instructions (no directives) and returns the list of bytes corresponding to the assembled version of the given string.



```z80
--8<-- "cpclib-basm/tests/asm/good_assemble.asm"
```

#### duration

- `duration(instruction)` returns the number of nop of the instruction

#### opcode

```z80
--8<-- "cpclib-basm/tests/asm/good_opcode.asm"
```

### Amstrad CPC video handling

- mode0_byte_to_pen_at
- mode1_byte_to_pen_at
- mode2_byte_to_pen_at
- pen_at_mode0_byte
- pen_at_mode1_byte
- pen_at_mode2_byte
- pens_to_mode0_byte
- pens_to_mode1_byte
- pens_to_mode2_byte


```z80
--8<-- "cpclib-basm/tests/asm/good_`pixels`.asm"
```

### List handling

- list_new
- list_get(LIST, INDEX)
- list_set
- list_len
- `list_sublist(list, start, end)` -> list: Return a new list from start until end not included 
- list_sort
- list_argsort
- list_push

### String handling
- string_new
- string_push
- string_concat
- string_from_list

### Matrix handling

- matrix_new
- matrix_set
- matrix_get
- matrix_col
- matrix_row
- matrix_set_row
- matrix_set_col
- matrix_width
- matrix_height

### File handing

- `load(fname) -> list of bytes`: return the bytes from the given file name


### Memory handling

#### memory(addr)


```z80
--8<-- "cpclib-basm/tests/asm/good_memory.asm"
```


## User defined functions


`BASM` allows to define functions that can be used in any expression.
The functions are fed with parameters and execute conditional directives as well as directives able to handle variables.
They finish at the execution of the `RETURN` directive.

```
FUNCTION [ARG1 [, ARGN]]
    INSTRUCTIONS
    RETURN VALUE
ENDFUNCTION
```

!!! failure Inacurate documentation

    Better explain how to build function


Example of the fibonacci function:

```z80
--8<-- "cpclib-basm/tests/asm/good_fibonacci.asm"
```

Example of function to handle lists:
```z80
--8<-- "cpclib-basm/tests/asm/good_function_load.asm"
```

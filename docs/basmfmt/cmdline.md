# BASM-FMT Command Line Reference

## Synopsis

```bash
basm-fmt [OPTIONS] [FILES]...
```

## Description

`basm-fmt` reformats Z80 assembly source files understood by BASM. Given no `FILES`, it
reads a single source from stdin and writes the formatted result to stdout. `-` as a file
argument also means stdin.

## Arguments

### `[FILES]...`
Source files to format. Use `-` to read from stdin (mixed with real paths is fine - stdin
is read wherever `-` appears in the list).

## Run Modes

### `-i, --inplace`
Rewrite each file in place instead of writing the formatted result to stdout. A file that
is already correctly formatted is left untouched (not rewritten).

### `-c, --check`
Don't write anything; exit with a non-zero code if any input would be reformatted. Prints
`<path>: would be reformatted` (or `<stdin>: would be reformatted`) to stderr for each file
that differs. Intended for CI.

### `--diff`
Print a diff of what would change for each file that differs, instead of the formatted
output or an in-place rewrite. Exits non-zero when anything differs, the same as
`--check`.

### `--lines <START:END>`
Format only lines `START:END` (1-based, inclusive); everything else in the file is passed
through byte-for-byte unchanged - the same mechanism `; fmt: off`/`; fmt: on` uses
internally. Works with a single file or with stdin; refused (exit code 2) when more than
one file is given, since a single range can't unambiguously apply to several files.

## Formatting Options

Every option below can also be set in a [`basm-fmt.toml` config file](#configuration-file);
a flag given on the command line always overrides the config file's value for that option.

### `--indent-size <N>`
Spaces per indentation level. Default: `4`.

### `--comment-column <N>`
Minimum column (0-indexed) at which trailing comments start. Default: `30`.

### `--mnemonic-case <uppercase|lowercase|untouched>`
Case applied to Z80 mnemonic keywords (`LD`, `PUSH`, ...). Default: `uppercase`.

### `--directive-case <uppercase|lowercase|untouched>`
Case applied to directive keywords (`ORG`, `EQU`, `REPEAT`, ...). Default: `uppercase`.

### `--register-case <uppercase|lowercase|untouched>`
Case applied to Z80 register names in operands. Default: `uppercase`.

### `--one-instruction-per-line <true|false>`
Split multiple instructions sharing one physical source line (`nop:nop:nop`) onto separate
output lines. Default: `true`.

### `--space-around-column <none|before|after|both|untouched>`
Spacing around `:` instruction separators. Only has an effect when
`--one-instruction-per-line=false` (with the default `true`, every separator becomes a
newline instead). Default: `untouched`.

### `--space-around-assignment <none|before|after|both|untouched>`
Spacing around assignment operators (`=`, `+=`, `>>=`, ...). Default: `untouched`.

### `--space-around-comma <none|before|after|both|untouched>`
Spacing around `,` inside an instruction's operand list, a `DB`/`DW` data list, or a
macro/struct call's arguments (`ld a,b` vs `ld a, b`; `list_new(2,-1)` vs
`list_new(2, -1)`). A comma inside a string literal (`db "a,b"`) is never touched.
Default: `untouched`.

### `--quote-style <single|double|untouched>`
Quote character used for string literals (`'x'` vs `"x"`). A literal whose own content
contains the target quote character is left exactly as written rather than corrupted -
converting `"it's a test"` to single quotes would need an escape this format has no syntax
for. Default: `untouched`.

### `--max-consecutive-blank-lines <N>`
Collapse a run of more than `N` consecutive blank lines down to exactly `N`. Unset by
default: blank lines are preserved exactly as written, however many there are.

### `--hexadecimal-case <uppercase|lowercase|untouched>`
Case applied to the `A`-`F` letters inside hexadecimal literals. Default: `untouched`.

### `--hexadecimal-encoding <0x|0X|#|$|&|h|H|untouched>`
Prefix or suffix form used when reformatting hexadecimal literals. Default: `untouched`.

### `--octal-encoding <0o|0O|@|untouched>`
Prefix form used when reformatting octal literals. Default: `untouched`.

### `--binary-encoding <0b|0B|%|untouched>`
Prefix form used when reformatting binary literals. Default: `untouched`.

### `--label-definition-postfix-with-column <no-column|with-column|untouched>`
Whether label definitions are emitted with (`myloop:`) or without (`myloop`) a trailing
`:`. Default: `with-column`.

### `-h, --help`
Print help.

## The `; fmt: off` / `; fmt: on` Pragma

A region between a `; fmt: off` comment and the next `; fmt: on` passes through completely
unchanged - case, spacing, indentation, comments, all of it. Recognized case/whitespace-
flexibly (`;fmt:off`, `; FMT: OFF`, `;  fmt : off` all match). An unclosed `; fmt: off`
disables formatting for the rest of the file, the same "you forgot to turn it back on"
behavior other formatters with this feature have. Use it for hand-laid-out code a
formatter has no way to infer the intent of: a data table aligned by eye, or a
`FONT_CREATE_CHAR` call whose string arguments are shaped like the glyph they encode.

```z80
; fmt: off
FONT_CREATE_CHAR( dot,
    "..",
    "##" )
; fmt: on
```

## Configuration File

`basm-fmt` searches for `basm-fmt.toml` starting from the current directory, walking up to
the filesystem root, then in `$XDG_CONFIG_HOME/basm-fmt/`. When found, the file is loaded
first as the base configuration; flags given on the command line override individual
options from the file, and omitting a flag keeps the config file's value for that option.

The file's keys are the same names as the long-flag options above, in `snake_case`:

```toml
indent_size = 2
mnemonic_case = "lowercase"
space_around_comma = "after"
quote_style = "double"
max_consecutive_blank_lines = 1

# Paths relative to this file's own directory; matching files are excluded from a
# run entirely. No flag equivalent - config file only, the same convention
# rustfmt.toml itself uses.
ignore = ["vendor/**", "generated/*.asm"]
```

## See Also

- [Overview](index.md) - Features and quick start
- [Examples](examples.md) - Usage examples and common workflows
- [BASM](../basm/index.md) - The assembler this formatter targets

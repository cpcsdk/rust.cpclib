# BASM-FMT - Z80 Assembly Formatter

`basm-fmt` (crate `cpclib-asmfmt`) is a source code formatter for the Z80 assembly dialect
understood by [BASM](../basm/index.md) - the same idea as `rustfmt`, `clang-format` or
`prettier`, applied to Z80 asm.

It reformats indentation, mnemonic/directive/register case, comment alignment, numeric
literal bases, and optionally spacing around `:`, `=` and `,`, while leaving the actual
assembled output unchanged. It parses source with the real `cpclib-asm` parser rather than
guessing from text, so it never misreads a `NOP:NOP:NOP:NOP` chain, a numeric operand
butting against a `:`, or a macro body full of `{param}` placeholders.

## Features

- Case control for mnemonics, directives, registers, and hex digits, independently
- Numeric literal reformatting (hex/octal/binary prefix or suffix style)
- One-instruction-per-line splitting (or keep multiple instructions per physical line)
- Comment-column alignment
- Spacing control around `:`, `=`/`+=`/... and `,`
- Quote-style normalization (`'x'` vs `"x"`), skipping a literal whose own content would
  need an escape this format has no syntax for
- Blank-line collapsing (`--max-consecutive-blank-lines`)
- MACRO bodies are formatted like any other code (falls back to verbatim only when a body
  doesn't parse standalone, e.g. some `{param}` placeholder shapes)
- `; fmt: off` / `; fmt: on` - an escape hatch for hand-laid-out code (data tables, a
  `FONT_CREATE_CHAR` call whose string arguments are shaped like the glyph they encode),
  the same idea as `#[rustfmt::skip]` or `// clang-format off`
- `--lines START:END` - format only a line range, leaving the rest of the file untouched
- `--diff` / `--check` - preview or CI-check formatting without rewriting files
- A `basm-fmt.toml` config file, with CLI flags overriding individual options, and a
  top-level `ignore = [...]` list to exclude paths from a run entirely

## Quick Start

```bash
# Format a file to stdout
basm-fmt game.asm

# Rewrite files in place
basm-fmt -i src/*.asm

# Check formatting in CI, without writing anything
basm-fmt --check src/*.asm

# Preview what would change
basm-fmt --diff game.asm

# Read from stdin, write to stdout
cat game.asm | basm-fmt -
```

For the full option list, see the [Command Line Reference](cmdline.md), or run
`basm-fmt --help`.

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all command-line options
- [Examples](examples.md) - Usage examples and common workflows
- [BASM](../basm/index.md) - The assembler this formatter targets

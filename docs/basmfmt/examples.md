# BASM-FMT Examples

## Basic Formatting

Given:

```z80
org 0x4000
myloop:
  push af
  ld a,b
  pop af
  ret
```

```bash
basm-fmt game.asm
```

produces:

```z80
    ORG 0x4000
myloop:
    PUSH AF
    LD A,B
    POP AF
    RET
```

## Rewriting Files In Place

```bash
basm-fmt -i src/*.asm
```

Each file that would change is rewritten; a file that's already correctly formatted is
left untouched (not touched on disk at all, so timestamps/watchers aren't disturbed for
no reason).

## Checking Formatting in CI

```bash
basm-fmt --check src/*.asm
```

Writes nothing. Exits `0` if every file is already formatted, or `1` and prints
`<path>: would be reformatted` for each file that isn't - a typical CI gate.

## Previewing Changes

```bash
echo 'push   af' | basm-fmt --diff -
```

```diff
 push   af
     PUSH AF
```

(shown here without the terminal color codes `basm-fmt` actually prints)

## Formatting Only a Line Range

Useful for formatting just the lines a patch touched, leaving the rest of a large file
exactly as it was:

```bash
basm-fmt --lines 2:2 game.asm
```

Given:

```z80
org 0x4000
push   af
pop  bc
```

only line 2 changes:

```z80
org 0x4000
    PUSH AF
pop  bc
```

## Case Style

```bash
basm-fmt --mnemonic-case lowercase --directive-case lowercase --register-case lowercase game.asm
```

`--mnemonic-case`, `--directive-case` and `--register-case` are independent: setting only
the first two leaves register names (`AF`, `HL`, ...) at their default (uppercase).

## Numeric Literal Style

```bash
basm-fmt --hexadecimal-encoding 0x --hexadecimal-case uppercase game.asm
```

turns `&4000` / `0FFFFh` into `0x4000` / `0xFFFF`.

## Comma and Quote Style

```bash
basm-fmt --space-around-comma after --quote-style single game.asm
```

turns `db 1,2,3` into `DB 1, 2, 3`, and `db "hello"` into `DB 'hello'` (a literal that
contains the target quote character, like `"it's here"`, is left untouched rather than
converted into something that would need an escape).

## Collapsing Blank Lines

```bash
basm-fmt --max-consecutive-blank-lines 1 game.asm
```

collapses any run of more than one consecutive blank line down to exactly one.

## Preserving Hand-Laid-Out Code

```z80
; fmt: off
FONT_CREATE_CHAR( dot,
    "..",
    "##" )
; fmt: on
```

The region between the markers is passed through completely unchanged; code before and
after it is still formatted normally.

## Using a Config File

`basm-fmt.toml`, in the project root or any ancestor directory:

```toml
indent_size = 2
space_around_comma = "after"
quote_style = "double"
ignore = ["vendor/**"]
```

```bash
basm-fmt src/*.asm   # picks up basm-fmt.toml automatically
basm-fmt --indent-size 4 src/*.asm   # the flag wins over the config file for this run
```

## See Also

- [Overview](index.md) - Features and quick start
- [Command Line Reference](cmdline.md) - Complete option reference
- [BASM](../basm/index.md) - The assembler this formatter targets

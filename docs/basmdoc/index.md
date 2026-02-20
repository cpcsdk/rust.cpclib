`basmdoc` aims at generating a HTML page that represents the documentation of a z80 assembler project.
As z80 source could we written on oldschool platform we have chosen to no use a verbose way to comment (such as `@param`).
Maybe we'll change that in the future if there are no users in for native code.

## Integration with BndBuild

Basmdoc is available as a standalone `basmdoc` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `basmdoc` or `doc` command aliases. See [BndBuild Commands](../bndbuild/commands.md#development-basmdoc-basmodocdoc) for integration details.

## Usage

```bash
basmdoc [OPTIONS] --output <OUTPUT> <INPUT>...
```

### Arguments

- `<INPUT>...` — Input assembly file(s) or folder(s) (recursively searches for `.asm` files in folders)

### Options

- `-o, --output <OUTPUT>` — Output markdown file (required)
- `-w, --wildcards` — Enable wildcard expansion on input files
- `-u, --undocumented` — Include all undocumented symbols (macros, functions, labels, equs)
- `--undocumented-macros` — Include undocumented macros
- `--undocumented-functions` — Include undocumented functions
- `--undocumented-labels` — Include undocumented labels
- `--undocumented-equs` — Include undocumented equs
- `-t, --title <TITLE>` — Output title
- `--no-minify` — Disable HTML minification (enabled by default)
- `-h, --help` — Print help

## Comment Syntax

There are two kinds of comments:

- `;;;` represents a file comment
- `;;` represents a standard comment and serves to comment a following z80 token. Documentable tokens are:

   - labels
   - EQU
   - macro definitions
   - function definitions
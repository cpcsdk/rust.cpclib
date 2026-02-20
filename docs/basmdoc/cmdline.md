# BASMDOC Command Line Reference

## Synopsis

```bash
basmdoc [OPTIONS] --output <OUTPUT> <INPUT>...
```

## Description

BASMDOC generates HTML documentation from Z80 assembly source files by extracting structured comments.

## Arguments

### `<INPUT>...`
One or more assembly source files (.asm) or folders. When folders are provided, recursively searches for .asm files.

## Options

### `-o, --output <OUTPUT>` (required)
Output file path for generated HTML documentation.

### `-w, --wildcards`
Enable wildcard expansion on input files.

### `-u, --undocumented`
Include all undocumented symbols (macros, functions, labels, equs).

### `--undocumented-macros`
Include undocumented macros in the output.

### `--undocumented-functions`
Include undocumented functions in the output.

### `--undocumented-labels`
Include undocumented labels in the output.

### `--undocumented-equs`
Include undocumented equs in the output.

### `-t, --title <TITLE>`
Set documentation title.

### `--no-minify`
Disable HTML minification (minification is enabled by default).

### `-h, --help`
Print help information.

## See Also


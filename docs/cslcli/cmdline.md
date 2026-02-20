# CSLCLI Command Line Reference

## Synopsis

```bash
cslcli [OPTIONS] <FILE>
```

## Description

CSLCLI parses and validates CSL (CPC Script Language) files, providing rich error reporting and syntax validation.

## Arguments

### `<FILE>`
Path to the CSL script file to parse.

## Options

### `-v, --verbose`
Enable verbose output. Shows:
- Number of instructions parsed
- Detailed parsing information
- Debug output

Example:
```bash
cslcli -v test_script.csl
```

### `-h, --help`
Print help information.

### `-V, --version`
Print version information and author credits.

## Output

### Success
When parsing succeeds:
- Parsed script is written to stdout
- Exit code 0
- With `-v`: instruction count to stderr

### Error
When parsing fails:
- Rich error message to stderr showing:
  - File name and location
  - Line and column number
  - Error description
  - Code snippet with error marker
- Exit code 1

## Error Format

Errors are reported in a readable format:

```
Error parsing script.csl at line 10, column 5:
   |
10 | WAIT invalid_arg
   |      ^^^^^^^^^^^ unexpected token
   |
Expected: number or time duration
```

## Exit Status

- `0` - Successfully parsed and validated
- `1` - Parse error or file not found

## Examples

See [Examples](examples.md) for detailed usage.

## See Also

- [CSL Language Reference](language.md) - CSL syntax
- [Examples](examples.md) - Common usage patterns

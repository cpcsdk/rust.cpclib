# Hideur Command Line Reference

!!! info "Latest Help"
    For the most current options: `hideur --help`

## Usage

```bash
hideur [OPTIONS] <INPUT>
```

## Arguments

### `<INPUT>`
Input file to manipulate (required).

## Options

### `--info`
Display information about the file's AMSDOS header without modifying it.

When this option is used:
- No output file is created
- The tool displays header metadata (type, addresses, user, etc.)
- Other modification options are ignored

### `-o, --output <OUTPUT>`
Output file to generate.

Required unless `--info` is specified.

### `-u, --user <USER>`
User number where to place the file (0-15).

Default: 0

### `-t, --type <TYPE>`
File type to set in the header.

**Required** unless `--info` is specified.

Accepted values (case-insensitive):
- `0`, `Basic`, `BASIC`, `basic` - Basic program
- `1`, `Protected`, `PROTECTED`, `protected` - Protected Basic
- `2`, `Binary`, `BINARY`, `binary` - Binary file

### `-l, --load <LOAD>`
Loading address for the file (hexadecimal format supported: `#4000` or `0x4000`).

**Required** when type is Binary.

### `-x, --execution <EXEC>`
Execution address (hexadecimal format supported: `#4000` or `0x4000`).

Defaults to the load address if not specified.

## Examples

See [Examples](examples.md) for practical usage scenarios.

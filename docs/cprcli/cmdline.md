# CPRCLI Command Line Reference

## Synopsis

```bash
cprcli --cpr1 <FILE> [OPTIONS]
```

## Description

CPRCLI provides tools for analyzing CPR cartridge files used with Amstrad CPC Plus computers.

## Required Arguments

### `--cpr1 <FILE>`
The CPR cartridge file to analyze.

## Options

### `--info`
Display information about the cartridge, including:
- Number of banks present
- Bank sizes
- Total cartridge size
- Bank organization

Example:
```bash
cprcli --cpr1 game.cpr --info
```

### `--dump`
Output the raw memory contents of the selected banks. By default, dumps all banks. Use `--bank` to select specific banks.

Example:
```bash
cprcli --cpr1 game.cpr --dump > cartridge_dump.bin
```

### `--bank <BANKS>`
Select specific banks to work with. Accepts:
- Single bank: `--bank 0`
- Multiple banks: `--bank 0,1,2`  
- Comma-separated list

Valid bank numbers: 0-31

Example:
```bash
cprcli --cpr1 game.cpr --bank 0,1 --dump
```

### `--cpr2 <FILE>`
Second CPR file for comparison. When specified with `--info`, shows differences between the two cartridges.

Example:
```bash
cprcli --cpr1 version1.cpr --cpr2 version2.cpr --info
```

### `-h, --help`
Print help information.

### `-V, --version`
Print version information.

## CPR Format

CPR files contain ROM banks for CPC Plus cartridges:
- Each bank: 16,384 bytes (16KB)
- Maximum banks: 32
- Maximum cartridge size: 512KB
- Banks are mapped to CPC memory on cartridge activation

## Exit Status

- `0` - Success
- `1` - Error (file not found, invalid CPR format)

## Examples

See [Examples](examples.md) for detailed usage.

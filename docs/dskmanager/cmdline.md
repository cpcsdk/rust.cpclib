# DSKManager Command Line Reference

## Synopsis

```
dskmanager [OPTIONS] <COMMAND> [ARGS...]
```

## Commands

### format
Format a new DSK disc image.

```bash
dskmanager format <output.dsk> [OPTIONS]
```

### add
Add files to a DSK image.

```bash
dskmanager add <disk.dsk> <file1> [file2...]
```

### catalog / cat
List the contents of a DSK image.

```bash
dskmanager catalog <disk.dsk>
```

### extract
Extract files from a DSK image.

```bash
dskmanager extract <disk.dsk> [output_dir]
```

## Options

### Global Options

- `-h, --help` - Show help information
- `-V, --version` - Show version information
- `-v, --verbose` - Enable verbose output

## Examples

See the [Examples](examples.md) page for detailed usage scenarios.

## Notes

- DSKManager works with standard Amstrad CPC DSK formats
- Supports both Data and System formats
- Preserves AMSDOS headers when present
- Can work with extended DSK formats

## Related Commands

- `catalog` - For viewing and creating CatArt catalogs
- `hideur` - For managing AMSDOS headers

*Note: Complete command-line documentation will be generated from dskmanager --help output.*

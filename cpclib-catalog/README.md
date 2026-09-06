# cpclib-catalog

Amsdos catalog manipulation tool for Amstrad CPC disk images.

## Features

- List catalog entries from DSK/HFE disk images or raw catalog files
- Display catalog using CatArt rendering (simulated CPC screen output)
- Modify catalog entries (readonly/system flags, user, filename, blocs, etc.)

## Usage

```bash
# List catalog entries (simple format)
catalog --input disk.dsk --list

# List all entries including those with control characters
catalog --input disk.dsk --listall

# Display catalog using CatArt rendering (simulated CPC screen)
catalog --input disk.dsk --cat

# Modify an entry
catalog --input disk.dsk --entry 0 --readonly --filename "NEWNAME.BIN"
```

## Options

- `-i, --input <FILE>`: Input/Output file (binary catalog or disk image)
- `-l, --list`: List catalog entries (no control chars)
- `-a, --listall`: List all entries (including control chars)
- `--cat`: Display catalog using CatArt rendering
- `--entry <N>`: Select entry to modify (0-63)
- `--readonly`, `--noreadonly`: Set/unset readonly flag
- `--system`, `--nosystem`: Set/unset system (hidden) flag
- `--user <N>`: Set user number
- `--filename <NAME>`: Set filename
- `--blocs <N...>`: Set block numbers
- `--numpage <N>`: Set page number
- `--size <N>`: Force entry size

## Dependencies

This crate depends on:
- `cpclib-disc`: Disk image handling
- `cpclib-catart`: CatArt rendering engine

It was extracted from `cpclib-disc` to avoid cyclic dependencies in the workspace.

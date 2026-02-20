# Catalog Command Line Reference

## Synopsis

```bash
catalog [INPUT_FILE] <COMMAND>
```

## Description

Catalog is a tool for creating, viewing, and manipulating CatArt - visually enhanced catalog displays embedded in BASIC programs for Amstrad CPC disk images.

## Arguments

- `[INPUT_FILE]` - Input file containing catalog entries (a binary file or a DSK). For the `build` command, this is the BASIC file if not specified in the command options.

## Commands

### `cat`
Display the catalog using CatArt rendering (sorted alphabetically).

```bash
catalog game.dsk cat
```

**Options:**
- `--png <PNG_OUTPUT>` - Optional PNG file to save pixel-accurate rendering of the catart
- `--locale <LOCALE>` - Font locale to use when generating PNG: `english`, `french`, `spanish`, `german`, `danish` [default: `english`]
- `--mode <MODE>` - Screen mode to use for catart rendering: 0, 1, 2, or 3 [default: `1`]
- `-h, --help` - Print help

**Examples:**
```bash
# Display catalog with CatArt rendering
catalog game.dsk cat

# Generate PNG image of the catalog
catalog game.dsk cat --png catalog.png

# Generate PNG with French locale and mode 0
catalog game.dsk cat --png catalog.png --locale french --mode 0
```

---

### `dir`
Display the catalog using CatArt rendering (directory order, unsorted).

```bash
catalog game.dsk dir
```

**Options:**
- `--png <PNG_OUTPUT>` - Optional PNG file to save pixel-accurate rendering
- `--locale <LOCALE>` - Font locale [default: `english`]
- `--mode <MODE>` - Screen mode (0-3) [default: `1`]
- `-h, --help` - Print help

**Examples:**
```bash
# Display directory with CatArt rendering
catalog game.dsk dir

# Generate PNG image of the directory
catalog game.dsk dir --png directory.png

# Generate PNG with specific mode
catalog game.dsk dir --png directory.png --mode 2
```

---

### `list`
List the content of the catalog **only** for files having no control characters.

```bash
catalog game.dsk list
```

**Options:**
- `-h, --help` - Print help

**Example:**
```bash
catalog game.dsk list
```

**Output (text):**
```
LOADER.BAS
GAME.BIN
MUSIC.BIN
```

---

### `listall`
List the content of the catalog **even** for files having control characters.

```bash
catalog game.dsk listall
```

**Options:**
- `-h, --help` - Print help

**Example:**
```bash
catalog game.dsk listall
```

---

### `build`
Build a catart from a BASIC program. Output will be a DSK/HFE file if the output filename ends with `.dsk` or `.hfe`, otherwise a raw 2048-byte catalog binary.

```bash
catalog game.dsk build
```

**Arguments:**
- `[BASIC_FILE]` - BASIC file to convert to catart (optional if `INPUT_FILE` is provided at top level)

**Options:**
- `-o, --output <OUTPUT_FILE>` - Output file (defaults to `catart.dsk`). Use `.dsk` or `.hfe` extension for disc images, otherwise creates raw binary
- `--png <PNG_OUTPUT>` - Optional PNG file to save pixel-accurate rendering of the catart
- `--locale <LOCALE>` - Font locale for PNG generation [default: `english`]
- `--mode <MODE>` - Screen mode (0-3) [default: `1`]
- `-h, --help` - Print help

**Example:**
```bash
# Build DSK from BASIC
catalog build loader.bas -o game.dsk

# Build with INPUT_FILE argument
catalog loader.bas build -o game.dsk

# Build raw binary
catalog build menu.bas -o catalog.bin

# Build HFE image
catalog build loader.bas -o game.hfe

# Build with PNG export
catalog build loader.bas -o game.dsk --png preview.png --locale french
```

---

### `decode`
Extract the BASIC listing from the input DSK. If no `--output` is provided, the listing is printed to standard output; otherwise, it is saved in the provided filename.

```bash
catalog game.dsk decode
```

**Options:**
- `-o, --output <OUTPUT_FILE>` - Optional output file for the decoded BASIC listing. If not provided, prints to stdout
- `-h, --help` - Print help

**Example:**
```bash
# Decode to stdout
catalog game.dsk decode

# Decode to file
catalog game.dsk decode -o restored.bas
```

---

### `modify`
Modify an entry in the catalog.

```bash
catalog game.dsk modify [OPTIONS] --entry <ENTRY>
```

**Required:**
- `--entry <ENTRY>` - Selects the entry to modify (entry index number)

**Options:**
- `--readonly` - Set the selected entry readonly
- `--system` - Set the selected entry hidden
- `--noreadonly` - Set the selected entry read and write
- `--nosystem` - Set the selected entry visible
- `--user <USER>` - Set the user value
- `--filename <FILENAME>` - Set the filename of the entry
- `--blocs [<BLOCS>...]` - Set the blocks to load (and update the number of blocks accordingly)
- `--numpage <NUMPAGE>` - Set the page number
- `--size <SIZE>` - Force the size of the entry
- `-h, --help` - Print help

**Example:**
```bash
# Make entry 0 readonly
catalog game.dsk modify --entry 0 --readonly

# Hide entry 2
catalog game.dsk modify --entry 2 --system

# Rename entry 1
catalog game.dsk modify --entry 1 --filename NEWNAME.BIN

# Change user number
catalog game.dsk modify --entry 3 --user 5

# Modify blocks
catalog game.dsk modify --entry 0 --blocs 0 1 2 3

# Make entry visible and writable
catalog game.dsk modify --entry 2 --nosystem --noreadonly
```

---

### `debug`
Debug catart by displaying each entry's bytes and corresponding BASIC commands.

```bash
catalog game.dsk debug
```

**Options:**
- `--cat` - Display entries in catalog (sorted alphabetically) order
- `--dir` - Display entries in directory (unsorted) order
- `-h, --help` - Print help

**Example:**
```bash
# Debug in catalog order
catalog game.dsk debug --cat

# Debug in directory order
catalog game.dsk debug --dir
```

**Output:**
Shows raw bytes and decoded BASIC commands for each catalog entry.

---

## Screen Modes

| Mode | Resolution | Colors | Typical Use |
|------|-----------|--------|-------------|
| 0 | 160×200 | 16 | Colorful catart displays |
| 1 | 320×200 | 4 | Standard catart (most common) |
| 2 | 640×200 | 2 | High-resolution text |
| 3 | 160×200 | 4 | Undocumented mode |

## Locales

Supported font locales for PNG rendering:
- `english` - English character set
- `french` - French character set (accented characters)
- `spanish` - Spanish character set
- `german` - German character set (umlauts)
- `danish` - Danish character set

## File Formats

### Input Formats
- `.bas` - Amstrad BASIC program (for `build` command)
- `.dsk` - Standard DSK disk image
- `.hfe` - HxC Floppy Emulator disk image
- Raw binary - 2048-byte catalog data

### Output Formats
- `.dsk` - Standard DSK disk image (when output filename ends with `.dsk`)
- `.hfe` - HFE disk image (when output filename ends with `.hfe`)
- Raw binary - 2048-byte catalog data (for other extensions)
- `.png` - PNG image (via `--png` option)

## Exit Status

- `0` - Success
- Non-zero - Error occurred

## See Also

- [Examples](examples.md) - Usage examples and workflows
- [Index](index.md) - Tool overview

## Examples

See [Examples](examples.md) for comprehensive workflows.

## See Also

- [Amsdos Format Documentation](amsdos.md)
- [Examples](examples.md)
- DSKManager for low-level disk operations

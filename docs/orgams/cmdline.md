# Orgams Command Line Reference

The Orgams integration is accessed through the `orgams` subcommand of `cpclib-runner`.

## Basic Syntax

```bash
cpclib-runner orgams --from <DATA_SOURCE> --src <SRC> [OPTIONS]
```

## Required Arguments

### `--from <DATA_SOURCE>` (alias: `-f`)

Specifies where the source files are located.

**Options:**

- **Disc Image**: Path to a DSK file (e.g., `project.dsk`)
- **Albireo Folder**: Path to a directory (e.g., `./cpcfolder`)

**Examples:**

```bash
# Use a disc image
cpclib-runner orgams --from demo.dsk --src MAIN.O

# Use Albireo folder (faster)
cpclib-runner orgams --from ./cpcfiles --src MAIN.O
```

### `--src <SRC>` (alias: `-s`)

The filename to assemble or edit within the data source.

**Format:**

- Must be a valid Amstrad CPC filename (8.3 format)
- Case-insensitive
- Extension typically `.O` for Orgams source files

**Examples:**

```bash
--src MAIN.O      # Orgams source file
--src SPRITE.O    # Another source file
--src CODE        # File without extension
```

## Output Options

### `--dst <DST>` (alias: `-d`)

Specify the output filename for the assembled binary.

**Default**: Uses the filename provided by Orgams during assembly

**Examples:**

```bash
# Save as PROGRAM
cpclib-runner orgams --from demo.dsk --src MAIN.O --dst PROGRAM

# Default behavior (Orgams chooses name)
cpclib-runner orgams --from demo.dsk --src MAIN.O
```

## Operation Modes

### `--edit` (alias: `-e`)

Launch the Orgams editor to modify the source file.

**Behavior:**

- Opens Orgams editor in emulator
- Loads specified source file
- Emulator remains open for interactive editing

**Example:**

```bash
# Edit MAIN.O in Orgams editor
cpclib-runner orgams --from demo.dsk --src MAIN.O --edit
```

### `--jump` (alias: `-j`)

Assemble the source and immediately execute it (without saving).

**Behavior:**

- Assembles the source file
- Jumps to the program entry point
- Does not save the binary to disc
- Useful for testing

**Example:**

```bash
# Assemble and run immediately
cpclib-runner orgams --from demo.dsk --src MAIN.O --jump
```

### Default Mode (Assemble and Save)

When neither `--edit` nor `--jump` is specified, Orgams assembles the source and saves the output.

**Example:**

```bash
# Assemble and save
cpclib-runner orgams --from demo.dsk --src MAIN.O --dst OUTPUT
```

## Format Conversion

### `--basm2orgams` (alias: `-b`)

Convert a BASM/ASCII format Z80 source file to Orgams binary format.

**Use Case:**

- Import modern assembly source into Orgams
- Convert cross-assembler code for native assembly
- Bridge BASM and Orgams workflows

**Example:**

```bash
# Convert BASM source to Orgams format
cpclib-runner orgams --from demo.dsk --src TEXT.ASM --basm2orgams

# Then assemble with Orgams
cpclib-runner orgams --from demo.dsk --src TEXT.ASM
```

!!! note "Conversion Limitations"
    Not all BASM directives may be compatible with Orgams. Manual adjustment of the converted source may be required.

## Emulator Options

Orgams integration uses the same emulator options as the main `cpclib-runner`. See [CPC Runner documentation](../runner/cmdline.md) for details.

### Common Emulator Flags

```bash
# Specify emulator
--emulator ace      # ACE-DL (default)
--emulator winape   # WinAPE

# Keep emulator open after operation
--keepemulator      # Don't close emulator

# Memory configuration
--memory 64         # CPC 464 (64KB)
--memory 128        # CPC 6128 (128KB)
```

## Complete Examples

### Basic Assembly

```bash
cpclib-runner orgams \
  --from project.dsk \
  --src MAIN.O \
  --dst MAIN
```

### Edit and Assemble Workflow

```bash
# Step 1: Edit source
cpclib-runner orgams --from project.dsk --src MAIN.O --edit

# Step 2: Assemble (after editing)
cpclib-runner orgams --from project.dsk --src MAIN.O --dst MAIN
```

### Quick Test Cycle

```bash
# Assemble and test immediately
cpclib-runner orgams \
  --from project.dsk \
  --src DEMO.O \
  --jump \
  --keepemulator
```

### Albireo Workflow

```bash
# Using folder instead of disc (faster)
cpclib-runner orgams \
  --from ./cpcfolder \
  --src CODE.O \
  --dst OUTPUT
```

### Import External Source

```bash
# Convert BASM source and assemble
cpclib-runner orgams \
  --from project.dsk \
  --src IMPORTED.ASM \
  --basm2orgams

cpclib-runner orgams \
  --from project.dsk \
  --src IMPORTED.ASM \
  --dst IMPORTED
```

## Exit Codes

- **0**: Successful operation
- **Non-zero**: Error occurred (check stderr for details)

## Environment Variables

### `CPCIP`

If set, used as default M4 address for `xfer` operations (not directly used by Orgams).

## See Also

- [Orgams Overview](index.md) - Introduction and features
- [Examples](examples.md) - Practical usage scenarios
- [CPC Runner Command Line](../runner/cmdline.md) - Additional emulator options

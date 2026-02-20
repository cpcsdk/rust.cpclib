# Orgams Usage Examples

Practical examples demonstrating Orgams integration in various workflows.

## Basic Workflows

### Simple Assembly

Assemble a single Orgams source file to a binary.

```bash
# Assemble MAIN.O to MAIN
cpclib-runner orgams --from demo.dsk --src MAIN.O --dst MAIN
```

**Use Case**: Convert Orgams source to executable binary.

---

### Edit and Assemble

Interactive workflow: edit source, then assemble.

```bash
# Step 1: Open Orgams editor
cpclib-runner orgams --from demo.dsk --src SPRITE.O --edit

# Step 2: After editing, assemble
cpclib-runner orgams --from demo.dsk --src SPRITE.O --dst SPRITE
```

**Use Case**: Modify existing Orgams source files.

---

### Quick Test Cycle

Assemble and immediately run without saving.

```bash
# Assemble and jump to program
cpclib-runner orgams --from demo.dsk --src TEST.O --jump --keepemulator
```

**Use Case**: Rapid prototyping and testing.

---

## Albireo Workflows

### Using Folders Instead of Discs

Albireo provides faster access by using host folders.

```bash
# Create folder structure
mkdir cpcfolder
# Place MAIN.O in cpcfolder/

# Assemble using folder
cpclib-runner orgams --from cpcfolder --src MAIN.O --dst MAIN
```

**Advantages:**

- Faster than disc image access
- Easier file management
- No disc image corruption risk

---

### Multi-File Project

Assemble multiple source files from an Albireo folder.

```bash
# Project structure:
# cpcfolder/
#   ├── MAIN.O
#   ├── SPRITES.O
#   └── MUSIC.O

# Assemble each component
cpclib-runner orgams --from cpcfolder --src MAIN.O --dst MAIN
cpclib-runner orgams --from cpcfolder --src SPRITES.O --dst SPRITES
cpclib-runner orgams --from cpcfolder --src MUSIC.O --dst MUSIC
```

**Use Case**: Modular project development.

---

## Build System Integration (bndbuild)

### Basic Orgams Build Rule

Integrate Orgams assembly into a bndbuild project.

**build.bnd:**

```yaml
- tgt: program
  dep: source/MAIN.O
  cmd: orgams --from source.dsk --src MAIN.O --dst program
```

**Usage:**

```bash
bndbuild program
```

---

### Complete Orgams Build File

Full example with editor integration and testing.

**build.bnd:**

```yaml
#!/usr/bin/env -S bndbuild -f

{% set FROM="cpcfolder" %}
{% set SRC="DEMO.O" %}
{% set DST="DEMO" %}

# Build target: assemble source
- tgt: build
  dep: {{FROM}}/{{DST}}

# Assemble source to binary
- tgt: {{FROM}}/{{DST}}
  dep: {{FROM}}/{{SRC}}
  cmd: orgams --from {{FROM}} --src {{SRC}} --dst {{DST}}

# Edit source in Orgams
- tgt: edit
  phony: true
  cmd: orgams --from {{FROM}} --src {{SRC}} --edit

# Test: assemble and run
- tgt: test
  phony: true
  cmd: orgams --from {{FROM}} --src {{SRC}} --jump --keepemulator

# Clean build outputs
- tgt: clean
  phony: true
  cmd: -rm {{FROM}}/{{DST}}
```

**Workflow:**

```bash
# Edit source
bndbuild edit

# Build binary
bndbuild build

# Quick test
bndbuild test

# Clean
bndbuild clean
```

---

### Orgams with Hardware Transfer

Build and deploy to real CPC via M4 board.

**build.bnd:**

```yaml
{% set FROM="cpcfolder" %}
{% set SRC="PROD.O" %}
{% set DST="PROD" %}

# Build and deploy to M4
- tgt: deploy
  dep: {{FROM}}/{{DST}}
  cmd: xfer 192.168.1.26 -y {{FROM}}/{{DST}}

# Assemble
- tgt: {{FROM}}/{{DST}}
  dep: {{FROM}}/{{SRC}}
  cmd: orgams --from {{FROM}} --src {{SRC}} --dst {{DST}}

# Edit source
- tgt: edit
  phony: true
  cmd: orgams --from {{FROM}} --src {{SRC}} --edit
```

**Usage:**

```bash
# Edit → Build → Deploy to hardware
bndbuild edit
bndbuild deploy
```

---

## Format Conversion

### Import BASM Source

Convert ASCII assembly to Orgams format.

```bash
# Step 1: Convert BASM source to Orgams format
cpclib-runner orgams --from project.dsk --src CODE.ASM --basm2orgams

# Step 2: Assemble with Orgams
cpclib-runner orgams --from project.dsk --src CODE.ASM --dst CODE
```

**Use Case**: Import cross-assembler code into native Orgams workflow.

---

### Extract Orgams Binary to ASCII

Use Borgams to convert compiled Orgams back to text.

```bash
# Convert Orgams binary to ASCII
cpclib-borgams --input COMPILED.O --output source.asm
```

**Note**: Borgams is currently work-in-progress. See [Borgams documentation](../borgams/).

---

## Advanced Scenarios

### Conditional Assembly with Emulator Selection

Choose emulator based on platform.

```bash
# Linux/macOS: use ACE
cpclib-runner orgams --from demo.dsk --src MAIN.O --emulator ace

# Windows: use WinAPE (if Orgams support improves)
cpclib-runner orgams --from demo.dsk --src MAIN.O --emulator winape
```

---

### Multi-Stage Build

Combine Orgams with other tools.

**build.bnd:**

```yaml
# Stage 1: Convert image assets
- tgt: assets/SPRITE.BIN
  dep: assets/sprite.png
  cmd: img2cpc --mode 1 --input assets/sprite.png --output assets/SPRITE.BIN

# Stage 2: Assemble with Orgams
- tgt: build/MAIN
  dep: 
    - source/MAIN.O
    - assets/SPRITE.BIN
  cmd: orgams --from build.dsk --src MAIN.O --dst MAIN

# Stage 3: Create disc image
- tgt: release.dsk
  dep: build/MAIN
  cmd: dsk release.dsk add --amsdos build/MAIN
```

---

## Debugging and Troubleshooting

### Keep Emulator Open for Inspection

Prevent automatic closure to examine results.

```bash
# Assembly fails? Keep emulator open to see errors
cpclib-runner orgams \
  --from demo.dsk \
  --src BROKEN.O \
  --keepemulator
```

---

### Verbose Output

Enable debug logging (implementation-specific).

```bash
# Check what commands are sent to the emulator
RUST_LOG=debug cpclib-runner orgams --from demo.dsk --src MAIN.O
```

---

## Performance Tips

### Use Albireo for Faster Iteration

Folders are significantly faster than disc images.

```bash
# SLOW: Disc image access
cpclib-runner orgams --from large.dsk --src CODE.O

# FAST: Folder access
cpclib-runner orgams --from cpcfolder --src CODE.O
```

### Reuse Disc Images

Keep source and output in the same disc to reduce I/O.

```bash
# Both source and destination on same disc
cpclib-runner orgams --from project.dsk --src SRC.O --dst OUTPUT
```

---

## Integration Examples

### Makefile Integration

```makefile
SRC = cpcfolder/MAIN.O
DST = cpcfolder/MAIN

build: $(DST)

$(DST): $(SRC)
	cpclib-runner orgams --from cpcfolder --src MAIN.O --dst MAIN

edit:
	cpclib-runner orgams --from cpcfolder --src MAIN.O --edit

test:
	cpclib-runner orgams --from cpcfolder --src MAIN.O --jump

clean:
	rm -f $(DST)

.PHONY: edit test clean
```

---

### Shell Script Automation

```bash
#!/bin/bash
# build_orgams.sh

set -e

FROM="cpcfolder"
SRC="DEMO.O"
DST="DEMO"

echo "Assembling $SRC..."
cpclib-runner orgams --from "$FROM" --src "$SRC" --dst "$DST"

echo "Assembly complete: $DST"
ls -lh "$FROM/$DST"
```

---

## See Also

- [Orgams Overview](index.md) - Introduction and features
- [Command Line Reference](cmdline.md) - Detailed option documentation
- [BNDBUILD Examples](../bndbuild/examples.md) - More build system patterns
- [CPC Runner Examples](../runner/examples.md) - Emulator automation patterns

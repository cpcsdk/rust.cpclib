# Locomotive Examples

## Basic Conversion

### Text to Binary
Convert a text BASIC program to binary format for use on CPC:

```bash
locomotive encode -i mygame.txt -o mygame.bas
```

With Amsdos header:
```bash
locomotive encode -i loader.txt -o LOADER.BAS --header
```

### Binary to Text
Convert a binary BASIC file to readable text:

```bash
locomotive decode -i MYGAME.BAS -o mygame.txt
```

View content directly (stdout):
```bash
locomotive decode -i GAME.BAS
```

## Development Workflow

### Version Control Friendly
Keep BASIC programs in text format for Git:

```bash
# 1. Write BASIC in text editor: game.txt
cat > game.txt << 'EOF'
10 MODE 1
20 PRINT "HELLO WORLD"
30 GOTO 20
EOF

# 2. Convert to binary
locomotive encode -i game.txt -o game.bas

# 3. Commit text version to Git
git add game.txt
git commit -m "Initial game code"
```

### Roundtrip Test
Verify encode/decode preserves your code:

```bash
# Original text
locomotive encode -i original.txt -o test.bas

# Decode back
locomotive decode -i test.bas -o decoded.txt

# Compare
diff original.txt decoded.txt
```

### Batch Convert to Text
Convert all binary BASIC files to text for archiving:

```bash
#!/bin/bash
# Convert all .bas files to .txt

for basfile in *.bas; do
  txtfile="${basfile%.bas}.txt"
  echo "Converting $basfile -> $txtfile"
  locomotive decode -i "$basfile" -o "$txtfile"
done
```

### Batch Convert from Text
Build binary BASIC files from text sources:

```bash
#!/bin/bash
# Convert all .txt files to .bas

for txtfile in src/*.txt; do
  basfile="build/$(basename "${txtfile%.txt}.bas")"
  echo "Building $txtfile -> $basfile"
  locomotive encode -i "$txtfile" -o "$basfile" --header
done
```

## Integration with Other Tools

### Working with DSK Images
Combine with **dsk** tool for disk operations:

```bash
# Extract BASIC from DSK, convert to text
dsk extract games.dsk LOADER.BAS -o loader.bas
locomotive decode -i loader.bas -o loader.txt

# Edit loader.txt in your editor...

# Convert back and add to DSK
locomotive encode -i loader.txt -o LOADER.BAS --header
dsk add games.dsk LOADER.BAS
```

### Build Complete Game Disk
Script to build a game disk from text sources:

```bash
#!/bin/bash
# build-disk.sh - Build game disk from source

# Create empty disk
dsk format game.dsk

# Convert and add all BASIC files
for src in src/*.txt; do
  base=$(basename "$src" .txt)
  basfile="build/${base}.BAS"
  
  echo "Processing $src..."
  locomotive encode -i "$src" -o "$basfile" --header
  dsk add game.dsk "$basfile"
done

# List disk contents
echo -e "\nDisk contents:"
dsk list game.dsk

echo "Game disk built successfully!"
```

### Automated Build System
Makefile for BASIC development:

```makefile
# Makefile for CPC BASIC project

SRC_DIR = src
BUILD_DIR = build
DISK = game.dsk

TXT_FILES = $(wildcard $(SRC_DIR)/*.txt)
BAS_FILES = $(patsubst $(SRC_DIR)/%.txt,$(BUILD_DIR)/%.BAS,$(TXT_FILES))

.PHONY: all disk clean

all: $(BAS_FILES)

$(BUILD_DIR)/%.BAS: $(SRC_DIR)/%.txt
	@mkdir -p $(BUILD_DIR)
	locomotive encode -i $< -o $@ --header

disk: $(BAS_FILES)
	dsk format $(DISK)
	@for f in $(BAS_FILES); do \
		dsk add $(DISK) $$f; \
	done
	@echo "Disk built successfully"
	dsk list $(DISK)

clean:
	rm -rf $(BUILD_DIR) $(DISK)
```

Usage:
```bash
make           # Convert all text to binary
make disk      # Build complete disk image
make clean     # Clean build artifacts
```

## Code Analysis

### Quick Inspection
View a BASIC file without creating output file:

```bash
locomotive decode -i MYSTERY.BAS
```

### Search for Patterns
Find all programs that use specific commands:

```bash
# Find all files that use GOSUB
for f in *.bas; do
  if locomotive decode -i "$f" | grep -q "GOSUB"; then
    echo "$f uses GOSUB"
  fi
done
```

### Extract Line Ranges
Get specific lines from a BASIC program:

```bash
# Show lines 100-200
locomotive decode -i game.bas | awk '/^100 /,/^200 /'
```

### Count Lines
```bash
locomotive decode -i game.bas | wc -l
```

## Documentation Export

### Generate README from BASIC
Create a README showing your BASIC code:

```bash
cat > README.md << 'EOF'
# My CPC Game

## BASIC Source Code

\`\`\`basic
EOF

locomotive decode -i game.bas >> README.md

cat >> README.md << 'EOF'
\`\`\`
EOF
```

### Side-by-Side Comparison
Compare two versions:

```bash
# Create text versions
locomotive decode -i version1.bas -o v1.txt
locomotive decode -i version2.bas -o v2.txt

# Compare
diff -y v1.txt v2.txt
```

## Testing

### Validate Conversion
Test that your BASIC file converts correctly:

```bash
#!/bin/bash
# test-basic.sh

TESTFILE="test.bas"

if locomotive decode -i "$TESTFILE" -o /dev/null 2>&1; then
  echo "✓ Valid BASIC file"
  exit 0
else
  echo "✗ Invalid BASIC file"
  exit 1
fi
```

### CI/CD Integration
GitHub Actions workflow:

```yaml
name: Build BASIC

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install cpclib
        run: |
          wget https://github.com/cpcsdk/rust.cpclib/releases/latest/download/locomotive
          chmod +x locomotive
          
      - name: Build BASIC files
        run: |
          for txt in src/*.txt; do
            bas="build/$(basename ${txt%.txt}.bas)"
            ./locomotive encode -i "$txt" -o "$bas"
          done
          
      - name: Upload artifacts
        uses: actions/upload-artifact@v2
        with:
          name: basic-files
          path: build/*.bas
```

## Tips

- **Use text format for source control** - Binary BASIC files don't diff well in Git
- **Add Amsdos headers** - Use `--header` when creating files for disk images
- **Validate before encoding** - Check your line numbers are in order
- **Use stdout for pipelines** - Decode without `-o` for easy piping
- **Batch operations** - Process multiple files with shell scripts
- **Keep backups** - Always keep the text version of your code

## Common Patterns

### Edit-Compile-Test Loop
```bash
# 1. Edit text file
vim loader.txt

# 2. Convert to binary
locomotive encode -i loader.txt -o LOADER.BAS --header

# 3. Add to test disk
dsk add test.dsk LOADER.BAS

# 4. Test in emulator
# ... test on CPC

# Repeat...
```

### Archive Binary BASIC Files
```bash
# Convert entire collection to text for archiving
mkdir -p archive
for bas in collection/*.bas; do
  txt="archive/$(basename ${bas%.bas}.txt)"
  locomotive decode -i "$bas" -o "$txt"
done
```

### Merge Multiple Programs
```bash
# Combine multiple BASIC programs (be careful with line numbers!)
{
  locomotive decode -i part1.bas
  locomotive decode -i part2.bas
  locomotive decode -i part3.bas
} > combined.txt

# Encode merged version
locomotive encode -i combined.txt -o combined.bas
```


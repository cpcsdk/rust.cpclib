# Borgams Examples

!!! warning "Not Yet Functional"
    Borgams is still work in progress and not currently usable. The examples below show the intended usage once development is complete.

## Basic Usage

### Convert Binary to ASCII
Convert an Orgams binary file to readable ASCII text:

```bash
cpclib-borgams --input myprogram.org --output myprogram.asm
```

### Short Option Form
Using short options:

```bash
cpclib-borgams -i compiled.org -o source.asm
```

## Conversion Workflows

### Source Recovery
Recover source code from Orgams binaries:

```bash
# Convert the binary
cpclib-borgams --input demo.org --output demo_recovered.asm

# View the recovered source
cat demo_recovered.asm
```

### Batch Conversion
Convert multiple Orgams files in a directory:

```bash
# PowerShell
Get-ChildItem *.org | ForEach-Object {
    $output = $_.BaseName + ".asm"
    cpclib-borgams --input $_.Name --output $output
    Write-Output "Converted $($_.Name) -> $output"
}

# Bash
for file in *.org; do
    output="${file%.org}.asm"
    cpclib-borgams --input "$file" --output "$output"
    echo "Converted $file -> $output"
done
```

### Format Migration
Migrate old Orgams projects to plain text:

```bash
# Create output directory
mkdir ascii_sources

# Convert all Orgams files
for orgfile in orgams_binaries/*.org; do
    basename=$(basename "$orgfile" .org)
    cpclib-borgams --input "$orgfile" --output "ascii_sources/${basename}.asm"
done

echo "Migration complete! Check ascii_sources/ directory"
```

## Integration with Other Tools

### With BASM Assembler
Convert Orgams binary to ASCII, then assemble with BASM:

```bash
# Step 1: Convert to ASCII
cpclib-borgams --input original.org --output converted.asm

# Step 2: Assemble with BASM
basm converted.asm -o output.bin
```

### With Version Control
Add converted sources to version control:

```bash
# Convert binary to text
cpclib-borgams --input project.org --output project.asm

# Add to git
git add project.asm
git commit -m "Add ASCII version of Orgams source"
```

### Inspection and Analysis
Convert for code inspection:

```bash
# Convert to ASCII
cpclib-borgams --input mystery.org --output mystery_inspected.asm

# Search for specific code patterns
grep -n "LD A," mystery_inspected.asm

# Count instruction types
grep -o "^[A-Z][A-Z]" mystery_inspected.asm | sort | uniq -c
```

## Development Workflows

### Debugging Orgams Files
Debug by examining the ASCII output:

```bash
# Convert to inspect structure
cpclib-borgams --input buggy.org --output debug.asm

# Open in editor for analysis
code debug.asm
```

### Archive Preservation
Convert entire archive for long-term storage:

```bash
#!/bin/bash
# archive_orgams.sh - Convert all Orgams files in an archive

archive_dir="$1"
output_dir="$2"

mkdir -p "$output_dir"

find "$archive_dir" -name "*.org" | while read orgfile; do
    relative_path="${orgfile#$archive_dir/}"
    output_path="$output_dir/${relative_path%.org}.asm"
    output_subdir=$(dirname "$output_path")
    
    mkdir -p "$output_subdir"
    cpclib-borgams --input "$orgfile" --output "$output_path"
    
    echo "Converted: $relative_path -> ${relative_path%.org}.asm"
done

echo "Archive conversion complete!"
```

Usage:
```bash
./archive_orgams.sh ./old_orgams_projects ./ascii_archive
```

### Comparison Between Versions
Compare two versions of an Orgams file:

```bash
# Convert both versions
cpclib-borgams --input version1.org --output version1.asm
cpclib-borgams --input version2.org --output version2.asm

# Compare differences
diff -u version1.asm version2.asm

# Or use a visual diff tool
meld version1.asm version2.asm
```

## Tips and Best Practices

### File Naming
- Use `.org` extension for Orgams binary files
- Use `.asm` or `.s` extension for ASCII output
- Preserve original filenames when converting for easy tracking

### Output Handling
- Always specify an output file to avoid stdout clutter
- Create output directories before batch conversions
- Use meaningful output names for better organization

### Error Handling
Check for conversion errors:

```bash
if cpclib-borgams --input input.org --output output.asm; then
    echo "Conversion successful"
else
    echo "Conversion failed - check input file format"
    exit 1
fi
```

### Workflow Integration
Combine with other CPC tools:

```bash
# Full workflow: Convert -> Assemble -> Create DSK -> Test
cpclib-borgams -i demo.org -o demo.asm
basm demo.asm -o demo.bin
bdasm demo.bin -o demo_disasm.asm
```

## Common Patterns

### Quick Inspection
Quickly inspect an Orgams binary:

```bash
cpclib-borgams -i mystery.org -o /tmp/inspect.asm && cat /tmp/inspect.asm | less
```

### Backup and Convert
Create backups before conversion:

```bash
# Backup originals
cp original.org original.org.backup

# Convert to ASCII
cpclib-borgams --input original.org --output original.asm
```

### Pipeline Processing
Use in pipelines:

```bash
# Find all Orgams files and report their sizes after conversion
for f in *.org; do
    cpclib-borgams -i "$f" -o temp.asm 2>/dev/null
    if [ $? -eq 0 ]; then
        lines=$(wc -l < temp.asm)
        echo "$f: $lines lines of assembly"
    fi
done
rm -f temp.asm
```

# CSLCLI Examples

## Basic Usage

### Validate a Script
Check if a CSL script is valid:

```bash
cslcli myscript.csl
```

If valid, outputs the parsed script. If invalid, shows errors.

### Verbose Mode
Get detailed parsing information:

```bash
cslcli -v automation.csl
```

Output:
```
Successfully parsed 42 instructions
[parsed script content follows]
```

## Error Handling

### Syntax Errors
Example with syntax error:

```bash
$ cslcli broken_script.csl
Error parsing broken_script.csl at line 5, column 10:
   |
5  | WAIT 10x
   |         ^ invalid character in number
   |
Expected: valid number
```

### Missing Files
```bash
$ cslcli nonexistent.csl
Error reading file 'nonexistent.csl': No such file or directory
```

## Scripting Integration

### Build Validation
Include CSL validation in build scripts:

```bash
#!/bin/bash
# Validate all CSL test scripts

for script in tests/*.csl; do
  echo "Validating $script..."
  if cslcli "$script" > /dev/null; then
    echo "  ✓ OK"
  else
    echo "  ✗ FAILED"
    exit 1
  fi
done

echo "All scripts valid!"
```

### CI/CD Pipeline
```yaml
# Example GitHub Actions workflow
- name: Validate CSL Scripts
  run: |
    for script in scripts/**/*.csl; do
      cslcli "$script"
    done
```

## Development Workflow

### Test Script Development
Iterative development with immediate feedback:

```bash
# Edit script
vi game_test.csl

# Validate
cslcli game_test.csl

# If errors, fix and repeat
# Once valid, use in automation
```

### Converting Scripts
Normalize/format CSL scripts:

```bash
# Parse and reformat
cslcli messy_script.csl > formatted_script.csl
```

## Common CSL Patterns

### Example Valid Script
```csl
# Load program
LOAD "game.dsk"

# Wait for loading
WAIT 3000

# Send keypress
KEYPRESS SPACE

# Wait and verify
WAIT 1000
CHECK SCREEN "GAME OVER"
```

### Validation Commands
```bash
# Quick syntax check
cslcli test.csl > /dev/null && echo "Valid" || echo "Invalid"

# Count instructions
cslcli -v test.csl 2>&1 | grep "parsed" | awk '{print $3}'
```

## Tips

- Use `-v` during development for detailed feedback
- Keep scripts in version control
- Validate scripts in CI/CD pipelines
- Use meaningful comments in CSLfiles
- Test scripts incrementally
- Pipe output to files for formatted versions

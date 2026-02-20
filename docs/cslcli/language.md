# CSL Language Reference

CSL (CPC Script Language) is a domain-specific language for scripting CPC emulator interactions and automating testing scenarios.

## Overview

CSL scripts consist of commands that control emulator behavior, simulate user input, and verify program state. Scripts are executed sequentially.

## Basic Syntax

### Comments
```csl
# This is a comment
// This is also a comment (C-style)
```

### Commands
Commands are typically one per line:
```csl
COMMAND arg1 arg2
```

## Common Commands

### LOAD
Load a program or disk image.

```csl
LOAD "program.dsk"
LOAD "game.sna"
```

### WAIT
Pause execution for a specified duration.

```csl
WAIT 1000        # Wait 1000 milliseconds
WAIT 2.5s        # Wait 2.5 seconds
WAIT 100ms       # Wait 100 milliseconds
```

### KEYPRESS
Simulate a key press.

```csl
KEYPRESS SPACE
KEYPRESS RETURN
KEYPRESS A
KEYPRESS f1
```

### TYPE
Type a string of characters.

```csl
TYPE "HELLO WORLD"
TYPE "RUN\"GAME"
```

### CHECK
Verify screen content or memory state.

```csl
CHECK SCREEN "GAME OVER"
CHECK MEMORY 0x4000 0xFF
```

### SCREENSHOT
Capture a screenshot.

```csl
SCREENSHOT "frame001.png"
```

### RESET
Reset the emulated CPC.

```csl
RESET
RESET SOFT      # Soft reset (Ctrl+Shift+Esc)
```

## Data Types

### Strings
Enclosed in double quotes, with escape sequences:
```csl
"simple string"
"string with \"quotes\""
"path\\to\\file"    # Backslash escape
```

### Numbers
Decimal, hexadecimal, or binary:
```csl
100         # Decimal
0xFF        # Hexadecimal  
0b1010      # Binary
```

### Time Durations
With unit suffixes:
```csl
1000ms      # Milliseconds
2.5s        # Seconds
100         # Default: milliseconds
```

### Key Names
Special key identifiers:
```csl
SPACE, RETURN, ESC, TAB
F1, F2, ..., F9
UP, DOWN, LEFT, RIGHT
CTRL, SHIFT, ALT
A, B, C, ..., Z
0, 1, ..., 9
```

## Control Flow

### Labels
```csl
:label_name
```

### GOTO
```csl
GOTO label_name
```

### IF/ENDIF
```csl
IF SCREEN CONTAINS "READY"
  KEYPRESS SPACE
ENDIF
```

## Advanced Features

### Variables
```csl
SET counter 0
SET name "test"
```

### Loops
```csl
REPEAT 10
  KEYPRESS SPACE
  WAIT 100
END
```

### Assertions
```csl
ASSERT SCREEN "SCORE: 1000"
ASSERT MEMORY 0x4000 < 0x80
```

## Best Practices

1. **Use Comments**: Document script purpose and complex sections
2. **Meaningful Waits**: Allow sufficient time for program reactions
3. **Incremental Testing**: Test scripts step-by-step
4. **Error Handling**: Use CHECK/ASSERT for validation
5. **Modular Scripts**: Break complex automation into smaller files

## Example Script

Complete example for automated game testing:

```csl
# Load game disk
LOAD "mygame.dsk"

# Wait for BASIC prompt
WAIT 1s

# Type and run loader
TYPE "RUN\"LOADER"
KEYPRESS RETURN

# Wait for title screen
WAIT 5s
CHECK SCREEN "PRESS SPACE"

# Start game
KEYPRESS SPACE
WAIT 2s

# Verify game started
CHECK SCREEN "LEVEL 1"

# Take screenshot
SCREENSHOT "level1_start.png"

# Play sequence
KEYPRESS RIGHT
WAIT 500ms
KEYPRESS SPACE
WAIT 500ms

# Verify score increase
CHECK SCREEN "SCORE:"

# Success
```

## Error Messages

Common CSL parsing errors:

- **Unexpected token**: Invalid syntax
- **Unknown command**: Unrecognized command name
- **Invalid argument**: Wrong argument type or format
- **Unterminated string**: Missing closing quote
- **Undefined label**: GOTO to non-existent label

## See Also

- [CSLCLI Command Reference](cmdline.md)
- [Examples](examples.md)
- CPC demo-scene scripting guides

# Orgams Native Assembler Integration

Orgams is a native Z80 assembler that runs directly on the Amstrad CPC. Unlike cross-assemblers that run on modern PCs, Orgams executes within a CPC emulator, providing an authentic development experience with the original Orgams IDE.

## Overview

Orgams integration allows you to:

- **Edit** Z80 assembly source code using the native Orgams editor
- **Assemble** source files within an emulated CPC environment
- **Test** assembled programs immediately by jumping to them
- **Save** assembled binaries back to your host filesystem

The integration uses emulator control to automate Orgams workflows, making native CPC development practical in modern build pipelines.

## Quick Start

### Assemble a Source File

```bash
# Assemble an Orgams source file from a disc image
cpclib-runner orgams --from demo.dsk --src MAIN.O --dst MAIN
```

### Edit with Orgams Editor

```bash
# Launch the Orgams editor to modify source
cpclib-runner orgams --from demo.dsk --src MAIN.O --edit
```

### Assemble and Execute

```bash
# Assemble and immediately jump to the program
cpclib-runner orgams --from demo.dsk --src MAIN.O --jump
```

## Key Features

## Integration with BndBuild

Orgams is available as a standalone `orgams` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `orgams` command. See [BndBuild Commands](../bndbuild/commands.md#assembler-orgams-orgams) for integration details. Note: Orgams is a native assembler that runs on an emulated CPC.

## Key Features

### Native CPC Assembly

- Runs actual Orgams assembler on emulated CPC
- Full compatibility with original Orgams source files
- Authentic development environment

### Albireo Support

- Works with Albireo virtual filesystem
- Use folders instead of disc images
- Faster development iteration

### Format Conversion

- Convert BASM/ASCII source to Orgams format
- Use `--basm2orgams` flag for conversion
- Bridge modern and native workflows

### Build Integration

- Seamless integration with bndbuild
- Automated assembly in build pipelines
- Combine with other cpclib tools

## System Requirements

- **Emulator**: ACE-DL (default), WinAPE, or CPCEC
- **ROM**: Orgams ROM must be available in emulator
- **Platform**: Linux, macOS (Windows support limited)

!!! warning "Windows Compatibility"
    Orgams integration currently does not work properly under Windows due to emulator control limitations.

## How It Works

1. **Source Loading**: Your source file is made available via disc image or Albireo folder
2. **Emulator Launch**: CPC emulator starts with Orgams ROM enabled
3. **Automation**: Keyboard commands are sent to load and assemble
4. **Result Capture**: Assembled binary is retrieved from emulator
5. **Cleanup**: Emulator closes (unless `--keepemulator` specified)

## Use Cases

### Legacy Project Maintenance

Work with existing Orgams projects without converting to modern assemblers.

### Authentic Development

Experience CPC development as it was done in the 1980s/90s.

### Build Pipeline Integration

Incorporate native assembly into automated build workflows.

### Format Experimentation

Compare Orgams output with modern cross-assemblers.

## Limitations

- **Speed**: Emulation overhead makes assembly slower than cross-assemblers
- **Platform**: Limited Windows support
- **Dependencies**: Requires configured emulator with Orgams ROM
- **Debugging**: Limited debugging integration compared to modern tools

## See Also

- [Command Line Reference](cmdline.md) - Detailed options and usage
- [Examples](examples.md) - Practical workflows and integration patterns
- [Borgams Documentation](../borgams/) - Convert Orgams binary format to ASCII
- [CPC Runner Documentation](../runner/) - Emulator control details

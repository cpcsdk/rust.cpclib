# Borgams - Orgams Binary to ASCII Converter

!!! warning "Work in Progress - Not Yet Usable"
    **Borgams is currently under active development and is not yet functional.** While the implementation is close to completion, the tool is not ready for production use. Please check back for updates or use alternative methods for Orgams file conversion in the meantime.

Borgams (Benediction Orgams) is a command-line tool for converting Orgams binary format files to ASCII text format.

## Features

- Convert Orgams binary files to readable ASCII format
- Preserve assembly code structure and formatting
- Support for Orgams preprocessed format used by the Orgams assembler
- Simple input/output file specification

## Quick Start

```bash
# Convert an Orgams binary file to ASCII
cpclib-borgams --input compiled.org --output source.asm
```

## What is Orgams Format?

Orgams is a Z80 assembler that stores assembly source code in a preprocessed binary format. This format is more compact and faster to process than plain text, but it's not human-readable. Borgams converts these binary files back to readable ASCII text, allowing you to:

- Recover source code from compiled Orgams files
- Inspect the structure of Orgams binaries
- Convert legacy Orgams projects to plain text format

## Use Cases

- **Source Recovery**: Extract assembly source from Orgams binary files
- **Format Conversion**: Convert Orgams projects to standard ASCII format
- **Code Inspection**: View the contents of Orgams binaries
- **Archive Preservation**: Convert old Orgams files to long-term readable format

## See Also

## Integration with BndBuild

Borgams is available as a standalone `borgams` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `borgams` command. See [BndBuild Commands](../bndbuild/commands.md#file-management-orgams-to-text-conversion-borgams) for integration details.

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all options
- [Examples](examples.md) - Usage examples and workflows

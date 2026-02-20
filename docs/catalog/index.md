# Catalog - CatArt Manipulation Tool

Catalog is a command-line utility for creating and managing **CatArt** - visually enhanced catalog displays for Amstrad CPC disk images using BASIC programs.

## What is CatArt?

CatArt is a technique for creating attractive, graphical disk catalogs on the Amstrad CPC by embedding catalog information within a specially crafted BASIC program. When the BASIC program runs, it displays a nice-looking catalog with custom fonts and colors.

## Features

- **Build CatArt** from BASIC programs → DSK/HFE or raw binary
- **Display CatArt** with rendering (cat/dir commands)
- **List** catalog content (text output)
- **Decode** CatArt back to BASIC listing
- **Modify** catalog entries (readonly, system flags, filenames, etc.)
- **Debug** CatArt structure (inspect bytes and BASIC commands)
- **PNG export** of pixel-accurate CatArt rendering
- **Multi-locale support** (English, French, Spanish, German, Danish fonts)
- **All screen modes** (0, 1, 2, or 3)

## Quick Start

```bash
# Build a catart DSK from BASIC program
catalog loader.bas build -o game.dsk

# Display catalog (alphabetically sorted)
catalog game.dsk cat 

# Display catalog (directory order)
catalog game.dsk dir

# List catalog content (text only)
catalog game.dsk list

# Decode CatArt back to BASIC
catalog game.dsk decode -o restored.bas

# Export rendering to PNG
catalog game.dsk cat --png catalog.png --locale english

# Modify an entry
catalog game.dsk modify --entry 0 --readonly

# Debug CatArt structure
catalog game.dsk debug --cat
```

## Use Cases

## Integration with BndBuild

Catalog is available as a standalone `catalog` binary but can also be used within [BndBuild](../bndbuild) build scripts using the `catalog` or `cat` command aliases. See [BndBuild Commands](../bndbuild/commands.md#disc-management-catalog-listing-catalogcat) for integration details.

- Create professional-looking disk menus
- Distribute demos with attractive catalogs
- Customize disk presentation
- Reverse-engineer existing CatArt disks
- Modify catalog metadata without breaking the display

## See Also

- [Command Line Reference](cmdline.md) - Detailed documentation of all commands
- [Examples](examples.md) - Usage examples and workflows

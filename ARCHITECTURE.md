# cpclib Architecture

This document provides an overview of the cpclib workspace architecture and how the various crates interact.

## Workspace Overview

cpclib is a large Cargo workspace containing 40+ crates organized into several functional layers:

```
┌─────────────────────────────────────────────────────────┐
│                    User-Facing Tools                     │
│  cpclib-basm, cpclib-bndbuild, cpclib-xfertool, etc.   │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│                   High-Level Libraries                   │
│   cpclib, cpclib-runner, cpclib-asm, cpclib-disc       │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│                   Core Libraries                         │
│   cpclib-common, cpclib-tokens, cpclib-macros           │
└─────────────────────────────────────────────────────────┘
```

## Core Crates

### Foundation Layer

- **`cpclib-common`** (v0.11.0)
  - Shared types, utilities, and event system
  - Used by almost all other crates
  - Provides: logging, paths, itertools re-exports

- **`cpclib-macros`** (v0.11.0)
  - Procedural macros for code generation
  - Derive macros for common patterns

- **`cpclib-tokens`** (v0.11.0)
  - Symbol table and token management
  - Expression evaluation
  - Used by assembler and parser

### File Format Crates

- **`cpclib-sna`** (v0.11.0)
  - Snapshot (.SNA) file reading/writing
  - Chunk-based format support
  - Memory state serialization

- **`cpclib-disc`** (v0.11.0)
  - DSK file manipulation
  - Format, read, write operations
  - Compatible with iDSK/dskmanager

- **`cpclib-cpr`** (v0.11.0)
  - Cartridge (.CPR) file handling
  - GX4000 cartridge format

- **`cpclib-cprcli`** (v0.11.0)
  - Command-line tool for CPR manipulation
  - Create and inspect cartridge files

- **`cpclib-basic`** (v0.11.0)
  - BASIC tokenization and parsing
  - Source to token conversion
  - Round-trip encode/decode support

- **`cpclib-locomotive`** (v0.11.0)
  - Locomotive BASIC manipulation tool
  - ASCII ↔ binary BASIC conversion
  - Integrated into bndbuild

- **`cpclib-files`** (v0.11.0)
  - Generic file utilities

### Assembly & Disassembly

- **`cpclib-asm`** (v0.11.0)
  - Core assembler engine
  - Z80 instruction set support
  - Expression parser and evaluator

- **`cpclib-basm`** (v0.11.0)
  - Command-line assembler binary
  - Multiple pass assembly
  - Macro system
  - **Primary assembler for the toolchain**

- **`cpclib-bdasm`** (v0.11.0)
  - Disassembler binary
  - Z80 code to assembly conversion

- **`cpclib-basmdoc`** (v0.11.0)
  - Documentation generator for assembly source

- **`cpclib-z80emu`** (v0.11.0)
  - Basic Z80 emulator
  - Used for instruction validation

- **`cpclib-orgams-ascii`** (v0.11.0)
  - Orgams ASCII format support
  - On-CPC assembler compatibility

- **`cpclib-borgams`** (v0.11.0)
  - Borgams tool for Orgams integration
  - Converts between Orgams formats

### Graphics & Image Processing

- **`cpclib-image`** (v0.11.0)
  - CPC image format representation
  - Mode 0, 1, 2 support
  - Palette management

- **`cpclib-imgconverter`** (v0.11.0)
  - Image conversion to CPC formats
  - Multiple dithering algorithms
  - Command-line tool

- **`cpclib-sprite-compiler`** (v0.11.0)
  - Sprite data compilation
  - Optimized sprite generation

### Compression

- **`cpclib-crunch`** (v0.11.0)
  - Compression interface
  - Z80-targeted crunchers

- **`cpclib-crunchers`** (v0.11.0)
  - Multiple compression algorithms
  - Exomizer, ZX0, LZSA support
  - Includes native C implementations

### Build & Automation

- **`cpclib-bndbuild`** (v0.11.0)
  - Build automation tool (Make alternative)
  - YAML-based project files
  - Task orchestration
  - **Central build system for demo projects**

- **`cpclib-bndbuild-ratatui`** (v0.11.0)
  - Terminal UI (TUI) frontend for bndbuild
  - Interactive build monitoring
  - Built with ratatui library

- **`cpclib-runner`** (v0.11.0)
  - External tool management
  - Emulator control
  - Download and cache external tools
  - Supports: Rasm, Winape, ACE, AT3, etc.

### Hardware Communication

- **`cpclib-xfer`** (v0.11.0)
  - Library for cpcwifi/M4 communication
  - File transfer protocols

- **`cpclib-xfertool`** (v0.11.0)
  - Command-line tool for hardware transfers
  - Interactive REPL mode

### Emulator Control

- **`cpclib-emucontrol`** (v0.11.0)
  - Emulator remote control
  - Script automation

- **`cpclib-csl`** (v0.11.0)
  - CSL (CPC Script Language) parser
  - Emulator scripting language types
  - AST and evaluation

- **`cpclib-cslcli`** (v0.11.0)
  - CSL command-line tool
  - Execute CSL scripts

### GUI Applications

- **`cpclib-visual-basm`** (v0.11.0)
  - GUI frontend for assembler
  - Syntax highlighting
  - Build integration

- **`cpclib-visual-bndbuild`** (v0.11.0)
  - GUI frontend for build system
  - Project visualization

- **`cpclib-bndbuild-tauri`** (v0.1.0)
  - Tauri-based GUI alternative
  - Cross-platform desktop app

### Language Bindings

- **`cpclib-wasm`** (v0.11.0)
  - WebAssembly bindings
  - Browser-based tooling
  - Website at https://cpcsdk.github.io/rust.cpclib/

- **`cpclib-python`** (v0.11.0)
  - Python bindings via PyO3
  - Python API for toolchain

### Demo & Catalog Tools

- **`cpclib-catart`** (v0.11.0)
  - Catart-related functionality
  - Demo catalog generation
  - Amstrad CPC demo scene tools

- **`cpclib-catalog`** (v0.11.0)
  - Amsdos catalog manipulation
  - Directory listing utilities
  - DSK catalog operations

### Main Integration Crate

- **`cpclib`** (v0.11.0)
  - Umbrella crate combining most functionality
  - Re-exports from other crates
  - Unified API surface
  - **Start here for library usage**

## Dependency Flow

```
cpclib (facade)
  ├── cpclib-asm
  │   ├── cpclib-tokens
  │   └── cpclib-common
  ├── cpclib-disc
  │   └── cpclib-common
  ├── cpclib-sna
  │   └── cpclib-common
  ├── cpclib-image
  │   └── cpclib-common
  └── cpclib-basic
      └── cpclib-common

cpclib-basm (CLI tool)
  └── cpclib-asm
      └── ...

cpclib-bndbuild (build tool)
  ├── cpclib-runner
  │   └── cpclib-common
  └── cpclib
      └── ...
```

## Common Patterns

### Error Handling
- Most crates use `Result<T, String>` or custom error types
- `thiserror` for error derivation
- Moving toward proper `Result` propagation (reducing `.unwrap()`)

### Event System
- `cpclib-common` provides `EventObserver` trait
- Used for progress reporting and logging
- Allows GUI and CLI tools to share logic

### Feature Flags
- Workspace dependencies use `default-features = false`
- Optional features for GUI, hardware, etc.
- Minimizes binary size for focused builds

### Workspace Inheritance
- Shared metadata in workspace `Cargo.toml`
- `edition = "2024"` across all crates
- MIT license throughout
- Current version: `v0.11.0` for all workspace crates

## Tool Integration Flow

```
User writes:
  - .asm files (assembly source)
  - .bas files (BASIC source)
  - build.yml (bndbuild config)
  - .png images

     ↓

bndbuild orchestrates:
  1. Image conversion (cpclib-imgconverter)
  2. BASIC conversion (cpclib-locomotive)
  3. Assembly (cpclib-basm)
  4. Compression (cpclib-crunchers)
  5. DSK creation (cpclib-disc)
  6. Catalog operations (cpclib-catalog)
  7. Emulator launch (cpclib-runner)
  8. Hardware transfer (cpclib-xfer)

     ↓

Output:
  - .sna snapshots
  - .dsk disk images
  - .bin binaries
  - .cpr cartridges
```

## Adding a New Crate

1. Create crate directory under workspace root
2. Add to `members` in workspace `Cargo.toml`
3. Add workspace dependency entry
4. Add `[patch.crates-io]` entry if needed
5. Use workspace inheritance:
   ```toml
   [package]
   edition.workspace = true
   license.workspace = true
   authors.workspace = true
   ```
6. Document in this file

## External Tool Integration

The `cpclib-runner` crate manages external tools:

- **Assemblers**: Rasm, Orgams, Sjasmplus, Vasm, uz80
- **Emulators**: WinAPE, ACE, CPCEC, Sugarbox, AmSpirit, Caprice
- **Trackers**: ArkosTracker 3, ChipNSFX
- **Compression**: Exomizer, LZSA, DZ80
- **Graphics**: Martine, Grafx2, img2cpc
- **Disk Utils**: ImpDSK, Disark
- **Others**: Various converters and utilities

Tools are downloaded, cached, and versioned automatically.

## Testing Strategy

- **Unit tests**: In each crate's `tests/` directory
- **Integration tests**: In workspace-level `tests/`
- **Cross-assembler tests**: `cpclib-rasm-basm-tests` validates compatibility
- **Examples**: In crate `examples/` directories

## Building Demos

Typical demo project structure:
```
my-demo/
├── build.yml          # bndbuild configuration
├── src/
│   ├── main.asm       # Main assembly source
│   └── data.asm       # Data includes
├── gfx/               # Source images
└── music/             # AT3 tracker files
```

Run: `bndbuild` in project directory

## Documentation

- Main docs: https://cpcsdk.github.io/rust.cpclib/
- Generated via `mkdocs` from `docs/` directory
- API docs: `cargo doc --open`

## Recent Improvements (v0.11.0)

- **BASIC Support**: Added `cpclib-locomotive` for BASIC ↔ binary conversion
  - Fixed round-trip encoding/decoding bugs in `cpclib-basic`
  - Properly handles string literals with closing quotes
  - Integrated into bndbuild workflow

- **Terminal UI**: Added `cpclib-bndbuild-ratatui` for interactive builds

- **Emulator Scripting**: Added `cpclib-csl` and `cpclib-cslcli` for CSL support

- **Catalog Tools**: Enhanced catalog manipulation with `cpclib-catalog` and `cpclib-catart`

- **Orgams Support**: Better on-CPC assembler compatibility via `cpclib-borgams`

## Future Architecture Goals

- Reduce `.unwrap()` panics throughout codebase
- Stabilize APIs for v1.0 release
- Better error messages with source spans
- Incremental compilation support
- Plugin system for custom tasks
- Improved IDE integration and LSP support

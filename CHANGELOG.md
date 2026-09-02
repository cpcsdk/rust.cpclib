# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- GitHub Actions CI/CD workflows for automated testing and quality checks
- CONTRIBUTING.md with development guidelines and setup instructions
- Workspace-level version management for consistency across crates
- Add all executbles in the documentation. They are AI generated. So they may not be perfect, but they should be a good starting point for documentation.
- `cpclib-catart`add this crate to handle catalog art
- `cpclib-csl`add support for CSL file parsing and generation (mainly to check validity of existing ones)
- `cpclib-basic` add support for binary encoded programs (tokenized BASIC)
- `cpclib-basmdoc` add a new crate to handle documetnation of z80 projects
- `cpclib-bndbuild` add support fof Z80Profiler by Targhan/Arkos
- `cpclib-bndbuild` add support of the catalog command
- `cpclib-bndbuild` add support to the hxcfe (inner) command
- `cpclib-bndbuild` add support to the csl command
- `cpclib-bndbuild` add `archive` command for creating, listing, and extracting .zip and .tar.gz archives
- `cpclib-emucontrol` add support to activate roms (it was only possible to dectivate them before)
- `cpclib-locomotive` new crate to handle the executable for basisc manipulation
- `cpclib-orgams-ascii` add support to ORGAMS files. This crate aims at converting orgams sourceode to ascii and ascii source code to orgams. (in fact utf8, but...)
- `cpclib-basm` add `//` integer-division operator to expressions - always returns an integer, unlike `/` which promotes int/int division to a real value
- `cpclib-basm` add a warning whenever a real (float) value is truncated to an integer anywhere it's used (register loads, memory writes, `list_set`/`string_format`, `.sym` export, bitwise/shift operators, comparisons, ...), since this almost always indicates an unintended `/` where `//` was meant. The warning is typed (`ExprWarning`/`ExprWarningKind`, room for more kinds later) and, inside a single expression with nested sub-expressions, located at the innermost one that produced it rather than just "somewhere in this statement". Never fires for a float that exactly encodes an integer (e.g. `6.0 / 2`)
- `cpclib-basm` the pre-existing "value does not fit" overflow warning (register/memory immediates) now uses the same typed warning mechanism as the new real-value-truncation warning, instead of being categorized by matching a substring in its rendered text

### Changed
- Standardized README filename from `.mkd` to `.md`
- Improved workspace dependency management
- `cpclib-basic` better support of string programs
- `cpclib-bndbuild` add support to catalog, locomotive, csl, basmdoc
- `cpclib-catalog` add catalog visualization and catart creation
- `cpclib-basm` add reorganize the source cdode of the parser
- `cpclib-emucontrol` add support of CSL for controling emulators (partially possible for those without CSL support)
- `cpclib-basm` **BREAKING**: `//` is no longer a line-comment marker (only `;` is now) - `//` is exclusively the new integer-division operator

### Fixed
- `cpclib-bndbuild` AT3 version detection and download URLs
- `cpclib-basm` fix various bugs 
- `cpclib-catalog` fix various bugs
- `HFE` files can be manipulated also from the windows platform (we had issues with the  `hxcfe` dependency before)

## [0.11.0] - 2025-12-15

### Added
- Multiple successful demo releases using the toolchain:
  - Blight (2025)
  - Amstrology (2025)
  - 4deKades (2025)
  - J'AI PÉ-TÉLÉCRAN (2024)
  - Come Join Us (2024)

### Notable Features
- Z80 assembler with auto-generated code support
- SNA file manipulation with chunk support
- Image conversion to CPC formats
- DSK manipulation (format and add files)
- cpcwifi communication (reset and run file)
- BASIC token generation from source

## Earlier Versions

See git history for detailed changes in earlier versions.

---

## Version Notes

- **0.10.0**: Most crates stabilized at this version
- **0.8.0**: Build tools (bndbuild, runner, cpr)
- **0.5.0**: WASM bindings
- **0.1.0**: Initial releases for newer crates

[Unreleased]: https://github.com/cpcsdk/rust.cpclib/compare/v0.11.0...HEAD
[0.11.0]: https://github.com/cpcsdk/rust.cpclib/releases/tag/v0.11.0

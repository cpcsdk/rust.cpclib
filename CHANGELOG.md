# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- GitHub Actions CI/CD workflows for automated testing and quality checks
- CONTRIBUTING.md with development guidelines and setup instructions
- Workspace-level version management for consistency across crates
- `cpclib-catart`add this crate to handle catalog art
- `cpclib-csl`add support for CSL file parsing and generation (mainly to check validity of existing ones)
- `cpclib-basic` add support for binary encoded programs (tokenized BASIC)
- `cpclib-basmdoc` add a new crate to handle documetnation of z80 projects
- `cpclib-bndbuild` add support fof Z80Profiler by Targhan/Arkos
- `cpclib-bndbuild` add support of the catalog command
- `cpclib-emucontrol` add support to activate roms (it was only possible to dectivate them before)
- `cpclib-locomotive` new crate to handle the executable for basisc manipulation
- `cpclib-orgams-ascii` add support to ORGAMS files. This crate aims at converting orgams sourceode to ascii and ascii source code to orgams. (in fact utf8, but...)

### Changed
- Standardized README filename from `.mkd` to `.md`
- Improved workspace dependency management
- `cpclib-basic` better support of string programs
- `cpclib-bndbuild` add support to catalog, locomotive, csl, basmdoc
- `cpclib-catalog` add catalog visualization and catart creation
- `cpclib-basm` add reorganize the source cdode of the parser
- `cpclib-emucontrol` add support of CSL for controling emulators (partially possible for those without CSL support)

### Fixed
- `cpclib-bndbuild` AT3 version detection and download URLs
- `cpclib-basm` fix various bugs 
- `cpclib-catalog` fix various bugs

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

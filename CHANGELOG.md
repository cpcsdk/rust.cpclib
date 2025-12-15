# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- GitHub Actions CI/CD workflows for automated testing and quality checks
- CONTRIBUTING.md with development guidelines and setup instructions
- Workspace-level version management for consistency across crates
- Support for AT3 version 3.5 (ArkosTracker)
- Support for Z80Profiler tool in bndbuild

### Changed
- Standardized README filename from `.mkd` to `.md`
- Improved workspace dependency management

### Fixed
- AT3 version detection and download URLs

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

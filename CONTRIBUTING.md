# Contributing to cpclib

Thank you for your interest in contributing to cpclib! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- **Rust Toolchain**: Nightly channel is required
  ```bash
  rustup toolchain install nightly
  rustup override set nightly
  ```

- **Platform-specific requirements**:
  - **Windows**: Use `nightly-x86_64-pc-windows-gnu` toolchain
    ```bash
    rustup toolchain install nightly-x86_64-pc-windows-gnu
    rustup default nightly-x86_64-pc-windows-gnu
    ```
  - **Linux/macOS**: Standard nightly toolchain works

### Building the Project

```bash
# Clone the repository
git clone https://github.com/cpcsdk/rust.cpclib
cd rust.cpcdemotools

# Build all workspace crates
cargo build --workspace

# Build in release mode
cargo build --release --workspace

# Build a specific crate
cargo build -p cpclib-basm
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p cpclib-asm

# Run a specific test
cargo test test_name
```

### Code Quality

Before submitting a PR, ensure your code passes these checks:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check for common issues
cargo deny check
```

## Project Structure

This is a Cargo workspace with multiple crates:

- **`cpclib`** - Core library with most functionality
- **`cpclib-asm`** - Z80 assembler components
- **`cpclib-basm`** - Main assembler binary
- **`cpclib-bndbuild`** - Build automation tool (like Make for CPC demos)
- **`cpclib-runner`** - Emulator and tool integration
- **`cpclib-disc`** - DSK file manipulation
- **`cpclib-sna`** - Snapshot file handling
- **`cpclib-image`** - Image conversion to CPC formats
- **`cpclib-xfer`** - Hardware communication (cpcwifi)
- **GUI tools** - `cpclib-visual-basm`, `cpclib-visual-bndbuild`

## Coding Guidelines

### Error Handling

- Avoid `.unwrap()` in library code - use `Result` and `?` operator
- Use `.expect()` only when panic is truly appropriate with a clear message
- Provide meaningful error messages

### Documentation

- Add doc comments (`///`) for public APIs
- Include examples in doc comments when helpful
- Update README files when adding new features

### Testing

- Add unit tests for new functionality
- Add integration tests for user-facing features
- Test edge cases and error conditions

## Pull Request Process

1. **Fork** the repository and create a feature branch
   ```bash
   git checkout -b feature/my-new-feature
   ```

2. **Make your changes** with clear, atomic commits
   ```bash
   git commit -m "Add support for new feature"
   ```

3. **Update documentation** if you're changing user-facing behavior

4. **Run quality checks** locally:
   ```bash
   cargo fmt --all
   cargo clippy --workspace
   cargo test --workspace
   ```

5. **Push** to your fork
   ```bash
   git push origin feature/my-new-feature
   ```

6. **Create a Pull Request** on GitHub with:
   - Clear description of what changed and why
   - Reference any related issues
   - Screenshots/examples if applicable

7. **Address review feedback** and update your PR

## Reporting Issues

When reporting bugs, please include:

- OS and Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Relevant error messages or logs

## Feature Requests

Feature requests are welcome! Please:

- Check if the feature already exists or is planned
- Describe the use case clearly
- Explain how it fits with the project's goals

## Cross-Platform Builds

### Building for Windows (from Linux)
```bash
./build_windows.sh
```

### Building for macOS (from Linux with osxcross)
```bash
export PATH=~/src/osxcross/target/bin:$PATH
export PKG_CONFIG_ALLOW_CROSS=1
export CC=o64-clang
export CXX=o64-clang++
./build_osx_from_linux.sh
```

### Building for Linux
```bash
./build_linux.sh
```

## Release Process

Releases are managed by project maintainers. Version numbers follow semantic versioning.

## Getting Help

- Open an issue for bugs or questions
- Check existing documentation at https://cpcsdk.github.io/rust.cpclib/
- Review demo projects listed in the main README

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

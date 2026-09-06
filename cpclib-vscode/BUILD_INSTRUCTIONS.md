# Build Instructions

The project now uses a Python build script (`build_extension.py`) instead of the Makefile for better cross-platform compatibility.

## Prerequisites

- Python 3.6 or later
- Rust/Cargo
- Node.js/npm

## Usage

### Build for current platform (auto-detected)
```bash
python build_extension.py build
```
This will create a `.vsix` file with the platform name included:
- Windows: `cpclib-vscode-windows-0.0.1.vsix`
- Linux: `cpclib-vscode-linux-0.0.1.vsix`
- macOS: `cpclib-vscode-macosx-0.0.1.vsix`

### Build for specific platforms
```bash
python build_extension.py build-linux      # Build for Linux
python build_extension.py build-windows    # Build for Windows  
python build_extension.py build-macosx     # Build for macOS
```

### Package for all platforms
```bash
python build_extension.py package-all
```
This will create `cpclib-vscode-all-0.0.1.vsix` containing binaries for all platforms.

### Clean build artifacts
```bash
python build_extension.py clean           # Clean everything
python build_extension.py clean-bins      # Clean only binaries
```

### Other commands
```bash
python build_extension.py install-deps    # Install npm dependencies
python build_extension.py compile         # Compile TypeScript
python build_extension.py watch           # Start TypeScript watch mode
python build_extension.py help            # Show all available commands
```

## Migration from Makefile

The Python script provides the same functionality as the old Makefile:

| Old Makefile command | New Python command |
|---------------------|-------------------|
| `make build` | `python build_extension.py build` |
| `make build-linux` | `python build_extension.py build-linux` |
| `make build-windows` | `python build_extension.py build-windows` |
| `make build-macosx` | `python build_extension.py build-macosx` |
| `make package-all` | `python build_extension.py package-all` |
| `make clean` | `python build_extension.py clean` |
| `make clean-bins` | `python build_extension.py clean-bins` |
| `make install-deps` | `python build_extension.py install-deps` |
| `make compile` | `python build_extension.py compile` |

## Advantages of Python Script

- **Truly cross-platform**: No dependency on GNU Make or Unix shell commands
- **Better Windows support**: Handles cmd.exe and PowerShell correctly
- **Readable code**: Easy to understand and modify
- **Native path handling**: Uses Python's pathlib for cross-platform paths
- **Better error messages**: Clear, colored output with detailed error information
- **Fixes linking issues**: Uses native builds (without --target flag) when building on the same OS to avoid library linking problems
- **Clear package naming**: Extension packages are automatically named with platform suffix (e.g., `cpclib-vscode-windows-0.0.1.vsix`) for easy identification

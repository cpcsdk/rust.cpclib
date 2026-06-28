# cpclib-lsp

Language Server Protocol (LSP) implementation for Amstrad CPC development tools.

## Overview

`cpclib-lsp` provides intelligent code editing features for:
- **Z80 Assembly** files (`.asm`, `.z80`) using basm syntax
- **Build files** (`.build`, `.bnd`) using bndbuild syntax

The LSP server integrates with any LSP-compatible editor (VS Code, Neovim, Emacs, etc.) to provide:
- 📝 **Code completion** for instructions, registers, directives, and build tasks
- 🔍 **Hover documentation** for Z80 opcodes and build task parameters
- 🚨 **Diagnostics** for syntax errors and invalid configurations
- 🎨 **Semantic highlighting** for better code readability

## Features

### Z80 Assembly Support

The assembly analyzer provides LSP features for Z80 assembly files using basm syntax:

#### Code Completion
- **Z80 Instructions**: All standard Z80 mnemonics (`LD`, `ADD`, `JP`, `CALL`, etc.)
- **Registers**: All Z80 registers including 8-bit (`A`, `B`, `C`, etc.), 16-bit (`HL`, `BC`, `DE`, `SP`), and index registers (`IX`, `IY`)
- **Assembler Directives**: 
  - Standalone: `ORG`, `EQU`, `DB`, `DW`, `DS`, `INCLUDE`, etc.
  - Block directives: `MACRO`/`MEND`, `REPEAT`/`REND`, etc.

#### Hover Information
- Quick reference for Z80 instructions
- Register descriptions and usage
- Directive documentation

#### Diagnostics
- Syntax error detection
- Invalid instruction/register warnings
- Assembler directive validation

### Build File Support

The build file analyzer provides LSP features for bndbuild YAML files:

#### Code Completion
- **Build Keywords**: `targets`, `tasks`, `deps`, `args`, `env`, `default`, `includes`
- **Task Types** (50+ supported):
  - **Assemblers**: `basm`, `rasm`, `sjasmplus`, `vasm`, `orgams`
  - **Emulators**: `ace`, `winape`, `cpcec`, `sugarbox`, `retrovm`, `amspirit`
  - **Disk Operations**: `dsk`, `sna`, `catalog`, `impdisc`, `hxcfe`
  - **Image Tools**: `img2cpc`, `cpc2img`, `martine`, `grafx2`, `convgeneric`
  - **Audio Tools**: `at` (Arkos Tracker), `ayt`, `chipnsfx`, `hspc`
  - **File Operations**: `cp`, `mv`, `rm`, `mkdir`, `archive`
  - **Disassemblers**: `bdasm`, `disark`, `uz80`
  - **Hardware**: `xfer` (CPC WiFi/M4 transfer)
  - **Utilities**: `echo`, `extern`, `locomotive`, `emuctrl`

#### Hover Information
- Task descriptions and usage examples
- Keyword documentation
- Parameter hints

#### Diagnostics
- YAML syntax validation
- Jinja template detection
- Build structure validation (targets/tasks presence)

## Installation

### As a Standalone Binary

Build from source:

```bash
cd cpclib-lsp
cargo build --release
```

The binary will be at `target/release/cpclib-lsp`.

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
cpclib-lsp = "0.11.0"
```

## Usage

### Starting the Server

Run the LSP server with stdio transport:

```bash
cpclib-lsp
```

The server communicates via standard input/output using the LSP protocol.

### Editor Configuration

#### VS Code

Create or update `.vscode/settings.json`:

```json
{
  "cpclib-lsp.enabled": true,
  "cpclib-lsp.serverPath": "/path/to/cpclib-lsp",
  "files.associations": {
    "*.asm": "z80-asm",
    "*.z80": "z80-asm",
    "*.build": "yaml",
    "*.bnd": "yaml"
  }
}
```

#### Neovim (nvim-lspconfig)

Add to your Neovim configuration:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define cpclib-lsp if not already defined
if not configs.cpclib then
  configs.cpclib = {
    default_config = {
      cmd = { '/path/to/cpclib-lsp' },
      filetypes = { 'asm', 'z80asm', 'yaml' },
      root_dir = lspconfig.util.root_pattern('*.build', '*.bnd', '.git'),
      settings = {},
    },
  }
end

-- Attach to assembly and build files
lspconfig.cpclib.setup{}
```

#### Emacs (lsp-mode)

Add to your Emacs configuration:

```elisp
(require 'lsp-mode)

(add-to-list 'lsp-language-id-configuration '(z80-asm-mode . "z80-asm"))

(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "/path/to/cpclib-lsp")
                  :major-modes '(z80-asm-mode yaml-mode)
                  :server-id 'cpclib-lsp))

(add-hook 'z80-asm-mode-hook #'lsp)
(add-hook 'yaml-mode-hook #'lsp)
```

## Architecture

### Module Structure

```
cpclib-lsp/
├── src/
│   ├── main.rs         # LSP server entry point
│   ├── lib.rs          # Public API
│   ├── backend.rs      # Main LSP backend implementation
│   ├── document.rs     # Document management and text operations
│   ├── asm.rs          # Z80 assembly analyzer
│   └── build.rs        # Build file analyzer
└── tests/
    ├── assembly_tests.rs    # Assembly LSP tests
    └── build_file_tests.rs  # Build file LSP tests
```

### Data Sources

The LSP server uses compile-time generated data from source-of-truth crates:

- **Assembly data**: From `cpclib-asm/build.rs`
  - `cpclib_asm::lsp::Z80_INSTRUCTIONS`
  - `cpclib_asm::lsp::Z80_REGISTERS`
  - `cpclib_asm::lsp::ASSEMBLER_DIRECTIVES_*`

- **Build data**: From `cpclib-bndbuild/src/lsp.rs`
  - `cpclib_bndbuild::lsp::TASK_TYPES`
  - `cpclib_bndbuild::lsp::BUILD_KEYWORDS`

This ensures the LSP server always stays synchronized with the assembler and build tool capabilities.

## Testing

Run the test suite:

```bash
# All tests
cargo test -p cpclib-lsp

# Just assembly tests
cargo test -p cpclib-lsp assembly_tests

# Just build file tests
cargo test -p cpclib-lsp build_file_tests

# Just integration tests
cargo test -p cpclib-lsp integration_tests
```

### Test Coverage

The test suite includes three categories:

#### Unit Tests

**Assembly Tests** (`tests/assembly_tests.rs`):
- ✅ All Z80 instructions are present
- ✅ All Z80 registers are present
- ✅ All assembler directives are present
- ✅ No duplicates in completion lists
- ✅ Case consistency

**Build File Tests** (`tests/build_file_tests.rs`):
- ✅ All build task types are synchronized with `ALL_APPLICATIONS`
- ✅ All build keywords are present
- ✅ All tasks have descriptions and examples
- ✅ Examples follow YAML syntax
- ✅ No duplicates in task names

#### Integration Tests

**LSP Protocol Tests** (`tests/integration_tests.rs`):

These tests launch the actual LSP server and exercise the LSP protocol:
- ✅ Server initialization and capability negotiation
- ✅ Document lifecycle (open, change, close)
- ✅ Completion for Z80 instructions and directives
- ✅ Completion for build file tasks and keywords
- ✅ Hover documentation for instructions
- ✅ Multiple concurrent documents
- ✅ Real-time document updates

The integration tests use `tower-lsp`'s test infrastructure to create a real LSP service and verify protocol-level behavior.

## Development

### Adding New Instructions/Directives

Z80 instructions and directives are defined in `cpclib-asm`. They are automatically included in the LSP through the build-time generation process. No changes to `cpclib-lsp` are needed.

### Adding New Build Tasks

When adding a new task type to `cpclib-bndbuild`:

1. Add the command constants to `cpclib-bndbuild/src/task.rs`:
   ```rust
   pub const NEW_TASK_CMDS: &[&str] = &["newtask", "nt"];
   ```

2. Add to `ALL_APPLICATIONS` in `cpclib-bndbuild/src/lib.rs`:
   ```rust
   (NEW_TASK_CMDS, false),  // false = not clearable
   ```

3. Add a `TaskType` entry in `cpclib-bndbuild/src/lsp.rs`:
   ```rust
   TaskType {
       names: NEW_TASK_CMDS,
       description: "Description of what this task does",
       example: "- newtask:\n    args: input.file",
   },
   ```

4. The compile-time test `test_all_applications_covered_in_task_types()` will fail if any task is missing from `TASK_TYPES`, ensuring synchronization.

### Debugging

Enable detailed logging:

```bash
RUST_LOG=tower_lsp=debug,cpclib_lsp=debug cpclib-lsp
```

Logs will show:
- LSP initialization
- Document open/change/close events
- Completion requests and responses
- Hover requests and responses
- Diagnostic generation

## Capabilities

The LSP server announces these capabilities to clients:

- ✅ `textDocumentSync` (incremental)
- ✅ `hoverProvider`
- ✅ `completionProvider` (with trigger characters: `.`, `:`, `#`, `%`, `$`, `{`)
- ✅ `definitionProvider`
- ✅ `referencesProvider`
- ✅ `documentSymbolProvider`
- ✅ `workspaceSymbolProvider`
- ✅ `semanticTokensProvider`

Note: Some capabilities are declared but not yet fully implemented. This is a work-in-progress.

## Limitations

Current limitations:
- Symbol definitions and references are not yet implemented
- Semantic tokens are not yet generated
- Jump to definition doesn't work yet
- Limited cross-file analysis
- Jinja templates in build files cause YAML validation to be limited

## Contributing

Contributions are welcome! Please:
1. Add tests for new features
2. Run `cargo fmt` and `cargo clippy`
3. Ensure all tests pass: `cargo test -p cpclib-lsp`
4. Update this README if adding major features

## License

Part of the cpclib project. See the root `LICENSE` file for details.

## Related Projects

- **cpclib-asm**: Z80 assembler with basm syntax
- **cpclib-bndbuild**: Build automation tool for CPC projects
- **cpclib**: Core library with CPC file format support

# CPClib VSCode Extension

**Language support for Amstrad CPC development** — Z80 assembly (basm), bndbuild project management, and Locomotive BASIC.

> ⚠️ **Benediction Toolchain Focus**: This extension is specifically designed for the [Benediction CPClib toolchain](https://github.com/cpcsdk/rust.cpclib) and focuses on the **basm assembler** and **bndbuild build tool** for **Amstrad CPC** production. While pull requests to extend compatibility to other assemblers and platforms are welcome, the current implementation is optimized for this ecosystem.

## Features

### 🔧 Assembly Language Support (basm)

The extension provides comprehensive support for Z80 assembly files (`.asm`, `.z80`) using the **basm** assembler syntax:

#### Code Intelligence
- **Syntax highlighting** with semantic token coloring
- **Code completion** for Z80 instructions, directives, registers, and labels
- **Hover documentation** showing:
  - Instruction timing (cycles, bytes, flags)
  - Macro/struct expansion preview
  - BASIC function evaluation
  - CPC firmware routine documentation
- **Go to definition** for labels, macros, structs, and functions
- **Document outline** (symbol list) for quick navigation
- **Call hierarchy** for label references and macro calls

#### Real-time Diagnostics
- **Syntax errors** as you type
- **Assembly warnings** (overflow, unused labels, etc.)
- **Disabled code is greyed out**: a branch of an `IF`/`ELSEIF`/`ELSE` whose condition can be decided at assembly time fades, the same way an inactive `#if` block does in C
- **Forgotten `equ $-1`**: `ld a, 0 : .counter` names the address *after* the instruction, so a self-modifying patch through it hits the next opcode instead of the operand. Flagged, with a quickfix that writes the `equ $-1` (or `$-2` for a 16-bit load)
- **Instruction validation** with timing information
- **Macro/struct expansion errors**

#### Advanced Features
- **Cycle count calculator**: Select assembly code and see total cycles in the status bar
- **Register usage tracking**: Shows which registers are used at cursor position
- **CPC color picker**: Right-click on `INK`/`BORDER`/`PAPER` directives to pick from the authentic 27-color CPC palette
- **Format on type**: Auto-formatting for consistent code style
- **Embedded BASIC**: Full support for LOCOMOTIVE BASIC blocks within assembly files
- **Breakpoints**: clicking the editor gutter (the red dot) writes a basm `breakpoint` directive in front of that line's instruction, and clearing it takes the directive back out - so the breakpoint travels into the snapshot and the emulator honours it. It works both ways: opening a file that already contains directives shows their red dots, wherever on the line they sit (`ld a,0 : breakpoint` counts as much as the other order), and removing one takes the statement separator from whichever side has it. Disable with `cpclib.breakpointDirective` if you would rather your sources were never edited on your behalf. (VS Code only lets you click the gutter for languages an extension declares as breakpoint-capable; this extension declares `basm`. If the dot still refuses to appear in some other file type, turn on VS Code's own `debug.allowBreakpointsEverywhere`.)

#### Code Actions & Refactoring
- **Extract to macro**: Convert code selection into a reusable macro
- **Wrap in repetition block**: Convert code selection into a repeat directive
- **Quick fixes** for common errors and optimizations
- **Code stabilization**: Convert an unstable code to a stable one (only handle simple cases ATM)
- **Inline macro**: Replace macro call with its expanded content (**Not yet implemented**)

### 📦 Build System Support (bndbuild)

Full integration with the **bndbuild** project automation tool:

- **Syntax highlighting** for `.bnd`, `.build`, and `bndbuild.yml` files
- **CodeLens buttons**: One-click "▶ Run" buttons above each rule definition
- **Task provider**: Integrated VS Code tasks for running bndbuild targets
- **Build output** streaming in real-time to the output panel
- **Diagnostic integration**: Failed builds show errors at the exact line in your files
- **Jinja2 template support**: Embedded template syntax highlighting
- **Integration within a .asm file** for embedding the building instructions in the source

#### Running Builds
1. Open a `bndbuild.yml` file
2. Click the "▶ Run" CodeLens button above any rule, or
3. Use the Command Palette → "Tasks: Run Task" → select a bndbuild target
4. Build output appears in the "CPClib LSP" output panel

### 📝 BASIC Language Support

Support for Locomotive BASIC (`.bas`) and CatArt (`.CAT`, `.ASC`) files:

- **Syntax highlighting** for BASIC keywords and structure
- **Format on type** for consistent code style
- **Color picker** for `INK`, `BORDER`, and `PAPER` commands
- **Document symbols** for label navigation
- **Hover documentation** for CPC firmware routines
- **Execution in emulator**

## Installation

Install the extension from the [VS Code Marketplace](https://marketplace.visualstudio.com/) by searching for "CPClib" or "Amstrad CPC".

Alternatively, download the `.vsix` file from [GitHub Releases](https://github.com/cpcsdk/rust.cpclib/releases) and install manually:
```bash
code --install-extension cpclib-vscode-0.0.1.vsix
```

### What's Included

The extension **bundles all required binaries** for Linux, Windows, and macOS. No additional installation or Rust toolchain is required - just install the extension and start coding!

### For Extension Developers

If you're developing the extension itself or the LSP server, you can override the bundled binary by:

1. Building your own `cpclib-lsp`:
   ```bash
   cargo install --path cpclib-lsp
   ```

2. Setting `cpclib-lsp.serverPath` in VS Code settings to point to your custom binary:
   ```json
   {
     "cpclib-lsp.serverPath": "/path/to/your/cpclib-lsp"
   }
   ```

The extension will use your custom binary instead of the bundled one.

## Configuration

### Settings

Access via *File → Preferences → Settings* and search for "cpclib":

- **`cpclib-lsp.serverPath`** (default: `"cpclib-lsp"`)  
  Path to the cpclib-lsp binary. The extension includes bundled binaries for all platforms, so you typically don't need to change this. Only set an explicit path if you're developing the LSP server:
  ```json
  {
    "cpclib-lsp.serverPath": "/path/to/custom/cpclib-lsp"
  }
  ```

- **`cpclib.breakpointDirective`** (default: `true`)  
  Mirror editor breakpoints into the source as basm `breakpoint` directives, and show a red dot for directives already in a file. The directive is inserted in front of the first *instruction* on the line, so a label on that line keeps pointing where it did; removing the breakpoint removes it again. Set to `false` to leave your files untouched. The word written is `cpclib-lsp.toml`'s `asm.breakpoint_directive`.

- **`cpclib-lsp.trace.server`** (default: `"off"`)  
  LSP communication tracing for debugging. Options: `"off"`, `"messages"`, `"verbose"`.

### LSP Server Configuration

The language server supports additional configuration via `cpclib-lsp.toml` in your workspace root. Generate a default config:

```bash
cpclib-lsp --init-config
```

This creates a `cpclib-lsp.toml` with options for:
- Assembler behavior (syntax strictness, macro expansion limits)
- Diagnostic filtering
- Performance tuning
- Custom firmware routine documentation paths

Update an existing config with new fields:
```bash
cpclib-lsp --update-config
```

## Usage Examples

### Assembly Workflow

1. Create a new file: `demo.asm`
2. Start typing Z80 code — completion suggests instructions as you type
3. Hover over any instruction to see cycles/bytes/flags
4. Press `F12` on a label to jump to its definition
5. Select a block of code to see total cycle count in the status bar
6. Right-click on `INK 1` → "Pick CPC Ink Color" to choose from the CPC palette

### Build Workflow

1. Create or open `bndbuild.yml` in your project
2. Define build rules (e.g., `assemble`, `run`, `test`)
3. Click "▶ Run" above any rule to execute it
4. Watch build output in the "CPClib LSP" panel
5. Errors and warnings appear inline in your source files

## Supported File Types

| Extension | Language | Description |
|-----------|----------|-------------|
| `.asm`, `.z80` | basm | Z80 assembly (basm syntax) |
| `.bnd`, `.build` | bndbuild | Build configuration files |
| `bndbuild.yml` | bndbuild | Build configuration (YAML) |
| `.bas`, `.BAS` | Locomotive BASIC | Amstrad CPC BASIC programs |
| `.CAT`, `.cat`, `.ASC`, `.asc` | CatArt | CatArt catalog BASIC files |

## Troubleshooting

### Extension won't start / No completion/diagnostics

The LSP server may not have started correctly:

1. Open *View → Output* and select "CPClib LSP" from the dropdown
2. Look for startup errors or crashes in the log
3. Enable tracing: set `cpclib-lsp.trace.server` to `"verbose"` in settings
4. **Restart VS Code** after changing settings
5. If the bundled binary doesn't work on your platform, file an issue with:
   - Your OS and architecture
   - The error message from the output panel
   - Output of: `file /path/to/extension/bin/<platform>/cpclib-lsp`

### For Extension Developers: Custom LSP Binary Not Loading

If you've built your own `cpclib-lsp` but the extension still uses the bundled one:

1. Check that `cpclib-lsp.serverPath` points to your custom binary (absolute path)
2. Verify the binary is executable: `chmod +x /path/to/cpclib-lsp`
3. Test it runs: `/path/to/cpclib-lsp --help`
4. Restart VS Code after changing settings

### Build commands show unexpected behavior

If bndbuild tasks behave differently than expected:

1. Check your `bndbuild.yml` syntax (the extension shows diagnostics for errors)
2. The bndbuild functionality is built directly into the LSP server (no separate binary needed)
3. Review build output in the "CPClib LSP" output panel

## Development & Contributing

This extension is part of the [Benediction CPClib project](https://github.com/cpcsdk/rust.cpclib).

**Pull requests are welcome**, especially for:
- Compatibility with other Z80 assemblers (rasm, winape, maxam, etc.)
- Additional code actions and refactoring tools
- Improved syntax highlighting for edge cases
- Documentation improvements

### Building from Source

```bash
git clone https://github.com/cpcsdk/rust.cpclib.git
cd rust.cpclib/cpclib-vscode
npm install
npm run compile
```

Package the extension:
```bash
vsce package
```

### Testing

The extension requires a working `cpclib-lsp` binary. Build it first:

```bash
cd ../cpclib-lsp
cargo build --release
# Ensure target/release/cpclib-lsp is in your PATH or set serverPath in settings
```

Then press `F5` in VS Code to launch the extension development host.

## License

[License information from main project]

## Links

- **Documentation**: https://cpcsdk.github.io/rust.cpclib/
- **GitHub Repository**: https://github.com/cpcsdk/rust.cpclib
- **Issue Tracker**: https://github.com/cpcsdk/rust.cpclib/issues
- **Benediction Discord**: [Join for support and discussions]

## Acknowledgments

This extension leverages:
- The **tower-lsp** Rust framework for Language Server Protocol
- The **cpclib** ecosystem for Amstrad CPC tooling
- Community contributions to Z80 syntax definitions and CPC firmware documentation

---

**Made with ❤️ by Claude & Krusty/Benediction for the Amstrad CPC demo scene**


# CPClib Extension for Zed

**Language support for Amstrad CPC development** in [Zed](https://zed.dev), including:

- **Z80 Assembly** (basm syntax)
- **Bndbuild** (declarative build system)
- **Locomotive BASIC** (Amstrad CPC BASIC)
- **CatArt BASIC** (ASCII-formatted BASIC)

This extension provides full **Language Server Protocol (LSP)** integration via `cpclib-lsp`, delivering:
- 📝 Syntax highlighting
- 🔍 Go to definition
- 💡 Intelligent completions
- 📚 Hover documentation
- 🔴 Real-time diagnostics
- 🏗️ One-click build execution (CodeLens)

---

## Installation

### 1. Install the LSP Server Binary

The extension requires the `cpclib-lsp` binary to be installed on your system.

#### Option A: Install from source (Recommended)

```bash
cargo install --path cpclib-lsp
```

This places `cpclib-lsp` in `~/.cargo/bin`, which the extension will automatically find.

#### Option B: Download pre-built binary

Download the latest release for your platform:
- [Linux](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/cpclib-lsp)
- [Windows](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/cpclib-lsp.exe)
- [macOS](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/cpclib-lsp-macos)

Make it executable and place it in your `PATH`:

```bash
# Linux/macOS
chmod +x cpclib-lsp
sudo mv cpclib-lsp /usr/local/bin/

# Or add to ~/.cargo/bin if you prefer
mv cpclib-lsp ~/.cargo/bin/
```

### 2. Install the Extension

Install from the Zed extension marketplace:

1. Open Zed
2. Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux/Windows)
3. Type "extensions"
4. Search for "CPClib"
5. Click Install

Or install manually by cloning this repository into `~/.config/zed/extensions/`.

### 3. Enable Semantic Tokens (Required for Syntax Highlighting)

Zed requires explicit configuration to enable LSP semantic tokens for syntax highlighting.

Add to your `~/.config/zed/settings.json`:

```json
{
  "lsp": {
    "cpclib-lsp": {
      "initialization_options": {
        "semanticTokens": {
          "enable": true
        }
      }
    }
  },
  "languages": {
    "Basm": {
      "enable_language_server": true,
      "semantic_tokens": true
    },
    "Bndbuild": {
      "enable_language_server": true,
      "semantic_tokens": true
    },
    "Locomotive BASIC": {
      "enable_language_server": true,
      "semantic_tokens": true
    },
    "CatArt BASIC": {
      "enable_language_server": true,
      "semantic_tokens": true
    }
  }
}
```

Without this configuration, all code will appear monochrome.

---

## Features

### 🔧 Z80 Assembly (basm)

Full language support for Z80 assembly with basm syntax:

- **Syntax highlighting** for instructions, registers, labels, macros
- **Go to definition** for labels, macros, constants
- **Intelligent completions** for Z80 instructions and directives
- **Hover documentation** showing instruction syntax and descriptions
- **Diagnostics** for syntax errors, undefined labels, invalid operands
- **Snippets** for common patterns (macros, structs, conditionals)

**Supported file extensions:** `.asm`, `.z80`

#### Example Features:
- Jump to label definitions with `Cmd+Click`
- See all references to a label or macro
- Auto-complete Z80 instructions as you type
- Hover over instructions to see cycle counts and flags

---

### 📦 Build System (bndbuild)

Full integration with the **bndbuild** project automation tool:

- **Syntax highlighting** for `.bnd`, `.build`, and `bndbuild.yml` files
- **CodeLens buttons**: One-click "▶ Run" buttons above each rule definition
- **Real-time diagnostics** for build configuration errors
- **Jinja2 template support** for variable substitution and logic
- **Snippets** for common build patterns

**Supported file names:** `bndbuild.yml`, `build.bnd`, `bnd.build`

#### Example Build File:

```yaml
# Build a simple Z80 program
- tgt: demo.sna
  dep: demo.asm
  cmd: basm demo.asm -o demo.sna --snapshot

- tgt: clean
  phony: true
  cmd: -rm *.sna *.lst *.sym
```

Click the "▶ Run" CodeLens button above any rule to execute it.

---

### 📝 Locomotive BASIC

Language support for Amstrad CPC BASIC:

- **Syntax highlighting** for BASIC keywords and statements
- **Format on type** (automatic indentation)
- **Hover documentation** for BASIC commands
- **Diagnostics** for common errors

**Supported file extensions:** `.bas`, `.BAS`

---

### 🎨 CatArt BASIC

Support for CatArt ASCII-formatted BASIC files:

- **Syntax highlighting**
- **Format on type**
- **Same LSP features as Locomotive BASIC**

**Supported file extensions:** `.cat`, `.CAT`, `.asc`, `.ASC`

---

## Usage

### Opening Files

The extension activates automatically when you open supported file types:
- `.asm` or `.z80` → Z80 Assembly
- `bndbuild.yml`, `.bnd`, `.build` → Bndbuild
- `.bas` → Locomotive BASIC
- `.cat`, `.asc` → CatArt BASIC

### Running Builds

1. Open a `bndbuild.yml` file
2. Click the "▶ Run" CodeLens button above any rule
3. Build output appears in the terminal panel

Or use the command palette:
- Press `Cmd+Shift+P` / `Ctrl+Shift+P`
- Type "run task"
- Select a bndbuild target

---

## Configuration

The extension automatically finds `cpclib-lsp` if it's in:
1. `~/.cargo/bin/cpclib-lsp`
2. Any directory in your `$PATH`

No additional configuration is required for most users.

---

## Troubleshooting

### Extension doesn't activate

**Symptom:** No LSP features, no syntax highlighting

**Solution:**
1. Verify `cpclib-lsp` is installed:
   ```bash
   which cpclib-lsp
   cpclib-lsp --version
   ```
2. Check Zed's log panel (View → Debug → Language Server Logs)
3. Restart Zed

### LSP server crashes or shows errors

**Solution:**
1. Update to the latest `cpclib-lsp`:
   ```bash
   cargo install --path cpclib-lsp --force
   ```
2. Check for error messages in the Debug panel
3. Report issues at: https://github.com/cpcsdk/rust.cpclib/issues

### Build commands fail

**Symptom:** CodeLens "Run" buttons don't work

**Possible causes:**
- bndbuild syntax errors (check diagnostics panel)
- Missing dependencies (e.g., external assemblers like rasm)
- File path issues (use relative paths in build files)

**Solution:**
1. Test the build file manually:
   ```bash
   bndbuild <target-name>
   ```
2. Fix any errors shown in the terminal
3. Reload the file in Zed

---

## Limitations & Differences from VSCode Extension

Due to Zed's extension architecture limitations, some features available in the VSCode extension are **not available** in this Zed extension:

### ❌ Not Available

**NOP Count in Status Bar**
- The VSCode extension shows live NOP/cycle counts for the instruction under the cursor or selected instructions in the status bar
- **Zed limitation:** Extensions cannot add status bar items
- **Alternative:** The LSP already provides this data via the `cpclib.cycleCountForSelection` command, but there's no way for extensions to display it in Zed's UI
- **Workaround:** Use code actions (right-click → "Cycle count for selection") or the code lens "Show cycle count" action

**Code Lens Execution**
- The LSP provides "▶ Run" code lens buttons above build targets
- **Zed limitation:** Extensions cannot intercept or execute code lens actions
- **Alternative:** Use the command palette (`Cmd+Shift+P` → "Run Task") or terminal commands manually:
  ```bash
  bndbuild -f bndbuild.yml <target-name>
  ```

**Register Values Display**
- The VSCode extension shows tracked register values in a status bar item
- **Zed limitation:** No status bar API for extensions
- **Alternative:** Use hover tooltips on instructions (shows register values when statically determinable)

### ✅ Fully Supported

Everything else works identically:
- Syntax highlighting (via LSP semantic tokens)
- Diagnostics (errors/warnings)
- Go to definition
- Hover documentation
- Completions
- Snippets
- Document symbols

---

## Related Tools

This extension is part of the **CPClib** suite:

- **basm** — Z80 assembler with modern features
- **bndbuild** — Makefile-like build automation
- **img2cpc** — Image converter for CPC graphics modes
- **cpclib-disc** — DSK/CDT disc image management
- **cpclib-runner** — Emulator integration and test runners

Learn more: https://github.com/cpcsdk/rust.cpclib

---

## Contributing

Contributions are welcome!

- **Bug reports:** https://github.com/cpcsdk/rust.cpclib/issues
- **Feature requests:** Open a GitHub issue with the `enhancement` label
- **Pull requests:** Fork the repo and submit a PR

---

## License

This extension is released under the same license as the CPClib project (see LICENSE file).

**Author:** Krusty Benediction  
**Repository:** https://github.com/cpcsdk/rust.cpclib  
**Documentation:** https://cpcsdk.github.io/rust.cpclib/

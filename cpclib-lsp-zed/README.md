# CPClib Extension for Zed

**Language support for Amstrad CPC development** in [Zed](https://zed.dev), including:

- **Z80 Assembly** (basm syntax)
- **Bndbuild** (declarative build system)
- **Locomotive BASIC** (Amstrad CPC BASIC)
- **CatArt BASIC** (ASCII-formatted BASIC)
- **CSL** (CPC Script Language)

This extension provides full **Language Server Protocol (LSP)** integration via `cpclib-lsp`, delivering:
- 📝 Syntax highlighting
- 🔍 Go to definition
- 💡 Intelligent completions
- 📚 Hover documentation
- 🔴 Real-time diagnostics
- 🏗️ One-click build execution (▶ run triangles, no setup required)
- 🐛 Source-level debugging (breakpoints, stepping, variables) via `cpclib-dap`

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
- **Runnable targets**: ▶ Run triangles next to build target definitions (requires one-time setup)
- **Real-time diagnostics** for build configuration errors
- **Jinja2 template support** for variable substitution and logic (full support via grammar injection)
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

#### Running Build Targets

No setup needed - the extension ships its own `tasks.json` bound to the
`bndbuild-target` tag. Just click the ▶ triangle next to a target name.

📖 **See [RUNNABLES_SETUP.md](RUNNABLES_SETUP.md) if you want to customize the bundled task (a different `cwd`, `reveal`/`hide` behavior, a watch-mode variant, ...).**

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

### 📜 CSL (CPC Script Language)

Language support for `.csl` scripts (emulator scripting - disk/snapshot
loading, key input, waits):

- **Syntax highlighting** (via LSP semantic tokens)
- **Diagnostics**
- **Same LSP features as the other languages above**

**Supported file extensions:** `.csl`, `.CSL`

---

## Debugging

Source-level debugging - breakpoints, stepping, variables, call stack - is
available through `cpclib-dap`, registered as a Zed debug adapter (adapter
name `cpclib-dap`). Create a `.zed/debug.json` in your project, for example:

```json
[
  {
    "label": "Debug this .asm file",
    "adapter": "cpclib-dap",
    "request": "launch",
    "program": "$ZED_FILE"
  },
  {
    "label": "Debug the demo",
    "adapter": "cpclib-dap",
    "request": "launch",
    "rule": "test_sna"
  }
]
```

Or use Zed's "New Process Debugger..." command and pick `cpclib-dap` -
its generic program field maps onto `program` above; for anything using
`rule`/`buildFile`/`emulator`/`watchLabels`/`topOfStack` instead, write the
scenario directly as shown. The full set of fields is documented in
`debug_adapter_schemas/cpclib-dap.json` and mirrors
`cpclib-vscode`'s own launch config one-for-one, minus `openInWebview`
(there's no Zed equivalent of a webview panel - see Limitations below).

Once a session starts, the emulator's own address - when it serves one
(1984js does; a native emulator window like SugarBox doesn't) - is printed
as a plain, clickable link in the debug console, so you can open it to
actually see/hear the demo run.

Just want to *run* a `.sna`/`.dsk` in a specific emulator, no debugging?
See [EMULATOR_TASKS.md](EMULATOR_TASKS.md) for ready-to-paste tasks (one
per known emulator - these can't ship bundled the way `bndbuild`'s task
does, since Zed only bundles tasks per-language and `.sna`/`.dsk` have none
here).

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
2. Click the ▶ triangle next to any target name
3. Build output appears in the terminal panel

No setup required - see [above](#running-build-targets).

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

## Implementation Status & Limitations

📋 **For detailed implementation status, testing checklist, and known issues, see [`../ZED_IMPLEMENTATION_STATUS.md`](../ZED_IMPLEMENTATION_STATUS.md).**

**Recent Updates (2026-07-31):**
- ✅ Fixed validation false warnings - now uses RULE_KEYS constants for all synonym keys
- ✅ Added runnable code detection (runnables.scm) for build targets with tasks.json setup
- ✅ Implemented Jinja2 template support via grammar injection (bndbuild language now handles both YAML and Jinja)
- ✅ Documented architecture differences between VS Code and Zed approaches
- ❓ Document outline status - LSP provides symbols, Zed display behavior unclear

---

## Limitations & Differences from VSCode Extension

Due to Zed's extension architecture limitations, some features available in the VSCode extension are **not available** in this Zed extension:

### ❌ Not Available

**NOP Count in Status Bar**
- The VSCode extension shows live NOP/cycle counts for the instruction under the cursor or selected instructions in the status bar
- **Zed limitation:** Extensions cannot add status bar items
- **Alternative:** The LSP already provides this data via the `cpclib.cycleCountForSelection` command, but there's no way for extensions to display it in Zed's UI
- **Workaround:** Use code actions (right-click → "Cycle count for selection") or the code lens "Show cycle count" action

**Hardware-Inspection Panels (memory/CRTC/PSG/FDC/PPI/tape/screen/disassembly/BASIC-listing views, the embedded emulator tab, the `.sna`/`.cpr` hex viewer)**
- The VSCode extension renders these as webview panels and custom editor tabs
- **Zed limitation:** the WASM extension API has no panel/webview/custom-editor surface at all - several of these (memory view, disassembly view) are on *Zed core's own* roadmap as future built-in editor features, not something an extension could add
- **Partial alternative:** once a debug session starts, the emulator's own address is printed as a clickable link in the debug console (see Debugging, above) when the emulator serves one (1984js does) - not the panels themselves, but enough to actually see/hear the demo run

**Register Values Display**
- The VSCode extension shows tracked register values in a status bar item
- **Zed limitation:** No status bar API for extensions
- **Alternative:** Use hover tooltips on instructions (shows register values when statically determinable)

**Skipping Full Assembly on Background Tabs**
- The LSP server has a mechanism to skip the expensive multi-pass assemble on a document that isn't the active editor tab - built specifically to avoid a multi-tab workspace restore triggering a full assemble (potentially "tens of seconds on a real demo") on every tab at once
- **How it's supposed to work:** the editor tells the server which document is active via a custom `cpclib.setActiveDocument` notification; the VS Code extension (`cpclib-vscode/src/lsp/activeDocument.ts`) sends this on every `onDidChangeActiveTextEditor`
- **Zed limitation:** this extension never sends that notification - Zed's WASM extension API (`zed_extension_api = "0.7.0"`, see `src/lib.rs`) exposes language-server bootstrapping/configuration hooks, not an "active editor changed" event an extension could forward
- **Effect:** every `.asm` tab opened in Zed gets the full assemble the VS Code-side mechanism exists to avoid - most noticeable restoring a workspace with several files open at once
- **Workaround:** none from the extension side; closing unused tabs limits how many pay the cost at once

### ✅ Fully Supported

Everything else works identically (or should work with proper testing):
- Syntax highlighting (via LSP semantic tokens)
- Diagnostics (errors/warnings)
- Go to definition
- Hover documentation
- Completions
- Snippets
- Document symbols (LSP provides them - Zed display behavior TBD)
- Runnable build targets (▶ triangles, zero setup - a `tasks.json` ships with the extension) and Jinja2-templated bndbuild files (full grammar-injection support - see [JINJA_TEMPLATE_SOLUTION.md](JINJA_TEMPLATE_SOLUTION.md))
- Source-level debugging via `cpclib-dap` (see Debugging, above) - breakpoints, stepping, variables, call stack

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

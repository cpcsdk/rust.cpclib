# Testing the cpclib-vscode extension

## Prerequisites

1. **Build and install the LSP server** (once, or after any change to `cpclib-lsp`):

   ```bash
   cargo install --path cpclib-lsp
   # binary lands at ~/.cargo/bin/cpclib-lsp
   ```

2. **Node.js** ≥ 18 (already installed). Dependencies are already installed and
   TypeScript is already compiled (`out/extension.js` exists).

   If you pull changes: `cd cpclib-vscode && npm install && npm run compile`

---

## Launch the Extension Development Host (F5 workflow)

1. Open the **`cpclib-vscode/` folder** in VS Code:
   ```
   code /path/to/rust.cpcdemotools/cpclib-vscode
   ```

2. Press **F5** (or Run → Start Debugging).
   - VS Code compiles TypeScript in watch mode (background task).
   - A second VS Code window opens labelled **[Extension Development Host]**.

3. In the Extension Development Host window, open any `.asm` or `.bnd`/`.build` file.

---

## What to check

### Syntax highlighting (no LSP needed)
Open any `.asm` file. You should see:
- Instructions (`LD`, `JP`, `CALL` …) highlighted as keywords
- Registers (`HL`, `A`, `BC` …) highlighted as variables
- Labels (`my_label:`) highlighted as function names
- `;` comments greyed out
- Strings and numbers coloured

If colours are wrong: check *View → Command Palette → "Developer: Inspect Editor Tokens and Scopes"*,
click a token to see which grammar scope matched it.

### LSP features (requires cpclib-lsp in PATH)

After opening an `.asm` file, wait ~2 s for the server to start. Verify in:

**View → Output → CPClib LSP** — you should see connection logs.
If you see `failed to start`, the binary is not in PATH; add this to VS Code settings:
```json
"cpclib-lsp.serverPath": "/home/romain/.cargo/bin/cpclib-lsp"
```

Then test each feature:

| Feature | How to trigger |
|---------|----------------|
| **Completion** | Type `LD ` or `JP` and press Ctrl+Space |
| **Hover** | Hover the mouse over `LD`, `HL`, or a label |
| **Document symbols** | Press Ctrl+Shift+O (or click the outline panel) — labels, EQU, macros should appear |
| **Diagnostics** | Introduce a syntax error (e.g. `LDDDD A, B`) — a red squiggle should appear |

### LSP trace (verbose debugging)

Add to VS Code settings, then reload the window:
```json
"cpclib-lsp.trace.server": "verbose"
```
Open **Output → CPClib LSP** — every LSP message is printed.
You can see exactly what the server receives and sends back.

---

## Iterating on the Rust server

After editing `cpclib-lsp/src/*.rs`:

```bash
cargo install --path cpclib-lsp   # reinstalls binary
```

Then in the Extension Development Host: **Ctrl+Shift+P → "Restart Language Server"**
(no need to close and reopen the window).

## Iterating on the TypeScript extension

The background `tsc --watch` task recompiles on every save.
Reload the Extension Development Host with **Ctrl+R** (or Ctrl+Shift+P → "Reload Window").

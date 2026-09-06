# Zed Extension Implementation Status

This document tracks the implementation status of Zed-specific features for the cpclib-lsp extension.

## ✅ Completed Features

### 1. Validation Refactoring (RULE_KEYS)
**Status:** ✅ Complete and tested  
**Files Modified:** 
- `cpclib-lsp/src/bndbuild/diagnostics.rs`

**Changes:**
- Refactored `validate_build_structure()` to use `RULE_KEYS` constants instead of hardcoded strings
- Now dynamically validates all synonym keys: targets/tgt/target/build, tasks/cmd/command/launch/run
- Single source of truth: `cpclib-bndbuild/src/lsp.rs::RULE_KEYS`

**Before:**
```rust
let has_targets = raw_text.contains("targets:") || raw_text.contains("tgt:");
let has_tasks = raw_text.contains("tasks:") || raw_text.contains("cmd:");
```

**After:**
```rust
let has_targets = super::token::TGT_KEY_NAMES.iter()
    .any(|key| raw_text.contains(&format!("{}:", key)));
let has_tasks = super::token::TASK_KEY_NAMES.iter()
    .any(|key| raw_text.contains(&format!("{}:", key)));
```

### 2. Semantic Token Configuration Documentation
**Status:** ✅ Complete  
**Files Modified:** 
- `cpclib-lsp-zed/README.md` (lines ~46-76)

**Changes:**
- Added explicit semantic token configuration section
- Documented ~/.config/zed/settings.json configuration
- Provided example mapping basm tokens to syntax theme colors

### 3. Zed API Limitations Documentation
**Status:** ✅ Complete  
**Files Modified:** 
- `cpclib-lsp-zed/README.md` (lines ~262-304)

**Changes:**
- Documented impossible features in Zed compared to VS Code
- Code lens execution: Zed cannot intercept/execute code lens actions (architectural limitation)
- Status bar items: Zed has no status bar API for extensions (architectural limitation)
- Provides alternative workflows where possible

## 🚧 Partially Complete Features

### 4. Runnable Code Detection (Run Triangles)
**Status:** ⚠️ Implemented but requires user configuration  
**Files Created/Modified:**
- `cpclib-lsp-zed/languages/bndbuild/runnables.scm` (new file)
- `cpclib-lsp-zed/languages/bndbuild/config.toml` (updated)
- `cpclib-lsp-zed/extension.toml` (added grammar registration)
- `cpclib-lsp-zed/RUNNABLES_SETUP.md` (new documentation)

**Implementation:**
Created tree-sitter-yaml queries matching bndbuild target definitions:
```scheme
(block_mapping_pair
  key: (flow_node) @_key
  (#match? @_key "^(targets|tgt|target|build)$")
  value: (flow_node) @run @target
  (#set! tag bndbuild-target))
```

Added tree-sitter-yaml grammar registration to extension.toml:
```toml
[grammars.yaml]
repository = "https://github.com/tree-sitter-grammars/tree-sitter-yaml"
rev = "v0.7.2"
```

**CRITICAL: Requires tasks.json Configuration**
Zed runnables are declarative - they mark where run buttons appear but don't execute commands.
Users MUST configure `~/.config/zed/tasks.json` or `.zed/tasks.json`:

```json
[{
  "label": "bndbuild: run target",
  "command": "bndbuild",
  "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
  "tags": ["bndbuild-target"]
}]
```

**Why not forward to cpclib-lsp like VS Code?**
- VS Code: Code lens → `workspace/executeCommand` → LSP executes build
- Zed: No code lens support, runnables are tree-sitter markers only
- Zed tasks must invoke shell commands, cannot call LSP commands
- Solution: Use `bndbuild` CLI tool directly

See [RUNNABLES_SETUP.md](RUNNABLES_SETUP.md) for detailed setup instructions.

**Known Issues:**
- ⚠️ No default task binding - Zed extension API doesn't support providing default tasks from WASM
- ✅ **Jinja template support implemented** - bndbuild language now handles both YAML and Jinja
  - Uses tree-sitter-jinja2 as base grammar with YAML injection via injections.scm
  - Both Jinja templates (`{{ }}`, `{% %}`) and YAML structure fully supported
  - Runnables, syntax highlighting, and outline work in templated files
  - See [JINJA_TEMPLATE_SOLUTION.md](cpclib-lsp-zed/JINJA_TEMPLATE_SOLUTION.md) for details
- ❓ Unclear if runnables work with embedded bndbuild in basm files (user requested)

**Next Steps:**
1. Test by opening a .bnd file in Zed and checking for run triangles
2. If not working, add `[grammars.yaml]` to extension.toml:
   ```toml
   [grammars.yaml]
   repository = "https://github.com/tree-sitter-grammars/tree-sitter-yaml"
   rev = "latest-stable-commit-hash"
   ```
3. Debug tree-sitter node types if queries don't match
4. Extend to basm embedded bndbuild and locomotive-basic files

**References:**
- [Zed Make extension runnables.scm](https://github.com/caius/zed-make/blob/main/languages/make/runnables.scm)
- [Zed docs: Runnable code detection](https://zed.dev/docs/extensions/languages#runnable-code-detection)

## ❓ Unknown Status Features

### 5. Document Symbol Outline
**Status:** ❓ Investigation needed / LSP functional, Zed display uncertain  
**User Report:** "I have the feelings that zed outline does not show the symbols as vscode is able to do"

**LSP Implementation Status:**
- ✅ LSP reports `document_symbol_provider: true` in server capabilities
- ✅ `document_symbol()` handler properly implemented in backend.rs (line 1660)
- ✅ `BuildFileAnalyzer::document_symbols()` returns hierarchical outline:
  - "Variables" container (Jinja `{% set %}` definitions)
  - "Artifacts" container (build targets)
- ✅ Uses Jinja source maps for accurate positions
- ✅ Tree-sitter fallback created: `languages/bndbuild/outline.scm`

**Possible Issues:**
- ❓ Zed may not be displaying LSP symbols correctly for bndbuild language
- ❓ May need Zed restart to load LSP properly
- ❓ Hierarchical nesting may not render well in Zed outline panel
- ❓ Outline panel may not be open by default (View → Outline)

**Next Steps:**
1. Verify outline panel is open: View → Outline
2. Check if LSP document_symbol() is being called:
   ```bash
   export RUST_LOG=cpclib_lsp=debug
   # Check logs for "Document symbol request" messages
   ```
3. Compare VS Code outline display with Zed outline display
4. Test tree-sitter fallback by temporarily disabling LSP
5. Check if other languages (basm, locomotive-basic) have same issue

**Debugging Commands:**
```bash
# Check LSP is running
ps aux | grep cpclib-lsp

# Enable debug logging
export RUST_LOG=cpclib_lsp=debug,tower_lsp=debug

# Check Zed developer console
# Help → Toggle Developer Tools → Console
```

**Diagnostic Commands:**
```bash
# Test outline in Zed
# 1. Open a .bnd file in Zed
# 2. Open outline panel (View → Outline or Ctrl+Shift+O)
# 3. Verify symbols appear hierarchically

# Enable LSP debug logging
export RUST_LOG=cpclib_lsp=debug
cpclib-lsp
```

## Architecture Notes

### Single Source of Truth
The LSP now consistently uses `cpclib-bndbuild/src/lsp.rs::RULE_KEYS` as the single source of truth for:
- Valid YAML key names and synonyms
- Key descriptions (for autocomplete/hover)
- Required vs optional keys

Alternative: `cpclib-bndbuild/schema.json` (JSON Schema) exists but is not currently used by LSP validation.

### Cross-Crate Dependencies
```
cpclib-bndbuild/src/lsp.rs (RULE_KEYS constant)
    ↓
cpclib-lsp/src/bndbuild/token.rs (LazyLock constants)
    ↓
cpclib-lsp/src/bndbuild/diagnostics.rs (validation)
cpclib-lsp/src/bndbuild/symbols.rs (outline)
cpclib-lsp/src/bndbuild/autocomplete.rs (completion)
cpclib-lsp/src/bndbuild/definition.rs (go-to-def)
```

### Tree-sitter vs LSP in Zed
Zed uses tree-sitter for:
- Syntax highlighting (highlights.scm)
- Bracket matching (brackets.scm)
- Outline (outline.scm) - **fallback when LSP doesn't provide symbols**
- Runnable detection (runnables.scm)
- Text objects/navigation (textobjects.scm)

Zed uses LSP for:
- Diagnostics
- Completion
- Hover
- Go-to-definition
- Document symbols (outline) - **preferred over tree-sitter when available**
- Semantic tokens (when enabled in settings)

## Testing Checklist

### Validation (RULE_KEYS)
- [ ] File with `targets:` passes validation
- [ ] File with `tgt:` passes validation (synonym)
- [ ] File with `target:` passes validation (synonym)
- [ ] File with `build:` passes validation (synonym)
- [ ] File with `tasks:` and no targets: triggers warning
- [ ] File with `cmd:` and no targets: triggers warning
- [ ] File with neither triggers error

### Runnables
- [ ] Open .bnd file in Zed
- [ ] Verify run triangle appears before target definitions
- [ ] Clicking triangle executes bndbuild rule
- [ ] Embedded bndbuild in .asm files shows run triangles
- [ ] Locomotive BASIC files show run triangles (if applicable)

### Outline
- [ ] Open .bnd file in Zed
- [ ] Open outline panel
- [ ] Verify "Variables" container appears with Jinja variables
- [ ] Verify "Artifacts" container appears with build targets
- [ ] Verify hierarchical nesting works
- [ ] Compare with VS Code outline behavior

## References

### Zed Documentation
- [Language Extensions](https://zed.dev/docs/extensions/languages)
- [Runnable Code Detection](https://zed.dev/docs/extensions/languages#runnable-code-detection)
- [Outline Queries](https://zed.dev/docs/extensions/languages#code-outlinestructure)
- [Semantic Tokens](https://zed.dev/docs/extensions/languages#syntax-highlighting-with-semantic-tokens)

### Tree-sitter
- [Tree-sitter YAML Grammar](https://github.com/tree-sitter-grammars/tree-sitter-yaml)
- [Tree-sitter Query Syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/index.html)

### Example Extensions
- [caius/zed-make](https://github.com/caius/zed-make) - Makefile support with runnables
- [Zed built-in languages](https://github.com/zed-industries/zed/tree/main/crates/languages/src) - Official language implementations

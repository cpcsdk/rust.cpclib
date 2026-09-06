# Testing Guide for Zed Extension Updates

This guide helps you test the recent changes to the cpclib-lsp Zed extension.

## Prerequisites

1. **Updated LSP binary installed:**
   ```bash
   cd /home/romain/Perso/CPC/rust.cpcdemotools
   cargo install --path cpclib-lsp
   ```

2. **Verify installation:**
   ```bash
   which cpclib-lsp
   cpclib-lsp --help
   ```

3. **Zed extension up to date:**
   - Extension files updated in `cpclib-lsp-zed/`
   - Restart Zed after any extension file changes

## Test 1: Validation with Synonym Keys

**What was fixed:** Validation now checks all synonym keys dynamically from RULE_KEYS constant.

### Test Case 1.1: File with `tgt:` synonym
Create `test_tgt.yml`:
```yaml
{% set OUTPUT = "test.sna" %}

tgt: [{{ OUTPUT }}]
cmd: orgams example.asm -o {{ OUTPUT }}
```

**Expected:** No validation warnings/errors about missing "targets:" or "tasks:" section.

### Test Case 1.2: File with `cmd:` synonym
Create `test_cmd.yml`:
```yaml
targets: output.bin
cmd: basm input.asm -o output.bin
```

**Expected:** No validation warnings/errors.

### Test Case 1.3: File with only `tasks:` (no targets)
Create `test_no_targets.yml`:
```yaml
tasks: echo "Hello"
```

**Expected:** ⚠️ Warning "Build file should contain 'targets:' or 'tgt:' section"

### Test Case 1.4: File with all synonyms
Create `test_all_synonyms.yml`:
```yaml
build: test.bin      # synonym for targets
dep: input.asm       # synonym for dependencies
command: basm input.asm -o test.bin  # synonym for tasks
```

**Expected:** ✅ No warnings (has target synonym, has task synonym).

**How to test in Zed:**
1. Open the test file in Zed
2. Check Problems panel (View → Problems) for diagnostics
3. Verify warnings/errors match expected behavior

---

## Test 2: Runnable Code Detection (Run Triangles)

**What was fixed:** Validation now checks all synonym keys dynamically from RULE_KEYS constant.
**CRITICAL:** Runnables require tasks.json configuration - see [RUNNABLES_SETUP.md](cpclib-lsp-zed/RUNNABLES_SETUP.md)

### Prerequisites for Test 2

**BEFORE TESTING:** Create tasks configuration file:

```bash
# Global configuration (all projects)
mkdir -p ~/.config/zed
cat > ~/.config/zed/tasks.json << 'EOF'
[
  {
    "label": "bndbuild: run target",
    "command": "bndbuild",
    "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
    "tags": ["bndbuild-target"],
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always"
  }
]
EOF

# OR: Project-specific configuration
mkdir -p /path/to/project/.zed
cat > /path/to/project/.zed/tasks.json << 'EOF'
[
  {
    "label": "bndbuild: run target",
    "command": "bndbuild",
    "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
    "tags": ["bndbuild-target"],
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always"
  }
]
EOF
```

**Then restart Zed** to load the task configuration.

### Test Case 2.1: Simple bndbuild file
Create `test_runnables.yml`:
```yaml
targets: hello.bin
tasks: echo "Building hello"

targets: world.bin  
tasks: echo "Building world"
```

**Expected:** 
- ▶ Triangle icon appears to the left of each `targets:` line
- Clicking triangle executes the corresponding build rule
- Both "hello.bin" and "world.bin" rules are runnable

### Test Case 2.2: Synonym keys
Create `test_runnables_synonyms.yml`:
```yaml
tgt: using_tgt.bin
cmd: echo "Using tgt synonym"

build: using_build.bin
command: echo "Using build synonym"
```

**Expected:**
- ▶ Triangle appears for both `tgt:` and `build:` lines
- Runnables.scm pattern matches: `^(targets|tgt|target|build)$`

### Test Case 2.3: Embedded bndbuild in basm
Create `test_embedded.asm`:
```asm
; Some Z80 assembly
ld a, 5

; BUILD
targets: embedded.bin
tasks: basm test_embedded.asm -o embedded.bin
; BUILEND
```

**Expected (uncertain):**
- ❓ May or may not show run triangle for embedded build block
- Depends on whether basm grammar includes bndbuild injection queries
- See ZED_IMPLEMENTATION_STATUS.md for current limitations

**How to test in Zed:**
1. Open the test file in Zed
2. Look for ▶ triangle icons in the left margin (gutter)
3. Hover over triangle to see target name/description
4. Click triangle to execute the build rule
5. Verify output appears in Zed's terminal panel

**Debugging if runnables don't work:**
1. **Most common issue:** Missing tasks.json configuration
   - Verify file exists: `cat ~/.config/zed/tasks.json`
   - Check tag matches: `"tags": ["bndbuild-target"]`
   - Restart Zed after creating/editing tasks.json

2. Check if tree-sitter-yaml grammar is loaded:
   - Open Zed
   - Open command palette: `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux)
   - Type "zed: reload"
   - Check if error messages appear in Zed's log console

3. Verify runnables.scm syntax:
   ```bash
   cd cpclib-lsp-zed/languages/bndbuild
   cat runnables.scm
   # Should contain tree-sitter queries with @run captures
   ```

4. Verify extension.toml has grammar reference:
   ```bash
   grep -A 2 '\[grammars.yaml\]' cpclib-lsp-zed/extension.toml
   # Should show yaml grammar repository and rev
   ```

5. Enable Zed developer tools:
   - Help → Toggle Developer Tools
   - Check console for extension loading errors

---

## Test 3: Document Outline (Symbols)

**What to verify:** LSP provides document symbols; check if Zed displays them correctly.
**IMPORTANT:** Make sure outline panel is open: View → Outline (or `Cmd+Shift+O` / `Ctrl+Shift+O`)

### Test Case 3.1: Simple outline
Create `test_outline.yml`:
```yaml
{% set VAR1 = "value1" %}
{% set VAR2 = "value2" %}

targets: artifact1.bin
tasks: echo "Artifact 1"

targets: artifact2.bin
dependencies: artifact1.bin
tasks: echo "Artifact 2"
```

**Expected outline hierarchy:**
```
📁 Variables
  ├─ VAR1: value1
  └─ VAR2: value2
📁 Artifacts
  ├─ artifact1.bin
  └─ artifact2.bin
```

**How to test in Zed:**
1. Open the test file in Zed
2. Open outline panel: View → Outline (or `Cmd+Shift+O` / `Ctrl+Shift+O`)
3. Verify two top-level containers: "Variables" and "Artifacts"
4. Verify each Jinja variable appears under Variables with its value
5. Verify each build target appears under Artifacts
6. Verify you can click outline entries to jump to their location

**Compare with VS Code (if available):**
1. Open same file in VS Code with cpclib-vscode extension
2. Open outline panel (View → Outline)
3. Compare structure and content with Zed's outline
4. Report discrepancies in GitHub issue

**If outline doesn't work in Zed:**
1. Check LSP is running:
   ```bash
   ps aux | grep cpclib-lsp
   ```

2. Enable LSP debug logging:
   ```bash
   export RUST_LOG=cpclib_lsp=debug
   # Restart Zed
   ```

3. Check Zed logs for document_symbol requests:
   - Help → Toggle Developer Tools → Console
   - Look for "Document symbol request for" messages

4. May need to create outline.scm as tree-sitter fallback (see ZED_IMPLEMENTATION_STATUS.md)

---

## Test 4: Semantic Tokens (Visual Verification)

**What was documented:** Semantic token configuration for proper syntax highlighting.

### Test Case 4.1: Configure semantic tokens in Zed
Edit `~/.config/zed/settings.json`:
```json
{
  "languages": {
    "Basm": {
      "semantic_tokens": "combined"
    },
    "Bndbuild": {
      "semantic_tokens": "combined"
    }
  },
  "global_lsp_settings": {
    "semantic_token_rules": [
      {
        "token_type": "macro",
        "style": ["function"]
      },
      {
        "token_type": "number",
        "style": ["number"]
      }
    ]
  }
}
```

### Test Case 4.2: Visual verification
Create `test_semantic.asm`:
```asm
; Test semantic tokens
ld a, 42        ; number literal
call MyRoutine  ; label
MY_MACRO        ; macro
```

**Expected:**
- Number `42` highlighted as number
- `MyRoutine` highlighted as function/label
- `MY_MACRO` highlighted as macro (if semantic token rules configured)

**How to test:**
1. Open file in Zed with semantic tokens enabled
2. Verify syntax highlighting matches configured token rules
3. Compare with VS Code to ensure similar appearance

---

## Reporting Issues

If you find problems during testing:

1. **Check ZED_IMPLEMENTATION_STATUS.md** for known issues
2. **Collect diagnostic info:**
   ```bash
   # LSP version
   cpclib-lsp --help | head -1
   
   # Zed version
   zed --version
   
   # Extension files
   ls -la cpclib-lsp-zed/languages/bndbuild/
   ```

3. **Enable debug logging:**
   ```bash
   export RUST_LOG=cpclib_lsp=debug,tower_lsp=debug
   ```

4. **Create GitHub issue** with:
   - Test case that fails
   - Expected vs actual behavior  
   - Zed version and LSP version
   - Relevant log output
   - Screenshots if visual issue

---

## Success Criteria

✅ **Validation:** All synonym keys pass validation, no false warnings  
✅ **Runnables:** Run triangles appear after tasks.json configured, execute correctly  
✅ **Outline:** Hierarchical symbol structure appears (may need Zed restart / panel open)  
✅ **Semantic tokens:** Syntax highlighting works with configuration  

**Note:** Runnables and outline are considered successful if:
- Runnables: Triangles appear and execute after tasks.json is configured
- Outline: Symbols appear when outline panel is open (View → Outline)

If all tests pass, update ZED_IMPLEMENTATION_STATUS.md to mark features as ✅ Complete and tested.

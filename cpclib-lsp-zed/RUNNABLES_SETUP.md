# Zed Runnables Setup

The cpclib-lsp Zed extension provides runnable code detection for bndbuild
files, bound to a task by default - `languages/bndbuild/tasks.json`, shipped
with the extension, runs `cpclib-lsp bndbuild -f <file> <target>` (the same
single-binary invocation VS Code's own task provider uses - no separate
`bndbuild` install needed). Nothing below is required for the default case;
it's kept for anyone who wants to override it.

## Quick Setup (only if you want to customize the default)

1. **Create tasks configuration:**
   Edit `~/.config/zed/tasks.json` (global) or `.zed/tasks.json`
   (project-specific) - a task there with the same `bndbuild-target` tag
   takes precedence over the bundled one:

```json
[
  {
    "label": "bndbuild: run target",
    "command": "bndbuild",
    "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
    "tags": ["bndbuild-target"],
    "reveal": "always"
  }
]
```

2. **Restart Zed** to load the task configuration

3. **Test it:** Open a .bnd file and look for ▶ triangles next to `targets:` lines

## Architecture: Why Not Forward to LSP?

**VS Code approach:**
- Code lens → `workspace/executeCommand` → LSP runs build internally
- LSP has `cpclib.runRule` command that executes builds

**Zed limitation:**
- Zed doesn't support code lens (architectural limitation)
- Zed runnables are tree-sitter markers, not executable commands
- Runnables must be bound to shell tasks in tasks.json
- No mechanism to invoke LSP commands from tasks

**Therefore:** We use the `bndbuild` CLI tool directly instead of delegating to the LSP.

## How It Works

The extension uses the YAML grammar for bndbuild files:

1. **Detection:** The `runnables.scm` query detects YAML `block_mapping_pair` nodes with build target keys
2. **Environment variable:** Target names are exported as `$ZED_CUSTOM_target`
3. **Task binding:** The `bndbuild-target` tag connects runnables to your task template
4. **Execution:** Clicking ▶ runs: `bndbuild -f <file> <target>`

This works for standard YAML bndbuild files.

## Customization

### Run from project directory (recommended)
```json
{
  "label": "bndbuild: run target",
  "command": "bndbuild",
  "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
  "tags": ["bndbuild-target"],
  "cwd": "$ZED_WORKTREE_ROOT"
}
```

### Hide terminal on success
```json
{
  "label": "bndbuild: run target",
  "command": "bndbuild",
  "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
  "tags": ["bndbuild-target"],
  "hide": "on_success"
}
```

### Watch mode (rebuild on file changes)
```json
{
  "label": "bndbuild: watch target",
  "command": "bndbuild",
  "args": ["-f", "$ZED_FILE", "-w", "$ZED_CUSTOM_target"],
  "tags": ["bndbuild-target"],
  "allow_concurrent_runs": false
}
```

## Troubleshooting

### No triangles appear
1. Check that tree-sitter-yaml grammar is installed (should be automatic)
2. Restart Zed after installing the extension
3. Open a .bnd or .yml file with `targets:` or `tgt:` keys

### Clicking triangle does nothing
1. Verify task is configured in tasks.json
2. Check the tag matches: `"tags": ["bndbuild-target"]`
3. Open command palette and search "task: spawn" to see if task appears

### Command not found
1. Ensure `bndbuild` is installed: `cargo install --path cpclib-bndbuild`
2. Check it's in your PATH: `which bndbuild`
3. Check `terminal.shell` settings in Zed
4. Try absolute path: `"command": "/home/user/.cargo/bin/bndbuild"`

### Wrong target is run
1. The `$ZED_CUSTOM_target` variable captures the target value
2. For multi-line targets, only flow nodes work currently:
   ```yaml
   targets: myfile.bin  # ✅ Works
   tgt: [multiple.bin]   # ✅ Works (first item)
   build:                # ❌ Doesn't work (block node)
     - file1.bin
   ```

### Jinja Templates — Known Limitation

**Current Status:** Jinja template support is **not yet working** in Zed due to grammar loading issues.

The bndbuild language currently uses the YAML grammar:
- ✅ Works for pure YAML bndbuild files
- ❌ Does not work for Jinja-templated files (Jinja syntax appears as errors)
- ❌ Run triangles may not appear in files with Jinja templates

**Why this is hard to fix:**
1. Zed extensions can't reference custom grammars from the same extension during load
2. The zed-jinja-universal approach creates **separate languages** (e.g., `yaml_jinja`) for each Jinja variant
3. We'd need to either:
   - Create a separate `bndbuild_jinja` language (but users want one language, not two)
   - Wait for Zed to support grammar injection during extension load
   - Find a different technical approach

**Workarounds:**
1. Use pure YAML bndbuild files (no Jinja templates) ✅
2. If you need Jinja: runnables won't work, but the `bndbuild` CLI still supports templated files
3. Run bndbuild commands manually from the terminal

**Tracking:** See [JINJA_IMPLEMENTATION.md](JINJA_IMPLEMENTATION.md) for attempted solutions and technical details.
```yaml
{% set output = "demo.sna" %}
- tgt: {{ output }}  # ✅ Run triangle appears!
  dep: demo.asm
  
{% for item in ["game", "demo", "test"] %}
- tgt: {{ item }}.bin  # ✅ Run triangles for all targets!
  dep: {{ item }}.asm
{% endfor %}
```

**How it works:**
1. tree-sitter-jinja2 parses the entire file (handles `{{ }}`, `{% %}` syntax)
2. `injections.scm` injects tree-sitter-yaml into content sections (between Jinja tags)
3. `runnables.scm` matches YAML nodes from the injected grammar
4. Result: Run triangles appear next to targets in templated files!

**Technical details:** See [JINJA_TEMPLATE_SOLUTION.md](JINJA_TEMPLATE_SOLUTION.md) for the complete implementation.

**No workarounds needed!** The extension now handles mixed Jinja+YAML files natively.

## Reference

- **Tag name:** `bndbuild-target`
- **Environment variables:**
  - `$ZED_FILE` - Current .bnd file path
  - `$ZED_CUSTOM_target` - Target name/path from the YAML
  - `$ZED_WORKTREE_ROOT` - Project root directory
- **Detected patterns:** `targets:`, `tgt:`, `target:`, `build:` (all synonyms)

## Example tasks.json

Full example with multiple options:

```json
[
  {
    "label": "bndbuild: run target",
    "command": "bndbuild",
    "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
    "tags": ["bndbuild-target"],
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always",
    "hide": "on_success",
    "use_new_terminal": false,
    "allow_concurrent_runs": true
  },
  {
    "label": "bndbuild: build all",
    "command": "bndbuild",
    "args": ["-f", "$ZED_FILE"],
    "cwd": "$ZED_WORKTREE_ROOT"
  },
  {
    "label": "bndbuild: show config",
    "command": "bndbuild",
    "args": ["-f", "$ZED_FILE", "--show"],
    "cwd": "$ZED_WORKTREE_ROOT"
  }
]
```

For more on Zed tasks, see: https://zed.dev/docs/tasks

# Zed vs VS Code: Runnables Architecture

## Problem Statement
Users expected Zed to show run triangles (▶) for bndbuild targets similar to VS Code's code lens functionality.

## VS Code Architecture

**Flow:**
```
User clicks code lens → workspace/executeCommand → cpclib-lsp → runs build internally
```

**Details:**
1. **Code Lens Provider**: LSP server implements `textDocument/codeLens`
2. **Command Advertisement**: Server advertises `cpclib.runRule` via `executeCommandProvider` capability
3. **Client Bridge**: vscode-languageclient auto-registers a bridge for advertised commands
4. **Execution**: Clicking "▶ Run" sends `workspace/executeCommand` to LSP
5. **Internal Build**: LSP's `execute_command` handler runs `BuildFileAnalyzer::run_rule()`

**Code Reference:**
```typescript
// cpclib-vscode/src/extension.ts
// NOTE: do NOT register `cpclib.runRule` here. The server advertises it in
// its `executeCommandProvider` capability and vscode-languageclient
// auto-registers a bridge for every advertised command
```

```rust
// cpclib-lsp/src/server/backend.rs
async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
    if params.command == "cpclib.runRule" {
        // Extract rule name and filename from params
        // Run BuildFileAnalyzer::run_rule()
        // Stream output via log_message
    }
}
```

## Zed Architecture Limitations

**Why we can't forward to LSP:**
1. ❌ Zed doesn't support code lens (architectural limitation)
2. ❌ Zed runnables are tree-sitter queries, not LSP features
3. ❌ Zed tasks can only invoke shell commands, not LSP commands
4. ❌ Zed WASM extension API has no mechanism to call `workspace/executeCommand`

**What Zed provides:**
- Tree-sitter queries mark runnable locations with `@run` capture
- Captured values become environment variables (e.g., `$ZED_CUSTOM_target`)
- Tasks in `tasks.json` bind to runnables via matching tags
- Tasks invoke shell commands with environment variable substitution

**Flow:**
```
Tree-sitter query → @run @target captures → tasks.json tag match → shell command
```

## Our Solution: Use `bndbuild` CLI

Since Zed tasks must invoke shell commands, we use the `bndbuild` CLI tool directly instead of delegating to the LSP.

**Implementation:**

1. **Tree-sitter query** (`runnables.scm`):
```scheme
(block_mapping_pair
  key: (flow_node) @_key
  (#match? @_key "^(targets|tgt|target|build)$")
  value: (flow_node) @run @target
  (#set! tag bndbuild-target))
```

2. **Task binding** (`~/.config/zed/tasks.json`):
```json
[{
  "label": "bndbuild: run target",
  "command": "bndbuild",
  "args": ["-f", "$ZED_FILE", "$ZED_CUSTOM_target"],
  "tags": ["bndbuild-target"]
}]
```

3. **Execution:**
```bash
bndbuild -f /path/to/build.bnd my_program.bin
```

## Trade-offs

### ✅ Advantages
- Works within Zed's architectural constraints
- Uses official `bndbuild` CLI tool (same result as LSP)
- Simple, transparent execution model
- Easy to customize (flags, cwd, output handling)

### ⚠️ Disadvantages
- Requires manual tasks.json setup (one-time configuration)
- No default task binding (Zed WASM API limitation)
- Duplicates some logic (LSP and CLI both run builds)

## Why Not Create a Bridge Script?

**Considered approach:**
Create a shell script that forwards to LSP via JSON-RPC.

**Why rejected:**
1. Complex: Need to maintain LSP JSON-RPC protocol manually
2. Fragile: Stdio/socket management, error handling
3. Unnecessary: `bndbuild` CLI already provides the functionality
4. Non-standard: Goes against Zed's shell-task design

## Conclusion

**For VS Code**: LSP command execution is the right approach (native code lens support).

**For Zed**: Direct CLI invocation is the right approach (architectural fit).

Both achieve the same result - running bndbuild targets from the editor - using the appropriate extension model for each editor.

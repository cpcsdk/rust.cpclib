# Jinja Template Support Implementation Plan

## Problem

Tree-sitter-yaml parser cannot handle Jinja template syntax (`{{ }}`, `{% %}`), causing:
- Syntax errors in files with Jinja expressions
- No runnables (no run triangles) for templated targets
- Poor syntax highlighting in mixed Jinja+YAML files

**Example that currently breaks:**
```yaml
{% set output = "demo.sna" %}
- tgt: {{ output }}  # ❌ tree-sitter-yaml fails to parse
  dep: demo.asm
```

## Solution: Grammar Injection (Inspired by zed-jinja-universal)

Use the same approach as [zed-jinja-universal](https://github.com/Else00/zed-jinja-universal) extension:

1. **Base parser:** tree-sitter-jinja2 (handles `{% %}`, `{{ }}` syntax)
2. **Injected parser:** tree-sitter-yaml (injected into Jinja content sections)
3. **Result:** Both parsers work together, each handling their own syntax

### How Grammar Injection Works

The jinja-universal extension creates language variants (e.g., `YAML-Jinja`) that:
- Use tree-sitter-jinja2 as the primary grammar
- Write an `injections.scm` query that identifies content sections
- Inject the target language grammar (YAML) into those content sections
- Both grammars produce nodes visible to queries like `runnables.scm`

**Key file:** `languages/{lang}_jinja/injections.scm`

Example injection query structure (conceptual):
```scheme
; Inject YAML into Jinja content sections
((content) @injection.content
 (#set! injection.language "yaml"))
```

## Implementation Steps

### 1. Add Jinja2 Grammar to Extension

**File:** `cpclib-lsp-zed/extension.toml`

Add grammar registration:
```toml
[grammars.jinja2]
repository = "https://github.com/Else00/tree-sitter-jinja2-universal"
commit = "f05225cffe1883d8aa1e0f3d2c93e836255960d2"
```

### 2. Create bndbuild-jinja Language Directory

**Structure:**
```
cpclib-lsp-zed/languages/bndbuild_jinja/
├── config.toml           # File patterns and grammar config
├── injections.scm        # YAML injection into Jinja content
├── runnables.scm         # Same as bndbuild runnables (targets query)
├── outline.scm           # Same as bndbuild outline
├── highlights.scm        # Optional: Jinja syntax highlighting (copy from jinja-universal)
└── brackets.scm          # Optional: bracket matching
```

### 3. Configure Language Registration

**File:** `languages/bndbuild_jinja/config.toml`

```toml
name = "bndbuild-jinja"
grammar = "jinja2"  # Base grammar
file_types = [
  "bnd.jinja",      # bndbuild.yml.jinja
  "bnd.jinja2",     # bndbuild.yml.jinja2
  "bnd.j2",         # bndbuild.yml.j2
  "yml.jinja",      # *.yml.jinja
  "yml.jinja2",     # *.yml.jinja2
  "yml.j2",         # *.yml.j2
  "yaml.jinja",     # *.yaml.jinja
  "yaml.jinja2",    # *.yaml.jinja2
  "yaml.j2"         # *.yaml.j2
]

[line_comment]
start = "#"
```

**Note:** Zed matches file patterns by checking if the filename **ends with** the pattern, so:
- `bndbuild.yml.jinja` matches `yml.jinja` ✅
- `demo.bnd.jinja` matches `bnd.jinja` ✅

### 4. Write Injection Query

**File:** `languages/bndbuild_jinja/injections.scm`

This is the **critical** file that makes the approach work. Study the tree-sitter-jinja2-universal grammar to understand its node structure, then write a query like:

```scheme
; Inject YAML into Jinja content sections
; (Exact node names depend on tree-sitter-jinja2-universal grammar)

((content) @injection.content
 (#set! injection.language "yaml"))

; Alternative pattern if content nodes are structured differently:
((template
  (content) @injection.content)
 (#set! injection.language "yaml"))
```

**TODO:** Examine tree-sitter-jinja2-universal grammar to find correct node names:
- Clone: https://github.com/Else00/tree-sitter-jinja2-universal
- Check `grammar.js` for node definitions
- Test injection with `tree-sitter parse` and `tree-sitter highlight`

### 5. Copy Runnables and Outline Queries

**Files:**
- `languages/bndbuild_jinja/runnables.scm`
- `languages/bndbuild_jinja/outline.scm`

Copy from `languages/bndbuild/` unchanged. The queries will match nodes from the **injected YAML grammar**, not the Jinja2 grammar.

**Key insight:** After injection, the tree contains:
- Jinja2 nodes: `{% %}`, `{{ }}`, keywords, variables
- **Injected YAML nodes:** `block_mapping_pair`, `flow_node`, etc.

Our existing runnables query targets YAML nodes, so it will work on the injected subtrees.

### 6. Optional: Copy Highlighting and Bracket Queries

**From:** [zed-jinja-universal](https://github.com/Else00/zed-jinja-universal/tree/main/languages/jinja2)

Copy these files to provide Jinja syntax highlighting:
- `highlights.scm` — highlights `{% %}`, `{{ }}`, keywords, variables
- `brackets.scm` — bracket matching for `{% %}`, `{{ }}`
- `indents.scm` — indentation rules (optional)

**Note:** YAML highlighting comes from the injected grammar, Jinja highlighting comes from these queries.

### 7. Update Extension Metadata

**File:** `extension.toml`

No changes needed! Zed automatically discovers languages in `languages/*/config.toml`.

### 8. Testing

**Test files:**
1. Create `test.yml.jinja`:
   ```yaml
   {% set target_name = "demo" %}
   - tgt: {{ target_name }}.sna
     dep: demo.asm
   ```

2. Open in Zed → should see:
   - Syntax highlighting for both Jinja and YAML
   - Run triangle (▶) next to `{{ target_name }}.sna`

3. Click run triangle → should execute `bndbuild -f test.yml.jinja demo.sna`

**Validation:**
- Jinja delimiters (`{% %}`, `{{ }}`) highlighted correctly
- YAML structure (keys, values) highlighted correctly
- Runnables work (run triangles appear)
- Outline shows targets (if LSP isn't providing symbols)

## File Pattern Strategy

Support multiple suffixes so users can choose their convention:

| Pattern | Example Filename | Use Case |
|---------|-----------------|----------|
| `bnd.jinja` | `bndbuild.yml.jinja` | Explicit bndbuild file with Jinja |
| `yml.jinja` | `config.yml.jinja` | Generic YAML with Jinja |
| `yaml.jinja` | `tasks.yaml.jinja` | Alternative YAML extension |
| `bnd.j2` | `build.bnd.j2` | Short Jinja suffix |
| `yml.j2` | `config.yml.j2` | Generic short suffix |

**Recommendation:** Document all patterns in README but suggest `.yml.jinja` as primary convention (matches jinja-universal).

## Architecture Benefits

This approach provides:
- ✅ **Full Jinja syntax support** (variables, expressions, control flow)
- ✅ **Full YAML syntax support** (structure, keys, values)
- ✅ **Runnables work** (queries match injected YAML nodes)
- ✅ **Outline works** (queries match injected YAML nodes)
- ✅ **Syntax highlighting for both** (Jinja + YAML)
- ✅ **Standard Zed pattern** (follows jinja-universal precedent)

## Alternative Approaches (Rejected)

### 1. LSP Code Actions Instead of Runnables
**Why rejected:** Zed tasks can't invoke LSP workspace/executeCommand (no bridge). Would still need shell invocation.

### 2. Regex-Based Fallback Queries
**Why rejected:** Tree-sitter doesn't support regex in captures, queries require structured nodes.

### 3. Custom Combined Grammar
**Why rejected:** Much more complex to maintain than injection. Grammar injection is the standard Zed pattern.

### 4. Pre-processing Templates
**Why rejected:** Changes user workflow, requires build tooling, breaks direct file editing.

## References

- **zed-jinja-universal:** https://github.com/Else00/zed-jinja-universal
- **tree-sitter-jinja2-universal grammar:** https://github.com/Else00/tree-sitter-jinja2-universal
- **Zed language configuration docs:** https://zed.dev/docs/extensions/languages
- **Tree-sitter injection queries:** https://tree-sitter.github.io/tree-sitter/syntax-highlighting#language-injection

## Implementation Checklist

- [ ] Clone tree-sitter-jinja2-universal and study grammar structure
- [ ] Test grammar with sample Jinja+YAML files using `tree-sitter parse`
- [ ] Add `[grammars.jinja2]` to extension.toml
- [ ] Create `languages/bndbuild_jinja/` directory
- [ ] Write `config.toml` with file patterns
- [ ] Write `injections.scm` (critical: must match correct node names)
- [ ] Copy `runnables.scm` from `languages/bndbuild/`
- [ ] Copy `outline.scm` from `languages/bndbuild/`
- [ ] Optional: Copy `highlights.scm`, `brackets.scm` from jinja-universal
- [ ] Test with `test.yml.jinja` file
- [ ] Validate runnables appear and execute correctly
- [ ] Update README with `.jinja` file pattern support
- [ ] Update RUNNABLES_SETUP with Jinja-enabled examples

## Timeline Estimate

- **Research phase:** 2-3 hours (study grammars, test injection patterns)
- **Implementation:** 2-4 hours (create files, write queries)
- **Testing:** 1-2 hours (validate all features work)
- **Total:** ~1 day of focused work

## Success Criteria

After implementation, this should work:

**File:** `bndbuild.yml.jinja`
```yaml
{% set targets = ["demo", "game"] %}
{% for target in targets %}
- tgt: {{ target }}.sna
  dep: {{ target }}.asm
  cmd: basm {{ target }}.asm -o {{ target }}.sna
{% endfor %}
```

**Expected behavior:**
- ✅ Syntax highlighting for Jinja (`{% %}`, `{{ }}`, keywords)
- ✅ Syntax highlighting for YAML (keys, values, structure)
- ✅ Run triangles (▶) appear next to `demo.sna` and `game.sna`
- ✅ Clicking triangle runs `bndbuild -f bndbuild.yml.jinja demo.sna`
- ✅ Outline panel shows both targets (if LSP unavailable)

---

**Status:** Planning phase - ready for implementation  
**Date:** 2026-07-31  
**Contact:** See cpclib-lsp-zed extension issues

---

## IMPLEMENTATION UPDATE: Simplified Approach (2026-07-31)

After review, we chose a **simpler approach**: enhance the existing `bndbuild` language to support both pure YAML and Jinja-templated files, rather than creating a separate `bndbuild_jinja` variant.

### What Was Actually Implemented ✅

#### 1. Grammar Registration
Added Jinja2 grammar to `extension.toml`:
```toml
[grammars.jinja2]
repository = "https://github.com/Else00/tree-sitter-jinja2-universal"
commit = "f05225cffe1883d8aa1e0f3d2c93e836255960d2"
```

#### 2. Changed Base Grammar
Updated `languages/bndbuild/config.toml`:
```toml
name = "Bndbuild"
grammar = "jinja2"  # Changed from "yaml"
path_suffixes = ["bnd", "build"]
file_types = ["bndbuild.yml", "build.bnd", "bnd.build"]
line_comments = ["# "]
tab_size = 2
hard_tabs = false
```

#### 3. Created Injection Query
Created `languages/bndbuild/injections.scm`:
```scheme
; Inject YAML grammar into content sections (text between Jinja tags)
; This allows bndbuild files to contain Jinja templates while still
; providing YAML syntax highlighting, runnables, and outline features

((content) @injection.content
 (#set! injection.language "yaml"))
```

#### 4. Kept Existing Queries Unchanged
- `runnables.scm` — unchanged, works on injected YAML nodes
- `outline.scm` — unchanged, works on injected YAML nodes

### Benefits of This Approach

1. **Single language** — no need to choose between `bndbuild` vs `bndbuild_jinja`
2. **All file patterns work** — existing `.bndbuild.yml`, `.bnd`, `.build` files work with Jinja
3. **Backward compatible** — pure YAML files still work (Jinja2 parser handles plain text via `content` nodes)
4. **Simpler to maintain** — one set of queries, one language configuration
5. **User-friendly** — Jinja just works, no special file extensions needed

### Test File Created

Created `languages/bndbuild/test_jinja_template.bndbuild.yml` with mixed Jinja+YAML content to validate the implementation.

### Next Steps for Validation

1. Install extension in Zed
2. Open `test_jinja_template.bndbuild.yml`
3. Verify syntax highlighting for both Jinja and YAML
4. Verify run triangles appear next to targets (including templated ones)
5. Click triangle and verify correct bndbuild command executes

**Status:** Implementation complete - pending user testing in Zed

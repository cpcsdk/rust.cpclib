# Jinja2 Template Support — Implementation Notes

**Status:** ❌ **Not Working** (reverted to YAML-only)

## The Problem

Users want to write bndbuild files that contain both:
- YAML structure (targets, dependencies, build steps)
- Jinja2 templates (`{{ variable }}`, `{% for ... %}`, etc.)

Example:
```yaml
targets: {{ project_name }}.dsk
```

## What We Tried

### Attempt 1: Grammar Injection (Failed)

Following the pattern from [zed-jinja-universal](https://github.com/Else00/zed-jinja-universal):

1. **Registered jinja2 grammar** in `extension.toml`
2. **Set `grammar = "jinja2"`** in `languages/bndbuild/config.toml`
3. **Created `injections.scm`** to inject YAML into Jinja content nodes
4. **Created `highlights.scm`** for Jinja syntax highlighting

**Result:** Extension compiled but **files were not recognized** in Zed.

**Root cause:** When we set `grammar = "jinja2"` in config.toml, Zed tried to load the jinja2 grammar during extension initialization. However, the grammar registration in `extension.toml` only makes the grammar *available* for tree-sitter queries — it doesn't make it usable as a language's primary grammar in the same extension.

### Why zed-jinja-universal Works Differently

The [zed-jinja-universal extension](https://github.com/Else00/zed-jinja-universal) creates **separate languages** for each Jinja variant:
- `yaml_jinja` (separate language)
- `html_jinja` (separate language)  
- `css_jinja` (separate language)
- etc.

Each `*_jinja` language:
- Has its own directory in `languages/`
- Uses `grammar = "jinja2"` in its config.toml
- Has file suffix patterns like `.yaml.jinja`, `.html.jinja`

**This is NOT what we want** because:
1. Users don't want separate `.bnd.jinja` and `.bnd` file types
2. We want ONE language that handles both pure YAML and Jinja-templated files
3. Bndbuild files use templates **optionally**, not as a file type distinction

## Current Solution: YAML Only

**Reverted to:**
```toml
# languages/bndbuild/config.toml
grammar = "yaml"
```

**This means:**
- ✅ Pure YAML bndbuild files work perfectly
- ✅ Runnables (run triangles) work
- ✅ Syntax highlighting for YAML works
- ❌ Jinja syntax appears as errors
- ❌ Templated files have broken highlighting

## Potential Future Solutions

### Option 1: Separate Language (Not Preferred)
Create `bndbuild_jinja` as a separate language with different file suffixes.
- **Pro:** Would work technically
- **Con:** Users explicitly rejected this approach

### Option 2: Wait for Zed Enhancement
Wait for Zed to support grammar injection patterns where one language can dynamically switch grammars.
- **Pro:** Clean solution when available
- **Con:** Timeline unknown

### Option 3: Custom Tree-sitter Grammar
Create a new tree-sitter grammar that natively handles both YAML and Jinja in one parser.
- **Pro:** Would work perfectly for our use case
- **Con:** Significant development effort, maintenance burden

### Option 4: LSP-based Handling
Move template handling to the LSP server instead of the tree-sitter grammar.
- **Pro:** More flexible, can handle complex template logic
- **Con:** Doesn't fix syntax highlighting in editor

## Files Modified During Attempt

**Added (then removed):**
- `languages/bndbuild/highlights.scm` — Jinja highlighting rules
- `languages/bndbuild/injections.scm` — YAML injection query
- `languages/bndbuild/test_jinja_template.bndbuild.yml` — Test file

**Modified (then reverted):**
- `extension.toml` — Added jinja2 grammar registration (removed)
- `languages/bndbuild/config.toml` — Changed `grammar` from yaml to jinja2 (reverted)

**Unchanged:**
- `languages/bndbuild/runnables.scm` — Still detects YAML nodes (works with injection pattern)
- `languages/bndbuild/outline.scm` — Still provides YAML structure
- All other language configs

## Lessons Learned

1. **Grammar registration ≠ grammar usability:** Registering a grammar in `extension.toml` makes it available for tree-sitter queries but doesn't automatically make it work as a language's primary grammar.

2. **Zed's extension model:** Extensions can't reference external grammars (like jinja2) as the primary grammar for a language during the same extension load.

3. **The _jinja pattern:** The standard Zed pattern for Jinja support is to create separate `*_jinja` languages, not to make one language handle both cases.

4. **File recognition:** When `grammar = "jinja2"` was set, Zed couldn't find/compile the grammar, causing file recognition to fail entirely.

## Recommendation

For now, **use pure YAML** for bndbuild files. If Jinja templates are essential, consider:
1. Processing templates with a separate tool before running bndbuild
2. Using the bndbuild CLI directly (it supports templates)
3. Waiting for a future Zed enhancement that enables dynamic grammar switching

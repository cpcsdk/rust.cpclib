# Symbol Linking in Documentation

## What was implemented

The documentation generator now automatically makes all symbols in source code (macros and functions) clickable. When you view the HTML documentation:

1. **Symbol Detection**: All symbols (labels, macros, functions, equs) are automatically detected in source code
2. **Link Creation**: Symbols are wrapped with HTML links pointing to their documentation anchors
3. **Rust-Side Processing**: All processing happens on the Rust side during documentation generation (no JavaScript overhead)

## How it works

### Data Flow

```
Source Code → Parse Tokens → Extract Symbols → Generate Documentation
                                    ↓
                            Link Symbols in Source
                                    ↓
                            HTML with Clickable Links
```

### Implementation Details

1. **New Field**: `ItemDocumentation` now has a `linked_source: Option<String>` field
2. **Helper Function**: `link_symbols_in_source()` uses regex to replace symbol names with HTML links
3. **Automatic Processing**: Called in:
   - `for_file()` - when parsing a single file
   - `merge()` - when merging multiple documentation pages  
   - `populate_all_cross_references()` - after collecting all references

### Example

Given this macro source code:
```asm
    ld hl, my_label
    call other_func
    ret
```

The generated HTML will be:
```html
    ld hl, <a href="#label_my_label" class="symbol-link">my_label</a>
    call <a href="#function_other_func" class="symbol-link">other_func</a>
    ret
```

When users click on `my_label` or `other_func`, the browser navigates to the corresponding documentation section.

## Benefits

- **Better Navigation**: Users can quickly jump to symbol definitions
- **No JavaScript Required**: All processing done server-side during generation
- **Performance**: Uses pre-compiled regex patterns for efficiency
- **Smart Matching**: Only matches whole words (word boundaries) to avoid false matches

## Testing

A test `test_link_symbols_in_source` verifies that:
- Symbols are correctly wrapped in links
- Links point to correct anchors (#label_*, #function_*, etc.)
- Non-symbols (like instructions) are not linked

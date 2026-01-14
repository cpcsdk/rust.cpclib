# Directive Systematization - Implementation Checklist

## Phase 1: Stabilize Foundation ⚡ HIGH PRIORITY

### Task 1.1: Debug & Fix IMPORT Regression
- [ ] Add debug output to trace IMPORT command handling
- [ ] Verify command byte 0x17 is being detected
- [ ] Check if expression size parsing is correct
- [ ] Verify string extraction logic
- [ ] Test: Line 21 outputs `IMPORT "const.i"  ; for max_sources`
- [ ] **Success Criteria**: IMPORT test passes

### Task 1.2: Implement decode_expression_range() Helper
```rust
/// Decode expression bytes to readable assembly text
/// Handles: E_PC ($), E_OBJC ($$), label refs, operators, values
fn decode_expression_range(&self, start: usize, end: usize) -> Result<String, String>
```
- [ ] Create function skeleton
- [ ] Handle E_PC (0x24) → "$"
- [ ] Handle E_OBJC (0x44) → "$$"
- [ ] Handle label references (SHORT_LABEL, LONG_LABEL)
- [ ] Handle operators (+, -, *, /, etc.)
- [ ] Handle numeric values (hex, decimal, binary)
- [ ] Handle E_BEGIN/E_END (expression grouping)
- [ ] Add unit tests for each expression type
- [ ] **Success Criteria**: Can decode any expression to text

### Task 1.3: Fix Expression Value Decoding
- [ ] Find where assignments are decoded
- [ ] Use decode_expression_range() for value part
- [ ] Test: Line 23 shows `save_pc = $` (not `"7$"`)
- [ ] Test: Line 24 shows `save_obj = $$` (not `"8D"`)
- [ ] **Success Criteria**: Lines 23-24 pass strict matching

### Task 1.4: Verify No Regressions
- [ ] Run full test suite
- [ ] MACRO.I: Still 100% (5/5)
- [ ] CONST.I: Still 100% (101/101)
- [ ] No new failures introduced
- [ ] **Success Criteria**: Foundation solid, ready for Phase 2

---

## Phase 2: Implement Verified Directives 🎯 MEDIUM PRIORITY

### Task 2.1: Implement IF Directive Handler
Binary format: `7F 15 [expr_size] [expression] 41`

- [ ] Update decode_command() IF case to read expr_size
- [ ] Read expression bytes (expr_size - 1 for E_ENDOFDATA)
- [ ] Call decode_expression_range() on expression
- [ ] Format as `"      IF {expression}"`
- [ ] Test with binary: `7F 15 03 84 85 41`
- [ ] Test: Line 97 outputs `"      IF vo0 - &7080"`
- [ ] **Success Criteria**: All IF directives in MEMMAP decode correctly

### Task 2.2: Implement SKIP Directive Handler  
Binary format: `7F 08 [expr_size] [expression] 41`

- [ ] Add Command::Skip variant (or reuse existing)
- [ ] Update decode_command() SKIP case
- [ ] Read expr_size and expression bytes
- [ ] Call decode_expression_range()
- [ ] Format as `"      SKIP {expression}"`
- [ ] Test with binary: `7F 08 09 42 35 E0 76 20 2D 20 24 45`
- [ ] Test: Line 206 outputs `"      SKIP &76E0 - $"`
- [ ] **Success Criteria**: All SKIP directives decode correctly

### Task 2.3: Implement END Directive Handler
Binary format: `7F 0C 4A` (no parameters)

- [ ] Update decode_command() END case
- [ ] Simply output `"      END"`
- [ ] Ensure newline handling is correct
- [ ] Test: Lines 99, 102, etc. output `"      END"`
- [ ] **Success Criteria**: All END directives decode correctly

### Task 2.4: Add Text Conversion for All Three
- [ ] Ensure IF commands convert to text immediately
- [ ] Ensure SKIP commands convert to text immediately  
- [ ] Ensure END commands convert to text immediately
- [ ] Remove "DEBUG IF args:" output
- [ ] **Success Criteria**: No command objects leak to output

### Task 2.5: Phase 2 Testing
- [ ] Run strict_matching test
- [ ] MEMMAP.I: > 30% match (target: 35%+)
- [ ] SWAPI.I: > 60% match (target: 65%+)
- [ ] Document improvement metrics
- [ ] **Success Criteria**: Measurable progress on both files

---

## Phase 3: Table-Driven Architecture 🏗️ LOW PRIORITY

### Task 3.1: Design DirectiveInfo Structure
```rust
enum DirectiveParamType {
    None,
    Expression,
    String,
}

struct DirectiveInfo {
    keyword: &'static str,
    command_byte: u8,
    param_type: DirectiveParamType,
    handler: fn(&mut OrgamsDecoder) -> Result<String>,
}
```
- [ ] Define structure
- [ ] Add documentation
- [ ] Create const array placeholder
- [ ] **Success Criteria**: Structure compiles and is usable

### Task 3.2: Implement Handler Functions
- [ ] `decode_import_directive(&mut self) -> Result<String>`
- [ ] `decode_if_directive(&mut self) -> Result<String>`
- [ ] `decode_skip_directive(&mut self) -> Result<String>`
- [ ] `decode_end_directive(&mut self) -> Result<String>`
- [ ] Each handler reads params and returns formatted text
- [ ] **Success Criteria**: All handlers work independently

### Task 3.3: Build Directive Dispatch Table
```rust
const DIRECTIVE_TABLE: &[DirectiveInfo] = &[
    DirectiveInfo {
        keyword: "IMPORT",
        command_byte: 0x17,
        param_type: DirectiveParamType::String,
        handler: OrgamsDecoder::decode_import_directive,
    },
    // ... more directives
];
```
- [ ] Add all verified directives
- [ ] Add helper function `get_directive_info(cmd_byte: u8)`
- [ ] **Success Criteria**: Table lookup works

### Task 3.4: Refactor decode_command() to Use Table
- [ ] Replace match statement with table lookup
- [ ] Call handler function from table
- [ ] Handle unknown directives gracefully
- [ ] Keep old code commented out for reference
- [ ] **Success Criteria**: Same output with cleaner code

### Task 3.5: Phase 3 Testing
- [ ] Run full test suite
- [ ] All tests still pass
- [ ] Code is more maintainable
- [ ] Unknown directives show helpful message
- [ ] **Success Criteria**: Refactoring complete, no regressions

---

## Phase 4: Complete Coverage 🎯 LOW PRIORITY

### Task 4.1: Analyze Remaining Command Bytes
Using analyze_command_bytes.py:
- [ ] Identify what 0x01 encodes (ASIS/comment marker)
- [ ] Identify what 0x03 encodes
- [ ] Identify what 0x04 encodes
- [ ] Identify what 0x09 encodes
- [ ] Identify what 0x0F encodes
- [ ] Identify what 0x43 encodes (inline comment)
- [ ] **Success Criteria**: All common command bytes identified

### Task 4.2: Implement Handlers for New Directives
For each newly identified directive:
- [ ] Add DirectiveInfo entry to table
- [ ] Implement handler function
- [ ] Add unit test
- [ ] Test on actual .I files
- [ ] **Success Criteria**: Each new directive decodes correctly

### Task 4.3: Handle Edge Cases
- [ ] Nested expressions
- [ ] Complex label references
- [ ] Multiple expressions in one directive
- [ ] Unusual spacing/formatting
- [ ] **Success Criteria**: Robust handling of all cases

### Task 4.4: Final Testing & Documentation
- [ ] Run comprehensive test suite
- [ ] MEMMAP.I: > 80% match
- [ ] SWAPI.I: > 90% match
- [ ] Update DIRECTIVE_MAPPINGS.md with all entries
- [ ] Add examples for each directive
- [ ] **Success Criteria**: Production-ready decoder

---

## Progress Tracking

### Current Status (Starting Point)
- ✅ MACRO.I: 100% (5/5 lines)
- ✅ CONST.I: 100% (101/101 lines)
- ⚠️ MEMMAP.I: 22.6% (118/523 lines)
- ⚠️ SWAPI.I: 54.5% (42/77 lines)

### After Phase 1 (Target)
- ✅ MACRO.I: 100% maintained
- ✅ CONST.I: 100% maintained
- 🎯 MEMMAP.I: 25%+ (lines 21, 23-24 fixed)
- 🎯 SWAPI.I: 55%+ (no regressions)

### After Phase 2 (Target)
- ✅ MACRO.I: 100% maintained
- ✅ CONST.I: 100% maintained
- 🎯 MEMMAP.I: 35%+ (IF/SKIP/END work)
- 🎯 SWAPI.I: 65%+ (directives improved)

### After Phase 3 (Target)
- ✅ MACRO.I: 100% maintained
- ✅ CONST.I: 100% maintained
- 🎯 MEMMAP.I: 50%+ (cleaner code)
- 🎯 SWAPI.I: 75%+ (better coverage)

### After Phase 4 (Target)
- ✅ MACRO.I: 100% maintained
- ✅ CONST.I: 100% maintained
- 🎯 MEMMAP.I: 80%+ (comprehensive)
- 🎯 SWAPI.I: 90%+ (near complete)

---

## Notes & Reminders

### Critical Path
1. **Expression decoder first** - everything depends on this
2. **Fix regressions before adding features** - maintain working state
3. **Test incrementally** - don't batch changes
4. **Document as you go** - record findings immediately

### Common Pitfalls to Avoid
- ❌ Don't change multiple mappings at once
- ❌ Don't skip testing after each change
- ❌ Don't assume - always verify with binary
- ❌ Don't break existing tests while refactoring

### Success Indicators
- ✅ Tests pass after each task
- ✅ Match percentages increase
- ✅ Code becomes cleaner
- ✅ Unknown directives show helpful errors

---

**Last Updated**: 2026-01-14
**Next Action**: Start Phase 1, Task 1.1 - Debug IMPORT regression

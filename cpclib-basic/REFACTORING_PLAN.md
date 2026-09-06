# BASIC String Parser Refactoring Plan

## Current Status
- **File**: `string_parser.rs`
- **Lines**: 5,225
- **Functions**: 157 parser functions
- **Issues**: High code duplication, repetitive patterns, verbose token assembly

## Identified Patterns for Refactoring

### 1. Simple Keyword Parsers (25+ instances)
**Pattern**: Many functions just parse a keyword and return a single token
```rust
// Current (7-9 lines each × 25 = ~200 lines)
pub fn parse_end<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    Caseless("END")
        .map(|_| vec![BasicToken::SimpleToken(BasicTokenNoPrefix::End)])
        .parse_next(input)
}
pub fn parse_stop<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    Caseless("STOP")
        .map(|_| vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Stop)])
        .parse_next(input)
}
// ... 23 more similar functions
```

**Refactoring**: Use macro to generate these
```rust
// Proposed (~30 lines total)
macro_rules! simple_keyword {
    ($fn_name:ident, $keyword:expr, $token:ident) => {
        pub fn $fn_name<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
            Caseless($keyword)
                .map(|_| vec![BasicToken::SimpleToken(BasicTokenNoPrefix::$token)])
                .parse_next(input)
        }
    };
}

simple_keyword!(parse_end, "END", End);
simple_keyword!(parse_stop, "STOP", Stop);
simple_keyword!(parse_return, "RETURN", Return);
simple_keyword!(parse_tron, "TRON", Tron);
simple_keyword!(parse_troff, "TROFF", Troff);
simple_keyword!(parse_new, "NEW", New);
simple_keyword!(parse_cont, "CONT", Cont);
simple_keyword!(parse_frame, "FRAME", Frame);
simple_keyword!(parse_cat, "CAT", Cat);
simple_keyword!(parse_closein, "CLOSEIN", Closein);
simple_keyword!(parse_closeout, "CLOSEOUT", Closeout);
simple_keyword!(parse_ei, "EI", Ei);
simple_keyword!(parse_di, "DI", Di);
simple_keyword!(parse_deg, "DEG", Deg);
simple_keyword!(parse_rad, "RAD", Rad);
simple_keyword!(parse_wend, "WEND", Wend);
simple_keyword!(parse_tagoff, "TAGOFF", Tagoff);
simple_keyword!(parse_clg, "CLG", Clg);
// etc.
```

**Savings**: ~170 lines (200 → 30)

### 2. Token Assembly Pattern (100+ instances)
**Pattern**: Repetitive `res.append(&mut vec)` sequences
```rust
// Current pattern repeated everywhere:
let mut res = vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Ink)];
res.append(&mut space_a);
res.append(&mut pen);
res.append(&mut comma1);
res.append(&mut color1);
if let Some((mut comma2, mut color2)) = opt_color2 {
    res.append(&mut comma2);
    res.append(&mut color2);
}
Ok(res)
```

**Refactoring**: Helper functions
```rust
// Proposed
fn build_tokens(parts: &mut [&mut Vec<BasicToken>]) -> Vec<BasicToken> {
    let mut res = Vec::new();
    for part in parts {
        res.append(part);
    }
    res
}

fn build_tokens_with_base(base: BasicToken, parts: &mut [&mut Vec<BasicToken>]) -> Vec<BasicToken> {
    let mut res = vec![base];
    for part in parts {
        res.append(part);
    }
    res
}

// Usage:
Ok(build_tokens_with_base(
    BasicToken::SimpleToken(BasicTokenNoPrefix::Ink),
    &mut [&mut space_a, &mut pen, &mut comma1, &mut color1]
))
```

**Savings**: ~500 lines (reduces repetitive token assembly code)

### 3. Keyword + Single Expression Pattern (15+ instances)
**Pattern**: Keyword followed by single numeric expression
```rust
// Current pattern (repeated 15+ times):
pub fn parse_border<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    let (_, mut space_a, mut color1, opt_color2) = (
        Caseless("BORDER"),
        parse_space1,
        cut_err(parse_numeric_expression(NumericExpressionConstraint::None)),
        opt((parse_comma, parse_numeric_expression(NumericExpressionConstraint::None)))
    ).parse_next(input)?;
    
    let mut res = vec![BasicToken::SimpleToken(BasicTokenNoPrefix::Border)];
    res.append(&mut space_a);
    res.append(&mut color1);
    if let Some((mut comma2, mut color2)) = opt_color2 {
        res.append(&mut comma2);
        res.append(&mut color2);
    }
    Ok(res)
}
```

**Refactoring**: Generic helper
```rust
// Proposed
fn parse_keyword_with_expressions<'src>(
    keyword: &'static str,
    token: BasicTokenNoPrefix,
    count: usize,
    constraint: NumericExpressionConstraint,
    input: &mut &'src str
) -> BasicSeveralTokensResult<'src> {
    let _ = Caseless(keyword).parse_next(input)?;
    let mut space = parse_space1.parse_next(input)?;
    let mut exprs = vec![cut_err(parse_numeric_expression(constraint))
        .parse_next(input)?];
    
    for _ in 1..count {
        let (mut comma, mut expr) = (
            cut_err(parse_comma),
            cut_err(parse_numeric_expression(constraint))
        ).parse_next(input)?;
        exprs.append(&mut comma);
        exprs.append(&mut expr);
    }
    
    Ok(build_tokens_with_base(
        BasicToken::SimpleToken(token),
        &mut [&mut space, &mut exprs]
    ))
}

// Usage:
pub fn parse_poke<'src>(input: &mut &'src str) -> BasicSeveralTokensResult<'src> {
    parse_keyword_with_expressions("POKE", BasicTokenNoPrefix::Poke, 2, 
        NumericExpressionConstraint::Integer, input)
}
```

**Savings**: ~400 lines

### 4. Duplicate String Function Parsers (10 instances)
**Pattern**: CHR$, MID$, LEFT$, RIGHT$, etc. follow same structure
```rust
// Current: Each is 20-30 lines
fn parse_mid_dollar<'src>(...) { /* 25 lines */ }
fn parse_left_dollar<'src>(...) { /* 22 lines */ }
fn parse_right_dollar<'src>(...) { /* 24 lines */ }
```

**Refactoring**: Parametric function
```rust
fn parse_string_function<'src>(
    name: &'static str,
    token: StringFunctionToken,
    arg_count: usize,
    input: &mut &'src str
) -> BasicSeveralTokensResult<'src> {
    // Generic implementation
}
```

**Savings**: ~150 lines

### 5. Optional Canal/Stream Pattern (10 instances)
**Pattern**: Many commands support optional `#stream,` prefix
```rust
// Current: Repeated in 10+ functions
let canal = opt((parse_canal, parse_comma)).parse_next(input)?;
// ... then complex token assembly
```

**Refactoring**: Extract helper
```rust
fn parse_optional_stream<'src>(input: &mut &'src str) 
    -> ModalResult<Option<(Vec<BasicToken>, Vec<BasicToken>)>, ContextError<StrContext>> {
    opt((parse_canal, parse_comma)).parse_next(input)
}
```

**Savings**: ~50 lines

## Estimated Total Reduction

| Category | Current Lines | Refactored Lines | Savings |
|----------|---------------|------------------|---------|
| Simple keywords | ~200 | ~30 | 170 |
| Token assembly | ~800 | ~300 | 500 |
| Keyword+expression | ~600 | ~200 | 400 |
| String functions | ~250 | ~100 | 150 |
| Optional stream | ~100 | ~50 | 50 |
| Misc consolidation | ~400 | ~200 | 200 |
| **TOTAL** | **~5,225** | **~3,750** | **~1,475** |

**Expected reduction: ~28% (1,475 lines saved)**

## Priority Order

1. **Phase 1: Simple keywords** (Quick win, low risk)
   - Create macro
   - Apply to 25+ simple functions
   - Test: Should have 100% compatibility

2. **Phase 2: Token assembly helpers** (Medium effort, high impact)
   - Create helper functions
   - Refactor 50+ functions gradually
   - Test after each batch

3. **Phase 3: Keyword+expression pattern** (Medium effort, medium risk)
   - Create generic helpers
   - Refactor 15+ similar functions
   - Careful testing

4. **Phase 4: String function consolidation** (Lower priority)
   - More complex, affects fewer functions

5. **Phase 5: Final cleanup** (Polish)
   - Remove dead code
   - Consolidate imports
   - Add documentation

## Implementation Strategy

- **Test coverage**: Run full test suite after each phase
- **Git commits**: One commit per phase
- **Rollback plan**: Each phase is independent
- **Performance**: No impact expected (same generated code)

## Non-Goals (Keep For Now)

- Don't refactor expression parsing logic (complex, working well)
- Don't change token types (affects entire codebase)
- Don't modify parse_basic_line (entry point, keep stable)
- Don't consolidate parse_if (too complex, high risk)

## Risks & Mitigation

**Risk**: Breaking existing functionality
**Mitigation**: Comprehensive test suite (571 lines, 100% passing)

**Risk**: Regression in reconstruction
**Mitigation**: Test reconstruction after each phase

**Risk**: Performance degradation
**Mitigation**: Benchmark before/after (unlikely issue with macros)

## Next Steps

1. Review this plan
2. Implement Phase 1 (simple keywords)
3. Run tests & verify
4. Continue with remaining phases

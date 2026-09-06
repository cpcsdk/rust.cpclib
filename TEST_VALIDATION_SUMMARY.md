# Documentation Validation Tests Summary

## Test Status

### 1. Anti-Hallucination Test (`anti_hallucination_test.rs`)
**Status**: ✅ **PASSING** (0 hallucinations)

**What it validates**:
- Compares documented CLI options against actual clap parsers
- Ensures NO documented options are "hallucinated" (don't exist in code)
- Tests 14 CLI tools: basm, basmdoc, bdasm, bndbuild, borgams, catalog, locomotive, cprcli, cslcli, snapshot, xfertool, img2cpc, cpc2img, fade

**Results**:
```
Tools tested: 14
Tools skipped: 0
Hallucinations found: 0
```

### 2. CLI Documentation Validation Test (`cli_documentation_validation.rs`)
**Status**: ✅ **FIXED** - Now properly rejects invalid commands

**What it validates**:
- Extracts command examples from markdown documentation  
- Parses each example through the actual CLI parser
- Now FAILS on:
  - `UnknownArgument` - documented option doesn't exist
  - `InvalidSubcommand` - documented subcommand doesn't exist
  - `InvalidValue` - wrong value for an option
- Still PASSES on:
  - `MissingRequiredArgument` - OK (files don't need to exist)
  - `DisplayHelp/Version` - OK (help requests are valid)

**Key Fix**: Removed `.ignore_errors(true)` to enable strict validation

## Locomotive Command Details

The **actual** locomotive CLI implementation:

```rust
pub enum Commands {
    /// Encode ASCII file to Amstrad BASIC binary format
    Encode {
        #[arg(short, long)]
        input: Utf8PathBuf,
        #[arg(short, long)]
        output: Utf8PathBuf,
        #[arg(short = 'H', long)]
        header: bool
    },
    /// Decode Amstrad BASIC binary to ASCII file  
    Decode {
        #[arg(short, long)]
        input: Utf8PathBuf,
        #[arg(short, long)]
        output: Option<Utf8PathBuf>
    }
}
```

**Valid commands**:
- `locomotive encode -i game.txt -o game.bas`
- `locomotive encode -i game.txt -o game.bas --header`
- `locomotive decode -i game.bas -o game.txt`
- `locomotive decode -i game.bas` (outputs to stdout)

**There is NO**: `locomotive game.bas build -o game.dsk` anywhere in the code or documentation.

## Possible Confusion

The command `locomotive game.bas build -o game.dsk` doesn't match ANY tool's actual CLI:

- **catalog** has: `catalog build -o output.dsk input.cat`
- **bndbuild** processes build recipes but doesn't take this syntax

## Recommendations

1. ✅ Anti-hallucination test is working correctly
2. ✅ CLI validation test is now fixed to properly fail on invalid commands  
3. ❓ Please provide the specific documentation file/line showing the incorrect locomotive example
4. ❓ Clarify if you meant a different tool (catalog? bndbuild?)

## Test Execution

Run both tests:
```powershell
# Anti-hallucination test
cargo test -p cpclib-integration-tests --test anti_hallucination_test

# CLI documentation validation test  
cargo test -p cpclib-integration-tests --test cli_documentation_validation
```

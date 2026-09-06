# Runner Macros Usage Guide

This document explains how to use the macros defined in `macros.rs` to reduce boilerplate code when creating new runners.

## Overview

The runner macros automatically generate:
- The struct definition with `PhantomData<E>`
- `Default` implementation with standard clap configuration
- `RunnerWithClap` trait implementation
- `RunnerWithClapMatches` or `RunnerWithClapDerive` trait implementation
- `Runner` trait implementation with standard error handling

## Macro Types

### 1. `define_clap_derive_runner!`

Use this macro for runners based on clap's derive API (structs with `#[derive(Parser)]`).

**Syntax:**
```rust
crate::define_clap_derive_runner! {
    RunnerName,                  // Name of the runner struct
    ArgsType,                    // Type that implements clap::CommandFactory
    COMMAND_NAME_CONST,          // Command name (usually from task constants)
    "package version string",    // Version info for --help
    |args| { /* process */ }     // Closure that processes the parsed args
}
```

**Example from `csl.rs`:**
```rust
crate::define_clap_derive_runner! {
    CslRunner,
    CslCliArgs,
    CSL_CMDS[0],
    concat!("cpclib-cslcli ", env!("CARGO_PKG_VERSION")),
    |cli| cpclib_cslcli::run(&cli).map_err(|e| e.to_string())
}
```

**What it generates:**
- Implements `RunnerWithClapDerive` with `type Args = ArgsType`
- Calls `self.get_args(itr, o)?` in `inner_run`
- Returns early if args parsing returns `None`

### 2. `define_custom_builder_runner!`

Use this macro for runners that use a custom builder function to create the clap `Command`.

**Syntax:**
```rust
crate::define_custom_builder_runner! {
    RunnerName,                     // Name of the runner struct
    builder_function_call(),        // Function that returns clap::Command
    COMMAND_NAME_CONST,             // Command name
    PKG_NAME_CONST,                 // Package name for --help
    PKG_VERSION_CONST,              // Package version for --help
    |matches, command| { /* */ }    // Closure with matches AND command ref
}
```

**Example:**
```rust
crate::define_custom_builder_runner! {
    BasmDocRunner,
    cpclib_basmdoc::cmdline::build_args_parser(),
    BASMDOC_CMD,
    cpclib_basmdoc::built_info::PKG_NAME,
    cpclib_basmdoc::built_info::PKG_VERSION,
    |matches, command| cpclib_basmdoc::cmdline::handle_matches(&matches, command)
        .map_err(|e| e.to_string())
}
```

**What it generates:**
- Implements `RunnerWithClapMatches`
- Calls `self.get_matches(itr, o)?` in `inner_run`
- Passes both `matches` and `&self.command` to the processing closure

### 3. `define_custom_builder_runner_simple!`

A simplified version when the processing function doesn't need the command reference.

**Syntax:**
```rust
crate::define_custom_builder_runner_simple! {
    RunnerName,                     // Name of the runner struct
    builder_function_call(),        // Function that returns clap::Command
    COMMAND_NAME_CONST,             // Command name
    PKG_NAME_CONST,                 // Package name for --help
    PKG_VERSION_CONST,              // Package version for --help
    |matches| { /* process */ }     // Closure with only matches
}
```

**Example from `xfer.rs`:**
```rust
crate::define_custom_builder_runner_simple! {
    XferRunner,
    cpclib_xfertool::build_args_parser(),
    XFER_CMDS[0],
    cpclib_xfertool::built_info::PKG_NAME,
    cpclib_xfertool::built_info::PKG_VERSION,
    |matches| cpclib_xfertool::process(&matches).map_err(|e| e.to_string())
}
```

## Standard Features

All macros automatically add:

1. **Help and Version flags:**
   - `--help/-h` flag (exclusive)
   - `--version/-V` flag (exclusive)

2. **After-help text:**
   - Shows embedded package info and bndbuild version

3. **No binary name mode:**
   - Commands work correctly when embedded

4. **Standard error handling:**
   - Returns early if argument parsing fails
   - Handles `None` returns from parsing

## Migration Guide

### Before (manual implementation):
```rust
pub struct MyRunner<E: EventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for MyRunner<E> {
    fn default() -> Self {
        let command = MyArgs::command()
            .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(Arg::new("help")...)
            .arg(Arg::new("version")...)
            .after_help(...);
        Self {
            command,
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> RunnerWithClap for MyRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EventObserver> RunnerWithClapDerive for MyRunner<E> {
    type Args = MyArgs;
}

impl<E: EventObserver> Runner for MyRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let cli = self.get_args(itr, o)?;
        if cli.is_none() {
            return Ok(());
        }
        let cli = cli.unwrap();
        my_process_function(&cli).map_err(|e| e.to_string())
    }

    fn get_command(&self) -> &str {
        MY_CMD
    }
}
```

### After (using macro):
```rust
crate::define_clap_derive_runner! {
    MyRunner,
    MyArgs,
    MY_CMD,
    concat!("my-package ", env!("CARGO_PKG_VERSION")),
    |cli| my_process_function(&cli).map_err(|e| e.to_string())
}
```

## When NOT to Use the Macros

Don't use these macros when:

1. **Custom EventObserver requirements:** If your runner needs a specific `EventObserver` type (e.g., `Arc<E>` instead of `E`)
2. **Complex initialization:** If you need custom logic in `Default::impl`
3. **Custom trait implementations:** If you need to override methods like `get_matches`
4. **Complex inner_run logic:** If you need more than simple arg parsing and function call

For these cases, implement the runner manually (see `assembler.rs`, `bndbuild.rs` for examples).

## Complete Examples

See these files for working examples:
- `csl.rs` - clap_derive runner
- `crunch.rs` - clap_derive runner  
- `xfer.rs` - custom builder runner (simple)
- `hxcfe.rs` - clap_derive runner
- `snapshot.rs` - custom builder runner

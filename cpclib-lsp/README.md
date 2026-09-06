# cpclib-lsp

Language Server Protocol (LSP) implementation for Amstrad CPC development tools.

## Overview

`cpclib-lsp` provides intelligent code editing features for:

- **Z80 Assembly** (`.asm`, `.s`, `.z80`) — basm syntax
- **Build files** (`.build`, `.bnd`, `bndbuild.yml`) — bndbuild YAML + Jinja templating
- **Locomotive BASIC** (`.bas`) — CPC BASIC
- **CatArt BASIC** (`.CAT`/`.ASC`) — the same BASIC grammar, restricted by convention to a
  whitelist of drawing commands, with its own extra diagnostics
- **CSL** (`.csl`) — CPC Script Language, a line-oriented emulator-automation script format

The same binary also doubles as the debug adapter (`cpclib-lsp dap`), the bndbuild CLI
(`cpclib-lsp bndbuild ...`), and `cpclib-runner`'s emulator CLI (`cpclib-lsp emu ...`) — see
[CLI subcommands](#cli-subcommands) — so an editor integration only needs to ship/locate one
binary.

## Features by document type

### Z80 Assembly (`.asm`/`.s`/`.z80`)

- **Diagnostics** — parse/assembly errors (recursive error-tree walk, bounded multi-error
  recovery), overflow/override-memory/fake-instruction/redundant-accumulator-prefix/unused-binding
  warnings, a self-modifying-code label lint, an optional (off by default) peephole-optimizer
  advisory pass
- **Hover** — instruction timing, register/directive docs, numeric-literal bases, label/symbol
  info, firmware-routine documentation (by symbol or address), embedded BASIC
- **Completion** — Z80 mnemonics/operands, registers, directives, labels from other open files
- **Go to definition / references** — labels/symbols, `INCLUDE`/`INCBIN`/`BINCLUDE` targets
  (including basm's embedded `inner://` resources), embedded-BASIC line targets, workspace-wide
  fallback when not defined locally
- **Rename** — single-file and cross-workspace (a `Global` label's every reference, across every
  `.asm` file under the workspace roots)
- **Call hierarchy** — `CALL`/`CALL cc,label`
- **Document symbols / outline** — modules, sections, labels, macros, functions, `EQU`/assign,
  local labels nested under their global label
- **Semantic tokens** — AST-derived syntax highlighting, plus embedded Locomotive BASIC blocks
- **Inlay hints** — closing-directive hints (which `IF`/`REPEAT`/... an `ENDIF`/`ENDM`/... closes)
- **Code actions / refactorings** — wrap in `REPEAT`, join/split statement lines, remove an
  unused `REPEAT`/`ITERATE`/`FOR` loop counter, remove an unused MACRO/FUNCTION parameter
  (cross-file call-site rewrite)
- **Peephole optimizer** — advisory diagnostics + quickfix + "⚡ Fix All" CodeLens (off by
  default; never changes real assembly output on its own)
- **Formatting** — whole-document (`cpclib-asmfmt`)
- **Document colors** — swatches next to numerals that plausibly encode a Gate Array ink-select
  byte
- **Cycle counting** — min/max NOP count over a selection, control-flow-aware
- **Register tracking** — hover shows a register's tracked value, honoring `IN:`/`OUT:` contract
  comments
- **Macro/struct/FUNCTION expansion preview** — hover shows what a call actually expands to, via
  a real dry-run assemble (never triggers `SAVE`/`BUILDSNA`/`PAUSE` side effects)
- **Embedded blocks** — `LOCOMOTIVE`/`ENDLOCOMOTIVE` BASIC blocks and `#!bndbuild` YAML rules
  embedded in comments get their own hover/completion/CodeLens/rename/call-hierarchy, delegated to
  the BASIC/bndbuild analyzers
- **"▶ Run in emulator" / "🐞 Debug" CodeLens** — builds exactly as a debug launch would, then
  hands the snapshot to the project's configured emulator
- **Breakpoint sync** — an editor gutter breakpoint becomes a `BREAKPOINT` directive in the
  source (and is removed on untoggle)

### Build files (`.build`/`.bnd`/`bndbuild.yml`)

- **Diagnostics** — YAML validity, missing `targets:`/`tasks:` structure, dependencies that are
  neither an existing file nor a target this file builds (with brace/glob expansion), Jinja
  template rendering errors mapped back to the original source line
- **Hover** — rule keywords, task-type documentation, Jinja macro-call preview
- **Completion** — build keywords, 50+ task types, filesystem paths, Jinja snippets, real
  `--help`-scraped flags for delegated (third-party) tasks, real `clap`-derived flags for internal
  tasks
- **Go to definition / references** — a dependency to the rule that produces it, a Jinja
  variable to its `{% set %}`, on-disk file references
- **Rename** — single-file and cross-workspace Jinja variable rename
- **Call hierarchy** — target dependencies and Jinja macro definitions/calls
- **Document symbols / outline** — `{% set %}` variables and rule/target names
- **Semantic tokens** — syntax highlighting (Jinja-aware)
- **"▶ Run" CodeLens** — per rule and per individual task command, streaming output back to the
  editor and mapping a failure to the source line that caused it
- **Quickfix** — a rule depends on something nothing builds → offer to write the missing rule
- Every position-sensitive feature above works correctly on Jinja-templated files via a source
  map from expanded text back to original lines

### Locomotive BASIC (`.bas`) / CatArt BASIC (`.CAT`/`.ASC`)

- **Diagnostics** — parse errors, `FOR`/`NEXT` balance, use-before-assignment variables; CatArt
  additionally flags any command outside its drawing-command whitelist (via the real
  `cpclib-catart` converter, not a hand-maintained list)
- **Hover** — keyword documentation, numeric literals, firmware-routine docs
- **Completion** — keywords, variables already defined in the document
- **Go to definition / references** — line-number jump targets (`GOTO`/`GOSUB`/`ON ... GOSUB`/
  `AFTER`/`EVERY`), `FOR`/`NEXT` pairing, first variable assignment
- **Call hierarchy** — `GOSUB` targets as the "call" relation
- **Document symbols / outline** — variables and lines
- **Semantic tokens** — syntax highlighting
- **Inlay hints** — `CHR$(n)` shows the printed character / firmware control-code name / graphics
  glyph
- **Document colors** — swatches for `INK`/`BORDER` arguments
- **On-Enter line numbering** — auto-inserts the next line number in a numbered program
- **Renumbering** — code action (10, 20, 30, ...)
- **Formatting** — keyword-case normalization
- **"▶ Run in emulator" / "🐞 Debug" CodeLens**

### CSL (`.csl`) — new

A line-oriented script format for automating CPC emulators (insert a disk, wait, type text,
reset, ...). Support added this session:

- **Diagnostics** — a parse error is reported at the real offending line/token (not a
  placeholder), via `cpclib_csl`'s own rich-error path
- **"▶ Run in emulator" CodeLens** (`cpclib.runCsl`) — and a matching **file-explorer
  right-click → "Run CSL script in emulator"** entry (`cpclib.runCslFile` client-side command, VS
  Code only for now — see [Editor integrations](#editor-integrations))
- **Native vs. interactive emulator support**: `Emulator::accept_csl()` (in `cpclib-runner`) tags
  which emulators accept a `.csl` file directly as a launch argument — currently **AMSpiriT**
  (`--csl=<path>`) and **SugarboxV2** (`-s`/`--csl <path>`, confirmed against its own
  documentation — these two use different flag shapes, both handled). Every other emulator is
  driven through a small interpreter (`cpclib_runner::csl_interpreter`): leading
  `disk_insert`/`snapshot_load` instructions are folded into the emulator's launch arguments, and
  `key_output`/`wait*` instructions occurring anywhere in the script are replayed live after
  launch through the same OS-level keystroke-injection layer already used for BASIC/Orgams
  autotype. A media/reset/config instruction that shows up *after* the script has already started
  typing or waiting has no live backend to execute it on any emulator today, and is reported
  rather than silently dropped or faked.
- No hover, completion, symbols, semantic tokens, or call hierarchy yet — diagnostics + run are
  the whole feature set so far.
- Also reachable outside the editor entirely: `cpclib-lsp emu --csl <file> --emulator <name> run`
  (or the standalone `cpclib-runner`/`emu` CLI) runs a `.csl` file with no editor involved at all.

## CLI subcommands

The `cpclib-lsp` binary is more than just the language server:

| Invocation | What it does |
|---|---|
| `cpclib-lsp` | Starts the language server (stdio transport) |
| `cpclib-lsp --init-config [DIR]` | Writes a fresh, fully-commented `cpclib-lsp.toml` into `DIR` (default: current directory); refuses to overwrite an existing file |
| `cpclib-lsp --update-config [DIR]` | Adds any config field missing from `DIR`'s `cpclib-lsp.toml` (new fields from a newer version of this tool), leaving existing values/comments untouched |
| `cpclib-lsp bndbuild <args...>` | Runs as the `bndbuild` CLI itself — every argument after `bndbuild` passes straight through |
| `cpclib-lsp emu <args...>` | Runs as `cpclib-runner`'s `emu` CLI (launch a `.sna`/`.dsk`/`.csl` in any installed/installable emulator) |
| `cpclib-lsp emu-list` | Prints every emulator `cpclib-runner` knows about as JSON (id, label, debuggable, installed, DAP id) — for an editor's "run/debug with..." picker |
| `cpclib-lsp dap` | Runs as the debug adapter, speaking DAP on stdio |

`--stdio` is also accepted (and ignored) since `vscode-languageclient` always appends it.

## Configuration (`cpclib-lsp.toml`)

Found by walking upward from the workspace root, falling back to an XDG-style global location.
Every field is optional and defaults to today's behavior — a missing or partial file changes
nothing. Generate one with `cpclib-lsp --init-config`.

| Section | Fields |
|---|---|
| *(top-level)* | `log` — write a trace log to this file |
| `[asm]` | `case_sensitive`, `warnings_as_errors`, `warnings.*` (7 classes), `run_emulator`, `firmware_docs`, `peephole_goal` (`neutral`/`size`/`speed`), `inactive_code`, `inlay_hints`, `code_lens`, `breakpoint_directive`, `entry` |
| `[basic]` | `warnings_as_errors`, `warnings.*` (2 classes), `code_lens`, `inlay_hints`, `run_emulator`, `firmware_docs` |
| `[bndbuild]` | `warnings_as_errors`, `warnings.*` (2 classes), `code_lens` |
| `[csl]` | `code_lens`, `run_emulator` (default `amspirit`) |
| `[dap]` | `log`, `emulator` (default `1984js`), `endpoint`, `port` (default `8765`) |
| `[music]` | `song_extensions`, `run_emulator` (default `ace`), `sid_wait_line_count` (default `72`) |

## Editor integrations

### VS Code (`cpclib-vscode`)

- Language registration for `.asm`, `.bnd`/`.build`, `.bas`, `.CAT`/`.ASC`, and (as of this
  session) `.csl`. Static TextMate grammars (comments/keywords/strings/numbers, work even before
  the LSP attaches) exist for `.asm`, `.bnd`/`.build`, and `.csl`; `.bas`/`.CAT`/`.ASC` instead
  rely entirely on the LSP's own semantic tokens for coloring.
- A `basm`-type debugger contribution (launches/attaches via `cpclib-dap`)
- File-explorer context menu: run/debug a `.sna`/`.dsk` with a specific emulator, play/build-DSK
  a music file, and (new) run a `.csl` script
- Status bar: live cycle-count display for the current selection

### Zed (`cpclib-lsp-zed`)

- Language server + tree-sitter grammars + snippets for `.asm`, `.bnd`/`.build`, `.bas`,
  `.CAT`/`.ASC`
- **`.csl` is not registered here yet** — CSL support only reaches Zed if a `.csl` file is
  manually set to one of the above language modes, which won't produce correct diagnostics.
  Adding a real CSL language definition (a `languages/csl/config.toml` plus, ideally, a grammar)
  is unstarted.
- No debug-adapter or file-explorer command integration (LSP + syntax highlighting + snippets
  only)

## Architecture

```text
cpclib-lsp/
├── src/
│   ├── main.rs          # binary entry point: LSP server + bndbuild/dap/emu subcommands
│   ├── lib.rs           # public API (config re-export, CpcLspBackend)
│   ├── server/          # tower-lsp backend: request dispatch, cross-file/workspace features
│   ├── common/          # Document/DocumentType, shared byte↔UTF-16 column helpers, firmware docs
│   ├── basm/            # Z80 assembly analyzer (one file per feature, see above)
│   ├── bndbuild/        # build-file analyzer (one file per feature, see above)
│   ├── locomotive/      # Locomotive BASIC / CatArt analyzer (one file per feature, see above)
│   └── csl/             # CSL analyzer: mod.rs (parse cache), diagnostics.rs, command.rs, run.rs
└── tests/
    ├── assembly_tests.rs
    ├── build_file_tests.rs
    ├── call_hierarchy_tests.rs
    ├── cli_bndbuild_tests.rs
    └── integration_tests.rs
```

Each analyzer module owns one document type end-to-end; `server/backend.rs` dispatches to the
right one by `DocumentType` and handles everything that's genuinely cross-cutting (workspace-wide
rename/goto-definition scans, the music/peephole/breakpoint/embedded-rule-discovery commands,
diagnostic publishing/debouncing).

Z80 instruction/register/directive/timing data comes from `cpclib-asm`'s own build-time
generation, and build-task metadata from `cpclib-bndbuild`'s own `lsp` module — both single
sources of truth, so the LSP can't drift from what the assembler/build tool actually support.

## Installation

```bash
cargo install --path cpclib-lsp
# binary lands at ~/.cargo/bin/cpclib-lsp
```

After installing a new version, VS Code: reload the window (or restart); the LSP server restarts
automatically. If you also changed `cpclib-vscode` itself (TypeScript/`package.json`), run
`npm run compile` inside `cpclib-vscode/` and reload the Extension Development Host (F5) or your
installed extension.

## Testing

```bash
# Everything
cargo test -p cpclib-lsp

# One area
cargo test -p cpclib-lsp assembly_tests
cargo test -p cpclib-lsp build_file_tests
cargo test -p cpclib-lsp integration_tests
```

Unit tests live alongside each feature module (`#[cfg(test)] mod tests` in the same file);
`tests/integration_tests.rs` drives a real `tower-lsp` service end-to-end over the LSP protocol
(initialize, document lifecycle, completion, hover, ...).

## Debugging the server itself

```bash
RUST_LOG=tower_lsp=debug,cpclib_lsp=debug cpclib-lsp
```

Or set `log = "cpclib-lsp.log"` at the top of `cpclib-lsp.toml` — useful when the editor gives no
visible stderr (a GUI launch has nowhere to show it).

## Known gaps

- CSL: no hover/completion/symbols/semantic-tokens; the non-native-emulator interpreter only
  executes leading disk/snapshot instructions plus `key_output`/`wait*` — a mid-script
  media/reset/config change on a non-native emulator is reported as unsupported, not executed
  (see the CSL section above for why).
- Zed: no `.csl` language registration at all yet.
- Locomotive BASIC: no rename support (assembly and build files have it; BASIC doesn't yet).
- Jinja-templated build files get position-accurate diagnostics/features via a source map, but a
  Jinja *rendering* error itself is reported as a single, file-level notice rather than pinpointed.

## Contributing

1. Add tests for new features (alongside the feature's own module)
2. Run `cargo fmt` and `cargo clippy`
3. Ensure all tests pass: `cargo test -p cpclib-lsp`
4. Update this README if adding or changing a user-visible feature

## License

Part of the cpclib project. See the root `LICENSE` file for details.

## Related Projects

- **cpclib-asm** — Z80 assembler with basm syntax
- **cpclib-bndbuild** — build automation tool for CPC projects
- **cpclib-csl** — CSL (CPC Script Language) parser
- **cpclib-runner** — emulator installation/launch/automation (`emucontrol`, `csl_interpreter`)
- **cpclib-dap** — the debug adapter this binary's `dap` subcommand runs
- **cpclib** — core library with CPC file format support

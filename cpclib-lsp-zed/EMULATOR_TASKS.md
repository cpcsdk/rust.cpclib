# Running a `.sna`/`.dsk` in an emulator from Zed

Unlike `languages/bndbuild/tasks.json` (bundled automatically - see
`RUNNABLES_SETUP.md`), tasks like these can't ship pre-registered with the
extension: Zed only lets an extension bundle tasks inside a
`languages/<name>/` folder, tied to that language's buffers, and `.sna`/
`.dsk` files have no language of their own here (they're raw binary,
nothing for the LSP to parse). Per Zed's own task docs, a task comes from
one of four places - the global `tasks.json`, a worktree's `.zed/tasks.json`,
an ad-hoc oneshot, or a language extension - and there's no fifth,
language-independent way for an extension to contribute one.

So: copy whichever of these you want into `~/.config/zed/tasks.json`
(global) or `.zed/tasks.json` (this project only). Each one runs
`cpclib-lsp emu --emulator <id> ... run` - the same CLI VS Code's own
"Run with..." commands use, just without VS Code's install-status picker
(these assume the emulator is already installed; run once from a terminal
first if you're not sure).

```json
[
  { "label": "Run .sna with ACE", "command": "cpclib-lsp", "args": ["emu", "--emulator", "ace", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with ACE", "command": "cpclib-lsp", "args": ["emu", "--emulator", "ace", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with WinAPE", "command": "cpclib-lsp", "args": ["emu", "--emulator", "winape", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with WinAPE", "command": "cpclib-lsp", "args": ["emu", "--emulator", "winape", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with CPCEC", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cpcec", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with CPCEC", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cpcec", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with Amspirit", "command": "cpclib-lsp", "args": ["emu", "--emulator", "amspirit", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with Amspirit", "command": "cpclib-lsp", "args": ["emu", "--emulator", "amspirit", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with SugarBox", "command": "cpclib-lsp", "args": ["emu", "--emulator", "sugarbox", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with SugarBox", "command": "cpclib-lsp", "args": ["emu", "--emulator", "sugarbox", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with CPCEmuPower", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cpcemupower", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with CPCEmuPower", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cpcemupower", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with CPCemu", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cpcemu", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with CPCemu", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cpcemu", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with Caprice", "command": "cpclib-lsp", "args": ["emu", "--emulator", "caprice", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with Caprice", "command": "cpclib-lsp", "args": ["emu", "--emulator", "caprice", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with Cadence", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cadence", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with Cadence", "command": "cpclib-lsp", "args": ["emu", "--emulator", "cadence", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with 1984", "command": "cpclib-lsp", "args": ["emu", "--emulator", "emulator1984", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with 1984", "command": "cpclib-lsp", "args": ["emu", "--emulator", "emulator1984", "--drivea", "$ZED_FILE", "run"] },

  { "label": "Run .sna with RetroVirtualMachine", "command": "cpclib-lsp", "args": ["emu", "--emulator", "rvm", "--snapshot", "$ZED_FILE", "run"] },
  { "label": "Run .dsk with RetroVirtualMachine", "command": "cpclib-lsp", "args": ["emu", "--emulator", "rvm", "--drivea", "$ZED_FILE", "run"] }
]
```

Keep only the pairs for emulators you actually use - Zed's task picker
(`task: spawn`) lists every task in scope at once, and 22 entries is a lot
of scrolling if you only ever reach for one or two.

For debugging (breakpoints, stepping, variables) rather than just running,
use Zed's "New Process Debugger..." / `.zed/debug.json` with the `cpclib-dap`
adapter instead - see the main `README.md`.

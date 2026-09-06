# Changelog

All notable changes to the CPClib VS Code extension are documented here.

## [0.0.1] - Initial release

- Language support for Z80 assembly (basm syntax), bndbuild, Locomotive BASIC, and CatArt
- Code intelligence for `.asm`/`.z80` files: completion, hover documentation, go to definition,
  document outline, call hierarchy, semantic highlighting
- Real-time diagnostics: syntax/assembly errors and warnings, disabled-code greying, a quickfix
  for the `equ $-1`/`equ $-2` self-modifying-code pitfall
- Cycle count calculator, register usage tracking, CPC color picker, format on type
- Breakpoints synced between the editor gutter and basm `breakpoint` directives
- "▶ Run in emulator" and "🐞 Debug" CodeLenses on `.asm` files and bndbuild rules
- Full debugger: stepping (including step-back), a reconstructed call stack, labelled
  registers, watches, and debug console commands (`-mv`, `-dv`, `-chips`, `-timer`, `-help`)
- bndbuild integration: syntax highlighting, CodeLens "▶ Run" buttons, task provider,
  streamed build output, inline diagnostics, Jinja2 template highlighting
- A custom hex-view editor for `.sna`/`.cpr` files
- Bundled `cpclib-lsp` binaries for Linux, Windows, and macOS - no separate install required

# Feedback from a Claude Code session using cpclib-mcp

Context: a Claude Code session was working in `skyline` (a 2K Amstrad CPC demo
project, `.mcp.json` pointing at this crate's `target/debug/cpclib-mcp`
binary) trying to shrink a ZX0-crunched binary. It ended up hand-rolling
everything with raw shell calls to `basm`/`bndbuild` and a live ACE emulator
window screenshotted via `wmctrl`+`import` (ImageMagick), because **no
cpclib-mcp tools were discoverable in that session at all** — a tool search
for both `"cpclib"` and `"mcp"` returned nothing beyond the harness's own
built-ins.

## Priority 0 — fix discovery first

Before adding features, work out why the server registered in `.mcp.json`
isn't surfacing any tools to a Claude Code session at all:

- Confirm the binary starts and responds to an MCP `tools/list` call when
  launched exactly as `.mcp.json` specifies (command `cpclib-mcp`, no args,
  env `RUST_LOG=info`).
- Check stderr/logs for a silent crash or an empty tool registry.
- Confirm the `.mcp.json` location/scoping matches what the Claude Code
  build expects — in the case that surfaced this, the file lived in a
  subdirectory (`main/.mcp.json`) one level below the project's git root,
  not at the root itself.

This is blocking — none of the features below matter if the server never
connects.

## Priority 1 — assemble-and-report tool

A single call that runs `basm` against a project's `build.bnd` /
`link_sky.asm`-style pipeline and returns **structured data**, not raw log
text: uncrunched size, crunched size per cruncher pass, final linked size,
free-bytes margin. The session was grepping colored terminal output for
lines like `CRUNCHED MAIN.B000 FROM 3456 BYTES TO 1769 BYTES` — that should
be a JSON field on the tool result.

## Priority 2 — cruncher comparison tool

A call that takes a `link_sky.asm`-style file and tries every
`SELECTED_CRUNCHER` option (APLIB, EXOMIZER, LZ4/48/49, LZSA1/2, SHRINKLER,
UPKR, ZX0, ZX0_BACKWARD, ZX7, TRANSPARENT) in one shot and returns sizes for
each, instead of a caller sed-patching the source and re-invoking `basm` a
dozen times by hand.

## Priority 3 — headless emulator run + frame capture

A call that takes a `.sna`/`.dsk` (+ optional autorun filename), runs the
emulator fully headlessly (offscreen framebuffer, **no window on the user's
real desktop**), advances N frames or seconds, and returns a PNG (or raw
pixel buffer) of the current screen. This is the one that most needs to not
touch the user's live X session — verifying a change currently means
literally launching a GUI emulator window on the user's actual desktop and
screenshotting it with ImageMagick.

## Priority 4 — crunch-friendly reordering search

A call that takes an assembly source file, a byte range or label range
considered "safe" (i.e. not cycle-exact / interrupt-timing code), and
enumerates legal reorderings of adjacent register-independent instructions
(or small groups of 3-4), rebuilding and reporting the crunched-size delta
for each candidate. The session built a throwaway Python harness for this
ad hoc (parsing `ld` instructions, checking register independence via
read/write set overlap, permuting, rebuilding via basm, diffing crunched
size). A general version of this — safe for arbitrary z80 asm, not just
`ld`-pairs, with a way to mark "hands off" timing-critical ranges — would be
the single most useful addition for this kind of size-golfing work.

---

# Round 2 — first real use of the tools (skyline 2K demo, 2026-09-20)

The updated server connected fine after a VSCode restart, and `report_build`,
`compare_crunchers`, `search_reorderings`, `suggest_optimizations` all worked.
Bugs / gaps found while using them:

1. **`search_reorderings` is unsound across control-flow instructions.** It
   proposed swapping `ld (SWITCH_PAGE+1),a` with the following `jr c,PAGE2`
   (skyline_int.asm, lines 294-295; -1 byte). That skips the store on the
   branch-taken path, so it changes behaviour. Conditional/unconditional
   jumps, calls, rets, djnz, halt, and anything with a label or self-modifying
   target between them must be hard barriers for the window.
2. **`suggest_optimizations` `jp2jr` is unsafe for relocated code.** It
   proposed `jp INTERx -> jr INTERx` for a 3-byte `jp` vector table that the
   program `ldi`-copies to address &38 (relative jr would break there).
   Suggest at least a warning when the jump's own address is referenced as a
   data/copy source (label used with `ld hl,LABEL` + `ldi`/`ldir`), or emit
   these as `bulk_unsafe: true` with a reason.
3. **`report_build` output is far too large.** The `log` field contains
   every basm warning (thousands of chars of "explicit 'A,' prefix" noise).
   Add a `verbose: false` default that only returns errors/PRINT lines.
4. **`report_build.free_bytes` ignores wrappers.** Total budget here is
   2048 = 128-byte AMSDOS header + linked size, but the tool computes
   target_size - linked. Either accept `overhead_bytes`, or document that
   callers must pass `target_size = budget - header`.
5. **`compare_crunchers` in isolation is misleading for linking decisions.**
   UPKR wins on the payload (1709 vs 1781 for zx0_backward) yet loses by 31
   bytes once the 170-byte decruncher stub is counted. A `compare_link_sizes`
   tool that re-links the project once per cruncher (rewriting
   SELECTED_CRUNCHER, and honouring UPKR_PROBS_ORIGIN so the probs array is
   not padded into the file) and reports total linked size would answer the
   real question in one call.
6. Cosmetic: `search_reorderings` baseline (1778) differs from the real build
   (1781) because of the backward-ZX0 delta bytes; document that the score is
   payload-only.

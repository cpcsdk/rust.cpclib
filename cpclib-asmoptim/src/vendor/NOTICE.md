# Vendored files

Everything in this directory is copied verbatim from
[mdlz80optimizer](https://github.com/santiontanon/mdlz80optimizer) by Santiago
Ontañón, licensed under the Apache License 2.0 (see `LICENSE` alongside them).

## `z80cpc-instruction-set.tsv` — per-instruction read/write semantics

Which registers, flags, ports and memory each instruction reads and writes —
the data behind `effects.rs`, and therefore behind the `regsNotUsedAfter` /
`flagsNotUsedAfter` constraints.

Deliberately the **CPC** variant of this table, not the `z80n` (ZX Spectrum
Next) one that sits beside it upstream: it carries CPC-specific timings and at
least one CPC-specific semantic correction its sibling lacks — *"out (c),r
instructions in the Amstrad CPC actually also depend on B, as ports are 16
addresses. So, even if they are written as out (c),r, in reality the
instruction is out (bc),r."*

Known imprecision, left unpatched on purpose: `LD A,R` is listed as writing no
flags where real hardware sets `S`/`Z`/`H`/`P/V`/`N`. See `effects.rs`'s module
comment for why that direction of error is harmless (it costs optimization
opportunities, never correctness).

## `pbo-patterns*.txt` — the optimization rules

They are compiled into this crate as its built-in rule set (see
`builtin_rules.rs`), and are also the corpus the pattern parser is validated
against, so a grammar regression that would break real upstream files fails CI
rather than being discovered later. The matching engine only *executes* the
subset of rules whose constraints it actually supports — parsing all of them is
deliberately independent of that.

Everything here is kept **verbatim**, deliberately: re-vendoring a newer
upstream release should be a plain file copy, with the tag/goal filtering and
the supported-constraint subset doing the selection at load time, rather than a
hand-edited fork that drifts away from upstream.

Upstream pattern sources:

- `pbo-patterns.txt` — the base pattern library
- `pbo-patterns-size.txt` — size-oriented patterns (`include`s the base file)
- `pbo-patterns-speed.txt` — speed-oriented patterns (`include`s the base file)

Note that the size and speed files contain **directly opposing** rules (size
turns `jp` into `jr`; speed turns `jr` into `jp`), which is why they are exposed
as alternative goals rather than unioned into one set.

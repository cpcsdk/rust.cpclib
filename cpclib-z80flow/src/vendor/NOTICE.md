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

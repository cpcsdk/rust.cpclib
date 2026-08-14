# Vendored files

Everything in this directory is copied verbatim from
[mdlz80optimizer](https://github.com/santiontanon/mdlz80optimizer) by Santiago
Ontañón, licensed under the Apache License 2.0 (see `LICENSE` alongside them).

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

## `tests/fixtures/upstream_potests/`

The peephole-optimizer test corpus from mdlz80optimizer
(`src/test/resources/data/potests/`), vendored verbatim: 88 inputs, 37 of them
paired with an `-expected.asm` recording what upstream's optimizer produces.
Apache-2.0, same project and licence as the pattern files above.

Kept as fixtures rather than rewritten because their value is precisely that
they are *not* ours: they are what the reference implementation is held to, so
running our engine over them is a differential test against it rather than
against our own assumptions.

Comparison has to be semantic. Upstream reformats as it emits (`ld a, (value)`)
and renames labels it moves (`__mdlrenamed__end`), so the expected files are
compared by what they *assemble to*, never by their text.
